use crate::storage_trait::StorageBackend;

use super::helpers::{
    read_issues_grouped_by_page, read_links_grouped_by_page, read_pages, ExportError,
};

/// Column selector for CSV export.
#[derive(Debug, Clone, Default)]
pub struct CsvColumnSelector {
    /// If true, include the page URL.
    pub url: bool,
    /// If true, include status code.
    pub status_code: bool,
    /// If true, include page title.
    pub title: bool,
    /// If true, include meta description.
    pub description: bool,
    /// If true, include canonical URL.
    pub canonical: bool,
    /// If true, include word count.
    pub word_count: bool,
    /// If true, include load time in ms.
    pub load_time_ms: bool,
    /// If true, include body size in bytes.
    pub body_size: bool,
    /// If true, include fetched_at timestamp.
    pub fetched_at: bool,
    /// If true, include has_structured_data.
    pub has_structured_data: bool,
    /// If true, include schema_types.
    pub schema_types: bool,
    /// If true, include viewport_ok.
    pub viewport_ok: bool,
    /// If true, include has_csp.
    pub has_csp: bool,
    /// If true, include has_hsts.
    pub has_hsts: bool,
    /// If true, include images_total.
    pub images_total: bool,
    /// If true, include images_missing_alt.
    pub images_missing_alt: bool,
    /// If true, include h1_count.
    pub h1_count: bool,
    /// If true, include heading_count.
    pub heading_count: bool,
    /// If true, include issue count per page.
    pub issue_count: bool,
    /// If true, include all issues as JSON array.
    pub issues_json: bool,
    /// If true, include all links as JSON array.
    pub links_json: bool,
    /// If true, include custom extractions as JSON object (one column per rule name).
    pub extractions_json: bool,
}

impl CsvColumnSelector {
    /// All columns enabled.
    pub fn all() -> Self {
        Self {
            url: true,
            status_code: true,
            title: true,
            description: true,
            canonical: true,
            word_count: true,
            load_time_ms: true,
            body_size: true,
            fetched_at: true,
            has_structured_data: true,
            schema_types: true,
            viewport_ok: true,
            has_csp: true,
            has_hsts: true,
            images_total: true,
            images_missing_alt: true,
            h1_count: true,
            heading_count: true,
            issue_count: true,
            issues_json: true,
            links_json: true,
            extractions_json: true,
        }
    }

    /// Return ordered list of enabled column names.
    pub fn headers(&self) -> Vec<&'static str> {
        let mut h = Vec::new();
        if self.url {
            h.push("url");
        }
        if self.status_code {
            h.push("status_code");
        }
        if self.title {
            h.push("title");
        }
        if self.description {
            h.push("description");
        }
        if self.canonical {
            h.push("canonical");
        }
        if self.word_count {
            h.push("word_count");
        }
        if self.load_time_ms {
            h.push("load_time_ms");
        }
        if self.body_size {
            h.push("body_size");
        }
        if self.fetched_at {
            h.push("fetched_at");
        }
        if self.has_structured_data {
            h.push("has_structured_data");
        }
        if self.schema_types {
            h.push("schema_types");
        }
        if self.viewport_ok {
            h.push("viewport_ok");
        }
        if self.has_csp {
            h.push("has_csp");
        }
        if self.has_hsts {
            h.push("has_hsts");
        }
        if self.images_total {
            h.push("images_total");
        }
        if self.images_missing_alt {
            h.push("images_missing_alt");
        }
        if self.h1_count {
            h.push("h1_count");
        }
        if self.heading_count {
            h.push("heading_count");
        }
        if self.issue_count {
            h.push("issue_count");
        }
        if self.issues_json {
            h.push("issues");
        }
        if self.links_json {
            h.push("links");
        }
        if self.extractions_json {
            h.push("extractions");
        }
        h
    }
}

/// Export crawl data to CSV.
///
/// One row per page. Nested data (issues, links) is JSON-encoded.
///
/// Issues and links are fetched in bulk (single query each) and grouped
/// by page_id in memory, avoiding N+1 query patterns for large crawls.
pub fn export_csv(
    storage: &dyn StorageBackend,
    crawl_id: &str,
    selector: &CsvColumnSelector,
) -> Result<Vec<u8>, ExportError> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(selector.headers())?;

    let pages = read_pages(storage, crawl_id)?;
    let issues_by_page = read_issues_grouped_by_page(storage, crawl_id)?;
    let links_by_page = read_links_grouped_by_page(storage, crawl_id)?;

    for page in &pages {
        let issues = issues_by_page.get(&page.id).cloned().unwrap_or_default();
        let links = links_by_page.get(&page.id).cloned().unwrap_or_default();
        let issue_count = issues.len();

        let mut record: Vec<String> = Vec::new();
        if selector.url {
            record.push(page.url.clone());
        }
        if selector.status_code {
            record.push(page.status_code.to_string());
        }
        if selector.title {
            record.push(page.title.clone().unwrap_or_default());
        }
        if selector.description {
            record.push(page.description.clone().unwrap_or_default());
        }
        if selector.canonical {
            record.push(page.canonical.clone().unwrap_or_default());
        }
        if selector.word_count {
            record.push(page.word_count.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.load_time_ms {
            record.push(page.load_time_ms.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.body_size {
            record.push(page.body_size.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.fetched_at {
            record.push(page.fetched_at.clone());
        }
        if selector.has_structured_data {
            record.push(
                page.has_structured_data
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            );
        }
        if selector.schema_types {
            record.push(page.schema_types.clone().unwrap_or_default());
        }
        if selector.viewport_ok {
            record.push(page.viewport_ok.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.has_csp {
            record.push(page.has_csp.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.has_hsts {
            record.push(page.has_hsts.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.images_total {
            record.push(page.images_total.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.images_missing_alt {
            record.push(
                page.images_missing_alt
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            );
        }
        if selector.h1_count {
            record.push(page.h1_count.map(|v| v.to_string()).unwrap_or_default());
        }
        if selector.heading_count {
            record.push(
                page.heading_count
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            );
        }
        if selector.issue_count {
            record.push(issue_count.to_string());
        }
        if selector.issues_json {
            record.push(serde_json::to_string(&issues).unwrap_or_default());
        }
        if selector.links_json {
            record.push(serde_json::to_string(&links).unwrap_or_default());
        }
        if selector.extractions_json {
            record.push(page.extractions.clone().unwrap_or_default());
        }
        wtr.write_record(&record)?;
    }

    wtr.flush()?;
    let bytes = wtr
        .into_inner()
        .map_err(|e| ExportError::Io(std::io::Error::other(e.to_string())))?;
    Ok(bytes)
}
