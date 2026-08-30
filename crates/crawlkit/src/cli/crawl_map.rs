use anyhow::{Context, Result};
use crawlkit_engine::crawl_map::{ColorScheme, CrawlMapConfig};
use crawlkit_engine::storage::Storage;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Generate a visual crawl map SVG from existing crawl data.
pub fn run(
    crawl_path: &Path,
    output: &Path,
    color_by: &str,
    max_nodes: usize,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message("Generating crawl map...");

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

    let pages = storage
        .get_pages(&crawl_id, max_nodes)
        .context("Failed to retrieve pages")?;

    let links = storage
        .get_links_for_crawl(&crawl_id)
        .context("Failed to retrieve links")?;

    pb.set_message("Computing layout...");

    let color_scheme = match color_by {
        "depth" => ColorScheme::Depth,
        "popularity" => ColorScheme::Popularity,
        _ => ColorScheme::Status,
    };

    let config = CrawlMapConfig {
        max_nodes,
        color_by: color_scheme,
        ..CrawlMapConfig::default()
    };

    let svg = crawlkit_engine::crawl_map::generate_svg(&pages, &links, &config);

    std::fs::write(output, &svg)
        .with_context(|| format!("Failed to write SVG to {}", output.display()))?;

    pb.finish_with_message(format!("Crawl map written to {}", output.display()));
    Ok(())
}
