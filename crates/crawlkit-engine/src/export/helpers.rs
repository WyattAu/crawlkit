use thiserror::Error;

use crate::storage::{IssueFilter, StorageError};
use crate::storage_trait::StorageBackend;
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

    /// Storage error.
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

impl From<ExportError> for CrawlError {
    fn from(e: ExportError) -> Self {
        CrawlError::Internal(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

pub(super) fn severity_order() -> &'static [&'static str] {
    &["critical", "error", "warning", "info"]
}

/// HTML-escape a string for safe interpolation into the report template.
///
/// Escapes the five characters that can alter HTML structure or attribute
/// boundaries: `&`, `<`, `>`, `"`, and `'`.
pub(super) fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Markdown-escape a string for safe interpolation into the Markdown report.
///
/// Escapes characters that could alter Markdown structure — link brackets,
/// code spans, and table pipes — everywhere, plus a leading `#` or `-` that
/// would otherwise render as a heading or list marker.
pub(super) fn escape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.char_indices() {
        match c {
            '[' | ']' | '`' | '|' => {
                out.push('\\');
                out.push(c);
            }
            '#' | '-' if i == 0 => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Database row types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub(super) struct PageRow {
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
    pub has_structured_data: Option<bool>,
    pub schema_types: Option<String>,
    pub viewport_ok: Option<bool>,
    pub has_csp: Option<bool>,
    pub has_hsts: Option<bool>,
    pub images_total: Option<usize>,
    pub images_missing_alt: Option<usize>,
    pub h1_count: Option<usize>,
    pub heading_count: Option<usize>,
    pub extractions: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct IssueRow {
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

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct TopIssue {
    pub severity: String,
    pub code: String,
    pub title: String,
    pub affected_pages: usize,
}

#[derive(Debug, Clone)]
pub(super) struct CruxRow {
    pub url: String,
    pub lcp_p75: Option<f64>,
    pub inp_p75: Option<f64>,
    pub cls_p75: Option<f64>,
    pub fcp_p75: Option<f64>,
    pub ttfb_p75: Option<f64>,
}

// ---------------------------------------------------------------------------
// Storage-backed read functions
// ---------------------------------------------------------------------------

pub(super) fn read_crawl_meta(
    storage: &dyn StorageBackend,
    crawl_id: &str,
) -> Result<super::json::JsonCrawlMeta, ExportError> {
    let meta = storage.get_crawl_meta(crawl_id)?;
    Ok(super::json::JsonCrawlMeta {
        id: meta.id,
        target_url: meta.target_url,
        start_time: meta.start_time,
        end_time: meta.end_time,
        pages_crawled: meta.pages_crawled,
        total_issues: meta.total_issues,
    })
}

/// Read all pages for a crawl, canonically ordered by URL string so that
/// identical storage contents always export byte-identical output.
pub(super) fn read_pages(
    storage: &dyn StorageBackend,
    crawl_id: &str,
) -> Result<Vec<PageRow>, ExportError> {
    let pages = storage.get_pages(crawl_id, usize::MAX)?;
    let mut rows: Vec<PageRow> = pages
        .into_iter()
        .map(|p| PageRow {
            id: p.id,
            url: p.url.to_string(),
            final_url: p.final_url.to_string(),
            status_code: p.status_code,
            title: p.title,
            description: p.description,
            canonical: p.canonical_url.map(|u| u.to_string()),
            word_count: p.word_count,
            load_time_ms: p.load_time_ms,
            body_size: p.body_size,
            fetched_at: p.fetched_at.to_rfc3339(),
            has_structured_data: p.has_structured_data,
            schema_types: p.schema_types,
            viewport_ok: p.viewport_ok,
            has_csp: p.has_csp,
            has_hsts: p.has_hsts,
            images_total: p.images_total,
            images_missing_alt: p.images_missing_alt,
            h1_count: p.h1_count,
            heading_count: p.heading_count,
            extractions: p.extractions,
        })
        .collect();
    rows.sort_by(|a, b| a.url.cmp(&b.url));
    Ok(rows)
}

/// Load all issues for a crawl in a single query, grouped by page_id.
///
/// This replaces per-page N+1 queries with a single bulk fetch,
/// then groups the results in memory by page_id for O(1) lookup.
pub(super) fn read_issues_grouped_by_page(
    storage: &dyn StorageBackend,
    crawl_id: &str,
) -> Result<std::collections::HashMap<String, Vec<IssueRow>>, ExportError> {
    let issues = storage.get_issues(crawl_id, &IssueFilter::default())?;
    let rows: Vec<IssueRow> = issues
        .into_iter()
        .map(|i| IssueRow {
            id: i.id,
            page_id: i.page_id,
            #[allow(clippy::redundant_clone)]
            category: i.category.as_str().to_string(),
            severity: i.severity.as_str().to_string(),
            code: i.code,
            title: i.title,
            description: i.description,
            element: i.element,
            recommendation: i.recommendation,
        })
        .collect();

    let mut grouped: std::collections::HashMap<String, Vec<IssueRow>> =
        std::collections::HashMap::new();
    for row in rows {
        grouped.entry(row.page_id.clone()).or_default().push(row);
    }
    // Canonical ordering per page: findings sorted by (code, element) so
    // identical storage contents always serialize byte-identically.
    for issues in grouped.values_mut() {
        issues.sort_by(|a, b| a.code.cmp(&b.code).then_with(|| a.element.cmp(&b.element)));
    }
    Ok(grouped)
}

/// Load all links for a crawl in a single query, grouped by page_id.
///
/// Replaces per-page N+1 queries with a single bulk fetch.
/// Uses the page→URL mapping from `get_pages` and the source-URL→targets
/// mapping from `get_links_for_crawl` to build the page_id→targets view.
pub(super) fn read_links_grouped_by_page(
    storage: &dyn StorageBackend,
    crawl_id: &str,
) -> Result<std::collections::HashMap<String, Vec<String>>, ExportError> {
    let pages = storage.get_pages(crawl_id, usize::MAX)?;
    let links_by_source = storage.get_links_for_crawl(crawl_id)?;

    // Build url→page_id map
    let url_to_page_id: std::collections::HashMap<String, String> = pages
        .iter()
        .map(|p| (p.url.to_string(), p.id.clone()))
        .collect();

    // Build source_url→Vec<target_url> from the trait's Vec<(String, Vec<String>)>
    let links_map: std::collections::HashMap<String, Vec<String>> =
        links_by_source.into_iter().collect();

    let mut grouped: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (source_url, targets) in links_map {
        if let Some(page_id) = url_to_page_id.get(&source_url) {
            grouped.entry(page_id.clone()).or_default().extend(targets);
        }
    }
    // Canonical ordering: sorted link lists for byte-identical exports.
    for links in grouped.values_mut() {
        links.sort();
    }
    Ok(grouped)
}

pub(super) fn read_crux_metrics(
    storage: &dyn StorageBackend,
    crawl_id: &str,
) -> Result<Vec<CruxRow>, ExportError> {
    let metrics = storage.get_crux_metrics_for_crawl(crawl_id)?;
    let mut rows: Vec<CruxRow> = metrics
        .into_iter()
        .map(|m| CruxRow {
            url: m.url,
            lcp_p75: m.lcp_p75,
            inp_p75: m.inp_p75,
            cls_p75: m.cls_p75,
            fcp_p75: m.fcp_p75,
            ttfb_p75: m.ttfb_p75,
        })
        .collect();
    rows.sort_by(|a, b| a.url.cmp(&b.url));
    Ok(rows)
}

pub(super) fn read_top_issues(
    storage: &dyn StorageBackend,
    crawl_id: &str,
    limit: usize,
) -> Result<Vec<TopIssue>, ExportError> {
    let issues = storage.get_top_issues(crawl_id, limit)?;
    Ok(issues
        .into_iter()
        .map(|ti| TopIssue {
            severity: ti.severity,
            code: ti.code,
            title: ti.title,
            affected_pages: ti.affected_pages,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Core Web Vitals helpers
// ---------------------------------------------------------------------------

pub(super) fn cwv_lcp_class(val: f64) -> &'static str {
    if val < 2500.0 {
        "cwv-good"
    } else if val < 4000.0 {
        "cwv-avg"
    } else {
        "cwv-poor"
    }
}

pub(super) fn cwv_cls_class(val: f64) -> &'static str {
    if val < 0.1 {
        "cwv-good"
    } else if val < 0.25 {
        "cwv-avg"
    } else {
        "cwv-poor"
    }
}

pub(super) fn cwv_inp_class(val: f64) -> &'static str {
    if val < 200.0 {
        "cwv-good"
    } else if val < 500.0 {
        "cwv-avg"
    } else {
        "cwv-poor"
    }
}

pub(super) fn cwv_generic_class(_val: f64) -> &'static str {
    "cwv-good"
}

pub(super) fn build_cwv_summary_cards(metrics: &[CruxRow]) -> String {
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

pub(super) fn build_cwv_detail_section(metrics: &[CruxRow]) -> String {
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
            url = escape_html(&m.url),
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

pub(super) fn average<'a>(iter: impl Iterator<Item = f64> + 'a) -> Option<f64> {
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
