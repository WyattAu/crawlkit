use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Compare two crawl results.
///
/// When `alerts` is configured and the diff contains new errors (§G3), a
/// signed `regression` event is delivered to the webhook (and optionally
/// emailed). Alert delivery is best-effort: a webhook outage is reported
/// on stderr but does not fail the comparison itself.
pub async fn run(
    crawl1_path: &Path,
    crawl2_path: &Path,
    output: Option<&Path>,
    format: &str,
    alerts: Option<&crate::alerts::AlertDelivery>,
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

    if let Some(alerts) = alerts {
        if crate::alerts::has_regressions(&diff) {
            if let Err(e) =
                fire_regression_alerts(alerts, &storage1_path, &storage2_path, &diff).await
            {
                tracing::warn!(error = %e, "alert delivery failed");
                eprintln!("warning: alert delivery failed: {e}");
            }
        }
    }

    Ok(())
}

/// Deliver the regression event for a diff to every configured target.
async fn fire_regression_alerts(
    alerts: &crate::alerts::AlertDelivery,
    baseline_db: &Path,
    target_db: &Path,
    diff: &crawlkit_engine::compare::CrawlDiff,
) -> Result<()> {
    let baseline_id = latest_crawl_id(baseline_db);
    let target_id = latest_crawl_id(target_db);
    let event = crate::alerts::AlertEvent::regression_from_diff(
        &baseline_id,
        &target_id,
        diff,
        chrono::Utc::now(),
    );

    if let Some(webhook) = &alerts.webhook {
        crate::alerts::deliver_event(&reqwest::Client::new(), webhook, &event).await?;
        println!(
            "alert: regression event delivered to webhook ({} new errors, {} removed pages, {} cwv regressions)",
            event.findings_delta.get("new_errors").copied().unwrap_or(0),
            event.findings_delta.get("removed_pages").copied().unwrap_or(0),
            event
                .findings_delta
                .get("cwv_regressions")
                .copied()
                .unwrap_or(0),
        );
    }

    if let Some(email) = &alerts.email {
        let subject = format!("crawlkit {} alert", event.event);
        let body = crate::alerts::canonical_event_body(&event)?;
        crate::alerts::send_email(email, &subject, &body).await?;
        println!("alert: regression event emailed to {}", email.to);
    }

    Ok(())
}

/// Latest crawl id in a crawl database, falling back to the database path
/// string when the id cannot be read (the id is advisory event context).
fn latest_crawl_id(db_path: &Path) -> String {
    crawlkit_engine::compare::latest_crawl_id_of(db_path)
        .unwrap_or_else(|_| db_path.display().to_string())
}
