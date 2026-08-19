//! Command-line interface for crawlkit — a high-performance site crawler for SEO analysis.
//!
//! Provides subcommands to **crawl** a website, **compare** two crawl results,
//! **generate reports**, **analyze backlinks**, and **inspect** individual pages.
//! Configuration can be supplied via CLI flags or a TOML config file.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod cli;

use anyhow::{Context, Result};
use clap::Parser;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::path::PathBuf;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

use cli::{Cli, Commands, Config};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize Sentry error tracking
    let _sentry_guard = std::env::var("SENTRY_DSN").ok().map(|dsn| {
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                environment: Some(
                    std::env::var("ENVIRONMENT")
                        .unwrap_or_else(|_| "development".into())
                        .into(),
                ),
                traces_sample_rate: 0.1,
                ..Default::default()
            },
        ))
    });

    // Initialize dhat profiling if enabled
    #[cfg(feature = "profiling")]
    let _profiler = {
        let profiler = dhat::Profiler::builder()
            .file_name("crawlkit-dhat.json")
            .build();
        Some(profiler)
    };

    let cli = Cli::parse();
    init_tracing(&cli);

    let config = load_config(&cli)?;
    let mut feature_flags = build_feature_flags(&config);

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

            let params = cli::CrawlParams {
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
            cli::crawl::run(&params).await
        }
        Commands::Compare {
            crawl1,
            crawl2,
            output,
            format,
        } => cli::compare::run(&crawl1, &crawl2, output.as_deref(), &format),
        Commands::Report {
            crawl,
            output,
            format,
            theme,
        } => {
            let format = format
                .or_else(|| config.output.as_ref().and_then(|o| o.format.clone()))
                .unwrap_or_else(|| "html".to_string());
            cli::report::run(&crawl, output.as_deref(), &format, &theme, &feature_flags)
        }
        Commands::Backlinks {
            crawl,
            output,
            format,
            source,
        } => cli::backlinks::run(&crawl, output.as_deref(), &format, source.as_deref()).await,
        Commands::Inspect {
            url,
            output,
            format,
            javascript,
            user_agent,
        } => {
            cli::inspect::run(
                &url,
                output.as_deref(),
                &format,
                javascript,
                user_agent.as_deref(),
                &feature_flags,
            )
            .await
        }
        Commands::Plugin { command } => cli::plugin::run(command),
    }
}

fn init_tracing(cli: &Cli) {
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
}

fn load_config(cli: &Cli) -> Result<Config> {
    if let Some(config_path) = &cli.config {
        let contents = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
        toml::from_str::<Config>(&contents)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))
    } else {
        Ok(Config::default())
    }
}

fn build_feature_flags(config: &Config) -> crawlkit_engine::FeatureFlags {
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
    feature_flags
}
