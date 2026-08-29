use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::auth;
use crate::types::*;
use crawlkit_engine::AuditEventType;

/// List users visible to the caller (own tenant, or all for admins).
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "users",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Users visible to the caller", body = [UserResponse]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn list_users(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Json<Vec<UserResponse>> {
    let tenant = extract_tenant(&claims);
    let admin = is_admin(&claims);
    let users: Vec<UserResponse> = state
        .auth
        .list_users()
        .into_iter()
        .filter(|u| admin || u.tenant_id == tenant)
        .map(|u| UserResponse {
            id: u.id,
            email: u.email,
            name: u.name,
            tenant_id: u.tenant_id,
            roles: u.roles,
            enabled: u.enabled,
        })
        .collect();
    Json(users)
}

/// Create a user in the caller's tenant. Admin only.
#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "users",
    request_body = CreateUserRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 400, description = "Weak password or duplicate email", body = ApiErrorBody),
        (status = 401, description = "Caller is not an admin, or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "CSRF origin validation failed", body = ApiErrorBody)
    )
)]
pub async fn create_user(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    if !is_admin(&claims) {
        return Err(ApiError::Unauthorized(
            "Only admins can create users".to_string(),
        ));
    }

    if let Err(reason) = auth::AuthManager::validate_password(&req.password) {
        return Err(ApiError::BadRequest(reason));
    }

    if state.auth.find_user(&req.email).is_some() {
        return Err(ApiError::BadRequest(
            "User with this email already exists".to_string(),
        ));
    }

    let password_hash = state
        .auth
        .hash_password(&req.password)
        .map_err(|e| ApiError::Internal(format!("Failed to hash password: {e}")))?;
    let user_id = Uuid::new_v4().to_string();
    let tenant_id = extract_tenant(&claims).to_string();
    let email = req.email.clone();
    let response = UserResponse {
        id: user_id.clone(),
        email: req.email.clone(),
        name: req.name.clone(),
        tenant_id: tenant_id.clone(),
        roles: req.roles.clone(),
        enabled: true,
    };

    state.auth.add_user(auth::User {
        id: user_id,
        email: req.email,
        name: req.name,
        password_hash,
        tenant_id,
        roles: req.roles,
        enabled: true,
    });

    // Write-through so the account survives restarts.
    if let Some(user) = state.auth.find_user_by_id(&response.id) {
        if let Some(persistence) = &state.persistence {
            if let Err(e) = persistence.save_user(&user).await {
                tracing::error!("Failed to persist user {}: {e}", response.id);
            }
        }
    }

    state.audit_trail.record_tenant(
        AuditEventType::UserCreated,
        &claims.sub,
        Some(extract_tenant(&claims)),
        &format!("user created: {} ({})", response.id, email),
    );

    Ok((StatusCode::CREATED, Json(response)))
}

/// Delete a user. Admin only.
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "users",
    params(
        ("id" = String, Path, description = "User identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Caller is not an admin, or CSRF origin rejected", body = ApiErrorBody),
        (status = 404, description = "User not found", body = ApiErrorBody)
    )
)]
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Only admins may delete users. Allowing same-tenant non-admins to
    // delete would permit viewer -> admin privilege escalation.
    if !is_admin(&claims) {
        return Err(ApiError::Forbidden(
            "Only admins can delete users".to_string(),
        ));
    }

    if state.auth.delete_user(&id) {
        if let Some(persistence) = &state.persistence {
            if let Err(e) = persistence.delete_user(&id).await {
                tracing::error!("Failed to persist user deletion {id}: {e}");
            }
        }
        state.audit_trail.record_tenant(
            AuditEventType::UserDeleted,
            &claims.sub,
            Some(extract_tenant(&claims)),
            &format!("user deleted: {id}"),
        );
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("User {id} not found")))
    }
}
