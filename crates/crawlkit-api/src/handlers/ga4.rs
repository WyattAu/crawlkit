//! GA4 integration API (ADR-013) — per-tenant OAuth2 credential management
//! and read-only reporting.
//!
//! Credential model (ADR-013 §2): the OAuth2 *application* credentials
//! (`GA4_CLIENT_ID` / `GA4_CLIENT_SECRET`) are deployment-level configuration;
//! the tenant's *grant* (refresh token) is per-tenant and lives only in the
//! encrypted credential store under the `ga4` connector. Token material is
//! never echoed in responses, never written to the audit trail, and never
//! logged — the no-secret-logging contract shared with the credential store.

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;

use crate::auth;
use crate::credential_store::{Credential, CredentialMetadata};
use crate::types::*;
use crawlkit_engine::AuditEventType;

/// Credential-store connector name for GA4.
pub const GA4_CONNECTOR: &str = "ga4";

/// Deployment-level OAuth2 application credentials (`GA4_CLIENT_ID` /
/// `GA4_CLIENT_SECRET`).
struct DeploymentClientConfig {
    client_id: String,
    client_secret: String,
}

impl DeploymentClientConfig {
    fn from_env() -> Option<Self> {
        let client_id = std::env::var("GA4_CLIENT_ID").ok()?;
        let client_secret = std::env::var("GA4_CLIENT_SECRET").ok()?;
        if client_id.is_empty() || client_secret.is_empty() {
            return None;
        }
        Some(Self {
            client_id,
            client_secret,
        })
    }
}

/// Build a [`Ga4Client`] from deployment env + the tenant's stored refresh
/// token. `property_id` comes from the request (non-secret key identifier).
fn client_for(
    config: &DeploymentClientConfig,
    refresh_token: String,
    property_id: String,
) -> crawlkit_engine::Ga4Client {
    let credentials = crawlkit_engine::Ga4Credentials {
        client_id: config.client_id.clone(),
        client_secret: config.client_secret.clone(),
        refresh_token,
    };
    crawlkit_engine::Ga4Client::new(credentials, property_id)
}

/// Exchange a one-time OAuth2 authorization code for the tenant's refresh
/// token and store it. This is step one of the connector flow (ADR-013 §1):
/// the exchange happens once; subsequent access tokens are minted from the
/// stored refresh token.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/ga4/exchange",
    tag = "integrations",
    request_body = Ga4ExchangeRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 201, description = "Refresh token stored for the tenant", body = Ga4StatusResponse),
        (status = 400, description = "Missing deployment OAuth config, credential store unavailable, empty code, or OAuth rejection", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn exchange_ga4_code(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<Ga4ExchangeRequest>,
) -> Result<(StatusCode, Json<Ga4StatusResponse>), ApiError> {
    if req.code.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "authorization code must not be empty".to_string(),
        ));
    }

    let config = DeploymentClientConfig::from_env().ok_or_else(|| {
        ApiError::BadRequest(
            "GA4 OAuth is not configured on this deployment (GA4_CLIENT_ID / GA4_CLIENT_SECRET)"
                .to_string(),
        )
    })?;

    let tenant = extract_tenant(&claims);
    let store = state.credential_store.as_ref().ok_or_else(|| {
        ApiError::BadRequest("credential store is not configured on this deployment".to_string())
    })?;

    // Temporary client solely for the exchange (no property id yet).
    let exchange_client = client_for(&config, String::new(), "properties/0".to_string());
    // The code exchange does not use the refresh token; grant type differs.
    let refresh = exchange_client
        .exchange_code(req.code.trim(), &req.redirect_uri)
        .await
        .map_err(|e| ApiError::BadRequest(format!("GA4 OAuth exchange failed: {e}")))?;

    store
        .put(
            tenant,
            Credential {
                secret: refresh.token,
                metadata: CredentialMetadata {
                    connector: GA4_CONNECTOR.to_string(),
                    identifiers: req
                        .property_id
                        .as_ref()
                        .map(|p| {
                            std::collections::HashMap::from([(
                                "property_id".to_string(),
                                p.clone(),
                            )])
                        })
                        .unwrap_or_default(),
                    created_at: Utc::now().to_rfc3339(),
                },
            },
        )
        .map_err(|e| ApiError::BadRequest(format!("failed to store GA4 credential: {e}")))?;

    // Audit records the event, never the token.
    state.audit_trail.record_tenant(
        AuditEventType::ConfigChanged,
        &claims.sub,
        Some(tenant),
        "ga4 credentials stored (authorization code exchanged)",
    );

    Ok((StatusCode::CREATED, Json(ga4_status(store, tenant))))
}

/// Store a refresh token directly (for tenants completing OAuth out of
/// band). Prefer `POST .../exchange` when crawlkit performs the exchange.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/ga4/credentials",
    tag = "integrations",
    request_body = Ga4StoreCredentialsRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 201, description = "Refresh token stored for the tenant", body = Ga4StatusResponse),
        (status = 400, description = "Credential store unavailable or empty token", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn store_ga4_credentials(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<Ga4StoreCredentialsRequest>,
) -> Result<(StatusCode, Json<Ga4StatusResponse>), ApiError> {
    if req.refresh_token.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "refresh_token must not be empty".to_string(),
        ));
    }

    let tenant = extract_tenant(&claims);
    let store = state.credential_store.as_ref().ok_or_else(|| {
        ApiError::BadRequest("credential store is not configured on this deployment".to_string())
    })?;

    store
        .put(
            tenant,
            Credential {
                secret: req.refresh_token.trim().to_string(),
                metadata: CredentialMetadata {
                    connector: GA4_CONNECTOR.to_string(),
                    identifiers: req
                        .property_id
                        .as_ref()
                        .map(|p| {
                            std::collections::HashMap::from([(
                                "property_id".to_string(),
                                p.clone(),
                            )])
                        })
                        .unwrap_or_default(),
                    created_at: Utc::now().to_rfc3339(),
                },
            },
        )
        .map_err(|e| ApiError::BadRequest(format!("failed to store GA4 credential: {e}")))?;

    state.audit_trail.record_tenant(
        AuditEventType::ConfigChanged,
        &claims.sub,
        Some(tenant),
        "ga4 credentials stored (direct)",
    );

    Ok((StatusCode::CREATED, Json(ga4_status(store, tenant))))
}

/// GA4 integration status: whether a grant exists and the non-secret
/// property identifier. Never includes token material.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/ga4/status",
    tag = "integrations",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "GA4 integration status", body = Ga4StatusResponse),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn ga4_integration_status(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Json<Ga4StatusResponse> {
    let tenant = extract_tenant(&claims);
    match state.credential_store.as_ref() {
        Some(store) => Json(ga4_status(store, tenant)),
        None => Json(Ga4StatusResponse {
            connected: false,
            property_id: None,
            credential_store_available: false,
        }),
    }
}

/// Disconnect GA4: delete the tenant's stored refresh token. Audited
/// without token material.
#[utoipa::path(
    delete,
    path = "/api/v1/integrations/ga4/credentials",
    tag = "integrations",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 204, description = "GA4 credentials deleted"),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn delete_ga4_credentials(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Result<StatusCode, ApiError> {
    let tenant = extract_tenant(&claims);
    if let Some(store) = state.credential_store.as_ref() {
        store.delete(tenant, GA4_CONNECTOR);
    }
    state.audit_trail.record_tenant(
        AuditEventType::ConfigChanged,
        &claims.sub,
        Some(tenant),
        "ga4 credentials deleted (disconnected)",
    );
    Ok(StatusCode::NO_CONTENT)
}

/// Run a read-only GA4 `runReport` engagement summary for the tenant
/// (ADR-013 §3). Uses the deployment OAuth app credentials and the tenant's
/// stored refresh token; access tokens are minted in-memory per call and
/// never persisted.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/ga4/report",
    tag = "integrations",
    request_body = Ga4ReportRequestBody,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "GA4 report rows", body = Ga4ReportResponse),
        (status = 400, description = "Missing OAuth config, credential store, stored grant, or GA4 API rejection", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn run_ga4_report(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<Ga4ReportRequestBody>,
) -> Result<Json<Ga4ReportResponse>, ApiError> {
    let config = DeploymentClientConfig::from_env().ok_or_else(|| {
        ApiError::BadRequest(
            "GA4 OAuth is not configured on this deployment (GA4_CLIENT_ID / GA4_CLIENT_SECRET)"
                .to_string(),
        )
    })?;

    let tenant = extract_tenant(&claims);
    let store = state.credential_store.as_ref().ok_or_else(|| {
        ApiError::BadRequest("credential store is not configured on this deployment".to_string())
    })?;

    let credential = store.get(tenant, GA4_CONNECTOR).map_err(|_| {
        ApiError::BadRequest(
            "no GA4 credentials stored for this tenant — connect first".to_string(),
        )
    })?;

    let property_id = credential
        .metadata
        .identifiers
        .get("property_id")
        .cloned()
        .or(req.property_id.clone())
        .ok_or_else(|| {
            ApiError::BadRequest(
                "property_id required (not stored with credentials and not provided)".to_string(),
            )
        })?;

    let request = crawlkit_engine::Ga4ReportRequest::engagement_summary(
        &req.start_date,
        &req.end_date,
        req.limit.unwrap_or(1000).clamp(1, 100_000),
    );

    let mut client = client_for(&config, credential.secret, property_id.clone());
    let report = client
        .run_report(&request)
        .await
        .map_err(|e| ApiError::BadRequest(format!("GA4 report failed: {e}")))?;

    Ok(Json(Ga4ReportResponse {
        property_id,
        start_date: req.start_date,
        end_date: req.end_date,
        row_count: report.row_count,
        rows: report
            .rows
            .iter()
            .map(crate::types::Ga4RowDto::from)
            .collect(),
    }))
}

/// Non-secret status snapshot for responses.
fn ga4_status(store: &crate::credential_store::CredentialStore, tenant: &str) -> Ga4StatusResponse {
    match store.get(tenant, GA4_CONNECTOR) {
        Ok(credential) => Ga4StatusResponse {
            connected: true,
            property_id: credential.metadata.identifiers.get("property_id").cloned(),
            credential_store_available: true,
        },
        Err(_) => Ga4StatusResponse {
            connected: false,
            property_id: None,
            credential_store_available: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_config_requires_both_variables() {
        // Env-dependent; both unset in CI. from_env must be None, and the
        // handlers must surface a clear 400 rather than panic.
        std::env::remove_var("GA4_CLIENT_ID");
        std::env::remove_var("GA4_CLIENT_SECRET");
        assert!(DeploymentClientConfig::from_env().is_none());
    }

    #[test]
    fn deployment_config_rejects_empty_values() {
        std::env::set_var("GA4_CLIENT_ID", "");
        std::env::set_var("GA4_CLIENT_SECRET", "x");
        assert!(DeploymentClientConfig::from_env().is_none());
        std::env::remove_var("GA4_CLIENT_ID");
        std::env::remove_var("GA4_CLIENT_SECRET");
    }
}
