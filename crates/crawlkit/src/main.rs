use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

use crawlkit_core::storage::Storage;

/// CLI configuration file structure.
#[derive(serde::Deserialize, Default)]
struct Config {
    /// Crawl-specific defaults.
    crawl: Option<CrawlConfig>,
    /// Output settings.
    output: Option<OutputConfig>,
}

/// Crawl configuration loaded from file.
#[derive(serde::Deserialize)]
struct CrawlConfig {
    /// Default max pages.
    max_pages: Option<usize>,
    /// Default request delay in milliseconds.
    delay_ms: Option<u64>,
    /// Default concurrency.
    concurrency: Option<usize>,
    /// Default request timeout in seconds.
    timeout_secs: Option<u64>,
    /// Custom user agent.
    user_agent: Option<String>,
    /// Whether to respect robots.txt.
    respect_robots_txt: Option<bool>,
}

/// Output configuration loaded from file.
#[derive(serde::Deserialize)]
struct OutputConfig {
    /// Default output directory.
    dir: Option<String>,
    /// Default output format.
    format: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "crawlkit",
    about = "A high-performance Rust-based site crawler for SEO analysis",
    version,
    propagate_version = true
)]
struct Cli {
    /// Path to configuration file.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Enable verbose output.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress all output except errors.
    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Crawl a website and analyze pages for SEO signals
    Crawl {
        /// The starting URL to crawl
        url: String,

        /// Maximum number of pages to crawl
        #[arg(long)]
        max_pages: Option<usize>,

        /// Delay between requests in milliseconds
        #[arg(long)]
        delay: Option<u64>,

        /// Number of concurrent fetchers
        #[arg(long)]
        concurrency: Option<usize>,

        /// Output directory for crawl results
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: json, csv, html, md, or all
        #[arg(long, default_value = "all")]
        format: String,

        /// Maximum crawl depth from the starting URL
        #[arg(long)]
        depth: Option<usize>,

        /// Custom user agent string
        #[arg(long)]
        user_agent: Option<String>,

        /// Request timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,

        /// Respect robots.txt directives
        #[arg(long)]
        respect_robots: Option<bool>,

        /// URL include patterns (glob-style)
        #[arg(long)]
        include: Vec<String>,

        /// URL exclude patterns (glob-style)
        #[arg(long)]
        exclude: Vec<String>,

        /// Enable JavaScript rendering (requires Chrome)
        #[arg(long)]
        javascript: bool,
    },

    /// Compare two crawl results
    Compare {
        /// First crawl database path or directory
        crawl1: PathBuf,

        /// Second crawl database path or directory
        crawl2: PathBuf,

        /// Output file for the comparison
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: json, html, or md
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Generate a report from an existing crawl
    Report {
        /// Crawl database path or directory
        #[arg(short, long)]
        crawl: PathBuf,

        /// Output file for the report
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Report format: html or md
        #[arg(long, default_value = "html")]
        format: String,

        /// Report theme: light or dark
        #[arg(long, default_value = "light")]
        theme: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing based on verbosity
    let log_level = if cli.quiet {
        "error"
    } else if cli.verbose {
        "debug"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(format!("crawlkit={log_level}"))),
        )
        .init();

    // Load config file if specified
    let config = if let Some(config_path) = &cli.config {
        let contents = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
        toml::from_str::<Config>(&contents)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?
    } else {
        Config::default()
    };

    match cli.command {
        Commands::Crawl {
            url,
            max_pages,
            delay,
            concurrency,
            output,
            format,
            depth,
            user_agent,
            timeout,
            respect_robots,
            include,
            exclude,
            javascript,
        } => {
            run_crawl(
                &url,
                max_pages.or_else(|| config.crawl.as_ref().and_then(|c| c.max_pages)),
                delay.or_else(|| config.crawl.as_ref().and_then(|c| c.delay_ms)),
                concurrency.or_else(|| config.crawl.as_ref().and_then(|c| c.concurrency)),
                output.or_else(|| {
                    config
                        .output
                        .as_ref()
                        .and_then(|o| o.dir.as_deref().map(PathBuf::from))
                }),
                &format,
                depth,
                user_agent.or_else(|| config.crawl.as_ref().and_then(|c| c.user_agent.clone())),
                timeout.or_else(|| config.crawl.as_ref().and_then(|c| c.timeout_secs)),
                respect_robots.or_else(|| config.crawl.as_ref().and_then(|c| c.respect_robots_txt)),
                include,
                exclude,
                javascript,
            )
            .await
        }
        Commands::Compare {
            crawl1,
            crawl2,
            output,
            format,
        } => run_compare(&crawl1, &crawl2, output.as_deref(), &format).await,
        Commands::Report {
            crawl,
            output,
            format,
            theme,
        } => run_report(&crawl, output.as_deref(), &format, &theme).await,
    }
}

/// Execute a crawl with the given parameters.
async fn run_crawl(
    url: &str,
    max_pages: Option<usize>,
    delay: Option<u64>,
    concurrency: Option<usize>,
    output: Option<PathBuf>,
    format: &str,
    depth: Option<usize>,
    user_agent: Option<String>,
    timeout: Option<u64>,
    respect_robots: Option<bool>,
    include: Vec<String>,
    exclude: Vec<String>,
    javascript: bool,
) -> Result<()> {
    use crawlkit_core::analyzers::AnalyzerRegistry;
    use crawlkit_core::http::HttpClient;
    use crawlkit_core::HtmlParser;
    use crawlkit_core::queue::{Priority, UrlQueue};
    use crawlkit_core::ratelimit::RateLimiter;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let max_pages = max_pages.unwrap_or(100);
    let delay = delay.unwrap_or(500);
    let concurrency = concurrency.unwrap_or(4);
    let timeout_secs = timeout.unwrap_or(30);

    tracing::info!(
        "Starting crawl of {} (max_pages={}, delay={}ms, concurrency={}, depth={:?}, js={})",
        url,
        max_pages,
        delay,
        concurrency,
        depth,
        javascript,
    );

    let pb = ProgressBar::new(max_pages as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} pages ({eta} remaining) - {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Initializing...");

    // Initialize storage
    let output_dir = output.unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;
    let db_path = output_dir.join("crawlkit.db");
    let storage = Storage::new(&db_path)
        .with_context(|| format!("Failed to open storage at {}", db_path.display()))?;

    let crawl_id = storage.start_crawl(url, None)?;
    tracing::info!("Crawl ID: {}", crawl_id);

    // Initialize components
    let http_config = crawlkit_core::http::HttpClientConfig {
        timeout: std::time::Duration::from_secs(timeout_secs),
        max_redirects: 20,
        retry_policy: crawlkit_core::http::RetryPolicy::default(),
        user_agent: std::sync::Arc::new(crawlkit_core::http::UserAgentRotator::new(vec![
            user_agent.unwrap_or_else(|| "crawlkit/0.1.0".to_string()),
        ])),
        max_body_size: 10 * 1024 * 1024,
    };
    let client = HttpClient::new(http_config).context("Failed to create HTTP client")?;
    let scope = crawlkit_core::queue::ScopeConfig {
        max_depth: depth,
        ..Default::default()
    };
    let queue = Arc::new(Mutex::new(UrlQueue::new(scope)));
    let rate_limiter = RateLimiter::new(concurrency as f64, 1.0 / (delay as f64 / 1000.0));
    let analyzer_registry = AnalyzerRegistry::new(&crawlkit_core::CrawlConfig::default());

    // Seed the queue
    let seed_url = url::Url::parse(url)
        .with_context(|| format!("Invalid URL: {}", url))?;
    {
        let mut q = queue.lock().await;
        q.push(seed_url.clone(), 0, Priority::HIGH);
    }

    let mut pages_crawled = 0;
    let mut pages_stored = 0;
    let mut visited = std::collections::HashSet::new();

    // Crawl loop
    while pages_crawled < max_pages {
        let entry = {
            let mut q = queue.lock().await;
            q.pop()
        };

        let entry = match entry {
            Some(e) => e,
            None => break, // Queue empty
        };

        // Skip if already visited
        if visited.contains(&entry.url.to_string()) {
            continue;
        }
        visited.insert(entry.url.to_string());

        // Rate limit
        let _ = rate_limiter.acquire(entry.url.host_str().unwrap_or("")).await;

        // Fetch
        let start = std::time::Instant::now();
        let result = match client.fetch(&entry.url).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to fetch {}: {}", entry.url, e);
                continue;
            }
        };
        let fetch_time = start.elapsed();

        pages_crawled += 1;
        pb.set_position(pages_crawled as u64);
        pb.set_message(format!("Fetched: {}", entry.url));

        // Parse HTML
        let body_text = result.body.clone();
        let parser = HtmlParser;
        let parsed = match HtmlParser::parse(&body_text, &entry.url) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to parse {}: {}", entry.url, e);
                continue;
            }
        };

        // Run analyzers
        let headers_vec: Vec<(String, String)> = result.headers.clone();
        let empty_chain: Vec<crawlkit_core::RedirectHop> = vec![];
        let ctx = crawlkit_core::analyzers::AnalysisContext {
            page: &parsed,
            status_code: Some(result.status_code),
            headers: &headers_vec,
            response_time: Some(fetch_time),
            redirect_chain: &empty_chain,
        };
        let findings = analyzer_registry.analyze(&ctx, &crawlkit_core::CrawlConfig::default());

        // Store page
        let page_data = crawlkit_core::storage::PageData {
            id: uuid::Uuid::new_v4().to_string(),
            url: entry.url.clone(),
            final_url: result.final_url.clone(),
            status_code: result.status_code,
            title: parsed.meta.title.clone(),
            description: parsed.meta.description.clone(),
            canonical_url: parsed.meta.canonical.clone(),
            word_count: Some(parsed.word_count),
            load_time_ms: Some(fetch_time.as_millis() as u64),
            body_size: Some(result.body.len()),
            fetched_at: chrono::Utc::now(),
            links: parsed.links.iter().filter_map(|l| url::Url::parse(&l.href).ok()).collect(),
        };

        if let Err(e) = storage.insert_page(&crawl_id, &page_data) {
            tracing::warn!("Failed to store page {}: {}", entry.url, e);
        } else {
            pages_stored += 1;
        }

        // Store findings
        for finding in &findings {
            let issue = crawlkit_core::storage::Issue {
                id: uuid::Uuid::new_v4().to_string(),
                page_id: page_data.id.clone(),
                category: finding.category.clone(),
                severity: finding.severity.clone(),
                code: finding.code.clone(),
                title: finding.title.clone(),
                description: finding.description.clone(),
                element: None,
                recommendation: finding.recommendation.clone(),
            };
            if let Err(e) = storage.insert_issue(&issue) {
                tracing::warn!("Failed to store issue: {}", e);
            }
        }

        // Extract and queue new links
        for link in &parsed.links {
            let link_url = match url::Url::parse(&link.href) {
                Ok(u) => u,
                Err(_) => continue, // Skip invalid URLs
            };
            if visited.contains(&link_url.to_string()) {
                continue;
            }
            let is_internal = link_url.host_str() == entry.url.host_str();
            if !include.is_empty() && !include.iter().any(|p| link.href.contains(p)) {
                continue;
            }
            if exclude.iter().any(|p| link.href.contains(p)) {
                continue;
            }

            let priority = if is_internal {
                Priority::NORMAL
            } else {
                Priority::LOW
            };

            let mut q = queue.lock().await;
            q.push(link_url, entry.depth + 1, priority);
        }
    }

    pb.finish_with_message(format!(
        "Crawl complete: {} pages crawled, {} stored",
        pages_crawled, pages_stored
    ));

    storage.finish_crawl(&crawl_id, pages_crawled, 0)?;

    // Write output
    if format == "json" || format == "all" {
        let json_path = output_dir.join("crawl-results.json");
        let stats = storage.get_stats(&crawl_id)?;
        let sample = serde_json::json!({
            "crawl_id": crawl_id,
            "target_url": url,
            "max_pages": max_pages,
            "pages_crawled": pages_crawled,
            "pages_stored": pages_stored,
            "total_issues": stats.total_issues,
            "status": "completed",
        });
        std::fs::write(&json_path, serde_json::to_string_pretty(&sample)?)?;
        tracing::info!("Wrote results to {}", json_path.display());
    }

    tracing::info!(
        "Crawl complete: {} pages crawled, {} stored. Database: {}",
        pages_crawled,
        pages_stored,
        db_path.display()
    );
    Ok(())
}

/// Compare two crawl results.
async fn run_compare(
    crawl1_path: &Path,
    crawl2_path: &Path,
    output: Option<&Path>,
    format: &str,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Comparing crawls...");

    let storage1_path = if crawl1_path.is_dir() {
        crawl1_path.join("crawlkit.db")
    } else {
        crawl1_path.to_path_buf()
    };
    let storage2_path = if crawl2_path.is_dir() {
        crawl2_path.join("crawlkit.db")
    } else {
        crawl2_path.to_path_buf()
    };

    let _storage1 = Storage::new(&storage1_path)
        .with_context(|| format!("Failed to open first crawl: {}", storage1_path.display()))?;
    let _storage2 = Storage::new(&storage2_path)
        .with_context(|| format!("Failed to open second crawl: {}", storage2_path.display()))?;

    pb.finish_with_message("Comparison not yet implemented");

    let result = serde_json::json!({
        "status": "not_implemented",
        "crawl1": storage1_path.display().to_string(),
        "crawl2": storage2_path.display().to_string(),
        "format": format,
    });

    if let Some(out) = output {
        std::fs::write(out, serde_json::to_string_pretty(&result)?)?;
        tracing::info!("Wrote comparison to {}", out.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}

/// Generate a report from an existing crawl.
async fn run_report(
    crawl_path: &Path,
    output: Option<&Path>,
    format: &str,
    theme: &str,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Generating report...");

    let db_path = if crawl_path.is_dir() {
        crawl_path.join("crawlkit.db")
    } else {
        crawl_path.to_path_buf()
    };

    let storage = Storage::new(&db_path)
        .with_context(|| format!("Failed to open crawl database: {}", db_path.display()))?;

    // For now, list crawl IDs from the database
    // TODO: Implement full report generation
    pb.finish_with_message("Report generation not yet implemented");

    let result = serde_json::json!({
        "status": "not_implemented",
        "crawl_db": db_path.display().to_string(),
        "format": format,
        "theme": theme,
    });

    if let Some(out) = output {
        std::fs::write(out, serde_json::to_string_pretty(&result)?)?;
        tracing::info!("Wrote report to {}", out.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}
