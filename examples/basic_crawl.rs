//! Basic crawl example demonstrating how to crawl a single URL.

use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
use crawlkit_engine::storage::Storage;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Create a temporary database for this crawl
    let db_path = Path::new("example_crawl.db");
    let storage = Storage::new(db_path)?;

    // Configure the crawl engine
    let config = CrawlEngineConfig {
        crawl_config: crawlkit_engine::CrawlConfig {
            respect_robots_txt: true,
            max_time: Some(std::time::Duration::from_secs(30)),
            request_delay: std::time::Duration::from_millis(100),
            concurrency: 4,
            ..Default::default()
        },
        ..Default::default()
    };

    // Create and run the crawl engine
    let engine = CrawlEngine::new(config, storage);
    let result = engine.run("https://example.com").await?;

    // Print results
    println!("Crawl completed!");
    println!("  Pages crawled: {}", result.pages_crawled);
    println!("  Pages stored: {}", result.pages_stored);
    println!("  Issues found: {}", result.issues_found);
    println!("  Duration: {:?}", result.elapsed);

    // Cleanup
    let _ = std::fs::remove_file(db_path);

    Ok(())
}
