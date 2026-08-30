use crate::storage_trait::StorageBackend;

use super::helpers::{
    build_cwv_detail_section, build_cwv_summary_cards, escape_html, read_crawl_meta,
    read_crux_metrics, read_issues_grouped_by_page, read_pages, read_top_issues, severity_order,
    ExportError,
};

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
pub fn export_html(storage: &dyn StorageBackend, crawl_id: &str) -> Result<String, ExportError> {
    let stats = storage.get_stats(crawl_id)?;
    let meta = read_crawl_meta(storage, crawl_id)?;
    let top_issues = read_top_issues(storage, crawl_id, 20)?;
    let pages = read_pages(storage, crawl_id)?;
    let crux_metrics = read_crux_metrics(storage, crawl_id)?;
    let issues_by_page = read_issues_grouped_by_page(storage, crawl_id)?;

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
        let url = escape_html(&page.url);
        rows.push_str(&format!(
            r#"<tr>
  <td><a href="{url}" target="_blank">{url}</a></td>
  <td class="{status_class}">{status}</td>
  <td>{title}</td>
  <td class="num">{wc}</td>
  <td class="num">{lt}</td>
  <td class="num">{ic}</td>
</tr>"#,
            url = url,
            status = page.status_code,
            title = escape_html(page.title.as_deref().unwrap_or("—")),
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
            sev = escape_html(&ti.severity),
            code = escape_html(&ti.code),
            title = escape_html(&ti.title),
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
        target = escape_html(&meta.target_url),
        crawl_id = escape_html(crawl_id),
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
            cats.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            cats.iter()
                .map(|(cat, count)| {
                    format!(
                        "<tr><td>{}</td><td class=\"num\">{count}</td></tr>",
                        escape_html(cat)
                    )
                })
                .collect::<String>()
        },
        issue_rows = issue_rows,
        cwv_detail_section = build_cwv_detail_section(&crux_metrics),
        rows = rows,
    );

    Ok(html)
}
