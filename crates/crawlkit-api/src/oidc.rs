use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// OIDC errors.
#[derive(Debug, Error)]
pub enum OidcError {
    #[error("OIDC discovery failed: {0}")]
    Discovery(String),

    #[error("Token exchange failed: {0}")]
    TokenExchange(String),

    #[error("Token validation failed: {0}")]
    #[allow(dead_code)]
    TokenValidation(String),

    #[error("User info fetch failed: {0}")]
    UserInfo(String),
}

/// OIDC provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// Provider name (e.g., "google", "github", "azure").
    pub provider: String,
    /// Client ID.
    pub client_id: String,
    /// Client secret (from environment variable).
    pub client_secret_env: String,
    /// OIDC discovery URL.
    pub discovery_url: String,
    /// Scopes to request.
    pub scopes: Vec<String>,
    /// Redirect URI after authentication.
    pub redirect_uri: String,
}

/// OIDC provider endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcEndpoints {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub jwks_uri: String,
}

/// OIDC tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokens {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
}

/// OIDC user info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// OIDC manager for handling authentication.
pub struct OidcManager {
    config: OidcConfig,
    endpoints: Arc<RwLock<Option<OidcEndpoints>>>,
    client: reqwest::Client,
}

impl OidcManager {
    /// Create new OIDC manager.
    pub fn new(config: OidcConfig) -> Self {
        Self {
            config,
            endpoints: Arc::new(RwLock::new(None)),
            client: reqwest::Client::new(),
        }
    }

    /// Discover OIDC endpoints.
    pub async fn discover(&self) -> Result<OidcEndpoints, OidcError> {
        let url = format!(
            "{}/.well-known/openid-configuration",
            self.config.discovery_url
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| OidcError::Discovery(e.to_string()))?;

        let discovery: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| OidcError::Discovery(e.to_string()))?;

        let endpoints = OidcEndpoints {
            authorization_endpoint: discovery["authorization_endpoint"]
                .as_str()
                .ok_or_else(|| OidcError::Discovery("Missing authorization_endpoint".into()))?
                .to_string(),
            token_endpoint: discovery["token_endpoint"]
                .as_str()
                .ok_or_else(|| OidcError::Discovery("Missing token_endpoint".into()))?
                .to_string(),
            userinfo_endpoint: discovery["userinfo_endpoint"]
                .as_str()
                .ok_or_else(|| OidcError::Discovery("Missing userinfo_endpoint".into()))?
                .to_string(),
            jwks_uri: discovery["jwks_uri"]
                .as_str()
                .ok_or_else(|| OidcError::Discovery("Missing jwks_uri".into()))?
                .to_string(),
        };

        *self.endpoints.write() = Some(endpoints.clone());
        Ok(endpoints)
    }

    /// Generate authorization URL.
    pub fn authorization_url(&self, state: &str) -> String {
        let scopes = self.config.scopes.join(" ");
        let guard = self.endpoints.read();
        let ep = guard
            .as_ref()
            .map(|e| e.authorization_endpoint.as_str())
            .unwrap_or("");
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            ep,
            self.config.client_id,
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(state),
        )
    }

    /// Exchange authorization code for tokens.
    pub async fn exchange_code(&self, code: &str) -> Result<OidcTokens, OidcError> {
        let client_secret = std::env::var(&self.config.client_secret_env)
            .map_err(|e| OidcError::TokenExchange(format!("Missing env var: {e}")))?;

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.redirect_uri),
            ("client_id", &self.config.client_id),
            ("client_secret", &client_secret),
        ];

        let token_endpoint = self
            .endpoints
            .read()
            .as_ref()
            .ok_or_else(|| OidcError::TokenExchange("Endpoints not discovered".into()))?
            .token_endpoint
            .clone();

        let resp = self
            .client
            .post(&token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| OidcError::TokenExchange(e.to_string()))?;

        let tokens: OidcTokens = resp
            .json()
            .await
            .map_err(|e| OidcError::TokenExchange(e.to_string()))?;

        Ok(tokens)
    }

    /// Get user info from access token.
    pub async fn get_user_info(&self, access_token: &str) -> Result<OidcUserInfo, OidcError> {
        let userinfo_endpoint = self
            .endpoints
            .read()
            .as_ref()
            .ok_or_else(|| OidcError::UserInfo("Endpoints not discovered".into()))?
            .userinfo_endpoint
            .clone();

        let resp = self
            .client
            .get(&userinfo_endpoint)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| OidcError::UserInfo(e.to_string()))?;

        let user_info: OidcUserInfo = resp
            .json()
            .await
            .map_err(|e| OidcError::UserInfo(e.to_string()))?;

        Ok(user_info)
    }

    /// Get config.
    #[allow(dead_code)]
    pub fn config(&self) -> &OidcConfig {
        &self.config
    }
}
