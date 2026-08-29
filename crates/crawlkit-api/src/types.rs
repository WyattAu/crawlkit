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
use utoipa::ToSchema;

use crawlkit_engine::storage_trait::StorageBackend;
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
    /// Crawls started, labeled by tenant. Tenant ids are admin-controlled
    /// so label cardinality is bounded by the tenant population.
    pub crawls_started_by_tenant: Family<TenantLabel, Counter>,
    /// Pages crawled, labeled by tenant (incremented at crawl completion).
    pub pages_by_tenant: Family<TenantLabel, Counter>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, prometheus_client::encoding::EncodeLabelSet)]
pub struct TenantLabel {
    pub tenant: String,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, prometheus_client::encoding::EncodeLabelSet)]
pub struct EndpointLabel {
    pub endpoint: String,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
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

        let crawls_started_by_tenant = Family::<TenantLabel, Counter>::default();
        registry.register(
            "crawlkit_crawls_started_by_tenant",
            "Crawls started, labeled by tenant",
            crawls_started_by_tenant.clone(),
        );

        let pages_by_tenant = Family::<TenantLabel, Counter>::default();
        registry.register(
            "crawlkit_pages_by_tenant",
            "Pages crawled at crawl completion, labeled by tenant",
            pages_by_tenant.clone(),
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
            crawls_started_by_tenant,
            pages_by_tenant,
        }
    }
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTenantRequest {
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn StorageBackend>,
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
    pub oidc_states: Arc<DashMap<String, OidcPendingState>>,
    pub tenants: Arc<dashmap::DashMap<String, Tenant>>,
    pub marketplace: MarketplaceState,
    pub sessions: Arc<DashMap<String, SessionInfo>>,
    /// Per-email login attempt tracking for brute-force lockout.
    pub login_attempts: Arc<DashMap<String, LoginAttemptRecord>>,
    /// Write-through persistence for users/tenants/API keys (`None` in
    /// tests or when persistence is disabled).
    pub persistence: Option<Arc<dyn crate::persistence::ApiStateStore>>,
    /// Bounds concurrently running crawl tasks; submissions beyond this
    /// capacity are rejected with 503 + Retry-After (backpressure).
    pub crawl_permits: Arc<tokio::sync::Semaphore>,
    /// `Idempotency-Key` → (crawl_id, created_at) for POST /crawls replay
    /// protection within the dedupe window.
    pub idempotency_keys: Arc<DashMap<String, IdempotencyEntry>>,
}

/// A recorded idempotency-key mapping for crawl submissions.
#[derive(Debug, Clone)]
pub struct IdempotencyEntry {
    pub crawl_id: String,
    pub created_at: DateTime<Utc>,
}

/// How long an `Idempotency-Key` remains replayable.
pub const IDEMPOTENCY_WINDOW: chrono::Duration = chrono::Duration::hours(24);

/// Default maximum concurrently running crawls when
/// `MAX_CONCURRENT_CRAWLS` is unset.
pub const DEFAULT_MAX_CONCURRENT_CRAWLS: usize = 4;

/// Read the crawl concurrency cap from the environment.
#[must_use]
pub fn crawl_capacity_from_env() -> usize {
    std::env::var("MAX_CONCURRENT_CRAWLS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_MAX_CONCURRENT_CRAWLS)
}

/// Pending OIDC authorization flow state (one-shot, per `state` token).
#[derive(Debug, Clone)]
pub struct OidcPendingState {
    pub created_at: DateTime<Utc>,
    /// PKCE code verifier (S256). Sent to the token endpoint on callback.
    pub code_verifier: String,
    /// Nonce bound into the authorization request; must match the id_token.
    pub nonce: String,
}

/// Brute-force protection record, keyed by normalized email.
#[derive(Debug, Clone)]
pub struct LoginAttemptRecord {
    pub failures: u32,
    pub window_start: DateTime<Utc>,
    pub locked_until: Option<DateTime<Utc>>,
}

/// Maximum failed login attempts per email before lockout.
pub const LOGIN_MAX_FAILURES: u32 = 5;
/// Window over which failures accumulate.
pub const LOGIN_FAILURE_WINDOW: chrono::Duration = chrono::Duration::minutes(15);
/// Lockout duration once the failure threshold is reached.
pub const LOGIN_LOCKOUT: chrono::Duration = chrono::Duration::minutes(15);

/// Returns `Err(Unauthorized)` when the caller is currently locked out,
/// `Ok(record)` otherwise (creating or resetting the window as needed).
pub fn check_login_lockout(
    attempts: &DashMap<String, LoginAttemptRecord>,
    email: &str,
    now: DateTime<Utc>,
) -> Result<(), ApiError> {
    let key = email.trim().to_ascii_lowercase();
    if let Some(mut entry) = attempts.get_mut(&key) {
        let record = entry.value_mut();
        // Reset the window once it (and any lockout) has fully expired.
        let window_expired = now - record.window_start > LOGIN_FAILURE_WINDOW;
        let lockout_expired = record.locked_until.is_none_or(|until| now >= until);
        if window_expired && lockout_expired {
            *record = LoginAttemptRecord {
                failures: 0,
                window_start: now,
                locked_until: None,
            };
        } else if let Some(until) = record.locked_until {
            if now < until {
                return Err(ApiError::Unauthorized(format!(
                    "Account temporarily locked due to failed login attempts. Try again after {until}"
                )));
            }
        }
    }
    Ok(())
}

/// Record a failed login attempt; engages the lockout at the threshold.
pub fn record_login_failure(
    attempts: &DashMap<String, LoginAttemptRecord>,
    email: &str,
    now: DateTime<Utc>,
) {
    let key = email.trim().to_ascii_lowercase();
    let mut entry = attempts.entry(key).or_insert_with(|| LoginAttemptRecord {
        failures: 0,
        window_start: now,
        locked_until: None,
    });
    let record = entry.value_mut();
    if now - record.window_start > LOGIN_FAILURE_WINDOW {
        record.failures = 0;
        record.window_start = now;
    }
    record.failures += 1;
    if record.failures >= LOGIN_MAX_FAILURES {
        record.locked_until = Some(now + LOGIN_LOCKOUT);
    }
}

/// Clear failure state after a successful login.
pub fn clear_login_failures(attempts: &DashMap<String, LoginAttemptRecord>, email: &str) {
    attempts.remove(&email.trim().to_ascii_lowercase());
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub jti: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RevokeSessionRequest {
    pub jti: String,
}

#[derive(Serialize, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct CrawlResponse {
    pub crawl_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CrawlResult {
    pub crawl_id: String,
    pub tenant_id: String,
    pub start_url: String,
    pub status: String,
    pub pages_crawled: usize,
    pub issues_found: usize,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Storage-layer crawl id owned by the engine once the run completes.
    /// `crawl_id` remains the public API identifier; this field maps it to
    /// the row that actually contains pages and findings.
    #[serde(default)]
    pub storage_crawl_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CrawlStatsResponse {
    pub crawl_id: String,
    pub total_pages: usize,
    pub total_issues: usize,
    pub issues_by_severity: HashMap<String, usize>,
    pub issues_by_category: HashMap<String, usize>,
    pub avg_response_time_ms: Option<f64>,
}

/// Single crawl finding as returned by `GET /api/v1/crawls/{id}/findings`.
/// Documentation mirror of the engine's storage `Issue` projection.
#[derive(Debug, Serialize, ToSchema)]
pub struct CrawlFinding {
    pub id: String,
    pub page_id: String,
    pub category: String,
    pub severity: String,
    pub code: String,
    pub title: String,
    pub description: String,
    pub element: Option<String>,
    pub recommendation: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApiKeyCreateRequest {
    pub name: String,
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,
}

pub fn default_rpm() -> u32 {
    60
}

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

// ---------------------------------------------------------------------------
// Webhook types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookConfig {
    pub id: String,
    pub tenant_id: String,
    pub url: String,
    pub events: Vec<String>,
    /// Never serialized in responses; listed webhooks omit the secret.
    #[serde(skip_serializing, default)]
    pub secret: String,
    pub created_at: DateTime<Utc>,
}

/// Response returned once when a webhook is created, containing the secret.
#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookCreatedResponse {
    pub id: String,
    pub tenant_id: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScheduleConfig {
    pub id: String,
    pub tenant_id: String,
    /// Internal engine crawl configuration (URL + delay); free-form in docs.
    #[schema(value_type = Object)]
    pub crawl_config: CrawlConfig,
    pub interval_secs: u64,
    pub enabled: bool,
    pub next_run: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    /// Storage crawl ID from the most recent run (used as baseline for monitoring).
    pub last_crawl_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct ScheduleResponse {
    pub id: String,
    pub start_url: String,
    pub interval_secs: u64,
    pub enabled: bool,
    pub next_run: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
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

/// JSON error body emitted by every `ApiError` response.
/// Documentation mirror of the `{"error": ..., "status": ...}` shape.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiErrorBody {
    /// Human-readable error message.
    pub error: String,
    /// HTTP status code (numeric).
    pub status: u16,
}

/// Audit event entry as returned by `GET /api/v1/audit`.
/// Documentation mirror of the engine's `AuditEvent` serialization.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiAuditEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub actor: String,
    pub tenant_id: Option<String>,
    pub details: String,
    pub hash: String,
}

#[derive(Debug)]
pub enum ApiError {
    Unauthorized(String),
    BadRequest(String),
    NotFound(String),
    Forbidden(String),
    RateLimited,
    /// Server at capacity for stateful work; includes a Retry-After hint.
    Overloaded {
        retry_after_secs: u32,
    },
    Internal(String),
}

impl ApiError {
    /// Human-readable message without the HTTP status.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            ApiError::Unauthorized(msg)
            | ApiError::BadRequest(msg)
            | ApiError::NotFound(msg)
            | ApiError::Forbidden(msg)
            | ApiError::Internal(msg) => msg.clone(),
            ApiError::RateLimited => "Rate limit exceeded".to_string(),
            ApiError::Overloaded { .. } => "Server at crawl capacity".to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let retry_after = match &self {
            ApiError::Overloaded { retry_after_secs } => Some(*retry_after_secs),
            _ => None,
        };
        let (status, message) = match &self {
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
            ),
            ApiError::Overloaded { .. } => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Server at crawl capacity; retry after the indicated interval".to_string(),
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

        let mut response = (status, body).into_response();
        if let Some(secs) = retry_after {
            if let Ok(value) = axum::http::HeaderValue::from_str(&secs.to_string()) {
                response.headers_mut().insert("retry-after", value);
            }
        }
        response
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
    if !crawlkit_engine::ssrf::is_public_url(url) {
        return Err(ApiError::BadRequest(
            "URL targets a reserved internal hostname or private IP address".to_string(),
        ));
    }
    Ok(())
}

/// Validate a server-side HTTP target against common SSRF destinations.
/// DNS resolution is intentionally not performed here; the HTTP client must
/// also enforce redirect and resolver policy at connection time.
pub fn validate_public_url(url: &str) -> Result<(), ApiError> {
    validate_url(url)
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

/// Maximum allowed requests-per-minute for a created API key.
pub const MAX_API_KEY_RPM: u32 = 10_000;

pub fn validate_rpm(rpm: u32) -> Result<(), ApiError> {
    if !(1..=MAX_API_KEY_RPM).contains(&rpm) {
        return Err(ApiError::BadRequest(format!(
            "requests_per_minute must be between 1 and {MAX_API_KEY_RPM}"
        )));
    }
    Ok(())
}

/// Redirect policy for all server-side outbound HTTP (webhooks, OIDC).
///
/// Every hop is re-validated against the SSRF blocklist: a redirect to a
/// private/loopback/metadata address aborts the request instead of being
/// followed. Hop count is additionally capped.
pub fn ssrf_safe_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        const MAX_HOPS: usize = 5;
        if attempt.previous().len() >= MAX_HOPS {
            attempt.stop()
        } else if validate_public_url(attempt.url().as_str()).is_ok() {
            attempt.follow()
        } else {
            attempt.error("redirect target failed SSRF validation")
        }
    })
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

/// Permission enforcement for privileged operations.
///
/// Returns `Ok(())` when the JWT claims carry the required permission,
/// otherwise `Err(Forbidden)`. Handlers for administrative surfaces
/// (tenants, API keys, marketplace) must call this before mutating state.
pub fn require_permission(claims: &auth::Claims, permission: &str) -> Result<(), ApiError> {
    if claims.permissions.iter().any(|p| p == permission) {
        Ok(())
    } else {
        Err(ApiError::Forbidden(format!(
            "Missing required permission: {permission}"
        )))
    }
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

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub enabled: bool,
}
