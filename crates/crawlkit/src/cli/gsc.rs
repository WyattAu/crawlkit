use anyhow::{Context, Result};
use crawlkit_engine::gsc::{GscClient, GscError};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Run GSC (Google Search Console) analysis.
pub async fn run(
    site_url: Option<&str>,
    output: Option<&Path>,
    format: &str,
    start_date: &str,
    end_date: &str,
    dimension: &str,
    limit: usize,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message("Connecting to Google Search Console...");

    let mut client = GscClient::from_env().ok_or_else(|| {
        anyhow::anyhow!(
            "GSC credentials not found. Set GSC_ACCESS_TOKEN and GSC_SITE_URL environment variables."
        )
    })?;

    // Override site URL if provided via CLI
    if let Some(url) = site_url {
        let token = std::env::var("GSC_ACCESS_TOKEN")
            .context("GSC_ACCESS_TOKEN environment variable not set")?;
        client = GscClient::new(token, url.to_string());
    }

    pb.set_message(format!(
        "Fetching {} data for {} to {}...",
        dimension, start_date, end_date
    ));

    let analytics = match dimension {
        "query" => {
            let queries = client
                .top_queries(start_date, end_date, limit)
                .await
                .map_err(map_gsc_error)?;
            GscOutput::Queries(queries)
        }
        "page" => {
            let pages = client
                .top_pages(start_date, end_date, limit)
                .await
                .map_err(map_gsc_error)?;
            GscOutput::Pages(pages)
        }
        "all" => {
            let data = client
                .get_search_analytics(start_date, end_date)
                .await
                .map_err(map_gsc_error)?;
            GscOutput::Full(data)
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid dimension: {dimension}. Use query, page, or all."
            ));
        }
    };

    let output_str = match format {
        "json" => serde_json::to_string_pretty(&output_struc(&analytics))
            .context("Failed to serialize GSC data")?,
        "md" => build_markdown(&analytics),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format: {format}. Use json or md."
            ));
        }
    };

    pb.finish_with_message(match &analytics {
        GscOutput::Queries(q) => format!("Fetched {} queries", q.len()),
        GscOutput::Pages(p) => format!("Fetched {} pages", p.len()),
        GscOutput::Full(a) => format!(
            "Fetched {} queries, {} pages, {} total clicks",
            a.queries.len(),
            a.pages.len(),
            a.total_clicks
        ),
    });

    if let Some(out) = output {
        std::fs::write(out, &output_str)?;
        tracing::info!("Wrote GSC analysis to {}", out.display());
    } else {
        println!("{output_str}");
    }

    Ok(())
}

enum GscOutput {
    Queries(Vec<crawlkit_engine::gsc::GscRow>),
    Pages(Vec<crawlkit_engine::gsc::GscRow>),
    Full(crawlkit_engine::gsc::GscAnalytics),
}

fn output_struc(output: &GscOutput) -> serde_json::Value {
    match output {
        GscOutput::Queries(rows) => serde_json::json!({
            "dimension": "query",
            "count": rows.len(),
            "rows": rows,
        }),
        GscOutput::Pages(rows) => serde_json::json!({
            "dimension": "page",
            "count": rows.len(),
            "rows": rows,
        }),
        GscOutput::Full(analytics) => serde_json::json!({
            "dimension": "all",
            "total_clicks": analytics.total_clicks,
            "total_impressions": analytics.total_impressions,
            "average_ctr": analytics.average_ctr,
            "average_position": analytics.average_position,
            "queries": analytics.queries,
            "pages": analytics.pages,
        }),
    }
}

fn build_markdown(output: &GscOutput) -> String {
    let mut md = String::new();

    match output {
        GscOutput::Queries(rows) => {
            md.push_str("# GSC Top Queries\n\n");
            md.push_str("| Query | Clicks | Impressions | CTR | Position |\n");
            md.push_str("|-------|--------|-------------|-----|----------|\n");
            for row in rows {
                md.push_str(&format!(
                    "| {} | {} | {} | {:.2}% | {:.1} |\n",
                    row.key,
                    row.clicks,
                    row.impressions,
                    row.ctr * 100.0,
                    row.position,
                ));
            }
        }
        GscOutput::Pages(rows) => {
            md.push_str("# GSC Top Pages\n\n");
            md.push_str("| Page | Clicks | Impressions | CTR | Position |\n");
            md.push_str("|------|--------|-------------|-----|----------|\n");
            for row in rows {
                md.push_str(&format!(
                    "| {} | {} | {} | {:.2}% | {:.1} |\n",
                    row.key,
                    row.clicks,
                    row.impressions,
                    row.ctr * 100.0,
                    row.position,
                ));
            }
        }
        GscOutput::Full(analytics) => {
            md.push_str("# GSC Search Analytics\n\n");
            md.push_str(&format!(
                "- **Total Clicks:** {}\n\
                 - **Total Impressions:** {}\n\
                 - **Average CTR:** {:.2}%\n\
                 - **Average Position:** {:.1}\n\n",
                analytics.total_clicks,
                analytics.total_impressions,
                analytics.average_ctr * 100.0,
                analytics.average_position,
            ));

            if !analytics.queries.is_empty() {
                md.push_str("## Top Queries\n\n");
                md.push_str("| Query | Clicks | Impressions | CTR | Position |\n");
                md.push_str("|-------|--------|-------------|-----|----------|\n");
                for row in analytics.queries.iter().take(20) {
                    md.push_str(&format!(
                        "| {} | {} | {} | {:.2}% | {:.1} |\n",
                        row.key,
                        row.clicks,
                        row.impressions,
                        row.ctr * 100.0,
                        row.position,
                    ));
                }
                md.push('\n');
            }

            if !analytics.pages.is_empty() {
                md.push_str("## Top Pages\n\n");
                md.push_str("| Page | Clicks | Impressions | CTR | Position |\n");
                md.push_str("|------|--------|-------------|-----|----------|\n");
                for row in analytics.pages.iter().take(20) {
                    md.push_str(&format!(
                        "| {} | {} | {} | {:.2}% | {:.1} |\n",
                        row.key,
                        row.clicks,
                        row.impressions,
                        row.ctr * 100.0,
                        row.position,
                    ));
                }
            }
        }
    }

    md
}

fn map_gsc_error(e: GscError) -> anyhow::Error {
    match e {
        GscError::EnvMissing(var) => {
            anyhow::anyhow!("GSC environment variable not set: {var}")
        }
        GscError::RequestFailed(msg) => anyhow::anyhow!("GSC API request failed: {msg}"),
        GscError::ApiError { status, body } => {
            anyhow::anyhow!("GSC API error (HTTP {status}): {body}")
        }
        GscError::ParseError(msg) => anyhow::anyhow!("GSC response parse error: {msg}"),
    }
}
