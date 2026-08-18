use anyhow::{Context, Result};
use crawlkit_engine::storage::Storage;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Analyze backlinks from an existing crawl.
pub async fn run(
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

    let link_pairs = storage.get_links_for_crawl(&crawl_id)?;
    let external_links = storage.get_external_links(&crawl_id)?;

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
        "json" => build_json(&crawl_id, &summary)?,
        "md" => build_markdown(&crawl_id, &summary),
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

fn build_json(crawl_id: &str, summary: &crawlkit_engine::BacklinkSummary) -> Result<String> {
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

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "crawl_id": crawl_id,
        "total_internal_links": summary.total_internal_links,
        "total_external_links": summary.total_external_links,
        "total_referring_domains": summary.total_referring_domains,
        "orphan_pages": summary.orphan_pages,
        "orphan_count": summary.orphan_pages.len(),
        "top_pages_by_pagerank": top_pages,
    }))?)
}

fn build_markdown(crawl_id: &str, summary: &crawlkit_engine::BacklinkSummary) -> String {
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
