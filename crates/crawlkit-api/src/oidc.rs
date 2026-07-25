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

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // OidcConfig serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_oidc_config_serialization_roundtrip() {
        let config = OidcConfig {
            provider: "google".to_string(),
            client_id: "my-client-id".to_string(),
            client_secret_env: "GOOGLE_CLIENT_SECRET".to_string(),
            discovery_url: "https://accounts.google.com".to_string(),
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            redirect_uri: "http://localhost:4000/callback".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: OidcConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.provider, "google");
        assert_eq!(deserialized.client_id, "my-client-id");
        assert_eq!(deserialized.client_secret_env, "GOOGLE_CLIENT_SECRET");
        assert_eq!(deserialized.scopes.len(), 3);
        assert_eq!(deserialized.redirect_uri, "http://localhost:4000/callback");
    }

    // ---------------------------------------------------------------
    // OidcEndpoints serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_oidc_endpoints_serialization_roundtrip() {
        let endpoints = OidcEndpoints {
            authorization_endpoint: "https://auth.example.com/authorize".to_string(),
            token_endpoint: "https://auth.example.com/token".to_string(),
            userinfo_endpoint: "https://auth.example.com/userinfo".to_string(),
            jwks_uri: "https://auth.example.com/.well-known/jwks.json".to_string(),
        };
        let json = serde_json::to_string(&endpoints).unwrap();
        let deserialized: OidcEndpoints = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.authorization_endpoint,
            "https://auth.example.com/authorize"
        );
        assert_eq!(
            deserialized.token_endpoint,
            "https://auth.example.com/token"
        );
        assert_eq!(
            deserialized.userinfo_endpoint,
            "https://auth.example.com/userinfo"
        );
        assert_eq!(
            deserialized.jwks_uri,
            "https://auth.example.com/.well-known/jwks.json"
        );
    }

    // ---------------------------------------------------------------
    // OidcTokens serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_oidc_tokens_serialization_roundtrip() {
        let tokens = OidcTokens {
            access_token: "access-123".to_string(),
            id_token: "id-456".to_string(),
            refresh_token: Some("refresh-789".to_string()),
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        let deserialized: OidcTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.access_token, "access-123");
        assert_eq!(deserialized.id_token, "id-456");
        assert_eq!(deserialized.refresh_token.as_deref(), Some("refresh-789"));
        assert_eq!(deserialized.expires_in, 3600);
        assert_eq!(deserialized.token_type, "Bearer");
    }

    #[test]
    fn test_oidc_tokens_without_refresh_token() {
        let tokens = OidcTokens {
            access_token: "access-123".to_string(),
            id_token: "id-456".to_string(),
            refresh_token: None,
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        let deserialized: OidcTokens = serde_json::from_str(&json).unwrap();
        assert!(deserialized.refresh_token.is_none());
    }

    // ---------------------------------------------------------------
    // OidcUserInfo serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_oidc_user_info_serialization_roundtrip() {
        let info = OidcUserInfo {
            sub: "user-abc".to_string(),
            email: Some("user@example.com".to_string()),
            name: Some("Test User".to_string()),
            picture: Some("https://example.com/photo.jpg".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: OidcUserInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, "user-abc");
        assert_eq!(deserialized.email.as_deref(), Some("user@example.com"));
        assert_eq!(deserialized.name.as_deref(), Some("Test User"));
    }

    #[test]
    fn test_oidc_user_info_optional_fields() {
        let info = OidcUserInfo {
            sub: "user-abc".to_string(),
            email: None,
            name: None,
            picture: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: OidcUserInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, "user-abc");
        assert!(deserialized.email.is_none());
        assert!(deserialized.name.is_none());
        assert!(deserialized.picture.is_none());
    }

    // ---------------------------------------------------------------
    // OidcManager::new
    // ---------------------------------------------------------------

    #[test]
    fn test_oidc_manager_new() {
        let config = OidcConfig {
            provider: "github".to_string(),
            client_id: "gh-client".to_string(),
            client_secret_env: "GH_SECRET".to_string(),
            discovery_url: "https://github.com".to_string(),
            scopes: vec!["openid".to_string()],
            redirect_uri: "http://localhost/callback".to_string(),
        };
        let manager = OidcManager::new(config.clone());
        assert_eq!(manager.config().provider, "github");
        assert_eq!(manager.config().client_id, "gh-client");
        // Endpoints should be None before discovery
        assert!(manager.endpoints.read().is_none());
    }

    #[test]
    fn test_oidc_manager_config_accessor() {
        let config = OidcConfig {
            provider: "azure".to_string(),
            client_id: "azure-id".to_string(),
            client_secret_env: "AZURE_SECRET".to_string(),
            discovery_url: "https://login.microsoftonline.com".to_string(),
            scopes: vec!["openid".to_string(), "email".to_string()],
            redirect_uri: "http://localhost:4000/callback".to_string(),
        };
        let manager = OidcManager::new(config.clone());
        let returned_config = manager.config();
        assert_eq!(returned_config.provider, "azure");
        assert_eq!(returned_config.scopes.len(), 2);
    }

    // ---------------------------------------------------------------
    // OidcError Display
    // ---------------------------------------------------------------

    #[test]
    fn test_oidc_error_display() {
        let err = OidcError::Discovery("bad discovery".to_string());
        assert!(err.to_string().contains("bad discovery"));

        let err = OidcError::TokenExchange("token fail".to_string());
        assert!(err.to_string().contains("token fail"));

        let err = OidcError::TokenValidation("validation fail".to_string());
        assert!(err.to_string().contains("validation fail"));

        let err = OidcError::UserInfo("userinfo fail".to_string());
        assert!(err.to_string().contains("userinfo fail"));
    }

    // ---------------------------------------------------------------
    // OidcConfig clone
    // ---------------------------------------------------------------

    #[test]
    fn test_oidc_config_clone() {
        let config = OidcConfig {
            provider: "okta".to_string(),
            client_id: "okta-id".to_string(),
            client_secret_env: "OKTA_SECRET".to_string(),
            discovery_url: "https://example.okta.com".to_string(),
            scopes: vec!["openid".to_string()],
            redirect_uri: "http://localhost/callback".to_string(),
        };
        let cloned = config.clone();
        assert_eq!(cloned.provider, "okta");
        assert_eq!(cloned.client_id, "okta-id");
    }

    // ---------------------------------------------------------------
    // OidcEndpoints clone
    // ---------------------------------------------------------------

    #[test]
    fn test_oidc_endpoints_clone() {
        let endpoints = OidcEndpoints {
            authorization_endpoint: "https://auth.example.com/authorize".to_string(),
            token_endpoint: "https://auth.example.com/token".to_string(),
            userinfo_endpoint: "https://auth.example.com/userinfo".to_string(),
            jwks_uri: "https://auth.example.com/jwks".to_string(),
        };
        let cloned = endpoints.clone();
        assert_eq!(
            cloned.authorization_endpoint,
            endpoints.authorization_endpoint
        );
        assert_eq!(cloned.token_endpoint, endpoints.token_endpoint);
    }
}
