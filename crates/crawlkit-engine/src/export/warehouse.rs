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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
}
