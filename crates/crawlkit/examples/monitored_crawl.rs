//! Example: Monitoring mode — compare a new crawl against a baseline and
//! raise alerts.
//!
//! Demonstrates the workflow used by `crawlkit crawl --monitor`:
//!
//! 1. Two crawl databases are built (a baseline and a "current" crawl) with
//!    deliberate differences: a new page, a removed page, a changed title,
//!    and a large content drop.
//! 2. `compare::compare_crawls` diffs the two databases by URL.
//! 3. `monitoring::ContinuousMonitor::check` evaluates the delta against an
//!    alert threshold and classifies each change by severity.
//! 4. `monitoring::Alert::from_result` builds a notification-ready alert.
//!
//! Run with: cargo run --example monitored_crawl

use std::path::PathBuf;

use chrono::Utc;
use crawlkit_engine::compare::{compare_crawls, diff_to_markdown};
use crawlkit_engine::monitoring::{Alert, AlertSeverity, ContinuousMonitor, MonitorConfig};
use crawlkit_engine::storage::{PageData, Storage};
use url::Url;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Set up two throwaway crawl databases in a temp directory ---------
    let dir = std::env::temp_dir().join("crawlkit-monitored-crawl-demo");
    std::fs::create_dir_all(&dir)?;

    let baseline_path: PathBuf = dir.join("baseline.db");
    let current_path: PathBuf = dir.join("current.db");
    // Remove leftovers from previous runs so each run starts clean.
    let _ = std::fs::remove_file(&baseline_path);
    let _ = std::fs::remove_file(&current_path);

    println!("Baseline database: {}", baseline_path.display());
    println!("Current database:  {}\n", current_path.display());

    let baseline_pages = vec![
        page(
            "p1",
            "https://example.com/",
            200,
            Some("Home"),
            Some(1_200),
            4_096,
        )?,
        page(
            "p2",
            "https://example.com/about",
            200,
            Some("About"),
            Some(800),
            2_048,
        )?,
        page(
            "p3",
            "https://example.com/pricing",
            200,
            Some("Pricing"),
            Some(600),
            1_500,
        )?,
    ];
    let current_pages = vec![
        page(
            "p1",
            "https://example.com/",
            200,
            Some("Home"),
            Some(1_150),
            4_000,
        )?,
        // Title changed (About -> About Us).
        page(
            "p2",
            "https://example.com/about",
            200,
            Some("About Us"),
            Some(780),
            2_010,
        )?,
        // Removed in current crawl: /pricing is gone.
        // Added in current crawl: /blog is new.
        page(
            "p4",
            "https://example.com/blog",
            200,
            Some("Blog"),
            Some(900),
            3_000,
        )?,
        // Content dropped by 60% relative to baseline /products (1000 -> 400 words).
        page(
            "p5",
            "https://example.com/products",
            200,
            Some("Products"),
            Some(400),
            1_200,
        )?,
    ];

    write_crawl(&baseline_path, "https://example.com", &baseline_pages)?;
    write_crawl(&current_path, "https://example.com", &current_pages)?;

    // --- Step 1: diff the two crawls --------------------------------------
    println!("[1/3] Comparing crawls...");
    let diff = compare_crawls(&baseline_path, &current_path)?;

    println!(
        "  baseline pages: {} | current pages: {} | total changes: {}\n",
        diff.baseline_pages,
        diff.target_pages,
        diff.total_changes()
    );

    for entry in &diff.added {
        println!("  + added:    {}", entry.url);
    }
    for entry in &diff.removed {
        println!("  - removed:  {}", entry.url);
    }
    for entry in &diff.title_changes {
        println!("  ~ title:    {}", entry.url);
    }
    for entry in &diff.content_changes {
        println!("  ~ content:  {}", entry.url);
    }

    // --- Step 2: run one monitoring cycle over the delta ------------------
    println!("\n[2/3] Running a monitoring cycle (threshold = 1 change)...");
    let monitor = ContinuousMonitor::new(MonitorConfig {
        check_interval_secs: 3600,
        alert_threshold: 1,
        notification_channels: Vec::new(),
    });
    let result = monitor.check(&diff)?;

    println!(
        "  new: {} | removed: {} | changed: {} | CWV regressions: {}",
        result.new_pages, result.removed_pages, result.changed_pages, result.cwv_regressions
    );
    println!("  alert triggered: {}", result.alert_triggered);
    println!("  overall severity: {}", result.overall_severity);

    for alert in &result.alerts {
        println!("  [{}] {}: {}", alert.severity, alert.url, alert.message);
    }

    // --- Step 3: build a notification-ready alert -------------------------
    println!("\n[3/3] Building a notification alert...");
    let alert = Alert::from_result(&result, 5);
    println!("  title:       {}", alert.title);
    println!("  severity:    {}", alert.severity);
    println!("  description: {}", alert.description);
    println!("  affected:    {:?}", alert.affected_urls);

    // The same diff can be rendered as a Markdown changelog, e.g. for a
    // Slack message or a GitHub comment.
    let markdown = diff_to_markdown(&diff);
    println!("\nMarkdown diff preview (first 400 chars):\n");
    let preview: String = markdown.chars().take(400).collect();
    println!("{preview}");

    // --- Cleanup -----------------------------------------------------------
    let _ = std::fs::remove_dir_all(&dir);
    println!("\nMonitoring demo complete (temp databases removed).");

    if result.overall_severity == AlertSeverity::Critical {
        println!(
            "A critical alert fired — in production this would notify the configured channels."
        );
    }

    Ok(())
}

/// Helper: build a `PageData` record for the demo.
fn page(
    id: &str,
    url: &str,
    status_code: u16,
    title: Option<&str>,
    word_count: Option<usize>,
    body_size: usize,
) -> Result<PageData, url::ParseError> {
    let parsed = Url::parse(url)?;
    Ok(PageData {
        id: id.to_string(),
        url: parsed.clone(),
        final_url: parsed,
        status_code,
        title: title.map(str::to_string),
        description: None,
        canonical_url: None,
        word_count,
        load_time_ms: Some(120),
        body_size: Some(body_size),
        fetched_at: Utc::now(),
        links: Vec::new(),
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
    })
}

/// Helper: open a storage database, record one finished crawl with `pages`.
fn write_crawl(
    path: &std::path::Path,
    target: &str,
    pages: &[PageData],
) -> Result<(), Box<dyn std::error::Error>> {
    let storage = Storage::new(path)?;
    let crawl_id = storage.start_crawl(target, None)?;
    storage.insert_pages(&crawl_id, pages)?;
    let total_issues = 0;
    storage.finish_crawl(&crawl_id, pages.len(), total_issues)?;
    Ok(())
}
