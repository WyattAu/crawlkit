use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;

use crate::auth;
use crate::types::*;

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let user = state
        .auth
        .find_user(&req.email)
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    if !user.enabled {
        return Err(ApiError::Unauthorized("Account disabled".to_string()));
    }

    if !state
        .auth
        .verify_password(&req.password, &user.password_hash)
    {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let token = state
        .auth
        .generate_token(&user)
        .map_err(|e| ApiError::Internal(format!("Failed to generate token: {e}")))?;

    let claims = state
        .auth
        .validate_token(&token)
        .map_err(|e| ApiError::Internal(format!("Failed to validate token: {e}")))?;

    let now = Utc::now();
    let session = SessionInfo {
        jti: claims.jti.clone(),
        user_id: user.id.clone(),
        created_at: now,
        expires_at: now + chrono::Duration::hours(1),
        revoked: false,
    };
    state.sessions.insert(claims.jti, session);

    Ok(Json(LoginResponse {
        token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
            tenant_id: user.tenant_id,
            roles: user.roles,
            enabled: user.enabled,
        },
    }))
}

#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

pub async fn refresh_token(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
) -> Result<Json<LoginResponse>, ApiError> {
    let user = state
        .auth
        .find_user_by_id(&claims.sub)
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    if !user.enabled {
        return Err(ApiError::Unauthorized("Account disabled".to_string()));
    }

    let token = state
        .auth
        .generate_token(&user)
        .map_err(|e| ApiError::Internal(format!("Failed to generate token: {e}")))?;

    Ok(Json(LoginResponse {
        token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
            tenant_id: user.tenant_id,
            roles: user.roles,
            enabled: user.enabled,
        },
    }))
}

pub async fn get_me(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
) -> Result<Json<UserResponse>, ApiError> {
    let user = state
        .auth
        .find_user_by_id(&claims.sub)
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        name: user.name,
        tenant_id: user.tenant_id,
        roles: user.roles,
        enabled: user.enabled,
    }))
}

pub async fn oidc_authorize(State(state): State<AppState>) -> Result<Response, ApiError> {
    let oidc = state
        .oidc
        .as_ref()
        .ok_or_else(|| ApiError::Internal("OIDC not configured".to_string()))?;

    let state_token = uuid::Uuid::new_v4().to_string();
    state.oidc_states.insert(state_token.clone(), Utc::now());

    let url = oidc.authorization_url(&state_token);
    let parsed_url = url::Url::parse(&url)
        .map_err(|e| ApiError::Internal(format!("Invalid authorization URL: {e}")))?;

    Ok((
        StatusCode::FOUND,
        [("location", parsed_url.as_str().to_string())],
    )
        .into_response())
}

#[derive(serde::Deserialize)]
pub struct OidcCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

pub async fn oidc_callback(
    State(state): State<AppState>,
    Query(params): Query<OidcCallbackParams>,
) -> Result<Json<LoginResponse>, ApiError> {
    if let Some(error) = &params.error {
        return Err(ApiError::BadRequest(format!(
            "OIDC provider error: {error}"
        )));
    }

    let code = params
        .code
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Missing authorization code".to_string()))?;

    let state_token = params
        .state
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Missing state parameter".to_string()))?;

    let created_at = state
        .oidc_states
        .get(state_token)
        .ok_or_else(|| ApiError::BadRequest("Invalid state parameter".to_string()))?;

    let state_ttl = chrono::Duration::minutes(10);
    if Utc::now() - *created_at.value() > state_ttl {
        state.oidc_states.remove(state_token);
        return Err(ApiError::BadRequest(
            "State parameter expired, please try again".to_string(),
        ));
    }
    state.oidc_states.remove(state_token);

    let oidc = state
        .oidc
        .as_ref()
        .ok_or_else(|| ApiError::Internal("OIDC not configured".to_string()))?;

    let tokens = oidc
        .exchange_code(code)
        .await
        .map_err(|e| ApiError::Internal(format!("Token exchange failed: {e}")))?;

    let user_info = oidc
        .get_user_info(&tokens.access_token)
        .await
        .map_err(|e| ApiError::Internal(format!("User info fetch failed: {e}")))?;

    let user = state
        .auth
        .find_user_by_id(&user_info.sub)
        .unwrap_or_else(|| {
            let roles = map_oidc_roles(&user_info.roles, &user_info.groups);
            let new_user = auth::User {
                id: user_info.sub.clone(),
                email: user_info.email.unwrap_or_default(),
                name: user_info.name.unwrap_or_default(),
                password_hash: String::new(),
                tenant_id: "default".to_string(),
                roles,
                enabled: true,
            };
            state.auth.add_user(new_user.clone());
            tracing::info!("Provisioned new OIDC user: {}", new_user.id);
            new_user
        });

    let token = state
        .auth
        .generate_token(&user)
        .map_err(|e| ApiError::Internal(format!("Failed to generate token: {e}")))?;

    Ok(Json(LoginResponse {
        token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
            tenant_id: user.tenant_id,
            roles: user.roles,
            enabled: user.enabled,
        },
    }))
}

pub async fn list_sessions(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Json<Vec<SessionResponse>> {
    let user_id = &claims.sub;
    let sessions: Vec<SessionResponse> = state
        .sessions
        .iter()
        .filter(|entry| entry.value().user_id == *user_id)
        .map(|entry| {
            let s = entry.value();
            SessionResponse {
                jti: s.jti.clone(),
                user_id: s.user_id.clone(),
                created_at: s.created_at,
                expires_at: s.expires_at,
                revoked: s.revoked,
            }
        })
        .collect();
    Json(sessions)
}

pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<RevokeSessionRequest>,
) -> Result<StatusCode, ApiError> {
    let user_id = &claims.sub;
    let mut session = state
        .sessions
        .get_mut(&req.jti)
        .ok_or_else(|| ApiError::NotFound(format!("Session {} not found", req.jti)))?;

    if session.user_id != *user_id && !is_admin(&claims) {
        return Err(ApiError::NotFound(format!("Session {} not found", req.jti)));
    }

    session.revoked = true;
    Ok(StatusCode::NO_CONTENT)
}
