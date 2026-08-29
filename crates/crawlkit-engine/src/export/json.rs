use serde::Serialize;

use crate::storage::CrawlStats;
use crate::storage_trait::StorageBackend;

use super::helpers::{
    read_crawl_meta, read_issues_grouped_by_page, read_links_grouped_by_page, read_pages,
    ExportError,
};

/// Schema version for JSON exports.
pub const JSON_SCHEMA_VERSION: &str = "2.0";

/// Top-level JSON export structure.
#[derive(Serialize)]
pub struct JsonExport {
    /// Schema version.
    pub schema_version: String,
    /// Crawl metadata.
    pub crawl: JsonCrawlMeta,
    /// All pages.
    pub pages: Vec<JsonPage>,
    /// Aggregate stats (serialized with sorted map keys for
    /// byte-deterministic output).
    #[serde(serialize_with = "serialize_stats_canonical")]
    pub stats: CrawlStats,
}

/// Serialize [`CrawlStats`] with canonically sorted map keys.
///
/// The aggregate maps are `HashMap`s whose iteration order differs per
/// instance (and therefore per export call), which would make JSON output
/// byte-nondeterministic. Sorting the entries here pins the serialized
/// form so identical input always exports identical bytes.
fn serialize_stats_canonical<S>(stats: &CrawlStats, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;
    let severity: std::collections::BTreeMap<&str, &usize> = stats
        .issues_by_severity
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    let category: std::collections::BTreeMap<&str, &usize> = stats
        .issues_by_category
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();

    let mut map = serializer.serialize_map(Some(6))?;
    map.serialize_entry("total_pages", &stats.total_pages)?;
    map.serialize_entry("total_issues", &stats.total_issues)?;
    map.serialize_entry("issues_by_severity", &severity)?;
    map.serialize_entry("issues_by_category", &category)?;
    map.serialize_entry("avg_response_time_ms", &stats.avg_response_time_ms)?;
    map.serialize_entry("total_body_size", &stats.total_body_size)?;
    map.end()
}

#[derive(Serialize)]
pub struct JsonCrawlMeta {
    pub id: String,
    pub target_url: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub pages_crawled: usize,
    pub total_issues: usize,
}

#[derive(Serialize)]
pub struct JsonPage {
    pub id: String,
    pub url: String,
    pub final_url: String,
    pub status_code: u16,
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical: Option<String>,
    pub word_count: Option<usize>,
    pub load_time_ms: Option<u64>,
    pub body_size: Option<usize>,
    pub fetched_at: String,
    pub has_structured_data: Option<bool>,
    pub schema_types: Option<String>,
    pub viewport_ok: Option<bool>,
    pub has_csp: Option<bool>,
    pub has_hsts: Option<bool>,
    pub images_total: Option<usize>,
    pub images_missing_alt: Option<usize>,
    pub h1_count: Option<usize>,
    pub heading_count: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<JsonIssue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
}

#[derive(Serialize)]
pub struct JsonIssue {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub code: String,
    pub title: String,
    pub description: String,
    pub element: Option<String>,
    pub recommendation: String,
}

/// Export crawl data to a structured JSON value.
///
/// Issues and links are fetched in bulk (single query each) and grouped
/// by page_id in memory, avoiding N+1 query patterns for large crawls.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::storage::Storage;
/// use crawlkit_engine::export::export_json;
///
/// let storage = Storage::new_in_memory().unwrap();
/// let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
/// storage.finish_crawl(&crawl_id, 0, 0).unwrap();
///
/// let json = export_json(&storage, &crawl_id, false).unwrap();
/// assert!(json.contains("schema_version"));
/// ```
pub fn export_json(
    storage: &dyn StorageBackend,
    crawl_id: &str,
    pretty: bool,
) -> Result<String, ExportError> {
    let stats = storage.get_stats(crawl_id)?;
    let meta = read_crawl_meta(storage, crawl_id)?;
    let pages = read_pages(storage, crawl_id)?;
    let issues_by_page = read_issues_grouped_by_page(storage, crawl_id)?;
    let links_by_page = read_links_grouped_by_page(storage, crawl_id)?;

    let json_pages: Vec<JsonPage> = pages
        .iter()
        .map(|p| {
            let issues = issues_by_page
                .get(&p.id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|i| JsonIssue {
                    id: i.id,
                    category: i.category,
                    severity: i.severity,
                    code: i.code,
                    title: i.title,
                    description: i.description,
                    element: i.element,
                    recommendation: i.recommendation,
                })
                .collect();
            let links = links_by_page.get(&p.id).cloned().unwrap_or_default();
            JsonPage {
                id: p.id.clone(),
                url: p.url.clone(),
                final_url: p.final_url.clone(),
                status_code: p.status_code,
                title: p.title.clone(),
                description: p.description.clone(),
                canonical: p.canonical.clone(),
                word_count: p.word_count,
                load_time_ms: p.load_time_ms,
                body_size: p.body_size,
                fetched_at: p.fetched_at.clone(),
                has_structured_data: p.has_structured_data,
                schema_types: p.schema_types.clone(),
                viewport_ok: p.viewport_ok,
                has_csp: p.has_csp,
                has_hsts: p.has_hsts,
                images_total: p.images_total,
                images_missing_alt: p.images_missing_alt,
                h1_count: p.h1_count,
                heading_count: p.heading_count,
                issues,
                links,
            }
        })
        .collect();

    let export = JsonExport {
        schema_version: JSON_SCHEMA_VERSION.to_string(),
        crawl: meta,
        pages: json_pages,
        stats,
    };

    if pretty {
        Ok(serde_json::to_string_pretty(&export)?)
    } else {
        Ok(serde_json::to_string(&export)?)
    }
}
