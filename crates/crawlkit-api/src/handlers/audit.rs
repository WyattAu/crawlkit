use axum::extract::State;
use axum::Json;

use crate::types::*;

pub async fn get_audit_events(State(state): State<AppState>) -> Json<Vec<crawlkit_engine::AuditEvent>> {
    Json(state.audit_trail.events())
}
