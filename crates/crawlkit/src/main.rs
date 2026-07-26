//! Command-line interface for crawlkit — a high-performance site crawler for SEO analysis.
//!
//! Provides subcommands to **crawl** a website, **compare** two crawl results,
//! and **generate reports** from stored crawl data. Configuration can be supplied
//! via CLI flags or a TOML config file.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::path::{Path, PathBuf};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

use crawlkit_engine::storage::Storage;

/// CLI configuration file structure.
#[derive(serde::Deserialize, Default)]
struct Config {
    /// Crawl-specific defaults.
    crawl: Option<CrawlConfig>,
    /// Output settings.
    output: Option<OutputConfig>,
    /// Feature flags.
    features: Option<FeaturesConfig>,
}

/// Feature flag configuration loaded from file.
#[derive(serde::Deserialize, Default)]
struct FeaturesConfig {
    /// Enable AI analyzers.
    ai_analyzers: Option<bool>,
    /// Enable WASM analyzers.
    wasm_analyzers: Option<bool>,
    /// Enable JS rendering.
    js_rendering: Option<bool>,
    /// Enable backlink analysis.
    backlink_analysis: Option<bool>,
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
#[allow(clippy::large_enum_variant)]
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

    let otel_env = std::env::var("OTEL_EXPORTER").ok();
    let use_otel = otel_env.as_deref();

    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(format!("crawlkit={log_level}"))),
    );

    match use_otel {
        Some("stdout") => {
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
        }
        Some("otlp") => {
            tracing::warn!("OTLP export requires opentelemetry-otlp crate. Install with: cargo add opentelemetry-otlp");
            tracing_subscriber::registry().with(fmt_layer).init();
        }
        _ => {
            tracing_subscriber::registry().with(fmt_layer).init();
        }
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

    // Build feature flags from config (shared across subcommands)
    let mut feature_flags = crawlkit_engine::FeatureFlags::default();
    if let Some(ref features_config) = config.features {
        if let Some(v) = features_config.ai_analyzers {
            feature_flags.set(crawlkit_engine::FLAG_AI_ANALYZERS, v);
        }
        if let Some(v) = features_config.wasm_analyzers {
            feature_flags.set(crawlkit_engine::FLAG_WASM_ANALYZERS, v);
        }
        if let Some(v) = features_config.js_rendering {
            feature_flags.set(crawlkit_engine::FLAG_JS_RENDERING, v);
        }
        if let Some(v) = features_config.backlink_analysis {
            feature_flags.set(crawlkit_engine::feature_flags::FLAG_BACKLINK_ANALYSIS, v);
        }
    }

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
            seed,
            enable_ai,
            enable_wasm,
            encrypt,
            metrics_json,
            tenant,
            incremental,
            force,
        } => {
            feature_flags.set(crawlkit_engine::FLAG_AI_ANALYZERS, enable_ai);
            feature_flags.set(crawlkit_engine::FLAG_WASM_ANALYZERS, enable_wasm);

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
                seed,
                encrypt,
                metrics_json,
                tenant,
                incremental,
                force,
                feature_flags,
            };
            run_crawl(&params).await
        }
        Commands::Compare {
            crawl1,
            crawl2,
            output,
            format,
        } => run_compare(&crawl1, &crawl2, output.as_deref(), &format),
        Commands::Report {
            crawl,
            output,
            format,
            theme,
        } => {
            let format = format
                .or_else(|| config.output.as_ref().and_then(|o| o.format.clone()))
                .unwrap_or_else(|| "html".to_string());
            run_report(&crawl, output.as_deref(), &format, &theme, &feature_flags)
        }
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
                &feature_flags,
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
    seed: Option<u64>,
    encrypt: bool,
    metrics_json: Option<PathBuf>,
    tenant: Option<String>,
    incremental: bool,
    force: bool,
    feature_flags: crawlkit_engine::FeatureFlags,
}

/// Execute a crawl with the given parameters.
async fn run_crawl(params: &CrawlParams) -> Result<()> {
    use crawlkit_engine::advanced_features::{AlertManager, AlertOperator};
    use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
    use crawlkit_engine::playwright::{PlaywrightConfig, PlaywrightDetector, PlaywrightRenderer};
    use crawlkit_engine::storage::Severity;
    use std::collections::HashMap;
    use std::sync::Arc;

    let max_pages = params.max_pages.unwrap_or(100);
    let concurrency = params.concurrency.unwrap_or(8);

    let _root_span = tracing::info_span!(
        "crawl",
        target_url = %params.url,
        max_pages = max_pages,
        concurrency = concurrency,
        seed = params.seed.unwrap_or(0),
    )
    .entered();

    tracing::info!(
        "Starting crawl of {} (max_pages={}, delay={}ms, concurrency={}, depth={}, js={}, allow_external={}, incremental={}, force={})",
        params.url,
        max_pages,
        params.delay.unwrap_or(100),
        concurrency,
        params.depth.map_or("none".to_string(), |d| d.to_string()),
        params.javascript,
        params.allow_external,
        params.incremental,
        params.force,
    );

    let pb = ProgressBar::new(max_pages as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} pages ({eta} remaining) - {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
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

    let encryption = crawlkit_engine::EncryptionManager::new(crawlkit_engine::EncryptionConfig {
        enabled: params.encrypt,
        ..Default::default()
    });
    if params.encrypt {
        encryption
            .initialize()
            .context("Failed to initialize encryption")?;
        tracing::info!("Encryption at rest enabled");
    }

    // Initialize audit trail if enabled
    let audit_trail = crawlkit_engine::AuditTrail::new();
    let audit_enabled = params
        .feature_flags
        .get(crawlkit_engine::feature_flags::FLAG_AUDIT_TRAIL);
    if audit_enabled {
        audit_trail.record(
            crawlkit_engine::AuditEventType::CrawlStarted,
            "cli",
            &format!("Crawl started for {}", params.url),
        );
    }

    // Log feature flags
    tracing::info!(
        "Feature flags: ai_analyzers={}, wasm_analyzers={}, js_rendering={}, audit_trail={}, observability={}, rum_integration={}, backlink_analysis={}",
        params.feature_flags.get(crawlkit_engine::FLAG_AI_ANALYZERS),
        params.feature_flags.get(crawlkit_engine::FLAG_WASM_ANALYZERS),
        params.feature_flags.get(crawlkit_engine::FLAG_JS_RENDERING),
        audit_enabled,
        params.feature_flags.get(crawlkit_engine::feature_flags::FLAG_OBSERVABILITY),
        params.feature_flags.get(crawlkit_engine::feature_flags::FLAG_RUM_INTEGRATION),
        params.feature_flags.get(crawlkit_engine::feature_flags::FLAG_BACKLINK_ANALYSIS),
    );

    // Initialize Playwright renderer if --javascript is enabled
    let playwright_detector = PlaywrightDetector::detect();
    let js_renderer: Option<Arc<dyn crawlkit_engine::crawl_engine::JsRenderer>> = if params
        .javascript
    {
        if playwright_detector.is_available() {
            tracing::info!("Playwright detected: JS rendering enabled");
            let renderer = PlaywrightRenderer::new(PlaywrightConfig {
                enabled: true,
                timeout: std::time::Duration::from_secs(30),
                max_memory_per_context: 512 * 1024 * 1024,
                max_cpu_seconds: 30,
                max_concurrent: 5,
                headless: true,
                ..Default::default()
            });
            Some(Arc::new(PlaywrightJsRenderer(renderer)))
        } else {
            tracing::warn!("Playwright not found: JS rendering disabled. Install with: npm install -g playwright");
            None
        }
    } else {
        None
    };

    // Build CrawlEngineConfig
    let engine_config = CrawlEngineConfig {
        crawl_config: crawlkit_engine::CrawlConfig {
            respect_robots_txt: params.respect_robots.unwrap_or(true),
            max_time: params.max_time_secs.map(std::time::Duration::from_secs),
            max_depth: params.depth,
            request_delay: std::time::Duration::from_millis(params.delay.unwrap_or(100)),
            concurrency,
            ..Default::default()
        },
        feature_flags: params.feature_flags.clone(),
        enable_js_rendering: params.javascript,
        js_renderer,
        allow_external: params.allow_external,
        include_patterns: params.include.clone(),
        exclude_patterns: params.exclude.clone(),
        seed: params.seed,
        tenant_id: params.tenant.clone(),
        user_agent: params.user_agent.clone(),
        encryption: if params.encrypt {
            Some(encryption.clone())
        } else {
            None
        },
        timeout_secs: Some(params.timeout.unwrap_or(30)),
        delay_ms: Some(params.delay.unwrap_or(100)),
        concurrency: Some(concurrency),
        incremental: params.incremental,
        force: params.force,
    };

    let engine = CrawlEngine::new(engine_config, storage);

    // Run the crawl with progress callback
    let pb_clone = pb.clone();
    let result = engine
        .run_with_callback(
            &params.url,
            Some(Arc::new(move |_url, _page_id, _findings| {
                pb_clone.inc(1);
            })),
        )
        .await?;

    pb.finish_with_message(format!(
        "Crawl complete: {} pages crawled, {} stored, {} issues, {} external skipped, {} blocked by robots.txt, {} duplicate content, {} unchanged, {} modified, {} new",
        result.pages_crawled, result.pages_stored, result.issues_found, result.skipped_external, result.skipped_robots, result.skipped_duplicate, result.pages_unchanged, result.pages_modified, result.pages_new
    ));

    if audit_enabled {
        audit_trail.record(
            crawlkit_engine::AuditEventType::CrawlCompleted,
            "cli",
            &format!(
                "Crawl completed: {} pages crawled, {} stored, {} issues",
                result.pages_crawled, result.pages_stored, result.issues_found
            ),
        );
    }

    // Log metrics snapshot
    let avg_fetch_ms = if result.metrics.pages_crawled > 0 {
        result.metrics.fetch_time_us as f64 / result.metrics.pages_crawled as f64 / 1000.0
    } else {
        0.0
    };
    let avg_analysis_ms = if result.metrics.pages_crawled > 0 {
        result.metrics.analysis_time_us as f64 / result.metrics.pages_crawled as f64 / 1000.0
    } else {
        0.0
    };
    tracing::info!(
        "Metrics: {:.2} pages/sec, avg fetch {:.2}ms, avg analysis {:.2}ms, {} bytes fetched, {} failures",
        result.metrics.pages_crawled as f64 / result.elapsed.as_secs_f64().max(0.001),
        avg_fetch_ms,
        avg_analysis_ms,
        result.metrics.bytes_fetched,
        result.metrics.pages_failed,
    );

    // Export metrics JSON if requested
    if let Some(metrics_path) = &params.metrics_json {
        std::fs::write(metrics_path, serde_json::to_string_pretty(&result.metrics)?)?;
        tracing::info!("Wrote metrics to {}", metrics_path.display());
    }

    // Alert manager: check for threshold violations
    let alert_manager = AlertManager::new();
    alert_manager.add_alert(crawlkit_engine::advanced_features::Alert {
        id: "high_error_rate".to_string(),
        name: "High Error Rate".to_string(),
        description: "Error rate exceeds 10% threshold".to_string(),
        severity: Severity::Warning,
        metric: "error_rate".to_string(),
        threshold: 0.1,
        operator: AlertOperator::GreaterThan,
        enabled: true,
    });
    let mut metrics_map = HashMap::new();
    metrics_map.insert("pages_crawled".to_string(), result.pages_crawled as f64);
    metrics_map.insert(
        "pages_failed".to_string(),
        result.metrics.pages_failed as f64,
    );
    metrics_map.insert(
        "error_rate".to_string(),
        result.metrics.pages_failed as f64 / (result.pages_crawled.max(1) as f64),
    );
    metrics_map.insert("avg_fetch_time_ms".to_string(), avg_fetch_ms);
    let triggered_alerts = alert_manager.check_alerts(&metrics_map);
    if !triggered_alerts.is_empty() {
        for alert in &triggered_alerts {
            tracing::warn!("Alert triggered: {} - {}", alert.name, alert.description);
        }
    }
    let alerts_json_path = output_dir.join("alerts.json");
    std::fs::write(
        &alerts_json_path,
        serde_json::to_string_pretty(&triggered_alerts)?,
    )?;
    tracing::info!("Wrote alerts to {}", alerts_json_path.display());

    // Write output
    if params.format == "json" || params.format == "all" {
        let json_path = output_dir.join("crawl-results.json");
        let stats = engine.storage().get_stats(&result.crawl_id)?;
        let sample = serde_json::json!({
            "crawl_id": result.crawl_id,
            "target_url": params.url,
            "max_pages": max_pages,
            "pages_crawled": result.pages_crawled,
            "pages_stored": result.pages_stored,
            "total_issues": stats.total_issues,
            "status": "completed",
            "seed": params.seed,
        });
        std::fs::write(&json_path, serde_json::to_string_pretty(&sample)?)?;
        tracing::info!("Wrote results to {}", json_path.display());

        let metrics_path = output_dir.join("metrics.json");
        std::fs::write(
            &metrics_path,
            serde_json::to_string_pretty(&result.metrics)?,
        )?;
        tracing::info!("Wrote metrics to {}", metrics_path.display());

        // Run post-crawl analysis for cross-page checks
        tracing::info!("Running post-crawl analysis...");
        let post_analysis = crawlkit_engine::post_crawl::run_post_crawl_analysis(
            engine.storage(),
            &result.crawl_id,
        );
        tracing::info!(
            "Post-crawl analysis: {} pages analyzed, {} canonical issues, {} sitemap issues",
            post_analysis.stats.pages_analyzed,
            post_analysis.stats.canonical_mismatches,
            post_analysis.stats.sitemap_issues,
        );

        // Write post-crawl findings
        if !post_analysis.findings.is_empty() {
            let post_findings_path = output_dir.join("post-crawl-findings.json");
            let findings_json: Vec<serde_json::Value> = post_analysis
                .findings
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "page_url": f.page_url,
                        "severity": format!("{:?}", f.severity).to_lowercase(),
                        "code": f.code,
                        "title": f.title,
                        "description": f.description,
                        "recommendation": f.recommendation,
                    })
                })
                .collect();
            std::fs::write(
                &post_findings_path,
                serde_json::to_string_pretty(&findings_json)?,
            )?;
            tracing::info!(
                "Wrote {} post-crawl findings to {}",
                post_analysis.findings.len(),
                post_findings_path.display()
            );
        }
    }

    tracing::info!(
        "Crawl complete: {} pages crawled, {} stored, {} issues, {} external skipped, {} blocked by robots.txt. Database: {}",
        result.pages_crawled,
        result.pages_stored,
        result.issues_found,
        result.skipped_external,
        result.skipped_robots,
        db_path.display()
    );
    Ok(())
}

/// Wrapper to adapt `PlaywrightRenderer` to the `JsRenderer` trait.
struct PlaywrightJsRenderer(crawlkit_engine::PlaywrightRenderer);

#[async_trait::async_trait]
impl crawlkit_engine::crawl_engine::JsRenderer for PlaywrightJsRenderer {
    fn is_available(&self) -> bool {
        self.0.is_available()
    }

    async fn render(&self, url: &str) -> Result<String, String> {
        self.0
            .render(url)
            .await
            .map(|r| r.html)
            .map_err(|e| e.to_string())
    }
}

/// Compare two crawl results.
fn run_compare(
    crawl1_path: &Path,
    crawl2_path: &Path,
    output: Option<&Path>,
    format: &str,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
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

    let diff = crawlkit_engine::compare::compare_crawls(&storage1_path, &storage2_path)
        .context("Failed to compare crawls")?;

    let output_str = match format {
        "json" => crawlkit_engine::compare::diff_to_json(&diff, true)
            .context("Failed to serialize comparison")?,
        "md" => crawlkit_engine::compare::diff_to_markdown(&diff),
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
fn run_report(
    crawl_path: &Path,
    output: Option<&Path>,
    format: &str,
    _theme: &str,
    feature_flags: &crawlkit_engine::FeatureFlags,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message("Generating report...");

    let db_path = if crawl_path.is_dir() {
        crawl_path.join("crawlkit.db")
    } else {
        crawl_path.to_path_buf()
    };

    let storage = Storage::new(&db_path)
        .with_context(|| format!("Failed to open crawl database: {}", db_path.display()))?;

    // Initialize encryption manager (disabled — decryption only)
    let _encryption = crawlkit_engine::EncryptionManager::default();

    // Get crawl statistics
    let crawl_id = storage
        .get_latest_crawl_id()
        .context("Failed to get latest crawl ID")?
        .ok_or_else(|| anyhow::anyhow!("No crawls found in database"))?;

    let stats = storage
        .get_stats(&crawl_id)
        .context("Failed to get crawl statistics")?;

    // Run backlink analysis
    let backlink_data = if feature_flags.get(crawlkit_engine::feature_flags::FLAG_BACKLINK_ANALYSIS)
    {
        let link_pairs = storage.get_links_for_crawl(&crawl_id).unwrap_or_default();
        let external_links = storage.get_external_links(&crawl_id).unwrap_or_default();
        let page_urls = storage.get_page_urls(&crawl_id).unwrap_or_default();

        let mut analyzer = crawlkit_engine::BacklinkAnalyzer::new();
        analyzer.load_from_crawl_data(&link_pairs);

        // Add external backlinks
        for (source, target) in &external_links {
            analyzer.add_backlink(crawlkit_engine::Backlink {
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
    } else {
        None
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
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
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
    let mut analyzer = crawlkit_engine::BacklinkAnalyzer::new();
    analyzer.load_from_crawl_data(&link_pairs);
    for (source_url, target_url) in &external_links {
        analyzer.add_backlink(crawlkit_engine::Backlink {
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
        let registry = crawlkit_engine::BacklinkAdapterRegistry::with_defaults();
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
                            analyzer.add_backlink(crawlkit_engine::Backlink {
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
    feature_flags: &crawlkit_engine::FeatureFlags,
) -> Result<()> {
    use crawlkit_engine::analyzers::AnalyzerRegistry;
    use crawlkit_engine::http::HttpClient;
    use crawlkit_engine::HtmlParser;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );

    let url = url::Url::parse(url_str).with_context(|| format!("Invalid URL: {url_str}"))?;

    // Initialize encryption manager (disabled — decryption only)
    let encryption = crawlkit_engine::EncryptionManager::default();

    // Initialize components
    let default_ua = format!("crawlkit/{}", env!("CARGO_PKG_VERSION"));
    let ua = user_agent.unwrap_or(&default_ua);
    let http_config = crawlkit_engine::http::HttpClientConfig {
        timeout: std::time::Duration::from_secs(30),
        max_redirects: 20,
        retry_policy: crawlkit_engine::http::RetryPolicy::default(),
        user_agent: std::sync::Arc::new(crawlkit_engine::http::UserAgentRotator::new(vec![
            ua.to_string()
        ])),
        max_body_size: 10 * 1024 * 1024,
        pool_max_idle_per_host: 32,
        pool_max_idle: 64,
        tcp_keepalive: Some(std::time::Duration::from_secs(30)),
        pool_idle_timeout: std::time::Duration::from_secs(90),
        connect_timeout: std::time::Duration::from_secs(10),
    };
    let client = HttpClient::new(http_config).context("Failed to create HTTP client")?;
    let config = crawlkit_engine::CrawlConfig::default();
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
    let empty_chain: Vec<crawlkit_engine::RedirectHop> = vec![];
    let ctx = crawlkit_engine::analyzers::AnalysisContext {
        page: &parsed,
        body: Some(&result.body),
        status_code: Some(result.status_code),
        headers: &headers_vec,
        response_time: Some(fetch_time),
        redirect_chain: &empty_chain,
        robots_txt: None,
    };
    let findings = registry.analyze(&ctx, &config);

    // Fetch CrUX data from PageSpeed Insights if API key is available
    let crux_data = if feature_flags.get(crawlkit_engine::feature_flags::FLAG_RUM_INTEGRATION) {
        let adapter = crawlkit_engine::CruxAdapter::from_env();
        if adapter.is_available() {
            pb.set_message("Fetching CrUX data from PageSpeed Insights...");
            adapter.fetch_crux_data(url_str).await.ok().flatten()
        } else {
            None
        }
    } else {
        None
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
                "title": decrypt_field(&encryption, &parsed.meta.title),
                "description": decrypt_field(&encryption, &parsed.meta.description),
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
                parsed
                    .meta
                    .title
                    .as_ref()
                    .and_then(|t| decrypt_field(&encryption, &Some(t.clone())))
                    .as_deref()
                    .unwrap_or("(none)"),
                parsed
                    .meta
                    .description
                    .as_ref()
                    .and_then(|d| decrypt_field(&encryption, &Some(d.clone())))
                    .as_deref()
                    .unwrap_or("(none)"),
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

            if findings.is_empty() {
                md.push_str("No issues found.\n");
            } else {
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

/// Decrypt an encrypted field value (hex-encoded, prefixed with "enc:").
fn decrypt_field(
    encryption: &crawlkit_engine::EncryptionManager,
    field: &Option<String>,
) -> Option<String> {
    match field {
        Some(val) if val.starts_with("enc:") => {
            let hex_str = &val[4..];
            if let Ok(bytes) = hex_decode(hex_str) {
                encryption
                    .decrypt(&bytes)
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())
            } else {
                Some(val.clone())
            }
        }
        other => other.clone(),
    }
}

/// Decode a hex string to bytes.
fn hex_decode(hex: &str) -> Result<Vec<u8>, anyhow::Error> {
    if !hex.len().is_multiple_of(2) {
        return Err(anyhow::anyhow!("Invalid hex string length"));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| anyhow::anyhow!(e)))
        .collect()
}
