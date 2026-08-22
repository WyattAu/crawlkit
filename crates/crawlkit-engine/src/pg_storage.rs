use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use sqlx::Row;
use url::Url;

use crate::storage::{CrawlStats, Issue, IssueFilter, PageData, StorageError};
use crate::storage_trait::StorageBackend;

/// PostgreSQL-backed storage for crawl data.
///
/// Uses sqlx with a connection pool for async database access.
/// Implements [`StorageBackend`] for seamless integration with the crawl engine.
///
/// # Examples
///
/// ```rust,no_run
/// use crawlkit_engine::pg_storage::PgStorage;
/// use crawlkit_engine::storage_trait::StorageBackend;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = PgStorage::new("postgres://localhost/crawlkit").await?;
/// storage.migrate().await?;
///
/// let crawl_id = storage.start_crawl("https://example.com", None)?;
/// // ... use storage
/// storage.finish()?;
/// # Ok(())
/// # }
/// ```
pub struct PgStorage {
    pool: PgPool,
}

/// Process-global runtime driving the synchronous `StorageBackend` bridge.
///
/// The sync trait methods must not call `Handle::current().block_on`:
/// that panics inside async contexts ("cannot block_on within a runtime")
/// and outside any runtime entirely. This shared runtime is lazily created
/// once per process and intentionally never dropped (statics are not
/// dropped), so it can be used safely from synchronous code and from
/// `spawn_blocking` threads.
static BLOCKING_RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

/// Borrow the global bridge runtime, creating it on first use.
fn blocking_runtime() -> &'static tokio::runtime::Runtime {
    // `get_or_init` cannot propagate errors, and runtime construction
    // only fails under process-fatal conditions (thread/resource
    // exhaustion); panicking here matches `tokio::main`'s behavior.
    #[allow(clippy::panic)]
    BLOCKING_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .thread_name("crawlkit-pg-bridge")
            .build()
            .unwrap_or_else(|e| panic!("PgStorage failed to build its blocking runtime: {e}"))
    })
}

impl PgStorage {
    /// Create a new PostgreSQL storage instance.
    ///
    /// Establishes a connection pool to the PostgreSQL database.
    /// Does not run migrations; call [`migrate`](Self::migrate) separately.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::PgDatabase`] if the connection cannot be established.
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    /// Create a new PostgreSQL storage with a custom pool.
    ///
    /// Useful when you need to configure pool settings (max connections, timeouts, etc.).
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run database migrations.
    ///
    /// Creates all required tables and indexes if they don't exist.
    /// Safe to call multiple times (uses `IF NOT EXISTS`).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::PgMigration`] if the migration fails.
    pub async fn migrate(&self) -> Result<(), StorageError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| StorageError::PgMigration(e.to_string()))?;
        Ok(())
    }

    /// Get a reference to the underlying connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Helper to parse a URL string with a fallback to "about:invalid".
fn parse_url_safe(s: &str) -> Url {
    Url::parse(s).unwrap_or_else(|_| {
        Url::parse("about:invalid")
            .unwrap_or_else(|_| unreachable!("about:invalid is always valid"))
    })
}

/// Extract a `PageData` from a sqlx row.
fn row_to_page_data(row: &sqlx::postgres::PgRow) -> Result<PageData, sqlx::error::BoxDynError> {
    let url_str: String = row.try_get("url")?;
    let final_url_str: String = row.try_get("final_url")?;
    let canonical_str: Option<String> = row.try_get("canonical")?;

    Ok(PageData {
        id: row.try_get("id")?,
        url: parse_url_safe(&url_str),
        final_url: parse_url_safe(&final_url_str),
        status_code: row.try_get::<i32, _>("status_code")? as u16,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        canonical_url: canonical_str.and_then(|s| Url::parse(&s).ok()),
        word_count: row
            .try_get::<Option<i64>, _>("word_count")?
            .map(|v| v as usize),
        load_time_ms: row
            .try_get::<Option<i64>, _>("load_time_ms")?
            .map(|v| v as u64),
        body_size: row
            .try_get::<Option<i64>, _>("body_size")?
            .map(|v| v as usize),
        fetched_at: row.try_get("fetched_at")?,
        links: Vec::new(),
        tenant_id: row.try_get("tenant_id")?,
        etag: row.try_get("etag")?,
        last_modified: row.try_get("last_modified")?,
        cwv_lcp: row.try_get("cwv_lcp")?,
        cwv_cls: row.try_get("cwv_cls")?,
        cwv_inp: row.try_get("cwv_inp")?,
    })
}

/// Extract an `Issue` from a sqlx row.
fn row_to_issue(row: &sqlx::postgres::PgRow) -> Result<Issue, sqlx::error::BoxDynError> {
    let category_str: String = row.try_get("category")?;
    let severity_str: String = row.try_get("severity")?;

    Ok(Issue {
        id: row.try_get("id")?,
        page_id: row.try_get("page_id")?,
        category: crate::types::IssueCategory::parse_category(&category_str),
        severity: crate::types::Severity::parse_severity(&severity_str)
            .unwrap_or(crate::types::Severity::Info),
        code: row.try_get("code")?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        element: row.try_get("element")?,
        recommendation: row.try_get("recommendation")?,
        tenant_id: row.try_get("tenant_id")?,
    })
}

impl PgStorage {
    fn get_issues_internal(
        &self,
        crawl_id: &str,
        tenant_id: Option<&str>,
        filters: &IssueFilter,
    ) -> Result<Vec<Issue>, StorageError> {
        let pool = self.pool.clone();
        let crawl_id = crawl_id.to_string();
        let tenant_id = tenant_id.map(|s| s.to_string());
        let filters = filters.clone();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let mut query = String::from(
                "SELECT f.id, f.page_id, f.category, f.severity, f.code, f.title, f.description, f.element, f.recommendation, f.tenant_id
                 FROM findings f
                 JOIN pages p ON f.page_id = p.id
                 WHERE p.crawl_id = $1",
            );

            let mut param_index = 2;

            if tenant_id.is_some() {
                query.push_str(&format!(
                    " AND (f.tenant_id = ${} OR f.tenant_id IS NULL)",
                    param_index
                ));
                param_index += 1;
            }

            if filters.severity.is_some() {
                query.push_str(&format!(" AND f.severity = ${}", param_index));
                param_index += 1;
            }
            if filters.category.is_some() {
                query.push_str(&format!(" AND f.category = ${}", param_index));
                param_index += 1;
            }
            if filters.page_id.is_some() {
                query.push_str(&format!(" AND f.page_id = ${}", param_index));
                param_index += 1;
            }
            if filters.code_prefix.is_some() {
                query.push_str(&format!(" AND f.code LIKE ${}", param_index));
            }

            query.push_str(" ORDER BY f.id ASC");

            // AssertSqlSafe (audit annotation): the only dynamic parts are
            // positional placeholders ($2..$n) — values flow through .bind().
            let mut q = sqlx::query(sqlx::AssertSqlSafe(query)).bind(&crawl_id);
            if let Some(ref tid) = tenant_id {
                q = q.bind(tid);
            }
            if let Some(ref severity) = filters.severity {
                q = q.bind(severity.as_str());
            }
            if let Some(ref category) = filters.category {
                q = q.bind(category.as_str());
            }
            if let Some(ref page_id) = filters.page_id {
                q = q.bind(page_id);
            }
            if let Some(ref code_prefix) = filters.code_prefix {
                q = q.bind(format!("{code_prefix}%"));
            }

            let rows = q.fetch_all(&pool).await?;

            let issues = rows
                .iter()
                .map(|r| row_to_issue(r))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| StorageError::PgDatabase(sqlx::Error::Decode(e)))?;

            Ok(issues)
        })
    }
}

#[async_trait]
impl StorageBackend for PgStorage {
    fn start_crawl(
        &self,
        seed_url: &str,
        _tenant_id: Option<&str>,
    ) -> Result<String, StorageError> {
        let crawl_id = uuid::Uuid::new_v4().to_string();
        let pool = self.pool.clone();
        let seed_url = seed_url.to_string();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            sqlx::query("INSERT INTO crawls (id, start_time, target_url) VALUES ($1, $2, $3)")
                .bind(&crawl_id)
                .bind(Utc::now())
                .bind(&seed_url)
                .execute(&pool)
                .await?;
            Ok::<_, StorageError>(crawl_id)
        })
    }

    fn finish_crawl(
        &self,
        crawl_id: &str,
        pages: usize,
        issues: usize,
    ) -> Result<(), StorageError> {
        let pool = self.pool.clone();
        let crawl_id = crawl_id.to_string();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            sqlx::query(
                "UPDATE crawls SET end_time = $1, pages_crawled = $2, total_issues = $3 WHERE id = $4",
            )
            .bind(Utc::now())
            .bind(pages as i64)
            .bind(issues as i64)
            .bind(&crawl_id)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn insert_page(&self, crawl_id: &str, page: &PageData) -> Result<(), StorageError> {
        let pool = self.pool.clone();
        let crawl_id = crawl_id.to_string();
        let page = page.clone();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let mut tx = pool.begin().await?;

            sqlx::query(
                "INSERT INTO pages (id, crawl_id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at, tenant_id, etag, last_modified, cwv_lcp, cwv_cls, cwv_inp)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
                 ON CONFLICT (id) DO UPDATE SET
                     crawl_id = EXCLUDED.crawl_id,
                     url = EXCLUDED.url,
                     final_url = EXCLUDED.final_url,
                     status_code = EXCLUDED.status_code,
                     title = EXCLUDED.title,
                     description = EXCLUDED.description,
                     canonical = EXCLUDED.canonical,
                     word_count = EXCLUDED.word_count,
                     load_time_ms = EXCLUDED.load_time_ms,
                     body_size = EXCLUDED.body_size,
                     fetched_at = EXCLUDED.fetched_at,
                     tenant_id = EXCLUDED.tenant_id,
                     etag = EXCLUDED.etag,
                     last_modified = EXCLUDED.last_modified,
                     cwv_lcp = EXCLUDED.cwv_lcp,
                     cwv_cls = EXCLUDED.cwv_cls,
                     cwv_inp = EXCLUDED.cwv_inp",
            )
            .bind(&page.id)
            .bind(&crawl_id)
            .bind(page.url.as_str())
            .bind(page.final_url.as_str())
            .bind(page.status_code as i32)
            .bind(&page.title)
            .bind(&page.description)
            .bind(page.canonical_url.as_ref().map(|u| u.as_str()))
            .bind(page.word_count.map(|v| v as i64))
            .bind(page.load_time_ms.map(|v| v as i64))
            .bind(page.body_size.map(|v| v as i64))
            .bind(page.fetched_at)
            .bind(&page.tenant_id)
            .bind(&page.etag)
            .bind(&page.last_modified)
            .bind(page.cwv_lcp)
            .bind(page.cwv_cls)
            .bind(page.cwv_inp)
            .execute(&mut *tx)
            .await?;

            for link in &page.links {
                let link_id = uuid::Uuid::new_v4().to_string();
                let is_external = link.domain() != page.url.domain();
                sqlx::query(
                    "INSERT INTO links (id, page_id, source_url, target_url, is_external) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(&link_id)
                .bind(&page.id)
                .bind(page.url.as_str())
                .bind(link.as_str())
                .bind(is_external)
                .execute(&mut *tx)
                .await?;
            }

            tx.commit().await?;
            Ok(())
        })
    }

    fn insert_pages_batch(&self, crawl_id: &str, pages: &[PageData]) -> Result<(), StorageError> {
        if pages.is_empty() {
            return Ok(());
        }
        let pool = self.pool.clone();
        let crawl_id = crawl_id.to_string();
        let pages = pages.to_vec();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let mut tx = pool.begin().await?;

            for page in &pages {
                sqlx::query(
                    "INSERT INTO pages (id, crawl_id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at, tenant_id, etag, last_modified, cwv_lcp, cwv_cls, cwv_inp)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
                     ON CONFLICT (id) DO UPDATE SET
                         crawl_id = EXCLUDED.crawl_id,
                         url = EXCLUDED.url,
                         final_url = EXCLUDED.final_url,
                         status_code = EXCLUDED.status_code,
                         title = EXCLUDED.title,
                         description = EXCLUDED.description,
                         canonical = EXCLUDED.canonical,
                         word_count = EXCLUDED.word_count,
                         load_time_ms = EXCLUDED.load_time_ms,
                         body_size = EXCLUDED.body_size,
                         fetched_at = EXCLUDED.fetched_at,
                         tenant_id = EXCLUDED.tenant_id,
                         etag = EXCLUDED.etag,
                         last_modified = EXCLUDED.last_modified,
                         cwv_lcp = EXCLUDED.cwv_lcp,
                         cwv_cls = EXCLUDED.cwv_cls,
                         cwv_inp = EXCLUDED.cwv_inp",
                )
                .bind(&page.id)
                .bind(&crawl_id)
                .bind(page.url.as_str())
                .bind(page.final_url.as_str())
                .bind(page.status_code as i32)
                .bind(&page.title)
                .bind(&page.description)
                .bind(page.canonical_url.as_ref().map(|u| u.as_str()))
                .bind(page.word_count.map(|v| v as i64))
                .bind(page.load_time_ms.map(|v| v as i64))
                .bind(page.body_size.map(|v| v as i64))
                .bind(page.fetched_at)
                .bind(&page.tenant_id)
                .bind(&page.etag)
                .bind(&page.last_modified)
                .bind(page.cwv_lcp)
                .bind(page.cwv_cls)
                .bind(page.cwv_inp)
                .execute(&mut *tx)
                .await?;

                for link in &page.links {
                    let link_id = uuid::Uuid::new_v4().to_string();
                    let is_external = link.domain() != page.url.domain();
                    sqlx::query(
                        "INSERT INTO links (id, page_id, source_url, target_url, is_external) VALUES ($1, $2, $3, $4, $5)",
                    )
                    .bind(&link_id)
                    .bind(&page.id)
                    .bind(page.url.as_str())
                    .bind(link.as_str())
                    .bind(is_external)
                    .execute(&mut *tx)
                    .await?;
                }
            }

            tx.commit().await?;
            Ok(())
        })
    }

    fn get_page(&self, crawl_id: &str, url: &str) -> Result<Option<PageData>, StorageError> {
        let pool = self.pool.clone();
        let crawl_id = crawl_id.to_string();
        let url = url.to_string();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let result = sqlx::query(
                "SELECT id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at, tenant_id, etag, last_modified, cwv_lcp, cwv_cls, cwv_inp
                 FROM pages WHERE crawl_id = $1 AND url = $2",
            )
            .bind(&crawl_id)
            .bind(&url)
            .fetch_optional(&pool)
            .await?;

            result
                .map(|r| row_to_page_data(&r))
                .transpose()
                .map_err(|e| StorageError::PgDatabase(sqlx::Error::Decode(e)))
        })
    }

    fn get_pages(&self, crawl_id: &str, limit: usize) -> Result<Vec<PageData>, StorageError> {
        let pool = self.pool.clone();
        let crawl_id = crawl_id.to_string();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let rows = sqlx::query(
                "SELECT id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at, tenant_id, etag, last_modified, cwv_lcp, cwv_cls, cwv_inp
                 FROM pages WHERE crawl_id = $1 ORDER BY fetched_at ASC LIMIT $2",
            )
            .bind(&crawl_id)
            .bind(limit as i64)
            .fetch_all(&pool)
            .await?;

            rows.iter()
                .map(|r| row_to_page_data(r))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| StorageError::PgDatabase(sqlx::Error::Decode(e)))
        })
    }

    fn get_pages_for_tenant(
        &self,
        crawl_id: &str,
        tenant_id: &str,
        limit: usize,
    ) -> Result<Vec<PageData>, StorageError> {
        let pool = self.pool.clone();
        let crawl_id = crawl_id.to_string();
        let tenant_id = tenant_id.to_string();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let rows = sqlx::query(
                "SELECT id, url, final_url, status_code, title, description, canonical, word_count, load_time_ms, body_size, fetched_at, tenant_id, etag, last_modified, cwv_lcp, cwv_cls, cwv_inp
                 FROM pages WHERE crawl_id = $1 AND (tenant_id = $2 OR tenant_id IS NULL)
                 ORDER BY fetched_at ASC LIMIT $3",
            )
            .bind(&crawl_id)
            .bind(&tenant_id)
            .bind(limit as i64)
            .fetch_all(&pool)
            .await?;

            rows.iter()
                .map(|r| row_to_page_data(r))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| StorageError::PgDatabase(sqlx::Error::Decode(e)))
        })
    }

    fn insert_issue(&self, issue: &Issue) -> Result<(), StorageError> {
        let pool = self.pool.clone();
        let issue = issue.clone();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            sqlx::query(
                "INSERT INTO findings (id, page_id, category, severity, code, title, description, element, recommendation, tenant_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(&issue.id)
            .bind(&issue.page_id)
            .bind(issue.category.as_str())
            .bind(issue.severity.as_str())
            .bind(&issue.code)
            .bind(&issue.title)
            .bind(&issue.description)
            .bind(&issue.element)
            .bind(&issue.recommendation)
            .bind(&issue.tenant_id)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn insert_issues_batch(&self, issues: &[Issue]) -> Result<(), StorageError> {
        if issues.is_empty() {
            return Ok(());
        }
        let pool = self.pool.clone();
        let issues = issues.to_vec();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let mut tx = pool.begin().await?;

            for issue in &issues {
                sqlx::query(
                    "INSERT INTO findings (id, page_id, category, severity, code, title, description, element, recommendation, tenant_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(&issue.id)
                .bind(&issue.page_id)
                .bind(issue.category.as_str())
                .bind(issue.severity.as_str())
                .bind(&issue.code)
                .bind(&issue.title)
                .bind(&issue.description)
                .bind(&issue.element)
                .bind(&issue.recommendation)
                .bind(&issue.tenant_id)
                .execute(&mut *tx)
                .await?;
            }

            tx.commit().await?;
            Ok(())
        })
    }

    fn get_issues(
        &self,
        crawl_id: &str,
        filters: &IssueFilter,
    ) -> Result<Vec<Issue>, StorageError> {
        self.get_issues_internal(crawl_id, None, filters)
    }

    fn get_issues_for_tenant(
        &self,
        crawl_id: &str,
        tenant_id: &str,
        filters: &IssueFilter,
    ) -> Result<Vec<Issue>, StorageError> {
        self.get_issues_internal(crawl_id, Some(tenant_id), filters)
    }

    fn get_stats(&self, crawl_id: &str) -> Result<CrawlStats, StorageError> {
        let pool = self.pool.clone();
        let crawl_id = crawl_id.to_string();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let total_pages: (i64,) = sqlx::query_as(
                "SELECT COALESCE(COUNT(*), 0) FROM pages WHERE crawl_id = $1",
            )
            .bind(&crawl_id)
            .fetch_one(&pool)
            .await?;

            let total_issues: (i64,) = sqlx::query_as(
                "SELECT COALESCE(COUNT(*), 0) FROM findings f JOIN pages p ON f.page_id = p.id WHERE p.crawl_id = $1",
            )
            .bind(&crawl_id)
            .fetch_one(&pool)
            .await?;

            let severity_rows = sqlx::query(
                "SELECT f.severity, COUNT(*) as count FROM findings f JOIN pages p ON f.page_id = p.id WHERE p.crawl_id = $1 GROUP BY f.severity",
            )
            .bind(&crawl_id)
            .fetch_all(&pool)
            .await?;

            let mut issues_by_severity = std::collections::HashMap::new();
            for row in severity_rows {
                let sev: String = row.try_get("severity")?;
                let count: i64 = row.try_get("count")?;
                issues_by_severity.insert(sev, count as usize);
            }

            let category_rows = sqlx::query(
                "SELECT f.category, COUNT(*) as count FROM findings f JOIN pages p ON f.page_id = p.id WHERE p.crawl_id = $1 GROUP BY f.category",
            )
            .bind(&crawl_id)
            .fetch_all(&pool)
            .await?;

            let mut issues_by_category = std::collections::HashMap::new();
            for row in category_rows {
                let cat: String = row.try_get("category")?;
                let count: i64 = row.try_get("count")?;
                issues_by_category.insert(cat, count as usize);
            }

            let avg_response: (Option<f64>,) = sqlx::query_as(
                "SELECT AVG(load_time_ms)::DOUBLE PRECISION FROM pages WHERE crawl_id = $1 AND load_time_ms IS NOT NULL",
            )
            .bind(&crawl_id)
            .fetch_one(&pool)
            .await?;

            let total_body: (Option<i64>,) = sqlx::query_as(
                "SELECT SUM(body_size)::BIGINT FROM pages WHERE crawl_id = $1 AND body_size IS NOT NULL",
            )
            .bind(&crawl_id)
            .fetch_one(&pool)
            .await?;

            Ok(CrawlStats {
                total_pages: total_pages.0 as usize,
                total_issues: total_issues.0 as usize,
                issues_by_severity,
                issues_by_category,
                avg_response_time_ms: avg_response.0,
                total_body_size: total_body.0.map(|v| v as usize),
            })
        })
    }

    fn get_page_conditional(
        &self,
        crawl_id: &str,
        url: &str,
    ) -> Result<Option<(String, Option<String>, Option<String>)>, StorageError> {
        let pool = self.pool.clone();
        let crawl_id = crawl_id.to_string();
        let url = url.to_string();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let result = sqlx::query(
                "SELECT id, etag, last_modified FROM pages WHERE crawl_id = $1 AND url = $2",
            )
            .bind(&crawl_id)
            .bind(&url)
            .fetch_optional(&pool)
            .await?;

            result
                .map(|r| {
                    let id: String = r.try_get("id")?;
                    let etag: Option<String> = r.try_get("etag")?;
                    let last_modified: Option<String> = r.try_get("last_modified")?;
                    Ok::<_, StorageError>((id, etag, last_modified))
                })
                .transpose()
        })
    }

    fn get_latest_conditional(
        &self,
        url: &str,
    ) -> Result<Option<(Option<String>, Option<String>)>, StorageError> {
        let pool = self.pool.clone();
        let url = url.to_string();

        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            let result = sqlx::query(
                "SELECT etag, last_modified FROM pages WHERE url = $1 ORDER BY fetched_at DESC LIMIT 1",
            )
            .bind(&url)
            .fetch_optional(&pool)
            .await?;

            result
                .map(|r| {
                    let etag: Option<String> = r.try_get("etag")?;
                    let last_modified: Option<String> = r.try_get("last_modified")?;
                    Ok::<_, StorageError>((etag, last_modified))
                })
                .transpose()
        })
    }

    fn finish(&self) -> Result<(), StorageError> {
        let pool = self.pool.clone();
        let rt = blocking_runtime().handle().clone();
        rt.block_on(async {
            pool.close().await;
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::storage::{IssueCategory, Severity};
    use crate::storage_trait::StorageBackend;

    /// A lazily-connecting pool that never actually reaches a server.
    /// sqlx requires a Tokio context even for lazy pool setup, so the pool
    /// is created inside a throwaway runtime.
    fn unreachable_pool() -> PgPool {
        let pool = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
            .block_on(async {
                sqlx::postgres::PgPoolOptions::new()
                    .max_connections(1)
                    .acquire_timeout(std::time::Duration::from_millis(250))
                    .connect_lazy("postgres://invalid:invalid@127.0.0.1:1/none")
                    .expect("lazy pool does not connect")
            });
        pool
    }

    /// Regression: the sync trait bridge previously called
    /// `Handle::current().block_on`, which panics outside any runtime —
    /// including from plain synchronous code and `spawn_blocking` threads.
    /// With the dedicated runtime, calls from a no-runtime context return a
    /// database error instead of panicking.
    #[test]
    fn sync_trait_methods_do_not_panic_outside_tokio() {
        let storage = PgStorage::from_pool(unreachable_pool());

        // No ambient tokio runtime here; the old implementation panicked at
        // `Handle::current()` before ever reaching the database.
        let result = storage.start_crawl("https://example.com", None);
        assert!(
            result.is_err(),
            "expected a connection error, got {result:?}"
        );
    }

    /// The same contract from inside `spawn_blocking` — the supported
    /// calling convention for sync storage on an async runtime.
    #[tokio::test]
    async fn sync_trait_methods_work_from_spawn_blocking() {
        // Pool setup itself needs a non-async thread (sqlx lazy pools
        // require a runtime context, and runtimes cannot nest).
        let storage = std::sync::Arc::new(
            tokio::task::spawn_blocking(|| PgStorage::from_pool(unreachable_pool()))
                .await
                .expect("pool setup"),
        );

        let storage_clone = storage.clone();
        let result = tokio::task::spawn_blocking(move || {
            storage_clone.start_crawl("https://example.com", None)
        })
        .await
        .expect("spawn_blocking join");
        assert!(
            result.is_err(),
            "expected a connection error, got {result:?}"
        );
    }

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

    /// Sync bootstrap for `#[ignore]`d service-backed tests.
    ///
    /// The `StorageBackend` trait is synchronous by contract; its callers
    /// are expected to be plain threads or `spawn_blocking` workers. These
    /// tests therefore run WITHOUT a `#[tokio::test]` context (block_on
    /// inside a tokio worker panics — the very bug this suite once had) and
    /// build their storage handle via the module's blocking runtime.
    /// Run-unique id so parallel tests and repeated suite runs never
    /// collide on primary keys.
    fn unique(prefix: &str) -> String {
        format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
    }

    fn sync_test_storage() -> PgStorage {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/crawlkit_test".to_string());
        blocking_runtime().block_on(async {
            let storage = PgStorage::new(&url).await.unwrap();
            storage.migrate().await.unwrap();
            storage
        })
    }

    #[test]
    #[ignore] // Requires a running PostgreSQL instance (DATABASE_URL)
    fn test_pg_start_and_finish_crawl() {
        let storage = sync_test_storage();

        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        assert!(!crawl_id.is_empty());
        storage.finish_crawl(&crawl_id, 0, 0).unwrap();
        storage.finish().unwrap();
    }

    #[test]
    #[ignore] // Requires a running PostgreSQL instance (DATABASE_URL)
    fn test_pg_insert_and_get_page() {
        let storage = sync_test_storage();

        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        let page = test_page(&unique("p"), "https://example.com/", 200);
        let expected_id = page.id.clone();
        storage.insert_page(&crawl_id, &page).unwrap();

        let retrieved = storage.get_page(&crawl_id, "https://example.com/").unwrap();
        assert!(retrieved.is_some());
        let p = retrieved.unwrap();
        assert_eq!(p.id, expected_id);
        assert_eq!(p.status_code, 200);

        storage.finish().unwrap();
    }

    #[test]
    #[ignore] // Requires a running PostgreSQL instance (DATABASE_URL)
    fn test_pg_insert_pages_batch() {
        let storage = sync_test_storage();

        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        let pages = vec![
            test_page(&unique("p"), "https://example.com/", 200),
            test_page(&unique("p"), "https://example.com/about", 200),
        ];
        storage.insert_pages_batch(&crawl_id, &pages).unwrap();

        let retrieved = storage.get_pages(&crawl_id, 10).unwrap();
        assert_eq!(retrieved.len(), 2);
        storage.finish().unwrap();
    }

    #[test]
    #[ignore] // Requires a running PostgreSQL instance (DATABASE_URL)
    fn test_pg_tenant_isolation() {
        let storage = sync_test_storage();

        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let mut page_a = test_page(&unique("p"), "https://example.com/a", 200);
        let id_a = page_a.id.clone();
        page_a.tenant_id = Some("tenant_a".to_string());
        let mut page_b = test_page(&unique("p"), "https://example.com/b", 200);
        page_b.tenant_id = Some("tenant_b".to_string());
        let page_shared = test_page(&unique("p"), "https://example.com/c", 200);
        let id_shared = page_shared.id.clone();

        storage.insert_page(&crawl_id, &page_a).unwrap();
        storage.insert_page(&crawl_id, &page_b).unwrap();
        storage.insert_page(&crawl_id, &page_shared).unwrap();

        let pages_a = storage
            .get_pages_for_tenant(&crawl_id, "tenant_a", 10)
            .unwrap();
        assert_eq!(pages_a.len(), 2);
        assert!(pages_a.iter().any(|p| p.id == id_a));
        assert!(pages_a.iter().any(|p| p.id == id_shared));

        let pages_b = storage
            .get_pages_for_tenant(&crawl_id, "tenant_b", 10)
            .unwrap();
        assert_eq!(pages_b.len(), 2);
        assert!(pages_b.iter().any(|p| p.id == page_b.id));
        assert!(pages_b.iter().any(|p| p.id == id_shared));

        storage.finish().unwrap();
    }

    #[test]
    #[ignore] // Requires a running PostgreSQL instance (DATABASE_URL)
    fn test_pg_insert_and_get_issues() {
        let storage = sync_test_storage();

        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        let page = test_page(&unique("p"), "https://example.com/", 200);
        storage.insert_page(&crawl_id, &page).unwrap();

        let pid = page.id.as_str();
        let issues = vec![
            test_issue(&unique("i"), pid, IssueCategory::Seo, Severity::Error),
            test_issue(&unique("i"), pid, IssueCategory::Images, Severity::Warning),
        ];
        storage.insert_issues_batch(&issues).unwrap();

        let retrieved = storage
            .get_issues(&crawl_id, &IssueFilter::default())
            .unwrap();
        assert_eq!(retrieved.len(), 2);
        storage.finish().unwrap();
    }

    #[test]
    #[ignore] // Requires a running PostgreSQL instance (DATABASE_URL)
    fn test_pg_stats() {
        let storage = sync_test_storage();

        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        let page = test_page(&unique("p"), "https://example.com/", 200);
        storage.insert_page(&crawl_id, &page).unwrap();
        let issue = test_issue(
            &unique("i"),
            page.id.as_str(),
            IssueCategory::Seo,
            Severity::Error,
        );
        storage.insert_issue(&issue).unwrap();

        let stats = storage.get_stats(&crawl_id).unwrap();
        assert_eq!(stats.total_pages, 1);
        assert_eq!(stats.total_issues, 1);
        storage.finish().unwrap();
    }

    #[test]
    #[ignore] // Requires a running PostgreSQL instance (DATABASE_URL)
    fn test_pg_conditional_requests() {
        let storage = sync_test_storage();

        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        let mut page = test_page(&unique("p"), "https://example.com/", 200);
        let expected_page_id = page.id.clone();
        page.etag = Some("\"abc123\"".to_string());
        page.last_modified = Some("Wed, 21 Oct 2024 07:28:00 GMT".to_string());
        storage.insert_page(&crawl_id, &page).unwrap();

        let result = storage
            .get_page_conditional(&crawl_id, "https://example.com/")
            .unwrap();
        assert!(result.is_some());
        let (page_id, etag, last_modified) = result.unwrap();
        assert_eq!(page_id, expected_page_id);
        assert_eq!(etag.as_deref(), Some("\"abc123\""));
        assert_eq!(
            last_modified.as_deref(),
            Some("Wed, 21 Oct 2024 07:28:00 GMT")
        );

        let result = storage
            .get_latest_conditional("https://example.com/")
            .unwrap();
        assert!(result.is_some());
        storage.finish().unwrap();
    }
}
