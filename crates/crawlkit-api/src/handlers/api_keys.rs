use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::types::*;

pub async fn create_api_key(
    State(state): State<AppState>,
    Json(req): Json<ApiKeyCreateRequest>,
) -> Result<(StatusCode, Json<ApiKeyResponse>), ApiError> {
    let key = format!("ck_{}", Uuid::new_v4().to_string().replace('-', ""));
    let name = req.name.clone();
    let rpm = req.requests_per_minute;

    let api_key = ApiKey {
        key: key.clone(),
        name: req.name,
        created_at: Utc::now(),
        requests_per_minute: rpm,
    };

    state.api_keys.insert(key.clone(), api_key);

    Ok((
        StatusCode::CREATED,
        Json(ApiKeyResponse {
            key,
            name,
            requests_per_minute: rpm,
        }),
    ))
}

pub async fn list_api_keys(State(state): State<AppState>) -> Json<Vec<ApiKeyResponse>> {
    let keys = state
        .api_keys
        .iter()
        .map(|entry| ApiKeyResponse {
            key: ApiKeyResponse::redacted(&entry.value().key),
            name: entry.value().name.clone(),
            requests_per_minute: entry.value().requests_per_minute,
        })
        .collect();

    Json(keys)
}

pub async fn delete_api_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .api_keys
        .remove(&key)
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::NotFound(format!("API key {key} not found")))
}
