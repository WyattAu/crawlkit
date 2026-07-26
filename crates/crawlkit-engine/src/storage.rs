use chrono::{DateTime, Utc};
use lru::LruCache;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;
use url::Url;

use crate::CrawlError;

/// Errors specific to storage operations.
///
/// Wraps SQLite database errors and URL parsing errors that can occur
/// during crawl data persistence.
#[derive(Debug, Error)]
pub enum StorageError {
    /// SQLite database error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// URL parsing error during retrieval.
    #[error("invalid URL in database: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

impl From<StorageError> for CrawlError {
    fn from(e: StorageError) -> Self {
        CrawlError::Storage(e.to_string())
    }
}

/// Severity level for an issue/finding.
///
/// Used by analyzers to classify the importance of detected issues.
/// Stored in the database as a lowercase string for querying.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    /// Critical issue requiring immediate attention.
    Critical,
    /// Error that should be fixed.
    Error,
    /// Warning suggesting improvement.
    Warning,
    /// Informational note.
    Info,
}

impl Severity {
    /// Convert to the string representation used in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// Parse from the database string representation.
    pub fn parse_severity(s: &str) -> Option<Self> {
        match s {
            "critical" => Some(Severity::Critical),
            "error" => Some(Severity::Error),
            "warning" => Some(Severity::Warning),
            "info" => Some(Severity::Info),
            _ => None,
        }
    }
}

/// Category of an analysis finding.
///
/// Groups related issues for filtering and reporting. Stored in the
/// database as a lowercase string. Custom categories use a `custom:` prefix.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum IssueCategory {
    /// HTTP-related issues (status codes, redirects, headers).
    Http,
    /// SEO issues (title, meta, canonical, robots).
    Seo,
    /// Content issues (word count, thin content, readability).
    Content,
    /// Link issues (broken links, redirect links, nofollow).
    Links,
    /// Image issues (missing alt, oversized, format).
    Images,
    /// Structured data issues (JSON-LD, microdata).
    Schema,
    /// Security issues (mixed content, headers).
    Security,
    /// Performance issues (page size, load time).
    Performance,
    /// Mobile-friendliness issues.
    Mobile,
    /// Accessibility issues (alt text, ARIA, contrast).
    Accessibility,
    /// Social metadata issues (Open Graph, Twitter Cards).
    Social,
    /// Custom analyzer issue.
    Custom(String),
}

impl IssueCategory {
    /// Convert to the string representation used in the database.
    pub fn as_str(&self) -> String {
        match self {
            IssueCategory::Http => "http".to_string(),
            IssueCategory::Seo => "seo".to_string(),
            IssueCategory::Content => "content".to_string(),
            IssueCategory::Links => "links".to_string(),
            IssueCategory::Images => "images".to_string(),
            IssueCategory::Schema => "schema".to_string(),
            IssueCategory::Security => "security".to_string(),
            IssueCategory::Performance => "performance".to_string(),
            IssueCategory::Mobile => "mobile".to_string(),
            IssueCategory::Accessibility => "accessibility".to_string(),
            IssueCategory::Social => "social".to_string(),
            IssueCategory::Custom(name) => format!("custom:{name}"),
        }
    }

    /// Parse from the database string representation.
    pub fn parse_category(s: &str) -> Self {
        match s {
            "http" => IssueCategory::Http,
            "seo" => IssueCategory::Seo,
            "content" => IssueCategory::Content,
            "links" => IssueCategory::Links,
            "images" => IssueCategory::Images,
            "schema" => IssueCategory::Schema,
            "security" => IssueCategory::Security,
            "performance" => IssueCategory::Performance,
            "mobile" => IssueCategory::Mobile,
            "accessibility" => IssueCategory::Accessibility,
            "social" => IssueCategory::Social,
            other => {
                // Strip "custom:" prefix if present
                let name = other.strip_prefix("custom:").unwrap_or(other);
                IssueCategory::Custom(name.to_string())
            }
        }
    }
}

/// A page extracted from the crawl, with full data for storage.
///
/// Contains all metadata needed to persist a crawled page to the database,
/// including URL, status, content metrics, and discovered links.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageData {
    /// Unique page identifier.
    pub id: String,
    /// The URL of the page.
    pub url: Url,
    /// The final URL after redirects.
    pub final_url: Url,
    /// The HTTP status code.
    pub status_code: u16,
    /// The page title.
    pub title: Option<String>,
    /// The meta description.
    pub description: Option<String>,
    /// The canonical URL.
    pub canonical_url: Option<Url>,
    /// Word count of the page body.
    pub word_count: Option<usize>,
    /// Response time in milliseconds.
    pub load_time_ms: Option<u64>,
    /// Body size in bytes.
    pub body_size: Option<usize>,
    /// When this page was fetched.
    pub fetched_at: DateTime<Utc>,
    /// Links discovered on this page.
    pub links: Vec<Url>,
    /// Tenant ID for multi-tenancy.
    pub tenant_id: Option<String>,
    /// ETag header value from the last fetch.
    pub etag: Option<String>,
    /// Last-Modified header value from the last fetch.
    pub last_modified: Option<String>,
}

/// An issue/finding detected during analysis.
///
/// Persisted to the database with a reference to the page it belongs to.
/// Includes category, severity, machine-readable code, and recommendation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Issue {
    /// Unique issue identifier.
    pub id: String,
    /// The page this issue belongs to.
    pub page_id: String,
    /// Issue category.
    pub category: IssueCategory,
    /// Issue severity.
    pub severity: Severity,
    /// Machine-readable issue code (e.g. "SEO001").
    pub code: String,
    /// Human-readable title.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// CSS selector of the affected element, if applicable.
    pub element: Option<String>,
    /// Recommendation for fixing the issue.
    pub recommendation: String,
    /// Tenant ID for multi-tenancy.
    pub tenant_id: Option<String>,
}

/// Filters for querying issues.
///
/// All fields are optional. When set, they narrow the query results.
/// When `None`, that filter dimension is not applied.
#[derive(Debug, Clone, Default)]
pub struct IssueFilter {
    /// Filter by severity.
    pub severity: Option<Severity>,
    /// Filter by category.
    pub category: Option<IssueCategory>,
    /// Filter by page ID.
    pub page_id: Option<String>,
    /// Filter by issue code prefix.
    pub code_prefix: Option<String>,
}

/// Aggregate crawl statistics.
///
/// Summary of pages crawled, issues found, and performance metrics.
/// Returned by [`Storage::get_stats`].
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CrawlStats {
    /// Total number of pages crawled.
    pub total_pages: usize,
    /// Total number of issues found.
    pub total_issues: usize,
    /// Issues by severity.
    pub issues_by_severity: std::collections::HashMap<String, usize>,
    /// Issues by category.
    pub issues_by_category: std::collections::HashMap<String, usize>,
    /// Average response time in milliseconds.
    pub avg_response_time_ms: Option<f64>,
    /// Total body size in bytes.
    pub total_body_size: Option<usize>,
}

/// Cache statistics for the LRU page cache.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Maximum cache capacity.
    pub capacity: usize,
    /// Current number of cached entries.
    pub size: usize,
    /// Cache hit count (since last reset).
    pub hits: usize,
    /// Cache miss count (since last reset).
    pub misses: usize,
}

/// SQLite-backed storage for crawl data.
///
/// Uses WAL mode for concurrent-safe access and provides
/// batch-friendly insert methods for performance. Includes an LRU cache
/// for frequently accessed pages and memory usage tracking.
pub struct Storage {
    conn: Mutex<Connection>,
    /// LRU cache for recently accessed pages.
    page_cache: Mutex<LruCache<String, PageData>>,
    /// Approximate memory usage in bytes.
    memory_usage: AtomicUsize,
    /// Whether memory-mapped I/O is enabled (for read-heavy workloads).
    mmap_enabled: bool,
}

impl Storage {
    /// Lock and return the underlying connection guard.
    ///
    /// Provides direct access to the SQLite connection for advanced
    /// queries or transactions not covered by the convenience methods.
    pub fn conn(&self) -> parking_lot::MutexGuard<'_, Connection> {
        self.conn.lock()
    }

    /// Open or create a SQLite database at the given path.
    ///
    /// Enables WAL mode, memory-mapped I/O, and creates the schema if it
    /// doesn't exist. Uses a default LRU cache of 1000 entries.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if the database cannot be opened
    /// or the schema cannot be created.
    pub fn new(path: &Path) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for concurrent-safe reads/writes
        // Enable memory-mapped I/O for faster reads (256MB)
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA mmap_size=268435456;
             PRAGMA cache_size=-64000;
             PRAGMA synchronous=NORMAL;",
        )?;

        let storage = Self {
            conn: Mutex::new(conn),
            page_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(1000).unwrap_or(NonZeroUsize::MIN),
            )),
            memory_usage: AtomicUsize::new(0),
            mmap_enabled: true,
        };
        storage.create_schema()?;
        Ok(storage)
    }

    /// Create in-memory storage for testing.
    ///
    /// Uses an in-memory SQLite database with WAL mode enabled.
    /// Ideal for unit tests that need fast, isolated storage.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if the in-memory database
    /// cannot be created.
    pub fn new_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA cache_size=-64000;",
        )?;
        let storage = Self {
            conn: Mutex::new(conn),
            page_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(1000).unwrap_or(NonZeroUsize::MIN),
            )),
            memory_usage: AtomicUsize::new(0),
            mmap_enabled: false,
        };
        storage.create_schema()?;
        Ok(storage)
    }

    /// Create storage with a custom LRU cache size.
    ///
    /// Use this when the default 1000-entry cache is not appropriate
    /// for your workload. Larger caches improve read performance for
    /// repeat queries but use more memory.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if the database cannot be opened.
    pub fn with_cache_size(path: &Path, cache_size: usize) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA mmap_size=268435456;
             PRAGMA cache_size=-64000;
             PRAGMA synchronous=NORMAL;",
        )?;

        let storage = Self {
            conn: Mutex::new(conn),
            page_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::MIN),
            )),
            memory_usage: AtomicUsize::new(0),
            mmap_enabled: true,
        };
        storage.create_schema()?;
        Ok(storage)
    }

    /// Returns the approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        self.memory_usage.load(Ordering::Relaxed)
    }

    /// Returns whether memory-mapped I/O is enabled.
    pub fn is_mmap_enabled(&self) -> bool {
        self.mmap_enabled
    }

    /// Returns cache statistics (hits, misses, current size).
    pub fn cache_stats(&self) -> CacheStats {
        let cache = self.page_cache.lock();
        CacheStats {
            capacity: cache.cap().get(),
            size: cache.len(),
            hits: 0, // Would need to track these separately
            misses: 0,
        }
    }

    /// Clears the page cache.
    pub fn clear_cache(&self) {
        let mut cache = self.page_cache.lock();
        let evicted = cache.len();
        cache.clear();
        if evicted > 0 {
            self.memory_usage
                .fetch_sub(evicted * std::mem::size_of::<PageData>(), Ordering::Relaxed);
        }
    }

    /// Create the database schema if it doesn't already exist.
    fn create_schema(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS crawls (
                id            TEXT PRIMARY KEY,
                start_time    DATETIME NOT NULL,
                end_time      DATETIME,
                target_url    TEXT NOT NULL,
                pages_crawled INTEGER DEFAULT 0,
                total_issues  INTEGER DEFAULT 0,
                config_json   TEXT
            );

            CREATE TABLE IF NOT EXISTS pages (
                id            TEXT PRIMARY KEY,
                crawl_id      TEXT NOT NULL REFERENCES crawls(id),
                url           TEXT NOT NULL,
                final_url     TEXT NOT NULL,
                status_code   INTEGER NOT NULL,
                title         TEXT,
                description   TEXT,
                canonical     TEXT,
                word_count    INTEGER,
                load_time_ms  INTEGER,
                body_size     INTEGER,
                fetched_at    DATETIME NOT NULL,
                tenant_id     TEXT,
                etag          TEXT,
                last_modified TEXT,
                UNIQUE(crawl_id, url)
            );

            CREATE TABLE IF NOT EXISTS links (
                id            TEXT PRIMARY KEY,
                page_id       TEXT NOT NULL REFERENCES pages(id),
                source_url    TEXT NOT NULL,
                target_url    TEXT NOT NULL,
                anchor_text   TEXT,
                rel           TEXT,
                is_external   BOOLEAN,
                is_nofollow   BOOLEAN
            );

            CREATE TABLE IF NOT EXISTS findings (
                id            TEXT PRIMARY KEY,
                page_id       TEXT NOT NULL REFERENCES pages(id),
                category      TEXT NOT NULL,
                severity      TEXT NOT NULL,
                code          TEXT NOT NULL,
                title         TEXT NOT NULL,
                description   TEXT NOT NULL,
                element       TEXT,
                recommendation TEXT,
                tenant_id     TEXT
            );

            CREATE TABLE IF NOT EXISTS images (
                id            TEXT PRIMARY KEY,
                page_id       TEXT NOT NULL REFERENCES pages(id),
                url           TEXT NOT NULL,
                alt           TEXT,
                width         INTEGER,
                height        INTEGER,
                format        TEXT,
                file_size     INTEGER,
                is_lazy_loaded BOOLEAN
            );

            CREATE TABLE IF NOT EXISTS schemas (
                id            TEXT PRIMARY KEY,
                page_id       TEXT NOT NULL REFERENCES pages(id),
                schema_type   TEXT NOT NULL,
                data_json     TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS crux_metrics (
                id            TEXT PRIMARY KEY,
                page_id       TEXT NOT NULL REFERENCES pages(id),
                url           TEXT NOT NULL,
                lcp_p75       REAL,
                inp_p75       REAL,
                cls_p75       REAL,
                fcp_p75       REAL,
                ttfb_p75      REAL,
                fetched_at    DATETIME NOT NULL,
                UNIQUE(page_id)
            );

            CREATE INDEX IF NOT EXISTS idx_pages_crawl ON pages(crawl_id);
            CREATE INDEX IF NOT EXISTS idx_pages_tenant ON pages(tenant_id);
            CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_url);
            CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_url);
            CREATE INDEX IF NOT EXISTS idx_findings_page ON findings(page_id);
            CREATE INDEX IF NOT EXISTS idx_findings_category ON findings(category);
            CREATE INDEX IF NOT EXISTS idx_findings_severity ON findings(severity);
            CREATE INDEX IF NOT EXISTS idx_findings_tenant ON findings(tenant_id);
            ",
        )?;
        Ok(())
    }

    /// Start a new crawl and return its ID.
    pub fn start_crawl(
        &self,
        target_url: &str,
        config_json: Option<&str>,
    ) -> Result<String, StorageError> {
        let crawl_id = uuid::Uuid::new_v4().to_string();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO crawls (id, start_time, target_url, config_json) VALUES (?1, ?2, ?3, ?4)",
            params![crawl_id, Utc::now().to_rfc3339(), target_url, config_json,],
        )?;
        Ok(crawl_id)
    }

    /// Finish a crawl, recording final statistics.
    pub fn finish_crawl(
        &self,
        crawl_id: &str,
        pages_crawled: usize,
        total_issues: usize,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE crawls SET end_time = ?1, pages_crawled = ?2, total_issues = ?3 WHERE id = ?4",
            params![
                Utc::now().to_rfc3339(),
                pages_crawled,
                total_issues,
                crawl_id
            ],
        )?;
        Ok(())
    }

    /// Insert a single page into the database under the given crawl.
    /// Uses a single SQLite transaction for the page row + all link rows
    /// to avoid per-statement fsync overhead.
    pub fn insert_page(&self, crawl_id: &str, page: &PageData) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let tx = conn.unchecked_transaction()?;

        tx.execute(
            "INSERT OR REPLACE INTO pages (id, crawl_id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at, tenant_id, etag, last_modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                page.id,
                crawl_id,
                page.url.as_str(),
                page.final_url.as_str(),
                page.status_code,
                page.title,
                page.description,
                page.canonical_url.as_ref().map(|u| u.as_str()),
                page.word_count.map(|v| v as i64),
                page.load_time_ms.map(|v| v as i64),
                page.body_size.map(|v| v as i64),
                page.fetched_at.to_rfc3339(),
                page.tenant_id,
                page.etag,
                page.last_modified,
            ],
        )?;

        // Insert links within the same transaction
        let mut stmt = tx.prepare(
            "INSERT INTO links (id, page_id, source_url, target_url, is_external) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for link in &page.links {
            let link_id = uuid::Uuid::new_v4().to_string();
            let is_external = link.domain() != page.url.domain();
            stmt.execute(params![
                link_id,
                page.id,
                page.url.as_str(),
                link.as_str(),
                is_external,
            ])?;
        }
        drop(stmt);

        tx.commit()?;
        Ok(())
    }

    /// Insert a batch of pages for performance.
    /// Wraps all inserts in a single SQLite transaction for O(n) vs O(n*fsync).
    pub fn insert_pages(&self, crawl_id: &str, pages: &[PageData]) -> Result<(), StorageError> {
        if pages.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock();
        let tx = conn.unchecked_transaction()?;

        let mut page_stmt = tx.prepare(
            "INSERT OR REPLACE INTO pages (id, crawl_id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at, tenant_id, etag, last_modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        )?;
        let mut link_stmt = tx.prepare(
            "INSERT INTO links (id, page_id, source_url, target_url, is_external) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;

        for page in pages {
            page_stmt.execute(params![
                page.id,
                crawl_id,
                page.url.as_str(),
                page.final_url.as_str(),
                page.status_code,
                page.title,
                page.description,
                page.canonical_url.as_ref().map(|u| u.as_str()),
                page.word_count.map(|v| v as i64),
                page.load_time_ms.map(|v| v as i64),
                page.body_size.map(|v| v as i64),
                page.fetched_at.to_rfc3339(),
                page.tenant_id,
                page.etag,
                page.last_modified,
            ])?;

            for link in &page.links {
                let link_id = uuid::Uuid::new_v4().to_string();
                let is_external = link.domain() != page.url.domain();
                link_stmt.execute(params![
                    link_id,
                    page.id,
                    page.url.as_str(),
                    link.as_str(),
                    is_external,
                ])?;
            }
        }

        drop(link_stmt);
        drop(page_stmt);
        tx.commit()?;
        Ok(())
    }

    /// Insert a single issue/finding into the database.
    pub fn insert_issue(&self, issue: &Issue) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO findings (id, page_id, category, severity, code, title, description, element, recommendation, tenant_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                issue.id,
                issue.page_id,
                issue.category.as_str(),
                issue.severity.as_str(),
                issue.code,
                issue.title,
                issue.description,
                issue.element,
                issue.recommendation,
                issue.tenant_id,
            ],
        )?;
        Ok(())
    }

    /// Insert a batch of issues for performance.
    pub fn insert_issues(&self, issues: &[Issue]) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "INSERT INTO findings (id, page_id, category, severity, code, title, description, element, recommendation, tenant_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;
        for issue in issues {
            stmt.execute(params![
                issue.id,
                issue.page_id,
                issue.category.as_str(),
                issue.severity.as_str(),
                issue.code,
                issue.title,
                issue.description,
                issue.element,
                issue.recommendation,
                issue.tenant_id,
            ])?;
        }
        Ok(())
    }

    /// Retrieve pages with a limit.
    ///
    /// Results are not cached because the cache type (`LruCache<String, PageData>`)
    /// cannot store `Vec<PageData>`. The query is fast with proper indexing.
    pub fn get_pages(&self, crawl_id: &str, limit: usize) -> Result<Vec<PageData>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at, tenant_id, etag, last_modified
             FROM pages WHERE crawl_id = ?1 ORDER BY fetched_at ASC LIMIT ?2",
        )?;

        let pages = stmt
            .query_map(params![crawl_id, limit as i64], |row| {
                let url_str: String = row.get(1)?;
                let final_url_str: String = row.get(2)?;
                let canonical_str: Option<String> = row.get(6)?;
                let fetched_at_str: String = row.get(10)?;

                Ok(PageData {
                    id: row.get(0)?,
                    url: Url::parse(&url_str).unwrap_or_else(|_| {
                        Url::parse("about:invalid")
                            .unwrap_or_else(|_| unreachable!("about:invalid is always a valid URL"))
                    }),
                    final_url: Url::parse(&final_url_str).unwrap_or_else(|_| {
                        Url::parse("about:invalid")
                            .unwrap_or_else(|_| unreachable!("about:invalid is always a valid URL"))
                    }),
                    status_code: row.get(3)?,
                    title: row.get(4)?,
                    description: row.get(5)?,
                    canonical_url: canonical_str.and_then(|s| Url::parse(&s).ok()),
                    word_count: row.get::<_, Option<i64>>(7)?.map(|v| v as usize),
                    load_time_ms: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                    body_size: row.get::<_, Option<i64>>(9)?.map(|v| v as usize),
                    fetched_at: DateTime::parse_from_rfc3339(&fetched_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    links: Vec::new(),
                    tenant_id: row.get(11)?,
                    etag: row.get(12)?,
                    last_modified: row.get(13)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(pages)
    }

    /// Retrieve issues/finding with optional filters.
    pub fn get_issues(
        &self,
        crawl_id: &str,
        filters: &IssueFilter,
    ) -> Result<Vec<Issue>, StorageError> {
        let conn = self.conn.lock();

        let mut query = String::from(
            "SELECT f.id, f.page_id, f.category, f.severity, f.code, f.title, f.description, f.element, f.recommendation, f.tenant_id
             FROM findings f
             JOIN pages p ON f.page_id = p.id
             WHERE p.crawl_id = ?1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(crawl_id.to_string()));

        if let Some(ref severity) = filters.severity {
            query.push_str(&format!(" AND f.severity = ?{}", param_values.len() + 1));
            param_values.push(Box::new(severity.as_str().to_string()));
        }
        if let Some(ref category) = filters.category {
            query.push_str(&format!(" AND f.category = ?{}", param_values.len() + 1));
            param_values.push(Box::new(category.as_str()));
        }
        if let Some(ref page_id) = filters.page_id {
            query.push_str(&format!(" AND f.page_id = ?{}", param_values.len() + 1));
            param_values.push(Box::new(page_id.clone()));
        }
        if let Some(ref code_prefix) = filters.code_prefix {
            query.push_str(&format!(" AND f.code LIKE ?{}", param_values.len() + 1));
            param_values.push(Box::new(format!("{code_prefix}%")));
        }

        query.push_str(" ORDER BY f.id ASC");

        let mut stmt = conn.prepare(&query)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let issues = stmt
            .query_map(params_refs.as_slice(), |row| {
                let category_str: String = row.get(2)?;
                let severity_str: String = row.get(3)?;

                Ok(Issue {
                    id: row.get(0)?,
                    page_id: row.get(1)?,
                    category: IssueCategory::parse_category(&category_str),
                    severity: Severity::parse_severity(&severity_str).unwrap_or(Severity::Info),
                    code: row.get(4)?,
                    title: row.get(5)?,
                    description: row.get(6)?,
                    element: row.get(7)?,
                    recommendation: row.get(8)?,
                    tenant_id: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(issues)
    }

    /// Get pages for a specific tenant.
    ///
    /// Returns pages belonging to the given tenant, or pages with no tenant
    /// assigned (shared/global data). This ensures tenant isolation at the
    /// storage layer while still allowing access to unscoped data.
    pub fn get_pages_for_tenant(
        &self,
        crawl_id: &str,
        tenant_id: &str,
        limit: usize,
    ) -> Result<Vec<PageData>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at, tenant_id, etag, last_modified
             FROM pages WHERE crawl_id = ?1 AND (tenant_id = ?2 OR tenant_id IS NULL)
             ORDER BY fetched_at ASC LIMIT ?3",
        )?;

        let pages = stmt
            .query_map(params![crawl_id, tenant_id, limit as i64], |row| {
                let url_str: String = row.get(1)?;
                let final_url_str: String = row.get(2)?;
                let canonical_str: Option<String> = row.get(6)?;
                let fetched_at_str: String = row.get(10)?;

                Ok(PageData {
                    id: row.get(0)?,
                    url: Url::parse(&url_str).unwrap_or_else(|_| {
                        Url::parse("about:invalid")
                            .unwrap_or_else(|_| unreachable!("about:invalid is always a valid URL"))
                    }),
                    final_url: Url::parse(&final_url_str).unwrap_or_else(|_| {
                        Url::parse("about:invalid")
                            .unwrap_or_else(|_| unreachable!("about:invalid is always a valid URL"))
                    }),
                    status_code: row.get(3)?,
                    title: row.get(4)?,
                    description: row.get(5)?,
                    canonical_url: canonical_str.and_then(|s| Url::parse(&s).ok()),
                    word_count: row.get::<_, Option<i64>>(7)?.map(|v| v as usize),
                    load_time_ms: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                    body_size: row.get::<_, Option<i64>>(9)?.map(|v| v as usize),
                    fetched_at: DateTime::parse_from_rfc3339(&fetched_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    links: Vec::new(),
                    tenant_id: row.get(11)?,
                    etag: row.get(12)?,
                    last_modified: row.get(13)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(pages)
    }

    /// Get issues for a specific tenant.
    ///
    /// Returns issues belonging to the given tenant, or issues with no tenant
    /// assigned (shared/global data). This ensures tenant isolation at the
    /// storage layer while still allowing access to unscoped data.
    pub fn get_issues_for_tenant(
        &self,
        crawl_id: &str,
        tenant_id: &str,
        filters: &IssueFilter,
    ) -> Result<Vec<Issue>, StorageError> {
        let conn = self.conn.lock();

        let mut query = String::from(
            "SELECT f.id, f.page_id, f.category, f.severity, f.code, f.title, f.description, f.element, f.recommendation, f.tenant_id
             FROM findings f
             JOIN pages p ON f.page_id = p.id
             WHERE p.crawl_id = ?1 AND (f.tenant_id = ?2 OR f.tenant_id IS NULL)",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(crawl_id.to_string()));
        param_values.push(Box::new(tenant_id.to_string()));

        if let Some(ref severity) = filters.severity {
            query.push_str(&format!(" AND f.severity = ?{}", param_values.len() + 1));
            param_values.push(Box::new(severity.as_str().to_string()));
        }
        if let Some(ref category) = filters.category {
            query.push_str(&format!(" AND f.category = ?{}", param_values.len() + 1));
            param_values.push(Box::new(category.as_str()));
        }
        if let Some(ref page_id) = filters.page_id {
            query.push_str(&format!(" AND f.page_id = ?{}", param_values.len() + 1));
            param_values.push(Box::new(page_id.clone()));
        }
        if let Some(ref code_prefix) = filters.code_prefix {
            query.push_str(&format!(" AND f.code LIKE ?{}", param_values.len() + 1));
            param_values.push(Box::new(format!("{code_prefix}%")));
        }

        query.push_str(" ORDER BY f.id ASC");

        let mut stmt = conn.prepare(&query)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let issues = stmt
            .query_map(params_refs.as_slice(), |row| {
                let category_str: String = row.get(2)?;
                let severity_str: String = row.get(3)?;

                Ok(Issue {
                    id: row.get(0)?,
                    page_id: row.get(1)?,
                    category: IssueCategory::parse_category(&category_str),
                    severity: Severity::parse_severity(&severity_str).unwrap_or(Severity::Info),
                    code: row.get(4)?,
                    title: row.get(5)?,
                    description: row.get(6)?,
                    element: row.get(7)?,
                    recommendation: row.get(8)?,
                    tenant_id: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(issues)
    }

    /// Get aggregate statistics for a crawl.
    pub fn get_stats(&self, crawl_id: &str) -> Result<CrawlStats, StorageError> {
        let conn = self.conn.lock();

        let total_pages: usize = conn.query_row(
            "SELECT COALESCE(COUNT(*), 0) FROM pages WHERE crawl_id = ?1",
            params![crawl_id],
            |row| row.get::<_, i64>(0),
        )? as usize;

        let total_issues: usize = conn
            .query_row(
                "SELECT COALESCE(COUNT(*), 0) FROM findings f JOIN pages p ON f.page_id = p.id WHERE p.crawl_id = ?1",
                params![crawl_id],
                |row| row.get::<_, i64>(0),
            )?
            as usize;

        let mut issues_by_severity = std::collections::HashMap::new();
        {
            let mut stmt = conn.prepare(
                "SELECT f.severity, COUNT(*) FROM findings f JOIN pages p ON f.page_id = p.id WHERE p.crawl_id = ?1 GROUP BY f.severity",
            )?;
            let rows = stmt.query_map(params![crawl_id], |row| {
                let sev: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((sev, count as usize))
            })?;
            for row in rows {
                let (sev, count) = row?;
                issues_by_severity.insert(sev, count);
            }
        }

        let mut issues_by_category = std::collections::HashMap::new();
        {
            let mut stmt = conn.prepare(
                "SELECT f.category, COUNT(*) FROM findings f JOIN pages p ON f.page_id = p.id WHERE p.crawl_id = ?1 GROUP BY f.category",
            )?;
            let rows = stmt.query_map(params![crawl_id], |row| {
                let cat: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((cat, count as usize))
            })?;
            for row in rows {
                let (cat, count) = row?;
                issues_by_category.insert(cat, count);
            }
        }

        let avg_response_time_ms: Option<f64> = conn
            .query_row(
                "SELECT AVG(load_time_ms) FROM pages WHERE crawl_id = ?1 AND load_time_ms IS NOT NULL",
                params![crawl_id],
                |row| row.get::<_, Option<f64>>(0),
            )
            .ok()
            .flatten();

        let total_body_size: Option<usize> = conn
            .query_row(
                "SELECT SUM(body_size) FROM pages WHERE crawl_id = ?1 AND body_size IS NOT NULL",
                params![crawl_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .ok()
            .flatten()
            .map(|v| v as usize);

        Ok(CrawlStats {
            total_pages,
            total_issues,
            issues_by_severity,
            issues_by_category,
            avg_response_time_ms,
            total_body_size,
        })
    }

    /// Get the latest crawl ID.
    pub fn get_latest_crawl_id(&self) -> Result<Option<String>, StorageError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT id FROM crawls ORDER BY start_time DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    /// Get all links for a crawl, grouped by source URL.
    ///
    /// Returns `Vec<(source_url, Vec<target_url>)>` suitable for
    /// feeding into `BacklinkAnalyzer::load_from_crawl_data`.
    pub fn get_links_for_crawl(
        &self,
        crawl_id: &str,
    ) -> Result<Vec<(String, Vec<String>)>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT l.source_url, l.target_url
             FROM links l
             JOIN pages p ON l.page_id = p.id
             WHERE p.crawl_id = ?1
             ORDER BY l.source_url",
        )?;

        let rows = stmt.query_map(params![crawl_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut links: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for row in rows {
            let (source, target) = row?;
            links.entry(source).or_default().push(target);
        }

        Ok(links.into_iter().collect())
    }

    /// Get all external links for a crawl.
    pub fn get_external_links(
        &self,
        crawl_id: &str,
    ) -> Result<Vec<(String, String)>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT l.source_url, l.target_url
             FROM links l
             JOIN pages p ON l.page_id = p.id
             WHERE p.crawl_id = ?1 AND l.is_external = 1",
        )?;

        let rows = stmt.query_map(params![crawl_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut links = Vec::new();
        for row in rows {
            links.push(row?);
        }

        Ok(links)
    }

    /// Get all page URLs for a crawl.
    pub fn get_page_urls(&self, crawl_id: &str) -> Result<Vec<String>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT url FROM pages WHERE crawl_id = ?1 ORDER BY url")?;

        let rows = stmt.query_map(params![crawl_id], |row| row.get::<_, String>(0))?;

        let mut urls = Vec::new();
        for row in rows {
            urls.push(row?);
        }

        Ok(urls)
    }

    /// Get the conditional request data (page_id, etag, last_modified) for a URL
    /// within a crawl. Returns `None` if the URL was not previously crawled.
    pub fn get_page_conditional(
        &self,
        crawl_id: &str,
        url: &str,
    ) -> Result<Option<(String, Option<String>, Option<String>)>, StorageError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT id, etag, last_modified FROM pages WHERE crawl_id = ?1 AND url = ?2",
            params![crawl_id, url],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        );

        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    /// Store CrUX metrics for a page.
    pub fn insert_crux_metrics(
        &self,
        page_id: &str,
        url: &str,
        lcp_p75: Option<f64>,
        inp_p75: Option<f64>,
        cls_p75: Option<f64>,
        fcp_p75: Option<f64>,
        ttfb_p75: Option<f64>,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO crux_metrics (id, page_id, url, lcp_p75, inp_p75, cls_p75, fcp_p75, ttfb_p75, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                uuid::Uuid::new_v4().to_string(),
                page_id,
                url,
                lcp_p75,
                inp_p75,
                cls_p75,
                fcp_p75,
                ttfb_p75,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get CrUX metrics for a page.
    pub fn get_crux_metrics(&self, page_id: &str) -> Result<Option<CruxMetrics>, StorageError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT url, lcp_p75, inp_p75, cls_p75, fcp_p75, ttfb_p75 FROM crux_metrics WHERE page_id = ?1",
            params![page_id],
            |row| {
                Ok(CruxMetrics {
                    url: row.get(0)?,
                    lcp_p75: row.get(1)?,
                    inp_p75: row.get(2)?,
                    cls_p75: row.get(3)?,
                    fcp_p75: row.get(4)?,
                    ttfb_p75: row.get(5)?,
                })
            },
        );

        match result {
            Ok(m) => Ok(Some(m)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    /// Get CrUX metrics for all pages in a crawl.
    pub fn get_crux_metrics_for_crawl(
        &self,
        crawl_id: &str,
    ) -> Result<Vec<CruxMetrics>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT cm.url, cm.lcp_p75, cm.inp_p75, cm.cls_p75, cm.fcp_p75, cm.ttfb_p75
             FROM crux_metrics cm
             JOIN pages p ON cm.page_id = p.id
             WHERE p.crawl_id = ?1",
        )?;

        let rows = stmt.query_map(params![crawl_id], |row| {
            Ok(CruxMetrics {
                url: row.get(0)?,
                lcp_p75: row.get(1)?,
                inp_p75: row.get(2)?,
                cls_p75: row.get(3)?,
                fcp_p75: row.get(4)?,
                ttfb_p75: row.get(5)?,
            })
        })?;

        let mut metrics = Vec::new();
        for row in rows {
            metrics.push(row?);
        }

        Ok(metrics)
    }
}

/// CrUX metrics for a single page.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CruxMetrics {
    pub url: String,
    pub lcp_p75: Option<f64>,
    pub inp_p75: Option<f64>,
    pub cls_p75: Option<f64>,
    pub fcp_p75: Option<f64>,
    pub ttfb_p75: Option<f64>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

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
    fn test_schema_creation() {
        let storage = Storage::new_in_memory().unwrap();
        // Schema should already exist; verify by inserting
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        assert!(!crawl_id.is_empty());
    }

    #[test]
    fn test_insert_and_get_pages() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let pages = vec![
            test_page("p1", "https://example.com/", 200),
            test_page("p2", "https://example.com/about", 200),
        ];
        storage.insert_pages(&crawl_id, &pages).unwrap();

        let retrieved = storage.get_pages(&crawl_id, 10).unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].id, "p1");
        assert_eq!(retrieved[1].id, "p2");
    }

    #[test]
    fn test_insert_and_get_issues() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let page = test_page("p1", "https://example.com/", 200);
        storage.insert_page(&crawl_id, &page).unwrap();

        let issues = vec![
            test_issue("i1", "p1", IssueCategory::Seo, Severity::Error),
            test_issue("i2", "p1", IssueCategory::Images, Severity::Warning),
        ];
        storage.insert_issues(&issues).unwrap();

        // Get all
        let retrieved = storage
            .get_issues(&crawl_id, &IssueFilter::default())
            .unwrap();
        assert_eq!(retrieved.len(), 2);

        // Filter by severity
        let filter = IssueFilter {
            severity: Some(Severity::Error),
            ..Default::default()
        };
        let retrieved = storage.get_issues(&crawl_id, &filter).unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].code, "i1001");

        // Filter by category
        let filter = IssueFilter {
            category: Some(IssueCategory::Images),
            ..Default::default()
        };
        let retrieved = storage.get_issues(&crawl_id, &filter).unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].code, "i2001");
    }

    #[test]
    fn test_get_stats() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let pages = vec![
            test_page("p1", "https://example.com/", 200),
            test_page("p2", "https://example.com/about", 200),
        ];
        storage.insert_pages(&crawl_id, &pages).unwrap();

        let issues = vec![
            test_issue("i1", "p1", IssueCategory::Seo, Severity::Error),
            test_issue("i2", "p1", IssueCategory::Seo, Severity::Warning),
            test_issue("i3", "p2", IssueCategory::Images, Severity::Warning),
        ];
        storage.insert_issues(&issues).unwrap();

        let stats = storage.get_stats(&crawl_id).unwrap();
        assert_eq!(stats.total_pages, 2);
        assert_eq!(stats.total_issues, 3);
        assert_eq!(stats.issues_by_severity.get("error"), Some(&1));
        assert_eq!(stats.issues_by_severity.get("warning"), Some(&2));
        assert_eq!(stats.issues_by_category.get("seo"), Some(&2));
        assert_eq!(stats.issues_by_category.get("images"), Some(&1));
    }

    #[test]
    fn test_finish_crawl() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let page = test_page("p1", "https://example.com/", 200);
        storage.insert_page(&crawl_id, &page).unwrap();
        let issue = test_issue("i1", "p1", IssueCategory::Seo, Severity::Error);
        storage.insert_issue(&issue).unwrap();

        storage.finish_crawl(&crawl_id, 1, 1).unwrap();
        // Verify no panic on double finish
        storage.finish_crawl(&crawl_id, 1, 1).unwrap();
    }

    #[test]
    fn test_severity_roundtrip() {
        for sev in [
            Severity::Critical,
            Severity::Error,
            Severity::Warning,
            Severity::Info,
        ] {
            let s = sev.as_str();
            assert_eq!(Severity::parse_severity(s), Some(sev));
        }
        assert_eq!(Severity::parse_severity("invalid"), None);
    }

    #[test]
    fn test_category_roundtrip() {
        for cat in [
            IssueCategory::Http,
            IssueCategory::Seo,
            IssueCategory::Content,
            IssueCategory::Links,
            IssueCategory::Images,
            IssueCategory::Schema,
            IssueCategory::Security,
            IssueCategory::Performance,
            IssueCategory::Mobile,
            IssueCategory::Accessibility,
            IssueCategory::Social,
        ] {
            let s = cat.as_str();
            assert_eq!(IssueCategory::parse_category(&s), cat);
        }
        let custom = IssueCategory::Custom("myplugin".to_string());
        let s = custom.as_str();
        assert_eq!(IssueCategory::parse_category(&s), custom);
    }

    #[test]
    fn test_pages_limit() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        for i in 0..10 {
            let page = test_page(
                &format!("p{i}"),
                &format!("https://example.com/page{i}"),
                200,
            );
            storage.insert_page(&crawl_id, &page).unwrap();
        }

        let pages = storage.get_pages(&crawl_id, 3).unwrap();
        assert_eq!(pages.len(), 3);
    }

    #[test]
    fn test_issue_filter_by_page_id() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let p1 = test_page("p1", "https://example.com/", 200);
        let p2 = test_page("p2", "https://example.com/about", 200);
        storage.insert_page(&crawl_id, &p1).unwrap();
        storage.insert_page(&crawl_id, &p2).unwrap();

        let issues = vec![
            test_issue("i1", "p1", IssueCategory::Seo, Severity::Error),
            test_issue("i2", "p2", IssueCategory::Seo, Severity::Error),
        ];
        storage.insert_issues(&issues).unwrap();

        let filter = IssueFilter {
            page_id: Some("p1".to_string()),
            ..Default::default()
        };
        let retrieved = storage.get_issues(&crawl_id, &filter).unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].page_id, "p1");
    }

    #[test]
    fn test_page_conditional_etag_and_last_modified() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let mut page = test_page("p1", "https://example.com/", 200);
        page.etag = Some("\"abc123\"".to_string());
        page.last_modified = Some("Wed, 21 Oct 2024 07:28:00 GMT".to_string());
        storage.insert_page(&crawl_id, &page).unwrap();

        // Retrieve conditional data
        let result = storage.get_page_conditional(&crawl_id, "https://example.com/").unwrap();
        assert!(result.is_some());
        let (page_id, etag, last_modified) = result.unwrap();
        assert_eq!(page_id, "p1");
        assert_eq!(etag.as_deref(), Some("\"abc123\""));
        assert_eq!(
            last_modified.as_deref(),
            Some("Wed, 21 Oct 2024 07:28:00 GMT")
        );

        // Non-existent URL returns None
        let result = storage
            .get_page_conditional(&crawl_id, "https://example.com/nonexistent")
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_page_etag_and_last_modified_roundtrip() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let mut page = test_page("p1", "https://example.com/", 200);
        page.etag = Some("\"strong-etag\"".to_string());
        page.last_modified = Some("Thu, 01 Jan 2025 00:00:00 GMT".to_string());
        storage.insert_page(&crawl_id, &page).unwrap();

        let pages = storage.get_pages(&crawl_id, 10).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].etag.as_deref(), Some("\"strong-etag\""));
        assert_eq!(
            pages[0].last_modified.as_deref(),
            Some("Thu, 01 Jan 2025 00:00:00 GMT")
        );
    }
}
