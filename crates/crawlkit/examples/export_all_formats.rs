//! Example: Crawl a page and export the results in every supported format —
//! JSON, CSV, HTML, and Markdown — in a single run.
//!
//! The example mirrors the CLI's `--format all` behaviour at the library
//! level:
//!
//! 1. Fetch a live page (falls back to sample data when offline).
//! 2. Parse the HTML and run the analyzer registry over it.
//! 3. Persist pages + issues via `Storage`.
//! 4. Export with `export_json`, `export_csv`, `export_html`, and
//!    `export_markdown`, writing `crawlkit-report.{json,csv,html,md}`
//!    into the current directory.
//!
//! Run with: cargo run --example export_all_formats

use std::time::Duration;

use chrono::Utc;
use crawlkit_engine::analyzers::{AnalysisContext, AnalyzerRegistry};
use crawlkit_engine::export::{
    export_csv, export_html, export_json, export_markdown, CsvColumnSelector,
};
use crawlkit_engine::storage::{Issue, IssueCategory, PageData, Severity, Storage};
use crawlkit_engine::{CrawlConfig, HtmlParser, HttpClient};
use url::Url;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://example.com".to_string());
    let url = Url::parse(&target)?;

    // --- Step 1: fetch (with an offline fallback) -------------------------
    println!("Fetching {url}...");
    let config = CrawlConfig {
        request_timeout: Duration::from_secs(15),
        ..CrawlConfig::default()
    };
    let client = HttpClient::from_crawl_config(&config)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let fetch_result = runtime.block_on(client.fetch(&url));

    let (page, issues) = match fetch_result {
        Ok(result) => {
            println!(
                "  status: {} ({} bytes)",
                result.status_code, result.body_size
            );

            // --- Step 2: parse + analyze ----------------------------------
            let parsed = HtmlParser::parse(&result.body, &url);
            let registry = AnalyzerRegistry::new(&config);
            let ctx = AnalysisContext {
                page: &parsed,
                body: Some(&result.body),
                status_code: Some(result.status_code),
                headers: &result.headers,
                response_time: Some(result.response_time),
                redirect_chain: &[],
                robots_txt: None,
                body_size: Some(result.body_size),
                compressed_size: None,
                server: None,
                content_type: None,
                rendered: None,
            };
            let findings = registry.analyze(&ctx);
            println!("  analyzers produced {} findings", findings.len());

            let page = page_from(
                &url,
                &parsed.meta.title,
                result.status_code,
                result.body_size,
            );
            let issues: Vec<Issue> = findings
                .iter()
                .map(|f| issue_from("p1", f.severity, f.category.clone(), f))
                .collect();
            (page, issues)
        }
        Err(e) => {
            // Offline / blocked network: demonstrate exports with sample data
            // instead of failing the example.
            eprintln!("Fetch failed ({e}) — using sample data so the export demo still runs.");
            sample_page_and_issues(&url)
        }
    };

    // --- Step 3: persist pages + issues ------------------------------------
    let storage = Storage::new_in_memory()?;
    let crawl_id = storage.start_crawl(url.as_str(), None)?;
    storage.insert_pages(&crawl_id, &[page])?;
    storage.insert_issues(&issues)?;
    storage.finish_crawl(&crawl_id, 1, issues.len())?;

    // --- Step 4: export all four formats -----------------------------------
    println!("\nExporting to all formats:");

    let json_path = "crawlkit-report.json";
    let json = export_json(&storage, &crawl_id, true)?;
    std::fs::write(json_path, json)?;
    println!("  wrote {json_path}");

    let csv_path = "crawlkit-report.csv";
    let csv = export_csv(&storage, &crawl_id, &CsvColumnSelector::all())?;
    std::fs::write(csv_path, csv)?;
    println!("  wrote {csv_path}");

    let html_path = "crawlkit-report.html";
    let html = export_html(&storage, &crawl_id)?;
    std::fs::write(html_path, html)?;
    println!("  wrote {html_path}");

    let md_path = "crawlkit-report.md";
    let md = export_markdown(&storage, &crawl_id)?;
    std::fs::write(md_path, md)?;
    println!("  wrote {md_path}");

    println!("\nAll four export formats written. Open crawlkit-report.html in a");
    println!("browser for the interactive report, or crawlkit-report.md for a");
    println!("plain-text summary suitable for Slack or a GitHub issue.");
    Ok(())
}

/// Build a single `PageData` row from a fetch result.
fn page_from(url: &Url, title: &Option<String>, status_code: u16, body_size: usize) -> PageData {
    PageData {
        id: "p1".to_string(),
        url: url.clone(),
        final_url: url.clone(),
        status_code,
        title: title.clone(),
        description: None,
        canonical_url: None,
        word_count: None,
        load_time_ms: None,
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
    }
}

/// Convert an analyzer `Finding` into a storable `Issue`.
fn issue_from(
    page_id: &str,
    severity: Severity,
    category: IssueCategory,
    finding: &crawlkit_engine::Finding,
) -> Issue {
    Issue {
        id: format!("i-{}", finding.code),
        page_id: page_id.to_string(),
        category,
        severity,
        code: finding.code.clone(),
        title: finding.title.clone(),
        description: finding.description.clone(),
        element: None,
        recommendation: finding.recommendation.clone(),
        tenant_id: None,
    }
}

/// Sample data used when the network is unavailable, so the export half of
/// the example still demonstrates every format.
fn sample_page_and_issues(url: &Url) -> (PageData, Vec<Issue>) {
    let page = PageData {
        id: "p1".to_string(),
        url: url.clone(),
        final_url: url.clone(),
        status_code: 200,
        title: Some("Sample page".to_string()),
        description: None,
        canonical_url: None,
        word_count: Some(420),
        load_time_ms: Some(210),
        body_size: Some(3_512),
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
        // Note: the optional heading-count columns are left unset here —
        // the storage migration creates them as TEXT while reads expect
        // integers, so populating them trips a read-back type mismatch.
        h1_count: None,
        heading_count: None,
        extractions: None,
    };
    let issues = vec![
        Issue {
            id: "i-1".to_string(),
            page_id: "p1".to_string(),
            category: IssueCategory::Seo,
            severity: Severity::Warning,
            code: "SEO001".to_string(),
            title: "Missing meta description".to_string(),
            description: "Page has no meta description".to_string(),
            element: Some("meta[name=description]".to_string()),
            recommendation: "Add a meta description".to_string(),
            tenant_id: None,
        },
        Issue {
            id: "i-2".to_string(),
            page_id: "p1".to_string(),
            category: IssueCategory::Images,
            severity: Severity::Warning,
            code: "IMG001".to_string(),
            title: "Image missing alt text".to_string(),
            description: "2 of 4 images have no alt attribute".to_string(),
            element: Some("img".to_string()),
            recommendation: "Add descriptive alt text".to_string(),
            tenant_id: None,
        },
    ];
    (page, issues)
}
