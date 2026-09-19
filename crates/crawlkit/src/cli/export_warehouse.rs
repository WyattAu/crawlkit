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

/// Destination family for `crawl export --layout <destination>`: print the
/// crawl-scoped layout plan (NDJSON) for a warehouse upload without touching
/// storage (6.0.0-alpha.1 groundwork).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LayoutDestination {
    /// S3 / data-lake object prefix (Parquet)
    S3,
    /// BigQuery dataset load (JSONL + `_schema`)
    Bigquery,
    /// Snowflake stage load (JSONL + `_schema`)
    Snowflake,
}

/// Print the destination layout plan for one crawl (NDJSON, one object).
pub fn run_layout(
    destination: LayoutDestination,
    crawl_id: &str,
    dataset: &str,
    stage: &str,
) -> anyhow::Result<()> {
    use crawlkit_engine::export::warehouse as wh;
    let layout = match destination {
        LayoutDestination::S3 => wh::s3_layout(crawl_id),
        LayoutDestination::Bigquery => wh::bigquery_layout(crawl_id, dataset)?,
        LayoutDestination::Snowflake => wh::snowflake_layout(crawl_id, stage)?,
    };
    println!(
        "{}",
        serde_json::to_string(&layout).context("serialize layout plan")?
    );
    Ok(())
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
    /// Upload destination URI (see the CLI help for the scheme grammar).
    /// `None` keeps the export local-only.
    pub upload: Option<String>,
    /// S3-compatible endpoint override (MinIO and friends); implies
    /// path-style addressing.
    pub s3_endpoint: Option<String>,
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

    let parquet;
    let jsonl;
    let (files, layout) = match params.format {
        ExportFormat::Parquet => {
            parquet = wh::export_crawl_parquet(&storage, &crawl_id, tenant, None)
                .context("warehouse export failed (ADR-016 contract violation)")?;
            let e = &parquet;
            (
                vec![
                    ("crawl_runs.parquet", &e.crawl_runs),
                    ("pages.parquet", &e.pages),
                    ("findings.parquet", &e.findings),
                ],
                wh::s3_layout(&crawl_id),
            )
        }
        ExportFormat::Jsonl => {
            jsonl = wh::export_crawl_jsonl(&storage, &crawl_id, tenant, None)
                .context("warehouse export failed (ADR-016 contract violation)")?;
            let e = &jsonl;
            (
                vec![
                    ("crawl_runs.jsonl", &e.crawl_runs),
                    ("pages.jsonl", &e.pages),
                    ("findings.jsonl", &e.findings),
                    ("_schema", &e.manifest),
                ],
                // The BigQuery/Snowflake prefix matches the S3 layout family
                // (ADR-016 §3: crawl-scoped `exports/{crawl_id}/…`).
                wh::s3_layout(&crawl_id),
            )
        }
    };

    for (name, bytes) in &files {
        write_out(&params.out_dir, name, bytes)?;
    }

    if let Some(uri) = &params.upload {
        upload_to_destination(
            uri,
            params.s3_endpoint.as_deref(),
            &layout,
            &files
                .iter()
                .map(|(n, b)| (*n, b.as_slice()))
                .collect::<Vec<_>>(),
            &crawl_id,
        )?;
    }

    println!(
        "exported crawl {crawl_id} as {} to {}{}",
        params.format.as_str(),
        params.out_dir.display(),
        if params.upload.is_some() {
            " (uploaded)"
        } else {
            ""
        }
    );
    Ok(())
}

/// Upload exported files to a warehouse destination. Credential material is
/// read from environment variables only — it never appears in CLI flags
/// (visible in `ps`), logs, or errors (5.3.0 §1 posture).
fn upload_to_destination(
    uri: &str,
    s3_endpoint: Option<&str>,
    layout: &crawlkit_engine::export::warehouse::DestinationLayout,
    files: &[(&str, &[u8])],
    _crawl_id: &str,
) -> anyhow::Result<()> {
    use crawlkit_engine::export::destinations::reqwest_transport::ReqwestTransport;
    use crawlkit_engine::export::destinations::*;

    let by_name: std::collections::BTreeMap<&str, &[u8]> =
        files.iter().map(|(n, b)| (*n, *b)).collect();
    let transport = ReqwestTransport::new();

    // Map table file names to their layout-plan object keys (crawl-scoped).
    let key_of = |table: &str, ext: &str| -> String {
        layout
            .files
            .iter()
            .find(|f| f.table == Some(table))
            .map(|f| f.name.clone())
            .unwrap_or_else(|| format!("{}/{}.{ext}", layout.prefix, table))
    };
    // Per-table BigQuery fields schemas derived from the ADR-016 manifests —
    // what load jobs embed (the `_schema` artifact is a version envelope for
    // readers, not a load-job schema).
    let bq_schema_of = |table: &str| -> anyhow::Result<Vec<u8>> {
        use crawlkit_engine::export::warehouse::{
            bigquery_schema_fields_json, crawl_runs_manifest, findings_manifest, pages_manifest,
        };
        let manifest = match table {
            "crawl_runs" => crawl_runs_manifest()?,
            "pages" => pages_manifest()?,
            "findings" => findings_manifest()?,
            other => anyhow::bail!("no ADR-016 manifest for table {other}"),
        };
        bigquery_schema_fields_json(manifest).map_err(|e| anyhow::anyhow!(e.to_string()))
    };
    let tables = [
        ("crawl_runs", "crawl_runs"),
        ("pages", "pages"),
        ("findings", "findings"),
    ];

    if let Some(bucket_uri) = uri.strip_prefix("s3://") {
        let bucket = bucket_uri.trim_end_matches('/');
        let access_key = std::env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| anyhow::anyhow!("AWS_ACCESS_KEY_ID not set (credentials are env-only)"))?;
        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| anyhow::anyhow!("AWS_SECRET_ACCESS_KEY not set"))?;
        let cfg = S3Config {
            bucket: bucket.to_string(),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".into()),
            endpoint: s3_endpoint
                .unwrap_or("https://s3.amazonaws.com")
                .to_string(),
            access_key,
            secret_key,
            session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
            path_style: s3_endpoint.is_some(),
        };
        let client = S3Client::new(cfg, &transport);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("build tokio runtime for upload")?;
        rt.block_on(async {
            for (table, _) in tables {
                let bytes = *by_name
                    .get(format!("{table}.parquet").as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "{table}.parquet not in export (need parquet format for s3)"
                        )
                    })?;
                let key = key_of(table, "parquet");
                client.put_object(&key, bytes.to_vec()).await?;
                println!("  s3://{bucket}/{key}");
            }
            Ok::<(), anyhow::Error>(())
        })?;
        return Ok(());
    }

    if let Some(rest) = uri.strip_prefix("bigquery://") {
        let (project, dataset) = rest
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("bigquery URI must be bigquery://PROJECT/DATASET"))?;
        let token = std::env::var("BIGQUERY_TOKEN")
            .map_err(|_| anyhow::anyhow!("BIGQUERY_TOKEN not set (credentials are env-only)"))?;
        let client = BigQueryClient::new(
            BigQueryConfig {
                project_id: project.to_string(),
                dataset: dataset.to_string(),
                bearer_token: token,
                endpoint: None,
            },
            &transport,
        );
        // The _schema artifact ships in the plan and is part of the reader
        // contract; the load schema itself comes from the manifest bindings
        // per table.
        if !by_name.contains_key("_schema") {
            anyhow::bail!("_schema manifest missing (need jsonl format for bigquery)");
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("build tokio runtime for upload")?;
        rt.block_on(async {
            for (table, _) in tables {
                let bytes = *by_name
                    .get(format!("{table}.jsonl").as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "{table}.jsonl not in export (need jsonl format for bigquery)"
                        )
                    })?;
                client
                    .load_jsonl(table, bytes.to_vec(), &bq_schema_of(table)?[..])
                    .await?;
                println!("  bigquery:{project}.{dataset}.{table}");
            }
            Ok::<(), anyhow::Error>(())
        })?;
        return Ok(());
    }

    if let Some(rest) = uri.strip_prefix("snowflake://") {
        // snowflake://ACCOUNT/DB/SCHEMA/STAGE
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() != 4 {
            return Err(anyhow::anyhow!(
                "snowflake URI must be snowflake://ACCOUNT/DB/SCHEMA/STAGE"
            ));
        }
        let (account, database, schema, stage) = (parts[0], parts[1], parts[2], parts[3]);
        let token = std::env::var("SNOWFLAKE_TOKEN")
            .map_err(|_| anyhow::anyhow!("SNOWFLAKE_TOKEN not set (credentials are env-only)"))?;
        let client = SnowflakeClient::new(
            SnowflakeConfig {
                account: account.to_string(),
                warehouse: std::env::var("SNOWFLAKE_WAREHOUSE")
                    .unwrap_or_else(|_| "COMPUTE_WH".into()),
                database: database.to_string(),
                schema: schema.to_string(),
                bearer_token: token,
                endpoint: None,
            },
            &transport,
        );
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("build tokio runtime for upload")?;
        rt.block_on(async {
            for (table, _) in tables {
                let path = key_of(table, "jsonl");
                client.copy_into(table, stage, &path).await?;
                println!("  snowflake:{account}.{database}.{schema}.{table}");
            }
            Ok::<(), anyhow::Error>(())
        })?;
        return Ok(());
    }

    Err(anyhow::anyhow!(
        "unsupported --upload URI: {uri} (expected s3://, bigquery://, or snowflake://)"
    ))
}

fn write_out(dir: &std::path::Path, name: &str, bytes: &[u8]) -> anyhow::Result<()> {
    let path = dir.join(name);
    std::fs::write(&path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    println!("  {}", path.display());
    Ok(())
}
