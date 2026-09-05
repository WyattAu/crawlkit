use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crawlkit_engine::analyzers::post_crawl_analyzers;
use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
use crawlkit_engine::playwright::{PlaywrightConfig, PlaywrightDetector, PlaywrightRenderer};
use crawlkit_engine::storage::{Severity, Storage};

use super::CrawlParams;

/// Execute a crawl with the given parameters.
pub async fn run(params: &CrawlParams) -> Result<()> {
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

    let plugin_dirs = match &params.plugins {
        Some(dirs) => dirs.clone(),
        None => crawlkit_engine::plugin_runtime::default_plugin_dirs(),
    };

    // In monitoring mode, resolve the previous crawl ID from storage before
    // starting the new crawl (storage will overwrite the crawl row).
    let previous_crawl_id_for_monitoring = if params.monitor {
        storage.get_latest_crawl_id().ok().flatten()
    } else {
        None
    };

    let crawl_config = crawlkit_engine::CrawlConfig {
        respect_robots_txt: params.respect_robots.unwrap_or(true),
        max_time: params.max_time_secs.map(std::time::Duration::from_secs),
        max_depth: params.depth,
        request_delay: std::time::Duration::from_millis(params.delay.unwrap_or(100)),
        max_pages,
        concurrency,
        llm: crawlkit_engine::llm_analyzer::LlmConfig {
            enabled: params.llm,
            ..Default::default()
        },
        ..Default::default()
    };

    let engine_config = CrawlEngineConfig {
        crawl_config: crawl_config.clone(),
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
        allow_http: params.allow_private,
        plugin_dirs,
        post_crawl_analyzers: post_crawl_analyzers::build_post_crawl_registry(&crawl_config),
        queue: None,
        crux_api_key: std::env::var("CRUX_API_KEY").ok().filter(|k| !k.is_empty()),
        previous_crawl_id: previous_crawl_id_for_monitoring,
        alert_threshold: params.alert_threshold,
        distributed_mode: false,
        partition_strategy: None,
        instance_id: None,
        instance_count: None,
        allow_private: params.allow_private,
    };

    let engine = CrawlEngine::new(engine_config, storage);

    let pb_clone = pb.clone();
    let result = engine
        .run_with_callback(
            &params.url,
            Some(Arc::new(move |_url, _page_id, _findings| {
                pb_clone.inc(1);
            })),
        )
        .await?;

    // Report crawl metrics to Sentry
    sentry::configure_scope(|scope| {
        scope.set_tag("crawl.url", &params.url);
        scope.set_tag("crawl.pages_crawled", result.pages_crawled);
        scope.set_tag("crawl.issues_found", result.issues_found);
        scope.set_tag("crawl.pages_failed", result.metrics.pages_failed);
    });

    if result.metrics.pages_failed > 0 {
        sentry::capture_message(
            &format!(
                "Crawl completed with {} failures out of {} pages: {}",
                result.metrics.pages_failed, result.pages_crawled, params.url
            ),
            sentry::Level::Warning,
        );
    }

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

    log_metrics(&result);
    export_metrics(&result, params)?;
    check_alerts(&result, &output_dir)?;

    if let Some(ref monitoring) = result.monitoring {
        report_monitoring(monitoring, &output_dir)?;
    }

    write_output(&engine, &result, params, &output_dir)?;

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

fn log_metrics(result: &crawlkit_engine::crawl_engine::CrawlOutput) {
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
}

fn export_metrics(
    result: &crawlkit_engine::crawl_engine::CrawlOutput,
    params: &CrawlParams,
) -> Result<()> {
    if let Some(metrics_path) = &params.metrics_json {
        std::fs::write(metrics_path, serde_json::to_string_pretty(&result.metrics)?)?;
        tracing::info!("Wrote metrics to {}", metrics_path.display());
    }
    Ok(())
}

fn check_alerts(
    result: &crawlkit_engine::crawl_engine::CrawlOutput,
    output_dir: &std::path::Path,
) -> Result<()> {
    use crawlkit_engine::advanced_features::{AlertManager, AlertOperator};

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

    let avg_fetch_ms = if result.metrics.pages_crawled > 0 {
        result.metrics.fetch_time_us as f64 / result.metrics.pages_crawled as f64 / 1000.0
    } else {
        0.0
    };

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
    Ok(())
}

fn write_output(
    engine: &CrawlEngine,
    result: &crawlkit_engine::crawl_engine::CrawlOutput,
    params: &CrawlParams,
    output_dir: &std::path::Path,
) -> Result<()> {
    if params.format == "json" || params.format == "all" {
        let json_path = output_dir.join("crawl-results.json");
        let stats = engine.storage().get_stats(&result.crawl_id)?;
        let sample = serde_json::json!({
            "crawl_id": result.crawl_id,
            "target_url": params.url,
            "max_pages": params.max_pages.unwrap_or(100),
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
    Ok(())
}

fn report_monitoring(
    result: &crawlkit_engine::monitoring::MonitoringResult,
    output_dir: &std::path::Path,
) -> Result<()> {
    let json_path = output_dir.join("monitoring.json");
    std::fs::write(&json_path, serde_json::to_string_pretty(result)?)?;

    if result.alert_triggered {
        tracing::warn!(
            "MONITORING ALERT: new={}, removed={}, changed={}, cwv_regressions={}",
            result.new_pages,
            result.removed_pages,
            result.changed_pages,
            result.cwv_regressions,
        );
    } else {
        tracing::info!(
            "Monitoring: new={}, removed={}, changed={}, cwv_regressions={} (below threshold)",
            result.new_pages,
            result.removed_pages,
            result.changed_pages,
            result.cwv_regressions,
        );
    }
    tracing::info!("Wrote monitoring results to {}", json_path.display());
    Ok(())
}

/// Wrapper to adapt `PlaywrightRenderer` to the `JsRenderer` trait.
struct PlaywrightJsRenderer(PlaywrightRenderer);

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
