//! Basic crawl example using crawlkit-core as a library.
//!
//! This example demonstrates:
//! - Configuring a crawl via `CrawlConfig`
//! - Using `HttpClient` with retry and redirect tracking
//! - Parsing HTML with `HtmlParser`
//! - Running the built-in analyzer registry
//! - Storing results in SQLite via `Storage`
//! - Exporting to JSON, CSV, and HTML
//!
//! Run with:
//!     cargo run --example basic-crawl

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use crawlkit_engine::analyzers::{AnalysisContext, AnalyzerRegistry};
use crawlkit_engine::export::{export_csv, export_html, export_json, CsvColumnSelector};
use crawlkit_engine::http::{HttpClient, HttpClientConfig, RetryPolicy, UserAgentRotator};
use crawlkit_engine::queue::{Priority, ScopeConfig, UrlQueue};
use crawlkit_engine::ratelimit::RateLimiter;
use crawlkit_engine::storage::{PageData, Storage};
use crawlkit_engine::{CrawlConfig, HtmlParser};
use tokio::sync::Mutex;
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Configure the crawl
    let target_url = "https://example.com";
    let max_pages = 50;
    let concurrency = 4;
    let delay_ms = 500;

    let config = CrawlConfig {
        start_url: Url::parse(target_url)?,
        max_pages,
        max_time: None,
        max_depth: None,
        request_delay: Duration::from_millis(delay_ms),
        concurrency,
        request_timeout: Duration::from_secs(30),
        user_agent: "crawlkit-example/0.1.0".to_string(),
        max_redirects: 20,
        respect_robots_txt: true,
        allowed_patterns: Vec::new(),
        disallowed_patterns: vec!["/admin/*".to_string(), "/api/*".to_string()],
    };

    println!(
        "Crawling {} (max {max_pages} pages, {concurrency} concurrent)",
        target_url
    );

    // 2. Initialize storage
    let output_dir = Path::new("crawl-output");
    std::fs::create_dir_all(output_dir)?;
    let db_path = output_dir.join("crawl.db");
    let storage = Storage::new(&db_path)?;
    let crawl_id = storage.start_crawl(target_url, Some(&serde_json::to_string(&config)?))?;
    println!("Crawl ID: {crawl_id}");

    // 3. Create HTTP client with retry and user-agent rotation
    let http_config = HttpClientConfig {
        timeout: Duration::from_secs(30),
        max_redirects: 20,
        retry_policy: RetryPolicy {
            max_retries: 3,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            retryable_statuses: vec![429, 500, 502, 503, 504],
        },
        user_agent: Arc::new(UserAgentRotator::new(vec![
            "crawlkit-example/0.1.0 (https://github.com/WyattAu/crawlkit)".to_string(),
            "Mozilla/5.0 (compatible; crawlkit/0.1.0)".to_string(),
        ])),
        max_body_size: 10 * 1024 * 1024, // 10 MB
        pool_max_idle_per_host: 32,
        pool_max_idle: 64,
        tcp_keepalive: Some(Duration::from_secs(30)),
        pool_idle_timeout: Duration::from_secs(90),
        connect_timeout: Duration::from_secs(10),
    };
    let client = HttpClient::new(http_config)?;

    // 4. Set up URL queue with scope control
    let scope = ScopeConfig {
        max_depth: Some(3),
        blocked_paths: vec!["/admin".to_string(), "/api".to_string()],
        ..Default::default()
    };
    let queue = Arc::new(Mutex::new(UrlQueue::new(scope)));

    // 5. Rate limiter
    let rate_limiter = RateLimiter::new(concurrency as f64, 1.0 / (delay_ms as f64 / 1000.0));

    // 6. Analyzer registry — runs all 18 built-in analyzers
    let analyzer_registry = AnalyzerRegistry::new(&config);

    // 7. Seed the queue
    let seed = Url::parse(target_url)?;
    {
        let q = queue.lock().await;
        q.push(seed.clone(), 0, Priority::HIGHEST);
    }

    let seed_domain = seed.host_str().unwrap_or("").to_string();
    let mut pages_crawled = 0usize;
    let mut issues_found = 0usize;

    // 8. Main crawl loop
    while pages_crawled < max_pages {
        let entry = {
            let q = queue.lock().await;
            q.pop()
        };
        let entry = match entry {
            Some(e) => e,
            None => break,
        };

        // Rate-limit per domain
        if rate_limiter
            .acquire(entry.url.host_str().unwrap_or(""))
            .await
            .is_err()
        {
            continue;
        }

        // Fetch
        let start = std::time::Instant::now();
        let result = match client.fetch(&entry.url).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  Failed {}: {e}", entry.url);
                continue;
            }
        };
        let fetch_time = start.elapsed();

        println!(
            "  [{:>3}] {} — {:.0?}, {} bytes",
            result.status_code, entry.url, fetch_time, result.body_size
        );

        // Parse HTML
        let parsed = match HtmlParser::parse(&result.body, &entry.url) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("  Parse error {}: {e}", entry.url);
                continue;
            }
        };

        // Run analyzers
        let headers_vec: Vec<(String, String)> = result.headers.clone();
        let ctx = AnalysisContext {
            page: &parsed,
            body: Some(&result.body),
            status_code: Some(result.status_code),
            headers: &headers_vec,
            response_time: Some(fetch_time),
            redirect_chain: &[],
            robots_txt: None,
        };
        let findings = analyzer_registry.analyze(&ctx, &config);
        issues_found += findings.len();

        // Store page
        let page_data = PageData {
            id: uuid::Uuid::new_v4().to_string(),
            url: entry.url.clone(),
            final_url: result.final_url.clone(),
            status_code: result.status_code,
            title: parsed.meta.title.clone(),
            description: parsed.meta.description.clone(),
            canonical_url: parsed.meta.canonical.clone(),
            word_count: Some(parsed.word_count),
            load_time_ms: Some(fetch_time.as_millis() as u64),
            body_size: Some(result.body_size),
            fetched_at: Utc::now(),
            links: parsed
                .links
                .iter()
                .filter_map(|l| Url::parse(&l.href).ok())
                .collect(),
            tenant_id: None,
            etag: result.etag.clone(),
            last_modified: result.last_modified.clone(),
            cwv_lcp: None,
            cwv_cls: None,
            cwv_inp: None,
        };
        if let Err(e) = storage.insert_page(&crawl_id, &page_data) {
            eprintln!("  Storage error: {e}");
        }

        // Store findings as issues
        for finding in &findings {
            let issue = crawlkit_engine::storage::Issue {
                id: uuid::Uuid::new_v4().to_string(),
                page_id: page_data.id.clone(),
                category: finding.category.clone(),
                severity: finding.severity.clone(),
                code: finding.code.clone(),
                title: finding.title.clone(),
                description: finding.description.clone(),
                element: None,
                recommendation: finding.recommendation.clone(),
                tenant_id: None,
            };
            let _ = storage.insert_issue(&issue);
        }

        // Discover new links
        for link in &parsed.links {
            let link_url = match Url::parse(&link.href) {
                Ok(u) => u,
                Err(_) => continue,
            };
            let is_internal = link_url.host_str() == Some(&seed_domain);
            let priority = if is_internal {
                Priority::NORMAL
            } else {
                Priority::LOW
            };
            let q = queue.lock().await;
            q.push(link_url, entry.depth + 1, priority);
        }

        pages_crawled += 1;
    }

    // 9. Finalize crawl
    storage.finish_crawl(&crawl_id, pages_crawled, issues_found)?;
    println!("\nCrawl complete: {pages_crawled} pages, {issues_found} issues");

    // 10. Export results

    // JSON export
    let json = export_json(&storage, &crawl_id, true)?;
    let json_path = output_dir.join("results.json");
    std::fs::write(&json_path, &json)?;
    println!("JSON → {}", json_path.display());

    // CSV export (subset of columns)
    let selector = CsvColumnSelector {
        url: true,
        status_code: true,
        title: true,
        word_count: true,
        load_time_ms: true,
        issue_count: true,
        ..Default::default()
    };
    let conn = &*storage.conn();
    let csv_bytes = export_csv(conn, &crawl_id, &selector)?;
    let csv_path = output_dir.join("results.csv");
    std::fs::write(&csv_path, &csv_bytes)?;
    println!("CSV  → {}", csv_path.display());

    // HTML report
    let html = export_html(&storage, &crawl_id)?;
    let html_path = output_dir.join("report.html");
    std::fs::write(&html_path, &html)?;
    println!("HTML → {}", html_path.display());

    Ok(())
}
