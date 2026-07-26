use rusqlite::Connection;
use serde::Serialize;
use thiserror::Error;

use crate::storage::{CrawlStats, Storage, StorageError};
use crate::CrawlError;

/// Errors specific to export operations.
#[derive(Debug, Error)]
pub enum ExportError {
    /// I/O error during write.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// CSV serialization error.
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),

    /// JSON serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// SQLite error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

impl From<ExportError> for CrawlError {
    fn from(e: ExportError) -> Self {
        CrawlError::Storage(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// CSV Export
// ---------------------------------------------------------------------------

/// Column selector for CSV export.
#[derive(Debug, Clone, Default)]
pub struct CsvColumnSelector {
    /// If true, include the page URL.
    pub url: bool,
    /// If true, include status code.
    pub status_code: bool,
    /// If true, include page title.
    pub title: bool,
    /// If true, include meta description.
    pub description: bool,
    /// If true, include canonical URL.
    pub canonical: bool,
    /// If true, include word count.
    pub word_count: bool,
    /// If true, include load time in ms.
    pub load_time_ms: bool,
    /// If true, include body size in bytes.
    pub body_size: bool,
    /// If true, include fetched_at timestamp.
    pub fetched_at: bool,
    /// If true, include issue count per page.
    pub issue_count: bool,
    /// If true, include all issues as JSON array.
    pub issues_json: bool,
    /// If true, include all links as JSON array.
    pub links_json: bool,
}

impl CsvColumnSelector {
    /// All columns enabled.
    pub fn all() -> Self {
        Self {
            url: true,
            status_code: true,
            title: true,
            description: true,
            canonical: true,
            word_count: true,
            load_time_ms: true,
            body_size: true,
            fetched_at: true,
            issue_count: true,
            issues_json: true,
            links_json: true,
        }
    }

    /// Return ordered list of enabled column names.
    pub fn headers(&self) -> Vec<&'static str> {
        let mut h = Vec::new();
        if self.url {
            h.push("url");
        }
        if self.status_code {
            h.push("status_code");
        }
        if self.title {
            h.push("title");
        }
        if self.description {
            h.push("description");
        }
        if self.canonical {
            h.push("canonical");
        }
        if self.word_count {
            h.push("word_count");
        }
        if self.load_time_ms {
            h.push("load_time_ms");
        }
        if self.body_size {
            h.push("body_size");
        }
        if self.fetched_at {
            h.push("fetched_at");
        }
        if self.issue_count {
            h.push("issue_count");
        }
        if self.issues_json {
            h.push("issues");
        }
        if self.links_json {
            h.push("links");
        }
        h
    }
}

/// Export crawl data to CSV.
///
/// One row per page. Nested data (issues, links) is JSON-encoded.
///
/// Issues and links are fetched in bulk (single query each) and grouped
/// by page_id in memory, avoiding N+1 query patterns for large crawls.
pub fn export_csv(
    conn: &Connection,
    crawl_id: &str,
    selector: &CsvColumnSelector,
) -> Result<Vec<u8>, ExportError> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(selector.headers())?;

    let pages = read_pages(conn, crawl_id)?;
    let issues_by_page = read_issues_grouped_by_page(conn, crawl_id)?;
    let links_by_page = read_links_grouped_by_page(conn, crawl_id)?;

    for page in &pages {
        let issues = issues_by_page.get(&page.id).cloned().unwrap_or_default();
        let links = links_by_page.get(&page.id).cloned().unwrap_or_default();
        let issue_count = issues.len();

        let mut record: Vec<String> = Vec::new();
        if selector.url {
            record.push(page.url.clone());
        }
        if selector.status_code {
            record.push(page.status_code.to_string());
        }
        if selector.title {
            record.push(page.title.clone().unwrap_or_default());
        }
        if selector.description {
            record.push(page.description.clone().unwrap_or_default());
        }
        if selector.canonical {
            record.push(page.canonical.clone().unwrap_or_default());
        }
        if selector.word_count {
            record.push(page.word_count.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.load_time_ms {
            record.push(page.load_time_ms.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.body_size {
            record.push(page.body_size.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.fetched_at {
            record.push(page.fetched_at.clone());
        }
        if selector.issue_count {
            record.push(issue_count.to_string());
        }
        if selector.issues_json {
            record.push(serde_json::to_string(&issues).unwrap_or_default());
        }
        if selector.links_json {
            record.push(serde_json::to_string(&links).unwrap_or_default());
        }
        wtr.write_record(&record)?;
    }

    wtr.flush()?;
    let bytes = wtr
        .into_inner()
        .map_err(|e| ExportError::Io(std::io::Error::other(e.to_string())))?;
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// JSON Export
// ---------------------------------------------------------------------------

/// Schema version for JSON exports.
pub const JSON_SCHEMA_VERSION: &str = "1.0";

/// Top-level JSON export structure.
#[derive(Serialize)]
pub struct JsonExport {
    /// Schema version.
    pub schema_version: String,
    /// Crawl metadata.
    pub crawl: JsonCrawlMeta,
    /// All pages.
    pub pages: Vec<JsonPage>,
    /// Aggregate stats.
    pub stats: CrawlStats,
}

#[derive(Serialize)]
pub struct JsonCrawlMeta {
    pub id: String,
    pub target_url: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub pages_crawled: usize,
    pub total_issues: usize,
}

#[derive(Serialize)]
pub struct JsonPage {
    pub id: String,
    pub url: String,
    pub final_url: String,
    pub status_code: u16,
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical: Option<String>,
    pub word_count: Option<usize>,
    pub load_time_ms: Option<u64>,
    pub body_size: Option<usize>,
    pub fetched_at: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<JsonIssue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
}

#[derive(Serialize)]
pub struct JsonIssue {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub code: String,
    pub title: String,
    pub description: String,
    pub element: Option<String>,
    pub recommendation: String,
}

/// Export crawl data to a structured JSON value.
///
/// Issues and links are fetched in bulk (single query each) and grouped
/// by page_id in memory, avoiding N+1 query patterns for large crawls.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::storage::Storage;
/// use crawlkit_engine::export::export_json;
///
/// let storage = Storage::new_in_memory().unwrap();
/// let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
/// storage.finish_crawl(&crawl_id, 0, 0).unwrap();
///
/// let json = export_json(&storage, &crawl_id, false).unwrap();
/// assert!(json.contains("schema_version"));
/// ```
pub fn export_json(storage: &Storage, crawl_id: &str, pretty: bool) -> Result<String, ExportError> {
    let stats = storage.get_stats(crawl_id)?;
    let conn = storage.conn();
    let meta = read_crawl_meta(&conn, crawl_id)?;
    let pages = read_pages(&conn, crawl_id)?;
    let issues_by_page = read_issues_grouped_by_page(&conn, crawl_id)?;
    let links_by_page = read_links_grouped_by_page(&conn, crawl_id)?;

    let json_pages: Vec<JsonPage> = pages
        .iter()
        .map(|p| {
            let issues = issues_by_page
                .get(&p.id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|i| JsonIssue {
                    id: i.id,
                    category: i.category,
                    severity: i.severity,
                    code: i.code,
                    title: i.title,
                    description: i.description,
                    element: i.element,
                    recommendation: i.recommendation,
                })
                .collect();
            let links = links_by_page.get(&p.id).cloned().unwrap_or_default();
            JsonPage {
                id: p.id.clone(),
                url: p.url.clone(),
                final_url: p.final_url.clone(),
                status_code: p.status_code,
                title: p.title.clone(),
                description: p.description.clone(),
                canonical: p.canonical.clone(),
                word_count: p.word_count,
                load_time_ms: p.load_time_ms,
                body_size: p.body_size,
                fetched_at: p.fetched_at.clone(),
                issues,
                links,
            }
        })
        .collect();

    let export = JsonExport {
        schema_version: JSON_SCHEMA_VERSION.to_string(),
        crawl: meta,
        pages: json_pages,
        stats,
    };

    if pretty {
        Ok(serde_json::to_string_pretty(&export)?)
    } else {
        Ok(serde_json::to_string(&export)?)
    }
}

// ---------------------------------------------------------------------------
// Markdown Summary
// ---------------------------------------------------------------------------

/// Generate a Markdown summary report.
pub fn export_markdown(storage: &Storage, crawl_id: &str) -> Result<String, ExportError> {
    let stats = storage.get_stats(crawl_id)?;
    let conn = storage.conn();
    let meta = read_crawl_meta(&conn, crawl_id)?;
    let top_issues = read_top_issues(&conn, crawl_id, 10)?;

    let mut md = String::new();

    md.push_str(&format!("# Crawl Report — `{}`\n\n", meta.target_url));
    md.push_str(&format!("**Crawl ID:** `{}`  \n", crawl_id));
    if let Some(ref start) = meta.start_time {
        md.push_str(&format!("**Started:** {start}  \n"));
    }
    if let Some(ref end) = meta.end_time {
        md.push_str(&format!("**Finished:** {end}  \n"));
    }
    md.push('\n');

    // Summary
    md.push_str("## Summary\n\n");
    md.push_str("| Metric | Value |\n|---|---|\n");
    md.push_str(&format!("| Pages crawled | {} |\n", stats.total_pages));
    md.push_str(&format!("| Total issues | {} |\n", stats.total_issues));
    if let Some(avg) = stats.avg_response_time_ms {
        md.push_str(&format!("| Avg response time | {avg:.0} ms |\n"));
    }
    if let Some(size) = stats.total_body_size {
        md.push_str(&format!(
            "| Total body size | {:.2} KB |\n",
            size as f64 / 1024.0
        ));
    }
    md.push('\n');

    // Issues by severity
    md.push_str("## Issues by Severity\n\n");
    let severity_order = ["critical", "error", "warning", "info"];
    md.push_str("| Severity | Count |\n|---|---|\n");
    for sev in &severity_order {
        let count = stats.issues_by_severity.get(*sev).unwrap_or(&0);
        md.push_str(&format!("| {sev} | {count} |\n"));
    }
    md.push('\n');

    // Issues by category
    md.push_str("## Issues by Category\n\n");
    md.push_str("| Category | Count |\n|---|---|\n");
    let mut cats: Vec<_> = stats.issues_by_category.iter().collect();
    cats.sort_by(|a, b| b.1.cmp(a.1));
    for (cat, count) in &cats {
        md.push_str(&format!("| {cat} | {count} |\n"));
    }
    md.push('\n');

    // Top issues
    if !top_issues.is_empty() {
        md.push_str("## Top Issues\n\n");
        md.push_str("| # | Severity | Code | Title | Pages |\n|---|---|---|---|---|\n");
        for (i, ti) in top_issues.iter().enumerate() {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                i + 1,
                ti.severity,
                ti.code,
                ti.title,
                ti.affected_pages
            ));
        }
        md.push('\n');
    }

    Ok(md)
}

// ---------------------------------------------------------------------------
// HTML Report
// ---------------------------------------------------------------------------

/// Generate a self-contained HTML report.
///
/// Issues are fetched in bulk (single query) and counted per page in
/// memory, avoiding N+1 query patterns for large crawls.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::storage::Storage;
/// use crawlkit_engine::export::export_html;
///
/// let storage = Storage::new_in_memory().unwrap();
/// let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
/// storage.finish_crawl(&crawl_id, 0, 0).unwrap();
///
/// let html = export_html(&storage, &crawl_id).unwrap();
/// assert!(html.contains("<!DOCTYPE html>"));
/// assert!(html.contains("Crawl Report"));
/// ```
pub fn export_html(storage: &Storage, crawl_id: &str) -> Result<String, ExportError> {
    let stats = storage.get_stats(crawl_id)?;
    let conn = storage.conn();
    let meta = read_crawl_meta(&conn, crawl_id)?;
    let top_issues = read_top_issues(&conn, crawl_id, 20)?;
    let pages = read_pages(&conn, crawl_id)?;
    let crux_metrics = read_crux_metrics(&conn, crawl_id)?;
    let issues_by_page = read_issues_grouped_by_page(&conn, crawl_id)?;

    let severity_color = |s: &str| -> &'static str {
        match s {
            "critical" => "#dc2626",
            "error" => "#ea580c",
            "warning" => "#ca8a04",
            "info" => "#2563eb",
            _ => "#6b7280",
        }
    };

    let mut rows = String::new();
    for page in &pages {
        let issue_count = issues_by_page.get(&page.id).map_or(0, |v| v.len());
        let status_class = if page.status_code >= 400 {
            "status-error"
        } else if page.status_code >= 300 {
            "status-redirect"
        } else {
            "status-ok"
        };
        rows.push_str(&format!(
            r#"<tr>
  <td><a href="{url}" target="_blank">{url}</a></td>
  <td class="{status_class}">{status}</td>
  <td>{title}</td>
  <td class="num">{wc}</td>
  <td class="num">{lt}</td>
  <td class="num">{ic}</td>
</tr>"#,
            url = page.url,
            status = page.status_code,
            title = page.title.as_deref().unwrap_or("—"),
            wc = page
                .word_count
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
            lt = page
                .load_time_ms
                .map(|v| format!("{v}ms"))
                .unwrap_or_else(|| "—".into()),
            ic = issue_count,
        ));
    }

    let mut issue_rows = String::new();
    for ti in &top_issues {
        let color = severity_color(&ti.severity);
        issue_rows.push_str(&format!(
            r#"<tr>
  <td><span class="badge" style="background:{color}">{sev}</span></td>
  <td>{code}</td>
  <td>{title}</td>
  <td class="num">{pages}</td>
</tr>"#,
            color = color,
            sev = ti.severity,
            code = ti.code,
            title = ti.title,
            pages = ti.affected_pages,
        ));
    }

    let severity_bars: String = severity_order()
        .iter()
        .map(|sev| {
            let count = stats.issues_by_severity.get(*sev).unwrap_or(&0);
            let color = severity_color(sev);
            let pct = if stats.total_issues > 0 {
                (*count as f64 / stats.total_issues as f64) * 100.0
            } else {
                0.0
            };
            format!(
                r#"<div class="bar" role="meter" aria-valuenow="{pct:.0}" aria-valuemin="0" aria-valuemax="100" aria-label="{sev}: {count}" style="width:{pct:.1}%;background:{color}"></div>"#,
            )
        })
        .collect();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Crawl Report — {target}</title>
<style>
  :root {{ --bg: #f9fafb; --fg: #111827; --card: #fff; --border: #e5e7eb; --muted: #6b7280; }}
  @media (prefers-color-scheme: dark) {{
    :root {{ --bg: #111827; --fg: #f9fafb; --card: #1f2937; --border: #374151; --muted: #9ca3af; }}
  }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: system-ui, -apple-system, sans-serif; background: var(--bg); color: var(--fg); line-height: 1.6; padding: 2rem; }}
  h1 {{ font-size: 1.5rem; margin-bottom: .5rem; }}
  h2 {{ font-size: 1.25rem; margin: 2rem 0 1rem; border-bottom: 1px solid var(--border); padding-bottom: .5rem; }}
  .meta {{ color: var(--muted); font-size: .875rem; margin-bottom: 1.5rem; }}
  .cards {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1rem; margin-bottom: 1.5rem; }}
  .card {{ background: var(--card); border: 1px solid var(--border); border-radius: .5rem; padding: 1rem; }}
  .card .value {{ font-size: 1.75rem; font-weight: 700; }}
  .card .label {{ color: var(--muted); font-size: .8rem; text-transform: uppercase; letter-spacing: .05em; }}
  .bar-chart {{ display: flex; height: 1.25rem; border-radius: .25rem; overflow: hidden; background: var(--border); margin-top: .5rem; }}
  .bar {{ height: 100%; }}
  table {{ width: 100%; border-collapse: collapse; font-size: .875rem; }}
  th, td {{ text-align: left; padding: .5rem .75rem; border-bottom: 1px solid var(--border); }}
  th {{ font-weight: 600; background: var(--card); position: sticky; top: 0; }}
  .num {{ text-align: right; }}
  .status-ok {{ color: #16a34a; }}
  .status-redirect {{ color: #ca8a04; }}
  .status-error {{ color: #dc2626; }}
  .badge {{ display: inline-block; padding: .125rem .5rem; border-radius: .25rem; color: #fff; font-size: .75rem; font-weight: 600; }}
  .search {{ margin-bottom: 1rem; }}
  .search input {{ width: 100%; padding: .5rem .75rem; border: 1px solid var(--border); border-radius: .375rem; background: var(--card); color: var(--fg); font-size: .875rem; }}
  a {{ color: #2563eb; text-decoration: none; }}
  a:hover {{ text-decoration: underline; }}
  .sr-only {{ position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); border: 0; }}
  .cwv-good {{ color: #16a34a; font-weight: 600; }}
  .cwv-avg {{ color: #ca8a04; font-weight: 600; }}
  .cwv-poor {{ color: #dc2626; font-weight: 600; }}
  .cwv-na {{ color: var(--muted); font-style: italic; }}
  @media (max-width: 640px) {{
    body {{ padding: 1rem; }}
    .cards {{ grid-template-columns: repeat(2, 1fr); }}
  }}
</style>
</head>
<body>
<h1>Crawl Report</h1>
<div class="meta">
  <code>{target}</code> &middot; {crawl_id}
</div>

<div class="cards">
  <div class="card"><div class="value">{total_pages}</div><div class="label">Pages</div></div>
  <div class="card"><div class="value">{total_issues}</div><div class="label">Issues</div></div>
  <div class="card"><div class="value">{avg_time}</div><div class="label">Avg Load</div></div>
  <div class="card"><div class="value">{total_size}</div><div class="label">Total Size</div></div>
</div>

{cwv_summary_cards}

<h2>Issue Distribution</h2>
<div class="bar-chart" role="img" aria-label="Issue distribution by severity">{severity_bars}</div>

<h2>Issues by Category</h2>
<table>
<thead><tr><th scope="col">Category</th><th scope="col" style="text-align:right">Count</th></tr></thead>
<tbody>{category_rows}</tbody>
</table>

<h2>Top Issues</h2>
<table>
<thead><tr><th scope="col">Severity</th><th scope="col">Code</th><th scope="col">Title</th><th scope="col" style="text-align:right">Pages</th></tr></thead>
<tbody>{issue_rows}</tbody>
</table>

{cwv_detail_section}

<h2>All Pages</h2>
<div class="search"><label for="search" class="sr-only">Filter pages</label><input type="text" id="search" placeholder="Filter pages..." oninput="filterTable()"></div>
<div style="overflow-x:auto">
<table id="pages-table">
<thead><tr><th scope="col">URL</th><th scope="col">Status</th><th scope="col">Title</th><th scope="col" style="text-align:right">Words</th><th scope="col" style="text-align:right">Load</th><th scope="col" style="text-align:right">Issues</th></tr></thead>
<tbody>{rows}</tbody>
</table>
</div>

<script>
function filterTable() {{
  const q = document.getElementById('search').value.toLowerCase();
  const rows = document.querySelectorAll('#pages-table tbody tr');
  rows.forEach(r => {{ r.style.display = r.textContent.toLowerCase().includes(q) ? '' : 'none'; }});
}}
</script>
</body>
</html>"#,
        target = meta.target_url,
        crawl_id = crawl_id,
        total_pages = stats.total_pages,
        total_issues = stats.total_issues,
        avg_time = stats
            .avg_response_time_ms
            .map(|v| format!("{v:.0}ms"))
            .unwrap_or_else(|| "—".into()),
        total_size = stats
            .total_body_size
            .map(|v| format!("{:.1} KB", v as f64 / 1024.0))
            .unwrap_or_else(|| "—".into()),
        cwv_summary_cards = build_cwv_summary_cards(&crux_metrics),
        severity_bars = severity_bars,
        category_rows = {
            let mut cats: Vec<_> = stats.issues_by_category.iter().collect();
            cats.sort_by(|a, b| b.1.cmp(a.1));
            cats.iter()
                .map(|(cat, count)| {
                    format!("<tr><td>{cat}</td><td class=\"num\">{count}</td></tr>")
                })
                .collect::<String>()
        },
        issue_rows = issue_rows,
        cwv_detail_section = build_cwv_detail_section(&crux_metrics),
        rows = rows,
    );

    Ok(html)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn severity_order() -> &'static [&'static str] {
    &["critical", "error", "warning", "info"]
}

#[derive(Debug, serde::Serialize)]
struct PageRow {
    id: String,
    url: String,
    final_url: String,
    status_code: u16,
    title: Option<String>,
    description: Option<String>,
    canonical: Option<String>,
    word_count: Option<usize>,
    load_time_ms: Option<u64>,
    body_size: Option<usize>,
    fetched_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct IssueRow {
    id: String,
    page_id: String,
    category: String,
    severity: String,
    code: String,
    title: String,
    description: String,
    element: Option<String>,
    recommendation: String,
}

#[derive(Debug, serde::Serialize)]
struct TopIssue {
    severity: String,
    code: String,
    title: String,
    affected_pages: usize,
}

fn read_crawl_meta(conn: &Connection, crawl_id: &str) -> Result<JsonCrawlMeta, ExportError> {
    conn.query_row(
        "SELECT id, target_url, start_time, end_time, pages_crawled, total_issues FROM crawls WHERE id = ?1",
        [crawl_id],
        |row| {
            Ok(JsonCrawlMeta {
                id: row.get(0)?,
                target_url: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                pages_crawled: row.get::<_, i64>(4)? as usize,
                total_issues: row.get::<_, i64>(5)? as usize,
            })
        },
    )
    .map_err(ExportError::from)
}

fn read_pages(conn: &Connection, crawl_id: &str) -> Result<Vec<PageRow>, ExportError> {
    let mut stmt = conn.prepare(
        "SELECT id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at
         FROM pages WHERE crawl_id = ?1 ORDER BY fetched_at ASC",
    )?;
    let rows = stmt
        .query_map([crawl_id], |row| {
            Ok(PageRow {
                id: row.get(0)?,
                url: row.get(1)?,
                final_url: row.get(2)?,
                status_code: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                canonical: row.get(6)?,
                word_count: row.get::<_, Option<i64>>(7)?.map(|v| v as usize),
                load_time_ms: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                body_size: row.get::<_, Option<i64>>(9)?.map(|v| v as usize),
                fetched_at: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Load all issues for a crawl in a single query, grouped by page_id.
///
/// This replaces per-page N+1 queries with a single bulk fetch,
/// then groups the results in memory by page_id for O(1) lookup.
fn read_issues_grouped_by_page(
    conn: &Connection,
    crawl_id: &str,
) -> Result<std::collections::HashMap<String, Vec<IssueRow>>, ExportError> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.page_id, f.category, f.severity, f.code, f.title, f.description, f.element, f.recommendation
         FROM findings f
         JOIN pages p ON f.page_id = p.id
         WHERE p.crawl_id = ?1",
    )?;
    let rows = stmt
        .query_map([crawl_id], |row| {
            Ok(IssueRow {
                id: row.get(0)?,
                page_id: row.get(1)?,
                category: row.get(2)?,
                severity: row.get(3)?,
                code: row.get(4)?,
                title: row.get(5)?,
                description: row.get(6)?,
                element: row.get(7)?,
                recommendation: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut grouped: std::collections::HashMap<String, Vec<IssueRow>> =
        std::collections::HashMap::new();
    for row in rows {
        grouped.entry(row.page_id.clone()).or_default().push(row);
    }
    Ok(grouped)
}

/// Load all links for a crawl in a single query, grouped by page_id.
///
/// Replaces per-page N+1 queries with a single bulk fetch.
fn read_links_grouped_by_page(
    conn: &Connection,
    crawl_id: &str,
) -> Result<std::collections::HashMap<String, Vec<String>>, ExportError> {
    let mut stmt = conn.prepare(
        "SELECT l.page_id, l.target_url
         FROM links l
         JOIN pages p ON l.page_id = p.id
         WHERE p.crawl_id = ?1",
    )?;
    let rows = stmt
        .query_map([crawl_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut grouped: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (page_id, target_url) in rows {
        grouped.entry(page_id).or_default().push(target_url);
    }
    Ok(grouped)
}

#[derive(Debug, Clone)]
struct CruxRow {
    url: String,
    lcp_p75: Option<f64>,
    inp_p75: Option<f64>,
    cls_p75: Option<f64>,
    fcp_p75: Option<f64>,
    ttfb_p75: Option<f64>,
}

fn read_crux_metrics(conn: &Connection, crawl_id: &str) -> Result<Vec<CruxRow>, ExportError> {
    let mut stmt = conn.prepare(
        "SELECT cm.url, cm.lcp_p75, cm.inp_p75, cm.cls_p75, cm.fcp_p75, cm.ttfb_p75
         FROM crux_metrics cm
         JOIN pages p ON cm.page_id = p.id
         WHERE p.crawl_id = ?1",
    )?;
    let rows = stmt
        .query_map([crawl_id], |row| {
            Ok(CruxRow {
                url: row.get(0)?,
                lcp_p75: row.get(1)?,
                inp_p75: row.get(2)?,
                cls_p75: row.get(3)?,
                fcp_p75: row.get(4)?,
                ttfb_p75: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn read_top_issues(
    conn: &Connection,
    crawl_id: &str,
    limit: usize,
) -> Result<Vec<TopIssue>, ExportError> {
    let mut stmt = conn.prepare(
        "SELECT f.severity, f.code, f.title, COUNT(DISTINCT f.page_id) as affected_pages
         FROM findings f
         JOIN pages p ON f.page_id = p.id
         WHERE p.crawl_id = ?1
         GROUP BY f.severity, f.code, f.title
         ORDER BY
           CASE f.severity WHEN 'critical' THEN 0 WHEN 'error' THEN 1 WHEN 'warning' THEN 2 ELSE 3 END,
           affected_pages DESC
         LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![crawl_id, limit as i64], |row| {
            Ok(TopIssue {
                severity: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                affected_pages: row.get::<_, i64>(3)? as usize,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn cwv_lcp_class(val: f64) -> &'static str {
    if val < 2500.0 {
        "cwv-good"
    } else if val < 4000.0 {
        "cwv-avg"
    } else {
        "cwv-poor"
    }
}

fn cwv_cls_class(val: f64) -> &'static str {
    if val < 0.1 {
        "cwv-good"
    } else if val < 0.25 {
        "cwv-avg"
    } else {
        "cwv-poor"
    }
}

fn cwv_inp_class(val: f64) -> &'static str {
    if val < 200.0 {
        "cwv-good"
    } else if val < 500.0 {
        "cwv-avg"
    } else {
        "cwv-poor"
    }
}

fn cwv_generic_class(_val: f64) -> &'static str {
    "cwv-good"
}

fn build_cwv_summary_cards(metrics: &[CruxRow]) -> String {
    if metrics.is_empty() {
        return String::new();
    }

    let fmt = |val: Option<f64>, unit: &str| -> (String, &'static str) {
        match val {
            Some(v) => {
                let cls = if unit == "ms" {
                    if v < 2500.0 {
                        "cwv-good"
                    } else if v < 4000.0 {
                        "cwv-avg"
                    } else {
                        "cwv-poor"
                    }
                } else if unit.is_empty() {
                    // CLS is unitless
                    if v < 0.1 {
                        "cwv-good"
                    } else if v < 0.25 {
                        "cwv-avg"
                    } else {
                        "cwv-poor"
                    }
                } else {
                    // ms for INP/FCP/TTFB
                    if v < 200.0 {
                        "cwv-good"
                    } else if v < 500.0 {
                        "cwv-avg"
                    } else {
                        "cwv-poor"
                    }
                };
                let display = if unit.is_empty() {
                    format!("{v:.3}")
                } else if unit == "ms" {
                    format!("{:.0}ms", v)
                } else {
                    format!("{v:.0}{unit}")
                };
                (display, cls)
            }
            None => ("N/A".into(), "cwv-na"),
        }
    };

    // Average across all pages
    let avg_lcp = average(metrics.iter().filter_map(|m| m.lcp_p75));
    let avg_cls = average(metrics.iter().filter_map(|m| m.cls_p75));
    let avg_inp = average(metrics.iter().filter_map(|m| m.inp_p75));

    let (lcp_display, lcp_cls) = fmt(avg_lcp, "ms");
    let (cls_display, cls_cls_val) = fmt(avg_cls, "");
    let (inp_display, inp_cls) = fmt(avg_inp, "ms");

    format!(
        r#"<div class="cards">
  <div class="card"><div class="value {lcp_cls}">{lcp_display}</div><div class="label">Avg LCP (p75)</div></div>
  <div class="card"><div class="value {cls_cls_val}">{cls_display}</div><div class="label">Avg CLS (p75)</div></div>
  <div class="card"><div class="value {inp_cls}">{inp_display}</div><div class="label">Avg INP (p75)</div></div>
  <div class="card"><div class="value">{}</div><div class="label">Pages with CrUX</div></div>
</div>"#,
        metrics.len(),
    )
}

fn build_cwv_detail_section(metrics: &[CruxRow]) -> String {
    if metrics.is_empty() {
        return String::new();
    }

    let mut rows = String::new();
    for m in metrics {
        let lcp_cls = m.lcp_p75.map(cwv_lcp_class).unwrap_or("cwv-na");
        let cls_cls = m.cls_p75.map(cwv_cls_class).unwrap_or("cwv-na");
        let inp_cls = m.inp_p75.map(cwv_inp_class).unwrap_or("cwv-na");
        let fcp_cls = m.fcp_p75.map(cwv_generic_class).unwrap_or("cwv-na");
        let ttfb_cls = m.ttfb_p75.map(cwv_generic_class).unwrap_or("cwv-na");

        rows.push_str(&format!(
            r#"<tr>
  <td><a href="{url}" target="_blank">{url}</a></td>
  <td class="num {lcp_cls}">{lcp}</td>
  <td class="num {cls_cls}">{cls}</td>
  <td class="num {inp_cls}">{inp}</td>
  <td class="num {fcp_cls}">{fcp}</td>
  <td class="num {ttfb_cls}">{ttfb}</td>
</tr>"#,
            url = m.url,
            lcp = m
                .lcp_p75
                .map(|v| format!("{:.0}ms", v))
                .unwrap_or_else(|| "—".into()),
            cls = m
                .cls_p75
                .map(|v| format!("{v:.3}"))
                .unwrap_or_else(|| "—".into()),
            inp = m
                .inp_p75
                .map(|v| format!("{:.0}ms", v))
                .unwrap_or_else(|| "—".into()),
            fcp = m
                .fcp_p75
                .map(|v| format!("{:.0}ms", v))
                .unwrap_or_else(|| "—".into()),
            ttfb = m
                .ttfb_p75
                .map(|v| format!("{:.0}ms", v))
                .unwrap_or_else(|| "—".into()),
        ));
    }

    format!(
        r#"<h2>Core Web Vitals</h2>
<table>
<thead><tr><th scope="col">URL</th><th scope="col" style="text-align:right">LCP (p75)</th><th scope="col" style="text-align:right">CLS (p75)</th><th scope="col" style="text-align:right">INP (p75)</th><th scope="col" style="text-align:right">FCP (p75)</th><th scope="col" style="text-align:right">TTFB (p75)</th></tr></thead>
<tbody>{rows}</tbody>
</table>"#
    )
}

fn average<'a>(iter: impl Iterator<Item = f64> + 'a) -> Option<f64> {
    let mut sum = 0.0;
    let mut count = 0usize;
    for v in iter {
        sum += v;
        count += 1;
    }
    if count > 0 {
        Some(sum / count as f64)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Issue, IssueCategory, PageData, Severity, Storage};
    use chrono::Utc;
    use url::Url;

    fn seed_data(storage: &Storage, crawl_id: &str) {
        let pages = vec![
            PageData {
                id: "p1".into(),
                url: Url::parse("https://example.com/").unwrap(),
                final_url: Url::parse("https://example.com/").unwrap(),
                status_code: 200,
                title: Some("Home".into()),
                description: Some("Home page".into()),
                canonical_url: Some(Url::parse("https://example.com/").unwrap()),
                word_count: Some(1200),
                load_time_ms: Some(150),
                body_size: Some(4096),
                fetched_at: Utc::now(),
                links: vec![Url::parse("https://example.com/about").unwrap()],
                tenant_id: None,
                etag: None,
                last_modified: None,
            },
            PageData {
                id: "p2".into(),
                url: Url::parse("https://example.com/about").unwrap(),
                final_url: Url::parse("https://example.com/about").unwrap(),
                status_code: 404,
                title: None,
                description: None,
                canonical_url: None,
                word_count: Some(50),
                load_time_ms: Some(80),
                body_size: Some(512),
                fetched_at: Utc::now(),
                links: vec![],
                tenant_id: None,
                etag: None,
                last_modified: None,
            },
            PageData {
                id: "p3".into(),
                url: Url::parse("https://example.com/blog").unwrap(),
                final_url: Url::parse("https://example.com/blog").unwrap(),
                status_code: 200,
                title: Some("Blog".into()),
                description: Some("Blog listing".into()),
                canonical_url: None,
                word_count: Some(3000),
                load_time_ms: Some(300),
                body_size: Some(8192),
                fetched_at: Utc::now(),
                links: vec![
                    Url::parse("https://example.com/").unwrap(),
                    Url::parse("https://external.com/other").unwrap(),
                ],
                tenant_id: None,
                etag: None,
                last_modified: None,
            },
        ];
        storage.insert_pages(crawl_id, &pages).unwrap();

        let issues = vec![
            Issue {
                id: "i1".into(),
                page_id: "p1".into(),
                category: IssueCategory::Seo,
                severity: Severity::Error,
                code: "SEO001".into(),
                title: "Missing meta description".into(),
                description: "Page has no meta description".into(),
                element: Some("meta[name=description]".into()),
                recommendation: "Add a meta description".into(),
                tenant_id: None,
            },
            Issue {
                id: "i2".into(),
                page_id: "p2".into(),
                category: IssueCategory::Http,
                severity: Severity::Critical,
                code: "HTTP001".into(),
                title: "404 Not Found".into(),
                description: "Page returns 404".into(),
                element: None,
                recommendation: "Fix or remove the page".into(),
                tenant_id: None,
            },
            Issue {
                id: "i3".into(),
                page_id: "p1".into(),
                category: IssueCategory::Images,
                severity: Severity::Warning,
                code: "IMG001".into(),
                title: "Image missing alt text".into(),
                description: "An image has no alt attribute".into(),
                element: Some("img.logo".into()),
                recommendation: "Add descriptive alt text".into(),
                tenant_id: None,
            },
            Issue {
                id: "i4".into(),
                page_id: "p3".into(),
                category: IssueCategory::Seo,
                severity: Severity::Warning,
                code: "SEO002".into(),
                title: "No canonical tag".into(),
                description: "Page is missing a canonical URL".into(),
                element: None,
                recommendation: "Add a canonical link tag".into(),
                tenant_id: None,
            },
            Issue {
                id: "i5".into(),
                page_id: "p3".into(),
                category: IssueCategory::Performance,
                severity: Severity::Info,
                code: "PERF001".into(),
                title: "Slow load time".into(),
                description: "Page took over 200ms to load".into(),
                element: None,
                recommendation: "Optimize page load speed".into(),
                tenant_id: None,
            },
        ];
        storage.insert_issues(&issues).unwrap();
        storage.finish_crawl(crawl_id, 3, 5).unwrap();
    }

    // --- CSV tests ---

    #[test]
    fn test_csv_all_columns() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let conn = &*storage.conn();
        let selector = CsvColumnSelector::all();
        let csv_bytes = export_csv(conn, &crawl_id, &selector).unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        assert!(csv_str.contains("url,status_code,title"));
        assert!(csv_str.contains("https://example.com/"));
        assert!(csv_str.contains("https://example.com/about"));
        assert!(csv_str.contains("https://example.com/blog"));
    }

    #[test]
    fn test_csv_subset_columns() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let conn = &*storage.conn();
        let selector = CsvColumnSelector {
            url: true,
            status_code: true,
            issue_count: true,
            ..Default::default()
        };
        let csv_bytes = export_csv(conn, &crawl_id, &selector).unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        let lines: Vec<&str> = csv_str.lines().collect();
        assert!(lines[0].contains("url"));
        assert!(lines[0].contains("status_code"));
        assert!(lines[0].contains("issue_count"));
        assert!(!lines[0].contains("title"));
    }

    #[test]
    fn test_csv_nested_json_escaping() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let conn = &*storage.conn();
        let selector = CsvColumnSelector {
            issues_json: true,
            ..Default::default()
        };
        let csv_bytes = export_csv(conn, &crawl_id, &selector).unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        // p1 has 2 issues, p3 has 2 issues, p2 has 1 issue
        assert!(csv_str.contains("Missing meta description"));
    }

    #[test]
    fn test_csv_empty_crawl() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let conn = &*storage.conn();
        let selector = CsvColumnSelector::all();
        let csv_bytes = export_csv(conn, &crawl_id, &selector).unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        let lines: Vec<&str> = csv_str.lines().collect();
        assert_eq!(lines.len(), 1); // header only
    }

    // --- JSON tests ---

    #[test]
    fn test_json_pretty() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let json_str = export_json(&storage, &crawl_id, true).unwrap();

        assert!(json_str.contains("schema_version"));
        assert!(json_str.contains(JSON_SCHEMA_VERSION));
        assert!(json_str.contains("https://example.com/"));
        // Pretty-printed has newlines
        assert!(json_str.contains('\n'));
    }

    #[test]
    fn test_json_compact() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let json_str = export_json(&storage, &crawl_id, false).unwrap();

        // Compact has no extra whitespace
        assert!(!json_str.contains('\n'));
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["schema_version"], JSON_SCHEMA_VERSION);
        assert_eq!(v["crawl"]["pages_crawled"], 3);
    }

    #[test]
    fn test_json_page_issues_and_links() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let json_str = export_json(&storage, &crawl_id, true).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let pages = v["pages"].as_array().unwrap();
        let home = pages
            .iter()
            .find(|p| p["url"] == "https://example.com/")
            .unwrap();
        assert_eq!(home["issues"].as_array().unwrap().len(), 2);
        assert_eq!(home["links"].as_array().unwrap().len(), 1);
    }

    // --- Markdown tests ---

    #[test]
    fn test_markdown_structure() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let md = export_markdown(&storage, &crawl_id).unwrap();

        assert!(md.contains("# Crawl Report"));
        assert!(md.contains("## Summary"));
        assert!(md.contains("## Issues by Severity"));
        assert!(md.contains("## Issues by Category"));
        assert!(md.contains("## Top Issues"));
        assert!(md.contains("3")); // total pages
        assert!(md.contains("5")); // total issues
    }

    #[test]
    fn test_markdown_empty_crawl() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let md = export_markdown(&storage, &crawl_id).unwrap();

        assert!(md.contains("| Pages crawled | 0 |"));
        assert!(md.contains("| Total issues | 0 |"));
    }

    // --- HTML tests ---

    #[test]
    fn test_html_structure() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let html = export_html(&storage, &crawl_id).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Crawl Report"));
        assert!(html.contains("status-error"));
        assert!(html.contains("https://example.com/"));
        assert!(html.contains("filterTable()"));
    }

    #[test]
    fn test_html_empty_crawl() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let html = export_html(&storage, &crawl_id).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("0")); // zero counts
    }

    // --- Helper access tests ---

    #[test]
    fn test_column_selector_headers() {
        let sel = CsvColumnSelector::all();
        assert_eq!(sel.headers().len(), 12);

        let sel = CsvColumnSelector {
            url: true,
            status_code: true,
            ..Default::default()
        };
        assert_eq!(sel.headers(), vec!["url", "status_code"]);
    }
}
