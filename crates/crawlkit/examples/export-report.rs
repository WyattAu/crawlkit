//! Export and report generation example.
//!
//! This example demonstrates how to export crawl data in every supported format:
//! JSON, CSV, Markdown, and HTML. It uses an in-memory SQLite database with
//! seed data so it runs without network access.
//!
//! Run with:
//!     cargo run --example export-report

use anyhow::Result;
use chrono::Utc;
use crawlkit_core::compare::{diff_to_json, diff_to_markdown};
use crawlkit_core::export::{
    export_csv, export_html, export_json, export_markdown, CsvColumnSelector,
};
use crawlkit_core::storage::{Issue, IssueCategory, PageData, Severity, Storage};
use std::path::Path;
use url::Url;

fn main() -> Result<()> {
    // 1. Create an in-memory database and seed it with sample data
    let storage = Storage::new_in_memory()?;
    let crawl_id = storage.start_crawl("https://example.com", None)?;

    let pages = vec![
        PageData {
            id: "p1".into(),
            url: Url::parse("https://example.com/")?,
            final_url: Url::parse("https://example.com/")?,
            status_code: 200,
            title: Some("Home — Example".into()),
            description: Some("Welcome to Example. We build great products.".into()),
            canonical_url: Some(Url::parse("https://example.com/")?),
            word_count: Some(1200),
            load_time_ms: Some(150),
            body_size: Some(4096),
            fetched_at: Utc::now(),
            links: vec![Url::parse("https://example.com/about")?],
        },
        PageData {
            id: "p2".into(),
            url: Url::parse("https://example.com/about")?,
            final_url: Url::parse("https://example.com/about")?,
            status_code: 200,
            title: Some("About Us".into()),
            description: Some("Learn more about our company.".into()),
            canonical_url: None,
            word_count: Some(800),
            load_time_ms: Some(200),
            body_size: Some(2048),
            fetched_at: Utc::now(),
            links: vec![],
        },
        PageData {
            id: "p3".into(),
            url: Url::parse("https://example.com/old-page")?,
            final_url: Url::parse("https://example.com/old-page")?,
            status_code: 404,
            title: None,
            description: None,
            canonical_url: None,
            word_count: Some(50),
            load_time_ms: Some(80),
            body_size: Some(512),
            fetched_at: Utc::now(),
            links: vec![],
        },
    ];
    storage.insert_pages(&crawl_id, &pages)?;

    let issues = vec![
        Issue {
            id: "i1".into(),
            page_id: "p1".into(),
            category: IssueCategory::Seo,
            severity: Severity::Warning,
            code: "META002".into(),
            title: "Title too short".into(),
            description: "Title is 25 characters, below the recommended minimum of 30.".into(),
            element: None,
            recommendation: "Expand the title to 30-60 characters.".into(),
        },
        Issue {
            id: "i2".into(),
            page_id: "p3".into(),
            category: IssueCategory::Http,
            severity: Severity::Critical,
            code: "HTTP004".into(),
            title: "Page not found (404)".into(),
            description: "The page returned a 404 status code.".into(),
            element: None,
            recommendation: "Remove links to this page or redirect to a valid URL.".into(),
        },
        Issue {
            id: "i3".into(),
            page_id: "p2".into(),
            category: IssueCategory::Seo,
            severity: Severity::Warning,
            code: "CANON001".into(),
            title: "Missing canonical URL".into(),
            description: "No <link rel=\"canonical\"> tag was found.".into(),
            element: None,
            recommendation: "Add a canonical URL tag pointing to the preferred version.".into(),
        },
        Issue {
            id: "i4".into(),
            page_id: "p1".into(),
            category: IssueCategory::Images,
            severity: Severity::Error,
            code: "IMG001".into(),
            title: "Image missing alt text".into(),
            description: "An image has no alt attribute.".into(),
            element: Some("img.hero".into()),
            recommendation: "Add descriptive alt text to all images.".into(),
        },
    ];
    storage.insert_issues(&issues)?;
    storage.finish_crawl(&crawl_id, 3, 4)?;

    // 2. JSON export (pretty-printed)
    println!("=== JSON Export ===\n");
    let json = export_json(&storage, &crawl_id, true)?;
    let json_path = Path::new("crawl-output/export.json");
    std::fs::create_dir_all("crawl-output")?;
    std::fs::write(json_path, &json)?;
    println!("Written to {json_path:?} ({} bytes)\n", json.len());

    // 3. CSV export with custom column selection
    println!("=== CSV Export ===\n");
    let selector = CsvColumnSelector {
        url: true,
        status_code: true,
        title: true,
        word_count: true,
        issue_count: true,
        ..Default::default()
    };
    let conn = &*storage.conn();
    let csv_bytes = export_csv(conn, &crawl_id, &selector)?;
    let csv_path = Path::new("crawl-output/export.csv");
    std::fs::write(csv_path, &csv_bytes)?;
    println!("Written to {csv_path:?} ({} bytes)\n", csv_bytes.len());

    // Print the CSV to stdout for verification
    let csv_str = String::from_utf8_lossy(&csv_bytes);
    for line in csv_str.lines().take(5) {
        println!("  {line}");
    }
    println!();

    // 4. Markdown summary
    println!("=== Markdown Export ===\n");
    let md = export_markdown(&storage, &crawl_id)?;
    let md_path = Path::new("crawl-output/report.md");
    std::fs::write(md_path, &md)?;
    println!("Written to {md_path:?} ({} bytes)\n", md.len());

    // Print first 20 lines
    for line in md.lines().take(20) {
        println!("  {line}");
    }
    println!("  ...\n");

    // 5. HTML report (self-contained, interactive)
    println!("=== HTML Report ===\n");
    let html = export_html(&storage, &crawl_id)?;
    let html_path = Path::new("crawl-output/report.html");
    std::fs::write(html_path, &html)?;
    println!("Written to {html_path:?} ({} bytes)\n", html.len());

    // 6. Demonstrate crawl comparison (diff)
    println!("=== Crawl Comparison ===\n");

    // Create a second database with changes
    let storage2 = Storage::new_in_memory()?;
    let crawl_id2 = storage2.start_crawl("https://example.com", None)?;

    let pages2 = vec![
        PageData {
            id: "p1".into(),
            url: Url::parse("https://example.com/")?,
            final_url: Url::parse("https://example.com/")?,
            status_code: 200,
            title: Some("Home — Example Inc.".into()), // title changed
            description: Some("Welcome to Example.".into()),
            canonical_url: Some(Url::parse("https://example.com/")?),
            word_count: Some(1500), // content grew
            load_time_ms: Some(120),
            body_size: Some(5120),
            fetched_at: Utc::now(),
            links: vec![],
        },
        // p2 unchanged, p3 removed, p4 new
        PageData {
            id: "p4".into(),
            url: Url::parse("https://example.com/blog")?,
            final_url: Url::parse("https://example.com/blog")?,
            status_code: 200,
            title: Some("Blog".into()),
            description: Some("Latest articles.".into()),
            canonical_url: None,
            word_count: Some(600),
            load_time_ms: Some(180),
            body_size: Some(3072),
            fetched_at: Utc::now(),
            links: vec![],
        },
    ];
    storage2.insert_pages(&crawl_id2, &pages2)?;
    storage2.finish_crawl(&crawl_id2, 2, 0)?;

    // Compare via in-memory connections
    let conn1 = &*storage.conn();
    let conn2 = &*storage2.conn();
    let diff = crawlkit_core::compare::compare_crawl_ids(conn1, &crawl_id, conn2, &crawl_id2)?;

    let diff_json = diff_to_json(&diff, true)?;
    println!("Diff (JSON):\n  {diff_json}\n");

    let diff_md = diff_to_markdown(&diff);
    println!("Diff (Markdown):");
    for line in diff_md.lines() {
        println!("  {line}");
    }

    println!("\n=== All exports complete ===");
    Ok(())
}
