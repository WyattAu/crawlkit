use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::auth;
use crate::types::*;
use crawlkit_engine::AuditEventType;

/// Create a new API key. Requires the `apikey:write` permission.
#[utoipa::path(
    post,
    path = "/api/v1/keys",
    tag = "api-keys",
    request_body = ApiKeyCreateRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 201, description = "API key created", body = ApiKeyResponse),
        (status = 400, description = "Invalid requests_per_minute", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing apikey:write permission", body = ApiErrorBody)
    )
)]
pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<ApiKeyCreateRequest>,
) -> Result<(StatusCode, Json<ApiKeyResponse>), ApiError> {
    require_permission(&claims, "apikey:write")?;
    validate_rpm(req.requests_per_minute)?;

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
    if let Some(persistence) = &state.persistence {
        if let Some(existing) = state.api_keys.get(&key) {
            if let Err(e) = persistence.save_api_key(existing.value()).await {
                tracing::error!("Failed to persist API key: {e}");
            }
        }
    }

    state.audit_trail.record_tenant(
        AuditEventType::ApiKeyCreated,
        &claims.sub,
        Some(extract_tenant(&claims)),
        &format!("API key created: {name}"),
    );

    Ok((
        StatusCode::CREATED,
        Json(ApiKeyResponse {
            key,
            name,
            requests_per_minute: rpm,
        }),
    ))
}

/// List API keys (redacted). Requires the `apikey:read` permission.
#[utoipa::path(
    get,
    path = "/api/v1/keys",
    tag = "api-keys",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "API keys (redacted to last 4 characters)", body = [ApiKeyResponse]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing apikey:read permission", body = ApiErrorBody)
    )
)]
pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Result<Json<Vec<ApiKeyResponse>>, ApiError> {
    require_permission(&claims, "apikey:read")?;
    let keys = state
        .api_keys
        .iter()
        .map(|entry| ApiKeyResponse {
            key: ApiKeyResponse::redacted(&entry.value().key),
            name: entry.value().name.clone(),
            requests_per_minute: entry.value().requests_per_minute,
        })
        .collect();

    Ok(Json(keys))
}

/// Delete an API key. Requires the `apikey:write` permission.
#[utoipa::path(
    delete,
    path = "/api/v1/keys/{key}",
    tag = "api-keys",
    params(
        ("key" = String, Path, description = "API key to delete")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 204, description = "API key deleted"),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing apikey:write permission", body = ApiErrorBody),
        (status = 404, description = "API key not found", body = ApiErrorBody)
    )
)]
pub async fn delete_api_key(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(key): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_permission(&claims, "apikey:write")?;
    let removed = state.api_keys.remove(&key);
    if let Some((removed_key, _)) = &removed {
        if let Some(persistence) = &state.persistence {
            if let Err(e) = persistence.delete_api_key(removed_key).await {
                tracing::error!("Failed to persist API key deletion: {e}");
            }
        }
        state.audit_trail.record_tenant(
            AuditEventType::ApiKeyRevoked,
            &claims.sub,
            Some(extract_tenant(&claims)),
            &format!("API key revoked: {key}"),
        );
    }
    removed
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::NotFound(format!("API key {key} not found")))
}
