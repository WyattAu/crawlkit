//! Command-line interface for crawlkit — a high-performance site crawler for SEO analysis.
//!
//! Provides subcommands to **crawl** a website, **compare** two crawl results,
//! and **generate reports** from stored crawl data. Configuration can be supplied
//! via CLI flags or a TOML config file.

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
    #[allow(dead_code)]
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

        /// Maximum crawl duration in seconds (0 = no limit)
        #[arg(long)]
        max_time: Option<u64>,

        /// Custom user agent string
        #[arg(long)]
        user_agent: Option<String>,

        /// Request timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,

        /// Respect robots.txt directives (default: true)
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

        /// Follow links to external domains (default: only follow same-domain links)
        #[arg(long)]
        allow_external: bool,
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

    /// Analyze backlinks from an existing crawl
    Backlinks {
        /// Crawl database path or directory
        #[arg(short, long)]
        crawl: PathBuf,

        /// Output file for the analysis
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: json or md
        #[arg(long, default_value = "json")]
        format: String,

        /// Fetch external backlinks from a source (ahrefs, gsc, majestic)
        #[arg(long)]
        source: Option<String>,
    },

    /// Deep single-page analysis: fetch + all analyzers + optional CrUX/GSC
    Inspect {
        /// URL to inspect
        url: String,

        /// Output file for the analysis
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: json or md
        #[arg(long, default_value = "json")]
        format: String,

        /// Enable JavaScript rendering (requires Playwright)
        #[arg(long)]
        javascript: bool,

        /// Custom user agent string
        #[arg(long)]
        user_agent: Option<String>,
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

    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_sdk::trace::SdkTracerProvider;
    use tracing_opentelemetry::OpenTelemetryLayer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    let use_otel = std::env::var("OTEL_EXPORTER").ok().as_deref() == Some("stdout");

    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(format!("crawlkit={log_level}"))),
    );

    if use_otel {
        use opentelemetry::trace::TracerProvider;
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
            .build();
        let tracer = provider.tracer("crawlkit");
        let otel_layer = OpenTelemetryLayer::new(tracer);
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(otel_layer)
            .init();
    } else {
        tracing_subscriber::registry().with(fmt_layer).init();
    }

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
            max_time,
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
            allow_external,
        } => {
            let params = CrawlParams {
                url,
                max_pages: max_pages.or_else(|| config.crawl.as_ref().and_then(|c| c.max_pages)),
                max_time_secs: max_time,
                delay: delay.or_else(|| config.crawl.as_ref().and_then(|c| c.delay_ms)),
                concurrency: concurrency
                    .or_else(|| config.crawl.as_ref().and_then(|c| c.concurrency)),
                output: output.or_else(|| {
                    config
                        .output
                        .as_ref()
                        .and_then(|o| o.dir.as_deref().map(PathBuf::from))
                }),
                format,
                depth,
                user_agent: user_agent
                    .or_else(|| config.crawl.as_ref().and_then(|c| c.user_agent.clone())),
                timeout: timeout.or_else(|| config.crawl.as_ref().and_then(|c| c.timeout_secs)),
                respect_robots: respect_robots
                    .or_else(|| config.crawl.as_ref().and_then(|c| c.respect_robots_txt)),
                include,
                exclude,
                javascript,
                allow_external,
            };
            run_crawl(&params).await
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
        Commands::Backlinks {
            crawl,
            output,
            format,
            source,
        } => run_backlinks(&crawl, output.as_deref(), &format, source.as_deref()).await,
        Commands::Inspect {
            url,
            output,
            format,
            javascript,
            user_agent,
        } => {
            run_inspect(
                &url,
                output.as_deref(),
                &format,
                javascript,
                user_agent.as_deref(),
            )
            .await
        }
    }
}

/// Parameters for a crawl operation, bundling all CLI/config values.
struct CrawlParams {
    url: String,
    max_pages: Option<usize>,
    max_time_secs: Option<u64>,
    delay: Option<u64>,
    concurrency: Option<usize>,
    output: Option<PathBuf>,
    format: String,
    depth: Option<usize>,
    user_agent: Option<String>,
    timeout: Option<u64>,
    respect_robots: Option<bool>,
    include: Vec<String>,
    exclude: Vec<String>,
    javascript: bool,
    allow_external: bool,
}

/// Execute a crawl with the given parameters.
async fn run_crawl(params: &CrawlParams) -> Result<()> {
    use crawlkit_core::analyzers::AnalyzerRegistry;
    use crawlkit_core::http::HttpClient;
    use crawlkit_core::queue::{Priority, UrlQueue};
    use crawlkit_core::ratelimit::RateLimiter;
    use crawlkit_core::HtmlParser;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;

    let max_pages = params.max_pages.unwrap_or(100);
    let delay = params.delay.unwrap_or(500);
    let concurrency = params.concurrency.unwrap_or(4);
    let timeout_secs = params.timeout.unwrap_or(30);

    tracing::info!(
        "Starting crawl of {} (max_pages={}, delay={}ms, concurrency={}, depth={:?}, js={}, allow_external={})",
        params.url,
        max_pages,
        delay,
        concurrency,
        params.depth,
        params.javascript,
        params.allow_external,
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
    let output_dir = params.output.clone().unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;
    let db_path = output_dir.join("crawlkit.db");
    let storage = Storage::new(&db_path)
        .with_context(|| format!("Failed to open storage at {}", db_path.display()))?;

    let crawl_id = storage.start_crawl(&params.url, None)?;
    tracing::info!("Crawl ID: {}", crawl_id);

    // Initialize components
    let http_config = crawlkit_core::http::HttpClientConfig {
        timeout: std::time::Duration::from_secs(timeout_secs),
        max_redirects: 20,
        retry_policy: crawlkit_core::http::RetryPolicy::default(),
        user_agent: std::sync::Arc::new(crawlkit_core::http::UserAgentRotator::new(vec![params
            .user_agent
            .clone()
            .unwrap_or_else(|| "crawlkit/0.1.0".to_string())])),
        max_body_size: 10 * 1024 * 1024,
        pool_max_idle_per_host: 32,
        pool_max_idle: 64,
        http2_prior_knowledge: false,
        tcp_keepalive: Some(std::time::Duration::from_secs(30)),
    };
    let client = HttpClient::new(http_config).context("Failed to create HTTP client")?;
    let client = Arc::new(client);
    let scope = crawlkit_core::queue::ScopeConfig {
        max_depth: params.depth,
        ..Default::default()
    };
    let queue = Arc::new(Mutex::new(UrlQueue::new(scope)));
    let rate_limiter = RateLimiter::new(concurrency as f64, 1.0 / (delay as f64 / 1000.0));
    let crawl_config = crawlkit_core::CrawlConfig {
        respect_robots_txt: params.respect_robots.unwrap_or(true),
        max_time: params.max_time_secs.map(Duration::from_secs),
        max_depth: params.depth,
        ..Default::default()
    };
    let analyzer_registry = AnalyzerRegistry::new(&crawl_config);
    let robots_cache = Arc::new(crawlkit_core::RobotsTxtCache::new(
        client.clone(),
        &crawl_config,
    ));
    let sitemap_cache = Arc::new(crawlkit_core::SitemapCache::new(client.clone()));

    // Seed the queue
    let seed_url =
        url::Url::parse(&params.url).with_context(|| format!("Invalid URL: {}", params.url))?;
    {
        let q = queue.lock().await;
        q.push(seed_url.clone(), 0, Priority::HIGH);
    }

    // Discover and queue sitemap URLs for the seed domain
    let mut known_sitemap_urls: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    {
        let sitemap_urls = robots_cache
            .sitemaps(seed_url.scheme(), seed_url.host_str().unwrap_or(""))
            .await;
        if !sitemap_urls.is_empty() {
            tracing::info!("Found {} sitemap URLs in robots.txt", sitemap_urls.len());
            let entries = sitemap_cache.fetch_all(&sitemap_urls).await;
            tracing::info!("Parsed {} URLs from sitemaps", entries.len());
            let q = queue.lock().await;
            for entry in &entries {
                if known_sitemap_urls.insert(entry.url.clone()) {
                    if let Ok(url) = url::Url::parse(&entry.url) {
                        q.push(url, 0, Priority::HIGHEST);
                    }
                }
            }
        }
    }

    let mut pages_crawled = 0;
    let mut pages_stored = 0;
    let mut issues_found: usize = 0;
    let mut skipped_external: usize = 0;
    let mut skipped_robots: usize = 0;
    let mut skipped_duplicate: usize = 0;
    let mut visited = std::collections::HashSet::new();
    let mut content_hashes: std::collections::HashSet<String> = std::collections::HashSet::new();
    let crawl_start = std::time::Instant::now();
    let seed_domain = seed_url.host_str().unwrap_or("").to_string();

    // Crawl loop
    while pages_crawled < max_pages {
        // Check time budget
        if let Some(max_time) = crawl_config.max_time {
            if crawl_start.elapsed() >= max_time {
                tracing::info!("Crawl time limit reached: {max_time:?}");
                break;
            }
        }

        // Check memory budget (every 100 pages)
        if pages_crawled % 100 == 0 && pages_crawled > 0 {
            if let Ok(rss_bytes) = get_process_rss_bytes() {
                let limit = 512 * 1024 * 1024; // 512 MB default
                if rss_bytes > limit {
                    tracing::warn!(
                        "Memory limit reached: {}MB > {}MB limit. Stopping crawl.",
                        rss_bytes / (1024 * 1024),
                        limit / (1024 * 1024)
                    );
                    break;
                }
            }
        }

        let entry = {
            let q = queue.lock().await;
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

        // Robots.txt check
        let robots_raw;
        if crawl_config.respect_robots_txt {
            let domain = entry.url.host_str().unwrap_or("");
            let scheme = entry.url.scheme();
            if robots_cache
                .is_disallowed(scheme, domain, entry.url.path())
                .await
            {
                tracing::debug!("Blocked by robots.txt: {}", entry.url);
                skipped_robots += 1;
                continue;
            }
            // Apply crawl-delay from robots.txt
            if let Some(delay_secs) = robots_cache.crawl_delay(scheme, domain).await {
                rate_limiter.set_crawl_delay(domain, Duration::from_secs_f64(delay_secs));
            }
            robots_raw = robots_cache.raw_content(scheme, domain).await;

            // Discover sitemaps for this domain (first visit only)
            if !domain.is_empty() && !known_sitemap_urls.contains(&format!("{scheme}://{domain}")) {
                known_sitemap_urls.insert(format!("{scheme}://{domain}"));
                let sitemap_urls = robots_cache.sitemaps(scheme, domain).await;
                if !sitemap_urls.is_empty() {
                    let entries = sitemap_cache.fetch_all(&sitemap_urls).await;
                    let q = queue.lock().await;
                    for sm_entry in &entries {
                        if let Ok(url) = url::Url::parse(&sm_entry.url) {
                            q.push(url, 0, Priority::HIGHEST);
                        }
                    }
                }
            }
        } else {
            robots_raw = String::new();
        }

        // Rate limit
        let _ = rate_limiter
            .acquire(entry.url.host_str().unwrap_or(""))
            .await;

        // Update progress with queue size
        let queue_len = {
            let q = queue.lock().await;
            q.len()
        };
        pb.set_message(format!("[q={queue_len}] Crawling: {}", entry.url));

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

        // Content-hash deduplication: skip pages with identical body content
        {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(result.body.as_bytes());
            let result = hasher.finalize();
            let hash: String = result.iter().map(|b| format!("{b:02x}")).collect();
            if !content_hashes.insert(hash) {
                tracing::debug!("Skipping duplicate content: {}", entry.url);
                skipped_duplicate += 1;
                continue;
            }
        }

        pages_crawled += 1;
        pb.set_position(pages_crawled as u64);
        pb.set_message(format!(
            "[q={queue_len}] Fetched: {} (issues: {issues_found})",
            entry.url
        ));

        // Parse HTML
        let body_text = result.body.clone();
        let _parser = HtmlParser;
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
        let robots_ref = if robots_raw.is_empty() {
            None
        } else {
            Some(robots_raw.as_str())
        };
        let ctx = crawlkit_core::analyzers::AnalysisContext {
            page: &parsed,
            status_code: Some(result.status_code),
            headers: &headers_vec,
            response_time: Some(fetch_time),
            redirect_chain: &empty_chain,
            robots_txt: robots_ref,
        };
        let findings = analyzer_registry.analyze(&ctx, &crawl_config);

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
            links: parsed
                .links
                .iter()
                .filter_map(|l| url::Url::parse(&l.href).ok())
                .collect(),
        };

        if let Err(e) = storage.insert_page(&crawl_id, &page_data) {
            tracing::warn!("Failed to store page {}: {}", entry.url, e);
        } else {
            pages_stored += 1;
        }

        // Store findings
        issues_found += findings.len();
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
            let is_internal = link_url.host_str() == Some(&seed_domain);

            // Enforce domain filtering
            if !is_internal && !params.allow_external {
                skipped_external += 1;
                tracing::debug!(
                    "Skipping external link: {} (host={})",
                    link_url,
                    link_url.host_str().unwrap_or("<none>")
                );
                continue;
            }

            if !params.include.is_empty() && !params.include.iter().any(|p| link.href.contains(p)) {
                continue;
            }
            if params.exclude.iter().any(|p| link.href.contains(p)) {
                continue;
            }

            let priority = if is_internal {
                Priority::NORMAL
            } else {
                Priority::LOW
            };

            // Enforce depth budget
            if let Some(max_depth) = crawl_config.max_depth {
                if entry.depth + 1 > max_depth {
                    continue;
                }
            }

            let q = queue.lock().await;
            q.push(link_url, entry.depth + 1, priority);
        }
    }

    pb.finish_with_message(format!(
        "Crawl complete: {} pages crawled, {} stored, {} issues, {} external skipped, {} blocked by robots.txt, {} duplicate content",
        pages_crawled, pages_stored, issues_found, skipped_external, skipped_robots, skipped_duplicate
    ));

    storage.finish_crawl(&crawl_id, pages_crawled, 0)?;

    // Write output
    if params.format == "json" || params.format == "all" {
        let json_path = output_dir.join("crawl-results.json");
        let stats = storage.get_stats(&crawl_id)?;
        let sample = serde_json::json!({
            "crawl_id": crawl_id,
            "target_url": params.url,
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
        "Crawl complete: {} pages crawled, {} stored, {} issues, {} external skipped, {} blocked by robots.txt. Database: {}",
        pages_crawled,
        pages_stored,
        issues_found,
        skipped_external,
        skipped_robots,
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

    // Validate both databases exist
    if !storage1_path.exists() {
        return Err(anyhow::anyhow!(
            "First crawl database not found: {}",
            storage1_path.display()
        ));
    }
    if !storage2_path.exists() {
        return Err(anyhow::anyhow!(
            "Second crawl database not found: {}",
            storage2_path.display()
        ));
    }

    let diff = crawlkit_core::compare::compare_crawls(&storage1_path, &storage2_path)
        .context("Failed to compare crawls")?;

    let output_str = match format {
        "json" => crawlkit_core::compare::diff_to_json(&diff, true)
            .context("Failed to serialize comparison")?,
        "md" => crawlkit_core::compare::diff_to_markdown(&diff),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format: {format}. Use json or md."
            ));
        }
    };

    pb.finish_with_message(format!(
        "Comparison: {} added, {} removed, {} status changes, {} title changes, {} content changes",
        diff.added.len(),
        diff.removed.len(),
        diff.status_changes.len(),
        diff.title_changes.len(),
        diff.content_changes.len(),
    ));

    if let Some(out) = output {
        std::fs::write(out, &output_str)?;
        tracing::info!("Wrote comparison to {}", out.display());
    } else {
        println!("{output_str}");
    }

    Ok(())
}

/// Generate a report from an existing crawl.
async fn run_report(
    crawl_path: &Path,
    output: Option<&Path>,
    format: &str,
    _theme: &str,
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

    // Get crawl statistics
    let crawl_id = storage
        .get_latest_crawl_id()
        .context("Failed to get latest crawl ID")?
        .ok_or_else(|| anyhow::anyhow!("No crawls found in database"))?;

    let stats = storage
        .get_stats(&crawl_id)
        .context("Failed to get crawl statistics")?;

    // Run backlink analysis
    let backlink_data = {
        let link_pairs = storage.get_links_for_crawl(&crawl_id).unwrap_or_default();
        let external_links = storage.get_external_links(&crawl_id).unwrap_or_default();
        let page_urls = storage.get_page_urls(&crawl_id).unwrap_or_default();

        let mut analyzer = crawlkit_core::BacklinkAnalyzer::new();
        analyzer.load_from_crawl_data(&link_pairs);

        // Add external backlinks
        for (source, target) in &external_links {
            analyzer.add_backlink(crawlkit_core::Backlink {
                source_url: source.clone(),
                target_url: target.clone(),
                anchor_text: String::new(),
                is_followed: true,
                is_internal: false,
            });
        }

        let _known_urls: std::collections::HashSet<String> = page_urls.into_iter().collect();
        let pagerank = analyzer.compute_pagerank(0.85, 20);
        let summary = analyzer.summarize();

        Some((summary, pagerank))
    };

    pb.set_message("Generating report...");

    // Generate report based on format
    let report = match format {
        "json" => {
            let mut data = serde_json::json!({
                "crawl_id": crawl_id,
                "total_pages": stats.total_pages,
                "total_issues": stats.total_issues,
                "issues_by_severity": stats.issues_by_severity,
                "issues_by_category": stats.issues_by_category,
                "avg_response_time_ms": stats.avg_response_time_ms,
                "total_body_size": stats.total_body_size,
                "status": "completed",
            });

            if let Some((summary, _pagerank)) = &backlink_data {
                let top_pages: Vec<serde_json::Value> = summary
                    .pages
                    .iter()
                    .take(20)
                    .map(|p| {
                        serde_json::json!({
                            "url": p.url,
                            "pagerank": p.pagerank,
                            "inbound_links": p.inbound_links,
                            "outbound_links": p.outbound_links,
                            "referring_domains": p.referring_domains,
                        })
                    })
                    .collect();

                data["backlinks"] = serde_json::json!({
                    "total_internal_links": summary.total_internal_links,
                    "total_external_links": summary.total_external_links,
                    "total_referring_domains": summary.total_referring_domains,
                    "orphan_pages": summary.orphan_pages,
                    "top_pages_by_pagerank": top_pages,
                });
            }

            serde_json::to_string_pretty(&data)?
        }
        "markdown" => {
            let mut md = format!(
                "# Crawl Report\n\n\
                - **Crawl ID:** {}\n\
                - **Total Pages:** {}\n\
                - **Total Issues:** {}\n\
                - **Avg Response Time:** {:.2}ms\n\
                - **Total Body Size:** {} bytes\n\
                - **Status:** Completed\n",
                crawl_id,
                stats.total_pages,
                stats.total_issues,
                stats.avg_response_time_ms.unwrap_or(0.0),
                stats.total_body_size.unwrap_or(0)
            );

            if let Some((summary, _)) = &backlink_data {
                md.push_str(&format!(
                    "\n## Backlinks\n\n\
                    - **Total Internal Links:** {}\n\
                    - **Total External Links:** {}\n\
                    - **Total Referring Domains:** {}\n\
                    - **Orphan Pages:** {}\n",
                    summary.total_internal_links,
                    summary.total_external_links,
                    summary.total_referring_domains,
                    summary.orphan_pages.len()
                ));

                if !summary.orphan_pages.is_empty() {
                    md.push_str("\n### Orphan Pages\n\n");
                    for url in summary.orphan_pages.iter().take(10) {
                        md.push_str(&format!("- {url}\n"));
                    }
                    if summary.orphan_pages.len() > 10 {
                        md.push_str(&format!(
                            "\n... and {} more\n",
                            summary.orphan_pages.len() - 10
                        ));
                    }
                }

                md.push_str("\n### Top Pages by PageRank\n\n");
                md.push_str("| URL | PageRank | Inbound | Outbound | Referring Domains |\n");
                md.push_str("|-----|----------|---------|----------|------------------|\n");
                for page in summary.pages.iter().take(10) {
                    md.push_str(&format!(
                        "| {} | {:.4} | {} | {} | {} |\n",
                        page.url,
                        page.pagerank,
                        page.inbound_links,
                        page.outbound_links,
                        page.referring_domains
                    ));
                }
            }

            md
        }
        "csv" => {
            let mut csv = String::from("crawl_id,total_pages,total_issues,status\n");
            csv.push_str(&format!(
                "{},{},{},completed\n",
                crawl_id, stats.total_pages, stats.total_issues
            ));
            csv
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported format: {}", format));
        }
    };

    pb.finish_with_message("Report generated");

    if let Some(out) = output {
        std::fs::write(out, &report)?;
        tracing::info!("Wrote report to {}", out.display());
    } else {
        println!("{}", report);
    }

    Ok(())
}

/// Analyze backlinks from an existing crawl.
async fn run_backlinks(
    crawl_path: &Path,
    output: Option<&Path>,
    format: &str,
    source: Option<&str>,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Analyzing backlinks...");

    let db_path = if crawl_path.is_dir() {
        crawl_path.join("crawlkit.db")
    } else {
        crawl_path.to_path_buf()
    };

    let storage = Storage::new(&db_path)
        .with_context(|| format!("Failed to open crawl database: {}", db_path.display()))?;

    let crawl_id = storage
        .get_latest_crawl_id()
        .context("Failed to get latest crawl ID")?
        .ok_or_else(|| anyhow::anyhow!("No crawls found in database"))?;

    // Load link data from storage
    let link_pairs = storage.get_links_for_crawl(&crawl_id)?;
    let external_links = storage.get_external_links(&crawl_id)?;

    // Build backlink analyzer
    let mut analyzer = crawlkit_core::BacklinkAnalyzer::new();
    analyzer.load_from_crawl_data(&link_pairs);
    for (source_url, target_url) in &external_links {
        analyzer.add_backlink(crawlkit_core::Backlink {
            source_url: source_url.clone(),
            target_url: target_url.clone(),
            anchor_text: String::new(),
            is_followed: true,
            is_internal: false,
        });
    }

    // Optionally fetch external backlinks from API
    if let Some(src) = source {
        pb.set_message(format!("Fetching external backlinks from {src}..."));
        let registry = crawlkit_core::BacklinkAdapterRegistry::with_defaults();
        if let Some(adapter) = registry.get(src) {
            let urls = storage.get_page_urls(&crawl_id)?;
            if let Some(first_url) = urls.first() {
                let domain = url::Url::parse(first_url)
                    .ok()
                    .and_then(|u| u.host_str().map(String::from))
                    .unwrap_or_default();
                match adapter.fetch_backlinks(&domain, 1000).await {
                    Ok(ext_backlinks) => {
                        for bl in &ext_backlinks {
                            analyzer.add_backlink(crawlkit_core::Backlink {
                                source_url: bl.source_url.clone(),
                                target_url: bl.target_url.clone(),
                                anchor_text: bl.anchor_text.clone(),
                                is_followed: bl.is_followed,
                                is_internal: false,
                            });
                        }
                        pb.set_message(format!(
                            "Fetched {} external backlinks",
                            ext_backlinks.len()
                        ));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch from {src}: {e}");
                    }
                }
            }
        } else {
            tracing::warn!("Unknown backlink source: {src}. Available: ahrefs, gsc, majestic");
        }
    }

    let _pagerank = analyzer.compute_pagerank(0.85, 20);
    let summary = analyzer.summarize();

    let output_str = match format {
        "json" => {
            let top_pages: Vec<serde_json::Value> = summary
                .pages
                .iter()
                .take(20)
                .map(|p| {
                    serde_json::json!({
                        "url": p.url,
                        "pagerank": p.pagerank,
                        "inbound_links": p.inbound_links,
                        "outbound_links": p.outbound_links,
                        "referring_domains": p.referring_domains,
                    })
                })
                .collect();

            serde_json::to_string_pretty(&serde_json::json!({
                "crawl_id": crawl_id,
                "total_internal_links": summary.total_internal_links,
                "total_external_links": summary.total_external_links,
                "total_referring_domains": summary.total_referring_domains,
                "orphan_pages": summary.orphan_pages,
                "orphan_count": summary.orphan_pages.len(),
                "top_pages_by_pagerank": top_pages,
            }))?
        }
        "md" => {
            let mut md = format!(
                "# Backlink Analysis\n\n\
                - **Crawl ID:** {crawl_id}\n\
                - **Total Internal Links:** {}\n\
                - **Total External Links:** {}\n\
                - **Total Referring Domains:** {}\n\
                - **Orphan Pages:** {}\n",
                summary.total_internal_links,
                summary.total_external_links,
                summary.total_referring_domains,
                summary.orphan_pages.len()
            );

            if !summary.orphan_pages.is_empty() {
                md.push_str("\n## Orphan Pages\n\n");
                for url in &summary.orphan_pages {
                    md.push_str(&format!("- {url}\n"));
                }
            }

            md.push_str("\n## Top Pages by PageRank\n\n");
            md.push_str("| URL | PageRank | Inbound | Outbound | Referring Domains |\n");
            md.push_str("|-----|----------|---------|----------|------------------|\n");
            for page in summary.pages.iter().take(20) {
                md.push_str(&format!(
                    "| {} | {:.4} | {} | {} | {} |\n",
                    page.url,
                    page.pagerank,
                    page.inbound_links,
                    page.outbound_links,
                    page.referring_domains
                ));
            }

            md
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format: {format}. Use json or md."
            ))
        }
    };

    pb.finish_with_message(format!(
        "Analysis complete: {} internal links, {} external links, {} orphan pages",
        summary.total_internal_links,
        summary.total_external_links,
        summary.orphan_pages.len()
    ));

    if let Some(out) = output {
        std::fs::write(out, &output_str)?;
        tracing::info!("Wrote backlink analysis to {}", out.display());
    } else {
        println!("{output_str}");
    }

    Ok(())
}

/// Deep single-page analysis: fetch, parse, run all analyzers, optional CrUX/GSC.
async fn run_inspect(
    url_str: &str,
    output: Option<&Path>,
    format: &str,
    _javascript: bool,
    user_agent: Option<&str>,
) -> Result<()> {
    use crawlkit_core::analyzers::AnalyzerRegistry;
    use crawlkit_core::http::HttpClient;
    use crawlkit_core::HtmlParser;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    let url = url::Url::parse(url_str).with_context(|| format!("Invalid URL: {url_str}"))?;

    // Initialize components
    let default_ua = format!("crawlkit/{}", env!("CARGO_PKG_VERSION"));
    let ua = user_agent.unwrap_or(&default_ua);
    let http_config = crawlkit_core::http::HttpClientConfig {
        timeout: std::time::Duration::from_secs(30),
        max_redirects: 20,
        retry_policy: crawlkit_core::http::RetryPolicy::default(),
        user_agent: std::sync::Arc::new(crawlkit_core::http::UserAgentRotator::new(vec![
            ua.to_string()
        ])),
        max_body_size: 10 * 1024 * 1024,
        pool_max_idle_per_host: 32,
        pool_max_idle: 64,
        http2_prior_knowledge: false,
        tcp_keepalive: Some(std::time::Duration::from_secs(30)),
    };
    let client = HttpClient::new(http_config).context("Failed to create HTTP client")?;
    let config = crawlkit_core::CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);

    // Fetch
    pb.set_message(format!("Fetching {url_str}..."));
    let start = std::time::Instant::now();
    let result = client.fetch(&url).await.context("Failed to fetch URL")?;
    let fetch_time = start.elapsed();

    // Parse
    pb.set_message("Parsing HTML...");
    let parsed = HtmlParser::parse(&result.body, &url).context("Failed to parse HTML")?;

    // Run all analyzers
    pb.set_message("Running 28 analyzers...");
    let headers_vec: Vec<(String, String)> = result.headers.clone();
    let empty_chain: Vec<crawlkit_core::RedirectHop> = vec![];
    let ctx = crawlkit_core::analyzers::AnalysisContext {
        page: &parsed,
        status_code: Some(result.status_code),
        headers: &headers_vec,
        response_time: Some(fetch_time),
        redirect_chain: &empty_chain,
        robots_txt: None,
    };
    let findings = registry.analyze(&ctx, &config);

    // Fetch CrUX data from PageSpeed Insights if API key is available
    let crux_data = {
        let adapter = crawlkit_core::CruxAdapter::from_env();
        if adapter.is_available() {
            pb.set_message("Fetching CrUX data from PageSpeed Insights...");
            adapter.fetch_crux_data(url_str).await.ok().flatten()
        } else {
            None
        }
    };

    // Build report
    let issues_by_severity: std::collections::HashMap<String, usize> = {
        let mut map = std::collections::HashMap::new();
        for f in &findings {
            *map.entry(f.severity.as_str().to_string()).or_insert(0) += 1;
        }
        map
    };
    let issues_by_category: std::collections::HashMap<String, usize> = {
        let mut map = std::collections::HashMap::new();
        for f in &findings {
            *map.entry(f.category.as_str().to_string()).or_insert(0) += 1;
        }
        map
    };

    let output_str = match format {
        "json" => {
            let issues: Vec<serde_json::Value> = findings
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "severity": f.severity.as_str(),
                        "category": f.category.as_str(),
                        "code": f.code,
                        "title": f.title,
                        "description": f.description,
                        "recommendation": f.recommendation,
                    })
                })
                .collect();

            serde_json::to_string_pretty(&serde_json::json!({
                "url": url_str,
                "status_code": result.status_code,
                "final_url": result.final_url.to_string(),
                "fetch_time_ms": fetch_time.as_millis(),
                "body_size": result.body.len(),
                "title": parsed.meta.title,
                "description": parsed.meta.description,
                "canonical": parsed.meta.canonical,
                "word_count": parsed.word_count,
                "links": parsed.links.len(),
                "images": parsed.images.len(),
                "headings": parsed.headings.len(),
                "findings_count": findings.len(),
                "issues_by_severity": issues_by_severity,
                "issues_by_category": issues_by_category,
                "findings": issues,
                "crux": crux_data.as_ref().map(|d| serde_json::json!({
                    "lcp_p75_ms": d.lcp_p75,
                    "inp_p75_ms": d.inp_p75,
                    "cls_p75": d.cls_p75,
                    "fcp_p75_ms": d.fcp_p75,
                    "ttfb_p75_ms": d.ttfb_p75,
                })),
            }))?
        }
        "md" => {
            let mut md = format!(
                "# Page Inspection: {url_str}\n\n\
                - **Status:** {}\n\
                - **Final URL:** {}\n\
                - **Fetch Time:** {:.0}ms\n\
                - **Body Size:** {} bytes\n\
                - **Title:** {}\n\
                - **Description:** {}\n\
                - **Canonical:** {}\n\
                - **Word Count:** {}\n\
                - **Links:** {} internal/external\n\
                - **Images:** {}\n\
                - **Headings:** {}\n",
                result.status_code,
                result.final_url,
                fetch_time.as_millis(),
                result.body.len(),
                parsed.meta.title.as_deref().unwrap_or("(none)"),
                parsed.meta.description.as_deref().unwrap_or("(none)"),
                parsed
                    .meta
                    .canonical
                    .as_ref()
                    .map(|u| u.as_str())
                    .unwrap_or("(none)"),
                parsed.word_count,
                parsed.links.len(),
                parsed.images.len(),
                parsed.headings.len(),
            );

            md.push_str(&format!("\n## Findings ({} total)\n\n", findings.len()));

            if !findings.is_empty() {
                md.push_str("| Severity | Code | Title |\n");
                md.push_str("|----------|------|-------|\n");
                for f in &findings {
                    md.push_str(&format!(
                        "| {} | {} | {} |\n",
                        f.severity.as_str(),
                        f.code,
                        f.title
                    ));
                }
            } else {
                md.push_str("No issues found.\n");
            }

            if let Some(ref d) = crux_data {
                md.push_str("\n## Core Web Vitals (CrUX p75)\n\n");
                md.push_str("| Metric | Value |\n|---|---|\n");
                if let Some(v) = d.lcp_p75 {
                    md.push_str(&format!("| LCP | {v:.0}ms |\n"));
                }
                if let Some(v) = d.cls_p75 {
                    md.push_str(&format!("| CLS | {v:.3} |\n"));
                }
                if let Some(v) = d.inp_p75 {
                    md.push_str(&format!("| INP | {v:.0}ms |\n"));
                }
                if let Some(v) = d.fcp_p75 {
                    md.push_str(&format!("| FCP | {v:.0}ms |\n"));
                }
                if let Some(v) = d.ttfb_p75 {
                    md.push_str(&format!("| TTFB | {v:.0}ms |\n"));
                }
            }

            md
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format: {format}. Use json or md."
            ))
        }
    };

    pb.finish_with_message(format!(
        "Inspection complete: {} findings ({} errors, {} warnings)",
        findings.len(),
        issues_by_severity.get("Error").unwrap_or(&0),
        issues_by_severity.get("Warning").unwrap_or(&0),
    ));

    if let Some(out) = output {
        std::fs::write(out, &output_str)?;
        tracing::info!("Wrote inspection to {}", out.display());
    } else {
        println!("{output_str}");
    }

    Ok(())
}

/// Get the current process RSS (Resident Set Size) in bytes.
///
/// Uses `/proc/self/statm` on Linux, returns `Err` on unsupported platforms.
fn get_process_rss_bytes() -> Result<u64> {
    #[cfg(target_os = "linux")]
    {
        let statm = std::fs::read_to_string("/proc/self/statm")?;
        let fields: Vec<&str> = statm.split_whitespace().collect();
        if fields.len() >= 2 {
            let pages: u64 = fields[1].parse()?;
            let page_size = 4096u64; // standard page size
            return Ok(pages * page_size);
        }
    }
    Err(anyhow::anyhow!("RSS not available on this platform"))
}
