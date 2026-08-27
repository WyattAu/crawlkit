//! Example: Crawl a website and analyze all pages.
//!
//! Run with: cargo run --example crawl_and_analyze

use crawlkit_engine::analyzers::AnalyzerRegistry;
use crawlkit_engine::{CrawlConfig, HtmlParser, HttpClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = url::Url::parse("https://example.com")?;
    let config = CrawlConfig::default();
    let client = HttpClient::from_crawl_config(&config)?;
    let registry = AnalyzerRegistry::new(&config);

    println!("Fetching {}...", url);

    let result = client.fetch(&url).await?;
    println!("Status: {}", result.status_code);
    println!("Body size: {} bytes", result.body.len());

    let parsed = HtmlParser::parse(&result.body, &url);
    println!("Title: {:?}", parsed.meta.title);
    println!("Links: {}", parsed.links.len());
    println!("Images: {}", parsed.images.len());

    let ctx = crawlkit_engine::analyzers::AnalysisContext {
        page: &parsed,
        body: Some(&result.body),
        status_code: Some(result.status_code),
        headers: &result.headers,
        response_time: Some(result.response_time),
        redirect_chain: &[],
        robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
    };

    let findings = registry.analyze(&ctx);
    println!("\nFindings ({} total):", findings.len());
    for finding in &findings {
        println!(
            "  [{}] {}: {}",
            finding.severity.as_str(),
            finding.code,
            finding.title
        );
    }

    Ok(())
}
