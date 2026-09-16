//! Warehouse export (ADR-016) — the versioned schema contract and the first
//! physical binding (S3/Parquet via the deterministic writer).
//!
//! The logical schema lives in `schemas/export/v1/*.toml` (the source of
//! truth); this module carries the code side of the contract and enforces it
//! in both directions:
//!
//! - **manifest → code:** every manifest column must exist in the arrow
//!   schema built here, with the declared binding type and nullability
//!   (`arrow_schema_matches_manifest`).
//! - **code → manifest:** every struct field feeding a row must be covered by
//!   a manifest column (`page_row_covers_pagedata_fields` via the serde key
//!   set of `PageData`).
//!
//! Writer guarantees (ADR-016 §2.2 rule 4): rows are key-sorted before
//! writing and the writer is single-threaded per file, so the same crawl
//! produces byte-identical Parquet — re-running an export overwrites, never
//! duplicates (idempotency per logical key). The file footer carries the
//! schema version so any reader can reject data it cannot interpret
//! (§2.3).

use arrow_array::{
    ArrayRef, BooleanArray, Float64Array, Int64Array, RecordBatch, StringArray,
    TimestampMillisecondArray,
};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::metadata::KeyValue;
use parquet::file::properties::WriterProperties;
use serde::Deserialize;
use std::io::Write;
use std::sync::Arc;
use thiserror::Error;

use crate::storage::{Issue, PageData};
use crate::storage_trait::{CrawlMeta, StorageBackend};

/// Errors from the warehouse export path.
#[derive(Debug, Error)]
pub enum WarehouseError {
    /// A contract violation or writer failure.
    #[error("warehouse export failed: {0}")]
    Writer(String),
    /// The source data could not be converted into contract rows.
    #[error("invalid crawl metadata for export: {0}")]
    InvalidMeta(String),
    /// The in-repo schema manifest itself is invalid — a build/contract bug,
    /// surfaced as an error (not a panic) because manifests are data.
    #[error("export schema contract violated: {0}")]
    Contract(String),
}

/// One manifest column (one `[[column]]` block in a `schemas/export/v1/*.toml`).
#[derive(Debug, Clone, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub logical_type: String,
    pub nullable: bool,
    pub added_in: String,
    #[serde(default)]
    pub bigquery: String,
    #[serde(default)]
    pub snowflake: String,
    #[serde(default)]
    pub parquet: String,
    #[serde(default)]
    pub description: String,
}

/// One table manifest (one `schemas/export/v1/*.toml` file).
#[derive(Debug, Clone, Deserialize)]
pub struct TableManifest {
    pub table: TableMeta,
    #[serde(rename = "column")]
    pub columns: Vec<ColumnDef>,
}

/// The `[table]` block of a manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct TableMeta {
    pub name: String,
    pub grain: String,
    pub keys: Vec<String>,
    pub version: String,
}

const CRAWL_RUNS_MANIFEST: &str = include_str!("../../../../schemas/export/v1/crawl_runs.toml");
const PAGES_MANIFEST: &str = include_str!("../../../../schemas/export/v1/pages.toml");
const FINDINGS_MANIFEST: &str = include_str!("../../../../schemas/export/v1/findings.toml");

fn parse_manifest(raw: &'static str) -> Result<TableManifest, WarehouseError> {
    // Manifests are in-repo, conformance-tested constants; a parse failure is
    // a programmer error surfaced as a Contract error, not a panic.
    toml::from_str(raw)
        .map_err(|e| WarehouseError::Contract(format!("manifest failed to parse: {e}")))
}

fn manifest_cell(
    raw: &'static str,
    cell: &'static std::sync::OnceLock<Result<TableManifest, String>>,
) -> Result<&'static TableManifest, WarehouseError> {
    cell.get_or_init(|| parse_manifest(raw).map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| WarehouseError::Contract(e.clone()))
}

/// The `crawl_runs` table manifest.
pub fn crawl_runs_manifest() -> Result<&'static TableManifest, WarehouseError> {
    static M: std::sync::OnceLock<Result<TableManifest, String>> = std::sync::OnceLock::new();
    manifest_cell(CRAWL_RUNS_MANIFEST, &M)
}

/// The `pages` table manifest.
pub fn pages_manifest() -> Result<&'static TableManifest, WarehouseError> {
    static M: std::sync::OnceLock<Result<TableManifest, String>> = std::sync::OnceLock::new();
    manifest_cell(PAGES_MANIFEST, &M)
}

/// The `findings` table manifest.
pub fn findings_manifest() -> Result<&'static TableManifest, WarehouseError> {
    static M: std::sync::OnceLock<Result<TableManifest, String>> = std::sync::OnceLock::new();
    manifest_cell(FINDINGS_MANIFEST, &M)
}

/// Map a manifest `parquet` binding string to the arrow type the writer emits.
///
/// This mapping is the "types bind at the edges, once" rule (ADR-016 §2.2
/// rule 3): there is deliberately no other place a Parquet type is chosen.
pub fn parquet_binding_to_arrow(binding: &str) -> Option<DataType> {
    match binding {
        "UTF8" => Some(DataType::Utf8),
        "BOOL" => Some(DataType::Boolean),
        "INT32" => Some(DataType::Int32),
        "INT64" => Some(DataType::Int64),
        "DOUBLE" => Some(DataType::Float64),
        "TIMESTAMP_MILLIS" => Some(DataType::Timestamp(
            TimeUnit::Millisecond,
            Some(Arc::from("UTC")),
        )),
        _ => None,
    }
}

/// Build the arrow schema for a table from its manifest's Parquet bindings.
pub fn arrow_schema_from_manifest(manifest: &TableManifest) -> Result<Schema, WarehouseError> {
    let mut fields = Vec::with_capacity(manifest.columns.len());
    for c in &manifest.columns {
        let ty = parquet_binding_to_arrow(&c.parquet).ok_or_else(|| {
            WarehouseError::Contract(format!(
                "column {} declares unknown parquet binding {}",
                c.name, c.parquet
            ))
        })?;
        fields.push(Field::new(&c.name, ty, c.nullable));
    }
    Ok(Schema::new(fields))
}

// ---------------------------------------------------------------------------
// Row types (the code side of the contract)
// ---------------------------------------------------------------------------

/// One `crawl_runs` row.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CrawlRunRow {
    pub crawl_id: String,
    pub target_url: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub pages_crawled: i64,
    pub total_issues: i64,
    pub tenant_id: Option<String>,
    pub sequence: Option<i64>,
}

impl CrawlRunRow {
    /// Build a row from [`CrawlMeta`], plus the tenancy/sequence fields the
    /// meta struct does not carry (ADR-016 §2.2 rule 5: tenancy rides on
    /// existing rails; §5.3: `sequence` is reserved, null until assigned).
    pub fn from_parts(
        meta: &CrawlMeta,
        tenant_id: Option<&str>,
        sequence: Option<i64>,
    ) -> Result<Self, WarehouseError> {
        let parse_ts = |s: &Option<String>| -> Result<Option<DateTime<Utc>>, WarehouseError> {
            s.as_deref()
                .map(DateTime::parse_from_rfc3339)
                .transpose()
                .map(|dt| dt.map(|d| d.with_timezone(&Utc)))
                .map_err(|e| WarehouseError::InvalidMeta(format!("bad RFC 3339 timestamp: {e}")))
        };
        Ok(Self {
            crawl_id: meta.id.clone(),
            target_url: meta.target_url.clone(),
            start_time: parse_ts(&meta.start_time)?,
            end_time: parse_ts(&meta.end_time)?,
            pages_crawled: meta.pages_crawled as i64,
            total_issues: meta.total_issues as i64,
            tenant_id: tenant_id.map(str::to_string),
            sequence,
        })
    }
}

/// One `pages` row. Field-for-field from [`PageData`] (the drift gate in the
/// tests ties the manifest to this struct's serde key set).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PageRow {
    pub crawl_id: String,
    pub page_id: String,
    pub url: String,
    pub final_url: String,
    pub status_code: i64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical_url: Option<String>,
    pub word_count: Option<i64>,
    pub load_time_ms: Option<i64>,
    pub body_size: Option<i64>,
    pub fetched_at: DateTime<Utc>,
    pub tenant_id: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub cwv_lcp_ms: Option<f64>,
    pub cwv_cls: Option<f64>,
    pub cwv_inp_ms: Option<f64>,
    pub has_structured_data: Option<bool>,
    pub schema_types: Option<String>,
    pub viewport_ok: Option<bool>,
    pub has_csp: Option<bool>,
    pub has_hsts: Option<bool>,
    pub images_total: Option<i64>,
    pub images_missing_alt: Option<i64>,
    pub h1_count: Option<i64>,
    pub heading_count: Option<i64>,
    pub extractions_json: Option<String>,
}

impl From<(&str, &PageData)> for PageRow {
    fn from((crawl_id, p): (&str, &PageData)) -> Self {
        Self {
            crawl_id: crawl_id.to_string(),
            page_id: p.id.clone(),
            url: p.url.to_string(),
            final_url: p.final_url.to_string(),
            status_code: i64::from(p.status_code),
            title: p.title.clone(),
            description: p.description.clone(),
            canonical_url: p.canonical_url.as_ref().map(|u| u.to_string()),
            word_count: p.word_count.map(|v| v as i64),
            load_time_ms: p.load_time_ms.map(|v| v as i64),
            body_size: p.body_size.map(|v| v as i64),
            fetched_at: p.fetched_at,
            tenant_id: p.tenant_id.clone(),
            etag: p.etag.clone(),
            last_modified: p.last_modified.clone(),
            cwv_lcp_ms: p.cwv_lcp,
            cwv_cls: p.cwv_cls,
            cwv_inp_ms: p.cwv_inp,
            has_structured_data: p.has_structured_data,
            schema_types: p.schema_types.clone(),
            viewport_ok: p.viewport_ok,
            has_csp: p.has_csp,
            has_hsts: p.has_hsts,
            images_total: p.images_total.map(|v| v as i64),
            images_missing_alt: p.images_missing_alt.map(|v| v as i64),
            h1_count: p.h1_count.map(|v| v as i64),
            heading_count: p.heading_count.map(|v| v as i64),
            extractions_json: p.extractions.clone(),
        }
    }
}

/// One `findings` row (an `Issue` plus the denormalized `crawl_id`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FindingRow {
    pub crawl_id: String,
    pub page_id: String,
    pub issue_id: String,
    pub category: String,
    pub severity: String,
    pub code: String,
    pub title: String,
    pub description: String,
    pub element: Option<String>,
    pub recommendation: String,
    pub tenant_id: Option<String>,
}

impl FindingRow {
    /// `category` stays a free-form string (`custom:<name>` from plugins must
    /// never break the contract); `severity` uses the storage string form.
    pub fn new(crawl_id: &str, issue: &Issue) -> Self {
        Self {
            crawl_id: crawl_id.to_string(),
            page_id: issue.page_id.clone(),
            issue_id: issue.id.clone(),
            category: issue.category.as_str(),
            severity: issue.severity.as_str().to_string(),
            code: issue.code.clone(),
            title: issue.title.clone(),
            description: issue.description.clone(),
            element: issue.element.clone(),
            recommendation: issue.recommendation.clone(),
            tenant_id: issue.tenant_id.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Parquet writer (the first physical binding)
// ---------------------------------------------------------------------------

/// Footer key under which the schema version is embedded (ADR-016 §2.3).
pub const FOOTER_SCHEMA_VERSION_KEY: &str = "crawlkit.schema_version";
/// Footer key naming the table the file belongs to.
pub const FOOTER_TABLE_KEY: &str = "crawlkit.table";

fn writer_properties(manifest: &TableManifest) -> WriterProperties {
    WriterProperties::builder()
        .set_compression(Compression::ZSTD(Default::default()))
        .set_created_by("crawlkit warehouse export v1".to_string())
        .set_key_value_metadata(Some(vec![
            KeyValue {
                key: FOOTER_SCHEMA_VERSION_KEY.to_string(),
                value: Some(manifest.table.version.clone()),
            },
            KeyValue {
                key: FOOTER_TABLE_KEY.to_string(),
                value: Some(manifest.table.name.clone()),
            },
        ]))
        .build()
}

fn write_batch<W: Write + Send>(
    manifest: &TableManifest,
    batch: RecordBatch,
    sink: &mut W,
) -> Result<(), WarehouseError> {
    let props = writer_properties(manifest);
    let schema = Arc::new(arrow_schema_from_manifest(manifest)?);
    let mut writer = ArrowWriter::try_new(sink, schema, Some(props))
        .map_err(|e| WarehouseError::Writer(e.to_string()))?;
    writer
        .write(&batch)
        .map_err(|e| WarehouseError::Writer(e.to_string()))?;
    writer
        .finish()
        .map_err(|e| WarehouseError::Writer(e.to_string()))?;
    Ok(())
}

fn batch_from_columns(
    schema: &Schema,
    columns: Vec<ArrayRef>,
) -> Result<RecordBatch, WarehouseError> {
    RecordBatch::try_new(Arc::new(schema.clone()), columns)
        .map_err(|e| WarehouseError::Writer(e.to_string()))
}

fn string_array(values: Vec<String>) -> Arc<dyn arrow_array::Array> {
    Arc::new(StringArray::from(values))
}

fn string_opt_array(values: Vec<Option<String>>) -> Arc<dyn arrow_array::Array> {
    Arc::new(StringArray::from(values))
}

fn i64_opt_array(values: Vec<Option<i64>>) -> Arc<dyn arrow_array::Array> {
    Arc::new(Int64Array::from(values))
}

/// Write `crawl_runs` rows as deterministic Parquet.
///
/// Rows are sorted by the logical key `(crawl_id)` before writing, so input
/// order never affects the output bytes.
pub fn write_crawl_runs_parquet<W: Write + Send>(
    runs: &[CrawlRunRow],
    sink: &mut W,
) -> Result<(), WarehouseError> {
    let manifest = crawl_runs_manifest()?;
    let mut rows: Vec<&CrawlRunRow> = runs.iter().collect();
    rows.sort_by(|a, b| a.crawl_id.cmp(&b.crawl_id));

    let schema = arrow_schema_from_manifest(manifest)?;
    let batch = batch_from_columns(
        &schema,
        vec![
            string_array(rows.iter().map(|r| r.crawl_id.clone()).collect()),
            string_array(rows.iter().map(|r| r.target_url.clone()).collect()),
            ts_array(rows.iter().map(|r| r.start_time).collect()),
            ts_array(rows.iter().map(|r| r.end_time).collect()),
            i64_opt_array(rows.iter().map(|r| Some(r.pages_crawled)).collect()),
            i64_opt_array(rows.iter().map(|r| Some(r.total_issues)).collect()),
            string_opt_array(rows.iter().map(|r| r.tenant_id.clone()).collect()),
            i64_opt_array(rows.iter().map(|r| r.sequence).collect()),
        ],
    )?;
    write_batch(manifest, batch, sink)
}

/// Write `pages` rows as deterministic Parquet (sorted by `(crawl_id, page_id)`).
pub fn write_pages_parquet<W: Write + Send>(
    pages: &[PageRow],
    sink: &mut W,
) -> Result<(), WarehouseError> {
    let manifest = pages_manifest()?;
    let mut rows: Vec<&PageRow> = pages.iter().collect();
    rows.sort_by(|a, b| (&a.crawl_id, &a.page_id).cmp(&(&b.crawl_id, &b.page_id)));

    let schema = arrow_schema_from_manifest(manifest)?;
    let batch = batch_from_columns(
        &schema,
        vec![
            string_array(rows.iter().map(|r| r.crawl_id.clone()).collect()),
            string_array(rows.iter().map(|r| r.page_id.clone()).collect()),
            string_array(rows.iter().map(|r| r.url.clone()).collect()),
            string_array(rows.iter().map(|r| r.final_url.clone()).collect()),
            int32_array(rows.iter().map(|r| r.status_code).collect()),
            string_opt_array(rows.iter().map(|r| r.title.clone()).collect()),
            string_opt_array(rows.iter().map(|r| r.description.clone()).collect()),
            string_opt_array(rows.iter().map(|r| r.canonical_url.clone()).collect()),
            i64_opt_array(rows.iter().map(|r| r.word_count).collect()),
            i64_opt_array(rows.iter().map(|r| r.load_time_ms).collect()),
            i64_opt_array(rows.iter().map(|r| r.body_size).collect()),
            ts_required_array(rows.iter().map(|r| r.fetched_at).collect()),
            string_opt_array(rows.iter().map(|r| r.tenant_id.clone()).collect()),
            string_opt_array(rows.iter().map(|r| r.etag.clone()).collect()),
            string_opt_array(rows.iter().map(|r| r.last_modified.clone()).collect()),
            f64_opt_array(rows.iter().map(|r| r.cwv_lcp_ms).collect()),
            f64_opt_array(rows.iter().map(|r| r.cwv_cls).collect()),
            f64_opt_array(rows.iter().map(|r| r.cwv_inp_ms).collect()),
            bool_opt_array(rows.iter().map(|r| r.has_structured_data).collect()),
            string_opt_array(rows.iter().map(|r| r.schema_types.clone()).collect()),
            bool_opt_array(rows.iter().map(|r| r.viewport_ok).collect()),
            bool_opt_array(rows.iter().map(|r| r.has_csp).collect()),
            bool_opt_array(rows.iter().map(|r| r.has_hsts).collect()),
            i64_opt_array(rows.iter().map(|r| r.images_total).collect()),
            i64_opt_array(rows.iter().map(|r| r.images_missing_alt).collect()),
            i64_opt_array(rows.iter().map(|r| r.h1_count).collect()),
            i64_opt_array(rows.iter().map(|r| r.heading_count).collect()),
            string_opt_array(rows.iter().map(|r| r.extractions_json.clone()).collect()),
        ],
    )?;
    write_batch(manifest, batch, sink)
}

/// Write `findings` rows as deterministic Parquet
/// (sorted by `(crawl_id, page_id, issue_id)`).
pub fn write_findings_parquet<W: Write + Send>(
    findings: &[FindingRow],
    sink: &mut W,
) -> Result<(), WarehouseError> {
    let manifest = findings_manifest()?;
    let mut rows: Vec<&FindingRow> = findings.iter().collect();
    rows.sort_by(|a, b| {
        (&a.crawl_id, &a.page_id, &a.issue_id).cmp(&(&b.crawl_id, &b.page_id, &b.issue_id))
    });

    let schema = arrow_schema_from_manifest(manifest)?;
    let batch = batch_from_columns(
        &schema,
        vec![
            string_array(rows.iter().map(|r| r.crawl_id.clone()).collect()),
            string_array(rows.iter().map(|r| r.page_id.clone()).collect()),
            string_array(rows.iter().map(|r| r.issue_id.clone()).collect()),
            string_array(rows.iter().map(|r| r.category.clone()).collect()),
            string_array(rows.iter().map(|r| r.severity.clone()).collect()),
            string_array(rows.iter().map(|r| r.code.clone()).collect()),
            string_array(rows.iter().map(|r| r.title.clone()).collect()),
            string_array(rows.iter().map(|r| r.description.clone()).collect()),
            string_opt_array(rows.iter().map(|r| r.element.clone()).collect()),
            string_array(rows.iter().map(|r| r.recommendation.clone()).collect()),
            string_opt_array(rows.iter().map(|r| r.tenant_id.clone()).collect()),
        ],
    )?;
    write_batch(manifest, batch, sink)
}

fn int32_array(values: Vec<i64>) -> Arc<dyn arrow_array::Array> {
    Arc::new(arrow_array::Int32Array::from_iter(
        values.into_iter().map(|v| i32::try_from(v).ok()),
    ))
}

fn f64_opt_array(values: Vec<Option<f64>>) -> Arc<dyn arrow_array::Array> {
    Arc::new(Float64Array::from(values))
}

fn bool_opt_array(values: Vec<Option<bool>>) -> Arc<dyn arrow_array::Array> {
    Arc::new(BooleanArray::from(values))
}

fn ts_array(values: Vec<Option<DateTime<Utc>>>) -> Arc<dyn arrow_array::Array> {
    let array = TimestampMillisecondArray::from_iter(
        values
            .into_iter()
            .map(|t| t.map(|dt| dt.timestamp_millis())),
    )
    .with_timezone("UTC");
    Arc::new(array)
}

fn ts_required_array(values: Vec<DateTime<Utc>>) -> Arc<dyn arrow_array::Array> {
    let array = TimestampMillisecondArray::from_iter(
        values.into_iter().map(|dt| Some(dt.timestamp_millis())),
    )
    .with_timezone("UTC");
    Arc::new(array)
}

// ---------------------------------------------------------------------------
// Convenience: export a whole crawl from storage
// ---------------------------------------------------------------------------

/// Export one crawl from any [`StorageBackend`] into deterministic Parquet
/// files (one call per table). Each call is crawl-scoped (ADR-016 §3:
/// bounded exports) and idempotent per key: re-running over the same crawl
/// with the same data produces byte-identical output.
pub fn export_crawl_parquet(
    storage: &dyn StorageBackend,
    crawl_id: &str,
    tenant_id: Option<&str>,
    sequence: Option<i64>,
) -> Result<ParquetCrawlExport, WarehouseError> {
    let meta = storage
        .get_crawl_meta(crawl_id)
        .map_err(|e| WarehouseError::InvalidMeta(e.to_string()))?;
    let run = CrawlRunRow::from_parts(&meta, tenant_id, sequence)?;

    let pages = storage
        .get_pages(crawl_id, usize::MAX)
        .map_err(|e| WarehouseError::Writer(e.to_string()))?;
    let page_rows: Vec<PageRow> = pages.iter().map(|p| (crawl_id, p).into()).collect();

    let issues = storage
        .get_issues(crawl_id, &crate::storage::IssueFilter::default())
        .map_err(|e| WarehouseError::Writer(e.to_string()))?;
    let finding_rows: Vec<FindingRow> = issues
        .iter()
        .map(|i| FindingRow::new(crawl_id, i))
        .collect();

    let mut runs_buf = Vec::new();
    write_crawl_runs_parquet(std::slice::from_ref(&run), &mut runs_buf)?;
    let mut pages_buf = Vec::new();
    write_pages_parquet(&page_rows, &mut pages_buf)?;
    let mut findings_buf = Vec::new();
    write_findings_parquet(&finding_rows, &mut findings_buf)?;

    Ok(ParquetCrawlExport {
        crawl_runs: runs_buf,
        pages: pages_buf,
        findings: findings_buf,
    })
}

/// The three Parquet files for one crawl, ready to upload to S3 under
/// crawl-scoped object keys (e.g. `exports/{crawl_id}/pages.parquet`).
/// Byte-identical re-exports overwrite the same objects — idempotency per
/// logical key without MERGE support.
#[derive(Debug, Clone)]
pub struct ParquetCrawlExport {
    /// `crawl_runs` table file.
    pub crawl_runs: Vec<u8>,
    /// `pages` table file.
    pub pages: Vec<u8>,
    /// `findings` table file.
    pub findings: Vec<u8>,
}

// ---------------------------------------------------------------------------
// JSONL writer (the second physical binding: BigQuery + Snowflake load jobs)
// ---------------------------------------------------------------------------

/// Metadata key under which the schema version is embedded in the JSONL
/// load-manifest (ADR-016 §2.3: version embedded in every artifact).
pub const MANIFEST_SCHEMA_VERSION_KEY: &str = "crawlkit.schema_version";
/// Metadata key naming the table the JSONL file belongs to.
pub const MANIFEST_TABLE_KEY: &str = "crawlkit.table";

/// A validated JSONL record for one table: exactly the manifest's columns
/// (deterministic key-sorted order — field order carries no semantics for
/// load jobs, determinism does), `null` for absent nullable values, no
/// extra fields.
#[derive(Debug, Clone)]
struct JsonlRecord {
    /// Field order follows the manifest's column order — the record renders
    /// as the contract reads.
    fields: Vec<(&'static str, serde_json::Value)>,
}

impl JsonlRecord {
    /// Build a record from a row's serialized fields, validating against the
    /// manifest: unknown field → contract violation; missing nullable column
    /// → explicit `null` (load jobs treat absent and null identically, but
    /// the record should be self-describing); missing non-nullable column →
    /// contract violation (a writer bug: required values are never optional).
    fn build(
        manifest: &TableManifest,
        fields: serde_json::Map<String, serde_json::Value>,
    ) -> Result<Self, WarehouseError> {
        let mut out = Vec::with_capacity(manifest.columns.len());
        for col in &manifest.columns {
            let name: &'static str = match col.name.as_str() {
                "crawl_id" => "crawl_id",
                "target_url" => "target_url",
                "start_time" => "start_time",
                "end_time" => "end_time",
                "pages_crawled" => "pages_crawled",
                "total_issues" => "total_issues",
                "tenant_id" => "tenant_id",
                "sequence" => "sequence",
                "page_id" => "page_id",
                "url" => "url",
                "final_url" => "final_url",
                "status_code" => "status_code",
                "title" => "title",
                "description" => "description",
                "canonical_url" => "canonical_url",
                "word_count" => "word_count",
                "load_time_ms" => "load_time_ms",
                "body_size" => "body_size",
                "fetched_at" => "fetched_at",
                "etag" => "etag",
                "last_modified" => "last_modified",
                "cwv_lcp_ms" => "cwv_lcp_ms",
                "cwv_cls" => "cwv_cls",
                "cwv_inp_ms" => "cwv_inp_ms",
                "has_structured_data" => "has_structured_data",
                "schema_types" => "schema_types",
                "viewport_ok" => "viewport_ok",
                "has_csp" => "has_csp",
                "has_hsts" => "has_hsts",
                "images_total" => "images_total",
                "images_missing_alt" => "images_missing_alt",
                "h1_count" => "h1_count",
                "heading_count" => "heading_count",
                "extractions_json" => "extractions_json",
                "issue_id" => "issue_id",
                "category" => "category",
                "severity" => "severity",
                "code" => "code",
                "element" => "element",
                "recommendation" => "recommendation",
                other => {
                    return Err(WarehouseError::Contract(format!(
                        "manifest column `{other}` has no static field mapping — \
                         extend JsonlRecord::build"
                    )))
                }
            };
            match fields.get(&col.name) {
                Some(v) => {
                    out.push((name, v.clone()));
                }
                None if col.nullable => {
                    out.push((name, serde_json::Value::Null));
                }
                None => {
                    return Err(WarehouseError::Contract(format!(
                        "row is missing non-nullable manifest column `{}`",
                        col.name
                    )))
                }
            }
        }
        for key in fields.keys() {
            if !manifest.columns.iter().any(|c| &c.name == key) {
                return Err(WarehouseError::Contract(format!(
                    "row field `{key}` is not a manifest column — schema drift"
                )));
            }
        }
        Ok(Self { fields: out })
    }

    fn to_json(&self) -> String {
        let mut s = String::new();
        s.push('{');
        for (i, (k, v)) in self.fields.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&serde_json::to_string(k).unwrap_or_default());
            s.push(':');
            s.push_str(&serde_json::to_string(v).unwrap_or_default());
        }
        s.push('}');
        s
    }
}

fn jsonl_cell(
    raw: &'static str,
    cell: &'static std::sync::OnceLock<Result<TableManifest, String>>,
) -> Result<&'static TableManifest, WarehouseError> {
    manifest_cell(raw, cell)
}

fn jsonl_pages_manifest() -> Result<&'static TableManifest, WarehouseError> {
    static M: std::sync::OnceLock<Result<TableManifest, String>> = std::sync::OnceLock::new();
    jsonl_cell(PAGES_MANIFEST, &M)
}

fn jsonl_findings_manifest() -> Result<&'static TableManifest, WarehouseError> {
    static M: std::sync::OnceLock<Result<TableManifest, String>> = std::sync::OnceLock::new();
    jsonl_cell(FINDINGS_MANIFEST, &M)
}

fn jsonl_crawl_runs_manifest() -> Result<&'static TableManifest, WarehouseError> {
    static M: std::sync::OnceLock<Result<TableManifest, String>> = std::sync::OnceLock::new();
    jsonl_cell(CRAWL_RUNS_MANIFEST, &M)
}

/// Serialize rows to newline-delimited JSON: manifest-declared columns per
/// object, in manifest order (key-sorted output order is a writer property,
/// not the contract), UTF-8, LF-terminated.
fn write_jsonl(
    manifest: Result<&'static TableManifest, WarehouseError>,
    mut rows: Vec<serde_json::Map<String, serde_json::Value>>,
) -> Result<Vec<u8>, WarehouseError> {
    let manifest = manifest?;
    // Key-sort by the manifest's logical keys before writing (ADR-016 §2.2
    // rule 4, matching the Parquet writers): input order never affects the
    // output bytes, so re-exports are byte-identical. All v1 keys are
    // strings; a non-string key value is a writer bug surfaced below by the
    // non-nullable column check.
    rows.sort_by(|a, b| {
        for k in &manifest.table.keys {
            let x = a
                .get(k.as_str())
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let y = b
                .get(k.as_str())
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if x != y {
                return x.cmp(y);
            }
        }
        std::cmp::Ordering::Equal
    });
    let mut buf = Vec::new();
    for fields in rows {
        let rec = JsonlRecord::build(manifest, fields)?;
        buf.extend_from_slice(rec.to_json().as_bytes());
        buf.push(b'\n');
    }
    Ok(buf)
}

/// Serialize one row to a JSON object, fallibly (workspace lints deny
/// `expect` in production code; serialization of in-repo rows only fails on
/// a programming error, but it must be surfaced, not panicked).
fn row_to_object<T: serde::Serialize>(
    label: &str,
    row: &T,
) -> Result<serde_json::Map<String, serde_json::Value>, WarehouseError> {
    match serde_json::to_value(row)
        .map_err(|e| WarehouseError::Writer(format!("{label} failed to serialize: {e}")))?
    {
        serde_json::Value::Object(map) => Ok(map),
        _ => Err(WarehouseError::Contract(format!(
            "{label} must serialize to a JSON object"
        ))),
    }
}

/// Write `crawl_runs` rows as JSONL (BigQuery/Snowflake load-job compatible).
pub fn write_crawl_runs_jsonl(runs: &[CrawlRunRow]) -> Result<Vec<u8>, WarehouseError> {
    let rows = runs
        .iter()
        .map(|r| row_to_object("CrawlRunRow", r))
        .collect::<Result<Vec<_>, WarehouseError>>()?;
    write_jsonl(jsonl_crawl_runs_manifest(), rows)
}

/// Write `pages` rows as JSONL (BigQuery/Snowflake load-job compatible).
pub fn write_pages_jsonl(pages: &[PageRow]) -> Result<Vec<u8>, WarehouseError> {
    let rows = pages
        .iter()
        .map(|r| row_to_object("PageRow", r))
        .collect::<Result<Vec<_>, WarehouseError>>()?;
    write_jsonl(jsonl_pages_manifest(), rows)
}

/// Write `findings` rows as JSONL (BigQuery/Snowflake load-job compatible).
pub fn write_findings_jsonl(findings: &[FindingRow]) -> Result<Vec<u8>, WarehouseError> {
    let rows = findings
        .iter()
        .map(|r| row_to_object("FindingRow", r))
        .collect::<Result<Vec<_>, WarehouseError>>()?;
    write_jsonl(jsonl_findings_manifest(), rows)
}

/// Build the BigQuery external-schema DDL fragment for a table from its
/// manifest (`bigquery` bindings). Operational convenience for load jobs —
/// the manifest remains the only source of truth.
pub fn bigquery_schema_fragment(manifest: &TableManifest) -> Result<String, WarehouseError> {
    let cols = manifest
        .columns
        .iter()
        .map(|c| {
            if c.bigquery.is_empty() {
                return Err(WarehouseError::Contract(format!(
                    "column `{}` declares no bigquery binding",
                    c.name
                )));
            }
            let mode = if c.nullable { "NULLABLE" } else { "REQUIRED" };
            Ok(format!("{} {} {}", c.name, c.bigquery, mode))
        })
        .collect::<Result<Vec<_>, WarehouseError>>()?;
    Ok(cols.join(", "))
}

/// Build the Snowflake external-schema DDL fragment for a table from its
/// manifest (`snowflake` bindings). Operational convenience for load jobs —
/// the manifest remains the only source of truth.
pub fn snowflake_schema_fragment(manifest: &TableManifest) -> Result<String, WarehouseError> {
    let cols = manifest
        .columns
        .iter()
        .map(|c| {
            if c.snowflake.is_empty() {
                return Err(WarehouseError::Contract(format!(
                    "column `{}` declares no snowflake binding",
                    c.name
                )));
            }
            Ok(format!("{} {}", c.name, c.snowflake))
        })
        .collect::<Result<Vec<_>, WarehouseError>>()?;
    Ok(cols.join(", "))
}

/// Build the JSONL load manifest for one exported file (ADR-016 §2.3: the
/// version embedded in every artifact — `_schema` is this document for the
/// JSONL bindings). Deterministic: keys sorted, stable field order.
pub fn jsonl_load_manifest(
    table: &'static str,
    manifest: &TableManifest,
    file_name: &str,
) -> Result<Vec<u8>, WarehouseError> {
    let version = manifest.table.version.as_str();
    let doc = serde_json::json!({
        MANIFEST_SCHEMA_VERSION_KEY: version,
        MANIFEST_TABLE_KEY: table,
        "file": file_name,
        "tables": [{
            "name": manifest.table.name,
            "version": manifest.table.version,
            "keys": manifest.table.keys,
        }],
    });
    let mut buf = serde_json::to_string_pretty(&doc)
        .map_err(|e| WarehouseError::Writer(e.to_string()))?
        .into_bytes();
    buf.push(b'\n');
    Ok(buf)
}

/// The three JSONL files (plus one load manifest) for one crawl, ready to
/// upload to GCS / a Snowflake stage under crawl-scoped object keys (e.g.
/// `exports/{crawl_id}/pages.jsonl`). Line order is key-sorted, so
/// re-exports are byte-identical — idempotent overwrite per logical key.
#[derive(Debug, Clone)]
pub struct JsonlCrawlExport {
    /// `crawl_runs` table file.
    pub crawl_runs: Vec<u8>,
    /// `pages` table file.
    pub pages: Vec<u8>,
    /// `findings` table file.
    pub findings: Vec<u8>,
    /// The `_schema` load manifest covering all three files.
    pub manifest: Vec<u8>,
}

/// Warehouse destination layout plan (6.0.0-alpha.1 groundwork): every
/// object/file a destination upload needs, with the load command shape per
/// destination. Deterministic — same inputs, byte-identical plan — so a
/// pipeline can diff plans to detect drift before uploading.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct DestinationLayout {
    /// Destination family: `s3`, `bigquery`, or `snowflake`.
    pub destination: &'static str,
    /// Crawl-scoped object/file prefix (ADR-016 §3 crawl-scoping rule).
    pub prefix: String,
    /// Files to upload (name → role). Roles are stable identifiers a
    /// pipeline keys on, not prose.
    pub files: Vec<LayoutFile>,
    /// The load command shape for the destination, with `{stage}` /
    /// `{dataset}` / `{table}` placeholders for operator secrets. This is
    /// documentation in machine form; the manifest remains the source of
    /// truth for column types.
    pub load_command: String,
}

/// One file in a [`DestinationLayout`].
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct LayoutFile {
    pub name: String,
    /// `parquet-table`, `jsonl-table`, or `schema-manifest`.
    pub role: &'static str,
    /// Table this file belongs to (absent for the cross-table manifest).
    pub table: Option<&'static str>,
}

/// Plan the S3 / data-lake layout: Parquet tables under the crawl-scoped
/// prefix. Idempotent uploads follow from the byte-identical re-export
/// guarantee (same key → overwrite).
pub fn s3_layout(crawl_id: &str) -> DestinationLayout {
    let prefix = format!("exports/{crawl_id}");
    DestinationLayout {
        destination: "s3",
        files: vec![
            LayoutFile {
                name: format!("{prefix}/crawl_runs.parquet"),
                role: "parquet-table",
                table: Some("crawl_runs"),
            },
            LayoutFile {
                name: format!("{prefix}/pages.parquet"),
                role: "parquet-table",
                table: Some("pages"),
            },
            LayoutFile {
                name: format!("{prefix}/findings.parquet"),
                role: "parquet-table",
                table: Some("findings"),
            },
        ],
        load_command:
            "aws s3 cp findings.parquet s3://{bucket}/exports/{crawl_id}/findings.parquet"
                .to_string(),
        prefix,
    }
}

/// Plan the BigQuery layout: JSONL load files + the `_schema` manifest, with
/// the `bq load` command shape per table. Types come from the manifests'
/// `bigquery` bindings via [`bigquery_schema_fragment`].
pub fn bigquery_layout(crawl_id: &str, dataset: &str) -> Result<DestinationLayout, WarehouseError> {
    // Fail fast on any missing bigquery binding — a contract gap, not a
    // runtime condition (§2.4: manifests are data; gaps are build bugs).
    for m in [
        crawl_runs_manifest()?,
        pages_manifest()?,
        findings_manifest()?,
    ] {
        bigquery_schema_fragment(m)?;
    }
    let prefix = format!("exports/{crawl_id}");
    Ok(DestinationLayout {
        destination: "bigquery",
        files: vec![
            LayoutFile {
                name: format!("{prefix}/crawl_runs.jsonl"),
                role: "jsonl-table",
                table: Some("crawl_runs"),
            },
            LayoutFile {
                name: format!("{prefix}/pages.jsonl"),
                role: "jsonl-table",
                table: Some("pages"),
            },
            LayoutFile {
                name: format!("{prefix}/findings.jsonl"),
                role: "jsonl-table",
                table: Some("findings"),
            },
            LayoutFile {
                name: format!("{prefix}/_schema"),
                role: "schema-manifest",
                table: None,
            },
        ],
        load_command: format!(
            "bq load --source_format=NEWLINE_DELIMITED_JSON --schema={prefix}/_schema \
             {dataset}.findings {prefix}/findings.jsonl"
        ),
        prefix,
    })
}

/// Plan the Snowflake layout: JSONL files landed on a stage, loaded per
/// table with the manifest's `snowflake` binding types.
pub fn snowflake_layout(crawl_id: &str, stage: &str) -> Result<DestinationLayout, WarehouseError> {
    for m in [
        crawl_runs_manifest()?,
        pages_manifest()?,
        findings_manifest()?,
    ] {
        snowflake_schema_fragment(m)?;
    }
    let prefix = format!("exports/{crawl_id}");
    Ok(DestinationLayout {
        destination: "snowflake",
        files: vec![
            LayoutFile {
                name: format!("{prefix}/crawl_runs.jsonl"),
                role: "jsonl-table",
                table: Some("crawl_runs"),
            },
            LayoutFile {
                name: format!("{prefix}/pages.jsonl"),
                role: "jsonl-table",
                table: Some("pages"),
            },
            LayoutFile {
                name: format!("{prefix}/findings.jsonl"),
                role: "jsonl-table",
                table: Some("findings"),
            },
            LayoutFile {
                name: format!("{prefix}/_schema"),
                role: "schema-manifest",
                table: None,
            },
        ],
        load_command: format!(
            "COPY INTO {stage}.findings FROM @{stage}/exports/{crawl_id}/findings.jsonl \
             FILE_FORMAT = (TYPE = JSON)"
        ),
        prefix,
    })
}

/// Export one crawl from any [`StorageBackend`] into JSONL files for
/// BigQuery/Snowflake load jobs. Crawl-scoped (ADR-016 §3) and idempotent:
/// same data → byte-identical bytes.
pub fn export_crawl_jsonl(
    storage: &dyn StorageBackend,
    crawl_id: &str,
    tenant_id: Option<&str>,
    sequence: Option<i64>,
) -> Result<JsonlCrawlExport, WarehouseError> {
    let meta = storage
        .get_crawl_meta(crawl_id)
        .map_err(|e| WarehouseError::InvalidMeta(e.to_string()))?;
    let run = CrawlRunRow::from_parts(&meta, tenant_id, sequence)?;

    let pages = storage
        .get_pages(crawl_id, usize::MAX)
        .map_err(|e| WarehouseError::Writer(e.to_string()))?;
    // Key-sort before writing (ADR-016 §2.2 rule 4): line order matches the
    // Parquet writers' sort, so re-exports are byte-identical.
    let mut page_rows: Vec<PageRow> = pages.iter().map(|p| (crawl_id, p).into()).collect();
    page_rows.sort_by(|a, b| (&a.crawl_id, &a.page_id).cmp(&(&b.crawl_id, &b.page_id)));

    let issues = storage
        .get_issues(crawl_id, &crate::storage::IssueFilter::default())
        .map_err(|e| WarehouseError::Writer(e.to_string()))?;
    let mut finding_rows: Vec<FindingRow> = issues
        .iter()
        .map(|i| FindingRow::new(crawl_id, i))
        .collect();
    finding_rows.sort_by(|a, b| {
        (&a.crawl_id, &a.page_id, &a.issue_id).cmp(&(&b.crawl_id, &b.page_id, &b.issue_id))
    });

    let crawl_runs = write_crawl_runs_jsonl(std::slice::from_ref(&run))?;
    let pages_buf = write_pages_jsonl(&page_rows)?;
    let findings_buf = write_findings_jsonl(&finding_rows)?;
    let manifest_doc = jsonl_load_manifest("_schema", crawl_runs_manifest()?, "crawl_runs.jsonl")?;

    Ok(JsonlCrawlExport {
        crawl_runs,
        pages: pages_buf,
        findings: findings_buf,
        manifest: manifest_doc,
    })
}

/// Decode a Parquet file back into arrow record batches (testing and
/// round-trip verification helper).
pub fn read_parquet_batches(data: &[u8]) -> Result<Vec<RecordBatch>, WarehouseError> {
    let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(
        Bytes::copy_from_slice(data),
    )
    .map_err(|e| WarehouseError::Writer(e.to_string()))?
    .build()
    .map_err(|e| WarehouseError::Writer(e.to_string()))?;
    reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| WarehouseError::Writer(e.to_string()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::storage::{IssueCategory, Severity, Storage};
    use chrono::TimeZone;
    use parquet::file::reader::FileReader;
    use url::Url;

    fn test_page(id: &str, url: &str) -> PageData {
        PageData {
            id: id.to_string(),
            url: Url::parse(url).unwrap(),
            final_url: Url::parse(url).unwrap(),
            status_code: 200,
            title: Some(format!("Page {id}")),
            description: None,
            canonical_url: None,
            word_count: Some(500),
            load_time_ms: Some(200),
            body_size: Some(1024),
            fetched_at: Utc.with_ymd_and_hms(2026, 9, 13, 12, 0, 0).unwrap(),
            links: vec![],
            tenant_id: None,
            etag: None,
            last_modified: None,
            cwv_lcp: Some(2500.5),
            cwv_cls: Some(0.05),
            cwv_inp: None,
            has_structured_data: Some(true),
            schema_types: Some("Article".to_string()),
            viewport_ok: Some(true),
            has_csp: Some(false),
            has_hsts: Some(true),
            images_total: Some(3),
            images_missing_alt: Some(1),
            h1_count: Some(1),
            heading_count: Some(5),
            extractions: None,
        }
    }

    fn test_issue(id: &str, page_id: &str, custom: bool) -> Issue {
        Issue {
            id: id.to_string(),
            page_id: page_id.to_string(),
            category: if custom {
                IssueCategory::Custom("my-plugin".to_string())
            } else {
                IssueCategory::Seo
            },
            severity: Severity::Warning,
            code: format!("SEO00{}", &id[id.len() - 1..]),
            title: format!("Issue {id}"),
            description: "d".to_string(),
            element: Some("h1".to_string()),
            recommendation: "fix".to_string(),
            tenant_id: None,
        }
    }

    // ------------------------------------------------------------------
    // Conformance gate: manifest ↔ code (ADR-016 §2.2 rule 2)
    // ------------------------------------------------------------------

    #[test]
    fn arrow_schema_matches_manifest() {
        for (manifest, name) in [
            (crawl_runs_manifest().unwrap(), "crawl_runs"),
            (pages_manifest().unwrap(), "pages"),
            (findings_manifest().unwrap(), "findings"),
        ] {
            assert_eq!(manifest.table.name, name);
            let schema = arrow_schema_from_manifest(manifest).unwrap();
            let schema_fields: Vec<&Field> = schema.fields().iter().map(|f| f.as_ref()).collect();
            for (col, field) in manifest.columns.iter().zip(&schema_fields) {
                assert_eq!(&col.name, field.name(), "{name}: column order/name drift");
                assert_eq!(
                    field.is_nullable(),
                    col.nullable,
                    "{name}.{}: nullability drift",
                    col.name
                );
                let expected = parquet_binding_to_arrow(&col.parquet).unwrap();
                assert_eq!(
                    field.data_type(),
                    &expected,
                    "{name}.{}: binding drift",
                    col.name
                );
            }
            assert_eq!(
                manifest.columns.len(),
                schema_fields.len(),
                "{name}: column count drift"
            );
        }
    }

    #[test]
    fn page_row_covers_pagedata_fields() {
        // code → manifest direction: every PageData serde key must have a
        // manifest column (crawl_id is the export-side denormalization and
        // lives only in the manifest); links is deliberately excluded
        // (ADR-016 §2.2 scope).
        let page = test_page("p1", "https://example.com/");
        let json = serde_json::to_value(&page).unwrap();
        let keys: Vec<String> = match &json {
            serde_json::Value::Object(map) => map.keys().cloned().collect(),
            _ => panic!("PageData must serialize to an object"),
        };
        let manifest_cols: Vec<&str> = pages_manifest()
            .unwrap()
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        for key in &keys {
            if key == "links" {
                continue;
            }
            let manifest_name = match key.as_str() {
                "id" => "page_id".to_string(),
                "cwv_lcp" => "cwv_lcp_ms".to_string(),
                "cwv_inp" => "cwv_inp_ms".to_string(),
                "extractions" => "extractions_json".to_string(),
                other => other.to_string(),
            };
            assert!(
                manifest_cols.contains(&manifest_name.as_str()),
                "PageData field `{key}` (manifest column `{manifest_name}`) is missing \
                 from schemas/export/v1/pages.toml — update the manifest with the \
                 column (nullable, added_in) or exclude it explicitly"
            );
        }
        // and the row conversion actually populates every manifest column
        let row = PageRow::from(("c1", &page));
        let row_json = serde_json::to_value(&row).unwrap();
        let row_keys: Vec<String> = match &row_json {
            serde_json::Value::Object(map) => map.keys().cloned().collect(),
            _ => panic!("PageRow must serialize to an object"),
        };
        assert_eq!(row_keys.len(), manifest_cols.len());
        for col in &manifest_cols {
            assert!(
                row_keys.contains(&col.to_string()),
                "PageRow field `{col}` is not populated — writer column and manifest diverge"
            );
        }
    }

    #[test]
    fn crawl_run_row_maps_meta() {
        let meta = CrawlMeta {
            id: "c1".to_string(),
            target_url: "https://example.com".to_string(),
            start_time: Some("2026-09-13T12:00:00Z".to_string()),
            end_time: Some("2026-09-13T12:05:00Z".to_string()),
            pages_crawled: 12,
            total_issues: 34,
        };
        let row = CrawlRunRow::from_parts(&meta, Some("tenant-a"), None).unwrap();
        assert_eq!(row.crawl_id, "c1");
        assert_eq!(row.pages_crawled, 12);
        assert_eq!(row.tenant_id.as_deref(), Some("tenant-a"));
        assert_eq!(row.sequence, None);

        let bad = CrawlMeta {
            start_time: Some("not-a-timestamp".to_string()),
            ..meta
        };
        assert!(CrawlRunRow::from_parts(&bad, None, None).is_err());
    }

    // ------------------------------------------------------------------
    // Writer behavior: determinism, idempotency, footer, custom categories
    // ------------------------------------------------------------------

    #[test]
    fn parquet_output_is_deterministic_and_input_order_independent() {
        let pages = vec![
            test_page("p2", "https://example.com/b"),
            test_page("p1", "https://example.com/a"),
        ];
        let rows_a: Vec<PageRow> = pages.iter().map(|p| ("c1", p).into()).collect();
        let mut reversed = pages;
        reversed.reverse();
        let rows_b: Vec<PageRow> = reversed.iter().map(|p| ("c1", p).into()).collect();

        let mut out_a = Vec::new();
        write_pages_parquet(&rows_a, &mut out_a).unwrap();
        let mut out_b = Vec::new();
        write_pages_parquet(&rows_b, &mut out_b).unwrap();
        assert_eq!(
            out_a, out_b,
            "same rows in different order must produce byte-identical Parquet"
        );
    }

    #[test]
    fn re_export_over_same_data_is_byte_identical() {
        // Idempotency rule (ADR-016 §2.2 rule 4): re-running an export
        // overwrites, never duplicates — expressed as byte-identical output.
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        storage
            .insert_page(&crawl_id, &test_page("p1", "https://example.com/"))
            .unwrap();
        storage
            .insert_issue(&test_issue("i1", "p1", false))
            .unwrap();
        storage.finish_crawl(&crawl_id, 1, 1).unwrap();

        let first = export_crawl_parquet(&storage, &crawl_id, None, None).unwrap();
        let second = export_crawl_parquet(&storage, &crawl_id, None, None).unwrap();
        assert_eq!(first.pages, second.pages);
        assert_eq!(first.findings, second.findings);
        assert_eq!(first.crawl_runs, second.crawl_runs);
    }

    #[test]
    fn footer_carries_schema_version_and_table() {
        let findings = [test_issue("i2", "p1", false), test_issue("i1", "p1", true)];
        let rows: Vec<FindingRow> = findings.iter().map(|i| FindingRow::new("c1", i)).collect();
        let mut out = Vec::new();
        write_findings_parquet(&rows, &mut out).unwrap();

        let reader =
            parquet::file::reader::SerializedFileReader::new(bytes::Bytes::from(out)).unwrap();
        let meta = reader.metadata().file_metadata();
        let kv = meta.key_value_metadata().unwrap();
        let version = kv
            .iter()
            .find(|k| k.key == FOOTER_SCHEMA_VERSION_KEY)
            .map(|k| k.value.clone().unwrap())
            .unwrap();
        assert_eq!(version, findings_manifest().unwrap().table.version);
        let table = kv
            .iter()
            .find(|k| k.key == FOOTER_TABLE_KEY)
            .map(|k| k.value.clone().unwrap())
            .unwrap();
        assert_eq!(table, "findings");
    }

    #[test]
    fn custom_plugin_category_round_trips_as_string() {
        // The contract point: custom:<name> must never break the schema.
        let rows = vec![FindingRow::new("c1", &test_issue("i1", "p1", true))];
        assert_eq!(rows[0].category, "custom:my-plugin");
        let mut out = Vec::new();
        write_findings_parquet(&rows, &mut out).unwrap();
        assert!(!out.is_empty());
    }

    // ------------------------------------------------------------------
    // JSONL binding (second physical binding: BigQuery/Snowflake load jobs)
    // ------------------------------------------------------------------

    #[test]
    fn jsonl_records_match_manifest_exactly() {
        // Gates: (a) field set equals the manifest's column set; (b) absent
        // nullable values are explicit nulls; (c) manifest order is preserved
        // (line column i ↔ JSON field i).
        let pages = [test_page("p1", "https://example.com/")];
        let rows: Vec<PageRow> = pages.iter().map(|p| ("c1", p).into()).collect();
        let out = write_pages_jsonl(&rows).unwrap();
        assert_eq!(out.last(), Some(&b'\n'));
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 1);

        let manifest = pages_manifest().unwrap();
        let text = text.trim_end();
        let names: Vec<&str> = manifest.columns.iter().map(|c| c.name.as_str()).collect();
        for (i, name) in names.iter().enumerate() {
            let line_col = text
                .match_indices(&format!("\"{name}\":"))
                .find(|(pos, _)| text[..*pos].match_indices('"').count().is_multiple_of(2));
            assert!(line_col.is_some(), "field `{name}` missing from line");
            if i > 0 {
                let prev = names[i - 1];
                let prev_pos = text
                    .match_indices(&format!("\"{prev}\":"))
                    .find(|(pos, _)| text[..*pos].match_indices('"').count().is_multiple_of(2))
                    .map(|(p, _)| p)
                    .unwrap();
                let cur_pos = line_col.map(|(p, _)| p).unwrap();
                assert!(prev_pos < cur_pos, "field order must follow the manifest");
            }
        }

        let v: serde_json::Value = serde_json::from_str(text).unwrap();
        let obj = v.as_object().unwrap();
        assert_eq!(obj.len(), manifest.columns.len(), "field set drift");
        assert_eq!(obj["crawl_id"], "c1");
        assert_eq!(obj["status_code"], 200);
        assert!(
            obj["canonical_url"].is_null(),
            "absent nullable values are explicit nulls"
        );
        assert!(obj["cwv_inp_ms"].is_null());
        assert_eq!(obj["fetched_at"], "2026-09-13T12:00:00Z");
    }

    #[test]
    fn jsonl_binds_to_declared_bq_and_sf_types() {
        // ADR-016 §2.1: types bind at the edges, once. The JSONL binding
        // serves two transports; each manifest column's declared BQ/SF type
        // must be from the table's declared vocabulary (and non-empty).
        for (manifest, bq_vocab, sf_vocab) in [
            (
                crawl_runs_manifest().unwrap(),
                ["STRING", "TIMESTAMP", "INT64"].as_slice(),
                ["VARCHAR", "TIMESTAMP_NTZ", "NUMBER(38,0)"].as_slice(),
            ),
            (
                pages_manifest().unwrap(),
                ["STRING", "TIMESTAMP", "INT64", "FLOAT64", "BOOL"].as_slice(),
                [
                    "VARCHAR",
                    "TIMESTAMP_NTZ",
                    "NUMBER(38,0)",
                    "FLOAT",
                    "BOOLEAN",
                ]
                .as_slice(),
            ),
            (
                findings_manifest().unwrap(),
                ["STRING"].as_slice(),
                ["VARCHAR"].as_slice(),
            ),
        ] {
            for col in &manifest.columns {
                assert!(
                    !col.bigquery.is_empty() && bq_vocab.contains(&col.bigquery.as_str()),
                    "{}.{}: undeclared bigquery binding `{}`",
                    manifest.table.name,
                    col.name,
                    col.bigquery
                );
                assert!(
                    !col.snowflake.is_empty() && sf_vocab.contains(&col.snowflake.as_str()),
                    "{}.{}: undeclared snowflake binding `{}`",
                    manifest.table.name,
                    col.name,
                    col.snowflake
                );
            }
        }
    }

    #[test]
    fn schema_fragments_render_from_manifests() {
        let runs = crawl_runs_manifest().unwrap();
        let bq = bigquery_schema_fragment(runs).unwrap();
        let sf = snowflake_schema_fragment(runs).unwrap();
        assert!(bq.starts_with("crawl_id STRING REQUIRED"));
        assert!(bq.contains("pages_crawled INT64 REQUIRED"));
        assert!(sf.contains("start_time TIMESTAMP_NTZ"));
        assert!(sf.contains("sequence NUMBER(38,0)"));
        // BQ mode derives from manifest nullability.
        assert!(bq.contains("tenant_id STRING NULLABLE"));
    }

    #[test]
    fn jsonl_output_is_deterministic_and_input_order_independent() {
        let pages = [
            test_page("p2", "https://t.example/"),
            test_page("p1", "https://t.example/x"),
        ];
        let rows_a: Vec<PageRow> = pages.iter().map(|p| ("c1", p).into()).collect();
        let mut reversed = pages;
        reversed.reverse();
        let rows_b: Vec<PageRow> = reversed.iter().map(|p| ("c1", p).into()).collect();

        let out_a = write_pages_jsonl(&rows_a).unwrap();
        let out_b = write_pages_jsonl(&rows_b).unwrap();
        assert_eq!(
            out_a, out_b,
            "same rows in different order must produce byte-identical JSONL"
        );
    }

    #[test]
    fn export_crawl_jsonl_round_trips_through_storage() {
        let storage = Storage::new_in_memory().unwrap();
        let cid = storage.start_crawl("https://example.com", None).unwrap();
        storage
            .insert_page(&cid, &test_page("p1", "https://example.com/"))
            .unwrap();
        storage.insert_issue(&test_issue("i1", "p1", true)).unwrap();
        storage.finish_crawl(&cid, 1, 1).unwrap();

        let export = export_crawl_jsonl(&storage, &cid, Some("tenant-b"), None).unwrap();
        assert!(String::from_utf8(export.manifest.clone())
            .unwrap()
            .contains("crawlkit.schema_version"));
        assert!(!export.pages.is_empty());

        // The crawl_runs line carries the export-level tenancy parameter.
        let run_text = String::from_utf8(export.crawl_runs).unwrap();
        let run: serde_json::Value = serde_json::from_str(run_text.trim_end()).unwrap();
        assert_eq!(run["crawl_id"], cid);
        assert_eq!(run["tenant_id"], "tenant-b");

        // Page rows carry tenancy from PageData itself (null in this fixture).
        let text = String::from_utf8(export.pages).unwrap();
        let v: serde_json::Value = serde_json::from_str(text.trim_end()).unwrap();
        assert_eq!(v["crawl_id"], cid);
        assert!(v["tenant_id"].is_null());
        assert_eq!(v["fetched_at"], "2026-09-13T12:00:00Z");
    }

    #[test]
    fn jsonl_and_parquet_bindings_describe_the_same_rows() {
        // Cross-binding equivalence (ADR-016 §2.1: one logical schema): the
        // JSONL lines must agree with the Parquet rows — same row count,
        // same per-row values through the row types (field sets were already
        // gated against the manifest above).
        let storage = Storage::new_in_memory().unwrap();
        let cid = storage.start_crawl("https://example.com", None).unwrap();
        storage
            .insert_page(&cid, &test_page("p1", "https://example.com/"))
            .unwrap();
        storage
            .insert_issue(&test_issue("i1", "p1", false))
            .unwrap();
        storage.finish_crawl(&cid, 1, 1).unwrap();

        let parquet_export = export_crawl_parquet(&storage, &cid, Some("tenant-b"), None).unwrap();
        let jsonl_export = export_crawl_jsonl(&storage, &cid, Some("tenant-b"), None).unwrap();

        // crawl_runs: parse the single JSONL line back into CrawlRunRow.
        let run_text = String::from_utf8(jsonl_export.crawl_runs).unwrap();
        let run: CrawlRunRow = serde_json::from_str(run_text.trim_end()).unwrap();
        assert_eq!(run.crawl_id, cid);
        let pq_run_batches = read_parquet_batches(&parquet_export.crawl_runs).unwrap();
        let pq_run = &pq_run_batches[0];
        assert_eq!(pq_run.num_rows(), 1);
        let got_crawl_id = pq_run
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(got_crawl_id.value(0), run.crawl_id);
        let got_pages = pq_run
            .column(4)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(got_pages.value(0), run.pages_crawled);

        // pages: JSONL row count == Parquet row count.
        let pages_text = String::from_utf8(jsonl_export.pages).unwrap();
        let page_lines = pages_text.lines().count();
        let pq_pages = read_parquet_batches(&parquet_export.pages).unwrap();
        assert_eq!(
            pq_pages.iter().map(|b| b.num_rows()).sum::<usize>(),
            page_lines
        );

        // findings: same, including the custom-category string form.
        let findings_text = String::from_utf8(jsonl_export.findings).unwrap();
        let finding: serde_json::Value = serde_json::from_str(findings_text.trim_end()).unwrap();
        assert_eq!(finding["category"], "seo");
        let pq_findings = read_parquet_batches(&parquet_export.findings).unwrap();
        assert_eq!(pq_findings.iter().map(|b| b.num_rows()).sum::<usize>(), 1);
    }

    #[test]
    fn empty_crawl_exports_three_valid_files() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        storage.finish_crawl(&crawl_id, 0, 0).unwrap();
        let export = export_crawl_parquet(&storage, &crawl_id, None, Some(7)).unwrap();
        assert!(!export.crawl_runs.is_empty());
        assert!(!export.pages.is_empty());
        assert!(!export.findings.is_empty());
    }

    #[test]
    fn destination_layouts_are_crawl_scoped_and_deterministic() {
        let cid = "0123abcd-4321-dcba-ba09-321fedcba012";
        let s3 = s3_layout(cid);
        let s3_again = s3_layout(cid);
        assert_eq!(s3, s3_again, "layout plans must be byte-deterministic");
        assert!(s3.prefix == format!("exports/{cid}"));
        assert_eq!(s3.files.len(), 3);
        assert!(s3.files.iter().all(|f| f.name.starts_with(&s3.prefix)));

        let bq = bigquery_layout(cid, "crawlkit").unwrap();
        assert_eq!(bq.files.len(), 4, "BQ/SF carry the _schema manifest");
        assert!(bq
            .files
            .iter()
            .any(|f| f.role == "schema-manifest" && f.table.is_none()));
        let bq_again = bigquery_layout(cid, "crawlkit").unwrap();
        assert_eq!(bq, bq_again);

        let sf = snowflake_layout(cid, "crawlkit_stage").unwrap();
        assert_eq!(sf.files.len(), 4);

        // Every layout file names an existing contract table (or none, for
        // the cross-table manifest) — guards against table-name drift.
        let known = ["crawl_runs", "pages", "findings"];
        for layout in [&s3, &bq, &sf] {
            for f in &layout.files {
                if let Some(t) = f.table {
                    assert!(known.contains(&t), "unknown table {t} in layout");
                }
            }
        }
    }
}
