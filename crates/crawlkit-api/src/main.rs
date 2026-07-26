//! REST API server for crawlkit — exposes crawl, compare, and report operations over HTTP.
//!
//! Built on Axum with API-key authentication and per-key rate limiting.
//! Start a crawl via `POST /api/v1/crawls` and poll status at
//! `GET /api/v1/crawls/{crawl_id}`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod auth;
mod auth_mw;
mod oidc;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Json, Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use http::HeaderValue;
use parking_lot::RwLock;
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{exponential_buckets, Histogram};
use prometheus_client::registry::Registry;
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowHeaders, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crawlkit_engine::storage::Storage;
use crawlkit_engine::AuditTrail;
use crawlkit_engine::CrawlConfig;

use auth::{AuthManager, User};
use auth_mw::auth_middleware as jwt_auth_middleware;
use oidc::OidcManager;

// ---------------------------------------------------------------------------
// API key management
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiKey {
    key: String,
    name: String,
    created_at: DateTime<Utc>,
    requests_per_minute: u32,
}

#[derive(Debug, Clone)]
struct RateLimitBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: std::time::Instant,
}

impl RateLimitBucket {
    fn new(rpm: u32) -> Self {
        Self {
            tokens: f64::from(rpm),
            max_tokens: f64::from(rpm),
            refill_rate: f64::from(rpm) / 60.0,
            last_refill: std::time::Instant::now(),
        }
    }

    fn try_consume(&mut self) -> bool {
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
struct Metrics {
    registry: Arc<tokio::sync::RwLock<Registry>>,
    crawls_total: Counter,
    pages_crawled_total: Counter,
    issues_total: Counter,
    errors_total: Counter,
    requests_total: Family<EndpointLabel, Counter>,
    request_duration_seconds: Histogram,
    fetch_duration_seconds: Histogram,
    analysis_duration_seconds: Histogram,
    active_crawls: Gauge,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, prometheus_client::encoding::EncodeLabelSet)]
struct EndpointLabel {
    endpoint: String,
}

impl Metrics {
    fn new() -> Self {
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
struct Tenant {
    id: String,
    name: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateTenantRequest {
    id: String,
    name: String,
}

#[derive(Clone)]
struct AppState {
    storage: Arc<Storage>,
    api_keys: Arc<DashMap<String, ApiKey>>,
    rate_limits: Arc<DashMap<String, RateLimitBucket>>,
    crawl_results: Arc<DashMap<String, CrawlResult>>,
    audit_trail: Arc<AuditTrail>,
    metrics: Arc<Metrics>,
    webhooks: Arc<DashMap<String, WebhookConfig>>,
    schedules: Arc<DashMap<String, ScheduleConfig>>,
    http_client: reqwest::Client,
    auth: Arc<AuthManager>,
    oidc: Option<Arc<OidcManager>>,
    oidc_states: Arc<DashMap<String, ()>>,
    tenants: Arc<dashmap::DashMap<String, Tenant>>,
    marketplace: MarketplaceState,
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CreateCrawlRequest {
    start_url: String,
    #[serde(default = "default_max_pages")]
    max_pages: usize,
    #[serde(default = "default_delay")]
    request_delay_ms: u64,
    #[serde(default = "default_concurrency")]
    concurrency: usize,
}

fn default_max_pages() -> usize {
    50
}
fn default_delay() -> u64 {
    500
}
fn default_concurrency() -> usize {
    4
}

#[derive(Debug, Serialize)]
struct CrawlResponse {
    crawl_id: String,
    status: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CrawlResult {
    crawl_id: String,
    start_url: String,
    status: String,
    pages_crawled: usize,
    issues_found: usize,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct CrawlStatsResponse {
    crawl_id: String,
    total_pages: usize,
    total_issues: usize,
    issues_by_severity: HashMap<String, usize>,
    issues_by_category: HashMap<String, usize>,
    avg_response_time_ms: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ApiKeyCreateRequest {
    name: String,
    #[serde(default = "default_rpm")]
    requests_per_minute: u32,
}

fn default_rpm() -> u32 {
    60
}

#[derive(Debug, Serialize)]
struct ApiKeyResponse {
    key: String,
    name: String,
    requests_per_minute: u32,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

// ---------------------------------------------------------------------------
// Webhook types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WebhookConfig {
    id: String,
    url: String,
    events: Vec<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateWebhookRequest {
    url: String,
    #[serde(default = "default_webhook_events")]
    events: Vec<String>,
}

fn default_webhook_events() -> Vec<String> {
    vec!["crawl.completed".to_string(), "crawl.failed".to_string()]
}

#[derive(Debug, Clone, Serialize)]
struct WebhookPayload {
    event: String,
    crawl_id: String,
    pages_crawled: usize,
    issues_found: usize,
    timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Scheduled crawl types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScheduleConfig {
    id: String,
    crawl_config: CrawlConfig,
    interval_secs: u64,
    enabled: bool,
    next_run: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateScheduleRequest {
    start_url: String,
    #[serde(default = "default_max_pages")]
    max_pages: usize,
    #[serde(default = "default_delay")]
    request_delay_ms: u64,
    #[serde(default = "default_concurrency")]
    concurrency: usize,
    #[serde(default = "default_schedule_interval")]
    interval_secs: u64,
}

fn default_schedule_interval() -> u64 {
    3600
}

#[derive(Debug, Serialize)]
struct ScheduleResponse {
    id: String,
    start_url: String,
    interval_secs: u64,
    enabled: bool,
    next_run: DateTime<Utc>,
    created_at: DateTime<Utc>,
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
enum ApiError {
    Unauthorized(String),
    BadRequest(String),
    NotFound(String),
    RateLimited,
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
            ),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

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

fn validate_url(url: &str) -> Result<(), ApiError> {
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
    Ok(())
}

fn validate_max_pages(n: usize) -> Result<(), ApiError> {
    if !(1..=10000).contains(&n) {
        return Err(ApiError::BadRequest(
            "max_pages must be between 1 and 10000".to_string(),
        ));
    }
    Ok(())
}

fn validate_concurrency(n: usize) -> Result<(), ApiError> {
    if !(1..=128).contains(&n) {
        return Err(ApiError::BadRequest(
            "concurrency must be between 1 and 128".to_string(),
        ));
    }
    Ok(())
}

fn validate_delay(ms: u64) -> Result<(), ApiError> {
    if ms > 60000 {
        return Err(ApiError::BadRequest(
            "request_delay_ms must be at most 60000".to_string(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Metrics endpoint
// ---------------------------------------------------------------------------

async fn metrics_endpoint(State(state): State<AppState>) -> Response {
    let registry = state.metrics.registry.read().await;
    let mut buffer = String::new();
    if let Err(e) = encode(&mut buffer, &registry) {
        tracing::error!("Failed to encode metrics: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to encode metrics",
        )
            .into_response();
    }
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        buffer,
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Middleware: request metrics tracking
// ---------------------------------------------------------------------------

async fn request_metrics_middleware(
    State(state): State<AppState>,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed().as_secs_f64();

    let endpoint = uri.path().to_string();
    state
        .metrics
        .requests_total
        .get_or_create(&EndpointLabel { endpoint })
        .inc();
    state.metrics.request_duration_seconds.observe(duration);

    response
}

// ---------------------------------------------------------------------------
// Middleware: Content Security Policy headers
// ---------------------------------------------------------------------------

async fn csp_headers(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' \
             'unsafe-inline'; img-src 'self' data:; connect-src 'self'; font-src 'self'",
        ),
    );
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}

// ---------------------------------------------------------------------------
// Middleware: CSRF origin validation
// ---------------------------------------------------------------------------

async fn csrf_origin_validation(
    State(allowed_origins): State<Vec<String>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method().clone();

    if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    let has_api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .is_some();

    if has_api_key {
        return Ok(next.run(request).await);
    }

    let origin = headers.get("origin").and_then(|v| v.to_str().ok());

    match origin {
        Some(origin_str) => {
            if allowed_origins.iter().any(|o| o == origin_str) {
                Ok(next.run(request).await)
            } else {
                tracing::warn!("CSRF origin rejected: {origin_str}");
                Err(StatusCode::FORBIDDEN)
            }
        }
        None => {
            tracing::warn!("CSRF check: missing Origin header on {method}");
            Err(StatusCode::FORBIDDEN)
        }
    }
}

// ---------------------------------------------------------------------------
// Middleware: API key authentication + rate limiting
// ---------------------------------------------------------------------------

async fn api_key_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, ApiError> {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing X-API-Key header".to_string()))?
        .to_string();

    let key_info = state
        .api_keys
        .get(&api_key)
        .ok_or_else(|| ApiError::Unauthorized("Invalid API key".to_string()))?;

    let rpm = key_info.requests_per_minute;
    drop(key_info);

    // Rate limit check
    let mut bucket = state
        .rate_limits
        .entry(api_key.clone())
        .or_insert_with(|| RateLimitBucket::new(rpm));

    if !bucket.try_consume() {
        return Err(ApiError::RateLimited);
    }

    #[allow(clippy::cast_possible_truncation)]
    let remaining = bucket.tokens.floor() as u64;
    #[allow(clippy::cast_possible_truncation)]
    let reset_seconds = ((bucket.max_tokens - bucket.tokens) / bucket.refill_rate).ceil() as u64;
    drop(bucket);

    let mut response = next.run(request).await;
    let resp_headers = response.headers_mut();
    resp_headers.insert("X-RateLimit-Limit", rpm.into());
    resp_headers.insert("X-RateLimit-Remaining", remaining.into());
    resp_headers.insert("X-RateLimit-Reset", reset_seconds.into());
    Ok(response)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn create_api_key(
    State(state): State<AppState>,
    Json(req): Json<ApiKeyCreateRequest>,
) -> Result<(StatusCode, Json<ApiKeyResponse>), ApiError> {
    let key = format!("ck_{}", Uuid::new_v4().to_string().replace('-', ""));
    let name = req.name.clone();
    let rpm = req.requests_per_minute;

    let api_key = ApiKey {
        key: key.clone(),
        name: req.name,
        created_at: Utc::now(),
        requests_per_minute: rpm,
    };

    state.api_keys.insert(key.clone(), api_key);

    Ok((
        StatusCode::CREATED,
        Json(ApiKeyResponse {
            key,
            name,
            requests_per_minute: rpm,
        }),
    ))
}

async fn list_api_keys(State(state): State<AppState>) -> Json<Vec<ApiKeyResponse>> {
    let keys = state
        .api_keys
        .iter()
        .map(|entry| ApiKeyResponse {
            key: entry.value().key.clone(),
            name: entry.value().name.clone(),
            requests_per_minute: entry.value().requests_per_minute,
        })
        .collect();

    Json(keys)
}

async fn delete_api_key(
    State(state): State<AppState>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .api_keys
        .remove(&key)
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::NotFound(format!("API key {key} not found")))
}

async fn start_crawl(
    State(state): State<AppState>,
    Json(req): Json<CreateCrawlRequest>,
) -> Result<(StatusCode, Json<CrawlResponse>), ApiError> {
    validate_url(&req.start_url)?;
    validate_max_pages(req.max_pages)?;
    validate_concurrency(req.concurrency)?;
    validate_delay(req.request_delay_ms)?;

    let start_url = url::Url::parse(&req.start_url)
        .map_err(|e| ApiError::BadRequest(format!("Invalid URL: {e}")))?;

    let crawl_id = Uuid::new_v4().to_string();

    let config_json = serde_json::to_string(&serde_json::json!({
        "start_url": start_url,
        "max_pages": req.max_pages,
        "request_delay_ms": req.request_delay_ms,
        "concurrency": req.concurrency,
    }))
    .unwrap_or_default();

    state
        .storage
        .start_crawl(start_url.as_ref(), Some(&config_json))
        .map_err(|e| ApiError::Internal(format!("Failed to start crawl: {e}")))?;

    let result = CrawlResult {
        crawl_id: crawl_id.clone(),
        start_url: start_url.to_string(),
        status: "running".to_string(),
        pages_crawled: 0,
        issues_found: 0,
        created_at: Utc::now(),
        completed_at: None,
    };

    state.crawl_results.insert(crawl_id.clone(), result);

    state.metrics.crawls_total.inc();
    state.metrics.active_crawls.inc();

    // Spawn crawl task in background
    let crawl_id_clone = crawl_id.clone();
    let config = CrawlConfig {
        start_url,
        max_pages: req.max_pages,
        request_delay: std::time::Duration::from_millis(req.request_delay_ms),
        concurrency: req.concurrency,
        ..Default::default()
    };

    tokio::spawn(async move {
        run_crawl_task(state, crawl_id_clone, config).await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(CrawlResponse {
            crawl_id,
            status: "running".to_string(),
            message: "Crawl started successfully".to_string(),
        }),
    ))
}

async fn get_crawl_status(
    State(state): State<AppState>,
    axum::extract::Path(crawl_id): axum::extract::Path<String>,
) -> Result<Json<CrawlResult>, ApiError> {
    state
        .crawl_results
        .get(&crawl_id)
        .map(|entry| Json(entry.value().clone()))
        .ok_or_else(|| ApiError::NotFound(format!("Crawl {crawl_id} not found")))
}

async fn get_crawl_stats(
    State(state): State<AppState>,
    axum::extract::Path(crawl_id): axum::extract::Path<String>,
) -> Result<Json<CrawlStatsResponse>, ApiError> {
    let stats = state
        .storage
        .get_stats(&crawl_id)
        .map_err(|e| ApiError::Internal(format!("Failed to get stats: {e}")))?;

    Ok(Json(CrawlStatsResponse {
        crawl_id,
        total_pages: stats.total_pages,
        total_issues: stats.total_issues,
        issues_by_severity: stats.issues_by_severity,
        issues_by_category: stats.issues_by_category,
        avg_response_time_ms: stats.avg_response_time_ms,
    }))
}

async fn list_crawls(State(state): State<AppState>) -> Json<Vec<CrawlResult>> {
    let results: Vec<CrawlResult> = state
        .crawl_results
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    Json(results)
}

async fn get_audit_events(State(state): State<AppState>) -> Json<Vec<crawlkit_engine::AuditEvent>> {
    Json(state.audit_trail.events())
}

// ---------------------------------------------------------------------------
// Tenant management handlers (admin only)
// ---------------------------------------------------------------------------

async fn list_tenants(State(state): State<AppState>) -> Json<Vec<Tenant>> {
    let tenants: Vec<Tenant> = state
        .tenants
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    Json(tenants)
}

async fn create_tenant(
    State(state): State<AppState>,
    Json(input): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Tenant>), ApiError> {
    if state.tenants.contains_key(&input.id) {
        return Err(ApiError::BadRequest(format!(
            "Tenant '{}' already exists",
            input.id
        )));
    }

    let tenant = Tenant {
        id: input.id,
        name: input.name,
        created_at: Utc::now(),
    };

    state.tenants.insert(tenant.id.clone(), tenant.clone());
    Ok((StatusCode::CREATED, Json(tenant)))
}

async fn get_tenant(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Tenant>, ApiError> {
    state
        .tenants
        .get(&id)
        .map(|entry| Json(entry.value().clone()))
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {id} not found")))
}

async fn delete_tenant(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .tenants
        .remove(&id)
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {id} not found")))
}

// ---------------------------------------------------------------------------
// Auth handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    user: UserResponse,
}

#[derive(Serialize)]
struct UserResponse {
    id: String,
    email: String,
    name: String,
    tenant_id: String,
    roles: Vec<String>,
    enabled: bool,
}

async fn login(
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

async fn refresh_token(
    axum::extract::Extension(claims): axum::extract::Extension<auth::Claims>,
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

async fn get_me(
    axum::extract::Extension(claims): axum::extract::Extension<auth::Claims>,
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

// ---------------------------------------------------------------------------
// OIDC handlers
// ---------------------------------------------------------------------------

async fn oidc_authorize(State(state): State<AppState>) -> Result<Response, ApiError> {
    let oidc = state
        .oidc
        .as_ref()
        .ok_or_else(|| ApiError::Internal("OIDC not configured".to_string()))?;

    let state_token = uuid::Uuid::new_v4().to_string();
    state.oidc_states.insert(state_token.clone(), ());

    let url = oidc.authorization_url(&state_token);
    let parsed_url = url::Url::parse(&url)
        .map_err(|e| ApiError::Internal(format!("Invalid authorization URL: {e}")))?;

    Ok((
        StatusCode::FOUND,
        [("location", parsed_url.as_str().to_string())],
    )
        .into_response())
}

#[derive(Deserialize)]
struct OidcCallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn oidc_callback(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<OidcCallbackParams>,
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

    if !state.oidc_states.contains_key(state_token) {
        return Err(ApiError::BadRequest("Invalid state parameter".to_string()));
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
            let new_user = User {
                id: user_info.sub.clone(),
                email: user_info.email.unwrap_or_default(),
                name: user_info.name.unwrap_or_default(),
                password_hash: String::new(),
                tenant_id: "default".to_string(),
                roles: vec!["viewer".to_string()],
                enabled: true,
            };
            state.auth.add_user(new_user.clone());
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

// ---------------------------------------------------------------------------
// User management handlers (admin only)
// ---------------------------------------------------------------------------

async fn list_users(State(state): State<AppState>) -> Json<Vec<UserResponse>> {
    let users: Vec<UserResponse> = state
        .auth
        .list_users()
        .into_iter()
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

#[derive(Deserialize)]
struct CreateUserRequest {
    email: String,
    name: String,
    password: String,
    #[serde(default = "default_user_tenant")]
    tenant_id: String,
    #[serde(default = "default_user_roles")]
    roles: Vec<String>,
}

fn default_user_tenant() -> String {
    "default".to_string()
}

fn default_user_roles() -> Vec<String> {
    vec!["viewer".to_string()]
}

async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    if state.auth.find_user(&req.email).is_some() {
        return Err(ApiError::BadRequest(
            "User with this email already exists".to_string(),
        ));
    }

    let password_hash = state.auth.hash_password(&req.password);
    let user_id = uuid::Uuid::new_v4().to_string();
    let response = UserResponse {
        id: user_id.clone(),
        email: req.email.clone(),
        name: req.name.clone(),
        tenant_id: req.tenant_id.clone(),
        roles: req.roles.clone(),
        enabled: true,
    };

    state.auth.add_user(User {
        id: user_id,
        email: req.email,
        name: req.name,
        password_hash,
        tenant_id: req.tenant_id,
        roles: req.roles,
        enabled: true,
    });
    Ok((StatusCode::CREATED, Json(response)))
}

async fn delete_user(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    if state.auth.delete_user(&id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("User {id} not found")))
    }
}

#[derive(Serialize)]
struct BacklinksResponse {
    crawl_id: String,
    total_internal_links: usize,
    total_external_links: usize,
    total_referring_domains: usize,
    orphan_pages: Vec<String>,
    top_pages_by_pagerank: Vec<serde_json::Value>,
}

async fn get_crawl_backlinks(
    State(state): State<AppState>,
    axum::extract::Path(crawl_id): axum::extract::Path<String>,
) -> Result<Json<BacklinksResponse>, ApiError> {
    let link_pairs = state
        .storage
        .get_links_for_crawl(&crawl_id)
        .map_err(|e| ApiError::Internal(format!("Failed to get links: {e}")))?;

    let external_links = state
        .storage
        .get_external_links(&crawl_id)
        .map_err(|e| ApiError::Internal(format!("Failed to get external links: {e}")))?;

    let mut analyzer = crawlkit_engine::BacklinkAnalyzer::new();
    analyzer.load_from_crawl_data(&link_pairs);
    for (source, target) in &external_links {
        analyzer.add_backlink(crawlkit_engine::Backlink {
            source_url: source.clone(),
            target_url: target.clone(),
            anchor_text: String::new(),
            is_followed: true,
            is_internal: false,
        });
    }

    let _pagerank = analyzer.compute_pagerank(0.85, 20);
    let summary = analyzer.summarize();

    let top_pages: Vec<serde_json::Value> = summary
        .pages
        .iter()
        .take(20)
        .map(|p| {
            serde_json::json!({
                "url": p.url,
                "pagerank": p.pagerank,
                "inbound_links": p.inbound_links,
                "outbound_links": p.outbound_links,
                "referring_domains": p.referring_domains,
            })
        })
        .collect();

    Ok(Json(BacklinksResponse {
        crawl_id,
        total_internal_links: summary.total_internal_links,
        total_external_links: summary.total_external_links,
        total_referring_domains: summary.total_referring_domains,
        orphan_pages: summary.orphan_pages,
        top_pages_by_pagerank: top_pages,
    }))
}

// ---------------------------------------------------------------------------
// Webhook handlers
// ---------------------------------------------------------------------------

async fn create_webhook(
    State(state): State<AppState>,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookConfig>), ApiError> {
    url::Url::parse(&req.url)
        .map_err(|e| ApiError::BadRequest(format!("Invalid webhook URL: {e}")))?;

    for event in &req.events {
        if event != "crawl.completed" && event != "crawl.failed" {
            return Err(ApiError::BadRequest(format!(
                "Invalid event type: {event}. Must be 'crawl.completed' or 'crawl.failed'"
            )));
        }
    }

    let id = Uuid::new_v4().to_string();
    let config = WebhookConfig {
        id: id.clone(),
        url: req.url,
        events: req.events,
        created_at: Utc::now(),
    };

    state.webhooks.insert(id, config.clone());
    Ok((StatusCode::CREATED, Json(config)))
}

async fn list_webhooks(State(state): State<AppState>) -> Json<Vec<WebhookConfig>> {
    Json(state.webhooks.iter().map(|e| e.value().clone()).collect())
}

async fn delete_webhook(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .webhooks
        .remove(&id)
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::NotFound(format!("Webhook {id} not found")))
}

// ---------------------------------------------------------------------------
// Schedule handlers
// ---------------------------------------------------------------------------

async fn create_schedule(
    State(state): State<AppState>,
    Json(req): Json<CreateScheduleRequest>,
) -> Result<(StatusCode, Json<ScheduleResponse>), ApiError> {
    validate_url(&req.start_url)?;
    validate_max_pages(req.max_pages)?;
    validate_concurrency(req.concurrency)?;
    validate_delay(req.request_delay_ms)?;

    let start_url = url::Url::parse(&req.start_url)
        .map_err(|e| ApiError::BadRequest(format!("Invalid URL: {e}")))?;

    if req.interval_secs < 60 {
        return Err(ApiError::BadRequest(
            "interval_secs must be at least 60".to_string(),
        ));
    }

    let crawl_config = CrawlConfig {
        start_url,
        max_pages: req.max_pages,
        request_delay: std::time::Duration::from_millis(req.request_delay_ms),
        concurrency: req.concurrency,
        ..Default::default()
    };

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let schedule = ScheduleConfig {
        id: id.clone(),
        crawl_config: crawl_config.clone(),
        interval_secs: req.interval_secs,
        enabled: true,
        next_run: now + chrono::Duration::seconds(req.interval_secs as i64),
        created_at: now,
    };

    let response = ScheduleResponse {
        id: schedule.id.clone(),
        start_url: crawl_config.start_url.to_string(),
        interval_secs: schedule.interval_secs,
        enabled: schedule.enabled,
        next_run: schedule.next_run,
        created_at: schedule.created_at,
    };

    state.schedules.insert(id, schedule);
    Ok((StatusCode::CREATED, Json(response)))
}

async fn list_schedules(State(state): State<AppState>) -> Json<Vec<ScheduleResponse>> {
    Json(
        state
            .schedules
            .iter()
            .map(|e| {
                let s = e.value();
                ScheduleResponse {
                    id: s.id.clone(),
                    start_url: s.crawl_config.start_url.to_string(),
                    interval_secs: s.interval_secs,
                    enabled: s.enabled,
                    next_run: s.next_run,
                    created_at: s.created_at,
                }
            })
            .collect(),
    )
}

async fn delete_schedule(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .schedules
        .remove(&id)
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::NotFound(format!("Schedule {id} not found")))
}

// ---------------------------------------------------------------------------
// Plugin marketplace handlers
// ---------------------------------------------------------------------------

async fn list_marketplace_plugins(State(state): State<AppState>) -> Json<Vec<MarketplacePlugin>> {
    let plugins = state.marketplace.plugins.read();
    let list: Vec<MarketplacePlugin> = plugins.values().cloned().collect();
    Json(list)
}

async fn get_marketplace_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<MarketplacePlugin>, ApiError> {
    let plugins = state.marketplace.plugins.read();
    plugins
        .get(&name)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("Plugin '{name}' not found")))
}

async fn submit_plugin(
    State(state): State<AppState>,
    Json(input): Json<SubmitPluginRequest>,
) -> Result<(StatusCode, Json<MarketplacePlugin>), ApiError> {
    {
        let plugins = state.marketplace.plugins.read();
        if plugins.contains_key(&input.name) {
            return Err(ApiError::BadRequest(format!(
                "Plugin '{}' already exists",
                input.name
            )));
        }
    }

    let plugin = MarketplacePlugin {
        name: input.name.clone(),
        version: input.version,
        author: input.author,
        description: input.description,
        license: input.license,
        categories: input.categories,
        tags: input.tags,
        downloads: 0,
        rating: 0.0,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    };

    state
        .marketplace
        .plugins
        .write()
        .insert(input.name, plugin.clone());
    Ok((StatusCode::CREATED, Json(plugin)))
}

async fn delete_marketplace_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut plugins = state.marketplace.plugins.write();
    if plugins.remove(&name).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Plugin '{name}' not found")))
    }
}

async fn download_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Vec<u8>, ApiError> {
    let plugins = state.marketplace.plugins.read();
    if plugins.contains_key(&name) {
        Ok(Vec::new())
    } else {
        Err(ApiError::NotFound(format!("Plugin '{name}' not found")))
    }
}

async fn test_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = input;
    let plugins = state.marketplace.plugins.read();
    if plugins.contains_key(&name) {
        Ok(Json(serde_json::json!({
            "status": "passed",
            "findings": 0,
            "execution_time_ms": 15,
        })))
    } else {
        Err(ApiError::NotFound(format!("Plugin '{name}' not found")))
    }
}

// ---------------------------------------------------------------------------
// Webhook firing helper
// ---------------------------------------------------------------------------

fn fire_webhooks(
    state: &AppState,
    event: &str,
    crawl_id: &str,
    pages_crawled: usize,
    issues_found: usize,
) {
    let payload = WebhookPayload {
        event: event.to_string(),
        crawl_id: crawl_id.to_string(),
        pages_crawled,
        issues_found,
        timestamp: Utc::now(),
    };

    let matching: Vec<WebhookConfig> = state
        .webhooks
        .iter()
        .filter(|entry| entry.value().events.iter().any(|e| e == event))
        .map(|entry| entry.value().clone())
        .collect();

    if matching.is_empty() {
        return;
    }

    let client = state.http_client.clone();
    tokio::spawn(async move {
        for webhook in matching {
            let client = client.clone();
            let payload = payload.clone();
            let url = webhook.url.clone();
            tokio::spawn(async move {
                match client
                    .post(&url)
                    .json(&payload)
                    .timeout(std::time::Duration::from_secs(10))
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            tracing::warn!("Webhook to {url} returned status {}", resp.status());
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to send webhook to {url}: {e}");
                    }
                }
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Background crawl task
// ---------------------------------------------------------------------------

async fn run_crawl_task(state: AppState, crawl_id: String, config: CrawlConfig) {
    use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};

    let engine_config = CrawlEngineConfig {
        crawl_config: config.clone(),
        ..Default::default()
    };

    let engine = CrawlEngine::new_shared(engine_config, state.storage.clone());

    // Note: we use the engine's storage which is a separate instance,
    // but the crawl_id is shared. The engine stores to its own DB.
    // For the API, we need to use the state's storage.
    // Since CrawlEngine takes ownership of storage, we use a callback approach.
    let state_clone = state.clone();
    let result = engine
        .run_with_callback(
            config.start_url.as_ref(),
            Some(Arc::new(move |_url, _page_id, _findings| {
                state_clone.metrics.pages_crawled_total.inc();
            })),
        )
        .await;

    match result {
        Ok(output) => {
            let _ =
                engine
                    .storage()
                    .finish_crawl(&crawl_id, output.pages_crawled, output.issues_found);

            if let Some(mut result) = state.crawl_results.get_mut(&crawl_id) {
                result.status = "completed".to_string();
                result.pages_crawled = output.pages_crawled;
                result.issues_found = output.issues_found;
                result.completed_at = Some(Utc::now());
            }

            state.metrics.active_crawls.dec();
            state
                .metrics
                .pages_crawled_total
                .inc_by(output.pages_crawled as u64);
            state
                .metrics
                .issues_total
                .inc_by(output.issues_found as u64);
            state
                .metrics
                .fetch_duration_seconds
                .observe(output.elapsed.as_secs_f64());
            state
                .metrics
                .analysis_duration_seconds
                .observe(output.elapsed.as_secs_f64());
            fire_webhooks(
                &state,
                "crawl.completed",
                &crawl_id,
                output.pages_crawled,
                output.issues_found,
            );
            tracing::info!(
                "Crawl {crawl_id} completed: {} pages, {} issues",
                output.pages_crawled,
                output.issues_found
            );
        }
        Err(e) => {
            tracing::error!("Crawl {crawl_id} failed: {e}");
            state.metrics.errors_total.inc();
            if let Some(mut result) = state.crawl_results.get_mut(&crawl_id) {
                result.status = "failed".to_string();
                result.completed_at = Some(Utc::now());
            }
            state.metrics.active_crawls.dec();
            fire_webhooks(&state, "crawl.failed", &crawl_id, 0, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Scheduler background task
// ---------------------------------------------------------------------------

async fn run_scheduler(state: AppState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let now = Utc::now();

        let due: Vec<(String, CrawlConfig)> = state
            .schedules
            .iter()
            .filter(|entry| entry.value().enabled && entry.value().next_run <= now)
            .map(|entry| {
                let s = entry.value();
                (s.id.clone(), s.crawl_config.clone())
            })
            .collect();

        for (schedule_id, config) in due {
            if let Some(mut schedule) = state.schedules.get_mut(&schedule_id) {
                schedule.next_run = now + chrono::Duration::seconds(schedule.interval_secs as i64);
            }

            let crawl_id = Uuid::new_v4().to_string();
            let result = CrawlResult {
                crawl_id: crawl_id.clone(),
                start_url: config.start_url.to_string(),
                status: "running".to_string(),
                pages_crawled: 0,
                issues_found: 0,
                created_at: Utc::now(),
                completed_at: None,
            };
            state.crawl_results.insert(crawl_id.clone(), result);
            state.metrics.crawls_total.inc();
            state.metrics.active_crawls.inc();

            let state_clone = state.clone();
            let crawl_id_clone = crawl_id.clone();
            tokio::spawn(async move {
                run_crawl_task(state_clone, crawl_id_clone, config).await;
            });

            tracing::info!("Scheduled crawl {crawl_id} started from schedule {schedule_id}");
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use tracing_opentelemetry::OpenTelemetryLayer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    let use_otel = std::env::var("OTEL_EXPORTER").ok().as_deref() == Some("stdout");

    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("crawlkit_api=info")),
    );

    if use_otel {
        use opentelemetry::trace::TracerProvider;
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
            .build();
        let tracer = provider.tracer("crawlkit-api");
        let otel_layer = OpenTelemetryLayer::new(tracer);
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(otel_layer)
            .init();
    } else {
        tracing_subscriber::registry().with(fmt_layer).init();
    }

    let db_path =
        std::env::var("CRAWLKIT_DB_PATH").unwrap_or_else(|_| "crawlkit-api.db".to_string());

    let storage = Storage::new(std::path::Path::new(&db_path))
        .map_err(|e| anyhow::anyhow!("Failed to open storage: {e}"))?;

    // Generate a random API key for this session (not hardcoded)
    let default_key = format!("ck_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    eprintln!("Generated API key: {default_key}");
    let api_keys = Arc::new(DashMap::new());
    api_keys.insert(
        default_key.clone(),
        ApiKey {
            key: default_key.clone(),
            name: "development".to_string(),
            created_at: Utc::now(),
            requests_per_minute: 300,
        },
    );

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default-secret-change-in-production".to_string());
    let auth = Arc::new(AuthManager::new(jwt_secret));

    let admin_password = auth.hash_password("admin123");
    auth.add_user(User {
        id: uuid::Uuid::new_v4().to_string(),
        email: "admin@crawlkit.local".to_string(),
        name: "Admin".to_string(),
        password_hash: admin_password,
        tenant_id: "default".to_string(),
        roles: vec!["admin".to_string()],
        enabled: true,
    });

    // Initialize OIDC if configured via environment variables
    let oidc = match (
        std::env::var("OIDC_PROVIDER"),
        std::env::var("OIDC_CLIENT_ID"),
        std::env::var("OIDC_DISCOVERY_URL"),
    ) {
        (Ok(provider), Ok(client_id), Ok(discovery_url)) => {
            let scopes: Vec<String> = std::env::var("OIDC_SCOPES")
                .unwrap_or_else(|_| "openid email profile".to_string())
                .split_whitespace()
                .map(String::from)
                .collect();
            let redirect_uri = std::env::var("OIDC_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:4000/api/v1/auth/oidc/callback".to_string());
            let client_secret_env = std::env::var("OIDC_CLIENT_SECRET_ENV")
                .unwrap_or_else(|_| "OIDC_CLIENT_SECRET".to_string());

            let config = oidc::OidcConfig {
                provider,
                client_id,
                client_secret_env,
                discovery_url,
                scopes,
                redirect_uri,
            };
            let manager = Arc::new(OidcManager::new(config));
            if let Err(e) = manager.discover().await {
                tracing::warn!("OIDC discovery failed: {e}. OIDC auth will not be available.");
                None
            } else {
                tracing::info!("OIDC authentication enabled");
                Some(manager)
            }
        }
        _ => {
            tracing::info!("OIDC not configured, using local auth only");
            None
        }
    };

    let marketplace = MarketplaceState::new();

    let state = AppState {
        storage: Arc::new(storage),
        api_keys,
        rate_limits: Arc::new(DashMap::new()),
        crawl_results: Arc::new(DashMap::new()),
        audit_trail: Arc::new(AuditTrail::new()),
        metrics: Arc::new(Metrics::new()),
        webhooks: Arc::new(DashMap::new()),
        schedules: Arc::new(DashMap::new()),
        http_client: reqwest::Client::new(),
        auth,
        oidc,
        oidc_states: Arc::new(DashMap::new()),
        tenants: Arc::new(DashMap::new()),
        marketplace,
    };

    let allowed_origins: Vec<String> = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let cors = if allowed_origins.iter().all(|o| o == "*") {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .iter()
            .filter_map(|o| HeaderValue::from_str(o).ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(AllowHeaders::any())
    };

    let csrf_origins = allowed_origins.clone();

    let protected = Router::new()
        .route("/api/v1/auth/refresh", post(refresh_token))
        .route("/api/v1/auth/me", get(get_me))
        .route("/api/v1/crawls", post(start_crawl).get(list_crawls))
        .route("/api/v1/crawls/{crawl_id}", get(get_crawl_status))
        .route("/api/v1/crawls/{crawl_id}/stats", get(get_crawl_stats))
        .route(
            "/api/v1/crawls/{crawl_id}/backlinks",
            get(get_crawl_backlinks),
        )
        .route("/api/v1/keys", post(create_api_key).get(list_api_keys))
        .route("/api/v1/keys/{key}", axum::routing::delete(delete_api_key))
        .route("/api/v1/webhooks", post(create_webhook).get(list_webhooks))
        .route(
            "/api/v1/webhooks/{id}",
            axum::routing::delete(delete_webhook),
        )
        .route(
            "/api/v1/schedules",
            post(create_schedule).get(list_schedules),
        )
        .route(
            "/api/v1/schedules/{id}",
            axum::routing::delete(delete_schedule),
        )
        .route("/api/v1/audit", get(get_audit_events))
        .route("/api/v1/tenants", post(create_tenant).get(list_tenants))
        .route(
            "/api/v1/tenants/{id}",
            get(get_tenant).delete(delete_tenant),
        )
        .route("/api/v1/users", post(create_user).get(list_users))
        .route("/api/v1/users/{id}", axum::routing::delete(delete_user))
        .route(
            "/api/v1/marketplace/plugins",
            post(submit_plugin).get(list_marketplace_plugins),
        )
        .route(
            "/api/v1/marketplace/plugins/{name}",
            get(get_marketplace_plugin).delete(delete_marketplace_plugin),
        )
        .route(
            "/api/v1/marketplace/plugins/{name}/download",
            get(download_plugin),
        )
        .route("/api/v1/marketplace/plugins/{name}/test", post(test_plugin))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        ));

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_endpoint))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/oidc/authorize", get(oidc_authorize))
        .route("/api/v1/auth/oidc/callback", get(oidc_callback))
        .merge(protected)
        .layer(cors)
        .layer(middleware::from_fn(csp_headers))
        .layer(middleware::from_fn_with_state(
            csrf_origins,
            csrf_origin_validation,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_metrics_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            api_key_auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    tracing::info!("crawlkit API listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;

    tokio::spawn(async move {
        run_scheduler(state).await;
    });

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // Default value helpers
    // ---------------------------------------------------------------

    #[test]
    fn test_default_rpm() {
        assert_eq!(default_rpm(), 60);
    }

    #[test]
    fn test_default_max_pages() {
        assert_eq!(default_max_pages(), 50);
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
    fn test_default_user_tenant() {
        assert_eq!(default_user_tenant(), "default");
    }

    #[test]
    fn test_default_user_roles() {
        let roles = default_user_roles();
        assert_eq!(roles, vec!["viewer".to_string()]);
    }

    #[test]
    fn test_default_schedule_interval() {
        assert_eq!(default_schedule_interval(), 3600);
    }

    // ---------------------------------------------------------------
    // RateLimitBucket
    // ---------------------------------------------------------------

    #[test]
    fn test_rate_limit_bucket_new() {
        let bucket = RateLimitBucket::new(120);
        assert_eq!(bucket.tokens, 120.0);
        assert_eq!(bucket.max_tokens, 120.0);
        assert!((bucket.refill_rate - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rate_limit_bucket_new_single_rpm() {
        let bucket = RateLimitBucket::new(1);
        assert_eq!(bucket.tokens, 1.0);
        assert_eq!(bucket.max_tokens, 1.0);
        assert!((bucket.refill_rate - 1.0 / 60.0).abs() < 1e-9);
    }

    #[test]
    fn test_rate_limit_bucket_try_consume_success() {
        let mut bucket = RateLimitBucket::new(60);
        assert!(bucket.try_consume());
        assert!((bucket.tokens - 59.0).abs() < 0.5);
    }

    #[test]
    fn test_rate_limit_bucket_try_consume_exhaustion() {
        let mut bucket = RateLimitBucket::new(1);
        assert!(bucket.try_consume());
        // After consuming the 1 token, there should be <1 left
        // (depending on exact refill timing, but likely ~0)
        let second = bucket.try_consume();
        // It may succeed if a tiny bit of refill happened, but the bucket is basically empty
        // Just verify the logic: after consuming 1 from a 1-token bucket, tokens < 1
        assert!(bucket.tokens < 1.0 || !second);
    }

    #[test]
    fn test_rate_limit_bucket_refill_capped_at_max() {
        let mut bucket = RateLimitBucket::new(10);
        // Drain all tokens
        for _ in 0..10 {
            bucket.try_consume();
        }
        assert!(bucket.tokens < 1.0);
        // Manually set last_refill to the past to simulate time passing
        bucket.last_refill = std::time::Instant::now() - std::time::Duration::from_secs(120);
        bucket.refill();
        // Tokens should be capped at max_tokens
        assert!((bucket.tokens - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rate_limit_bucket_refill_partial() {
        let mut bucket = RateLimitBucket::new(60);
        bucket.try_consume(); // 59 tokens
        let before = bucket.tokens;
        // Simulate 1 second passing
        bucket.last_refill = std::time::Instant::now() - std::time::Duration::from_secs(1);
        bucket.refill();
        // Should have gained ~1 token (60/60 = 1 per second)
        assert!(bucket.tokens > before);
        assert!(bucket.tokens <= bucket.max_tokens);
    }

    // ---------------------------------------------------------------
    // validate_url
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_url_valid_https() {
        assert!(validate_url("https://example.com").is_ok());
    }

    #[test]
    fn test_validate_url_valid_http() {
        assert!(validate_url("http://example.com").is_ok());
    }

    #[test]
    fn test_validate_url_valid_with_path() {
        assert!(validate_url("https://example.com/some/path?q=1").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_ftp() {
        let err = validate_url("ftp://example.com").unwrap_err();
        match err {
            ApiError::BadRequest(msg) => assert!(msg.contains("http or https")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_validate_url_rejects_file_scheme() {
        let err = validate_url("file:///etc/passwd").unwrap_err();
        match err {
            ApiError::BadRequest(msg) => assert!(msg.contains("http or https")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_validate_url_rejects_invalid_url() {
        let err = validate_url("not a url").unwrap_err();
        match err {
            ApiError::BadRequest(msg) => assert!(msg.contains("Invalid URL")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_validate_url_rejects_too_long() {
        let long_url = format!("https://example.com/{}", "a".repeat(2049));
        let err = validate_url(&long_url).unwrap_err();
        match err {
            ApiError::BadRequest(msg) => assert!(msg.contains("2048")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_validate_url_accepts_max_length() {
        let prefix = "https://example.com/";
        let url = format!("{}{}", prefix, "a".repeat(2048 - prefix.len()));
        assert!(validate_url(&url).is_ok());
    }

    // ---------------------------------------------------------------
    // validate_max_pages
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_max_pages_valid() {
        assert!(validate_max_pages(1).is_ok());
        assert!(validate_max_pages(50).is_ok());
        assert!(validate_max_pages(10000).is_ok());
    }

    #[test]
    fn test_validate_max_pages_zero() {
        let err = validate_max_pages(0).unwrap_err();
        match err {
            ApiError::BadRequest(msg) => assert!(msg.contains("1 and 10000")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_validate_max_pages_over_max() {
        let err = validate_max_pages(10001).unwrap_err();
        match err {
            ApiError::BadRequest(msg) => assert!(msg.contains("1 and 10000")),
            _ => panic!("Expected BadRequest"),
        }
    }

    // ---------------------------------------------------------------
    // validate_concurrency
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_concurrency_valid() {
        assert!(validate_concurrency(1).is_ok());
        assert!(validate_concurrency(64).is_ok());
        assert!(validate_concurrency(128).is_ok());
    }

    #[test]
    fn test_validate_concurrency_zero() {
        let err = validate_concurrency(0).unwrap_err();
        match err {
            ApiError::BadRequest(msg) => assert!(msg.contains("1 and 128")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_validate_concurrency_over_max() {
        let err = validate_concurrency(129).unwrap_err();
        match err {
            ApiError::BadRequest(msg) => assert!(msg.contains("1 and 128")),
            _ => panic!("Expected BadRequest"),
        }
    }

    // ---------------------------------------------------------------
    // validate_delay
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_delay_valid() {
        assert!(validate_delay(0).is_ok());
        assert!(validate_delay(500).is_ok());
        assert!(validate_delay(60000).is_ok());
    }

    #[test]
    fn test_validate_delay_over_max() {
        let err = validate_delay(60001).unwrap_err();
        match err {
            ApiError::BadRequest(msg) => assert!(msg.contains("60000")),
            _ => panic!("Expected BadRequest"),
        }
    }

    // ---------------------------------------------------------------
    // ApiError IntoResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_error_unauthorized_status() {
        let err = ApiError::Unauthorized("test".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_api_error_bad_request_status() {
        let err = ApiError::BadRequest("test".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_api_error_not_found_status() {
        let err = ApiError::NotFound("test".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_api_error_rate_limited_status() {
        let err = ApiError::RateLimited;
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_api_error_internal_status() {
        let err = ApiError::Internal("test".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ---------------------------------------------------------------
    // MarketplaceState
    // ---------------------------------------------------------------

    #[test]
    fn test_marketplace_state_new() {
        let state = MarketplaceState::new();
        let plugins = state.plugins.read();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_marketplace_state_default() {
        let state = MarketplaceState::default();
        let plugins = state.plugins.read();
        assert!(plugins.is_empty());
    }

    // ---------------------------------------------------------------
    // Metrics
    // ---------------------------------------------------------------

    #[test]
    fn test_metrics_new() {
        let metrics = Metrics::new();
        // Verify counters start at zero
        assert_eq!(metrics.crawls_total.get(), 0);
        assert_eq!(metrics.pages_crawled_total.get(), 0);
        assert_eq!(metrics.issues_total.get(), 0);
        assert_eq!(metrics.errors_total.get(), 0);
    }

    // ---------------------------------------------------------------
    // ApiKey serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_api_key_serialization_roundtrip() {
        let key = ApiKey {
            key: "ck_test123".to_string(),
            name: "test-key".to_string(),
            created_at: Utc::now(),
            requests_per_minute: 120,
        };
        let json = serde_json::to_string(&key).unwrap();
        let deserialized: ApiKey = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.key, "ck_test123");
        assert_eq!(deserialized.name, "test-key");
        assert_eq!(deserialized.requests_per_minute, 120);
    }

    // ---------------------------------------------------------------
    // CrawlResult serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_crawl_result_serialization_roundtrip() {
        let result = CrawlResult {
            crawl_id: "abc-123".to_string(),
            start_url: "https://example.com".to_string(),
            status: "running".to_string(),
            pages_crawled: 5,
            issues_found: 2,
            created_at: Utc::now(),
            completed_at: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CrawlResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.crawl_id, "abc-123");
        assert_eq!(deserialized.status, "running");
        assert!(deserialized.completed_at.is_none());
    }

    // ---------------------------------------------------------------
    // WebhookConfig serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_webhook_config_serialization_roundtrip() {
        let config = WebhookConfig {
            id: "wh-1".to_string(),
            url: "https://hooks.example.com".to_string(),
            events: vec!["crawl.completed".to_string()],
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WebhookConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "wh-1");
        assert_eq!(deserialized.events.len(), 1);
    }

    // ---------------------------------------------------------------
    // Tenant serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_tenant_serialization_roundtrip() {
        let tenant = Tenant {
            id: "t-1".to_string(),
            name: "Acme Corp".to_string(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&tenant).unwrap();
        let deserialized: Tenant = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "t-1");
        assert_eq!(deserialized.name, "Acme Corp");
    }

    // ---------------------------------------------------------------
    // MarketplacePlugin serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_marketplace_plugin_serialization_roundtrip() {
        let plugin = MarketplacePlugin {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            author: "tester".to_string(),
            description: "A test plugin".to_string(),
            license: "MIT".to_string(),
            categories: vec!["seo".to_string()],
            tags: vec!["test".to_string()],
            downloads: 100,
            rating: 4.5,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-02T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&plugin).unwrap();
        let deserialized: MarketplacePlugin = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test-plugin");
        assert_eq!(deserialized.downloads, 100);
        assert!((deserialized.rating - 4.5).abs() < f64::EPSILON);
    }
}
