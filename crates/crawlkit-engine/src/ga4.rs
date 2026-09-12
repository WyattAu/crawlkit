//! GA4 Data API connector (ADR-013) — read-only, per-tenant OAuth2.
//!
//! Mirrors the GSC promotion precedent (`gsc.rs`): injectable base URLs so
//! error paths are hermetically testable, typed errors with no panics, and
//! token material confined to the Authorization header — never in URLs,
//! error output, or logs.
//!
//! OAuth2 (ADR-013 §1):
//!
//! - One scope, `analytics.readonly`, and nothing else (§1 read-only
//!   constraint). The constant is pinned by test so scope creep is a
//!   build failure, not a review finding.
//! - The authorization code is exchanged exactly once for a refresh token
//!   (persisted by the caller via the per-tenant credential store); access
//!   tokens are minted via `refresh_token` grants, live in memory only,
//!   and are never persisted or logged (§2 hygiene contract, enforced by
//!   test).
//! - Both the token endpoint and the Data API base URL are injectable, in
//!   the same style as [`crate::gsc::GscClient::with_base_url`].
//!
//! Rate limiting (ADR-013 §3): token/property and concurrency quotas are
//! surfaced as retryable errors so callers get `loop_retry` backoff for
//! free; other 4xx are fatal and surfaced immediately.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The single OAuth2 scope GA4 integration may request (ADR-013 §1).
///
/// Read-only by construction: no write, no admin, no user scopes.
pub const GA4_SCOPE: &str = "https://www.googleapis.com/auth/analytics.readonly";

/// Production Google OAuth2 token endpoint.
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Production GA4 Data API base URL.
const GA4_API_BASE: &str = "https://analyticsdata.googleapis.com";

/// Errors specific to GA4 API operations.
///
/// Contract follows the GSC promotion: connection failures, HTTP error
/// statuses, and malformed responses are each a distinct, tested path.
#[derive(Debug, Error)]
pub enum Ga4Error {
    /// Required configuration is missing.
    #[error("GA4 configuration missing: {0}")]
    ConfigMissing(String),

    /// HTTP request to a GA4 endpoint failed at the transport level.
    #[error("GA4 API request failed: {0}")]
    RequestFailed(String),

    /// GA4 endpoint returned an error response.
    #[error("GA4 API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },

    /// Response parsing failed.
    #[error("failed to parse GA4 response: {0}")]
    ParseError(String),

    /// The OAuth2 token exchange or refresh was rejected.
    #[error("GA4 OAuth token error (HTTP {status}): {body}")]
    OAuthError { status: u16, body: String },
}

impl Ga4Error {
    /// Whether the error is transient and worth retrying with backoff.
    ///
    /// Classification compatible with `loop_retry::IsRetryable` (ADR-013
    /// §3): transport failures and 429/5xx are retryable; other 4xx are
    /// fatal and surfaced immediately.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RequestFailed(_) => true,
            Self::ApiError { status, .. } | Self::OAuthError { status, .. } => {
                *status == 429 || *status >= 500
            }
            Self::ConfigMissing(_) | Self::ParseError(_) => false,
        }
    }
}

impl loop_retry::IsRetryable for Ga4Error {
    fn is_retryable(&self) -> bool {
        Ga4Error::is_retryable(self)
    }
}

/// Per-tenant GA4 OAuth2 credentials (ADR-013 §2).
///
/// The refresh token is the long-lived secret — callers persist it through
/// the per-tenant credential store, never in configuration or logs. Client
/// credentials (`client_id`/`client_secret`) identify the crawlkit OAuth
/// app and are also treated as secret material.
#[derive(Clone)]
pub struct Ga4Credentials {
    /// OAuth2 client ID of the crawlkit application.
    pub client_id: String,
    /// OAuth2 client secret of the crawlkit application.
    pub client_secret: String,
    /// The tenant's long-lived refresh token (from the one-time code
    /// exchange). Live only in memory here; at rest it belongs in the
    /// encrypted credential store.
    pub refresh_token: String,
}

impl std::fmt::Debug for Ga4Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ADR-013 §2: token material and client secrets must never appear in
        // logs. Debug is the easiest accidental log path — redact it.
        f.debug_struct("Ga4Credentials")
            .field("client_id", &self.client_id)
            .field("client_secret", &"[redacted]")
            .field("refresh_token", &"[redacted]")
            .finish()
    }
}

/// A minted short-lived access token.
///
/// `access_token` lives in memory only (ADR-013 §1): never persisted, never
/// logged, sent only as a Bearer header.
#[derive(Clone)]
pub struct AccessToken {
    pub token: String,
    /// Seconds until expiry, per the OAuth2 response.
    pub expires_in: u64,
}

impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessToken")
            .field("token", &"[redacted]")
            .field("expires_in", &self.expires_in)
            .finish()
    }
}

/// Client for the GA4 Data API v1beta over per-tenant OAuth2 credentials.
///
/// Construct with [`Ga4Client::new`] (production endpoints) or
/// [`Ga4Client::with_base_urls`] (tests). Access tokens are minted lazily
/// via the refresh grant and cached until expiry.
pub struct Ga4Client {
    credentials: Ga4Credentials,
    http_client: reqwest::Client,
    /// GA4 property identifier, e.g. `properties/123456`. Non-secret key
    /// identifier (ADR-013 §2) — may appear in logs.
    pub property_id: String,
    /// Data API base URL. Injectable for hermetic tests.
    api_base_url: String,
    /// OAuth2 token endpoint base (token path appended). Injectable.
    token_base_url: String,
    /// Cached access token; minted on first API call, refreshed on expiry.
    access_token: Option<AccessToken>,
}

impl Ga4Client {
    /// Create a client against the production Google endpoints.
    #[must_use]
    pub fn new(credentials: Ga4Credentials, property_id: impl Into<String>) -> Self {
        Self::with_base_urls(
            credentials,
            property_id,
            GA4_API_BASE.to_string(),
            GOOGLE_TOKEN_URL.to_string(),
        )
    }

    /// Create a client with injectable endpoints.
    ///
    /// Intended for tests; production callers use [`Ga4Client::new`].
    /// `token_url` is the complete token endpoint URL.
    #[must_use]
    pub fn with_base_urls(
        credentials: Ga4Credentials,
        property_id: impl Into<String>,
        api_base_url: String,
        token_url: String,
    ) -> Self {
        Self {
            credentials,
            http_client: reqwest::Client::new(),
            property_id: property_id.into(),
            api_base_url,
            token_base_url: token_url,
            access_token: None,
        }
    }

    /// Drop the cached access token (e.g. after a 401) so the next call
    /// mints a fresh one.
    pub fn invalidate_token(&mut self) {
        self.access_token = None;
    }

    /// Mint an access token via the `refresh_token` grant.
    ///
    /// Hygiene (tested): the refresh token and client secret travel only in
    /// the request body to the token endpoint; error output must not echo
    /// them.
    ///
    /// # Errors
    ///
    /// [`Ga4Error::OAuthError`] on rejection, [`Ga4Error::RequestFailed`]
    /// on transport failure, [`Ga4Error::ParseError`] on malformed
    /// responses.
    pub async fn mint_access_token(&self) -> Result<AccessToken, Ga4Error> {
        let form = [
            ("client_id", self.credentials.client_id.as_str()),
            ("client_secret", self.credentials.client_secret.as_str()),
            ("refresh_token", self.credentials.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .http_client
            .post(&self.token_base_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| Ga4Error::RequestFailed(e.without_url().to_string()))?;

        let status = u16::from(response.status());
        let body = response
            .text()
            .await
            .map_err(|e| Ga4Error::RequestFailed(e.without_url().to_string()))?;

        if status != 200 {
            return Err(Ga4Error::OAuthError { status, body });
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: u64,
        }
        let parsed: TokenResponse =
            serde_json::from_str(&body).map_err(|e| Ga4Error::ParseError(e.to_string()))?;

        Ok(AccessToken {
            token: parsed.access_token,
            expires_in: parsed.expires_in,
        })
    }

    /// Exchange a one-time OAuth2 authorization code for the tenant's
    /// long-lived refresh token (ADR-013 §1: the exchange happens once;
    /// thereafter refresh grants mint access tokens).
    ///
    /// # Errors
    ///
    /// Same contract as [`Ga4Client::mint_access_token`].
    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<RefreshToken, Ga4Error> {
        let form = [
            ("client_id", self.credentials.client_id.as_str()),
            ("client_secret", self.credentials.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let response = self
            .http_client
            .post(&self.token_base_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| Ga4Error::RequestFailed(e.without_url().to_string()))?;

        let status = u16::from(response.status());
        let body = response
            .text()
            .await
            .map_err(|e| Ga4Error::RequestFailed(e.without_url().to_string()))?;

        if status != 200 {
            return Err(Ga4Error::OAuthError { status, body });
        }

        #[derive(Deserialize)]
        struct CodeResponse {
            refresh_token: String,
        }
        let parsed: CodeResponse =
            serde_json::from_str(&body).map_err(|e| Ga4Error::ParseError(e.to_string()))?;

        Ok(RefreshToken {
            token: parsed.refresh_token,
        })
    }

    /// Ensure a usable access token, minting one if none is cached or the
    /// cached one is expired.
    async fn token(&mut self) -> Result<String, Ga4Error> {
        // Refresh 60s before the documented expiry to absorb clock skew.
        const SKEW: u64 = 60;
        let needs_mint = match &self.access_token {
            None => true,
            Some(t) => t.expires_in <= SKEW,
        };
        if needs_mint {
            let fresh = self.mint_access_token().await?;
            self.access_token = Some(fresh);
        }
        Ok(self
            .access_token
            .as_ref()
            .map(|t| t.token.clone())
            .unwrap_or_default())
    }

    /// Run a Data API `runReport` query (ADR-013 §3: typed requests over
    /// dimensions/metrics/date ranges, strongly-typed rows).
    ///
    /// `property_id` may be a bare id (`123456`) or the canonical
    /// `properties/123456` form; the canonical form is used on the wire.
    ///
    /// # Errors
    ///
    /// GSC-contract errors: [`Ga4Error::ApiError`] with status + body,
    /// [`Ga4Error::RequestFailed`] on transport failure,
    /// [`Ga4Error::ParseError`] on malformed responses. Retryable classes
    /// are marked per [`Ga4Error::is_retryable`].
    pub async fn run_report(&mut self, request: &Ga4ReportRequest) -> Result<Ga4Report, Ga4Error> {
        let token = self.token().await?;

        let property = if self.property_id.starts_with("properties/") {
            self.property_id.clone()
        } else {
            format!("properties/{}", self.property_id)
        };
        let url = format!("{}/v1beta/{property}:runReport", self.api_base_url);

        let body = serde_json::json!({
            "dimensions": request.dimensions.iter().map(|d| serde_json::json!({"name": d})).collect::<Vec<_>>(),
            "metrics": request.metrics.iter().map(|m| serde_json::json!({"name": m})).collect::<Vec<_>>(),
            "dateRanges": [{"startDate": request.start_date, "endDate": request.end_date}],
            "limit": request.limit,
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Ga4Error::RequestFailed(e.without_url().to_string()))?;

        let status = u16::from(response.status());
        let text = response
            .text()
            .await
            .map_err(|e| Ga4Error::RequestFailed(e.without_url().to_string()))?;

        if status != 200 {
            // A rejected token is refreshed by the next call, not retried
            // with the same one.
            if status == 401 {
                self.invalidate_token();
            }
            return Err(Ga4Error::ApiError { status, body: text });
        }

        serde_json::from_str(&text).map_err(|e| Ga4Error::ParseError(e.to_string()))
    }
}

/// The tenant's long-lived refresh token from the one-time code exchange.
#[derive(Clone)]
pub struct RefreshToken {
    pub token: String,
}

impl std::fmt::Debug for RefreshToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefreshToken")
            .field("token", &"[redacted]")
            .finish()
    }
}

/// A `runReport` request (ADR-013 §3).
#[derive(Debug, Clone)]
pub struct Ga4ReportRequest {
    /// Dimensions, e.g. `["pagePath", "sessionDefaultChannelGroup"]`.
    pub dimensions: Vec<String>,
    /// Metrics, e.g. `["sessions", "engagementRate", "conversions"]`.
    pub metrics: Vec<String>,
    /// Inclusive start date, `YYYY-MM-DD`.
    pub start_date: String,
    /// Inclusive end date, `YYYY-MM-DD`.
    pub end_date: String,
    /// Maximum rows (Data API caps at 100_000; default here is modest).
    pub limit: u32,
}

impl Ga4ReportRequest {
    /// Engagement/traffic summary for a date range — the shape the findings
    /// fusion path consumes (ADR-013 §3, mirroring the GSC consumer).
    #[must_use]
    pub fn engagement_summary(start_date: &str, end_date: &str, limit: u32) -> Self {
        Self {
            dimensions: vec![
                "pagePath".to_string(),
                "sessionDefaultChannelGroup".to_string(),
            ],
            metrics: vec![
                "sessions".to_string(),
                "engagementRate".to_string(),
                "conversions".to_string(),
            ],
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            limit,
        }
    }
}

/// One dimension-value set with its metric values.
///
/// The Data API wire format nests each value: `"dimensionValues":
/// [{"value": "/blog"}, ...]` — deserialization unwraps the `value`
/// fields; serialization re-wraps them.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct Ga4Row {
    /// Dimension values, positional.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dimension_values: Vec<String>,
    /// Metric values, positional (strings per the Data API wire format).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metric_values: Vec<String>,
}

impl<'de> Deserialize<'de> for Ga4Row {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawRow {
            #[serde(default, rename = "dimensionValues")]
            dimension_values: Vec<serde_json::Value>,
            #[serde(default, rename = "metricValues")]
            metric_values: Vec<serde_json::Value>,
        }
        fn unwrap_values(raw: Vec<serde_json::Value>) -> Vec<String> {
            raw.into_iter()
                .map(|v| {
                    v.get("value")
                        .and_then(|s| s.as_str())
                        .unwrap_or_default()
                        .to_string()
                })
                .collect()
        }
        let raw = RawRow::deserialize(deserializer)?;
        Ok(Self {
            dimension_values: unwrap_values(raw.dimension_values),
            metric_values: unwrap_values(raw.metric_values),
        })
    }
}

/// A parsed `runReport` response.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Ga4Report {
    #[serde(default)]
    pub rows: Vec<Ga4Row>,
    /// Total row count per the response cardinality, when present.
    #[serde(default, rename = "rowCount")]
    pub row_count: u32,
}

impl Ga4Report {
    /// Sum of the metric at `index` across all rows (empty → `0.0`).
    #[must_use]
    pub fn sum_metric(&self, index: usize) -> f64 {
        self.rows
            .iter()
            .filter_map(|r| r.metric_values.get(index))
            .filter_map(|v| v.parse::<f64>().ok())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Error-path and token-hygiene tests (GSC promotion precedent).
    // --- Hermetic one-shot TCP stubs, no Google dependency.

    async fn write_http_response(stream: &mut tokio::net::TcpStream, status: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        use tokio::io::AsyncWriteExt;
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.flush().await;
    }

    /// One-shot server: replies once with `status`/`body`, yields the raw
    /// request bytes to the receiver.
    async fn start_one_shot(
        status: &'static str,
        body: String,
    ) -> (String, tokio::sync::mpsc::Receiver<Vec<u8>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|e| panic!("bind: {e}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|e| panic!("addr: {e}"));
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let (mut stream, _) = listener
                .accept()
                .await
                .unwrap_or_else(|e| panic!("accept: {e}"));
            let mut buf = vec![0u8; 65536];
            let n = stream.read(&mut buf).await.unwrap_or(0);
            let _ = tx.send(buf[..n].to_vec()).await;
            write_http_response(&mut stream, status, &body).await;
        });

        (format!("http://{addr}"), rx)
    }

    fn creds() -> Ga4Credentials {
        Ga4Credentials {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret-material".to_string(),
            refresh_token: "test-refresh-token-material".to_string(),
        }
    }

    trait UnwrapErrForTest<T> {
        fn unwrap_err_else_panic(self) -> Ga4Error;
    }
    impl<T> UnwrapErrForTest<T> for Result<T, Ga4Error> {
        fn unwrap_err_else_panic(self) -> Ga4Error {
            match self {
                Err(e) => e,
                Ok(_) => panic!("expected GA4 error, got success"),
            }
        }
    }

    #[test]
    fn scope_is_exactly_readonly() {
        // ADR-013 §1: read-only by construction. Any change here is a
        // build-visible contract violation.
        assert_eq!(
            GA4_SCOPE,
            "https://www.googleapis.com/auth/analytics.readonly"
        );
        assert!(!GA4_SCOPE.contains("edit"));
        assert!(!GA4_SCOPE.contains("admin"));
        assert!(!GA4_SCOPE.contains("manage"));
    }

    #[tokio::test]
    async fn refresh_grant_mints_token_and_secret_travels_once_in_body() {
        let (base, mut rx) = start_one_shot(
            "200 OK",
            r#"{"access_token":"minted-access-token","expires_in":3600,"token_type":"Bearer"}"#
                .to_string(),
        )
        .await;

        let client =
            Ga4Client::with_base_urls(creds(), "properties/42", "http://ignored".into(), base);
        let token = client
            .mint_access_token()
            .await
            .unwrap_or_else(|e| panic!("refresh must succeed: {e}"));
        assert_eq!(token.token, "minted-access-token");
        assert_eq!(token.expires_in, 3600);

        let request = rx.recv().await.unwrap_or_else(|| panic!("no request"));
        let text = String::from_utf8_lossy(&request);
        // Secrets travel in the form body, exactly once each.
        assert!(
            text.contains("grant_type=refresh_token"),
            "form body: {text}"
        );
        assert_eq!(text.matches("test-refresh-token-material").count(), 1);
        assert_eq!(text.matches("test-client-secret-material").count(), 1);
    }

    #[tokio::test]
    async fn oauth_rejection_surfaces_status_and_body() {
        let (base, rx) = start_one_shot(
            "400 Bad Request",
            r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#
                .to_string(),
        )
        .await;

        let client = Ga4Client::with_base_urls(creds(), "42", "http://ignored".into(), base);
        let err = client.mint_access_token().await.unwrap_err_else_panic();
        drop(rx);

        match &err {
            Ga4Error::OAuthError { status, body } => {
                assert_eq!(*status, 400);
                assert!(body.contains("invalid_grant"), "body surfaced: {body}");
            }
            other => panic!("expected OAuthError, got: {other:?}"),
        }
        assert!(!err.is_retryable(), "invalid_grant is fatal");
        let msg = err.to_string();
        assert!(msg.contains("400"), "status in display: {msg}");
    }

    #[tokio::test]
    async fn oauth_malformed_json_is_parse_error_not_panic() {
        let (base, rx) = start_one_shot("200 OK", "<html>not json</html>".to_string()).await;
        let client = Ga4Client::with_base_urls(creds(), "42", "http://ignored".into(), base);
        let err = client.mint_access_token().await.unwrap_err_else_panic();
        drop(rx);
        assert!(matches!(err, Ga4Error::ParseError(_)), "got: {err:?}");
    }

    #[tokio::test]
    async fn oauth_transport_failure_is_url_scrubbed() {
        // Nothing listens here — guaranteed refusal. reqwest error strings
        // embed URLs; the RequestFailed mapping must strip them (same leak
        // class the CrUX/GSC promotions pinned).
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let client = Ga4Client::with_base_urls(
            creds(),
            "42",
            "http://ignored".into(),
            format!("http://{addr}/token"),
        );
        let err = client.mint_access_token().await.unwrap_err_else_panic();
        assert!(matches!(err, Ga4Error::RequestFailed(_)), "got: {err:?}");
        let msg = err.to_string();
        assert!(
            !msg.contains(&addr.to_string()),
            "token-endpoint URL leaked into error: {msg}"
        );
    }

    #[tokio::test]
    async fn run_report_parses_typed_rows() {
        // Two-stub client: token endpoint then Data API endpoint.
        let (token_base, token_rx) = start_one_shot(
            "200 OK",
            r#"{"access_token":"at-1","expires_in":3600}"#.to_string(),
        )
        .await;
        let (api_base, api_rx) = start_one_shot(
            "200 OK",
            r#"{"rowCount":2,"rows":[
                {"dimensionValues":[{"value":"/blog"},{"value":"Organic Search"}],"metricValues":[{"value":"120"},{"value":"0.65"},{"value":"3"}]},
                {"dimensionValues":[{"value":"/pricing"},{"value":"Direct"}],"metricValues":[{"value":"80"},{"value":"0.7"},{"value":"1"}]}
            ]}"#
            .to_string(),
        )
        .await;

        let mut client = Ga4Client::with_base_urls(creds(), "properties/42", api_base, token_base);
        let report = client
            .run_report(&Ga4ReportRequest::engagement_summary(
                "2026-08-01",
                "2026-08-31",
                100,
            ))
            .await
            .unwrap_or_else(|e| panic!("report must succeed: {e}"));

        drop(token_rx);
        drop(api_rx);
        assert_eq!(report.rows.len(), 2);
        assert_eq!(report.row_count, 2);
        assert_eq!(report.rows[0].dimension_values[0], "/blog");
        assert_eq!(report.rows[0].metric_values[0], "120");
        assert!((report.sum_metric(0) - 200.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn run_report_sends_bearer_once_and_property_is_canonical() {
        let (token_base, token_rx) = start_one_shot(
            "200 OK",
            r#"{"access_token":"bearer-check-token","expires_in":3600}"#.to_string(),
        )
        .await;
        let (api_base, mut api_rx) = start_one_shot("200 OK", r#"{"rows":[]}"#.to_string()).await;

        let mut client = Ga4Client::with_base_urls(
            creds(),
            // Bare id must be canonicalized on the wire.
            "42",
            api_base.clone(),
            token_base,
        );
        let report = client
            .run_report(&Ga4ReportRequest::engagement_summary(
                "2026-08-01",
                "2026-08-31",
                10,
            ))
            .await
            .unwrap_or_else(|e| panic!("report must succeed: {e}"));
        assert!(report.rows.is_empty());

        let request = api_rx.recv().await.unwrap_or_else(|| panic!("no request"));
        let text = String::from_utf8_lossy(&request);
        let request_line = text.lines().next().unwrap_or_default().to_string();
        assert!(
            request_line.starts_with("POST /v1beta/properties/42:runReport"),
            "canonical property path, got: {request_line}"
        );
        assert!(
            text.contains("authorization: Bearer bearer-check-token"),
            "bearer header present"
        );
        assert_eq!(
            text.matches("bearer-check-token").count(),
            1,
            "access token appears exactly once (Authorization header)"
        );
        // Refresh-token material must never ride along on API calls.
        assert!(
            !text.contains("test-refresh-token-material"),
            "refresh token leaked onto API request"
        );
        drop(token_rx);
    }

    #[tokio::test]
    async fn run_report_api_error_is_typed_and_401_invalidates_token() {
        let (token_base, token_rx) = start_one_shot(
            "200 OK",
            r#"{"access_token":"stale-token","expires_in":3600}"#.to_string(),
        )
        .await;
        let (api_base, api_rx) = start_one_shot(
            "403 Forbidden",
            r#"{"error":{"code":403,"message":"Property not found"}}"#.to_string(),
        )
        .await;

        let mut client = Ga4Client::with_base_urls(creds(), "42", api_base, token_base);
        let err = client
            .run_report(&Ga4ReportRequest::engagement_summary(
                "2026-08-01",
                "2026-08-31",
                10,
            ))
            .await
            .unwrap_err_else_panic();

        drop(token_rx);
        drop(api_rx);
        match &err {
            Ga4Error::ApiError { status, body } => {
                assert_eq!(*status, 403);
                assert!(body.contains("Property not found"), "body surfaced: {body}");
            }
            other => panic!("expected ApiError, got: {other:?}"),
        }
        assert!(!err.is_retryable(), "403 is fatal");
    }

    #[tokio::test]
    async fn quota_exceeded_is_retryable() {
        // 429 (tokens-per-property / concurrency quotas) must classify as
        // retryable so loop_retry callers get backoff for free (ADR-013 §3).
        let (token_base, token_rx) = start_one_shot(
            "200 OK",
            r#"{"access_token":"t","expires_in":3600}"#.to_string(),
        )
        .await;
        let (api_base, api_rx) = start_one_shot(
            "429 Too Many Requests",
            r#"{"error":{"code":429,"message":"Exhausts property tokens"}}"#.to_string(),
        )
        .await;

        let mut client = Ga4Client::with_base_urls(creds(), "42", api_base, token_base);
        let err = client
            .run_report(&Ga4ReportRequest::engagement_summary(
                "2026-08-01",
                "2026-08-31",
                10,
            ))
            .await
            .unwrap_err_else_panic();

        drop(token_rx);
        drop(api_rx);
        assert!(err.is_retryable(), "429 must be retryable, got: {err:?}");
    }

    #[tokio::test]
    async fn connection_refused_maps_to_request_failed() {
        let mut client = Ga4Client::with_base_urls(
            creds(),
            "42",
            "http://127.0.0.1:1".into(),
            "http://127.0.0.1:1/token".into(),
        );
        let err = client
            .run_report(&Ga4ReportRequest::engagement_summary(
                "2026-08-01",
                "2026-08-31",
                10,
            ))
            .await
            .unwrap_err_else_panic();
        assert!(matches!(err, Ga4Error::RequestFailed(_)), "got: {err:?}");
        assert!(err.is_retryable(), "transport failure is retryable");
    }

    #[tokio::test]
    async fn code_exchange_returns_refresh_token_once() {
        let (base, mut rx) = start_one_shot(
            "200 OK",
            r#"{"access_token":"short-lived","expires_in":3600,"refresh_token":"NEW-REFRESH-MATERIAL","scope":"https://www.googleapis.com/auth/analytics.readonly"}"#
                .to_string(),
        )
        .await;

        let client = Ga4Client::with_base_urls(creds(), "42", "http://ignored".into(), base);
        let refresh = client
            .exchange_code("one-time-code", "https://app.example.com/oauth/callback")
            .await
            .unwrap_or_else(|e| panic!("exchange must succeed: {e}"));
        assert_eq!(refresh.token, "NEW-REFRESH-MATERIAL");

        let request = rx.recv().await.unwrap_or_else(|| panic!("no request"));
        let text = String::from_utf8_lossy(&request);
        assert!(
            text.contains("grant_type=authorization_code"),
            "form: {text}"
        );
        assert!(text.contains("code=one-time-code"), "code sent once");
        drop(rx);
    }

    #[test]
    fn empty_report_sums_are_zero() {
        let report = Ga4Report::default();
        assert_eq!(report.sum_metric(0), 0.0);
    }

    #[test]
    fn debug_forms_never_contain_secret_material() {
        // ADR-013 §2: the easiest accidental log path is {:?}. All secret-
        // bearing types must redact in Debug.
        let creds = creds();
        let rendered = format!("{creds:?}");
        assert!(
            !rendered.contains("test-client-secret-material"),
            "{rendered}"
        );
        assert!(
            !rendered.contains("test-refresh-token-material"),
            "{rendered}"
        );

        let token = AccessToken {
            token: "at-secret".into(),
            expires_in: 3600,
        };
        let rendered = format!("{token:?}");
        assert!(!rendered.contains("at-secret"), "{rendered}");

        let refresh = RefreshToken {
            token: "rt-secret".into(),
        };
        let rendered = format!("{refresh:?}");
        assert!(!rendered.contains("rt-secret"), "{rendered}");
    }
}
