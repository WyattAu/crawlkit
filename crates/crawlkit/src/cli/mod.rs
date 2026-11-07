#[cfg(feature = "full")]
pub mod backlinks;
#[cfg(feature = "full")]
pub mod clone;
#[cfg(feature = "full")]
pub mod compare;
#[cfg(feature = "full")]
pub mod crawl;
#[cfg(feature = "full")]
pub mod crawl_map;
#[cfg(feature = "warehouse")]
pub mod export_warehouse;
#[cfg(feature = "full")]
pub(crate) mod findings;
#[cfg(feature = "full")]
pub mod gsc;
#[cfg(feature = "full")]
pub mod history;
#[cfg(feature = "full")]
pub mod inspect;
#[cfg(feature = "full")]
pub mod lighthouse;
pub mod log_analyze;
#[cfg(feature = "full")]
pub mod md;
#[cfg(feature = "full")]
pub mod plugin;
#[cfg(feature = "queue-ops")]
pub mod queue;
#[cfg(feature = "full")]
pub mod rank;
#[cfg(feature = "full")]
pub mod report;
#[cfg(feature = "full")]
pub mod schedule;
/// Shared polite fetch loop behind `clone` and `md` (GAP G8).
#[cfg(feature = "full")]
pub(crate) mod site_fetch;
#[cfg(feature = "full")]
pub mod trend;
/// `crawlkit usage` — ADR-017 read-only usage mirror.
#[cfg(feature = "full")]
pub mod usage;
#[cfg(feature = "full")]
pub mod util;

pub use log_analyze::LogAnalyzeArgs;
#[cfg(feature = "full")]
pub use plugin::PluginCommands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLI configuration file structure.
#[cfg(feature = "full")]
#[derive(serde::Deserialize, Default)]
pub struct Config {
    /// Crawl-specific defaults.
    pub crawl: Option<CrawlConfig>,
    /// Output settings.
    pub output: Option<OutputConfig>,
    /// Feature flags.
    pub features: Option<FeaturesConfig>,
    /// §G3 alerting defaults (`[alerts]` section). CLI `--alert-*` flags
    /// override these.
    pub alerts: Option<AlertsConfig>,
}

/// §G3 alerting configuration (`[alerts]` TOML section).
#[cfg(feature = "full")]
#[derive(serde::Deserialize, Default, Clone)]
pub struct AlertsConfig {
    /// Signed webhook URL (generic, Slack, or Discord endpoint).
    pub webhook_url: Option<String>,
    /// Hex-encoded HMAC-SHA256 secret for the `X-Crawlkit-Signature`
    /// header. Optional for adapter URLs that embed a credential.
    pub secret_hex: Option<String>,
    /// Payload formatting: `generic`, `slack`, or `discord`.
    pub format: Option<String>,
    /// Email recipient (requires the `smtp` feature).
    pub email_to: Option<String>,
    /// SMTP relay host (`host` or `host:port`).
    pub smtp_server: Option<String>,
}

/// Feature flag configuration loaded from file.
#[cfg(feature = "full")]
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
#[cfg(feature = "full")]
#[derive(serde::Deserialize)]
pub struct CrawlConfig {
    /// Analyzer registry profile: `full`, `core`, `standard`, or `deep`.
    pub analyzer_profile: Option<String>,
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
    /// Default per-crawl render quota (pages).
    pub render_max: Option<u64>,
    /// Default per-page render ceiling in seconds.
    pub render_timeout_secs: Option<u64>,
}

/// Output configuration loaded from file.
#[cfg(feature = "full")]
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
    #[cfg(feature = "full")]
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

        /// Allow crawling private/internal IPs (localhost, RFC 1918)
        /// For local development and testing only
        #[arg(long)]
        allow_private: bool,

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

        /// Enable LLM-powered analysis (requires API key in env)
        #[arg(long)]
        llm: bool,

        /// Analyzer profile: full, core, standard, or deep (default: full)
        #[arg(long, default_value = "full")]
        analyzer_profile: String,

        /// Keep page measurements (readability indices, keyword extraction,
        /// entity detection, composite scores) in the issue output.
        ///
        /// They are always stored, but excluded from issues, reports and
        /// aggregates by default: on a 100-page crawl they were 39% of all rows,
        /// which made every issue count and severity roll-up unreadable.
        #[arg(long)]
        include_metrics: bool,

        /// Maximum pages this crawl may render with JavaScript. When the
        /// quota is exhausted, remaining pages are analyzed statically and
        /// carry an explicit RENDER001 degradation finding (default:
        /// unbounded).
        #[arg(long)]
        render_max: Option<u64>,

        /// Wall-clock ceiling for rendering one page in seconds. Exceeding
        /// it produces an explicit RENDER002 degradation finding for that
        /// page instead of the engine default timeout.
        #[arg(long)]
        render_timeout: Option<u64>,
    },

    /// Compare two crawl results
    #[cfg(feature = "full")]
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

        /// §G3: fire a signed `regression` webhook event when the diff
        /// contains new errors. Overrides `[alerts].webhook_url` from the
        /// config file.
        #[arg(long)]
        alert_webhook: Option<String>,

        /// Hex-encoded HMAC-SHA256 secret for the `X-Crawlkit-Signature`
        /// header. Overrides `[alerts].secret_hex`.
        #[arg(long)]
        alert_secret: Option<String>,

        /// Webhook payload formatting: generic, slack, or discord
        #[arg(long, default_value = "generic")]
        alert_format: String,

        /// Also email the alert (requires building with `--features smtp`)
        #[arg(long)]
        alert_email: Option<String>,
    },

    /// Generate a report from an existing crawl
    ///
    /// With `--ci` the report doubles as a CI quality gate (GAP G4): after
    /// the report renders, a one-line summary is printed and the process
    /// exits with:
    ///
    /// - `0` — pass
    /// - `10` — more errors than `--max-errors` (default 0), or new errors
    ///   versus `--baseline`
    /// - `11` — more warnings than `--max-warnings` (default unbounded)
    /// - `12` — score below `--min-score` (§G2 site score over the crawl's
    ///   findings)
    ///
    /// Critical-severity findings count as errors.
    ///
    /// `--section` (GAP G9) limits the report to the named sections, given
    /// repeatable or comma-separated: `summary`, `backlinks`, `previews`,
    /// `scores`, or `all`. The default — every section that predates G9
    /// (summary + backlinks) — keeps output byte-identical with earlier
    /// versions. `previews` renders per-platform social share mock frames
    /// (Google, Facebook, X, LinkedIn, iMessage) from the stored page meta;
    /// styled frames appear in html output (and `--pdf`), markdown lists
    /// the card text. `scores` (§G2) renders the weighted scores table
    /// (pages × categories plus the site mean) and the prioritized fix
    /// list; json embeds it as data.
    ///
    /// `--pdf PATH` (GAP G10) additionally prints the HTML report to `PATH`
    /// by shelling out to a `weasyprint`, `chromium`, or `chromium-browser`
    /// binary found in PATH (10 s subprocess timeout). Exit codes: those of
    /// `--ci` above, plus `3` when no PDF engine is found in PATH.
    #[cfg(feature = "full")]
    Report {
        /// Crawl database path or directory
        #[arg(short, long)]
        crawl: PathBuf,

        /// Output file for the report
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Report format: json, markdown (md), html, or csv
        ///
        /// Defaults to markdown. `html` was previously advertised here and was
        /// also the built-in default, but no HTML report was ever implemented,
        /// so both the documented value and the default failed at runtime.
        /// The html format now exists (GAP G10) and is what `--pdf` prints.
        #[arg(long)]
        format: Option<String>,

        /// Report theme: light or dark.
        ///
        /// Only affects HTML output and is currently a no-op, since no HTML
        /// report format is offered.
        #[arg(long, default_value = "light", hide = true)]
        theme: String,

        /// Enable CI mode: after rendering, print a one-line gate summary
        /// and exit non-zero (10/11) when a threshold is exceeded. Exit
        /// codes are documented in this command's help.
        #[arg(long)]
        ci: bool,

        /// Maximum tolerated error-severity findings in `--ci` mode
        /// (critical findings count as errors). Default 0: any error fails.
        #[arg(long, default_value_t = 0)]
        max_errors: usize,

        /// Maximum tolerated warning-severity findings in `--ci` mode.
        /// Omit for unbounded warnings.
        #[arg(long)]
        max_warnings: Option<usize>,

        /// Lower bound on the §G2 site score in `--ci` mode: a computed
        /// score below SCORE fails the gate with exit 12. Scores are
        /// computed from the crawl's findings with the embedded weight
        /// table, so this works on any existing crawl database.
        #[arg(long, value_name = "SCORE")]
        min_score: Option<f64>,

        /// Baseline crawl database (or directory) to compare against. In
        /// `--ci` mode, any new error-severity findings versus the baseline
        /// fail the gate with exit 10, even under a lenient `--max-errors`.
        #[arg(long)]
        baseline: Option<PathBuf>,

        /// Report sections to render: summary, backlinks, previews, scores,
        /// or all. Repeatable or comma-separated (GAP G9). Default is
        /// summary and backlinks (everything that predates the previews
        /// section), so unqualified output is unchanged. Applies to
        /// markdown and html output (and to the page `--pdf` prints); json
        /// accepts `scores` only (embedded as data) and csv has a fixed
        /// layout.
        #[arg(long, value_name = "SECTIONS")]
        section: Vec<String>,

        /// Also print the HTML report to a PDF file at PATH, using a PDF
        /// engine found in PATH (`weasyprint`, `chromium`, or
        /// `chromium-browser`; 10 s timeout). Exits with code 3 when no
        /// engine is found. The textual report is still produced according
        /// to `--format`.
        #[arg(long, value_name = "PATH")]
        pdf: Option<PathBuf>,
    },

    /// Analyze backlinks from an existing crawl
    #[cfg(feature = "full")]
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

    /// Generate a visual crawl map (SVG) from existing crawl data
    #[cfg(feature = "full")]
    CrawlMap {
        /// Crawl database path or directory
        #[arg(short, long)]
        crawl: PathBuf,

        /// Output file for the SVG map
        #[arg(short, long)]
        output: PathBuf,

        /// Color scheme: status, depth, or popularity
        #[arg(long, default_value = "status")]
        color_by: String,

        /// Maximum number of pages to include
        #[arg(long, default_value_t = 200)]
        max_nodes: usize,
    },

    /// Deep single-page analysis: fetch + all analyzers + optional CrUX/GSC
    #[cfg(feature = "full")]
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

    /// Ingest a Lighthouse JSON report and classify CWV (GAP G5)
    #[cfg(feature = "full")]
    Lighthouse {
        /// Path to the Lighthouse JSON report file
        #[arg(long)]
        report: PathBuf,

        /// URL the report was generated for
        #[arg(long)]
        url: String,

        /// Crawl database to store findings into (prints when omitted)
        #[arg(long)]
        crawl: Option<PathBuf>,
    },

    /// Mirror a site offline: pages + assets with relative links (GAP G8)
    #[cfg(feature = "full")]
    Clone {
        /// Site to mirror
        url: String,

        /// Output directory for the mirror
        #[arg(short, long)]
        out: PathBuf,

        /// Maximum number of resources to fetch
        #[arg(long)]
        max_pages: Option<usize>,

        /// Delay between requests in milliseconds
        #[arg(long)]
        delay: Option<u64>,
    },

    /// Convert a site to Markdown, one file per page (GAP G8)
    #[cfg(feature = "full")]
    Md {
        /// Site to convert
        url: String,

        /// Output directory for the markdown files
        #[arg(short, long)]
        out: PathBuf,

        /// Merge every page into a single site.md, dropping duplicate content
        #[arg(long)]
        merge: bool,

        /// Maximum number of pages to fetch
        #[arg(long)]
        max_pages: Option<usize>,

        /// Delay between requests in milliseconds
        #[arg(long)]
        delay: Option<u64>,
    },

    /// Manage WASM plugin signing keys and verify plugin trust chains
    #[cfg(feature = "full")]
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },

    /// Analyze web server access logs
    LogAnalyze(LogAnalyzeArgs),

    /// Show per-tenant usage metering rollups (ADR-017 read-only mirror)
    #[cfg(feature = "full")]
    Usage {
        /// Path to the crawlkit storage database (e.g. <out-dir>/crawlkit.db)
        #[arg(long)]
        db: PathBuf,

        /// Tenant id to report usage for
        #[arg(long)]
        tenant: String,

        /// Inclusive start UTC date (YYYY-MM-DD; defaults to 30 days ago)
        #[arg(long)]
        from: Option<String>,

        /// Inclusive end UTC date (YYYY-MM-DD; defaults to today)
        #[arg(long)]
        to: Option<String>,
    },

    /// Inspect and re-drive dead-lettered queue entries (ADR-015 §3)
    #[cfg(feature = "queue-ops")]
    Queue {
        #[command(subcommand)]
        command: QueueCommands,
    },

    /// Export a completed crawl to warehouse files (ADR-016 bindings)
    #[cfg(feature = "warehouse")]
    Export {
        /// Path to the crawlkit storage database (e.g. <out-dir>/crawlkit.db)
        #[arg(long)]
        db: Option<PathBuf>,

        /// Crawl id to export (defaults to the only crawl in the database)
        #[arg(long)]
        crawl_id: Option<String>,

        /// Tenant id recorded on the export's crawl_runs row
        #[arg(long)]
        tenant: Option<String>,

        /// Physical binding to emit
        #[arg(long, value_enum, default_value = "parquet")]
        format: export_warehouse::ExportFormat,

        /// Print the destination layout plan (NDJSON) instead of exporting:
        /// `--layout s3` | `--layout bigquery --dataset D` | `--layout snowflake --stage S`
        #[arg(long, value_enum)]
        layout: Option<export_warehouse::LayoutDestination>,

        /// BigQuery dataset for `--layout bigquery`
        #[arg(long, default_value = "crawlkit")]
        dataset: String,

        /// Snowflake stage for `--layout snowflake`
        #[arg(long, default_value = "crawlkit")]
        stage: String,

        /// Directory receiving the exported table files
        #[arg(short, long, default_value = "warehouse-export")]
        output: PathBuf,

        /// Upload the export to a destination instead of (or after) writing
        /// local files: `s3://bucket/` | `bigquery://project/dataset` |
        /// `snowflake://account/db/schema/STAGE`. Credentials come from env
        /// (never CLI flags): AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY for s3,
        /// BIGQUERY_TOKEN for bigquery, SNOWFLAKE_TOKEN (+SNOWFLAKE_WAREHOUSE)
        /// for snowflake. Objects are keyed by the crawl-scoped layout plan,
        /// so re-uploading the same export is idempotent.
        #[arg(long)]
        upload: Option<String>,

        /// S3-compatible endpoint override (e.g. http://127.0.0.1:9000 for
        /// MinIO). Implies path-style addressing.
        #[arg(long)]
        s3_endpoint: Option<String>,
    },

    /// Analyze trends across multiple crawl snapshots
    #[cfg(feature = "full")]
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

    /// §G6 score history: print the finding-count trend recorded by
    /// scheduled runs (`history` table in a crawlkit database).
    ///
    /// TARGET is a database path/directory, or a run id — when it is not a
    /// path, the database is resolved from `--db` / `$CRAWLKIT_DB` /
    /// `./crawlkit.db`.
    #[cfg(feature = "full")]
    History {
        /// Database path/directory, or a run id
        #[arg(default_value = ".")]
        target: String,

        /// Database to use when TARGET is a run id
        #[arg(long)]
        db: Option<PathBuf>,
    },

    /// Fetch and analyze Google Search Console data
    #[cfg(feature = "full")]
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

    /// Track keyword rankings across search engines
    #[cfg(feature = "full")]
    Rank {
        #[command(subcommand)]
        command: crate::cli::rank::RankCommands,
    },

    /// Manage recurring crawls on a crawlkit-api server
    #[cfg(feature = "full")]
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommands,
    },
}

/// Subcommands for managing recurring crawls against a crawlkit-api server.
///
/// Credentials: CRAWLKIT_API_KEY (X-API-Key) or CRAWLKIT_JWT (Bearer).
#[cfg(feature = "full")]
#[derive(Subcommand)]
pub enum ScheduleCommands {
    /// Create a recurring crawl schedule (minimum interval: 60 seconds)
    Add {
        /// Base URL of the crawlkit-api server
        #[arg(long, default_value = "http://localhost:8080")]
        api: String,

        /// Starting URL for the scheduled crawl
        url: String,

        /// Seconds between crawl runs (>= 60)
        #[arg(long, default_value_t = 3600)]
        interval_secs: u64,

        /// Maximum pages per run
        #[arg(long, default_value_t = 100)]
        max_pages: usize,

        /// Delay between requests in milliseconds
        #[arg(long, default_value_t = 500)]
        delay_ms: u64,

        /// Concurrent fetchers per run
        #[arg(long, default_value_t = 4)]
        concurrency: usize,

        /// §G3: webhook to POST a signed `regression` event to whenever a
        /// scheduled run's monitoring detects a regression. Persisted on
        /// the schedule server-side.
        #[arg(long)]
        alert_webhook: Option<String>,

        /// Hex-encoded HMAC-SHA256 secret for the webhook signature
        #[arg(long)]
        alert_secret: Option<String>,

        /// Webhook payload formatting: generic, slack, or discord
        #[arg(long, default_value = "generic")]
        alert_format: String,
    },

    /// List schedules visible to your credentials
    List {
        /// Base URL of the crawlkit-api server
        #[arg(long, default_value = "http://localhost:8080")]
        api: String,
    },

    /// Enable a paused schedule
    Enable {
        /// Base URL of the crawlkit-api server
        #[arg(long, default_value = "http://localhost:8080")]
        api: String,

        /// Schedule ID
        id: String,
    },

    /// Pause a schedule without deleting it
    Disable {
        /// Base URL of the crawlkit-api server
        #[arg(long, default_value = "http://localhost:8080")]
        api: String,

        /// Schedule ID
        id: String,
    },

    /// Permanently remove a schedule
    Remove {
        /// Base URL of the crawlkit-api server
        #[arg(long, default_value = "http://localhost:8080")]
        api: String,

        /// Schedule ID
        id: String,
    },
}

/// Queue dead-letter operator commands (ADR-015 §3).
#[cfg(feature = "queue-ops")]
#[derive(Subcommand)]
pub enum QueueCommands {
    /// List dead-lettered entries (NDJSON, one object per line)
    List {
        /// Restrict to one crawl namespace; omit to list all
        #[arg(long)]
        crawl_id: Option<String>,
    },
    /// Re-drive a dead letter (reset its attempt count, re-queue it)
    Redrive {
        /// Entry index as shown by `list` (oldest = 0)
        index: usize,

        /// Restrict to one crawl namespace; omit to index across all
        #[arg(long)]
        crawl_id: Option<String>,
    },
    /// Namespace hygiene: list every crawl namespace's queue volumes (NDJSON)
    NamespaceList,
    /// Namespace hygiene: clear a namespace's queue keys (pending, leases,
    /// dead letters, visited). Refuses namespaces with live leases unless
    /// --force. The 2026-09-16 capacity finding: stale namespaces grind
    /// workers through breaker/backoff storms on dead hosts.
    NamespacePurge {
        /// Namespace (crawl ID) to purge
        #[arg(long)]
        crawl_id: Option<String>,
        /// Purge every discovered namespace
        #[arg(long)]
        all: bool,
        /// Purge even when entries are leased to a live worker
        #[arg(long)]
        force: bool,
    },
}

/// Resolve §G3 alert delivery targets: CLI flags override the `[alerts]`
/// config section. `None` when neither configures anything.
///
/// # Errors
///
/// Returns an error when the format name is unknown.
#[cfg(feature = "full")]
pub fn resolve_alert_delivery(
    config: Option<&AlertsConfig>,
    alert_webhook: Option<&str>,
    alert_secret: Option<&str>,
    alert_format: &str,
    alert_email: Option<&str>,
) -> anyhow::Result<Option<crate::alerts::AlertDelivery>> {
    // The flag's default is `generic`, so a non-generic `[alerts].format`
    // in the config wins unless the caller explicitly asked for a format.
    let format = match crate::alerts::AlertFormat::parse(alert_format)? {
        crate::alerts::AlertFormat::Generic => {
            let configured = config
                .and_then(|c| c.format.as_deref())
                .map(str::trim)
                .filter(|f| !f.is_empty());
            match configured {
                Some(name) => crate::alerts::AlertFormat::parse(name)?,
                None => crate::alerts::AlertFormat::Generic,
            }
        }
        explicit => explicit,
    };
    let mut delivery = crate::alerts::AlertDelivery::default();

    fn non_empty(value: Option<&str>) -> Option<&str> {
        value.map(str::trim).filter(|v| !v.is_empty())
    }

    let url = non_empty(alert_webhook)
        .map(str::to_string)
        .or_else(|| non_empty(config.and_then(|c| c.webhook_url.as_deref())).map(str::to_string));
    if let Some(url) = url {
        let secret = non_empty(alert_secret).map(str::to_string).or_else(|| {
            non_empty(config.and_then(|c| c.secret_hex.as_deref())).map(str::to_string)
        });
        delivery.webhook = Some(crate::alerts::WebhookConfig {
            url,
            secret_hex: secret,
            format,
        });
    }

    let email_to = non_empty(alert_email)
        .map(str::to_string)
        .or_else(|| config.and_then(|c| c.email_to.clone()));
    if let Some(to) = email_to {
        delivery.email = Some(crate::alerts::EmailTarget {
            server: config
                .and_then(|c| c.smtp_server.clone())
                .unwrap_or_else(|| "localhost:25".to_string()),
            to,
        });
    }

    Ok(delivery.is_configured().then_some(delivery))
}

/// Parameters for a crawl operation, bundling all CLI/config values.
#[cfg(feature = "full")]
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
    pub allow_private: bool,
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
    /// Enable LLM-powered analysis (requires API key in env).
    pub llm: bool,
    /// Selected analyzer registry profile.
    pub analyzer_profile: String,
    /// Emit page measurements alongside issues in the primary output.
    ///
    /// Measurements are always stored; this only controls whether they appear
    /// in `crawl-results.json` and the console summary.
    pub include_metrics: bool,
    /// Per-crawl render quota (pages). `None` = unbounded.
    pub render_max: Option<u64>,
    /// Per-page render ceiling in seconds. `None` = engine default.
    pub render_timeout: Option<u64>,
}
