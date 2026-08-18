use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;

use crate::types::*;

pub async fn list_tenants(State(state): State<AppState>) -> Json<Vec<Tenant>> {
    let tenants: Vec<Tenant> = state
        .tenants
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    Json(tenants)
}

pub async fn create_tenant(
    State(state): State<AppState>,
    Json(input): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Tenant>), ApiError> {
    if state.tenants.contains_key(&input.id) {
        return Err(ApiError::BadRequest(format!(
            "Tenant '{}' already exists",
            input.id
        )));
    }

    let tenant = Tenant {
        id: input.id,
        name: input.name,
        created_at: Utc::now(),
    };

    state.tenants.insert(tenant.id.clone(), tenant.clone());
    Ok((StatusCode::CREATED, Json(tenant)))
}

pub async fn get_tenant(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Tenant>, ApiError> {
    state
        .tenants
        .get(&id)
        .map(|entry| Json(entry.value().clone()))
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {id} not found")))
}

pub async fn delete_tenant(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .tenants
        .remove(&id)
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {id} not found")))
}
