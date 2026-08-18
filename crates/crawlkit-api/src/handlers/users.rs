use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::auth;
use crate::types::*;

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
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if !is_admin(&claims) {
        let target_user = state
            .auth
            .find_user_by_id(&id)
            .ok_or_else(|| ApiError::NotFound(format!("User {id} not found")))?;
        if target_user.tenant_id != extract_tenant(&claims) {
            return Err(ApiError::NotFound(format!("User {id} not found")));
        }
    }

    if state.auth.delete_user(&id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("User {id} not found")))
    }
}
