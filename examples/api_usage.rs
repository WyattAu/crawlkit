//! API usage example demonstrating programmatic use of crawlkit.

use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
use crawlkit_engine::storage::Storage;
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Crawlkit API Usage Examples");
    println!("===========================");
    println!();

    // Example 1: Basic crawl with default settings
    example_basic_crawl().await?;

    // Example 2: Crawl with custom configuration
    example_custom_config().await?;

    // Example 3: Crawl with progress callback
    example_progress_callback().await?;

    Ok(())
}

/// Example 1: Basic crawl with default settings.
async fn example_basic_crawl() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Basic Crawl");
    println!("---------------------");

    let db_path = Path::new("example_basic.db");
    let storage = Storage::new(db_path)?;
    let config = CrawlEngineConfig::default();

    let engine = CrawlEngine::new(config, storage);
    let result = engine.run("https://example.com").await?;

    println!("  Crawled {} pages in {:?}", result.pages_crawled, result.elapsed);
    println!();

    let _ = std::fs::remove_file(db_path);
    Ok(())
}

/// Example 2: Crawl with custom configuration.
async fn example_custom_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Custom Configuration");
    println!("------------------------------");

    let db_path = Path::new("example_custom.db");
    let storage = Storage::new(db_path)?;

    let config = CrawlEngineConfig {
        crawl_config: crawlkit_engine::CrawlConfig {
            respect_robots_txt: true,
            max_time: Some(std::time::Duration::from_secs(10)),
            request_delay: std::time::Duration::from_millis(200),
            concurrency: 2,
            max_depth: Some(2),
            ..Default::default()
        },
        allow_external: false,
        incremental: true,
        ..Default::default()
    };

    let engine = CrawlEngine::new(config, storage);
    let result = engine.run("https://example.com").await?;

    println!("  Crawled {} pages (incremental)", result.pages_crawled);
    println!("  Unchanged: {}, Modified: {}, New: {}",
        result.pages_unchanged, result.pages_modified, result.pages_new);
    println!();

    let _ = std::fs::remove_file(db_path);
    Ok(())
}

/// Example 3: Crawl with progress callback.
async fn example_progress_callback() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Progress Callback");
    println!("---------------------------");

    let db_path = Path::new("example_progress.db");
    let storage = Storage::new(db_path)?;
    let config = CrawlEngineConfig::default();

    let engine = CrawlEngine::new(config, storage);

    // Create a progress callback
    let progress = Arc::new(|url: &str, _page_id: &str, findings: usize| {
        println!("  Crawled: {} ({} findings)", url, findings);
    });

    let result = engine.run_with_callback("https://example.com", Some(progress)).await?;

    println!("  Completed: {} pages", result.pages_crawled);
    println!();

    let _ = std::fs::remove_file(db_path);
    Ok(())
}
