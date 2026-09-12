//! Hosted free-scan scanner (ADR-012) — HTTP API.
//!
//! Three endpoints, no auth, no accounts:
//!
//! - `POST /scan` — validate + budget-check, run the bounded scan, return
//!   `{token}` (202) with the result URL.
//! - `GET /scan/{token}` — result when ready (200); `404` for unknown or
//!   expired tokens; rejected/blocked states surfaced explicitly.
//! - `GET /healthz` — liveness.
//!
//! Results are retained for [`RESULT_RETENTION`] and served only to holders
//! of the unguessable token. The API never echoes submission source IPs into
//! results, and responses carry `Cache-Control: no-store` so results are not
//! cached by intermediaries or browsers.
//!
//! The prototype runs scans inline per request (bounded to 30 s wall clock)
//! with a bounded in-memory result map. Production deployment must move the
//! result store behind shared state and the scan execution behind the queue
//! infrastructure (ADR-012 §2) before multi-replica rollout.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::bounds::{BudgetStore, InMemoryBudgetStore, RESULT_RETENTION};
use crate::fetcher::PinnedFetcher;
use crate::scan::{run_scan, ScanOutcome};
use crate::token::{new_result_token, parse_result_token, TokenError};

/// Result-store ceiling: the oldest entries are evicted when exceeded. This
/// bounds prototype memory; retention (`RESULT_RETENTION`) still applies.
const MAX_STORED_RESULTS: usize = 512;

/// Max concurrently running scans (worker bound; queued requests wait).
const MAX_CONCURRENT_SCANS: usize = 4;

/// Submitted scan request.
#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    /// The single URL to scan (absolute http(s) URL).
    pub url: String,
}

/// Scan submission response.
#[derive(Debug, Serialize)]
pub struct ScanAccepted {
    pub token: String,
    /// Relative URL for retrieving the result.
    pub result_url: String,
}

/// Result payload returned for a finished scan.
#[derive(Debug, Serialize)]
pub struct ScanResultResponse {
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<crate::scan::ScanReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Seconds remaining before this result is deleted.
    pub expires_in_secs: u64,
}

/// Stored outcome with its expiry.
struct StoredResult {
    outcome: ScanOutcome,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// Shared server state.
pub struct AppState {
    fetcher: Arc<PinnedFetcher>,
    budget: Arc<dyn BudgetStore>,
    registry: Arc<crawlkit_engine::analyzers::AnalyzerRegistry>,
    results: dashmap::DashMap<String, StoredResult>,
    /// One-Gil-free bound on concurrent scans.
    scan_permits: Semaphore,

    /// Which budget backend was selected at startup (ADR-015 §A1 posture).
    /// Surfaced for runbook verification and tests.
    pub budget_backend: BudgetBackend,

    /// Shared Redis URL when running the multi-replica posture (queue
    /// execution + shared results); `None` in the single-replica posture.
    pub redis_url: Option<String>,
}

/// Errors possible when constructing [`AppState`] from the environment.
#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    /// Fetcher/trust-boundary construction failed.
    #[error(transparent)]
    Guard(#[from] crate::guard::GuardError),
    /// Budget-backend configuration is invalid; message names the variable
    /// and the accepted values so misconfiguration fails fast at boot.
    #[error("budget backend configuration error: {0}")]
    Config(String),
}

/// The selected politeness-budget deployment posture (ADR-015 §A1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetBackend {
    /// In-process store: single-replica default. Budget resets on process
    /// restart; no cross-replica accounting (acceptable — one replica cannot
    /// multiply its own budget).
    InMemory,
    /// Redis-backed shared-state store: multi-replica posture. Fail-closed
    /// on Redis outage by design.
    Redis,
}

impl AppState {
    /// Build state from the production fetcher and registry.
    pub fn new() -> Result<Self, crate::guard::GuardError> {
        Ok(Self {
            fetcher: Arc::new(PinnedFetcher::new()?),
            budget: Arc::new(InMemoryBudgetStore::new()),
            registry: Arc::new(crawlkit_engine::analyzers::AnalyzerRegistry::new(
                &crawlkit_engine::CrawlConfig::default(),
            )),
            results: dashmap::DashMap::new(),
            scan_permits: Semaphore::new(MAX_CONCURRENT_SCANS),
            budget_backend: BudgetBackend::InMemory,
            redis_url: None,
        })
    }

    /// Whether scan execution is queued (multi-replica posture).
    ///
    /// True exactly when the Redis budget backend is selected: the runbook's
    /// multi-replica posture is "shared budget + queue fan-out", so one
    /// switch selects both. Inline execution remains the single-replica
    /// path, where Redis is not required at all.
    pub fn queue_mode(&self) -> bool {
        self.budget_backend == BudgetBackend::Redis
    }

    /// Build scan dependencies from this state (shared fetcher, budget,
    /// registry). Used by the inline handler and, in queue mode, by the
    /// worker pool's deps factory.
    pub fn deps(&self) -> crate::scan::ScanDeps {
        crate::scan::ScanDeps {
            fetcher: self.fetcher.clone(),
            budget: self.budget.clone(),
            registry: self.registry.clone(),
        }
    }

    /// Build state with the budget backend selected from the environment
    /// (ADR-015 §A1 deployment-posture split):
    ///
    /// - `CRAWLKIT_BUDGET_BACKEND` unset or `in-memory` → single-replica
    ///   posture ([`InMemoryBudgetStore`]; documented weaker guarantees).
    /// - `CRAWLKIT_BUDGET_BACKEND=redis` → multi-replica posture
    ///   ([`RedisBudgetStore`] against `CRAWLKIT_REDIS_URL`; fail-closed).
    ///   Requires the `shared-budget` feature at build time.
    ///
    /// Any other value is a hard startup error naming the accepted values,
    /// so a typo cannot silently degrade the anti-abuse guarantee.
    ///
    /// # Errors
    ///
    /// Returns [`StartupError`] on fetcher construction failure or invalid
    /// backend configuration.
    pub fn from_env() -> Result<Self, StartupError> {
        let backend =
            std::env::var("CRAWLKIT_BUDGET_BACKEND").unwrap_or_else(|_| "in-memory".to_string());
        let redis_url = std::env::var("CRAWLKIT_REDIS_URL").ok();
        Self::with_budget_backend(&backend, redis_url.as_deref())
    }

    /// Build state with an explicit budget backend, the testable core of
    /// [`Self::from_env`].
    ///
    /// # Errors
    ///
    /// Returns [`StartupError`] on fetcher construction failure or invalid
    /// backend configuration.
    pub fn with_budget_backend(
        backend: &str,
        redis_url: Option<&str>,
    ) -> Result<Self, StartupError> {
        match backend {
            "in-memory" | "" => {
                let mut state = Self::new()?;
                state.budget_backend = BudgetBackend::InMemory;
                Ok(state)
            }
            "redis" => {
                let redis_url = redis_url.ok_or_else(|| {
                    StartupError::Config(
                        "CRAWLKIT_BUDGET_BACKEND=redis requires CRAWLKIT_REDIS_URL".to_string(),
                    )
                })?;
                #[cfg(feature = "shared-budget")]
                {
                    let store = crate::budget_redis::RedisBudgetStore::new(redis_url)
                        .map_err(|e| StartupError::Config(format!("Redis budget store: {e}")))?;
                    Ok(Self {
                        fetcher: Arc::new(PinnedFetcher::new()?),
                        budget: Arc::new(store),
                        registry: Arc::new(crawlkit_engine::analyzers::AnalyzerRegistry::new(
                            &crawlkit_engine::CrawlConfig::default(),
                        )),
                        results: dashmap::DashMap::new(),
                        scan_permits: Semaphore::new(MAX_CONCURRENT_SCANS),
                        budget_backend: BudgetBackend::Redis,
                        redis_url: Some(redis_url.to_string()),
                    })
                }
                #[cfg(not(feature = "shared-budget"))]
                {
                    let _ = redis_url;
                    Err(StartupError::Config(
                        "CRAWLKIT_BUDGET_BACKEND=redis requires building with the \
                         `shared-budget` feature"
                            .to_string(),
                    ))
                }
            }
            other => Err(StartupError::Config(format!(
                "CRAWLKIT_BUDGET_BACKEND={other:?} is not a valid backend; \
                 accepted values: in-memory, redis"
            ))),
        }
    }

    /// Remove expired results; called opportunistically on submissions.
    fn sweep(&self) {
        let now = chrono::Utc::now();
        self.results.retain(|_, v| v.expires_at > now);
        // Hard ceiling on stored results (prototype memory bound): if the
        // store is over capacity after expiry sweeping, evict oldest first.
        while self.results.len() > MAX_STORED_RESULTS {
            let Some(oldest_key) = self
                .results
                .iter()
                .min_by_key(|entry| entry.value().expires_at)
                .map(|e| e.key().clone())
            else {
                break;
            };
            self.results.remove(&oldest_key);
        }
    }
}

/// Build the scanner router.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/scan", post(submit_scan))
        .route("/scan/{token}", get(get_result))
        .route("/healthz", get(healthz))
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

/// POST /scan: validate, run the bounded scan, store and return the token.
///
/// Inline execution keeps the prototype honest about its bounds: a request
/// is accepted only when a worker slot is free, and every trust decision in
/// the scan path is identical to the production engine's.
async fn submit_scan(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScanRequest>,
) -> Response {
    state.sweep();

    let token = new_result_token();

    // Acquire a worker slot; shed load with 503 + Retry-After when full.
    let _permit = match state.scan_permits.try_acquire() {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                [(header::RETRY_AFTER, "30")],
                Json(serde_json::json!({ "error": "scanner busy; retry later" })),
            )
                .into_response();
        }
    };

    let deps = state.deps();

    // Multi-replica posture: enqueue for the worker pool and return the
    // token immediately. The scan runs with the identical trust path in a
    // worker; GET /scan/{token} serves the shared outcome key. Enqueue
    // failure is fail-closed: 503, nothing acknowledged.
    #[cfg(feature = "shared-budget")]
    if state.queue_mode() {
        let Some(redis_url) = state.redis_url.as_deref() else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                [(header::RETRY_AFTER, "30")],
                Json(serde_json::json!({ "error": "queue backend unavailable" })),
            )
                .into_response();
        };
        return match crate::worker::enqueue_scan(redis_url, &token, &request.url).await {
            Ok(true) => (
                StatusCode::ACCEPTED,
                [
                    (header::CACHE_CONTROL, "no-store"),
                    (header::RETRY_AFTER, "2"),
                ],
                Json(serde_json::json!({
                    "token": token,
                    "result_url": format!("/scan/{token}"),
                    "state": "queued"
                })),
            )
                .into_response(),
            Ok(false) => (
                StatusCode::CONFLICT,
                [(header::CACHE_CONTROL, "no-store")],
                Json(serde_json::json!({ "error": "job already enqueued" })),
            )
                .into_response(),
            Err(e) => {
                tracing::warn!(error = %e, "scan enqueue failed; failing closed");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    [(header::RETRY_AFTER, "30")],
                    Json(serde_json::json!({ "error": "queue backend unavailable" })),
                )
                    .into_response()
            }
        };
    }

    let outcome = run_scan(&request.url, deps).await;

    let (status, body) = match &outcome {
        ScanOutcome::Rejected { .. } => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({ "state": "rejected", "token": token }),
        ),
        ScanOutcome::RobotsBlocked { .. } => (
            StatusCode::OK,
            serde_json::json!({ "state": "robots_blocked", "token": token }),
        ),
        ScanOutcome::BudgetExhausted { retry_after_secs } => (
            StatusCode::TOO_MANY_REQUESTS,
            serde_json::json!({ "state": "budget_exhausted", "retry_after_secs": retry_after_secs }),
        ),
        ScanOutcome::Complete(_) => (
            StatusCode::ACCEPTED,
            serde_json::json!({ "token": token, "result_url": format!("/scan/{token}") }),
        ),
    };

    // Only addressable outcomes are stored; rejected inputs and budget
    // refusals are not retained.
    if matches!(
        outcome,
        ScanOutcome::Complete(_) | ScanOutcome::RobotsBlocked { .. }
    ) {
        state.results.insert(
            token.clone(),
            StoredResult {
                outcome,
                expires_at: chrono::Utc::now()
                    + chrono::Duration::from_std(RESULT_RETENTION)
                        .unwrap_or_else(|_| chrono::Duration::hours(24)),
            },
        );
    }

    (status, [(header::CACHE_CONTROL, "no-store")], Json(body)).into_response()
}

/// GET /scan/{token}: return the stored result if present and unexpired.
async fn get_result(State(state): State<Arc<AppState>>, Path(token): Path<String>) -> Response {
    // Shape-validate before using as a map key; malformed tokens 404 without
    // touching the store (and without leaking whether siblings exist).
    if parse_result_token(&token).is_err() {
        return token_error_response();
    }

    // Multi-replica posture: serve from the shared done key so any replica
    // can answer for any token (results are not replica-local in this mode).
    #[cfg(feature = "shared-budget")]
    if let Some(redis_url) = state.redis_url.as_deref() {
        if parse_result_token(&token).is_err() {
            return token_error_response();
        }
        let shared = match crate::worker::read_outcome(redis_url, &token).await {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!(error = %e, "shared result read failed");
                None
            }
        };
        return match shared {
            Some(outcome) => {
                let body = match &outcome {
                    ScanOutcome::Complete(report) => ScanResultResponse {
                        state: "complete",
                        report: Some((**report).clone()),
                        reason: None,
                        expires_in_secs: RESULT_RETENTION.as_secs(),
                    },
                    ScanOutcome::RobotsBlocked { submitted_url } => ScanResultResponse {
                        state: "robots_blocked",
                        report: None,
                        reason: Some(format!("robots.txt disallows scanning {submitted_url}")),
                        expires_in_secs: RESULT_RETENTION.as_secs(),
                    },
                    ScanOutcome::Rejected { .. } | ScanOutcome::BudgetExhausted { .. } => {
                        ScanResultResponse {
                            state: "expired",
                            report: None,
                            reason: None,
                            expires_in_secs: 0,
                        }
                    }
                };
                (
                    StatusCode::OK,
                    [(header::CACHE_CONTROL, "no-store")],
                    Json(body),
                )
                    .into_response()
            }
            None => token_error_response(),
        };
    }

    match state.results.get(&token) {
        Some(entry) => {
            let expires_in_secs =
                u64::try_from((entry.expires_at - chrono::Utc::now()).num_seconds().max(0))
                    .unwrap_or(0);
            let body = match &entry.outcome {
                ScanOutcome::Complete(report) => ScanResultResponse {
                    state: "complete",
                    report: Some((**report).clone()),
                    reason: None,
                    expires_in_secs,
                },
                ScanOutcome::RobotsBlocked { submitted_url } => ScanResultResponse {
                    state: "robots_blocked",
                    report: None,
                    reason: Some(format!("robots.txt disallows scanning {submitted_url}")),
                    expires_in_secs,
                },
                ScanOutcome::Rejected { .. } | ScanOutcome::BudgetExhausted { .. } => {
                    // Not stored; defensive.
                    ScanResultResponse {
                        state: "expired",
                        report: None,
                        reason: None,
                        expires_in_secs: 0,
                    }
                }
            };
            (
                StatusCode::OK,
                [(header::CACHE_CONTROL, "no-store")],
                Json(body),
            )
                .into_response()
        }
        None => token_error_response(),
    }
}

fn token_error_response() -> Response {
    (
        StatusCode::NOT_FOUND,
        [(header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({ "error": "unknown or expired result token" })),
    )
        .into_response()
}

impl From<TokenError> for Response {
    fn from(_: TokenError) -> Self {
        token_error_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    fn test_app() -> Router {
        // Production state but the registry/fetcher are exercised only for
        // liveness here; scan-path tests live in scan.rs with fixtures.
        let state = Arc::new(AppState::new().expect("state"));
        create_router(state)
    }

    #[tokio::test]
    async fn healthz_responds_ok() {
        let app = test_app();
        let res = app
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    // --- Budget-posture selection (ADR-015 §A1) ---

    /// Extract the Config error message, failing the test on any other
    /// outcome (AppState is not Debug, so `expect_err` is unavailable).
    fn expect_config_err(r: Result<AppState, StartupError>) -> String {
        match r {
            Ok(_) => panic!("expected Config error, got Ok state"),
            Err(StartupError::Config(msg)) => msg,
            Err(other) => panic!("expected Config error, got {other:?}"),
        }
    }

    #[test]
    fn backend_default_is_in_memory() {
        let state = AppState::with_budget_backend("in-memory", None).expect("state");
        assert_eq!(state.budget_backend, BudgetBackend::InMemory);
    }

    #[test]
    fn backend_empty_string_is_in_memory() {
        let state = AppState::with_budget_backend("", None).expect("state");
        assert_eq!(state.budget_backend, BudgetBackend::InMemory);
    }

    #[test]
    fn backend_unknown_value_is_rejected_not_silently_degraded() {
        // A typo like "Redis" or "inmemory" must fail startup loudly —
        // silent fallback would multiply the budget across replicas.
        for bad in ["Redis", "inmemory", "memory", "redis-"] {
            let msg = expect_config_err(AppState::with_budget_backend(bad, None));
            assert!(
                msg.contains("accepted values"),
                "error must name accepted values: {msg}"
            );
        }
    }

    #[cfg(feature = "shared-budget")]
    #[test]
    fn backend_redis_without_url_is_rejected() {
        let msg = expect_config_err(AppState::with_budget_backend("redis", None));
        assert!(
            msg.contains("CRAWLKIT_REDIS_URL"),
            "error names the variable: {msg}"
        );
    }

    #[cfg(feature = "shared-budget")]
    #[test]
    fn backend_redis_selects_shared_store() {
        // Store construction succeeds even against a non-listening address:
        // the ConnectionManager connects lazily on first use.
        let state =
            AppState::with_budget_backend("redis", Some("redis://127.0.0.1:1/")).expect("state");
        assert_eq!(state.budget_backend, BudgetBackend::Redis);
    }

    #[cfg(not(feature = "shared-budget"))]
    #[test]
    fn backend_redis_requires_shared_budget_feature() {
        let msg = expect_config_err(AppState::with_budget_backend(
            "redis",
            Some("redis://127.0.0.1:1/"),
        ));
        assert!(
            msg.contains("shared-budget"),
            "error names the feature: {msg}"
        );
    }

    #[tokio::test]
    async fn submit_rejects_invalid_json_and_url_shape() {
        let app = test_app();
        // Malformed body → 422/400 from axum's Json extractor.
        let res = app
            .clone()
            .oneshot(
                Request::post("/scan")
                    .header("content-type", "application/json")
                    .body(Body::from("not json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            res.status().is_client_error(),
            "malformed body must be a client error"
        );

        // A structurally valid JSON body with a disallowed target is stored
        // nothing and returns 400 with state rejected — but only after the
        // scan path validates; fetcher never runs (no network in tests:
        // scan for a literal-private IP fails validation before any fetch).
        let app2 = test_app();
        let res = app2
            .oneshot(
                Request::post("/scan")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"url":"http://169.254.169.254/"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn unknown_token_is_404_without_store_touch() {
        let app = test_app();
        let res = app
            .oneshot(
                Request::get("/scan/AAAAAAAAAAAAAAAAAAAAAA")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn malformed_token_is_404() {
        let app = test_app();
        let res = app
            .oneshot(
                Request::get("/scan/../../etc/passwd")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
