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
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crawlkit_core::storage::Storage;
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
            tokens: rpm as f64,
            max_tokens: rpm as f64,
            refill_rate: rpm as f64 / 60.0,
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
// Application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    storage: Arc<Storage>,
    api_keys: Arc<DashMap<String, ApiKey>>,
    rate_limits: Arc<DashMap<String, RateLimitBucket>>,
    crawl_results: Arc<DashMap<String, CrawlResult>>,
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

    Ok(next.run(request).await)
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

async fn start_crawl(
    State(state): State<AppState>,
    Json(req): Json<CreateCrawlRequest>,
) -> Result<(StatusCode, Json<CrawlResponse>), ApiError> {
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

    // Spawn crawl task in background
    let state_clone = state.clone();
    let crawl_id_clone = crawl_id.clone();
    let config = CrawlConfig {
        start_url,
        max_pages: req.max_pages,
        request_delay: std::time::Duration::from_millis(req.request_delay_ms),
        concurrency: req.concurrency,
        ..Default::default()
    };

    tokio::spawn(async move {
        run_crawl_task(state_clone, crawl_id_clone, config).await;
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
            return;
        }
    };

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

    while pages_crawled < max_pages {
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
        pages_crawled += 1;

        let parsed = match HtmlParser::parse(&result.body, &entry.url) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to parse {}: {e}", entry.url);
                continue;
            }
        };

        let headers_vec: Vec<(String, String)> = result.headers.clone();
        let empty_chain: Vec<crawlkit_core::RedirectHop> = vec![];
        let ctx = crawlkit_core::analyzers::AnalysisContext {
            page: &parsed,
            status_code: Some(result.status_code),
            headers: &headers_vec,
            response_time: Some(fetch_time),
            redirect_chain: &empty_chain,
        };
        let findings = analyzer_registry.analyze(&ctx, &config);
        total_issues += findings.len();

        let page_data = crawlkit_core::storage::PageData {
            id: Uuid::new_v4().to_string(),
            url: entry.url.clone(),
            final_url: result.final_url.clone(),
            status_code: result.status_code,
            title: parsed.meta.title.clone(),
            description: parsed.meta.description.clone(),
            canonical_url: parsed.meta.canonical.clone(),
            word_count: Some(parsed.word_count),
            load_time_ms: Some(fetch_time.as_millis() as u64),
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

    tracing::info!("Crawl {crawl_id} completed: {pages_crawled} pages, {total_issues} issues");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("crawlkit_api=info")),
        )
        .init();

    let db_path =
        std::env::var("CRAWLKIT_DB_PATH").unwrap_or_else(|_| "crawlkit-api.db".to_string());

    let storage = Storage::new(std::path::Path::new(&db_path))
        .map_err(|e| anyhow::anyhow!("Failed to open storage: {e}"))?;

    // Seed a default API key for development
    let default_key = "ck_dev_default_key_for_testing".to_string();
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
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/crawls", post(start_crawl).get(list_crawls))
        .route("/api/v1/crawls/{crawl_id}", get(get_crawl_status))
        .route("/api/v1/crawls/{crawl_id}/stats", get(get_crawl_stats))
        .route("/api/v1/keys", post(create_api_key).get(list_api_keys))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    tracing::info!("crawlkit API listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
