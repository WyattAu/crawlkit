//! `crawl export-warehouse` — user-facing access to the ADR-016 warehouse
//! bindings (Parquet for S3/data-lake targets; JSONL load files for
//! BigQuery/Snowflake).
//!
//! Reads a completed crawl out of a crawlkit storage database and writes
//! the three contract tables (`crawl_runs`, `pages`, `findings`) as files
//! named per the load-job convention. Byte-identical re-exports are
//! guaranteed by the exporter (key-sorted, deterministic), so re-running
//! this command over the same crawl is always idempotent.
//!
//! Exit codes: 0 on success, 1 on invalid arguments (bad database,
//! unknown crawl, no crawls present), 2 on exporter failure (contract
//! violation, write error) — mapped by `anyhow`'s default top-level
//! handling in `main`.

use std::path::PathBuf;

use anyhow::Context;

/// The physical binding to emit (ADR-016 §2.1: one logical schema,
/// physical bindings declared per column in `schemas/export/v1/*.toml`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ExportFormat {
    /// Columnar Parquet files (S3 / data-lake targets)
    Parquet,
    /// Newline-delimited JSON + `_schema` load manifest (BigQuery / Snowflake)
    Jsonl,
}

impl ExportFormat {
    fn as_str(self) -> &'static str {
        match self {
            ExportFormat::Parquet => "parquet",
            ExportFormat::Jsonl => "jsonl",
        }
    }
}

pub struct ExportParams {
    /// Path to the crawlkit storage database (typically `<out>/crawlkit.db`).
    pub db: PathBuf,
    /// Crawl to export; `None` means "the only crawl in the database".
    pub crawl_id: Option<String>,
    /// Tenant scoping recorded on the `crawl_runs` row (multi-tenant exports).
    pub tenant: Option<String>,
    pub format: ExportFormat,
    /// Directory receiving the table files (created if missing).
    pub out_dir: PathBuf,
}

pub fn run(params: ExportParams) -> anyhow::Result<()> {
    use crawlkit_engine::export::warehouse as wh;
    use crawlkit_engine::storage::Storage;

    let storage = Storage::new(&params.db)
        .with_context(|| format!("failed to open storage at {}", params.db.display()))?;

    // Resolve the crawl: explicit id, else the single crawl in the database,
    // else list what's available and fail with a helpful message.
    let crawl_id = match &params.crawl_id {
        Some(id) => {
            storage
                .get_crawl_meta(id)
                .with_context(|| format!("crawl '{id}' not found in {}", params.db.display()))?;
            id.clone()
        }
        None => {
            let crawls = storage.list_crawls().context("failed to list crawls")?;
            match crawls.len() {
                1 => crawls[0].0.clone(),
                0 => anyhow::bail!(
                    "no crawls found in {}; run a crawl first or pass --crawl-id",
                    params.db.display()
                ),
                _ => anyhow::bail!(
                    "{} crawls found in {}; pass --crawl-id with one of: {}",
                    crawls.len(),
                    params.db.display(),
                    crawls
                        .iter()
                        .map(|(id, _)| id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        }
    };

    let tenant = params.tenant.as_deref();
    std::fs::create_dir_all(&params.out_dir).with_context(|| {
        format!(
            "failed to create output directory {}",
            params.out_dir.display()
        )
    })?;

    match params.format {
        ExportFormat::Parquet => {
            let export = wh::export_crawl_parquet(&storage, &crawl_id, tenant, None)
                .context("warehouse export failed (ADR-016 contract violation)")?;
            write_out(&params.out_dir, "crawl_runs.parquet", &export.crawl_runs)?;
            write_out(&params.out_dir, "pages.parquet", &export.pages)?;
            write_out(&params.out_dir, "findings.parquet", &export.findings)?;
        }
        ExportFormat::Jsonl => {
            let export = wh::export_crawl_jsonl(&storage, &crawl_id, tenant, None)
                .context("warehouse export failed (ADR-016 contract violation)")?;
            write_out(&params.out_dir, "crawl_runs.jsonl", &export.crawl_runs)?;
            write_out(&params.out_dir, "pages.jsonl", &export.pages)?;
            write_out(&params.out_dir, "findings.jsonl", &export.findings)?;
            write_out(&params.out_dir, "_schema", &export.manifest)?;
        }
    }

    println!(
        "exported crawl {crawl_id} as {} to {}",
        params.format.as_str(),
        params.out_dir.display()
    );
    Ok(())
}

fn write_out(dir: &std::path::Path, name: &str, bytes: &[u8]) -> anyhow::Result<()> {
    let path = dir.join(name);
    std::fs::write(&path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    println!("  {}", path.display());
    Ok(())
}
