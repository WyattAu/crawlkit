//! Usage metering and quota handlers (ADR-017, 6.0.0-alpha.3).
//!
//! Read path: `GET /tenants/{id}/usage` — per-day per-unit rollups for the
//! dashboard chart. Quota surface: `GET|PUT /tenants/{id}/quotas` — the
//! commercial control (default posture: unmetered; limits exist only where
//! an operator sets them).

use axum::extract::{Extension, Path, Query, State};
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth;
use crate::types::{extract_tenant, require_permission, ApiError, ApiErrorBody, AppState};

/// One per-day per-unit usage rollup (ADR-017 §2 read model).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UsageEntryDto {
    /// Metered unit (`pages`, `crawl_started`, `findings`, `export_bytes`,
    /// `scan_submitted`).
    pub unit: String,
    /// UTC day (RFC 3339, midnight) the usage is attributed to.
    pub day_utc: String,
    /// Total recorded that day.
    pub total: i64,
}

/// Query parameters for the usage read path.
#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    /// Inclusive lower bound, UTC date (`YYYY-MM-DD`). Defaults to 30 days
    /// before `to`.
    pub from: Option<String>,
    /// Inclusive upper bound, UTC date (`YYYY-MM-DD`). Defaults to today.
    pub to: Option<String>,
}

/// A tenant's configured daily quotas. `null` = unmetered for that unit
/// (the default posture: no limits exist unless an operator sets them).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct QuotaDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crawl_started: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan_submitted: Option<i64>,
}

impl From<QuotaDto> for crawlkit_engine::metering::Quota {
    fn from(dto: QuotaDto) -> Self {
        Self {
            pages: dto.pages,
            crawl_started: dto.crawl_started,
            findings: dto.findings,
            export_bytes: dto.export_bytes,
            scan_submitted: dto.scan_submitted,
        }
    }
}

impl From<&crawlkit_engine::metering::Quota> for QuotaDto {
    fn from(q: &crawlkit_engine::metering::Quota) -> Self {
        Self {
            pages: q.pages,
            crawl_started: q.crawl_started,
            findings: q.findings,
            export_bytes: q.export_bytes,
            scan_submitted: q.scan_submitted,
        }
    }
}

/// Parses a `YYYY-MM-DD` UTC date into a day-floor timestamp.
fn parse_day(s: &str, field: &str) -> Result<chrono::DateTime<Utc>, ApiError> {
    let d = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| ApiError::BadRequest(format!("`{field}` must be YYYY-MM-DD, got {s:?}")))?;
    d.and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc())
        .ok_or_else(|| ApiError::BadRequest(format!("`{field}` is not a valid date: {s:?}")))
}

/// Usage rollups for a tenant. Requires the `tenant:read` permission.
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{id}/usage",
    tag = "tenants",
    params(
        ("id" = String, Path, description = "Tenant identifier"),
        ("from" = Option<String>, Query, description = "Inclusive start UTC date (YYYY-MM-DD)"),
        ("to" = Option<String>, Query, description = "Inclusive end UTC date (YYYY-MM-DD)")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Per-day per-unit usage rollups", body = [UsageEntryDto]),
        (status = 400, description = "Invalid date range", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing tenant:read permission", body = ApiErrorBody),
        (status = 404, description = "Tenant not found", body = ApiErrorBody)
    )
)]
pub async fn get_tenant_usage(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
    Query(q): Query<UsageQuery>,
) -> Result<Json<Vec<UsageEntryDto>>, ApiError> {
    require_permission(&claims, "tenant:read")?;
    if !state.tenants.contains_key(&id) {
        return Err(ApiError::NotFound(format!("Tenant {id} not found")));
    }
    let to = match q.to.as_deref() {
        Some(s) => parse_day(s, "to")?,
        None => crawlkit_engine::metering::day_floor(Utc::now()),
    };
    let from = match q.from.as_deref() {
        Some(s) => parse_day(s, "from")?,
        None => to - chrono::Duration::days(30),
    };
    if from > to {
        return Err(ApiError::BadRequest("`from` must not be after `to`".into()));
    }
    // Storage calls are synchronous SQLite reads: keep them off the runtime.
    let storage = state.storage.clone();
    let rows = tokio::task::spawn_blocking(move || storage.get_usage(&id, from, to))
        .await
        .map_err(|e| ApiError::Internal(format!("storage task panicked: {e}")))?
        .map_err(|e| ApiError::Internal(format!("usage read failed: {e}")))?;
    Ok(Json(
        rows.into_iter()
            .map(|entry| UsageEntryDto {
                unit: entry.unit.as_str().to_string(),
                day_utc: entry.day_utc.to_rfc3339(),
                total: entry.total,
            })
            .collect(),
    ))
}

/// Read a tenant's quota configuration. Requires `tenant:read`.
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{id}/quotas",
    tag = "tenants",
    params(
        ("id" = String, Path, description = "Tenant identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Configured quotas (absent fields are unmetered)", body = QuotaDto),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing tenant:read permission", body = ApiErrorBody),
        (status = 404, description = "Tenant not found", body = ApiErrorBody)
    )
)]
pub async fn get_tenant_quotas(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
) -> Result<Json<QuotaDto>, ApiError> {
    require_permission(&claims, "tenant:read")?;
    if !state.tenants.contains_key(&id) {
        return Err(ApiError::NotFound(format!("Tenant {id} not found")));
    }
    let storage = state.storage.clone();
    let quota = tokio::task::spawn_blocking(move || storage.get_quota(&id))
        .await
        .map_err(|e| ApiError::Internal(format!("storage task panicked: {e}")))?
        .map_err(|e| ApiError::Internal(format!("quota read failed: {e}")))?;
    Ok(Json(QuotaDto::from(&quota)))
}

/// Rejects non-positive quota limits (a limit must be at least 0; `0` is a
/// hard pause of that unit).
fn validate_quota_dto(dto: &QuotaDto) -> Result<(), ApiError> {
    let fields = [
        ("pages", dto.pages),
        ("crawl_started", dto.crawl_started),
        ("findings", dto.findings),
        ("export_bytes", dto.export_bytes),
        ("scan_submitted", dto.scan_submitted),
    ];
    for (name, value) in fields {
        if let Some(v) = value {
            if v < 0 {
                return Err(ApiError::BadRequest(format!(
                    "`{name}` quota must be >= 0, got {v}"
                )));
            }
        }
    }
    Ok(())
}

/// Set a tenant's quota configuration. Requires `tenant:write`.
///
/// This is the commercial control from ADR-017 §3: absent fields are
/// unmetered; `0` is a hard pause of that unit.
#[utoipa::path(
    put,
    path = "/api/v1/tenants/{id}/quotas",
    tag = "tenants",
    params(
        ("id" = String, Path, description = "Tenant identifier")
    ),
    request_body = QuotaDto,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Quotas stored", body = QuotaDto),
        (status = 400, description = "Negative quota limit", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing tenant:write permission or CSRF origin rejected", body = ApiErrorBody),
        (status = 404, description = "Tenant not found", body = ApiErrorBody)
    )
)]
pub async fn set_tenant_quotas(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
    Json(dto): Json<QuotaDto>,
) -> Result<Json<QuotaDto>, ApiError> {
    require_permission(&claims, "tenant:write")?;
    if !state.tenants.contains_key(&id) {
        return Err(ApiError::NotFound(format!("Tenant {id} not found")));
    }
    validate_quota_dto(&dto)?;
    let quota = crawlkit_engine::metering::Quota::from(dto);
    let storage = state.storage.clone();
    let tenant_id = id.clone();
    tokio::task::spawn_blocking(move || storage.set_quota(&tenant_id, &quota))
        .await
        .map_err(|e| ApiError::Internal(format!("storage task panicked: {e}")))?
        .map_err(|e| ApiError::Internal(format!("quota write failed: {e}")))?;
    state.audit_trail.record_tenant(
        crawlkit_engine::AuditEventType::TenantUpdated,
        &claims.sub,
        Some(extract_tenant(&claims)),
        &format!("quotas updated for tenant {id}"),
    );
    Ok(Json(QuotaDto::from(&quota)))
}
