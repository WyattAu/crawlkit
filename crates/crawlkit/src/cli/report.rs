use anyhow::{Context, Result};
use crawlkit_engine::storage::Storage;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Generate a report from an existing crawl.
pub fn run(
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

    let _encryption = crawlkit_engine::EncryptionManager::default();

    let crawl_id = storage
        .get_latest_crawl_id()
        .context("Failed to get latest crawl ID")?
        .ok_or_else(|| anyhow::anyhow!("No crawls found in database"))?;

    let stats = storage
        .get_stats(&crawl_id)
        .context("Failed to get crawl statistics")?;

    let backlink_data = if feature_flags.get(crawlkit_engine::feature_flags::FLAG_BACKLINK_ANALYSIS)
    {
        let link_pairs = storage.get_links_for_crawl(&crawl_id).unwrap_or_default();
        let external_links = storage.get_external_links(&crawl_id).unwrap_or_default();
        let page_urls = storage.get_page_urls(&crawl_id).unwrap_or_default();

        let mut analyzer = crawlkit_engine::BacklinkAnalyzer::new();
        analyzer.load_from_crawl_data(&link_pairs);

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

    let report = match format {
        "json" => build_json_report(&crawl_id, &stats, &backlink_data)?,
        "markdown" | "md" => build_markdown_report(&crawl_id, &stats, &backlink_data),
        "csv" => build_csv_report(&crawl_id, &stats),
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

fn build_json_report(
    crawl_id: &str,
    stats: &crawlkit_engine::storage::CrawlStats,
    backlink_data: &Option<(
        crawlkit_engine::BacklinkSummary,
        std::collections::HashMap<String, f64>,
    )>,
) -> Result<String> {
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

    if let Some((summary, _pagerank)) = backlink_data {
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

    Ok(serde_json::to_string_pretty(&data)?)
}

fn build_markdown_report(
    crawl_id: &str,
    stats: &crawlkit_engine::storage::CrawlStats,
    backlink_data: &Option<(
        crawlkit_engine::BacklinkSummary,
        std::collections::HashMap<String, f64>,
    )>,
) -> String {
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

    if let Some((summary, _)) = backlink_data {
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

fn build_csv_report(crawl_id: &str, stats: &crawlkit_engine::storage::CrawlStats) -> String {
    let mut csv = String::from("crawl_id,total_pages,total_issues,status\n");
    csv.push_str(&format!(
        "{},{},{},completed\n",
        crawl_id, stats.total_pages, stats.total_issues
    ));
    csv
}
