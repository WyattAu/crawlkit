use crate::storage_trait::StorageBackend;

use super::helpers::{escape_markdown, read_crawl_meta, read_top_issues, ExportError};

/// Generate a Markdown summary report.
pub fn export_markdown(
    storage: &dyn StorageBackend,
    crawl_id: &str,
) -> Result<String, ExportError> {
    let stats = storage.get_stats(crawl_id)?;
    let meta = read_crawl_meta(storage, crawl_id)?;
    let top_issues = read_top_issues(storage, crawl_id, 10)?;

    let mut md = String::new();

    md.push_str(&format!(
        "# Crawl Report — `{}`\n\n",
        escape_markdown(&meta.target_url)
    ));
    md.push_str(&format!(
        "**Crawl ID:** `{}`  \n",
        escape_markdown(crawl_id)
    ));
    if let Some(ref start) = meta.start_time {
        md.push_str(&format!("**Started:** {}  \n", escape_markdown(start)));
    }
    if let Some(ref end) = meta.end_time {
        md.push_str(&format!("**Finished:** {}  \n", escape_markdown(end)));
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
    // Count desc, then key asc as a deterministic tie-breaker.
    cats.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (cat, count) in &cats {
        md.push_str(&format!("| {} | {count} |\n", escape_markdown(cat)));
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
                escape_markdown(&ti.severity),
                escape_markdown(&ti.code),
                escape_markdown(&ti.title),
                ti.affected_pages
            ));
        }
        md.push('\n');
    }

    Ok(md)
}
