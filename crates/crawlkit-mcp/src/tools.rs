//! Thin tool implementations (spec G1: reuse, don't reimplement — every tool
//! delegates to an existing `crawlkit-engine` or `crawlkit-api` function; no
//! analysis logic lives here).

use std::path::{Path, PathBuf};

use crawlkit_engine::storage::{Issue, IssueCategory, Severity};
use rmcp::ErrorData;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::convert::{
    CompareOutput, CrawlSummary, FindingsOutput, ReportOutput, ScheduleListOutput,
};

/// Field inside a request's `_meta` object that must carry the API key when
/// the server is started with one (see README "Authentication").
pub const META_API_KEY_FIELD: &str = "crawlkit.api_key";

/// Default schedule API endpoint when `base_url` is omitted.
const DEFAULT_API_BASE: &str = "http://127.0.0.1:8080";

/// Upper bound on rows returned by a single `findings_search` call.
const FINDINGS_LIMIT_MAX: usize = 500;
/// Default row count for `findings_search`.
const FINDINGS_LIMIT_DEFAULT: usize = 50;

/// Map an internal failure onto the JSON-RPC `internal error` shape.
fn internal(context: impl std::fmt::Display, error: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(format!("{context}: {error}"), None)
}

/// Reject a malformed tool argument with the JSON-RPC `invalid params` shape.
fn invalid_params(message: impl std::fmt::Display) -> ErrorData {
    ErrorData::invalid_params(message.to_string(), None)
}

/// Resolve a user-supplied crawl location (directory or SQLite file) to the
/// database path, matching the CLI convention of `<dir>/crawlkit.db`.
fn resolve_db(path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_dir() {
        path.join("crawlkit.db")
    } else {
        path.to_path_buf()
    }
}

/// Open a crawl store, resolving `crawl_dir` and defaulting `crawl_id` to the
/// latest crawl when absent.
///
/// Read-path validation: the store must already exist. `Storage::new` opens
/// SQLite in create-if-missing mode, so opening a nonexistent path would
/// silently materialize an empty `crawlkit.db` (with full schema) wherever the
/// caller typo'd, then fail confusingly with "no crawls found". Reject it as
/// invalid params instead, and never touch the filesystem beyond the stat.
fn open_crawl(
    crawl_dir: &str,
    crawl_id: &Option<String>,
) -> Result<(crawlkit_engine::storage::Storage, String), ErrorData> {
    let db_path = resolve_db(crawl_dir);
    if !db_path.exists() {
        return Err(invalid_params(format!(
            "crawl store not found at {} (pass an existing crawl directory or .db file)",
            db_path.display()
        )));
    }
    let storage = crawlkit_engine::storage::Storage::new(&db_path)
        .map_err(|e| internal(format!("failed to open {}", db_path.display()), e))?;
    let id = match crawl_id {
        Some(id) => id.clone(),
        None => storage
            .get_latest_crawl_id()
            .map_err(|e| internal("failed to list crawls", e))?
            .ok_or_else(|| invalid_params(format!("no crawls found in {}", db_path.display())))?,
    };
    Ok((storage, id))
}

// ---------------------------------------------------------------------------
// crawl
// ---------------------------------------------------------------------------

/// Arguments for the `crawl` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct CrawlArgs {
    /// Absolute URL where the crawl starts (the seed).
    pub url: String,
    /// Directory for the crawl database. The SQLite store is written to
    /// `<out_dir>/crawlkit.db` and later accepted by `report` / `compare` /
    /// `findings_search`.
    pub out_dir: String,
    /// Maximum number of pages to crawl (default 100).
    #[serde(default)]
    pub max_pages: Option<u32>,
    /// Milliseconds to wait between same-domain requests (default 500).
    #[serde(default)]
    pub request_delay_ms: Option<u64>,
    /// Respect robots.txt disallow rules (default true).
    #[serde(default)]
    pub respect_robots_txt: Option<bool>,
}

/// Start a crawl and block until it completes (spec G1 v1: start + summary;
/// polling is a follow-up). Thin wrapper over
/// [`crawlkit_engine::crawl_engine::CrawlEngine::run`].
pub async fn run_crawl(args: CrawlArgs) -> Result<CrawlSummary, ErrorData> {
    let out_dir = PathBuf::from(&args.out_dir);
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| internal(format!("failed to create out_dir {}", out_dir.display()), e))?;
    let db_path = out_dir.join("crawlkit.db");
    let storage = crawlkit_engine::storage::Storage::new(&db_path).map_err(|e| {
        internal(
            format!("failed to open crawl database {}", db_path.display()),
            e,
        )
    })?;

    let defaults = crawlkit_engine::CrawlConfig::default();
    let crawl_config = crawlkit_engine::CrawlConfig {
        start_url: url::Url::parse(&args.url)
            .map_err(|e| invalid_params(format!("invalid `url`: {e}")))?,
        max_pages: args.max_pages.map_or(defaults.max_pages, |max_pages| {
            usize::try_from(max_pages).unwrap_or(usize::MAX)
        }),
        request_delay: args
            .request_delay_ms
            .map_or(defaults.request_delay, std::time::Duration::from_millis),
        respect_robots_txt: args
            .respect_robots_txt
            .unwrap_or(defaults.respect_robots_txt),
        ..defaults
    };

    let engine = crawlkit_engine::crawl_engine::CrawlEngine::new(
        crawlkit_engine::crawl_engine::CrawlEngineConfig {
            crawl_config,
            ..crawlkit_engine::crawl_engine::CrawlEngineConfig::default()
        },
        storage,
    );
    let output = engine
        .run(&args.url)
        .await
        .map_err(|e| internal("crawl failed", e))?;
    Ok(CrawlSummary {
        crawl_id: output.crawl_id,
        seed_domain: output.seed_domain,
        pages_crawled: output.pages_crawled as u64,
        pages_stored: output.pages_stored as u64,
        issues_found: output.issues_found as u64,
        measurements_found: output.measurements_found as u64,
        cross_page_findings: output.cross_page_findings.len() as u64,
        skipped_external: output.skipped_external as u64,
        skipped_robots: output.skipped_robots as u64,
        skipped_duplicate: output.skipped_duplicate as u64,
        elapsed_ms: u64::try_from(output.elapsed.as_millis()).unwrap_or(u64::MAX),
        db_path: db_path.display().to_string(),
    })
}

// ---------------------------------------------------------------------------
// compare
// ---------------------------------------------------------------------------

/// Arguments for the `compare` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct CompareArgs {
    /// Baseline crawl: a crawl directory or a `crawlkit.db` file.
    pub baseline: String,
    /// Target crawl: a crawl directory or a `crawlkit.db` file.
    pub target: String,
    /// `json` (default) or `markdown`.
    #[serde(default)]
    pub format: Option<String>,
}

/// Diff two crawl databases. Thin wrapper over
/// [`crawlkit_engine::compare::compare_crawls`].
pub fn run_compare(args: CompareArgs) -> Result<CompareOutput, ErrorData> {
    let baseline = resolve_db(&args.baseline);
    let target = resolve_db(&args.target);
    let diff = crawlkit_engine::compare::compare_crawls(&baseline, &target)
        .map_err(|e| internal("compare failed", e))?;
    let total_changes = diff.total_changes() as u64;
    match args.format.as_deref() {
        Some("markdown" | "md") => Ok(CompareOutput {
            format: "markdown".to_string(),
            total_changes,
            diff: None,
            markdown: Some(crawlkit_engine::compare::diff_to_markdown(&diff)),
        }),
        _ => {
            let json = crawlkit_engine::compare::diff_to_json(&diff, false)
                .map_err(|e| internal("failed to serialize diff", e))?;
            let diff_json: serde_json::Value =
                serde_json::from_str(&json).map_err(|e| internal("failed to parse diff", e))?;
            Ok(CompareOutput {
                format: "json".to_string(),
                total_changes,
                diff: Some(diff_json),
                markdown: None,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// report
// ---------------------------------------------------------------------------

/// Arguments for the `report` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ReportArgs {
    /// Crawl directory (or `crawlkit.db` file) to render.
    pub crawl_dir: String,
    /// Specific crawl ID; defaults to the latest crawl in the store.
    #[serde(default)]
    pub crawl_id: Option<String>,
    /// `json` (default), `markdown` (`md`), `csv`, or `html`.
    #[serde(default)]
    pub format: Option<String>,
}

/// Render a report for an existing crawl. Thin wrapper over the
/// [`crawlkit_engine::export`] renderers — the same engine code paths the CLI
/// report command uses.
pub fn render_report(args: ReportArgs) -> Result<ReportOutput, ErrorData> {
    let (storage, crawl_id) = open_crawl(&args.crawl_dir, &args.crawl_id)?;
    let format = args.format.as_deref().unwrap_or("json");
    let fail = |e: crawlkit_engine::export::ExportError| {
        ErrorData::internal_error(format!("failed to render {format} report: {e}"), None)
    };
    let content: String = match format {
        "markdown" | "md" => {
            crawlkit_engine::export::export_markdown(&storage, &crawl_id).map_err(fail)?
        }
        "csv" => {
            let bytes = crawlkit_engine::export::export_csv(
                &storage,
                &crawl_id,
                &crawlkit_engine::export::CsvColumnSelector::default(),
            )
            .map_err(fail)?;
            String::from_utf8(bytes).map_err(|e| internal("csv report is not valid UTF-8", e))?
        }
        "html" => crawlkit_engine::export::export_html(&storage, &crawl_id).map_err(fail)?,
        _ => crawlkit_engine::export::export_json(&storage, &crawl_id, true).map_err(fail)?,
    };
    Ok(ReportOutput {
        crawl_id,
        format: format.to_string(),
        content,
    })
}

// ---------------------------------------------------------------------------
// findings_search
// ---------------------------------------------------------------------------

/// Arguments for the `findings_search` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct FindingsSearchArgs {
    /// Crawl directory or `crawlkit.db` file to query.
    pub db_path: String,
    /// Crawl ID to query; defaults to the latest crawl in the store.
    #[serde(default)]
    pub crawl_id: Option<String>,
    /// Filter by severity: `critical`, `error`, `warning`, or `info`.
    #[serde(default)]
    pub severity: Option<String>,
    /// Filter by category: `http`, `seo`, `content`, `links`, `images`,
    /// `schema`, `security`, `performance`, `mobile`, `accessibility`,
    /// `social`, or any custom analyzer category (passed through verbatim).
    #[serde(default)]
    pub category: Option<String>,
    /// Filter by issue-code prefix (e.g. `SEO`, `SEC`).
    #[serde(default)]
    pub code_prefix: Option<String>,
    /// Maximum rows to return, 1–500 (default 50). `total_matches` reports
    /// the untruncated count.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Query findings from the SQLite store. Thin wrapper over
/// [`crawlkit_engine::storage::Storage::get_issues`] with an
/// [`crawlkit_engine::storage::IssueFilter`].
pub fn search_findings(args: FindingsSearchArgs) -> Result<FindingsOutput, ErrorData> {
    let (storage, crawl_id) = open_crawl(&args.db_path, &args.crawl_id)?;

    let severity = args
        .severity
        .as_deref()
        .map(parse_severity)
        .transpose()
        .map_err(invalid_params)?;
    let category = args
        .category
        .as_deref()
        .map(parse_category)
        .transpose()
        .map_err(invalid_params)?;
    let filter = crawlkit_engine::storage::IssueFilter {
        severity,
        category,
        page_id: None,
        code_prefix: args.code_prefix.clone(),
        kind: crawlkit_engine::storage::IssueKindFilter::default(),
    };

    let mut issues = storage
        .get_issues(&crawl_id, &filter)
        .map_err(|e| internal("findings query failed", e))?;
    let total_matches = issues.len() as u64;
    let limit = args
        .limit
        .unwrap_or(FINDINGS_LIMIT_DEFAULT)
        .clamp(1, FINDINGS_LIMIT_MAX);
    issues.truncate(limit);
    let findings = issues.iter().map(issue_to_json).collect();
    Ok(FindingsOutput {
        crawl_id,
        total_matches,
        returned: issues.len() as u64,
        findings,
    })
}

/// Parse a severity name as accepted by the `findings_search` tool.
fn parse_severity(name: &str) -> Result<Severity, String> {
    match name.to_ascii_lowercase().as_str() {
        "critical" => Ok(Severity::Critical),
        "error" => Ok(Severity::Error),
        "warning" => Ok(Severity::Warning),
        "info" => Ok(Severity::Info),
        other => Err(format!(
            "invalid severity '{other}': expected critical, error, warning, or info"
        )),
    }
}

/// Parse a category name as accepted by the `findings_search` tool. Built-in
/// categories are matched case-insensitively; anything else is treated as a
/// custom analyzer category and passed through verbatim (the storage layer
/// matches on it exactly).
fn parse_category(name: &str) -> Result<IssueCategory, String> {
    match name.to_ascii_lowercase().as_str() {
        "http" => Ok(IssueCategory::Http),
        "seo" => Ok(IssueCategory::Seo),
        "content" => Ok(IssueCategory::Content),
        "links" => Ok(IssueCategory::Links),
        "images" => Ok(IssueCategory::Images),
        "schema" => Ok(IssueCategory::Schema),
        "security" => Ok(IssueCategory::Security),
        "performance" => Ok(IssueCategory::Performance),
        "mobile" => Ok(IssueCategory::Mobile),
        "accessibility" => Ok(IssueCategory::Accessibility),
        "social" => Ok(IssueCategory::Social),
        _ => Ok(IssueCategory::Custom(name.to_string())),
    }
}

/// Convert a stored finding to its JSON shape for tool output.
pub fn issue_to_json(issue: &Issue) -> serde_json::Value {
    serde_json::json!({
        "id": issue.id,
        "page_id": issue.page_id,
        "category": issue.category.as_str(),
        "severity": issue.severity.as_str(),
        "code": issue.code,
        "title": issue.title,
        "description": issue.description,
        "element": issue.element,
        "recommendation": issue.recommendation,
    })
}

// ---------------------------------------------------------------------------
// schedule_list
// ---------------------------------------------------------------------------

/// Arguments for the `schedule_list` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ScheduleListArgs {
    /// Base URL of a running `crawlkit-api` server
    /// (default `http://127.0.0.1:8080`).
    #[serde(default)]
    pub base_url: Option<String>,
}

/// List scheduled crawls. Thin wrapper over the crawlkit-api
/// `GET /api/v1/schedules` endpoint; the server's API key (if configured) is
/// sent as the bearer token.
pub async fn list_schedules(
    args: ScheduleListArgs,
    api_key: Option<&str>,
) -> Result<ScheduleListOutput, ErrorData> {
    let base = args
        .base_url
        .as_deref()
        .map(|url| url.trim_end_matches('/'))
        .unwrap_or(DEFAULT_API_BASE)
        .to_string();
    let url = format!("{base}/api/v1/schedules");
    let client = reqwest::Client::new();
    let mut request = client.get(&url);
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| internal(format!("schedule API unreachable at {url}"), e))?;
    let status = response.status().as_u16();
    let schedules: serde_json::Value = response
        .json()
        .await
        .map_err(|e| internal("schedule API returned invalid JSON", e))?;
    Ok(ScheduleListOutput {
        base_url: base,
        status,
        schedules,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn resolve_db_appends_crawlkit_db_for_directories() {
        let dir = std::env::temp_dir();
        assert_eq!(resolve_db(dir.to_str().unwrap()), dir.join("crawlkit.db"));
        assert_eq!(resolve_db("/tmp/some.db"), PathBuf::from("/tmp/some.db"));
    }

    /// A read tool pointed at a path that does not exist must fail as invalid
    /// params and must NOT create an empty store at that path (SQLite's
    /// create-if-missing open previously materialized one on any typo).
    #[test]
    fn open_crawl_rejects_missing_store_without_creating_it() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nowhere");
        let err = open_crawl(
            missing.to_str().unwrap(),
            &Some("some-crawl-id".to_string()),
        )
        .err()
        .expect("missing store must error");
        assert_eq!(err.code.0, -32602, "expected invalid-params code");
        assert!(
            !missing.exists() && !missing.join("crawlkit.db").exists(),
            "no store may be created at the missing path"
        );
    }

    #[test]
    fn severity_parsing_covers_all_variants_and_rejects_garbage() {
        assert!(matches!(parse_severity("critical"), Ok(Severity::Critical)));
        assert!(matches!(parse_severity("ERROR"), Ok(Severity::Error)));
        assert!(matches!(parse_severity("warning"), Ok(Severity::Warning)));
        assert!(matches!(parse_severity("info"), Ok(Severity::Info)));
        assert!(parse_severity("catastrophic").is_err());
    }

    #[test]
    fn category_parsing_maps_known_names_and_passes_custom_through() {
        assert!(matches!(parse_category("seo"), Ok(IssueCategory::Seo)));
        assert!(matches!(parse_category("Links"), Ok(IssueCategory::Links)));
        match parse_category("my-custom-thing") {
            Ok(IssueCategory::Custom(name)) => assert_eq!(name, "my-custom-thing"),
            other => panic!("expected custom category, got {other:?}"),
        }
    }

    #[test]
    fn issue_to_json_includes_recommendation_and_codes() {
        let issue = Issue {
            id: "issue-1".to_string(),
            page_id: "page-1".to_string(),
            category: IssueCategory::Seo,
            severity: Severity::Warning,
            code: "SEO001".to_string(),
            title: "Missing title".to_string(),
            description: "The page has no title.".to_string(),
            element: None,
            recommendation: "Add a title element.".to_string(),
            tenant_id: None,
        };
        let json = issue_to_json(&issue);
        assert_eq!(json["code"], "SEO001");
        assert_eq!(json["severity"], "warning");
        assert_eq!(json["recommendation"], "Add a title element.");
        assert_eq!(json["element"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn schedule_list_surfaces_unreachable_api_as_internal_error() {
        // Nothing listens on the discard port: the tool must return a clean
        // JSON-RPC internal error instead of panicking.
        let err = list_schedules(
            ScheduleListArgs {
                base_url: Some("http://127.0.0.1:9".to_string()),
            },
            None,
        )
        .await
        .expect_err("unreachable API must error");
        assert_eq!(err.code.0, -32603, "expected the internal-error code");
    }
}
