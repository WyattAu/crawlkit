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

    #[error("User info fetch failed: {0}")]
    UserInfo(String),

    #[error("id_token validation failed: {0}")]
    InvalidIdToken(String),
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
    /// `issuer` from the discovery document; pinned into id_token validation.
    pub issuer: String,
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
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

/// Claims extracted from a validated id_token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcIdClaims {
    pub sub: String,
    pub nonce: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

/// PKCE (RFC 7636) challenge pair.
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
}

/// Generate a PKCE S256 challenge pair from 32 random bytes.
pub fn generate_pkce() -> PkceChallenge {
    use rand::RngCore;
    use sha2::{Digest, Sha256};

    let mut entropy = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut entropy);
    let code_verifier = base64url_no_pad(&entropy);
    let digest = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = base64url_no_pad(&digest);
    PkceChallenge {
        code_verifier,
        code_challenge,
    }
}

/// RFC 4648 base64url without padding (no external dependency).
fn base64url_no_pad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        }
    }
    out
}

/// OIDC manager for handling authentication.
pub struct OidcManager {
    config: OidcConfig,
    endpoints: Arc<RwLock<Option<OidcEndpoints>>>,
    jwks: Arc<RwLock<Option<jsonwebtoken::jwk::JwkSet>>>,
    client: reqwest::Client,
}

impl OidcManager {
    /// Create new OIDC manager.
    pub fn new(config: OidcConfig) -> Self {
        Self {
            config,
            endpoints: Arc::new(RwLock::new(None)),
            jwks: Arc::new(RwLock::new(None)),
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
            issuer: discovery["issuer"]
                .as_str()
                .ok_or_else(|| OidcError::Discovery("Missing issuer".into()))?
                .to_string(),
        };

        // Fetch and cache the provider JWKS so id_tokens can be validated.
        let jwks: jsonwebtoken::jwk::JwkSet = self
            .client
            .get(&endpoints.jwks_uri)
            .send()
            .await
            .map_err(|e| OidcError::Discovery(format!("JWKS fetch failed: {e}")))?
            .json()
            .await
            .map_err(|e| OidcError::Discovery(format!("JWKS parse failed: {e}")))?;
        *self.jwks.write() = Some(jwks);

        *self.endpoints.write() = Some(endpoints.clone());
        Ok(endpoints)
    }

    /// Generate authorization URL with PKCE (S256) and nonce bound in.
    pub fn authorization_url(&self, state: &str, nonce: &str, code_challenge: &str) -> String {
        let scopes = self.config.scopes.join(" ");
        let guard = self.endpoints.read();
        let ep = guard
            .as_ref()
            .map(|e| e.authorization_endpoint.as_str())
            .unwrap_or("");
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",
            ep,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(state),
            urlencoding::encode(nonce),
            urlencoding::encode(code_challenge),
        )
    }

    /// Exchange authorization code for tokens (PKCE-aware).
    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<OidcTokens, OidcError> {
        let client_secret = std::env::var(&self.config.client_secret_env)
            .map_err(|e| OidcError::TokenExchange(format!("Missing env var: {e}")))?;

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.redirect_uri),
            ("client_id", &self.config.client_id),
            ("client_secret", &client_secret),
            ("code_verifier", code_verifier),
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

    /// Validate an id_token against the provider's JWKS.
    ///
    /// Enforces signature (via cached JWKS, matched by `kid`), issuer (from
    /// discovery), audience (our client_id), expiry, and — when provided —
    /// the one-shot `nonce` bound into the authorization request.
    pub fn validate_id_token(
        &self,
        id_token: &str,
        expected_nonce: &str,
    ) -> Result<OidcIdClaims, OidcError> {
        let endpoints = self
            .endpoints
            .read()
            .clone()
            .ok_or_else(|| OidcError::InvalidIdToken("Endpoints not discovered".into()))?;
        let jwks = self
            .jwks
            .read()
            .clone()
            .ok_or_else(|| OidcError::InvalidIdToken("JWKS not loaded".into()))?;

        let header = jsonwebtoken::decode_header(id_token)
            .map_err(|e| OidcError::InvalidIdToken(format!("Malformed token header: {e}")))?;

        let jwk = jwks
            .keys
            .iter()
            .find(|k| match &header.kid {
                Some(kid) => k.common.key_id.as_deref() == Some(kid.as_str()),
                None => true,
            })
            .ok_or_else(|| {
                OidcError::InvalidIdToken("No matching signing key in provider JWKS".into())
            })?;

        let algorithm = header.alg;

        let decoding_key = jsonwebtoken::DecodingKey::from_jwk(jwk)
            .map_err(|e| OidcError::InvalidIdToken(format!("Unsupported JWKS key: {e}")))?;

        let mut validation = jsonwebtoken::Validation::new(algorithm);
        validation.set_audience(&[&self.config.client_id]);
        validation.set_issuer(&[&endpoints.issuer]);
        validation.validate_exp = true;

        let token_data = jsonwebtoken::decode::<OidcIdClaims>(id_token, &decoding_key, &validation)
            .map_err(|e| OidcError::InvalidIdToken(format!("{e}")))?;

        let claims = token_data.claims;
        match claims.nonce.as_deref() {
            Some(nonce) if nonce == expected_nonce => Ok(claims),
            _ => Err(OidcError::InvalidIdToken(
                "nonce mismatch: token is not bound to this authorization request".into(),
            )),
        }
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
            issuer: "https://auth.example.com".to_string(),
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
        assert_eq!(deserialized.issuer, "https://auth.example.com");
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
            groups: vec!["engineering".to_string()],
            roles: vec!["admin".to_string()],
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: OidcUserInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, "user-abc");
        assert_eq!(deserialized.email.as_deref(), Some("user@example.com"));
        assert_eq!(deserialized.name.as_deref(), Some("Test User"));
        assert_eq!(deserialized.groups, vec!["engineering".to_string()]);
        assert_eq!(deserialized.roles, vec!["admin".to_string()]);
    }

    #[test]
    fn test_oidc_user_info_optional_fields() {
        let info = OidcUserInfo {
            sub: "user-abc".to_string(),
            email: None,
            name: None,
            picture: None,
            groups: vec![],
            roles: vec![],
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: OidcUserInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, "user-abc");
        assert!(deserialized.email.is_none());
        assert!(deserialized.name.is_none());
        assert!(deserialized.picture.is_none());
        assert!(deserialized.groups.is_empty());
        assert!(deserialized.roles.is_empty());
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
        let manager = OidcManager::new(config);
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
        let manager = OidcManager::new(config);
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
        let cloned = config;
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
            issuer: "https://auth.example.com".to_string(),
        };
        let cloned = endpoints.clone();
        assert_eq!(
            cloned.authorization_endpoint,
            endpoints.authorization_endpoint
        );
        assert_eq!(cloned.token_endpoint, endpoints.token_endpoint);
    }

    // ---------------------------------------------------------------
    // PKCE + base64url helpers
    // ---------------------------------------------------------------

    #[test]
    fn test_base64url_no_pad_known_vectors() {
        // RFC 4648 test vectors (base64url variant, unpadded).
        assert_eq!(base64url_no_pad(b""), "");
        assert_eq!(base64url_no_pad(b"f"), "Zg");
        assert_eq!(base64url_no_pad(b"fo"), "Zm8");
        assert_eq!(base64url_no_pad(b"foo"), "Zm9v");
        assert_eq!(base64url_no_pad(b"foob"), "Zm9vYg");
        assert_eq!(base64url_no_pad(b"fooba"), "Zm9vYmE");
        assert_eq!(base64url_no_pad(b"foobar"), "Zm9vYmFy");
        // URL-safe alphabet: 0xfb 0xff produces '+' '/' in standard base64.
        assert_eq!(base64url_no_pad(&[0xfb, 0xff]), "-_8");
    }

    #[test]
    fn test_generate_pkce_produces_s256_consistent_pair() {
        use sha2::{Digest, Sha256};
        let pkce = generate_pkce();
        assert_eq!(pkce.code_verifier.len(), 43); // 32 bytes -> 43 base64url chars
        let digest = Sha256::digest(pkce.code_verifier.as_bytes());
        assert_eq!(pkce.code_challenge, base64url_no_pad(&digest));
    }

    #[test]
    fn test_generate_pkce_is_non_deterministic() {
        let a = generate_pkce();
        let b = generate_pkce();
        assert_ne!(a.code_verifier, b.code_verifier);
    }

    #[test]
    fn test_authorization_url_includes_pkce_and_nonce() {
        let manager = OidcManager::new(OidcConfig {
            provider: "test".to_string(),
            client_id: "client id/&".to_string(),
            client_secret_env: "SECRET".to_string(),
            discovery_url: "https://idp.example.com".to_string(),
            scopes: vec!["openid".to_string(), "email".to_string()],
            redirect_uri: "http://localhost:4000/cb".to_string(),
        });
        // parking_lot RwLock::write takes &self, so no `mut` binding needed.
        *manager.endpoints.write() = Some(OidcEndpoints {
            authorization_endpoint: "https://idp.example.com/authorize".to_string(),
            token_endpoint: "https://idp.example.com/token".to_string(),
            userinfo_endpoint: "https://idp.example.com/userinfo".to_string(),
            jwks_uri: "https://idp.example.com/jwks".to_string(),
            issuer: "https://idp.example.com".to_string(),
        });
        let url = manager.authorization_url("state123", "nonce456", "challenge789");
        assert!(url.contains("code_challenge=challenge789"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("nonce=nonce456"));
        assert!(url.contains("state=state123"));
        // client_id must be URL-encoded (no raw spaces or ampersands).
        assert!(url.contains("client_id=client%20id%2F%26"));
    }

    #[test]
    fn test_validate_id_token_rejects_when_not_discovered() {
        let manager = OidcManager::new(OidcConfig {
            provider: "test".to_string(),
            client_id: "cid".to_string(),
            client_secret_env: "SECRET".to_string(),
            discovery_url: "https://idp.example.com".to_string(),
            scopes: vec!["openid".to_string()],
            redirect_uri: "http://localhost:4000/cb".to_string(),
        });
        let result = manager.validate_id_token("x.y.z", "nonce");
        assert!(matches!(result, Err(OidcError::InvalidIdToken(_))));
    }
}
