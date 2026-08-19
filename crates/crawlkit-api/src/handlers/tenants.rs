use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;

use crate::auth;
use crate::types::*;

/// List tenants. Requires the `tenant:read` permission.
#[utoipa::path(
    get,
    path = "/api/v1/tenants",
    tag = "tenants",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "All tenants", body = [Tenant]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing tenant:read permission", body = ApiErrorBody)
    )
)]
pub async fn list_tenants(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Result<Json<Vec<Tenant>>, ApiError> {
    require_permission(&claims, "tenant:read")?;
    let tenants: Vec<Tenant> = state
        .tenants
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    Ok(Json(tenants))
}

/// Create a tenant. Requires the `tenant:write` permission.
#[utoipa::path(
    post,
    path = "/api/v1/tenants",
    tag = "tenants",
    request_body = CreateTenantRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 201, description = "Tenant created", body = Tenant),
        (status = 400, description = "Tenant id already exists", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing tenant:write permission or CSRF origin rejected", body = ApiErrorBody)
    )
)]
pub async fn create_tenant(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(input): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Tenant>), ApiError> {
    require_permission(&claims, "tenant:write")?;

    if state.tenants.contains_key(&input.id) {
        return Err(ApiError::BadRequest(format!(
            "Tenant '{}' already exists",
            input.id
        )));
    }

    let input_id = input.id.clone();
    let tenant = Tenant {
        id: input.id,
        name: input.name,
        created_at: Utc::now(),
    };

    state.tenants.insert(tenant.id.clone(), tenant.clone());
    if let Some(persistence) = &state.persistence {
        if let Err(e) = persistence.save_tenant(&tenant).await {
            tracing::error!("Failed to persist tenant {}: {e}", tenant.id);
        }
    }
    state.audit_trail.record_tenant(
        crawlkit_engine::AuditEventType::TenantCreated,
        &claims.sub,
        Some(extract_tenant(&claims)),
        &format!("tenant created: {}", input_id),
    );
    Ok((StatusCode::CREATED, Json(tenant)))
}

/// Get a tenant by id. Requires the `tenant:read` permission.
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{id}",
    tag = "tenants",
    params(
        ("id" = String, Path, description = "Tenant identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Tenant details", body = Tenant),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing tenant:read permission", body = ApiErrorBody),
        (status = 404, description = "Tenant not found", body = ApiErrorBody)
    )
)]
pub async fn get_tenant(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
) -> Result<Json<Tenant>, ApiError> {
    require_permission(&claims, "tenant:read")?;
    state
        .tenants
        .get(&id)
        .map(|entry| Json(entry.value().clone()))
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {id} not found")))
}

/// Delete a tenant. Requires the `tenant:write` permission.
#[utoipa::path(
    delete,
    path = "/api/v1/tenants/{id}",
    tag = "tenants",
    params(
        ("id" = String, Path, description = "Tenant identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 204, description = "Tenant deleted"),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing tenant:write permission or CSRF origin rejected", body = ApiErrorBody),
        (status = 404, description = "Tenant not found", body = ApiErrorBody)
    )
)]
pub async fn delete_tenant(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_permission(&claims, "tenant:write")?;
    let removed = state.tenants.remove(&id);
    if let Some((removed_id, _)) = &removed {
        if let Some(persistence) = &state.persistence {
            if let Err(e) = persistence.delete_tenant(removed_id).await {
                tracing::error!("Failed to persist tenant deletion {removed_id}: {e}");
            }
        }
        state.audit_trail.record_tenant(
            crawlkit_engine::AuditEventType::TenantDeleted,
            &claims.sub,
            Some(extract_tenant(&claims)),
            &format!("tenant deleted: {removed_id}"),
        );
    }
    removed
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {id} not found")))
}
