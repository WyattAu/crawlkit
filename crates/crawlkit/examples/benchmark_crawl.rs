//! Example: Benchmark a crawl with detailed per-page timing metrics.
//!
//! Measures and reports, for every fetched page:
//!
//! - **fetch time** — network round-trip (HTTP request → body received)
//! - **parse time** — HTML parsing into a `ParsedPage`
//! - **analysis time** — running the full analyzer registry
//!
//! Aggregate numbers (average fetch/analysis time, throughput, pages/sec)
//! come from `observability::Metrics`, the same counters the crawler engine
//! records during a real run, so the output matches `metrics.json` emitted
//! by the CLI.
//!
//! Run with: cargo run --example benchmark_crawl
//! Optional: pass a target URL as the first CLI argument.

use std::time::{Duration, Instant};

use crawlkit_engine::analyzers::{AnalysisContext, AnalyzerRegistry};
use crawlkit_engine::observability::Metrics;
use crawlkit_engine::{CrawlConfig, HtmlParser, HttpClient};

/// Crawl at most this many pages to keep the demo quick.
const MAX_PAGES: usize = 5;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://example.com".to_string());
    let start_url = url::Url::parse(&target)?;

    let config = CrawlConfig {
        max_pages: MAX_PAGES,
        request_timeout: Duration::from_secs(15),
        ..CrawlConfig::default()
    };
    let client = HttpClient::from_crawl_config(&config)?;
    let registry = AnalyzerRegistry::new(&config);
    let metrics = Metrics::new();

    println!("Benchmarking crawl of {start_url} (up to {MAX_PAGES} pages)\n");

    let started = Instant::now();
    let mut frontier = vec![start_url.clone()];
    let mut queued: Vec<String> = vec![start_url.to_string()];
    let mut crawled = 0usize;

    println!(
        "{:<55} {:>10} {:>10} {:>10}",
        "URL", "fetch(ms)", "parse(ms)", "analyze(ms)"
    );
    println!("{}", "-".repeat(90));

    while let Some(url) = frontier.pop() {
        if crawled >= MAX_PAGES {
            break;
        }

        // --- Phase 1: fetch ------------------------------------------------
        let fetch_start = Instant::now();
        let result = match client.fetch(&url).await {
            Ok(result) => result,
            Err(e) => {
                // A single failed page must not abort the benchmark.
                println!("{:<55} fetch failed: {e}", truncate(url.as_str(), 55));
                metrics.record_page_failure();
                continue;
            }
        };
        let fetch_us = fetch_start.elapsed().as_micros() as u64;

        // --- Phase 2: parse ------------------------------------------------
        let parse_start = Instant::now();
        let parsed = HtmlParser::parse(&result.body, &url);
        let parse_us = parse_start.elapsed().as_micros() as u64;

        // --- Phase 3: analyze ----------------------------------------------
        let analysis_start = Instant::now();
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
        let analysis_us = analysis_start.elapsed().as_micros() as u64;

        // Storage is external to this example — record a zero write cost so
        // the shared Metrics averages stay comparable with engine runs.
        metrics.record_page_success(
            result.body_size as u64,
            fetch_us,
            analysis_us,
            0,
            findings.len() as u64,
        );

        println!(
            "{:<55} {:>10} {:>10} {:>10}",
            truncate(url.as_str(), 55),
            fetch_us as f64 / 1000.0,
            parse_us as f64 / 1000.0,
            analysis_us as f64 / 1000.0,
        );

        crawled += 1;

        // Enqueue same-host links discovered on this page.
        for link in &parsed.links {
            if crawled + frontier.len() >= MAX_PAGES {
                break;
            }
            if link.is_external {
                continue;
            }
            let Ok(link_url) = url::Url::parse(&link.href) else {
                continue;
            };
            if link_url.host_str() != url.host_str() {
                continue;
            }
            let key = link_url.to_string();
            if !queued.contains(&key) {
                queued.push(key);
                frontier.push(link_url);
            }
        }
    }

    // --- Aggregate report ---------------------------------------------------
    let snapshot = metrics.snapshot();
    let elapsed = started.elapsed();
    println!("\nAggregate metrics:");
    println!("  pages crawled:      {}", snapshot.pages_crawled);
    println!("  pages failed:       {}", snapshot.pages_failed);
    println!("  findings generated: {}", snapshot.findings_generated);
    println!("  bytes fetched:      {}", snapshot.bytes_fetched);
    println!(
        "  avg fetch time:     {:.2} ms",
        metrics.avg_fetch_time_ms()
    );
    println!(
        "  avg analysis time:  {:.2} ms",
        metrics.avg_analysis_time_ms()
    );
    println!(
        "  throughput:         {:.2} pages/sec ({elapsed:.1?} total)",
        metrics.pages_per_second(elapsed)
    );

    println!("\nBenchmark complete. Compare these numbers with `crawlkit crawl");
    println!("--metrics-json` output when tuning request_delay/concurrency.");
    Ok(())
}

/// Truncate a string to at most `max` characters for table output.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let kept: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{kept}…")
    }
}
