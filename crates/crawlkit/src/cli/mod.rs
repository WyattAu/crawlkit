pub mod backlinks;
pub mod compare;
pub mod crawl;
pub mod gsc;
pub mod inspect;
pub mod log_analyze;
pub mod plugin;
pub mod report;
pub mod trend;
pub mod util;

pub use log_analyze::LogAnalyzeArgs;
pub use plugin::PluginCommands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLI configuration file structure.
#[derive(serde::Deserialize, Default)]
pub struct Config {
    /// Crawl-specific defaults.
    pub crawl: Option<CrawlConfig>,
    /// Output settings.
    pub output: Option<OutputConfig>,
    /// Feature flags.
    pub features: Option<FeaturesConfig>,
}

/// Feature flag configuration loaded from file.
#[derive(serde::Deserialize, Default)]
pub struct FeaturesConfig {
    /// Enable AI analyzers.
    pub ai_analyzers: Option<bool>,
    /// Enable WASM analyzers.
    pub wasm_analyzers: Option<bool>,
    /// Enable JS rendering.
    pub js_rendering: Option<bool>,
    /// Enable backlink analysis.
    pub backlink_analysis: Option<bool>,
}

/// Crawl configuration loaded from file.
#[derive(serde::Deserialize)]
pub struct CrawlConfig {
    /// Default max pages.
    pub max_pages: Option<usize>,
    /// Default request delay in milliseconds.
    pub delay_ms: Option<u64>,
    /// Default concurrency.
    pub concurrency: Option<usize>,
    /// Default request timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Custom user agent.
    pub user_agent: Option<String>,
    /// Whether to respect robots.txt.
    pub respect_robots_txt: Option<bool>,
}

/// Output configuration loaded from file.
#[derive(serde::Deserialize)]
pub struct OutputConfig {
    /// Default output directory.
    pub dir: Option<String>,
    /// Default output format.
    pub format: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "crawlkit",
    about = "A high-performance Rust-based site crawler for SEO analysis",
    version,
    propagate_version = true
)]
pub struct Cli {
    /// Path to configuration file.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Enable verbose output.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
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

        /// Random seed for reproducible crawls
        #[arg(long)]
        seed: Option<u64>,

        /// Enable AI analyzers (default: true)
        #[arg(long, default_value_t = true)]
        enable_ai: bool,

        /// Enable WASM analyzers (default: true)
        #[arg(long, default_value_t = true)]
        enable_wasm: bool,

        /// Enable encryption at rest for stored data
        #[arg(long)]
        encrypt: bool,

        /// Export metrics snapshot to JSON file
        #[arg(long)]
        metrics_json: Option<PathBuf>,

        /// Tenant ID for multi-tenant operations
        #[arg(long)]
        tenant: Option<String>,

        /// Enable incremental crawling using ETag / If-Modified-Since
        #[arg(long)]
        incremental: bool,

        /// Force a full re-crawl, ignoring cached ETag/Last-Modified conditions
        #[arg(long)]
        force: bool,

        /// Directories to load crawl plugins from (repeatable). Omit to
        /// use the default install roots (~/.crawlkit/plugins +
        /// $CRAWLKIT_PLUGIN_DIRS); pass an empty value to disable.
        #[arg(long)]
        plugins: Option<Vec<PathBuf>>,

        /// Enable monitoring mode: compare this crawl against the previous one
        /// and trigger alerts on significant changes.
        #[arg(long)]
        monitor: bool,

        /// Minimum total changes (new + removed + changed + regressions)
        /// required to trigger a monitoring alert. Default: 1 (any change).
        #[arg(long)]
        alert_threshold: Option<usize>,
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
        #[arg(long)]
        format: Option<String>,

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

    /// Manage WASM plugin signing keys and verify plugin trust chains
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },

    /// Analyze web server access logs
    LogAnalyze(LogAnalyzeArgs),

    /// Analyze trends across multiple crawl snapshots
    Trend {
        /// Path to crawl database or directory
        #[arg(short, long)]
        db: PathBuf,

        /// Specific crawl IDs to analyze (auto-discovers all if empty)
        #[arg(long)]
        crawl_ids: Vec<String>,

        /// Output file for the trend report
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: json or md
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Fetch and analyze Google Search Console data
    Gsc {
        /// Site URL to query (overrides GSC_SITE_URL env var)
        #[arg(long)]
        site_url: Option<String>,

        /// Output file for the analysis
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: json or md
        #[arg(long, default_value = "json")]
        format: String,

        /// Start date (YYYY-MM-DD)
        #[arg(long, default_value = "2026-01-01")]
        start_date: String,

        /// End date (YYYY-MM-DD)
        #[arg(long, default_value = "2026-01-31")]
        end_date: String,

        /// Dimension to analyze: query, page, or all
        #[arg(long, default_value = "all")]
        dimension: String,

        /// Maximum number of results
        #[arg(long, default_value = "100")]
        limit: usize,
    },
}

/// Parameters for a crawl operation, bundling all CLI/config values.
pub struct CrawlParams {
    pub url: String,
    pub max_pages: Option<usize>,
    pub max_time_secs: Option<u64>,
    pub delay: Option<u64>,
    pub concurrency: Option<usize>,
    pub output: Option<PathBuf>,
    pub format: String,
    pub depth: Option<usize>,
    pub user_agent: Option<String>,
    pub timeout: Option<u64>,
    pub respect_robots: Option<bool>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub javascript: bool,
    pub allow_external: bool,
    pub seed: Option<u64>,
    pub encrypt: bool,
    pub metrics_json: Option<PathBuf>,
    pub tenant: Option<String>,
    pub incremental: bool,
    pub force: bool,
    pub feature_flags: crawlkit_engine::FeatureFlags,
    /// Run installed plugins during the crawl. `None` = default dirs
    /// (~/.crawlkit/plugins + CRAWLKIT_PLUGIN_DIRS); `Some(dirs)` =
    /// explicit; empty = disabled.
    pub plugins: Option<Vec<PathBuf>>,
    /// Enable monitoring mode (compare against previous crawl).
    pub monitor: bool,
    /// Minimum changes required to trigger a monitoring alert.
    pub alert_threshold: Option<usize>,
}
