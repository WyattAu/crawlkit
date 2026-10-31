//! `GET /api/v1/reports/{run_id}` (§G6) — the share-link primitive.
//!
//! Serves the stored HTML report for a finished run. Reports are rendered
//! deterministically from the run's findings (one findings-count section
//! per page/category) and persisted next to the API database
//! (`<db dir>/reports/{run_id}.html`) when a crawl completes, so the same
//! run always serves the same bytes (ITER-09 determinism rule).
//!
//! The route sits behind the standard auth middleware (JWT or API key) —
//! the "share link" is the authenticated URL itself. The weighted scoring
//! engine (§G2) does not exist yet, so the report shows finding counts
//! rather than scores.

use std::path::PathBuf;

use axum::extract::{Path, State};
use axum::response::Html;

use crate::types::{ApiError, ApiErrorBody, AppState};

/// Validate a run id before it touches the filesystem (no path
/// separators, no traversal). UUID-style ids (hex + hyphens) pass.
#[must_use]
pub fn valid_run_id(run_id: &str) -> bool {
    !run_id.is_empty()
        && run_id.len() <= 128
        && run_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Directory holding stored reports, derived from the API database path.
#[must_use]
pub fn report_dir(db_path: &std::path::Path) -> PathBuf {
    db_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."))
        .join("reports")
}

/// Persist a report for `run_id`; returns the written path.
///
/// # Errors
///
/// Returns the I/O error when the directory cannot be created or the file
/// cannot be written.
pub fn store_report(
    db_path: &std::path::Path,
    run_id: &str,
    html: &str,
) -> std::io::Result<PathBuf> {
    let dir = report_dir(db_path);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{run_id}.html"));
    std::fs::write(&path, html)?;
    Ok(path)
}

/// Load the stored report for `run_id`, if one exists.
#[must_use]
pub fn load_report(db_path: &std::path::Path, run_id: &str) -> Option<String> {
    if !valid_run_id(run_id) {
        return None;
    }
    std::fs::read_to_string(report_dir(db_path).join(format!("{run_id}.html"))).ok()
}

/// Minimal deterministic HTML report from `(page, category, count)` rows.
///
/// Rows are rendered in the given order; callers sort them
/// `(page, category)` so identical data produces byte-identical HTML.
#[must_use]
pub fn render_report_html(
    run_id: &str,
    generated_at: &str,
    counts: &[(String, String, i64)],
) -> String {
    let mut out = String::with_capacity(4096 + counts.len() * 96);
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    out.push_str("<title>crawlkit report ");
    out.push_str(&html_escape(run_id));
    out.push_str("</title>\n<style>body{font-family:sans-serif;margin:2rem}table{border-collapse:collapse}th,td{border:1px solid #ccc;padding:.35rem .6rem;text-align:left}th{background:#f4f4f4}</style>\n");
    out.push_str("</head>\n<body>\n<h1>crawlkit report</h1>\n");
    out.push_str("<p>run: <code>");
    out.push_str(&html_escape(run_id));
    out.push_str("</code><br>generated: ");
    out.push_str(&html_escape(generated_at));
    out.push_str(
        "<br>metric: finding counts (score history lands with the §G2 scoring engine)</p>\n",
    );

    if counts.is_empty() {
        out.push_str("<p>No findings recorded for this run.</p>\n");
    } else {
        out.push_str("<table>\n<thead><tr><th>Page</th><th>Category</th><th>Findings</th></tr></thead>\n<tbody>\n");
        for (page, category, count) in counts {
            out.push_str("<tr><td>");
            out.push_str(&html_escape(page));
            out.push_str("</td><td>");
            out.push_str(&html_escape(category));
            out.push_str("</td><td>");
            out.push_str(&count.to_string());
            out.push_str("</td></tr>\n");
        }
        out.push_str("</tbody>\n</table>\n");
    }
    out.push_str("</body>\n</html>\n");
    out
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `GET /api/v1/reports/{run_id}` — serve the stored HTML report.
///
/// Auth is enforced by the standard middleware stack (the route is
/// registered inside the protected router).
#[utoipa::path(
    get,
    path = "/api/v1/reports/{run_id}",
    tag = "reports",
    params(
        ("run_id" = String, Path, description = "Storage crawl (run) identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Stored HTML report", content_type = "text/html"),
        (status = 400, description = "Malformed run id", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 404, description = "No stored report for this run", body = ApiErrorBody)
    )
)]
pub async fn get_report(
    State(_state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Html<String>, ApiError> {
    if !valid_run_id(&run_id) {
        return Err(ApiError::BadRequest(format!("invalid run id '{run_id}'")));
    }

    let run_id_for_lookup = run_id.clone();
    let db_path = crate::handlers::reports::db_path_from_env();
    let report = tokio::task::spawn_blocking(move || load_report(&db_path, &run_id_for_lookup))
        .await
        .map_err(|e| ApiError::Internal(format!("Blocking task failed: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("no stored report for run '{run_id}'")))?;

    Ok(Html(report))
}

/// The API database path, mirroring `main.rs` (`CRAWLKIT_DB_PATH`,
/// default `crawlkit-api.db`). Reports are stored next to it.
#[must_use]
pub fn db_path_from_env() -> PathBuf {
    std::env::var("CRAWLKIT_DB_PATH")
        .unwrap_or_else(|_| "crawlkit-api.db".to_string())
        .into()
}

/// Render + persist the report for a finished run and record the §G6
/// history snapshot. Best-effort: failures are logged, never propagated —
/// a report glitch must not fail a completed crawl.
pub fn snapshot_and_store_report(
    storage: &dyn crawlkit_engine::storage_trait::StorageBackend,
    run_id: &str,
    ts: &str,
    db_path: &std::path::Path,
) {
    // §G6 history snapshot (finding counts until §G2 scores exist).
    if let Err(e) = storage.record_history_snapshot(run_id, ts) {
        tracing::warn!(run_id, error = %e, "history snapshot failed");
    }

    // Stored HTML report for GET /api/v1/reports/{run_id}.
    let counts = match storage.get_issues(run_id, &crawlkit_engine::storage::IssueFilter::default())
    {
        Ok(issues) => {
            // Resolve page ids to URLs for the report (fall back to the
            // raw page id when the page row is gone).
            let page_urls: std::collections::HashMap<String, String> = storage
                .get_pages(run_id, 100_000)
                .unwrap_or_default()
                .into_iter()
                .map(|p| (p.id.clone(), p.url.to_string()))
                .collect();
            let mut counts: std::collections::BTreeMap<(String, String), i64> =
                std::collections::BTreeMap::new();
            for issue in &issues {
                let page = page_urls
                    .get(&issue.page_id)
                    .cloned()
                    .unwrap_or_else(|| issue.page_id.clone());
                *counts
                    .entry((page, issue.category.as_str().to_string()))
                    .or_insert(0) += 1;
            }
            counts
                .into_iter()
                .map(|((page, category), count)| (page, category, count))
                .collect::<Vec<_>>()
        }
        Err(e) => {
            tracing::warn!(run_id, error = %e, "could not read findings for report");
            Vec::new()
        }
    };

    let html = render_report_html(run_id, ts, &counts);
    if let Err(e) = store_report(db_path, run_id, &html) {
        tracing::warn!(run_id, error = %e, "could not store report");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthManager;
    use crate::types::{AppState, MarketplaceState, Metrics};
    use axum::extract::Path;
    use dashmap::DashMap;
    use std::sync::Arc;

    fn make_state(dir: &std::path::Path) -> AppState {
        let storage = crawlkit_engine::storage::Storage::new(&dir.join("test.db")).unwrap();
        AppState {
            storage: Arc::new(storage),
            api_keys: Arc::new(DashMap::new()),
            rate_limits: Arc::new(DashMap::new()),
            crawl_results: Arc::new(DashMap::new()),
            audit_trail: Arc::new(crawlkit_engine::AuditTrail::new()),
            metrics: Arc::new(Metrics::new()),
            webhooks: Arc::new(DashMap::new()),
            schedules: Arc::new(DashMap::new()),
            http_client: reqwest::Client::new(),
            auth: Arc::new(AuthManager::new("test-secret".to_string())),
            access_logger: Arc::new(crawlkit_engine::access_log::AccessLogger::new(100)),
            oidc: None,
            oidc_states: Arc::new(DashMap::new()),
            tenants: Arc::new(DashMap::new()),
            marketplace: MarketplaceState::new(),
            sessions: Arc::new(DashMap::new()),
            login_attempts: Arc::new(DashMap::<String, LoginAttemptRecord>::new()),
            persistence: None,
            crawl_permits: Arc::new(tokio::sync::Semaphore::new(1)),
            idempotency_keys: Arc::new(DashMap::new()),
            credential_store: None,
            alert_channels: Arc::new(DashMap::new()),
        }
    }

    use crate::types::LoginAttemptRecord;

    /// Serializes tests that set `CRAWLKIT_DB_PATH` (process-global env).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn render_is_deterministic_and_escapes() {
        let counts = vec![
            ("https://x/<b>".to_string(), "seo".to_string(), 2),
            ("https://x/a".to_string(), "a11y".to_string(), 1),
        ];
        let a = render_report_html("run-1", "2026-10-31T00:00:00Z", &counts);
        let b = render_report_html("run-1", "2026-10-31T00:00:00Z", &counts);
        assert_eq!(a, b, "identical data → byte-identical HTML");
        assert!(a.contains("&lt;b&gt;"), "HTML is escaped: {a}");
        assert!(a.contains("<td>2</td>"));
        assert!(!a.contains("<b>"), "raw markup must not leak: {a}");

        let empty = render_report_html("run-2", "t", &[]);
        assert!(empty.contains("No findings recorded"));
    }

    #[test]
    fn run_id_validation_blocks_traversal() {
        assert!(valid_run_id("abc-123_X"));
        assert!(!valid_run_id(""));
        assert!(!valid_run_id("../etc/passwd"));
        assert!(!valid_run_id("a/b"));
        assert!(!valid_run_id(".hidden"));
        assert!(!valid_run_id("a b"));
    }

    // ENV_LOCK intentionally serializes CRAWLKIT_DB_PATH across the awaits below:
    // the async handlers under test mutate/read env-derived state, and parallel
    // tests must not race the process-global env. Test-only; scoped to this file.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn store_load_serve_roundtrip() {
        let _env = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("api.db");
        std::env::set_var("CRAWLKIT_DB_PATH", &db_path);

        let html = render_report_html("run-9", "2026-10-31T00:00:00Z", &[]);
        let stored = store_report(&db_path, "run-9", &html).unwrap();
        assert!(stored.ends_with("reports/run-9.html"));

        let state = make_state(dir.path());
        let response = get_report(State(state), Path("run-9".to_string()))
            .await
            .unwrap();
        assert_eq!(response.0, html);

        // Missing report → 404.
        let state = make_state(dir.path());
        let err = get_report(State(state), Path("run-missing".to_string()))
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));

        // Malformed run id → 400.
        let state = make_state(dir.path());
        let err = get_report(State(state), Path("../evil".to_string()))
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));

        std::env::remove_var("CRAWLKIT_DB_PATH");
    }

    /// Full path: findings in storage → snapshot + stored report → served.
    // See store_load_serve_roundtrip: intentional test-only env serialization.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn snapshot_and_store_produces_served_report() {
        let _env = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("api.db");
        std::env::set_var("CRAWLKIT_DB_PATH", &db_path);

        let storage = crawlkit_engine::storage::Storage::new(&db_path).unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        let page = crawlkit_engine::storage::PageData {
            id: "p1".to_string(),
            url: url::Url::parse("https://example.com/").unwrap(),
            final_url: url::Url::parse("https://example.com/").unwrap(),
            status_code: 200,
            title: Some("Home".to_string()),
            description: None,
            canonical_url: None,
            word_count: Some(500),
            load_time_ms: Some(10),
            body_size: Some(2048),
            fetched_at: chrono::Utc::now(),
            links: vec![],
            tenant_id: None,
            etag: None,
            last_modified: None,
            cwv_lcp: None,
            cwv_cls: None,
            cwv_inp: None,
            has_structured_data: None,
            schema_types: None,
            viewport_ok: None,
            has_csp: None,
            has_hsts: None,
            images_total: None,
            images_missing_alt: None,
            h1_count: None,
            heading_count: None,
            extractions: None,
        };
        storage.insert_pages(&crawl_id, &[page]).unwrap();
        storage
            .insert_issue(&crawlkit_engine::storage::Issue {
                id: "i1".to_string(),
                page_id: "p1".to_string(),
                category: crawlkit_engine::types::IssueCategory::Seo,
                severity: crawlkit_engine::types::Severity::Error,
                code: "SEO001".to_string(),
                title: "Missing title".to_string(),
                description: "d".to_string(),
                element: None,
                recommendation: "r".to_string(),
                tenant_id: None,
            })
            .unwrap();

        let backend: &dyn crawlkit_engine::storage_trait::StorageBackend = &storage;
        snapshot_and_store_report(backend, &crawl_id, "2026-10-31T00:00:00+00:00", &db_path);

        // History snapshot recorded.
        let trend = storage.history_trend(None).unwrap();
        assert_eq!(trend.len(), 1);
        assert_eq!(trend[0].last_value(), Some(1.0));

        // Report stored + served through the handler.
        let state = make_state(dir.path());
        let response = get_report(State(state), Path(crawl_id.clone()))
            .await
            .unwrap();
        assert!(
            response.0.contains("https://example.com/"),
            "{}",
            response.0
        );
        assert!(response.0.contains("seo"));
        std::env::remove_var("CRAWLKIT_DB_PATH");
    }
}
