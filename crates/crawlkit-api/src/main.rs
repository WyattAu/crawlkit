//! REST API server for crawlkit — exposes crawl, compare, and report operations over HTTP.
//!
//! Built on Axum with API-key authentication and per-key rate limiting.
//! Start a crawl via `POST /api/v1/crawls` and poll status at
//! `GET /api/v1/crawls/{crawl_id}`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Json, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{exponential_buckets, Histogram};
use prometheus_client::registry::Registry;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crawlkit_core::storage::Storage;
use crawlkit_core::AuditTrail;
use crawlkit_core::CrawlConfig;

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
    requests_total: Family<EndpointLabel, Counter>,
    request_duration_seconds: Histogram,
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
            requests_total,
            request_duration_seconds,
            active_crawls,
        }
    }
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

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
// Middleware: API key authentication + rate limiting
// ---------------------------------------------------------------------------

async fn auth_middleware(
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

async fn get_audit_events(State(state): State<AppState>) -> Json<Vec<crawlkit_core::AuditEvent>> {
    Json(state.audit_trail.events())
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

    let mut analyzer = crawlkit_core::BacklinkAnalyzer::new();
    analyzer.load_from_crawl_data(&link_pairs);
    for (source, target) in &external_links {
        analyzer.add_backlink(crawlkit_core::Backlink {
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
    use crawlkit_core::analyzers::AnalyzerRegistry;
    use crawlkit_core::http::HttpClient;
    use crawlkit_core::queue::{Priority, UrlQueue};
    use crawlkit_core::HtmlParser;

    let max_pages = config.max_pages;
    let http_client = match HttpClient::from_crawl_config(&config) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create HTTP client: {e}");
            if let Some(mut result) = state.crawl_results.get_mut(&crawl_id) {
                result.status = "failed".to_string();
                result.completed_at = Some(Utc::now());
            }
            fire_webhooks(&state, "crawl.failed", &crawl_id, 0, 0);
            return;
        }
    };

    let http_client = Arc::new(http_client);
    let robots_cache = Arc::new(crawlkit_core::RobotsTxtCache::new(
        http_client.clone(),
        &config,
    ));

    let queue = Arc::new(tokio::sync::Mutex::new(UrlQueue::from_crawl_config(
        &config,
    )));
    let analyzer_registry = AnalyzerRegistry::new(&config);

    // Seed the queue
    {
        let q = queue.lock().await;
        q.push(config.start_url.clone(), 0, Priority::HIGH);
    }

    let mut pages_crawled = 0usize;
    let mut total_issues = 0usize;
    let mut visited = std::collections::HashSet::new();
    let mut content_hashes = std::collections::HashSet::new();
    let crawl_start = std::time::Instant::now();

    while pages_crawled < max_pages {
        // Check time budget
        if let Some(max_time) = config.max_time {
            if crawl_start.elapsed() >= max_time {
                tracing::info!("Crawl time limit reached: {max_time:?}");
                break;
            }
        }

        let entry = {
            let q = queue.lock().await;
            q.pop()
        };

        let entry = match entry {
            Some(e) => e,
            None => break,
        };

        if visited.contains(&entry.url.to_string()) {
            continue;
        }
        visited.insert(entry.url.to_string());

        // Robots.txt check
        let robots_raw;
        if config.respect_robots_txt {
            let domain = entry.url.host_str().unwrap_or("");
            let scheme = entry.url.scheme();
            if robots_cache
                .is_disallowed(scheme, domain, entry.url.path())
                .await
            {
                tracing::debug!("Blocked by robots.txt: {}", entry.url);
                continue;
            }
            robots_raw = robots_cache.raw_content(scheme, domain).await;
        } else {
            robots_raw = String::new();
        }

        // Respect delay between requests
        tokio::time::sleep(config.request_delay).await;

        let start = std::time::Instant::now();
        let result = match http_client.fetch(&entry.url).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to fetch {}: {e}", entry.url);
                continue;
            }
        };
        let fetch_time = start.elapsed();

        // Content-hash deduplication
        {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(result.body.as_bytes());
            let result = hasher.finalize();
            let hash: String = result.iter().map(|b| format!("{b:02x}")).collect();
            if !content_hashes.insert(hash) {
                tracing::debug!("Skipping duplicate content: {}", entry.url);
                continue;
            }
        }

        pages_crawled += 1;
        state.metrics.pages_crawled_total.inc();

        let parsed = match HtmlParser::parse(&result.body, &entry.url) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to parse {}: {e}", entry.url);
                continue;
            }
        };

        let headers_vec: Vec<(String, String)> = result.headers.clone();
        let empty_chain: Vec<crawlkit_core::RedirectHop> = vec![];
        let robots_ref = if robots_raw.is_empty() {
            None
        } else {
            Some(robots_raw.as_str())
        };
        let ctx = crawlkit_core::analyzers::AnalysisContext {
            page: &parsed,
            status_code: Some(result.status_code),
            headers: &headers_vec,
            response_time: Some(fetch_time),
            redirect_chain: &empty_chain,
            robots_txt: robots_ref,
        };
        let findings = analyzer_registry.analyze(&ctx, &config);
        total_issues += findings.len();
        state.metrics.issues_total.inc_by(findings.len() as u64);

        let page_data = crawlkit_core::storage::PageData {
            id: Uuid::new_v4().to_string(),
            url: entry.url.clone(),
            final_url: result.final_url.clone(),
            status_code: result.status_code,
            title: parsed.meta.title.clone(),
            description: parsed.meta.description.clone(),
            canonical_url: parsed.meta.canonical.clone(),
            word_count: Some(parsed.word_count),
            load_time_ms: Some({
                #[allow(clippy::cast_possible_truncation)]
                let v = fetch_time.as_millis() as u64;
                v
            }),
            body_size: Some(result.body.len()),
            fetched_at: Utc::now(),
            links: parsed
                .links
                .iter()
                .filter_map(|l| url::Url::parse(&l.href).ok())
                .collect(),
        };

        let _ = state.storage.insert_page(&crawl_id, &page_data);

        for finding in &findings {
            let issue = crawlkit_core::storage::Issue {
                id: Uuid::new_v4().to_string(),
                page_id: page_data.id.clone(),
                category: finding.category.clone(),
                severity: finding.severity.clone(),
                code: finding.code.clone(),
                title: finding.title.clone(),
                description: finding.description.clone(),
                element: None,
                recommendation: finding.recommendation.clone(),
            };
            let _ = state.storage.insert_issue(&issue);
        }

        // Queue new links
        for link in &parsed.links {
            let link_url = match url::Url::parse(&link.href) {
                Ok(u) => u,
                Err(_) => continue,
            };
            if visited.contains(&link_url.to_string()) {
                continue;
            }
            let is_internal = link_url.host_str() == entry.url.host_str();
            let priority = if is_internal {
                Priority::NORMAL
            } else {
                Priority::LOW
            };
            let q = queue.lock().await;
            q.push(link_url, entry.depth + 1, priority);
        }
    }

    let _ = state
        .storage
        .finish_crawl(&crawl_id, pages_crawled, total_issues);

    if let Some(mut result) = state.crawl_results.get_mut(&crawl_id) {
        result.status = "completed".to_string();
        result.pages_crawled = pages_crawled;
        result.issues_found = total_issues;
        result.completed_at = Some(Utc::now());
    }

    state.metrics.active_crawls.dec();
    fire_webhooks(
        &state,
        "crawl.completed",
        &crawl_id,
        pages_crawled,
        total_issues,
    );
    tracing::info!("Crawl {crawl_id} completed: {pages_crawled} pages, {total_issues} issues");
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
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_endpoint))
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
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_metrics_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
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
