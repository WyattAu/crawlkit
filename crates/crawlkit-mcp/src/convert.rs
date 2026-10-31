//! Typed MCP tool outputs.
//!
//! Every tool returns JSON matching a documented schema, so each output type
//! derives [`rmcp::schemars::JsonSchema`] — the schema that `tools/list`
//! advertises and that the `tool_schema_matches_meta_schema` test pins down.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Output of the `crawl` tool: run id plus crawl summary.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrawlSummary {
    /// Storage ID of the run — pass to `report`, `compare`, `findings_search`.
    pub crawl_id: String,
    /// Host/domain that was crawled.
    pub seed_domain: String,
    /// Pages fetched and analyzed.
    pub pages_crawled: u64,
    /// Pages persisted to the SQLite store.
    pub pages_stored: u64,
    /// Defect-level findings across all pages.
    pub issues_found: u64,
    /// Measurement rows (readability, keywords, scores) — not defects.
    pub measurements_found: u64,
    /// Findings from post-crawl cross-page analyzers.
    pub cross_page_findings: u64,
    /// Pages skipped because they were off-site.
    pub skipped_external: u64,
    /// Pages skipped because robots.txt disallows them.
    pub skipped_robots: u64,
    /// Pages skipped as duplicate content.
    pub skipped_duplicate: u64,
    /// Wall-clock crawl duration in milliseconds.
    pub elapsed_ms: u64,
    /// SQLite database holding the crawl (`<out_dir>/crawlkit.db`).
    pub db_path: String,
}

/// Output of the `compare` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompareOutput {
    /// Echo of the requested format: `json` or `markdown`.
    pub format: String,
    /// Number of detected changes (added/removed/changed pages, CWV shifts).
    pub total_changes: u64,
    /// Structured diff document (`format: "json"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<serde_json::Value>,
    /// Rendered markdown diff (`format: "markdown"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

/// Output of the `report` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReportOutput {
    /// Crawl the report was rendered for.
    pub crawl_id: String,
    /// Echo of the requested format.
    pub format: String,
    /// The rendered report body.
    pub content: String,
}

/// Output of the `findings_search` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindingsOutput {
    /// Crawl the findings belong to.
    pub crawl_id: String,
    /// Matches before `limit` was applied.
    pub total_matches: u64,
    /// Findings returned (`<= limit`, itself `<= 500`).
    pub returned: u64,
    /// Finding objects (id, url, category, severity, code, title, ...).
    pub findings: Vec<serde_json::Value>,
}

/// Output of the `schedule_list` tool: a proxied crawlkit-api response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScheduleListOutput {
    /// API base the request was made against.
    pub base_url: String,
    /// HTTP status returned by the API.
    pub status: u16,
    /// The API's schedule list document.
    pub schedules: serde_json::Value,
}
