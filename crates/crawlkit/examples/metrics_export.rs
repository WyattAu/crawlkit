//! Example: Export crawl metrics to JSON.
//!
//! Run with: cargo run --example metrics_export

use crawlkit_core::observability::Metrics;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metrics = Metrics::new();

    // Simulate crawl activity
    for i in 0..100 {
        let bytes = 1024 + (i * 100) as u64;
        let fetch_us = 100 + (i * 10) as u64;
        let analysis_us = 50 + (i * 5) as u64;
        let storage_us = 10 + (i * 2) as u64;
        let findings = i as u64 % 10;

        metrics.record_page_success(bytes, fetch_us, analysis_us, storage_us, findings);

        if i % 20 == 0 {
            metrics.record_page_failure();
        }
    }

    let snapshot = metrics.snapshot();
    let json = serde_json::to_string_pretty(&snapshot)?;
    println!("{}", json);

    println!("\nSummary:");
    println!("  Pages crawled: {}", snapshot.pages_crawled);
    println!("  Pages failed: {}", snapshot.pages_failed);
    println!("  Total bytes: {}", snapshot.bytes_fetched);
    println!("  Avg fetch time: {:.2}ms", metrics.avg_fetch_time_ms());
    println!(
        "  Avg analysis time: {:.2}ms",
        metrics.avg_analysis_time_ms()
    );
    println!(
        "  Throughput: {:.2} bytes/sec",
        metrics.throughput_bps(Duration::from_secs(1))
    );

    Ok(())
}
