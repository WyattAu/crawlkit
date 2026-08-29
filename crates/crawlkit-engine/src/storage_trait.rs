use crate::storage::{CrawlStats, CruxMetrics, Issue, IssueFilter, PageData, StorageError};

/// Metadata about a single crawl, returned by [`StorageBackend::get_crawl_meta`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrawlMeta {
    /// Unique crawl identifier.
    pub id: String,
    /// The seed / target URL of the crawl.
    pub target_url: String,
    /// When the crawl started (RFC 3339 string).
    pub start_time: Option<String>,
    /// When the crawl finished (RFC 3339 string).
    pub end_time: Option<String>,
    /// Number of pages successfully crawled.
    pub pages_crawled: usize,
    /// Total issues discovered.
    pub total_issues: usize,
}

/// An aggregated issue row for the "top issues" section of export reports.
///
/// Each row represents a unique `(severity, code, title)` tuple with the
/// count of distinct pages it affects.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TopIssue {
    /// Issue severity (`critical`, `error`, `warning`, `info`).
    pub severity: String,
    /// Machine-readable issue code (e.g. `SEO001`).
    pub code: String,
    /// Human-readable issue title.
    pub title: String,
    /// Number of distinct pages affected.
    pub affected_pages: usize,
}

/// A trait abstracting storage backends for crawl data.
///
/// Implementors provide persistent or in-memory storage for pages,
/// issues, and crawl metadata. The SQLite-backed [`Storage`](crate::storage::Storage)
/// struct implements this trait, and an in-memory variant is provided
/// for testing.
pub trait StorageBackend: Send + Sync {
    /// Start a new crawl and return its unique identifier.
    fn start_crawl(&self, seed_url: &str, tenant_id: Option<&str>) -> Result<String, StorageError>;

    /// Mark a crawl as finished, recording final statistics.
    fn finish_crawl(&self, crawl_id: &str, pages: usize, issues: usize)
        -> Result<(), StorageError>;

    /// Insert a single page into the database under the given crawl.
    fn insert_page(&self, crawl_id: &str, page: &PageData) -> Result<(), StorageError>;

    /// Insert a batch of pages for performance.
    fn insert_pages_batch(&self, crawl_id: &str, pages: &[PageData]) -> Result<(), StorageError>;

    /// Retrieve a single page by crawl ID and URL.
    fn get_page(&self, crawl_id: &str, url: &str) -> Result<Option<PageData>, StorageError>;

    /// Retrieve pages with a limit.
    fn get_pages(&self, crawl_id: &str, limit: usize) -> Result<Vec<PageData>, StorageError>;

    /// Retrieve pages for a specific tenant.
    fn get_pages_for_tenant(
        &self,
        crawl_id: &str,
        tenant_id: &str,
        limit: usize,
    ) -> Result<Vec<PageData>, StorageError>;

    /// Insert a single issue/finding.
    fn insert_issue(&self, issue: &Issue) -> Result<(), StorageError>;

    /// Insert a batch of issues for performance.
    fn insert_issues_batch(&self, issues: &[Issue]) -> Result<(), StorageError>;

    /// Retrieve issues with optional filters.
    fn get_issues(&self, crawl_id: &str, filters: &IssueFilter)
        -> Result<Vec<Issue>, StorageError>;

    /// Retrieve issues for a specific tenant.
    fn get_issues_for_tenant(
        &self,
        crawl_id: &str,
        tenant_id: &str,
        filters: &IssueFilter,
    ) -> Result<Vec<Issue>, StorageError>;

    /// Get aggregate statistics for a crawl.
    fn get_stats(&self, crawl_id: &str) -> Result<CrawlStats, StorageError>;

    /// Get conditional request data (page_id, etag, last_modified) for a URL within a crawl.
    fn get_page_conditional(
        &self,
        crawl_id: &str,
        url: &str,
    ) -> Result<Option<(String, Option<String>, Option<String>)>, StorageError>;

    /// Get the most recent ETag and Last-Modified for a URL across all crawls.
    fn get_latest_conditional(
        &self,
        url: &str,
    ) -> Result<Option<(Option<String>, Option<String>)>, StorageError>;

    /// Update the `fetched_at` timestamp of an existing page (304 Not Modified path).
    fn update_page_fetched_at(
        &self,
        page_id: &str,
        fetched_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), StorageError>;

    /// Update the Core Web Vitals fields of an existing page.
    fn update_page_cwv(
        &self,
        page_id: &str,
        cwv_lcp: Option<f64>,
        cwv_cls: Option<f64>,
        cwv_inp: Option<f64>,
    ) -> Result<(), StorageError>;

    /// List all crawl IDs with their timestamps, ordered chronologically.
    fn list_crawls(&self) -> Result<Vec<(String, String)>, StorageError>;

    /// Get the most recent crawl ID.
    fn get_latest_crawl_id(&self) -> Result<Option<String>, StorageError>;

    /// Get the most recent crawl other than `exclude_id`.
    fn get_previous_crawl_id(&self, exclude_id: &str) -> Result<Option<String>, StorageError>;

    /// Get all page URLs for a crawl.
    fn get_page_urls(&self, crawl_id: &str) -> Result<Vec<String>, StorageError>;

    /// Get all links for a crawl, grouped by source URL.
    fn get_links_for_crawl(
        &self,
        crawl_id: &str,
    ) -> Result<Vec<(String, Vec<String>)>, StorageError>;

    /// Get all external links for a crawl.
    fn get_external_links(&self, crawl_id: &str) -> Result<Vec<(String, String)>, StorageError>;

    /// Get metadata for a single crawl (target URL, timestamps, counts).
    fn get_crawl_meta(&self, crawl_id: &str) -> Result<CrawlMeta, StorageError>;

    /// Get the top issues for a crawl, ordered by severity rank (critical
    /// first) then by number of affected pages descending.
    fn get_top_issues(
        &self,
        crawl_id: &str,
        limit: usize,
    ) -> Result<Vec<TopIssue>, StorageError>;

    /// Get CrUX field metrics for all pages in a crawl.
    fn get_crux_metrics_for_crawl(
        &self,
        crawl_id: &str,
    ) -> Result<Vec<CruxMetrics>, StorageError>;

    /// Release any resources held by the backend (connections, caches, etc.).
    fn finish(&self) -> Result<(), StorageError>;

    /// Purge crawls older than `max_age_days` days.
    ///
    /// Returns the number of crawls deleted.
    fn purge_old_crawls(&self, max_age_days: u32) -> Result<usize, StorageError>;

    /// Compare two crawls within the same storage backend.
    ///
    /// Returns a [`CrawlDiff`](crate::compare::CrawlDiff) describing
    /// added, removed, and changed pages between `baseline_crawl_id` and
    /// `target_crawl_id`.
    fn compare_crawls(
        &self,
        baseline_crawl_id: &str,
        target_crawl_id: &str,
    ) -> Result<crate::compare::CrawlDiff, StorageError>;
}

/// Create an in-memory storage backend suitable for testing.
///
/// Returns a boxed [`StorageBackend`] backed by an in-memory SQLite database.
pub fn new_in_memory_backend() -> Result<Box<dyn StorageBackend>, StorageError> {
    let storage = crate::storage::Storage::new_in_memory()?;
    Ok(Box::new(storage))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{IssueCategory, Severity, Storage};
    use chrono::Utc;
    use url::Url;

    fn test_page(id: &str, url: &str, status: u16) -> PageData {
        PageData {
            id: id.to_string(),
            url: Url::parse(url).unwrap(),
            final_url: Url::parse(url).unwrap(),
            status_code: status,
            title: Some(format!("Page {id}")),
            description: None,
            canonical_url: None,
            word_count: Some(500),
            load_time_ms: Some(200),
            body_size: Some(1024),
            fetched_at: Utc::now(),
            links: vec![],
            tenant_id: None,
            etag: None,
            last_modified: None,
            cwv_lcp: None,
            cwv_cls: None,
            cwv_inp: None,
            has_structured_data: None,
            schema_types: None,
            viewport_ok: None,
            has_csp: None,
            has_hsts: None,
            images_total: None,
            images_missing_alt: None,
            h1_count: None,
            heading_count: None,
            extractions: None,
        }
    }

    fn test_issue(id: &str, page_id: &str, category: IssueCategory, severity: Severity) -> Issue {
        Issue {
            id: id.to_string(),
            page_id: page_id.to_string(),
            category,
            severity,
            code: format!("{}001", id),
            title: format!("Issue {id}"),
            description: format!("Description for issue {id}"),
            element: None,
            recommendation: "Fix this".to_string(),
            tenant_id: None,
        }
    }

    #[test]
    fn test_trait_start_and_finish_crawl() {
        let storage: Box<dyn StorageBackend> = new_in_memory_backend().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        assert!(!crawl_id.is_empty());
        storage.finish_crawl(&crawl_id, 0, 0).unwrap();
        storage.finish().unwrap();
    }

    #[test]
    fn test_trait_insert_and_get_page() {
        let storage: Box<dyn StorageBackend> = new_in_memory_backend().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let page = test_page("p1", "https://example.com/", 200);
        storage.insert_page(&crawl_id, &page).unwrap();

        let retrieved = storage.get_page(&crawl_id, "https://example.com/").unwrap();
        assert!(retrieved.is_some());
        let p = retrieved.unwrap();
        assert_eq!(p.id, "p1");
        assert_eq!(p.status_code, 200);

        let missing = storage
            .get_page(&crawl_id, "https://example.com/missing")
            .unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_trait_insert_pages_batch() {
        let storage: Box<dyn StorageBackend> = new_in_memory_backend().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let pages = vec![
            test_page("p1", "https://example.com/", 200),
            test_page("p2", "https://example.com/about", 200),
        ];
        storage.insert_pages_batch(&crawl_id, &pages).unwrap();

        let retrieved = storage.get_pages(&crawl_id, 10).unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn test_trait_tenant_isolation() {
        let storage: Box<dyn StorageBackend> = new_in_memory_backend().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let mut page_a = test_page("p1", "https://example.com/a", 200);
        page_a.tenant_id = Some("tenant_a".to_string());
        let mut page_b = test_page("p2", "https://example.com/b", 200);
        page_b.tenant_id = Some("tenant_b".to_string());
        let page_shared = test_page("p3", "https://example.com/c", 200);

        storage.insert_page(&crawl_id, &page_a).unwrap();
        storage.insert_page(&crawl_id, &page_b).unwrap();
        storage.insert_page(&crawl_id, &page_shared).unwrap();

        let pages_a = storage
            .get_pages_for_tenant(&crawl_id, "tenant_a", 10)
            .unwrap();
        assert_eq!(pages_a.len(), 2); // page_a + shared
        assert!(pages_a.iter().any(|p| p.id == "p1"));
        assert!(pages_a.iter().any(|p| p.id == "p3"));

        let pages_b = storage
            .get_pages_for_tenant(&crawl_id, "tenant_b", 10)
            .unwrap();
        assert_eq!(pages_b.len(), 2); // page_b + shared
        assert!(pages_b.iter().any(|p| p.id == "p2"));
        assert!(pages_b.iter().any(|p| p.id == "p3"));
    }

    #[test]
    fn test_trait_insert_and_get_issues() {
        let storage: Box<dyn StorageBackend> = new_in_memory_backend().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let page = test_page("p1", "https://example.com/", 200);
        storage.insert_page(&crawl_id, &page).unwrap();

        let issues = vec![
            test_issue("i1", "p1", IssueCategory::Seo, Severity::Error),
            test_issue("i2", "p1", IssueCategory::Images, Severity::Warning),
        ];
        storage.insert_issues_batch(&issues).unwrap();

        let retrieved = storage
            .get_issues(&crawl_id, &IssueFilter::default())
            .unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn test_trait_stats() {
        let storage: Box<dyn StorageBackend> = new_in_memory_backend().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let page = test_page("p1", "https://example.com/", 200);
        storage.insert_page(&crawl_id, &page).unwrap();
        let issue = test_issue("i1", "p1", IssueCategory::Seo, Severity::Error);
        storage.insert_issue(&issue).unwrap();

        let stats = storage.get_stats(&crawl_id).unwrap();
        assert_eq!(stats.total_pages, 1);
        assert_eq!(stats.total_issues, 1);
    }

    #[test]
    fn test_trait_conditional_requests() {
        let storage: Box<dyn StorageBackend> = new_in_memory_backend().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let mut page = test_page("p1", "https://example.com/", 200);
        page.etag = Some("\"abc123\"".to_string());
        page.last_modified = Some("Wed, 21 Oct 2024 07:28:00 GMT".to_string());
        storage.insert_page(&crawl_id, &page).unwrap();

        let result = storage
            .get_page_conditional(&crawl_id, "https://example.com/")
            .unwrap();
        assert!(result.is_some());
        let (page_id, etag, last_modified) = result.unwrap();
        assert_eq!(page_id, "p1");
        assert_eq!(etag.as_deref(), Some("\"abc123\""));
        assert_eq!(
            last_modified.as_deref(),
            Some("Wed, 21 Oct 2024 07:28:00 GMT")
        );

        let result = storage
            .get_latest_conditional("https://example.com/")
            .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_trait_sqlite_implementation_matches_direct() {
        let direct = Storage::new_in_memory().unwrap();
        let trait_based: Box<dyn StorageBackend> = new_in_memory_backend().unwrap();

        let crawl_id1 = direct.start_crawl("https://example.com", None).unwrap();
        let crawl_id2 = trait_based
            .start_crawl("https://example.com", None)
            .unwrap();

        let page = test_page("p1", "https://example.com/", 200);
        direct.insert_page(&crawl_id1, &page).unwrap();
        trait_based.insert_page(&crawl_id2, &page).unwrap();

        let stats1 = direct.get_stats(&crawl_id1).unwrap();
        let stats2 = trait_based.get_stats(&crawl_id2).unwrap();

        assert_eq!(stats1.total_pages, stats2.total_pages);
        assert_eq!(stats1.total_issues, stats2.total_issues);
    }
}
