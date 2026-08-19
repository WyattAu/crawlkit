use axum::extract::{Extension, State};
use axum::Json;

use crate::auth;
use crate::types::*;

/// List audit events.
///
/// Admins see all events; non-admins see only their own tenant's events.
/// Requires the `audit:read` permission (admin role).
#[utoipa::path(
    get,
    path = "/api/v1/audit",
    tag = "audit",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Audit events visible to the caller", body = [ApiAuditEvent]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing audit:read permission", body = ApiErrorBody)
    )
)]
pub async fn get_audit_events(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Result<Json<Vec<crawlkit_engine::AuditEvent>>, ApiError> {
    require_permission(&claims, "audit:read")?;

    let events = if is_admin(&claims) {
        state.audit_trail.events()
    } else {
        state.audit_trail.events_for_tenant(extract_tenant(&claims))
    };
    Ok(Json(events))
}
