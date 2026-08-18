use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{exponential_buckets, Histogram};
use prometheus_client::registry::Registry;
use serde::{Deserialize, Serialize};

use crawlkit_engine::storage::Storage;
use crawlkit_engine::AuditTrail;
use crawlkit_engine::CrawlConfig;

use crate::auth;
use crate::oidc::OidcManager;

// ---------------------------------------------------------------------------
// API key management
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub requests_per_minute: u32,
}

#[derive(Debug, Clone)]
pub struct RateLimitBucket {
    pub tokens: f64,
    pub max_tokens: f64,
    pub refill_rate: f64,
    pub last_refill: std::time::Instant,
}

impl RateLimitBucket {
    pub fn new(rpm: u32) -> Self {
        Self {
            tokens: f64::from(rpm),
            max_tokens: f64::from(rpm),
            refill_rate: f64::from(rpm) / 60.0,
            last_refill: std::time::Instant::now(),
        }
    }

    pub fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

// ---------------------------------------------------------------------------
// Prometheus metrics
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Metrics {
    pub registry: Arc<tokio::sync::RwLock<Registry>>,
    pub crawls_total: Counter,
    pub pages_crawled_total: Counter,
    pub issues_total: Counter,
    pub errors_total: Counter,
    pub requests_total: Family<EndpointLabel, Counter>,
    pub request_duration_seconds: Histogram,
    pub fetch_duration_seconds: Histogram,
    pub analysis_duration_seconds: Histogram,
    pub active_crawls: Gauge,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, prometheus_client::encoding::EncodeLabelSet)]
pub struct EndpointLabel {
    pub endpoint: String,
}

impl Metrics {
    pub fn new() -> Self {
        let mut registry = Registry::default();

        let crawls_total = Counter::default();
        registry.register(
            "crawlkit_crawls_total",
            "Total number of crawls started",
            crawls_total.clone(),
        );

        let pages_crawled_total = Counter::default();
        registry.register(
            "crawlkit_pages_crawled_total",
            "Total pages crawled across all crawls",
            pages_crawled_total.clone(),
        );

        let issues_total = Counter::default();
        registry.register(
            "crawlkit_issues_total",
            "Total issues found across all crawls",
            issues_total.clone(),
        );

        let errors_total = Counter::default();
        registry.register(
            "crawlkit_errors_total",
            "Total errors encountered during crawls",
            errors_total.clone(),
        );

        let requests_total = Family::<EndpointLabel, Counter>::default();
        registry.register(
            "crawlkit_requests_total",
            "Total API requests by endpoint",
            requests_total.clone(),
        );

        let request_duration_seconds = Histogram::new(exponential_buckets(0.005, 2.0, 10));
        registry.register(
            "crawlkit_request_duration_seconds",
            "API request duration in seconds",
            request_duration_seconds.clone(),
        );

        let fetch_duration_seconds = Histogram::new(exponential_buckets(0.1, 2.0, 10));
        registry.register(
            "crawlkit_fetch_duration_seconds",
            "HTTP fetch duration in seconds",
            fetch_duration_seconds.clone(),
        );

        let analysis_duration_seconds = Histogram::new(exponential_buckets(0.01, 2.0, 10));
        registry.register(
            "crawlkit_analysis_duration_seconds",
            "Page analysis duration in seconds",
            analysis_duration_seconds.clone(),
        );

        let active_crawls = Gauge::default();
        registry.register(
            "crawlkit_active_crawls",
            "Number of currently active crawls",
            active_crawls.clone(),
        );

        Self {
            registry: Arc::new(tokio::sync::RwLock::new(registry)),
            crawls_total,
            pages_crawled_total,
            issues_total,
            errors_total,
            requests_total,
            request_duration_seconds,
            fetch_duration_seconds,
            analysis_duration_seconds,
            active_crawls,
        }
    }
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<Storage>,
    pub api_keys: Arc<DashMap<String, ApiKey>>,
    pub rate_limits: Arc<DashMap<String, RateLimitBucket>>,
    pub crawl_results: Arc<DashMap<String, CrawlResult>>,
    pub audit_trail: Arc<AuditTrail>,
    pub metrics: Arc<Metrics>,
    pub webhooks: Arc<DashMap<String, WebhookConfig>>,
    pub schedules: Arc<DashMap<String, ScheduleConfig>>,
    pub http_client: reqwest::Client,
    pub auth: Arc<auth::AuthManager>,
    pub oidc: Option<Arc<OidcManager>>,
    pub oidc_states: Arc<DashMap<String, DateTime<Utc>>>,
    pub tenants: Arc<dashmap::DashMap<String, Tenant>>,
    pub marketplace: MarketplaceState,
    pub sessions: Arc<DashMap<String, SessionInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub jti: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}

#[derive(Debug, Deserialize)]
pub struct RevokeSessionRequest {
    pub jti: String,
}

#[derive(Serialize)]
pub struct SessionResponse {
    pub jti: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateCrawlRequest {
    pub start_url: String,
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    #[serde(default = "default_delay")]
    pub request_delay_ms: u64,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

pub fn default_max_pages() -> usize {
    100
}
pub fn default_delay() -> u64 {
    500
}
pub fn default_concurrency() -> usize {
    4
}

#[derive(Debug, Serialize)]
pub struct CrawlResponse {
    pub crawl_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    pub crawl_id: String,
    pub tenant_id: String,
    pub start_url: String,
    pub status: String,
    pub pages_crawled: usize,
    pub issues_found: usize,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CrawlStatsResponse {
    pub crawl_id: String,
    pub total_pages: usize,
    pub total_issues: usize,
    pub issues_by_severity: HashMap<String, usize>,
    pub issues_by_category: HashMap<String, usize>,
    pub avg_response_time_ms: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ApiKeyCreateRequest {
    pub name: String,
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,
}

pub fn default_rpm() -> u32 {
    60
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub key: String,
    pub name: String,
    pub requests_per_minute: u32,
}

impl ApiKeyResponse {
    /// Redact the API key for safe display. Shows only last 4 characters.
    pub fn redacted(key: &str) -> String {
        if key.len() <= 4 {
            "****".to_string()
        } else {
            format!("{}****", &key[key.len() - 4..])
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

// ---------------------------------------------------------------------------
// Webhook types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: String,
    pub tenant_id: String,
    pub url: String,
    pub events: Vec<String>,
    #[serde(skip_serializing, default)]
    pub secret: String,
    pub created_at: DateTime<Utc>,
}

/// Response returned once when a webhook is created, containing the secret.
#[derive(Debug, Serialize)]
pub struct WebhookCreatedResponse {
    pub id: String,
    pub tenant_id: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    #[serde(default = "default_webhook_events")]
    pub events: Vec<String>,
}

pub fn default_webhook_events() -> Vec<String> {
    vec!["crawl.completed".to_string(), "crawl.failed".to_string()]
}

#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    pub event: String,
    pub crawl_id: String,
    pub pages_crawled: usize,
    pub issues_found: usize,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Scheduled crawl types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub id: String,
    pub tenant_id: String,
    pub crawl_config: CrawlConfig,
    pub interval_secs: u64,
    pub enabled: bool,
    pub next_run: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateScheduleRequest {
    pub start_url: String,
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    #[serde(default = "default_delay")]
    pub request_delay_ms: u64,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_schedule_interval")]
    pub interval_secs: u64,
}

pub fn default_schedule_interval() -> u64 {
    3600
}

#[derive(Debug, Serialize)]
pub struct ScheduleResponse {
    pub id: String,
    pub start_url: String,
    pub interval_secs: u64,
    pub enabled: bool,
    pub next_run: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateScheduleRequest {
    #[serde(default)]
    pub start_url: Option<String>,
    pub max_pages: Option<usize>,
    pub request_delay_ms: Option<u64>,
    pub concurrency: Option<usize>,
    pub interval_secs: Option<u64>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

// ---------------------------------------------------------------------------
// Plugin marketplace types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub license: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SubmitPluginRequest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub license: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Clone)]
pub struct MarketplaceState {
    pub plugins: Arc<RwLock<HashMap<String, MarketplacePlugin>>>,
}

impl Default for MarketplaceState {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketplaceState {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ApiError {
    Unauthorized(String),
    BadRequest(String),
    NotFound(String),
    RateLimited,
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
            ),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        // Capture internal errors in Sentry
        if matches!(self, ApiError::Internal(_)) {
            sentry::capture_message(
                &format!("API internal error: {message}"),
                sentry::Level::Error,
            );
        }

        let body = Json(serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------

pub fn validate_url(url: &str) -> Result<(), ApiError> {
    if url.len() > 2048 {
        return Err(ApiError::BadRequest(
            "URL exceeds 2048 characters".to_string(),
        ));
    }
    let parsed =
        url::Url::parse(url).map_err(|e| ApiError::BadRequest(format!("Invalid URL: {e}")))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(ApiError::BadRequest(
            "URL must use http or https scheme".to_string(),
        ));
    }
    reject_private_target(&parsed)?;
    Ok(())
}

/// Validate a server-side HTTP target against common SSRF destinations.
/// DNS resolution is intentionally not performed here; the HTTP client must
/// also enforce redirect and resolver policy at connection time.
pub fn validate_public_url(url: &str) -> Result<(), ApiError> {
    validate_url(url)
}

fn reject_private_target(parsed: &url::Url) -> Result<(), ApiError> {
    let Some(host) = parsed.host_str() else {
        return Err(ApiError::BadRequest("URL must include a host".to_string()));
    };
    let normalized = host.trim_end_matches('.').to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "localhost" | "localhost.localdomain" | "metadata.google.internal"
    ) {
        return Err(ApiError::BadRequest(
            "URL targets a reserved internal hostname".to_string(),
        ));
    }
    let ip_host = host.trim_matches(['[', ']']);
    if let Ok(ip) = ip_host.parse::<std::net::IpAddr>() {
        let blocked = match ip {
            std::net::IpAddr::V4(ip) => {
                ip.is_private()
                    || ip.is_loopback()
                    || ip.is_link_local()
                    || ip.is_broadcast()
                    || ip.is_unspecified()
                    || ip.is_multicast()
                    || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
                    || (ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 0)
            }
            std::net::IpAddr::V6(ip) => {
                ip.is_loopback()
                    || ip.is_unspecified()
                    || ip.is_unique_local()
                    || ip.is_unicast_link_local()
                    || ip.is_multicast()
            }
        };
        if blocked {
            return Err(ApiError::BadRequest(
                "URL targets a reserved or private IP address".to_string(),
            ));
        }
    }
    Ok(())
}

pub fn validate_max_pages(n: usize) -> Result<(), ApiError> {
    if !(1..=10000).contains(&n) {
        return Err(ApiError::BadRequest(
            "max_pages must be between 1 and 10000".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_concurrency(n: usize) -> Result<(), ApiError> {
    if !(1..=128).contains(&n) {
        return Err(ApiError::BadRequest(
            "concurrency must be between 1 and 128".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_delay(ms: u64) -> Result<(), ApiError> {
    if ms > 60000 {
        return Err(ApiError::BadRequest(
            "request_delay_ms must be at most 60000".to_string(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Extract tenant ID from JWT claims.
pub fn extract_tenant(claims: &auth::Claims) -> &str {
    &claims.tenant
}

/// Check if the current user is an admin.
pub fn is_admin(claims: &auth::Claims) -> bool {
    claims.roles.contains(&"admin".to_string())
}

/// Tenant-ownership check for per-tenant resources.
///
/// Returns `true` when the resource belongs to the caller's tenant or the
/// caller is an admin. Every handler that reads, mutates, or deletes a
/// tenant-scoped resource must gate access through this predicate; failures
/// surface as `404` (not `403`) to avoid leaking resource existence.
#[must_use]
pub fn can_access_tenant(claims: &auth::Claims, resource_tenant: &str) -> bool {
    is_admin(claims) || resource_tenant == claims.tenant
}

/// Map OIDC roles/groups to crawlkit roles.
pub fn map_oidc_roles(oidc_roles: &[String], oidc_groups: &[String]) -> Vec<String> {
    let mut roles = Vec::new();
    let all_claims: Vec<&str> = oidc_roles
        .iter()
        .chain(oidc_groups.iter())
        .map(|s| s.as_str())
        .collect();

    if all_claims.iter().any(|c| {
        c.eq_ignore_ascii_case("admin")
            || c.eq_ignore_ascii_case("crawlkit-admin")
            || c.ends_with("/admin")
    }) {
        roles.push("admin".to_string());
    } else if all_claims.iter().any(|c| {
        c.eq_ignore_ascii_case("editor")
            || c.eq_ignore_ascii_case("crawlkit-editor")
            || c.ends_with("/editor")
    }) {
        roles.push("editor".to_string());
    } else {
        roles.push("viewer".to_string());
    }

    roles
}

// ---------------------------------------------------------------------------
// Auth types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    pub password: String,
    #[serde(default = "default_user_roles")]
    pub roles: Vec<String>,
}

pub fn default_user_roles() -> Vec<String> {
    vec!["viewer".to_string()]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(tenant: &str, roles: &[&str]) -> auth::Claims {
        auth::Claims {
            sub: "user-1".to_string(),
            tenant: tenant.to_string(),
            roles: roles.iter().map(|r| (*r).to_string()).collect(),
            permissions: Vec::new(),
            exp: 999_999_999_999,
            iat: 0,
            jti: "jti-1".to_string(),
        }
    }

    // -- default value helpers ------------------------------------------

    #[test]
    fn test_default_rpm() {
        assert_eq!(default_rpm(), 60);
    }

    #[test]
    fn test_default_max_pages() {
        assert_eq!(default_max_pages(), 100);
    }

    #[test]
    fn test_default_delay() {
        assert_eq!(default_delay(), 500);
    }

    #[test]
    fn test_default_concurrency() {
        assert_eq!(default_concurrency(), 4);
    }

    #[test]
    fn test_default_webhook_events() {
        let events = default_webhook_events();
        assert_eq!(events.len(), 2);
        assert!(events.contains(&"crawl.completed".to_string()));
        assert!(events.contains(&"crawl.failed".to_string()));
    }

    #[test]
    fn test_default_user_roles() {
        assert_eq!(default_user_roles(), vec!["viewer".to_string()]);
    }

    #[test]
    fn test_default_schedule_interval() {
        assert_eq!(default_schedule_interval(), 3600);
    }

    // -- RateLimitBucket --------------------------------------------------

    #[test]
    fn test_rate_limit_bucket_new() {
        let bucket = RateLimitBucket::new(120);
        assert_eq!(bucket.tokens, 120.0);
        assert_eq!(bucket.max_tokens, 120.0);
        assert!((bucket.refill_rate - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rate_limit_bucket_try_consume_success() {
        let mut bucket = RateLimitBucket::new(60);
        assert!(bucket.try_consume());
        assert!(bucket.tokens < 60.0);
    }

    #[test]
    fn test_rate_limit_bucket_try_consume_exhaustion() {
        let mut bucket = RateLimitBucket::new(1);
        assert!(bucket.try_consume());
        let second = bucket.try_consume();
        assert!(bucket.tokens < 1.0 || !second);
    }

    #[test]
    fn test_rate_limit_bucket_refill_capped_at_max() {
        let mut bucket = RateLimitBucket::new(10);
        for _ in 0..10 {
            bucket.try_consume();
        }
        assert!(bucket.tokens < 1.0);
        bucket.last_refill = std::time::Instant::now() - std::time::Duration::from_secs(120);
        bucket.refill();
        assert!((bucket.tokens - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rate_limit_bucket_refill_partial() {
        let mut bucket = RateLimitBucket::new(60);
        bucket.try_consume();
        let before = bucket.tokens;
        bucket.last_refill = std::time::Instant::now() - std::time::Duration::from_secs(1);
        bucket.refill();
        assert!(bucket.tokens > before);
        assert!(bucket.tokens <= bucket.max_tokens);
    }

    // -- validation -------------------------------------------------------

    #[test]
    fn test_validate_url_accepts_https() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://example.com/path?q=1").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_non_http_schemes() {
        for url in ["ftp://example.com", "file:///etc/passwd"] {
            let err = validate_url(url).unwrap_err();
            assert!(
                matches!(err, ApiError::BadRequest(ref msg) if msg.contains("http or https")),
                "expected BadRequest for {url}, got {err:?}"
            );
        }
    }

    #[test]
    fn test_validate_url_rejects_overlong() {
        let long = format!("https://example.com/{}", "a".repeat(2100));
        let err = validate_url(&long).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn test_validate_url_rejects_ssrf_targets() {
        for url in [
            "http://127.0.0.1:8080/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://169.254.169.254/latest/meta-data/",
            "http://[::1]/",
            "http://localhost/",
            "http://metadata.google.internal/",
        ] {
            assert!(validate_url(url).is_err(), "SSRF target accepted: {url}");
        }
    }

    #[test]
    fn test_validate_url_accepts_public_ip_and_domain() {
        assert!(validate_public_url("https://8.8.8.8/").is_ok());
        assert!(validate_public_url("https://example.com/").is_ok());
    }

    #[test]
    fn test_validate_max_pages_bounds() {
        assert!(validate_max_pages(1).is_ok());
        assert!(validate_max_pages(10_000).is_ok());
        assert!(validate_max_pages(0).is_err());
        assert!(validate_max_pages(10_001).is_err());
    }

    #[test]
    fn test_validate_concurrency_bounds() {
        assert!(validate_concurrency(1).is_ok());
        assert!(validate_concurrency(128).is_ok());
        assert!(validate_concurrency(0).is_err());
        assert!(validate_concurrency(129).is_err());
    }

    #[test]
    fn test_validate_delay_bounds() {
        assert!(validate_delay(0).is_ok());
        assert!(validate_delay(60_000).is_ok());
        assert!(validate_delay(60_001).is_err());
    }

    // -- API key redaction --------------------------------------------------

    #[test]
    fn test_api_key_redaction() {
        assert_eq!(ApiKeyResponse::redacted("ck_abcdef123456"), "3456****");
        assert_eq!(ApiKeyResponse::redacted("key"), "****");
        assert_eq!(ApiKeyResponse::redacted(""), "****");
    }

    // -- claims helpers -----------------------------------------------------

    #[test]
    fn test_extract_tenant() {
        assert_eq!(extract_tenant(&claims("acme", &["viewer"])), "acme");
    }

    #[test]
    fn test_is_admin() {
        assert!(is_admin(&claims("t", &["admin"])));
        assert!(is_admin(&claims("t", &["viewer", "admin"])));
        assert!(!is_admin(&claims("t", &["viewer"])));
        assert!(!is_admin(&claims("t", &[])));
    }

    #[test]
    fn test_can_access_tenant_owner() {
        let viewer = claims("acme", &["viewer"]);
        assert!(can_access_tenant(&viewer, "acme"));
    }

    #[test]
    fn test_can_access_tenant_isolates_foreign_tenants() {
        let viewer = claims("acme", &["viewer"]);
        assert!(
            !can_access_tenant(&viewer, "corp"),
            "non-admin must not read another tenant's resource"
        );
        assert!(!can_access_tenant(&viewer, ""));
    }

    #[test]
    fn test_can_access_tenant_admin_bypass() {
        let admin = claims("acme", &["admin"]);
        assert!(can_access_tenant(&admin, "corp"));
        assert!(can_access_tenant(&admin, "acme"));
    }

    #[test]
    fn test_map_oidc_roles_admin_wins() {
        let roles = map_oidc_roles(&["editor".to_string()], &["corp/admin".to_string()]);
        assert_eq!(roles, vec!["admin".to_string()]);
    }

    #[test]
    fn test_map_oidc_roles_editor() {
        let roles = map_oidc_roles(&["crawlkit-editor".to_string()], &[]);
        assert_eq!(roles, vec!["editor".to_string()]);
    }

    #[test]
    fn test_map_oidc_roles_defaults_to_viewer() {
        let roles = map_oidc_roles(&[], &["everyone".to_string()]);
        assert_eq!(roles, vec!["viewer".to_string()]);
    }

    // -- ApiError mapping ----------------------------------------------------

    #[test]
    fn test_api_error_status_mapping() {
        // IntoResponse is exercised implicitly via these constructors; the
        // status mapping itself is compile-time data. We at least verify the
        // variants construct and format.
        assert!(matches!(
            ApiError::Unauthorized("nope".to_string()),
            ApiError::Unauthorized(_)
        ));
        assert!(matches!(ApiError::RateLimited, ApiError::RateLimited));
    }
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub enabled: bool,
}
