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
    let max_pages = max_pages.unwrap_or(100);
    let delay = delay.unwrap_or(500);
    let concurrency = concurrency.unwrap_or(4);

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

    // TODO: Implement actual crawl loop (Phase 0 tasks 0.4–0.9)
    pb.set_message("Crawl engine not yet implemented");
    pb.finish_with_message("Crawl not yet implemented — Phase 0 foundation in progress");

    // Write output format marker
    if format == "json" || format == "all" {
        let json_path = output_dir.join("crawl-results.json");
        let sample = serde_json::json!({
            "crawl_id": crawl_id,
            "target_url": url,
            "max_pages": max_pages,
            "delay_ms": delay,
            "concurrency": concurrency,
            "status": "not_implemented",
            "format": format,
        });
        std::fs::write(&json_path, serde_json::to_string_pretty(&sample)?)?;
        tracing::info!("Wrote results to {}", json_path.display());
    }

    tracing::info!("Crawl complete. Database at: {}", db_path.display());
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
