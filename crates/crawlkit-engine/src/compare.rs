use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;
use thiserror::Error;

use crate::CrawlError;

/// Errors specific to comparison operations.
#[derive(Debug, Error)]
pub enum CompareError {
    /// SQLite error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<CompareError> for CrawlError {
    fn from(e: CompareError) -> Self {
        CrawlError::Internal(e.to_string())
    }
}

/// A single page entry for comparison (keyed by URL).
#[derive(Debug, Clone, Serialize)]
struct ComparePage {
    url: String,
    status_code: u16,
    title: Option<String>,
    word_count: Option<usize>,
    body_size: Option<usize>,
}

/// The type of change detected for a page.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ChangeKind {
    /// Page exists only in the baseline.
    Removed,
    /// Page exists only in the target.
    Added,
    /// HTTP status code changed.
    StatusChanged { from: u16, to: u16 },
    /// Page title changed.
    TitleChanged {
        from: Option<String>,
        to: Option<String>,
    },
    /// Word count changed significantly (>10% or >100 words).
    ContentChanged {
        from: Option<usize>,
        to: Option<usize>,
    },
    /// Body size changed significantly (>10%).
    SizeChanged {
        from: Option<usize>,
        to: Option<usize>,
    },
}

/// A single diff entry.
#[derive(Debug, Clone, Serialize)]
pub struct DiffEntry {
    /// The page URL.
    pub url: String,
    /// What changed.
    pub change: ChangeKind,
}

/// A Core Web Vitals regression or improvement between crawls.
#[derive(Debug, Clone, Serialize)]
pub struct CwvChange {
    /// The page URL.
    pub url: String,
    /// Which metric changed (e.g. "LCP", "CLS", "INP", "FCP", "TTFB").
    pub metric_name: String,
    /// Previous value (p75).
    pub old_value: Option<f64>,
    /// New value (p75).
    pub new_value: Option<f64>,
    /// True if the metric worsened by >10%.
    pub regression: bool,
}

/// Result of comparing two crawl databases.
#[derive(Debug, Clone, Serialize)]
pub struct CrawlDiff {
    /// Total pages in the baseline crawl.
    pub baseline_pages: usize,
    /// Total pages in the target crawl.
    pub target_pages: usize,
    /// Pages added in the target.
    pub added: Vec<DiffEntry>,
    /// Pages removed from the baseline.
    pub removed: Vec<DiffEntry>,
    /// Pages with status changes.
    pub status_changes: Vec<DiffEntry>,
    /// Pages with title changes.
    pub title_changes: Vec<DiffEntry>,
    /// Pages with content changes.
    pub content_changes: Vec<DiffEntry>,
    /// Pages with size changes.
    pub size_changes: Vec<DiffEntry>,
    /// Core Web Vitals regressions/improvements.
    pub cwv_changes: Vec<CwvChange>,
}

impl CrawlDiff {
    /// Total number of changes detected.
    pub fn total_changes(&self) -> usize {
        self.added.len()
            + self.removed.len()
            + self.status_changes.len()
            + self.title_changes.len()
            + self.content_changes.len()
            + self.size_changes.len()
            + self.cwv_changes.len()
    }

    /// True if no changes were detected.
    pub fn is_empty(&self) -> bool {
        self.total_changes() == 0
    }
}

/// Compare two SQLite crawl databases by their latest crawl.
///
/// `baseline_path` and `target_path` are paths to crawlkit SQLite databases.
/// The function finds the most recent crawl in each database and compares
/// their pages by URL.
pub fn compare_crawls(baseline_path: &Path, target_path: &Path) -> Result<CrawlDiff, CompareError> {
    let baseline_conn = Connection::open(baseline_path)?;
    let target_conn = Connection::open(target_path)?;

    let baseline_crawl = latest_crawl_id(&baseline_conn)?;
    let target_crawl = latest_crawl_id(&target_conn)?;

    compare_crawl_ids(&baseline_conn, &baseline_crawl, &target_conn, &target_crawl)
}

/// Compare two specific crawl IDs within the same or different databases.
pub fn compare_crawl_ids(
    baseline_conn: &Connection,
    baseline_crawl_id: &str,
    target_conn: &Connection,
    target_crawl_id: &str,
) -> Result<CrawlDiff, CompareError> {
    let baseline_pages = load_pages(baseline_conn, baseline_crawl_id)?;
    let target_pages = load_pages(target_conn, target_crawl_id)?;

    let baseline_map: std::collections::HashMap<&str, &ComparePage> =
        baseline_pages.iter().map(|p| (p.url.as_str(), p)).collect();
    let target_map: std::collections::HashMap<&str, &ComparePage> =
        target_pages.iter().map(|p| (p.url.as_str(), p)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut status_changes = Vec::new();
    let mut title_changes = Vec::new();
    let mut content_changes = Vec::new();
    let mut size_changes = Vec::new();

    // Pages in target but not baseline → added
    for url in target_map.keys() {
        if !baseline_map.contains_key(url) {
            added.push(DiffEntry {
                url: url.to_string(),
                change: ChangeKind::Added,
            });
        }
    }

    // Pages in baseline but not target → removed
    for url in baseline_map.keys() {
        if !target_map.contains_key(url) {
            removed.push(DiffEntry {
                url: url.to_string(),
                change: ChangeKind::Removed,
            });
        }
    }

    // Pages in both → check for changes
    for (url, &baseline_page) in &baseline_map {
        if let Some(&target_page) = target_map.get(url) {
            if baseline_page.status_code != target_page.status_code {
                status_changes.push(DiffEntry {
                    url: url.to_string(),
                    change: ChangeKind::StatusChanged {
                        from: baseline_page.status_code,
                        to: target_page.status_code,
                    },
                });
            }

            if baseline_page.title != target_page.title {
                title_changes.push(DiffEntry {
                    url: url.to_string(),
                    change: ChangeKind::TitleChanged {
                        from: baseline_page.title.clone(),
                        to: target_page.title.clone(),
                    },
                });
            }

            if significant_content_change(&baseline_page.word_count, &target_page.word_count) {
                content_changes.push(DiffEntry {
                    url: url.to_string(),
                    change: ChangeKind::ContentChanged {
                        from: baseline_page.word_count,
                        to: target_page.word_count,
                    },
                });
            }

            if significant_size_change(&baseline_page.body_size, &target_page.body_size) {
                size_changes.push(DiffEntry {
                    url: url.to_string(),
                    change: ChangeKind::SizeChanged {
                        from: baseline_page.body_size,
                        to: target_page.body_size,
                    },
                });
            }
        }
    }

    // Compare CrUX metrics for URLs present in both crawls
    let baseline_crux = load_crux(baseline_conn, baseline_crawl_id)?;
    let target_crux = load_crux(target_conn, target_crawl_id)?;

    let baseline_crux_map: std::collections::HashMap<&str, &CompareCrux> =
        baseline_crux.iter().map(|c| (c.url.as_str(), c)).collect();
    let target_crux_map: std::collections::HashMap<&str, &CompareCrux> =
        target_crux.iter().map(|c| (c.url.as_str(), c)).collect();

    let mut cwv_changes = Vec::new();

    for (url, &b) in &baseline_crux_map {
        if let Some(&t) = target_crux_map.get(url) {
            check_cwv_regression(url, "LCP", b.lcp_p75, t.lcp_p75, &mut cwv_changes);
            check_cwv_regression(url, "CLS", b.cls_p75, t.cls_p75, &mut cwv_changes);
            check_cwv_regression(url, "INP", b.inp_p75, t.inp_p75, &mut cwv_changes);
            check_cwv_regression(url, "FCP", b.fcp_p75, t.fcp_p75, &mut cwv_changes);
            check_cwv_regression(url, "TTFB", b.ttfb_p75, t.ttfb_p75, &mut cwv_changes);
        }
    }

    Ok(CrawlDiff {
        baseline_pages: baseline_pages.len(),
        target_pages: target_pages.len(),
        added,
        removed,
        status_changes,
        title_changes,
        content_changes,
        size_changes,
        cwv_changes,
    })
}

fn check_cwv_regression(
    url: &str,
    metric_name: &str,
    old: Option<f64>,
    new: Option<f64>,
    out: &mut Vec<CwvChange>,
) {
    if let (Some(o), Some(n)) = (old, new) {
        let pct = if o.abs() > f64::EPSILON {
            (n - o) / o.abs()
        } else {
            return;
        };
        if pct > 0.10 {
            out.push(CwvChange {
                url: url.to_string(),
                metric_name: metric_name.to_string(),
                old_value: Some(o),
                new_value: Some(n),
                regression: true,
            });
        }
    }
}

/// Write a diff to JSON.
pub fn diff_to_json(diff: &CrawlDiff, pretty: bool) -> Result<String, serde_json::Error> {
    if pretty {
        serde_json::to_string_pretty(diff)
    } else {
        serde_json::to_string(diff)
    }
}

/// Write a diff to Markdown.
pub fn diff_to_markdown(diff: &CrawlDiff) -> String {
    let mut md = String::new();

    md.push_str("# Crawl Comparison\n\n");
    md.push_str(&format!(
        "Baseline: **{}** pages  \nTarget: **{}** pages\n\n",
        diff.baseline_pages, diff.target_pages
    ));
    md.push_str(&format!("**Total changes: {}**\n\n", diff.total_changes()));

    if !diff.added.is_empty() {
        md.push_str(&format!("## Added Pages ({})\n\n", diff.added.len()));
        for e in &diff.added {
            md.push_str(&format!("- `{}`\n", e.url));
        }
        md.push('\n');
    }

    if !diff.removed.is_empty() {
        md.push_str(&format!("## Removed Pages ({})\n\n", diff.removed.len()));
        for e in &diff.removed {
            md.push_str(&format!("- `{}`\n", e.url));
        }
        md.push('\n');
    }

    if !diff.status_changes.is_empty() {
        md.push_str(&format!(
            "## Status Changes ({})\n\n",
            diff.status_changes.len()
        ));
        md.push_str("| URL | From | To |\n|---|---|---|\n");
        for e in &diff.status_changes {
            if let ChangeKind::StatusChanged { from, to } = &e.change {
                md.push_str(&format!("| {} | {from} | {to} |\n", e.url));
            }
        }
        md.push('\n');
    }

    if !diff.title_changes.is_empty() {
        md.push_str(&format!(
            "## Title Changes ({})\n\n",
            diff.title_changes.len()
        ));
        md.push_str("| URL | Old Title | New Title |\n|---|---|---|\n");
        for e in &diff.title_changes {
            if let ChangeKind::TitleChanged { from, to } = &e.change {
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    e.url,
                    from.as_deref().unwrap_or("—"),
                    to.as_deref().unwrap_or("—")
                ));
            }
        }
        md.push('\n');
    }

    if !diff.content_changes.is_empty() {
        md.push_str(&format!(
            "## Content Changes ({})\n\n",
            diff.content_changes.len()
        ));
        for e in &diff.content_changes {
            if let ChangeKind::ContentChanged { from, to } = &e.change {
                md.push_str(&format!(
                    "- `{}`: {} → {} words\n",
                    e.url,
                    from.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
                    to.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
                ));
            }
        }
        md.push('\n');
    }

    if !diff.size_changes.is_empty() {
        md.push_str(&format!(
            "## Size Changes ({})\n\n",
            diff.size_changes.len()
        ));
        for e in &diff.size_changes {
            if let ChangeKind::SizeChanged { from, to } = &e.change {
                md.push_str(&format!(
                    "- `{}`: {} → {} bytes\n",
                    e.url,
                    from.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
                    to.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
                ));
            }
        }
        md.push('\n');
    }

    if !diff.cwv_changes.is_empty() {
        md.push_str(&format!(
            "## Core Web Vitals Changes ({})\n\n",
            diff.cwv_changes.len()
        ));
        md.push_str(
            "| URL | Metric | Old (p75) | New (p75) | Regression |\n|---|---|---|---|---|\n",
        );
        for c in &diff.cwv_changes {
            let old_str = c
                .old_value
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "—".into());
            let new_str = c
                .new_value
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "—".into());
            let regression = if c.regression { "YES" } else { "no" };
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                c.url, c.metric_name, old_str, new_str, regression,
            ));
        }
        md.push('\n');
    }

    if diff.is_empty() {
        md.push_str("No changes detected.\n");
    }

    md
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn latest_crawl_id(conn: &Connection) -> Result<String, CompareError> {
    conn.query_row(
        "SELECT id FROM crawls ORDER BY start_time DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .map_err(CompareError::from)
}

fn load_pages(conn: &Connection, crawl_id: &str) -> Result<Vec<ComparePage>, CompareError> {
    let mut stmt = conn.prepare(
        "SELECT url, status_code, title, word_count, body_size FROM pages WHERE crawl_id = ?1",
    )?;
    let pages = stmt
        .query_map([crawl_id], |row| {
            Ok(ComparePage {
                url: row.get(0)?,
                status_code: row.get(1)?,
                title: row.get(2)?,
                word_count: row.get::<_, Option<i64>>(3)?.map(|v| v as usize),
                body_size: row.get::<_, Option<i64>>(4)?.map(|v| v as usize),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(pages)
}

#[derive(Debug, Clone)]
struct CompareCrux {
    url: String,
    lcp_p75: Option<f64>,
    inp_p75: Option<f64>,
    cls_p75: Option<f64>,
    fcp_p75: Option<f64>,
    ttfb_p75: Option<f64>,
}

fn load_crux(conn: &Connection, crawl_id: &str) -> Result<Vec<CompareCrux>, CompareError> {
    let mut stmt = conn.prepare(
        "SELECT cm.url, cm.lcp_p75, cm.inp_p75, cm.cls_p75, cm.fcp_p75, cm.ttfb_p75
         FROM crux_metrics cm
         JOIN pages p ON cm.page_id = p.id
         WHERE p.crawl_id = ?1",
    )?;
    let rows = stmt
        .query_map([crawl_id], |row| {
            Ok(CompareCrux {
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

/// A content change is significant if word count differs by >10% or >100 words.
fn significant_content_change(from: &Option<usize>, to: &Option<usize>) -> bool {
    match (from, to) {
        (None, Some(_)) | (Some(_), None) => true,
        (Some(f), Some(t)) => {
            let diff = (*f as isize - *t as isize).unsigned_abs();
            let threshold_pct = (*f as f64 * 0.10) as usize;
            let threshold_abs = 100;
            diff > threshold_pct || diff > threshold_abs
        }
        (None, None) => false,
    }
}

/// A size change is significant if it differs by >10%.
fn significant_size_change(from: &Option<usize>, to: &Option<usize>) -> bool {
    match (from, to) {
        (None, Some(_)) | (Some(_), None) => true,
        (Some(f), Some(t)) => {
            let diff = (*f as isize - *t as isize).unsigned_abs();
            diff > (*f as f64 * 0.10) as usize
        }
        (None, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{PageData, Storage};
    use chrono::Utc;
    use url::Url;

    fn make_page(
        id: &str,
        url: &str,
        status: u16,
        title: &str,
        words: usize,
        size: usize,
    ) -> PageData {
        PageData {
            id: id.into(),
            url: Url::parse(url).unwrap(),
            final_url: Url::parse(url).unwrap(),
            status_code: status,
            title: Some(title.into()),
            description: None,
            canonical_url: None,
            word_count: Some(words),
            load_time_ms: Some(200),
            body_size: Some(size),
            fetched_at: Utc::now(),
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
        }
    }

    fn create_baseline_db(path: &Path) {
        let storage = Storage::new(path).unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        let pages = vec![
            make_page("p1", "https://example.com/", 200, "Home", 1000, 4096),
            make_page("p2", "https://example.com/about", 200, "About", 500, 2048),
            make_page(
                "p3",
                "https://example.com/contact",
                200,
                "Contact",
                300,
                1024,
            ),
        ];
        storage.insert_pages(&crawl_id, &pages).unwrap();
        storage.finish_crawl(&crawl_id, 3, 0).unwrap();
    }

    fn create_target_db(path: &Path) {
        let storage = Storage::new(path).unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        let pages = vec![
            make_page("p1", "https://example.com/", 200, "Home", 1000, 4096), // unchanged
            make_page(
                "p2",
                "https://example.com/about",
                301,
                "About v2",
                600,
                2500,
            ), // status + title + content changed
            // p3 removed
            make_page("p4", "https://example.com/blog", 200, "Blog", 800, 3072), // new page
        ];
        storage.insert_pages(&crawl_id, &pages).unwrap();
        storage.finish_crawl(&crawl_id, 3, 0).unwrap();
    }

    #[test]
    fn test_compare_detects_changes() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_path = dir.path().join("baseline.db");
        let target_path = dir.path().join("target.db");

        create_baseline_db(&baseline_path);
        create_target_db(&target_path);

        let diff = compare_crawls(&baseline_path, &target_path).unwrap();

        assert_eq!(diff.baseline_pages, 3);
        assert_eq!(diff.target_pages, 3);
        assert!(!diff.is_empty());

        // p4 is new
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].url, "https://example.com/blog");

        // p3 was removed
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].url, "https://example.com/contact");

        // p2 status changed 200 → 301
        assert_eq!(diff.status_changes.len(), 1);
        assert_eq!(diff.status_changes[0].url, "https://example.com/about");

        // p2 title changed
        assert_eq!(diff.title_changes.len(), 1);
        assert_eq!(diff.title_changes[0].url, "https://example.com/about");

        // p2 content changed (500 → 600, +20% > threshold)
        assert_eq!(diff.content_changes.len(), 1);
        assert_eq!(diff.content_changes[0].url, "https://example.com/about");
    }

    #[test]
    fn test_compare_no_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("a.db");
        let path_b = dir.path().join("b.db");

        // Create identical databases
        let storage_a = Storage::new(&path_a).unwrap();
        let crawl_id = storage_a.start_crawl("https://example.com", None).unwrap();
        let pages = vec![make_page(
            "p1",
            "https://example.com/",
            200,
            "Home",
            1000,
            4096,
        )];
        storage_a.insert_pages(&crawl_id, &pages).unwrap();
        storage_a.finish_crawl(&crawl_id, 1, 0).unwrap();

        let storage_b = Storage::new(&path_b).unwrap();
        let crawl_id = storage_b.start_crawl("https://example.com", None).unwrap();
        storage_b.insert_pages(&crawl_id, &pages).unwrap();
        storage_b.finish_crawl(&crawl_id, 1, 0).unwrap();

        let diff = compare_crawls(&path_a, &path_b).unwrap();
        assert!(diff.is_empty());
        assert_eq!(diff.total_changes(), 0);
    }

    #[test]
    fn test_diff_to_json() {
        let diff = CrawlDiff {
            baseline_pages: 10,
            target_pages: 12,
            added: vec![DiffEntry {
                url: "https://example.com/new".into(),
                change: ChangeKind::Added,
            }],
            removed: vec![],
            status_changes: vec![],
            title_changes: vec![],
            content_changes: vec![],
            size_changes: vec![],
            cwv_changes: vec![],
        };

        let json = diff_to_json(&diff, true).unwrap();
        assert!(json.contains("baseline_pages"));
        assert!(json.contains("Added"));
        assert!(json.contains("https://example.com/new"));

        let compact = diff_to_json(&diff, false).unwrap();
        assert!(!compact.contains('\n'));
    }

    #[test]
    fn test_diff_to_markdown() {
        let diff = CrawlDiff {
            baseline_pages: 5,
            target_pages: 7,
            added: vec![
                DiffEntry {
                    url: "https://example.com/new1".into(),
                    change: ChangeKind::Added,
                },
                DiffEntry {
                    url: "https://example.com/new2".into(),
                    change: ChangeKind::Added,
                },
            ],
            removed: vec![DiffEntry {
                url: "https://example.com/old".into(),
                change: ChangeKind::Removed,
            }],
            status_changes: vec![DiffEntry {
                url: "https://example.com/changed".into(),
                change: ChangeKind::StatusChanged { from: 200, to: 404 },
            }],
            title_changes: vec![],
            content_changes: vec![],
            size_changes: vec![],
            cwv_changes: vec![],
        };

        let md = diff_to_markdown(&diff);
        assert!(md.contains("# Crawl Comparison"));
        assert!(md.contains("Baseline: **5** pages"));
        assert!(md.contains("Target: **7** pages"));
        assert!(md.contains("## Added Pages (2)"));
        assert!(md.contains("## Removed Pages (1)"));
        assert!(md.contains("## Status Changes (1)"));
        assert!(md.contains("200"));
        assert!(md.contains("404"));
    }

    #[test]
    fn test_diff_empty_markdown() {
        let diff = CrawlDiff {
            baseline_pages: 3,
            target_pages: 3,
            added: vec![],
            removed: vec![],
            status_changes: vec![],
            title_changes: vec![],
            content_changes: vec![],
            size_changes: vec![],
            cwv_changes: vec![],
        };

        let md = diff_to_markdown(&diff);
        assert!(md.contains("No changes detected"));
    }

    #[test]
    fn test_significant_content_change() {
        assert!(!significant_content_change(&Some(1000), &Some(1050))); // +5% < 100
        assert!(significant_content_change(&Some(1000), &Some(1200))); // +20%
        assert!(significant_content_change(&Some(100), &Some(50))); // -50%
        assert!(significant_content_change(&None, &Some(100)));
        assert!(significant_content_change(&Some(100), &None));
        assert!(!significant_content_change(&None, &None));
    }

    #[test]
    fn test_significant_size_change() {
        assert!(!significant_size_change(&Some(1000), &Some(1050))); // +5%
        assert!(significant_size_change(&Some(1000), &Some(1200))); // +20%
        assert!(significant_size_change(&None, &Some(100)));
        assert!(!significant_size_change(&None, &None));
    }
}
