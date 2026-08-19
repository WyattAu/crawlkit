use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use utoipa::ToSchema;

use crate::auth;
use crate::types::*;
use crawlkit_engine::AuditEventType;

/// Exchange email/password credentials for a JWT.
///
/// Subject to per-email brute-force lockout after repeated failures.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated; returns JWT and user profile", body = LoginResponse),
        (status = 401, description = "Invalid credentials, disabled account, or lockout", body = ApiErrorBody)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    // Brute-force lockout keyed by email (checked before user lookup so
    // timing does not reveal whether the account exists).
    check_login_lockout(&state.login_attempts, &req.email, Utc::now())?;

    let Some(user) = state.auth.find_user(&req.email) else {
        record_login_failure(&state.login_attempts, &req.email, Utc::now());
        state.audit_trail.record(
            AuditEventType::LoginFailed,
            &req.email,
            "login failed: unknown user",
        );
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    };

    if !user.enabled {
        return Err(ApiError::Unauthorized("Account disabled".to_string()));
    }

    if !state
        .auth
        .verify_password(&req.password, &user.password_hash)
    {
        record_login_failure(&state.login_attempts, &req.email, Utc::now());
        state.audit_trail.record(
            AuditEventType::LoginFailed,
            &req.email,
            "login failed: invalid password",
        );
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    clear_login_failures(&state.login_attempts, &req.email);

    state.audit_trail.record_tenant(
        AuditEventType::LoginSucceeded,
        &user.email,
        Some(&user.tenant_id),
        "login succeeded",
    );

    let token = state
        .auth
        .generate_token(&user)
        .map_err(|e| ApiError::Internal(format!("Failed to generate token: {e}")))?;

    let claims = state
        .auth
        .validate_token(&token)
        .map_err(|e| ApiError::Internal(format!("Failed to validate token: {e}")))?;

    register_session(&state, &claims);

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

/// Record a session for server-side revocation tracking.
fn register_session(state: &AppState, claims: &auth::Claims) {
    let now = Utc::now();
    let session = SessionInfo {
        jti: claims.jti.clone(),
        user_id: claims.sub.clone(),
        created_at: now,
        expires_at: now + chrono::Duration::hours(1),
        revoked: false,
    };
    state.sessions.insert(claims.jti.clone(), session);
}

#[derive(serde::Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

/// Refresh an authenticated session, issuing a new JWT.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "auth",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "New token issued", body = LoginResponse),
        (status = 401, description = "Missing, invalid, or revoked credentials", body = ApiErrorBody)
    )
)]
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

    if let Ok(new_claims) = state.auth.validate_token(&token) {
        register_session(&state, &new_claims);
    }

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

/// Return the authenticated user's profile.
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "auth",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Current user profile", body = UserResponse),
        (status = 401, description = "Missing, invalid, or revoked credentials", body = ApiErrorBody),
        (status = 404, description = "User not found", body = ApiErrorBody)
    )
)]
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

/// Start an OIDC authorization-code flow with PKCE.
///
/// Redirects (302) to the configured OIDC provider's authorization endpoint.
#[utoipa::path(
    get,
    path = "/api/v1/auth/oidc/authorize",
    tag = "auth",
    responses(
        (status = 302, description = "Redirect to the OIDC provider authorization endpoint"),
        (status = 500, description = "OIDC not configured or invalid provider URL", body = ApiErrorBody)
    )
)]
pub async fn oidc_authorize(State(state): State<AppState>) -> Result<Response, ApiError> {
    let oidc = state
        .oidc
        .as_ref()
        .ok_or_else(|| ApiError::Internal("OIDC not configured".to_string()))?;

    let state_token = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let pkce = crate::oidc::generate_pkce();

    state.oidc_states.insert(
        state_token.clone(),
        OidcPendingState {
            created_at: Utc::now(),
            code_verifier: pkce.code_verifier.clone(),
            nonce: nonce.clone(),
        },
    );

    let url = oidc.authorization_url(&state_token, &nonce, &pkce.code_challenge);
    let parsed_url = url::Url::parse(&url)
        .map_err(|e| ApiError::Internal(format!("Invalid authorization URL: {e}")))?;

    Ok((
        StatusCode::FOUND,
        [("location", parsed_url.as_str().to_string())],
    )
        .into_response())
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct OidcCallbackParams {
    /// Authorization code returned by the provider.
    pub code: Option<String>,
    /// One-shot state token issued by `oidc_authorize`.
    pub state: Option<String>,
    /// Error indicator when the provider rejected the flow.
    pub error: Option<String>,
}

/// Complete an OIDC authorization-code flow (token exchange + validation).
#[utoipa::path(
    get,
    path = "/api/v1/auth/oidc/callback",
    tag = "auth",
    params(OidcCallbackParams),
    responses(
        (status = 200, description = "Authenticated; returns JWT and user profile", body = LoginResponse),
        (status = 400, description = "Missing/expired state, missing code, or provider error", body = ApiErrorBody),
        (status = 401, description = "Token exchange or id_token validation failed", body = ApiErrorBody),
        (status = 500, description = "OIDC not configured or token generation failed", body = ApiErrorBody)
    )
)]
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

    let pending = state
        .oidc_states
        .get(state_token)
        .ok_or_else(|| ApiError::BadRequest("Invalid state parameter".to_string()))?;

    let state_ttl = chrono::Duration::minutes(10);
    if Utc::now() - pending.created_at > state_ttl {
        drop(pending);
        state.oidc_states.remove(state_token);
        return Err(ApiError::BadRequest(
            "State parameter expired, please try again".to_string(),
        ));
    }
    let pending = pending.value().clone();
    state.oidc_states.remove(state_token);

    let oidc = state
        .oidc
        .as_ref()
        .ok_or_else(|| ApiError::Internal("OIDC not configured".to_string()))?;

    let tokens = oidc
        .exchange_code(code, &pending.code_verifier)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Token exchange failed: {e}")))?;

    // The id_token is the source of identity: signature (JWKS), issuer,
    // audience, expiry, and nonce binding are all enforced before any
    // account is provisioned or a session issued.
    let id_claims = oidc
        .validate_id_token(&tokens.id_token, &pending.nonce)
        .map_err(|e| ApiError::Unauthorized(format!("{e}")))?;

    let user_info = oidc
        .get_user_info(&tokens.access_token)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("User info fetch failed: {e}")))?;

    let sub = id_claims.sub;
    let roles = map_oidc_roles(
        user_info
            .roles
            .iter()
            .chain(id_claims.roles.iter())
            .cloned()
            .collect::<Vec<String>>()
            .as_slice(),
        user_info
            .groups
            .iter()
            .chain(id_claims.groups.iter())
            .cloned()
            .collect::<Vec<String>>()
            .as_slice(),
    );
    let email = id_claims.email.or(user_info.email).unwrap_or_default();
    let name = id_claims.name.or(user_info.name).unwrap_or_default();

    let user = state.auth.find_user_by_id(&sub).unwrap_or_else(|| {
        let new_user = auth::User {
            id: sub.clone(),
            email: email.clone(),
            name: name.clone(),
            password_hash: String::new(),
            tenant_id: "default".to_string(),
            roles: roles.clone(),
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

    if let Ok(new_claims) = state.auth.validate_token(&token) {
        register_session(&state, &new_claims);
    }

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

/// List the authenticated user's active sessions.
#[utoipa::path(
    get,
    path = "/api/v1/sessions",
    tag = "auth",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Sessions belonging to the caller", body = [SessionResponse]),
        (status = 401, description = "Missing, invalid, or revoked credentials", body = ApiErrorBody)
    )
)]
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

/// Revoke a session by its JWT ID (`jti`).
///
/// Users may revoke their own sessions; admins may revoke any session.
/// Mutations require an allowed `Origin`/`Referer` (CSRF protection).
#[utoipa::path(
    post,
    path = "/api/v1/sessions/revoke",
    tag = "auth",
    request_body = RevokeSessionRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 204, description = "Session revoked"),
        (status = 401, description = "Missing, invalid, or revoked credentials", body = ApiErrorBody),
        (status = 403, description = "CSRF origin validation failed", body = ApiErrorBody),
        (status = 404, description = "Session not found (or owned by another non-admin user)", body = ApiErrorBody)
    )
)]
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
    state.audit_trail.record_tenant(
        crawlkit_engine::AuditEventType::SessionRevoked,
        &claims.sub,
        Some(extract_tenant(&claims)),
        &format!("session {jti} revoked", jti = req.jti),
    );
    Ok(StatusCode::NO_CONTENT)
}
