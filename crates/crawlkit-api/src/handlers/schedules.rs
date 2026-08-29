use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::auth;
use crate::types::*;
use crawlkit_engine::AuditEventType;

/// Create a recurring crawl schedule (minimum interval: 60 seconds).
#[utoipa::path(
    post,
    path = "/api/v1/schedules",
    tag = "schedules",
    request_body = CreateScheduleRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 201, description = "Schedule created", body = ScheduleResponse),
        (status = 400, description = "Invalid URL, bounds, or interval below 60s", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "CSRF origin validation failed", body = ApiErrorBody)
    )
)]
pub async fn create_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<CreateScheduleRequest>,
) -> Result<(StatusCode, Json<ScheduleResponse>), ApiError> {
    validate_url(&req.start_url)?;
    validate_max_pages(req.max_pages)?;
    validate_concurrency(req.concurrency)?;
    validate_delay(req.request_delay_ms)?;

    let start_url = url::Url::parse(&req.start_url)
        .map_err(|e| ApiError::BadRequest(format!("Invalid URL: {e}")))?;

    if req.interval_secs < 60 {
        return Err(ApiError::BadRequest(
            "interval_secs must be at least 60".to_string(),
        ));
    }

    let start_url_str = start_url.to_string();
    let interval_secs = req.interval_secs;

    let crawl_config = crawlkit_engine::CrawlConfig {
        start_url,
        max_pages: req.max_pages,
        request_delay: std::time::Duration::from_millis(req.request_delay_ms),
        concurrency: req.concurrency,
        ..Default::default()
    };

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let schedule = ScheduleConfig {
        id: id.clone(),
        tenant_id: extract_tenant(&claims).to_string(),
        crawl_config: crawl_config.clone(),
        interval_secs: req.interval_secs,
        enabled: true,
        next_run: now + chrono::Duration::seconds(req.interval_secs as i64),
        last_run_at: None,
        last_crawl_id: None,
        created_at: now,
    };

    let response = ScheduleResponse {
        id: schedule.id.clone(),
        start_url: crawl_config.start_url.to_string(),
        interval_secs: schedule.interval_secs,
        enabled: schedule.enabled,
        next_run: schedule.next_run,
        last_run_at: schedule.last_run_at,
        created_at: schedule.created_at,
    };

    state.schedules.insert(id.clone(), schedule);

    state.audit_trail.record_tenant(
        AuditEventType::ScheduleCreated,
        &claims.sub,
        Some(extract_tenant(&claims)),
        &format!("schedule created: {id} for {start_url_str} every {interval_secs}s"),
    );

    Ok((StatusCode::CREATED, Json(response)))
}

/// List schedules visible to the caller (own tenant, or all for admins).
#[utoipa::path(
    get,
    path = "/api/v1/schedules",
    tag = "schedules",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Schedules visible to the caller", body = [ScheduleResponse]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn list_schedules(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Json<Vec<ScheduleResponse>> {
    let tenant = extract_tenant(&claims);
    let admin = is_admin(&claims);
    Json(
        state
            .schedules
            .iter()
            .filter(|entry| admin || entry.value().tenant_id == tenant)
            .map(|e| {
                let s = e.value();
                ScheduleResponse {
                    id: s.id.clone(),
                    start_url: s.crawl_config.start_url.to_string(),
                    interval_secs: s.interval_secs,
                    enabled: s.enabled,
                    next_run: s.next_run,
                    last_run_at: s.last_run_at,
                    created_at: s.created_at,
                }
            })
            .collect(),
    )
}

/// Delete a schedule. Cross-tenant access returns `404` by design.
#[utoipa::path(
    delete,
    path = "/api/v1/schedules/{id}",
    tag = "schedules",
    params(
        ("id" = String, Path, description = "Schedule identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 204, description = "Schedule deleted"),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "CSRF origin validation failed", body = ApiErrorBody),
        (status = 404, description = "Schedule not found or owned by another tenant", body = ApiErrorBody)
    )
)]
pub async fn delete_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let entry = state
        .schedules
        .get(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Schedule {id} not found")))?;

    if !can_access_tenant(&claims, &entry.tenant_id) {
        return Err(ApiError::NotFound(format!("Schedule {id} not found")));
    }
    drop(entry);

    state
        .schedules
        .remove(&id)
        .map(|_| {
            state.audit_trail.record_tenant(
                AuditEventType::ScheduleDeleted,
                &claims.sub,
                Some(extract_tenant(&claims)),
                &format!("schedule deleted: {id}"),
            );
            StatusCode::NO_CONTENT
        })
        .ok_or_else(|| ApiError::NotFound(format!("Schedule {id} not found")))
}

/// Partially update a schedule. Omitted fields keep their current values.
#[utoipa::path(
    patch,
    path = "/api/v1/schedules/{id}",
    tag = "schedules",
    request_body = UpdateScheduleRequest,
    params(
        ("id" = String, Path, description = "Schedule identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Updated schedule", body = ScheduleResponse),
        (status = 400, description = "Invalid URL, bounds, or interval below 60s", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "CSRF origin validation failed", body = ApiErrorBody),
        (status = 404, description = "Schedule not found or owned by another tenant", body = ApiErrorBody)
    )
)]
pub async fn update_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpdateScheduleRequest>,
) -> Result<Json<ScheduleResponse>, ApiError> {
    let entry = state
        .schedules
        .get(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Schedule {id} not found")))?;

    if !can_access_tenant(&claims, &entry.tenant_id) {
        return Err(ApiError::NotFound(format!("Schedule {id} not found")));
    }
    drop(entry);

    if let Some(ref url) = req.start_url {
        validate_url(url)?;
    }
    if let Some(pages) = req.max_pages {
        validate_max_pages(pages)?;
    }
    if let Some(conc) = req.concurrency {
        validate_concurrency(conc)?;
    }
    if let Some(delay) = req.request_delay_ms {
        validate_delay(delay)?;
    }
    if let Some(interval) = req.interval_secs {
        if interval < 60 {
            return Err(ApiError::BadRequest(
                "interval_secs must be at least 60".to_string(),
            ));
        }
    }

    let mut entry = state
        .schedules
        .get_mut(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Schedule {id} not found")))?;

    if let Some(ref url) = req.start_url {
        let parsed =
            url::Url::parse(url).map_err(|e| ApiError::BadRequest(format!("Invalid URL: {e}")))?;
        entry.crawl_config.start_url = parsed;
    }
    if let Some(pages) = req.max_pages {
        entry.crawl_config.max_pages = pages;
    }
    if let Some(delay) = req.request_delay_ms {
        entry.crawl_config.request_delay = std::time::Duration::from_millis(delay);
    }
    if let Some(conc) = req.concurrency {
        entry.crawl_config.concurrency = conc;
    }
    if let Some(interval) = req.interval_secs {
        entry.interval_secs = interval;
        entry.next_run = Utc::now() + chrono::Duration::seconds(interval as i64);
    }
    if let Some(enabled) = req.enabled {
        entry.enabled = enabled;
    }

    let response = ScheduleResponse {
        id: entry.id.clone(),
        start_url: entry.crawl_config.start_url.to_string(),
        interval_secs: entry.interval_secs,
        enabled: entry.enabled,
        next_run: entry.next_run,
        last_run_at: entry.last_run_at,
        created_at: entry.created_at,
    };

    state.audit_trail.record_tenant(
        AuditEventType::ScheduleUpdated,
        &claims.sub,
        Some(extract_tenant(&claims)),
        &format!("schedule updated: {id}"),
    );

    Ok(Json(response))
}

pub async fn run_scheduler(state: AppState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let now = Utc::now();

        let due: Vec<(String, crawlkit_engine::CrawlConfig, String, Option<String>)> = state
            .schedules
            .iter()
            .filter(|entry| entry.value().enabled && entry.value().next_run <= now)
            .map(|entry| {
                let s = entry.value();
                (
                    s.id.clone(),
                    s.crawl_config.clone(),
                    s.tenant_id.clone(),
                    s.last_crawl_id.clone(),
                )
            })
            .collect();

        for (schedule_id, config, tenant_id, previous_crawl_id) in due {
            if let Some(mut schedule) = state.schedules.get_mut(&schedule_id) {
                schedule.last_run_at = Some(now);
                schedule.next_run = now + chrono::Duration::seconds(schedule.interval_secs as i64);
            }

            let crawl_id = Uuid::new_v4().to_string();
            let result = CrawlResult {
                crawl_id: crawl_id.clone(),
                tenant_id: tenant_id.clone(),
                start_url: config.start_url.to_string(),
                status: "running".to_string(),
                pages_crawled: 0,
                issues_found: 0,
                created_at: Utc::now(),
                completed_at: None,
                storage_crawl_id: None,
            };
            state.crawl_results.insert(crawl_id.clone(), result);
            state.metrics.crawls_total.inc();
            state.metrics.active_crawls.inc();

            // Best-effort backpressure: at capacity, skip this cycle — the
            // schedule fires again at the next interval.
            let permit = state.crawl_permits.clone().try_acquire_owned().ok();
            if permit.is_none() {
                state.metrics.active_crawls.dec();
                if let Some(mut entry) = state.crawl_results.get_mut(&crawl_id) {
                    entry.status = "skipped_at_capacity".to_string();
                    entry.completed_at = Some(Utc::now());
                }
                tracing::warn!("Scheduled crawl {crawl_id} skipped: server at crawl capacity");
                continue;
            }

            let state_clone = state.clone();
            let crawl_id_clone = crawl_id.clone();
            let schedule_id_clone = schedule_id.clone();
            let tenant_id_clone = tenant_id.clone();
            tokio::spawn(async move {
                super::crawls::run_crawl_task_with_monitoring(
                    state_clone,
                    crawl_id_clone,
                    config,
                    permit,
                    previous_crawl_id,
                    Some(schedule_id_clone),
                    Some(tenant_id_clone),
                )
                .await;
            });

            tracing::info!("Scheduled crawl {crawl_id} started from schedule {schedule_id}");
        }
    }
}
