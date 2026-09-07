//! Example: Incremental crawling with ETag / If-Modified-Since.
//!
//! Demonstrates the conditional-request path in `HttpClient::fetch_conditional`:
//!
//! 1. A plain fetch records the response's `ETag` and `Last-Modified` headers.
//! 2. A conditional re-fetch sends `If-None-Match` / `If-Modified-Since`.
//!    An unchanged resource answers `304 Not Modified` with an empty body,
//!    saving bandwidth and letting the crawler skip re-analysis.
//! 3. A conditional fetch with a deliberately stale `Last-Modified` date
//!    forces the server to return the full `200 OK` body.
//!
//! Run with: cargo run --example incremental_crawl
//! Optional: pass a target URL as the first CLI argument.

use std::time::Duration;

use crawlkit_engine::{CrawlConfig, HttpClient};

/// `Last-Modified` date far in the past — forces a fresh 200 response.
const STALE_LAST_MODIFIED: &str = "Mon, 01 Jan 1990 00:00:00 GMT";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Allow overriding the target via the first CLI argument.
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://example.com".to_string());
    let url = url::Url::parse(&target)?;

    let config = CrawlConfig {
        request_timeout: Duration::from_secs(15),
        ..CrawlConfig::default()
    };
    let client = HttpClient::from_crawl_config(&config)?;

    println!("Incremental crawl demo for {url}\n");

    // --- Step 1: plain fetch — learn the validators -----------------------
    println!("[1/3] Plain fetch...");
    let first = match client.fetch(&url).await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Fetch failed ({e}). Check your network connection and try again.");
            return Ok(());
        }
    };

    println!("  status:      {}", first.status_code);
    println!("  body size:   {} bytes", first.body_size);
    println!("  etag:        {:?}", first.etag);
    println!("  last-modified: {:?}", first.last_modified);

    if first.etag.is_none() && first.last_modified.is_none() {
        println!(
            "\nNote: the server sent no ETag or Last-Modified header, so conditional\n\
             revalidation cannot skip the body. The demo continues anyway — many\n\
             production servers (CDNs, static hosts) do send validators."
        );
    }

    // --- Step 2: conditional re-fetch with the stored validators ----------
    println!("\n[2/3] Conditional re-fetch (If-None-Match / If-Modified-Since)...");
    let second = client
        .fetch_conditional(&url, first.etag.as_deref(), first.last_modified.as_deref())
        .await?;

    if second.status_code == 304 {
        println!(
            "  status: 304 Not Modified — body empty ({} bytes), nothing to re-parse.",
            second.body_size
        );
        println!("  -> An incremental crawl would reuse the stored page and skip analysis.");
    } else {
        println!(
            "  status: {} — server returned a fresh body ({} bytes).",
            second.status_code, second.body_size
        );
        println!("  -> Content changed since the last crawl; the page is re-analyzed.");
    }

    // --- Step 3: stale validator forces the modified path -----------------
    println!("\n[3/3] Conditional fetch with a stale Last-Modified date...");
    let third = client
        .fetch_conditional(&url, None, Some(STALE_LAST_MODIFIED))
        .await?;

    println!(
        "  status: {} — full body returned ({} bytes).",
        third.status_code, third.body_size
    );
    println!("  -> A stale validator makes the server serve the resource again.");

    println!("\nIncremental crawl demo complete.");
    Ok(())
}
