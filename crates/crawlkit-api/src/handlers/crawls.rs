use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth;
use crate::types::*;
use crawlkit_engine::AuditEventType;
use crawlkit_engine::CrawlConfig;

/// Start an asynchronous crawl. Returns immediately with `202`; poll
/// `GET /api/v1/crawls/{crawl_id}` for status.
#[utoipa::path(
    post,
    path = "/api/v1/crawls",
    tag = "crawls",
    request_body = CreateCrawlRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 202, description = "Crawl accepted and started in the background", body = CrawlResponse),
        (status = 400, description = "Invalid URL, max_pages, concurrency, or delay", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "CSRF origin validation failed", body = ApiErrorBody)
    )
)]
pub async fn start_crawl(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    headers: HeaderMap,
    Json(req): Json<CreateCrawlRequest>,
) -> Result<(StatusCode, Json<CrawlResponse>), ApiError> {
    validate_url(&req.start_url)?;
    validate_max_pages(req.max_pages)?;
    validate_concurrency(req.concurrency)?;
    validate_delay(req.request_delay_ms)?;

    if !crawlkit_engine::ssrf::is_public_url(&req.start_url) {
        return Err(ApiError::BadRequest(
            "URL targets a reserved internal hostname or private IP address".to_string(),
        ));
    }

    // Idempotent replay: a known key within the dedupe window returns the
    // original crawl instead of starting a duplicate.
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|k| !k.is_empty() && k.len() <= 256)
        .map(str::to_string);
    if let Some(key) = &idempotency_key {
        if let Some(entry) = state.idempotency_keys.get(key) {
            if Utc::now() - entry.created_at < IDEMPOTENCY_WINDOW {
                let crawl_id = entry.crawl_id.clone();
                drop(entry);
                tracing::info!(key = %key, %crawl_id, "Idempotent crawl replay");
                return Ok((
                    StatusCode::OK,
                    Json(CrawlResponse {
                        crawl_id,
                        status: "running".to_string(),
                        message: "Idempotent replay of an existing crawl".to_string(),
                    }),
                ));
            }
            drop(entry);
            state.idempotency_keys.remove(key);
        }
    }

    // Backpressure: bound concurrent crawl tasks; reject with 503 +
    // Retry-After when at capacity rather than queueing unboundedly.
    let permit = state
        .crawl_permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::Overloaded {
            retry_after_secs: 30,
        })?;

    let start_url = url::Url::parse(&req.start_url)
        .map_err(|e| ApiError::BadRequest(format!("Invalid URL: {e}")))?;

    let crawl_id = Uuid::new_v4().to_string();
    let tenant_id = extract_tenant(&claims).to_string();

    // The engine owns the storage row (it starts the crawl inside
    // `run_with_callback` and reports the id via `CrawlOutput`); the API-level
    // crawl_id above is the public identifier tracked in `crawl_results`.
    // Pages/findings are written under the engine's id and resolved through
    // `CrawlResult::storage_crawl_id` once the run completes.

    let result = CrawlResult {
        crawl_id: crawl_id.clone(),
        tenant_id: tenant_id.clone(),
        start_url: start_url.to_string(),
        status: "running".to_string(),
        pages_crawled: 0,
        issues_found: 0,
        created_at: Utc::now(),
        completed_at: None,
        storage_crawl_id: None,
    };

    state.crawl_results.insert(crawl_id.clone(), result);

    state.audit_trail.record_tenant(
        AuditEventType::CrawlStarted,
        &claims.sub,
        Some(extract_tenant(&claims)),
        &format!(
            "crawl started for {start_url} (max_pages={})",
            req.max_pages
        ),
    );

    state.metrics.crawls_total.inc();
    state.metrics.active_crawls.inc();
    state
        .metrics
        .crawls_started_by_tenant
        .get_or_create(&TenantLabel {
            tenant: extract_tenant(&claims).to_string(),
        })
        .inc();

    // Reserve the idempotency mapping before acknowledging the request.
    if let Some(key) = idempotency_key {
        // Opportunistic cleanup of expired entries (bounded scan; the map
        // is sized by distinct client keys, not crawl volume).
        let now = Utc::now();
        state
            .idempotency_keys
            .retain(|_, entry| now - entry.created_at < IDEMPOTENCY_WINDOW);
        state.idempotency_keys.insert(
            key,
            IdempotencyEntry {
                crawl_id: crawl_id.clone(),
                created_at: now,
            },
        );
    }

    // Spawn crawl task in background
    let crawl_id_clone = crawl_id.clone();
    let tenant_id_clone = tenant_id.clone();
    let config = CrawlConfig {
        start_url,
        max_pages: req.max_pages,
        request_delay: std::time::Duration::from_millis(req.request_delay_ms),
        concurrency: req.concurrency,
        ..Default::default()
    };

    tokio::spawn(async move {
        run_crawl_task(state, crawl_id_clone, config, Some(permit), Some(tenant_id_clone)).await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(CrawlResponse {
            crawl_id,
            status: "running".to_string(),
            message: "Crawl started successfully".to_string(),
        }),
    ))
}

/// Get the status of a crawl. Cross-tenant access returns `404` by design.
#[utoipa::path(
    get,
    path = "/api/v1/crawls/{crawl_id}",
    tag = "crawls",
    params(
        ("crawl_id" = String, Path, description = "Crawl identifier returned by the start endpoint")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Crawl status", body = CrawlResult),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Crawl not found or owned by another tenant", body = ApiErrorBody)
    )
)]
pub async fn get_crawl_status(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(crawl_id): Path<String>,
) -> Result<Json<CrawlResult>, ApiError> {
    let entry = state
        .crawl_results
        .get(&crawl_id)
        .ok_or_else(|| ApiError::NotFound(format!("Crawl {crawl_id} not found")))?;

    if !can_access_tenant(&claims, &entry.tenant_id) {
        return Err(ApiError::NotFound(format!("Crawl {crawl_id} not found")));
    }

    Ok(Json(entry.value().clone()))
}

/// Default-deny tenant gate for crawl-scoped endpoints.
///
/// The crawl MUST be known to this instance AND belong to the caller's
/// tenant (or the caller must be an admin). Unknown ids are rejected: an
/// absent entry must never fall through to an unscoped storage query.
fn authorize_crawl_access(
    state: &AppState,
    claims: &auth::Claims,
    crawl_id: &str,
) -> Result<String, ApiError> {
    let entry = state
        .crawl_results
        .get(crawl_id)
        .ok_or_else(|| ApiError::NotFound(format!("Crawl {crawl_id} not found")))?;

    if !can_access_tenant(claims, &entry.tenant_id) {
        return Err(ApiError::NotFound(format!("Crawl {crawl_id} not found")));
    }

    // Storage rows live under the engine's crawl id once available.
    Ok(entry
        .storage_crawl_id
        .clone()
        .unwrap_or_else(|| crawl_id.to_string()))
}

/// Aggregate statistics for a crawl (page/issue counts, severity breakdown).
#[utoipa::path(
    get,
    path = "/api/v1/crawls/{crawl_id}/stats",
    tag = "crawls",
    params(
        ("crawl_id" = String, Path, description = "Crawl identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Crawl statistics", body = CrawlStatsResponse),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Crawl not found or owned by another tenant", body = ApiErrorBody),
        (status = 500, description = "Storage failure", body = ApiErrorBody)
    )
)]
pub async fn get_crawl_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(crawl_id): Path<String>,
) -> Result<Json<CrawlStatsResponse>, ApiError> {
    let storage_crawl_id = authorize_crawl_access(&state, &claims, &crawl_id)?;

    let storage = state.storage.clone();
    let cid = storage_crawl_id.clone();
    let stats = tokio::task::spawn_blocking(move || storage.get_stats(&cid))
        .await
        .map_err(|e| ApiError::Internal(format!("Blocking task failed: {e}")))?
        .map_err(|e| ApiError::Internal(format!("Failed to get stats: {e}")))?;

    Ok(Json(CrawlStatsResponse {
        crawl_id,
        total_pages: stats.total_pages,
        total_issues: stats.total_issues,
        issues_by_severity: stats.issues_by_severity,
        issues_by_category: stats.issues_by_category,
        avg_response_time_ms: stats.avg_response_time_ms,
    }))
}

/// List findings (issues) detected during a crawl.
#[utoipa::path(
    get,
    path = "/api/v1/crawls/{crawl_id}/findings",
    tag = "crawls",
    params(
        ("crawl_id" = String, Path, description = "Crawl identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Findings for the crawl", body = [CrawlFinding]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Crawl not found or owned by another tenant", body = ApiErrorBody),
        (status = 500, description = "Storage failure", body = ApiErrorBody)
    )
)]
pub async fn get_crawl_findings(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(crawl_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let storage_crawl_id = authorize_crawl_access(&state, &claims, &crawl_id)?;

    let filter = crawlkit_engine::storage::IssueFilter::default();
    let storage = state.storage.clone();
    let cid = storage_crawl_id.clone();
    let issues = tokio::task::spawn_blocking(move || storage.get_issues(&cid, &filter))
        .await
        .map_err(|e| ApiError::Internal(format!("Blocking task failed: {e}")))?
        .map_err(|e| ApiError::Internal(format!("Failed to get findings: {e}")))?;

    let findings: Vec<serde_json::Value> = issues
        .iter()
        .map(|issue| {
            serde_json::json!({
                "id": issue.id,
                "page_id": issue.page_id,
                "category": issue.category.as_str(),
                "severity": issue.severity.as_str(),
                "code": issue.code,
                "title": issue.title,
                "description": issue.description,
                "element": issue.element,
                "recommendation": issue.recommendation,
            })
        })
        .collect();

    Ok(Json(findings))
}

/// List crawls visible to the caller (own tenant, or all for admins).
#[utoipa::path(
    get,
    path = "/api/v1/crawls",
    tag = "crawls",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Crawls visible to the caller", body = [CrawlResult]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn list_crawls(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Json<Vec<CrawlResult>> {
    let tenant = extract_tenant(&claims);
    let admin = is_admin(&claims);
    let results: Vec<CrawlResult> = state
        .crawl_results
        .iter()
        .filter(|entry| admin || entry.value().tenant_id == tenant)
        .map(|entry| entry.value().clone())
        .collect();

    Json(results)
}

#[derive(serde::Serialize, ToSchema)]
#[allow(dead_code)]
pub struct BacklinksResponse {
    crawl_id: String,
    total_internal_links: usize,
    total_external_links: usize,
    total_referring_domains: usize,
    orphan_pages: Vec<String>,
    top_pages_by_pagerank: Vec<serde_json::Value>,
}

/// Internal/external link analysis and PageRank summary for a crawl.
#[utoipa::path(
    get,
    path = "/api/v1/crawls/{crawl_id}/backlinks",
    tag = "crawls",
    params(
        ("crawl_id" = String, Path, description = "Crawl identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Backlink and PageRank summary", body = BacklinksResponse),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 404, description = "Crawl not found or owned by another tenant", body = ApiErrorBody),
        (status = 500, description = "Storage failure", body = ApiErrorBody)
    )
)]
pub async fn get_crawl_backlinks(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(crawl_id): Path<String>,
) -> Result<Json<BacklinksResponse>, ApiError> {
    let storage_crawl_id = authorize_crawl_access(&state, &claims, &crawl_id)?;

    let storage1 = state.storage.clone();
    let storage2 = state.storage.clone();
    let cid1 = storage_crawl_id.clone();
    let cid2 = storage_crawl_id.clone();
    let (link_pairs, external_links) = tokio::task::spawn_blocking(move || {
        let links = storage1.get_links_for_crawl(&cid1)?;
        let external = storage2.get_external_links(&cid2)?;
        Ok::<_, crawlkit_engine::storage::StorageError>((links, external))
    })
    .await
    .map_err(|e| ApiError::Internal(format!("Blocking task failed: {e}")))?
    .map_err(|e| ApiError::Internal(format!("Failed to get links: {e}")))?;

    let mut analyzer = crawlkit_engine::BacklinkAnalyzer::new();
    analyzer.load_from_crawl_data(&link_pairs);
    for (source, target) in &external_links {
        analyzer.add_backlink(crawlkit_engine::Backlink {
            source_url: source.clone(),
            target_url: target.clone(),
            anchor_text: String::new(),
            is_followed: true,
            is_internal: false,
        });
    }

    let _pagerank = analyzer.compute_pagerank(0.85, 20);
    let summary = analyzer.summarize();

    let top_pages: Vec<serde_json::Value> = summary
        .pages
        .iter()
        .take(20)
        .map(|p| {
            serde_json::json!({
                "url": p.url,
                "pagerank": p.pagerank,
                "inbound_links": p.inbound_links,
                "outbound_links": p.outbound_links,
                "referring_domains": p.referring_domains,
            })
        })
        .collect();

    Ok(Json(BacklinksResponse {
        crawl_id,
        total_internal_links: summary.total_internal_links,
        total_external_links: summary.total_external_links,
        total_referring_domains: summary.total_referring_domains,
        orphan_pages: summary.orphan_pages,
        top_pages_by_pagerank: top_pages,
    }))
}

pub async fn run_crawl_task(
    state: AppState,
    crawl_id: String,
    config: crawlkit_engine::CrawlConfig,
    // Held for the crawl's duration; released on drop (backpressure slot).
    _permit: Option<tokio::sync::OwnedSemaphorePermit>,
    tenant_id: Option<String>,
) {
    run_crawl_task_with_monitoring(state, crawl_id, config, _permit, None, None, tenant_id).await;
}

/// Run a crawl task with optional monitoring (compare against a previous crawl)
/// and schedule tracking (update `last_crawl_id` when done).
pub async fn run_crawl_task_with_monitoring(
    state: AppState,
    crawl_id: String,
    config: crawlkit_engine::CrawlConfig,
    _permit: Option<tokio::sync::OwnedSemaphorePermit>,
    previous_crawl_id: Option<String>,
    schedule_id: Option<String>,
    tenant_id: Option<String>,
) {
    use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};

    let alert_threshold = previous_crawl_id.as_ref().map(|_| 1);

    let engine_config = CrawlEngineConfig {
        crawl_config: config.clone(),
        previous_crawl_id,
        alert_threshold,
        tenant_id,
        ..Default::default()
    };

    let engine = CrawlEngine::new_shared(engine_config, state.storage.clone());

    let state_clone = state.clone();
    let result = engine
        .run_with_callback(
            config.start_url.as_ref(),
            Some(Arc::new(move |_url, _page_id, _findings| {
                state_clone.metrics.pages_crawled_total.inc();
            })),
        )
        .await;

    match result {
        Ok(output) => {
            if let Some(mut entry) = state.crawl_results.get_mut(&crawl_id) {
                entry.storage_crawl_id = Some(output.crawl_id.clone());
                entry.status = "completed".to_string();
                entry.pages_crawled = output.pages_crawled;
                entry.issues_found = output.issues_found;
                entry.completed_at = Some(Utc::now());
            }

            let tenant_id = state
                .crawl_results
                .get(&crawl_id)
                .map(|r| r.tenant_id.clone())
                .unwrap_or_default();

            state.metrics.active_crawls.dec();
            state
                .metrics
                .pages_crawled_total
                .inc_by(output.pages_crawled as u64);
            state
                .metrics
                .issues_total
                .inc_by(output.issues_found as u64);
            if !tenant_id.is_empty() {
                state
                    .metrics
                    .pages_by_tenant
                    .get_or_create(&TenantLabel {
                        tenant: tenant_id.clone(),
                    })
                    .inc_by(output.pages_crawled as u64);
            }
            state.audit_trail.record_tenant(
                AuditEventType::CrawlCompleted,
                "system",
                Some(&tenant_id),
                &format!(
                    "crawl completed: {} pages, {} issues",
                    output.pages_crawled, output.issues_found
                ),
            );
            state
                .metrics
                .fetch_duration_seconds
                .observe(output.elapsed.as_secs_f64());
            state
                .metrics
                .analysis_duration_seconds
                .observe(output.elapsed.as_secs_f64());
            super::webhooks::fire_webhooks(
                &state,
                "crawl.completed",
                &crawl_id,
                &tenant_id,
                output.pages_crawled,
                output.issues_found,
            );

            // If monitoring detected an alert, fire monitoring webhooks
            // and update the schedule's last_crawl_id.
            if let Some(ref monitoring) = output.monitoring {
                if monitoring.alert_triggered {
                    super::webhooks::fire_monitoring_webhooks(
                        &state,
                        &crawl_id,
                        &tenant_id,
                        monitoring,
                    );
                }
            }

            // Update the schedule's last_crawl_id so the next run can
            // compare against this crawl.
            if let Some(ref sid) = schedule_id {
                if let Some(mut schedule) = state.schedules.get_mut(sid) {
                    schedule.last_crawl_id = Some(output.crawl_id.clone());
                }
            }

            tracing::info!(
                "Crawl {crawl_id} completed: {} pages, {} issues",
                output.pages_crawled,
                output.issues_found
            );
        }
        Err(e) => {
            tracing::error!("Crawl {crawl_id} failed: {e}");
            state.metrics.errors_total.inc();

            let tenant_id = state
                .crawl_results
                .get(&crawl_id)
                .map(|r| r.tenant_id.clone())
                .unwrap_or_default();

            if let Some(mut result) = state.crawl_results.get_mut(&crawl_id) {
                result.status = "failed".to_string();
                result.completed_at = Some(Utc::now());
            }
            state.metrics.active_crawls.dec();
            state.audit_trail.record_tenant(
                AuditEventType::CrawlFailed,
                "system",
                Some(&tenant_id),
                &format!("crawl failed: {e}"),
            );
            super::webhooks::fire_webhooks(&state, "crawl.failed", &crawl_id, &tenant_id, 0, 0);
        }
    }
}
