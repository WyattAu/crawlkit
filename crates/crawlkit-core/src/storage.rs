use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;
use url::Url;

use crate::CrawlError;

/// Errors specific to storage operations.
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
    pub fn from_str(s: &str) -> Option<Self> {
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
    pub fn from_str(s: &str) -> Self {
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
}

/// An issue/finding detected during analysis.
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
}

/// Filters for querying issues.
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

/// SQLite-backed storage for crawl data.
///
/// Uses WAL mode for concurrent-safe access and provides
/// batch-friendly insert methods for performance.
pub struct Storage {
    conn: Mutex<Connection>,
}

impl Storage {
    /// Open or create a SQLite database at the given path.
    ///
    /// Enables WAL mode and creates the schema if it doesn't exist.
    pub fn new(path: &Path) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for concurrent-safe reads/writes
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.create_schema()?;
        Ok(storage)
    }

    /// Create in-memory storage for testing.
    pub fn new_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.create_schema()?;
        Ok(storage)
    }

    /// Create the database schema if it doesn't already exist.
    fn create_schema(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
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
                recommendation TEXT
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

            CREATE INDEX IF NOT EXISTS idx_pages_crawl ON pages(crawl_id);
            CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_url);
            CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_url);
            CREATE INDEX IF NOT EXISTS idx_findings_page ON findings(page_id);
            CREATE INDEX IF NOT EXISTS idx_findings_category ON findings(category);
            CREATE INDEX IF NOT EXISTS idx_findings_severity ON findings(severity);
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
    pub fn insert_page(&self, crawl_id: &str, page: &PageData) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO pages (id, crawl_id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
            ],
        )?;

        // Insert links
        let mut stmt = conn.prepare(
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

        Ok(())
    }

    /// Insert a batch of pages for performance.
    pub fn insert_pages(&self, crawl_id: &str, pages: &[PageData]) -> Result<(), StorageError> {
        for page in pages {
            self.insert_page(crawl_id, page)?;
        }
        Ok(())
    }

    /// Insert a single issue/finding into the database.
    pub fn insert_issue(&self, issue: &Issue) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO findings (id, page_id, category, severity, code, title, description, element, recommendation)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
            ],
        )?;
        Ok(())
    }

    /// Insert a batch of issues for performance.
    pub fn insert_issues(&self, issues: &[Issue]) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "INSERT INTO findings (id, page_id, category, severity, code, title, description, element, recommendation)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
            ])?;
        }
        Ok(())
    }

    /// Retrieve pages with a limit.
    pub fn get_pages(&self, crawl_id: &str, limit: usize) -> Result<Vec<PageData>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at
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
                    url: Url::parse(&url_str)
                        .unwrap_or_else(|_| Url::parse("about:blank").unwrap()),
                    final_url: Url::parse(&final_url_str)
                        .unwrap_or_else(|_| Url::parse("about:blank").unwrap()),
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
        let conn = self.conn.lock().unwrap();

        let mut query = String::from(
            "SELECT f.id, f.page_id, f.category, f.severity, f.code, f.title, f.description, f.element, f.recommendation
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
                    category: IssueCategory::from_str(&category_str),
                    severity: Severity::from_str(&severity_str).unwrap_or(Severity::Info),
                    code: row.get(4)?,
                    title: row.get(5)?,
                    description: row.get(6)?,
                    element: row.get(7)?,
                    recommendation: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(issues)
    }

    /// Get aggregate statistics for a crawl.
    pub fn get_stats(&self, crawl_id: &str) -> Result<CrawlStats, StorageError> {
        let conn = self.conn.lock().unwrap();

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
}

#[cfg(test)]
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
            assert_eq!(Severity::from_str(s), Some(sev));
        }
        assert_eq!(Severity::from_str("invalid"), None);
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
            assert_eq!(IssueCategory::from_str(&s), cat);
        }
        let custom = IssueCategory::Custom("myplugin".to_string());
        let s = custom.as_str();
        assert_eq!(IssueCategory::from_str(&s), custom);
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
}
