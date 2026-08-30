use anyhow::{Context, Result};
use crawlkit_engine::storage::Storage;
use crawlkit_engine::trends::{analyze_trends, trend_to_json, trend_to_markdown, CrawlSnapshot};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Run trend analysis across multiple crawl snapshots.
pub fn run(
    db_path: &Path,
    crawl_ids: &[String],
    output: Option<&Path>,
    format: &str,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message("Analyzing trends...");

    let db_file = if db_path.is_dir() {
        db_path.join("crawlkit.db")
    } else {
        db_path.to_path_buf()
    };

    if !db_file.exists() {
        return Err(anyhow::anyhow!(
            "Database not found: {}",
            db_file.display()
        ));
    }

    let storage =
        Storage::new(&db_file).with_context(|| format!("Failed to open database: {}", db_file.display()))?;

    // If no crawl IDs provided, auto-discover from the database
    let ids_to_use = if crawl_ids.is_empty() {
        pb.set_message("Discovering crawl snapshots...");
        let all_crawls = storage
            .list_crawls()
            .context("Failed to list crawls")?;
        if all_crawls.is_empty() {
            return Err(anyhow::anyhow!("No crawls found in database"));
        }
        all_crawls.into_iter().map(|(id, _)| id).collect()
    } else {
        crawl_ids.to_vec()
    };

    if ids_to_use.len() < 2 {
        return Err(anyhow::anyhow!(
            "Need at least 2 crawl snapshots for trend analysis, found {}",
            ids_to_use.len()
        ));
    }

    pb.set_message(format!(
        "Loading {} snapshots...",
        ids_to_use.len()
    ));

    let mut snapshots = Vec::new();
    for crawl_id in &ids_to_use {
        let meta = storage
            .get_crawl_meta(crawl_id)
            .with_context(|| format!("Failed to get metadata for crawl {crawl_id}"))?;
        let stats = storage
            .get_stats(crawl_id)
            .with_context(|| format!("Failed to get stats for crawl {crawl_id}"))?;

        let timestamp = meta
            .start_time
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        snapshots.push(CrawlSnapshot {
            crawl_id: crawl_id.clone(),
            timestamp,
            stats,
        });
    }

    // Sort chronologically
    snapshots.sort_by_key(|s| s.timestamp);

    pb.set_message("Computing trends...");

    let analysis = analyze_trends(snapshots).context("Failed to analyze trends")?;

    let output_str = match format {
        "json" => trend_to_json(&analysis, true).context("Failed to serialize trend report")?,
        "md" => trend_to_markdown(&analysis),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format: {format}. Use json or md."
            ));
        }
    };

    pb.finish_with_message(format!(
        "Trend analysis: {} snapshots, direction={:?}, avg_health={:.1}",
        analysis.summary.snapshot_count,
        analysis.direction,
        analysis.summary.avg_health_score,
    ));

    if let Some(out) = output {
        std::fs::write(out, &output_str)?;
        tracing::info!("Wrote trend analysis to {}", out.display());
    } else {
        println!("{output_str}");
    }

    Ok(())
}
