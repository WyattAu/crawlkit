use crate::analyzers::AnalyzerRegistry;
use crate::encryption::EncryptionManager;
use crate::http::{HttpClient, HttpClientConfig};
use crate::queue::{Priority, UrlQueue};
use crate::ratelimit::RateLimiter;
use crate::robots::RobotsTxtCache;
use crate::sitemap::SitemapCache;
use crate::storage::{Issue, PageData, Storage};
use crate::{
    CircuitBreakerRegistry, CrawlConfig, DeterminismController, FeatureFlags, Metrics, RedirectHop,
    ResourceMonitor,
};
use chrono::Utc;
use dashmap::DashSet;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;

/// Freshness classification for incremental crawls.
#[derive(Debug, Clone, Copy)]
enum Freshness {
    /// First time this URL has been seen in an incremental crawl.
    New,
    /// Previously seen; content changed since the last crawl.
    Modified,
    /// Incremental crawling disabled — no classification performed.
    Unconditional,
}

/// The outcome of a single worker fetch.
///
/// Replaces the former magic-string encoding of 304 responses and carries
/// the freshness classification needed for incremental-crawl statistics.
enum FetchOutcome {
    /// Page fetched successfully.
    Fetched {
        result: crate::FetchResult,
        freshness: Freshness,
    },
    /// Server answered 304 Not Modified. `page_id` identifies the stored page
    /// whose `fetched_at` timestamp should be refreshed.
    NotModified { page_id: Option<String> },
    /// Fetch failed after retries.
    Failed(crate::CrawlError),
}

/// A completed fetch, as delivered from a worker task to the main loop.
struct FetchedPage {
    entry: crate::queue::QueueEntry,
    robots_raw: String,
    fetch_time: Duration,
    outcome: FetchOutcome,
}

/// Trait for JavaScript page rendering.
///
/// Implementations can render JavaScript-heavy pages to their final HTML.
/// The engine uses this trait to optionally render pages before analysis.
#[async_trait::async_trait]
pub trait JsRenderer: Send + Sync {
    /// Check if the renderer is available and ready to use.
    fn is_available(&self) -> bool;

    /// Render a page at the given URL and return the final HTML.
    async fn render(&self, url: &str) -> Result<String, String>;
}

/// Configuration for the crawl engine.
///
/// Bundles all settings needed to run a crawl. This is the primary
/// input to [`CrawlEngine::new`].
pub struct CrawlEngineConfig {
    /// The crawl configuration (URLs, limits, politeness).
    pub crawl_config: CrawlConfig,

    /// Feature flags controlling analyzer sets and capabilities.
    pub feature_flags: FeatureFlags,

    /// Whether to enable JavaScript rendering via Playwright.
    pub enable_js_rendering: bool,

    /// Optional JS renderer for rendering JavaScript-heavy pages.
    pub js_renderer: Option<Arc<dyn JsRenderer>>,

    /// Whether to allow crawling external domains.
    pub allow_external: bool,

    /// URL include patterns (glob-style). Empty means allow all.
    pub include_patterns: Vec<String>,

    /// URL exclude patterns (glob-style).
    pub exclude_patterns: Vec<String>,

    /// Optional random seed for deterministic crawls.
    pub seed: Option<u64>,

    /// Tenant ID for multi-tenant storage.
    pub tenant_id: Option<String>,

    /// Custom user agent string.
    pub user_agent: Option<String>,

    /// Optional encryption manager for encrypting sensitive fields at rest.
    pub encryption: Option<EncryptionManager>,

    /// HTTP request timeout in seconds.
    pub timeout_secs: Option<u64>,

    /// Delay between requests in milliseconds.
    pub delay_ms: Option<u64>,

    /// Number of concurrent fetchers.
    pub concurrency: Option<usize>,

    /// Whether to enable incremental crawling (ETag / If-Modified-Since).
    pub incremental: bool,

    /// Allow fetching over plain HTTP. Secure by default (`false`); intended
    /// for local test servers and trusted intranets.
    pub allow_http: bool,

    /// Whether to force a full re-crawl, ignoring cached ETag/Last-Modified conditions.
    pub force: bool,
}

impl std::fmt::Debug for CrawlEngineConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrawlEngineConfig")
            .field("crawl_config", &self.crawl_config)
            .field("feature_flags", &self.feature_flags)
            .field("enable_js_rendering", &self.enable_js_rendering)
            .field("js_renderer", &self.js_renderer.is_some())
            .field("allow_external", &self.allow_external)
            .field("include_patterns", &self.include_patterns)
            .field("exclude_patterns", &self.exclude_patterns)
            .field("seed", &self.seed)
            .field("tenant_id", &self.tenant_id)
            .field("user_agent", &self.user_agent)
            .field("encryption", &self.encryption.is_some())
            .field("timeout_secs", &self.timeout_secs)
            .field("delay_ms", &self.delay_ms)
            .field("concurrency", &self.concurrency)
            .field("incremental", &self.incremental)
            .field("force", &self.force)
            .finish()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for CrawlEngineConfig {
    fn default() -> Self {
        Self {
            crawl_config: CrawlConfig::default(),
            feature_flags: FeatureFlags::default(),
            enable_js_rendering: false,
            js_renderer: None,
            allow_external: false,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            seed: None,
            tenant_id: None,
            user_agent: None,
            encryption: None,
            timeout_secs: None,
            delay_ms: None,
            concurrency: None,
            incremental: false,
            force: false,
            allow_http: false,
        }
    }
}

/// Result returned after a crawl completes.
#[derive(Debug, Clone)]
pub struct CrawlOutput {
    /// The storage crawl ID.
    pub crawl_id: String,
    /// Number of pages successfully fetched and analyzed.
    pub pages_crawled: usize,
    /// Number of pages stored in the database.
    pub pages_stored: usize,
    /// Total analysis findings (issues) across all pages.
    pub issues_found: usize,
    /// Pages skipped because they were external and not allowed.
    pub skipped_external: usize,
    /// Pages skipped due to robots.txt disallow.
    pub skipped_robots: usize,
    /// Pages skipped due to duplicate content.
    pub skipped_duplicate: usize,
    /// Pages that returned 304 Not Modified during incremental crawl.
    pub pages_unchanged: usize,
    /// Pages that were modified (fetched fresh) during incremental crawl.
    pub pages_modified: usize,
    /// Pages that are new (not previously seen) during incremental crawl.
    pub pages_new: usize,
    /// The seed domain used for internal/external link classification.
    pub seed_domain: String,
    /// The metrics snapshot at crawl end.
    pub metrics: crate::MetricsSnapshot,
    /// The elapsed wall-clock time for the crawl.
    pub elapsed: Duration,
}

/// Callback invoked for each page successfully crawled and stored.
///
/// `page_url` is the URL, `page_id` is the storage ID, `findings_count`
/// is the number of analysis findings for that page.
pub type OnPageCrawled = Arc<dyn Fn(&str, &str, usize) + Send + Sync>;

// ---------------------------------------------------------------------------
// Crawl execution state
// ---------------------------------------------------------------------------

/// Relaxed-ordering counter helpers. Counters are statistics only; they never
/// gate correctness, so `Relaxed` ordering is sufficient and cheapest.
fn bump(counter: &AtomicUsize) {
    counter.fetch_add(1, Ordering::Relaxed);
}

fn bump_by(counter: &AtomicUsize, n: usize) {
    counter.fetch_add(n, Ordering::Relaxed);
}

fn current(counter: &AtomicUsize) -> usize {
    counter.load(Ordering::Relaxed)
}

/// Interior-mutable crawl counters shared across the fetch pipeline.
#[derive(Default)]
struct CrawlCounters {
    pages_crawled: AtomicUsize,
    pages_stored: AtomicUsize,
    issues_found: AtomicUsize,
    skipped_external: AtomicUsize,
    skipped_robots: AtomicUsize,
    skipped_duplicate: AtomicUsize,
    pages_unchanged: AtomicUsize,
    pages_modified: AtomicUsize,
    pages_new: AtomicUsize,
}

/// Plain-value snapshot of [`CrawlCounters`] for reporting.
#[derive(Debug, Clone, Copy)]
struct CounterSnapshot {
    pages_crawled: usize,
    pages_stored: usize,
    issues_found: usize,
    skipped_external: usize,
    skipped_robots: usize,
    skipped_duplicate: usize,
    pages_unchanged: usize,
    pages_modified: usize,
    pages_new: usize,
}

impl CrawlCounters {
    fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            pages_crawled: current(&self.pages_crawled),
            pages_stored: current(&self.pages_stored),
            issues_found: current(&self.issues_found),
            skipped_external: current(&self.skipped_external),
            skipped_robots: current(&self.skipped_robots),
            skipped_duplicate: current(&self.skipped_duplicate),
            pages_unchanged: current(&self.pages_unchanged),
            pages_modified: current(&self.pages_modified),
            pages_new: current(&self.pages_new),
        }
    }
}

/// Content-hash deduplication set. The strategy is chosen once at crawl start
/// based on whether deterministic (seeded) mode is enabled.
enum ContentHashes {
    Deterministic(DashSet<u64>),
    Sha256(DashSet<String>),
}

impl ContentHashes {
    fn deterministic() -> Self {
        Self::Deterministic(DashSet::new())
    }

    fn sha256() -> Self {
        Self::Sha256(DashSet::new())
    }

    /// Insert the body's hash; returns `false` if the content was already seen.
    fn insert(&self, body: &str) -> bool {
        match self {
            Self::Deterministic(set) => set.insert(DeterminismController::content_hash(body)),
            Self::Sha256(set) => {
                use sha2::{Digest, Sha256};
                let digest = Sha256::digest(body.as_bytes());
                set.insert(digest.iter().map(|b| format!("{b:02x}")).collect())
            }
        }
    }
}

/// Shared state for one crawl execution: counters, dedup sets, and the
/// collaborators needed to analyze and persist each fetched page.
///
/// Each stage of the per-page pipeline is a focused method
/// (dedup → render → analyze → store → link extraction), keeping every
/// unit small and independently testable.
struct CrawlRun<'a> {
    counters: CrawlCounters,
    visited: DashSet<String>,
    content_hashes: ContentHashes,
    analyzer_registry: &'a AnalyzerRegistry,
    cfg: &'a CrawlEngineConfig,
    storage: Arc<Storage>,
    crawl_id: String,
    seed_domain: String,
    on_page: Option<OnPageCrawled>,
    metrics: Metrics,
    resource_monitor: ResourceMonitor,
    queue: Arc<Mutex<UrlQueue>>,
}

impl CrawlRun<'_> {
    /// Route a completed fetch through dedup, analysis, storage, and discovery.
    async fn process(&self, fetched: &FetchedPage) {
        match &fetched.outcome {
            FetchOutcome::NotModified { page_id } => {
                self.record_not_modified(page_id).await;
            }
            FetchOutcome::Failed(err) => self.record_failure(&fetched.entry.url, err),
            FetchOutcome::Fetched { result, freshness } => {
                self.process_fetched(fetched, result, *freshness).await;
            }
        }
    }

    /// Handle a 304: count it and refresh the stored page's access timestamp.
    async fn record_not_modified(&self, page_id: &Option<String>) {
        bump(&self.counters.pages_unchanged);
        self.metrics.record_page_unchanged();
        let Some(id) = page_id.clone() else {
            return;
        };
        let storage = Arc::clone(&self.storage);
        let result =
            tokio::task::spawn_blocking(move || storage.update_page_fetched_at(&id, Utc::now()))
                .await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                tracing::warn!(error = %e, page_id = ?page_id, "Failed to refresh fetched_at")
            }
            Err(e) => {
                tracing::warn!(error = %e, page_id = ?page_id, "Blocking task join failed")
            }
        }
    }

    /// Handle a failed fetch.
    fn record_failure(&self, url: &Url, err: &crate::CrawlError) {
        tracing::warn!("Failed to fetch {}: {}", url, err);
        self.metrics.record_page_failure();
    }

    /// Process a successfully fetched page.
    async fn process_fetched(
        &self,
        fetched: &FetchedPage,
        result: &crate::FetchResult,
        freshness: Freshness,
    ) {
        if !self.content_hashes.insert(&result.body) {
            tracing::debug!("Skipping duplicate content: {}", fetched.entry.url);
            bump(&self.counters.skipped_duplicate);
            self.metrics.record_page_skipped_duplicate();
            return;
        }

        bump(&self.counters.pages_crawled);
        match freshness {
            Freshness::New => bump(&self.counters.pages_new),
            Freshness::Modified => bump(&self.counters.pages_modified),
            Freshness::Unconditional => {}
        }

        let mut body_text = result.body.clone();
        let mut parsed = crate::HtmlParser::parse(&body_text, &fetched.entry.url);

        self.render_js_if_needed(&fetched.entry.url, &mut body_text, &mut parsed)
            .await;

        let (findings, analysis_time) = self.analyze(&parsed, &body_text, result, fetched);
        bump_by(&self.counters.issues_found, findings.len());

        let page_id = uuid::Uuid::new_v4().to_string();
        let page_data = self.build_page_data(&page_id, &fetched.entry.url, result, &parsed);
        self.store(&page_data, &findings).await;

        self.metrics.record_page_success(
            result.body.len() as u64,
            fetched.fetch_time.as_micros() as u64,
            analysis_time.as_micros() as u64,
            0, // storage_time not tracked per page
            findings.len() as u64,
        );
        self.resource_monitor.record_page();

        if let Some(cb) = &self.on_page {
            cb(fetched.entry.url.as_ref(), &page_id, findings.len());
        }

        self.extract_links(&parsed, fetched.entry.depth).await;
    }

    /// Re-render with JavaScript when the decision engine requires it.
    async fn render_js_if_needed(
        &self,
        url: &Url,
        body_text: &mut String,
        parsed: &mut crate::ParsedPage,
    ) {
        if !self.cfg.enable_js_rendering {
            return;
        }
        let decision =
            crate::JsRenderDecisionEngine::new().should_render_js(url.as_ref(), Some(body_text));
        let crate::JsRenderDecision::Render { reason } = decision else {
            return;
        };
        tracing::info!("JS render decision for {}: {}", url, reason);

        let Some(renderer) = self.cfg.js_renderer.as_ref() else {
            return;
        };
        if !renderer.is_available() {
            tracing::warn!("JS renderer not available, using static HTML: {}", url);
            return;
        }
        match tokio::time::timeout(Duration::from_secs(30), renderer.render(url.as_str())).await {
            Ok(Ok(rendered_html)) => {
                *body_text = rendered_html;
                *parsed = crate::HtmlParser::parse(body_text, url);
            }
            Ok(Err(e)) => tracing::warn!("JS render failed for {}: {}", url, e),
            Err(_) => tracing::warn!("JS render timed out for {}", url),
        }
    }

    /// Run all analyzers against the parsed page.
    #[must_use]
    fn analyze(
        &self,
        parsed: &crate::ParsedPage,
        body_text: &str,
        result: &crate::FetchResult,
        fetched: &FetchedPage,
    ) -> (Vec<crate::Finding>, Duration) {
        let headers_vec: Vec<(String, String)> = result.headers.clone();
        let empty_chain: Vec<RedirectHop> = Vec::new();
        let robots_ref = if fetched.robots_raw.is_empty() {
            None
        } else {
            Some(fetched.robots_raw.as_str())
        };
        let ctx = crate::analyzers::AnalysisContext {
            page: parsed,
            body: Some(body_text),
            status_code: Some(result.status_code),
            headers: &headers_vec,
            response_time: Some(fetched.fetch_time),
            redirect_chain: &empty_chain,
            robots_txt: robots_ref,
        };
        let analysis_start = std::time::Instant::now();
        let findings = self.analyzer_registry.analyze(&ctx);
        (findings, analysis_start.elapsed())
    }

    /// Assemble the storage record for a crawled page.
    #[must_use]
    fn build_page_data(
        &self,
        page_id: &str,
        url: &Url,
        result: &crate::FetchResult,
        parsed: &crate::ParsedPage,
    ) -> PageData {
        let mut page_data = PageData {
            id: page_id.to_string(),
            url: url.clone(),
            final_url: result.final_url.clone(),
            status_code: result.status_code,
            title: parsed.meta.title.clone(),
            description: parsed.meta.description.clone(),
            canonical_url: parsed.meta.canonical.clone(),
            word_count: Some(parsed.word_count),
            load_time_ms: None,
            body_size: Some(result.body.len()),
            fetched_at: Utc::now(),
            links: parsed
                .links
                .iter()
                .filter_map(|l| Url::parse(&l.href).ok())
                .collect(),
            tenant_id: self.cfg.tenant_id.clone(),
            etag: result.etag.clone(),
            last_modified: result.last_modified.clone(),
            cwv_lcp: None,
            cwv_cls: None,
            cwv_inp: None,
        };
        if let Some(encryption) = self.cfg.encryption.as_ref() {
            if encryption.is_enabled() {
                page_data.title = encrypt_field(encryption, page_data.title.take());
                page_data.description = encrypt_field(encryption, page_data.description.take());
            }
        }
        page_data
    }

    /// Persist the page and its findings (batched in one transaction) on the
    /// blocking pool so SQLite writes never stall the async runtime.
    async fn store(&self, page_data: &PageData, findings: &[crate::Finding]) {
        let issues: Vec<Issue> = findings
            .iter()
            .map(|finding| Issue {
                id: uuid::Uuid::new_v4().to_string(),
                page_id: page_data.id.clone(),
                category: finding.category.clone(),
                severity: finding.severity.clone(),
                code: finding.code.clone(),
                title: finding.title.clone(),
                description: finding.description.clone(),
                element: None,
                recommendation: finding.recommendation.clone(),
                tenant_id: self.cfg.tenant_id.clone(),
            })
            .collect();

        let storage = Arc::clone(&self.storage);
        let crawl_id = self.crawl_id.clone();
        let page = page_data.clone();
        let result = tokio::task::spawn_blocking(move || {
            let page_stored = storage
                .insert_page(&crawl_id, &page)
                .map(|()| page.url.clone());
            let issues_stored = storage.insert_issues(&issues);
            (page_stored, issues_stored)
        })
        .await;

        match result {
            Ok((Ok(_), Ok(()))) => bump(&self.counters.pages_stored),
            Ok((page_res, issue_res)) => {
                if let Err(e) = page_res {
                    tracing::warn!("Failed to store page {}: {}", page_data.url, e);
                }
                if let Err(e) = issue_res {
                    tracing::warn!("Failed to store issues: {}", e);
                }
            }
            Err(e) => tracing::warn!(error = %e, "Blocking store task join failed"),
        }
    }

    /// Apply scope/pattern filters and enqueue newly discovered links.
    async fn extract_links(&self, parsed: &crate::ParsedPage, depth: usize) {
        let max_depth = self.cfg.crawl_config.max_depth;
        let queue = self.queue.lock().await;
        for link in &parsed.links {
            let link_url = match Url::parse(&link.href) {
                Ok(u) => u,
                Err(_) => continue,
            };
            if self.visited.contains(&link_url.to_string()) {
                continue;
            }
            let is_internal = link_url.host_str() == Some(self.seed_domain.as_str());

            if !is_internal && !self.cfg.allow_external {
                bump(&self.counters.skipped_external);
                continue;
            }
            if !self.cfg.include_patterns.is_empty()
                && !self
                    .cfg
                    .include_patterns
                    .iter()
                    .any(|p| link.href.contains(p.as_str()))
            {
                continue;
            }
            if self
                .cfg
                .exclude_patterns
                .iter()
                .any(|p| link.href.contains(p.as_str()))
            {
                continue;
            }
            if let Some(max) = max_depth {
                if depth + 1 > max {
                    continue;
                }
            }

            let priority = if is_internal {
                Priority::NORMAL
            } else {
                Priority::LOW
            };
            queue.push(link_url, depth + 1, priority);
        }
    }
}

/// Encrypt one field value for storage, hex-encoded and `enc:`-prefixed.
/// Falls back to the plaintext value if encryption fails.
fn encrypt_field(encryption: &EncryptionManager, value: Option<String>) -> Option<String> {
    value.map(|v| {
        encryption
            .encrypt(v.as_bytes())
            .map(|enc| {
                format!(
                    "enc:{}",
                    enc.iter().map(|b| format!("{b:02x}")).collect::<String>()
                )
            })
            .unwrap_or(v)
    })
}

/// Execute a single fetch, applying conditional-request logic when the crawl
/// is incremental. Owns all state needed by the worker task.
async fn execute_fetch(
    client: Arc<HttpClient>,
    storage: Arc<Storage>,
    crawl_id: String,
    entry: crate::queue::QueueEntry,
    incremental: bool,
    force: bool,
) -> FetchOutcome {
    if !incremental || force {
        return match client.fetch(&entry.url).await {
            Ok(result) => FetchOutcome::Fetched {
                result,
                freshness: Freshness::Unconditional,
            },
            Err(e) => FetchOutcome::Failed(e),
        };
    }

    // Look up cached validators on the blocking pool so worker threads never
    // stall on SQLite reads.
    let url_string = entry.url.to_string();
    let (previous, cross_previous) = tokio::task::spawn_blocking(move || {
        let previous = storage
            .get_page_conditional(&crawl_id, &url_string)
            .ok()
            .flatten();
        let cross_previous = storage.get_latest_conditional(&url_string).ok().flatten();
        (previous, cross_previous)
    })
    .await
    .unwrap_or((None, None));

    // Prefer the same-crawl record (it carries the page_id needed for 304
    // updates); fall back to the cross-crawl record for headers only.
    let (existing_etag, existing_lm) = if let Some((_, ref etag, ref lm)) = previous {
        (etag.as_deref(), lm.as_deref())
    } else if let Some((ref etag, ref lm)) = cross_previous {
        (etag.as_deref(), lm.as_deref())
    } else {
        (None, None)
    };

    match client
        .fetch_conditional(&entry.url, existing_etag, existing_lm)
        .await
    {
        Ok(r) if r.status_code == 304 => FetchOutcome::NotModified {
            page_id: previous.map(|(id, _, _)| id),
        },
        Ok(r) => FetchOutcome::Fetched {
            result: r,
            freshness: if previous.is_some() {
                Freshness::Modified
            } else {
                Freshness::New
            },
        },
        Err(e) => FetchOutcome::Failed(e),
    }
}

/// The crawl engine encapsulates the core crawl loop shared between
/// CLI and API consumers. It handles queue management, robots.txt,
/// rate limiting, circuit breaking, backpressure, HTTP fetching,
/// content-hash dedup, HTML parsing, analyzer execution, storage
/// writes, link extraction, and metrics recording.
///
/// # Examples
///
/// ```rust,no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
/// use crawlkit_engine::storage::Storage;
/// use std::path::Path;
///
/// let config = CrawlEngineConfig::default();
/// let storage = Storage::new(Path::new("crawl.db"))?;
/// let engine = CrawlEngine::new(config, storage);
/// let result = engine.run("https://example.com").await?;
/// println!("Crawled {} pages", result.pages_crawled);
/// # Ok(())
/// # }
/// ```
pub struct CrawlEngine {
    config: CrawlEngineConfig,
    storage: Arc<Storage>,
}

impl CrawlEngine {
    /// Create a new crawl engine.
    ///
    /// The engine takes ownership of the configuration and a reference-counted
    /// storage handle. All internal components (HTTP client, queue, caches)
    /// are created internally when [`run`](Self::run) is called.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
    /// use crawlkit_engine::storage::Storage;
    ///
    /// let storage = Storage::new_in_memory().unwrap();
    /// let engine = CrawlEngine::new(CrawlEngineConfig::default(), storage);
    /// ```
    pub fn new(config: CrawlEngineConfig, storage: Storage) -> Self {
        Self {
            config,
            storage: Arc::new(storage),
        }
    }

    /// Create a new crawl engine from an Arc-wrapped storage, sharing ownership.
    pub fn new_shared(config: CrawlEngineConfig, storage: Arc<Storage>) -> Self {
        Self { config, storage }
    }

    /// Create a new crawl engine, returning an error if storage creation fails.
    pub fn try_new(config: CrawlEngineConfig, storage: Storage) -> Result<Self, crate::CrawlError> {
        Ok(Self::new(config, storage))
    }

    /// Run the crawl starting from the given URL.
    ///
    /// This is the main entry point. It performs the full crawl loop:
    /// queue seeding, page fetching, parsing, analysis, storage, and
    /// link extraction.
    ///
    /// # Arguments
    ///
    /// * `start_url` - The URL to begin crawling from.
    ///
    /// # Returns
    ///
    /// A [`CrawlOutput`] with crawl statistics, or an error if the
    /// crawl could not be started.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
    /// use crawlkit_engine::storage::Storage;
    ///
    /// let storage = Storage::new_in_memory()?;
    /// let engine = CrawlEngine::new(CrawlEngineConfig::default(), storage);
    /// let result = engine.run("https://example.com").await?;
    /// println!("Crawled {} pages, {} issues", result.pages_crawled, result.issues_found);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(&self, start_url: &str) -> Result<CrawlOutput, crate::CrawlError> {
        self.run_with_callback(start_url, None).await
    }

    /// Run the crawl with an optional per-page callback.
    ///
    /// The callback is invoked after each page is successfully stored,
    /// receiving `(page_url, page_id, findings_count)`.
    pub async fn run_with_callback(
        &self,
        start_url: &str,
        on_page: Option<OnPageCrawled>,
    ) -> Result<CrawlOutput, crate::CrawlError> {
        let cfg = &self.config;
        let max_pages = cfg.crawl_config.max_pages;
        let seed_url = Url::parse(start_url)?;
        let seed_domain = seed_url.host_str().unwrap_or("").to_string();

        tracing::info!(
            "Starting crawl of {} (max_pages={}, concurrency={})",
            start_url,
            max_pages,
            cfg.concurrency.unwrap_or(cfg.crawl_config.concurrency),
        );

        // Start crawl in storage. Off the async runtime: this is a
        // synchronous SQLite write (with fsync) under a Mutex.
        let storage_for_start = Arc::clone(&self.storage);
        let start_url_owned = start_url.to_string();
        let crawl_id = tokio::task::spawn_blocking(move || {
            storage_for_start.start_crawl(&start_url_owned, None)
        })
        .await
        .map_err(|e| crate::CrawlError::Internal(format!("storage task panicked: {e}")))??;

        // Initialize components
        let concurrency = cfg.concurrency.unwrap_or(cfg.crawl_config.concurrency);
        let mut http_config = HttpClientConfig::from(&cfg.crawl_config);
        http_config.allow_http = cfg.allow_http;
        let http_client = HttpClient::new(http_config)?;
        let http_client = Arc::new(http_client);

        let robots_cache = Arc::new(RobotsTxtCache::new(http_client.clone(), &cfg.crawl_config));
        let sitemap_cache = Arc::new(SitemapCache::new(http_client.clone()));

        let metrics = Metrics::new();
        let resource_monitor = ResourceMonitor::with_default_limits();
        let circuit_breaker_registry = CircuitBreakerRegistry::with_default_config();

        let determinism = cfg.seed.map(DeterminismController::new);

        let scope = crate::queue::ScopeConfig {
            max_depth: cfg.crawl_config.max_depth,
            ..Default::default()
        };
        let queue = Arc::new(Mutex::new(UrlQueue::new(scope)));

        let rate_limiter = RateLimiter::new(
            concurrency as f64,
            1.0 / (cfg.crawl_config.request_delay.as_millis() as f64 / 1000.0),
        );

        // Build analyzer registry
        let analyzer_registry = self.build_analyzer_registry();

        // Seed, incremental history, and sitemap URLs into the queue.
        let mut known_sitemap_urls = self
            .prefill_queue(&queue, &seed_url, &crawl_id, &robots_cache, &sitemap_cache)
            .await;

        let run = CrawlRun {
            counters: CrawlCounters::default(),
            visited: DashSet::new(),
            content_hashes: if determinism.is_some() {
                ContentHashes::deterministic()
            } else {
                ContentHashes::sha256()
            },
            analyzer_registry: &analyzer_registry,
            cfg,
            storage: Arc::clone(&self.storage),
            crawl_id: crawl_id.clone(),
            seed_domain: seed_domain.clone(),
            on_page,
            metrics,
            resource_monitor,
            queue: queue.clone(),
        };
        let crawl_start = std::time::Instant::now();

        // Parallel fetch pipeline: worker tasks deliver FetchedPage values
        // that the main loop drains and processes sequentially.
        let fetch_semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
        let mut in_flight: FuturesUnordered<tokio::task::JoinHandle<Option<FetchedPage>>> =
            FuturesUnordered::new();

        // Main crawl loop: dispatch URLs and process completed fetches.
        loop {
            // Check time budget
            if let Some(max_time) = cfg.crawl_config.max_time {
                if crawl_start.elapsed() >= max_time {
                    tracing::info!("Crawl time limit reached: {max_time:?}");
                    break;
                }
            }

            let crawled = current(&run.counters.pages_crawled);
            if crawled >= max_pages {
                break;
            }

            // Check resource limits (every 100 pages)
            if crawled.is_multiple_of(100) && crawled > 0 {
                if let Ok(rss_bytes) = get_process_rss_bytes() {
                    let usage = crate::ResourceUsage {
                        memory_bytes: rss_bytes,
                        pages_processed: crawled,
                        elapsed: crawl_start.elapsed(),
                        ..Default::default()
                    };
                    run.resource_monitor.update(usage);
                    let exceeded = run.resource_monitor.exceeded_limits();
                    if !exceeded.is_empty() {
                        tracing::warn!("Resource limits exceeded: {:?}", exceeded);
                        run.metrics.record_resource_limit_hit();
                        break;
                    }
                }
            }

            // Dispatch new fetches while we have capacity, URLs, and page
            // budget. Counting in-flight fetches against the budget keeps the
            // parallel pipeline from overshooting max_pages.
            while in_flight.len() < concurrency
                && current(&run.counters.pages_crawled) + in_flight.len() < max_pages
            {
                let entry = {
                    let q = queue.lock().await;
                    q.pop()
                };

                let entry = match entry {
                    Some(e) => e,
                    None => break,
                };

                if run.visited.contains(&entry.url.to_string()) {
                    continue;
                }
                run.visited.insert(entry.url.to_string());

                // Robots.txt check (sequential, fast). The cache is keyed by
                // authority (host:port) so origins on non-standard ports get
                // their own robots.txt instead of the default port's.
                let robots_raw;
                if cfg.crawl_config.respect_robots_txt {
                    let domain = authority_of(&entry.url);
                    let scheme = entry.url.scheme();
                    if robots_cache
                        .is_disallowed(scheme, &domain, entry.url.path())
                        .await
                    {
                        tracing::debug!("Blocked by robots.txt: {}", entry.url);
                        bump(&run.counters.skipped_robots);
                        run.metrics.record_page_skipped_robots();
                        continue;
                    }
                    if let Some(delay_secs) = robots_cache.crawl_delay(scheme, &domain).await {
                        rate_limiter.set_crawl_delay(&domain, Duration::from_secs_f64(delay_secs));
                    }
                    robots_raw = robots_cache.raw_content(scheme, &domain).await;

                    // Discover sitemaps for this origin (first visit only)
                    if !domain.is_empty()
                        && !known_sitemap_urls.contains(&format!("{scheme}://{domain}"))
                    {
                        known_sitemap_urls.insert(format!("{scheme}://{domain}"));
                        let sitemap_urls = robots_cache.sitemaps(scheme, &domain).await;
                        if !sitemap_urls.is_empty() {
                            let entries = sitemap_cache.fetch_all(&sitemap_urls).await;
                            let q = queue.lock().await;
                            for sm_entry in &entries {
                                if let Ok(url) = Url::parse(&sm_entry.url) {
                                    q.push(url, 0, Priority::HIGHEST);
                                }
                            }
                        }
                    }
                } else {
                    robots_raw = String::new();
                }

                // Rate limit
                let _ = rate_limiter
                    .acquire(entry.url.host_str().unwrap_or(""))
                    .await;

                // Circuit breaker check
                let domain = entry.url.host_str().unwrap_or("");
                let breaker = circuit_breaker_registry.get_or_create(domain);
                if !breaker.is_allowed() {
                    tracing::debug!("Circuit breaker open for domain: {}", domain);
                    run.metrics.record_page_skipped_circuit_breaker();
                    continue;
                }

                // Acquire concurrency permit (owned, movable into the task)
                let Ok(permit) = fetch_semaphore.clone().acquire_owned().await else {
                    tracing::warn!("Fetch semaphore closed");
                    continue;
                };

                // Spawn fetch task; the permit is released when it completes.
                let client = http_client.clone();
                let storage = self.storage.clone();
                let crawl_id_clone = crawl_id.clone();
                let incremental = cfg.incremental;
                let force = cfg.force;

                let handle = tokio::spawn(async move {
                    let fetch_start = std::time::Instant::now();
                    let outcome = execute_fetch(
                        client,
                        storage,
                        crawl_id_clone,
                        entry.clone(),
                        incremental,
                        force,
                    )
                    .await;
                    let fetch_time = fetch_start.elapsed();
                    drop(permit);
                    Some(FetchedPage {
                        entry,
                        robots_raw,
                        fetch_time,
                        outcome,
                    })
                });
                in_flight.push(handle);
            }

            // If no in-flight tasks and the queue is exhausted, we're done
            if in_flight.is_empty() {
                break;
            }

            // Wait for the next completed fetch and process it
            if let Some(Ok(Some(fetched))) = in_flight.next().await {
                run.process(&fetched).await;
            }
        }

        // Drain remaining in-flight fetches
        while let Some(Ok(Some(fetched))) = in_flight.next().await {
            run.process(&fetched).await;
        }

        Ok(self.finish_and_report(&run, crawl_start).await)
    }

    /// Prefill the crawl queue before the dispatch loop runs.
    ///
    /// Seeds the start URL, pulls the previous crawl's page set in
    /// incremental mode (so pages reachable only via link extraction are
    /// still revalidated), and queues URLs from robots.txt-declared
    /// sitemaps for the seed domain.
    ///
    /// Returns the set of origins whose sitemaps were already consumed so
    /// the dispatch loop fetches each origin's sitemaps at most once.
    async fn prefill_queue(
        &self,
        queue: &Arc<Mutex<UrlQueue>>,
        seed_url: &Url,
        crawl_id: &str,
        robots_cache: &Arc<RobotsTxtCache>,
        sitemap_cache: &Arc<SitemapCache>,
    ) -> HashSet<String> {
        let cfg = &self.config;

        // Seed the queue
        queue.lock().await.push(seed_url.clone(), 0, Priority::HIGH);

        // Incremental mode: seed from the previous crawl's page set so pages
        // that are reachable only through link extraction still get
        // revalidated. A 304 on the seed skips re-parsing (and therefore
        // link discovery), which would otherwise strand the rest of the site.
        if cfg.incremental && !cfg.force {
            // Storage lookups run on the blocking pool; failures degrade
            // gracefully to a fresh crawl.
            let storage_for_lookup = Arc::clone(&self.storage);
            let crawl_id_for_lookup = crawl_id.to_string();
            let previous = tokio::task::spawn_blocking(move || {
                storage_for_lookup
                    .get_previous_crawl_id(&crawl_id_for_lookup)
                    .ok()
                    .flatten()
                    .map(|previous_crawl| {
                        let urls = storage_for_lookup
                            .get_page_urls(&previous_crawl)
                            .unwrap_or_default();
                        (previous_crawl, urls)
                    })
            })
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Incremental seed task failed");
                None
            });

            if let Some((previous_crawl, prev_urls)) = previous {
                if !prev_urls.is_empty() {
                    let count = prev_urls.len();
                    {
                        let q = queue.lock().await;
                        for url_str in &prev_urls {
                            if let Ok(url) = Url::parse(url_str) {
                                q.push(url, 0, Priority::HIGHEST);
                            }
                        }
                    }
                    tracing::info!(
                        "Incremental: seeded {count} URLs from previous crawl {previous_crawl}"
                    );
                }
            }
        }

        // Discover and queue sitemap URLs for the seed domain
        let mut known_sitemap_urls: HashSet<String> = HashSet::new();
        {
            let sitemap_urls = robots_cache
                .sitemaps(seed_url.scheme(), &authority_of(seed_url))
                .await;
            if !sitemap_urls.is_empty() {
                tracing::info!("Found {} sitemap URLs in robots.txt", sitemap_urls.len());
                let entries = sitemap_cache.fetch_all(&sitemap_urls).await;
                tracing::info!("Parsed {} URLs from sitemaps", entries.len());
                let q = queue.lock().await;
                for entry in &entries {
                    if known_sitemap_urls.insert(entry.url.clone()) {
                        if let Ok(url) = Url::parse(&entry.url) {
                            q.push(url, 0, Priority::HIGHEST);
                        }
                    }
                }
            }
        }
        known_sitemap_urls
    }

    /// Persist crawl completion and assemble the output report.
    ///
    /// Runs the final storage write on the blocking pool (SQLite write with
    /// fsync), emits the completion summary, and snapshots metrics.
    async fn finish_and_report<'a>(
        &self,
        run: &CrawlRun<'a>,
        crawl_start: std::time::Instant,
    ) -> CrawlOutput {
        let crawl_id = run.crawl_id.clone();
        let stats = run.counters.snapshot();
        let storage_for_finish = Arc::clone(&self.storage);
        let crawl_id_for_finish = crawl_id.clone();
        let finish_result = tokio::task::spawn_blocking(move || {
            storage_for_finish.finish_crawl(
                &crawl_id_for_finish,
                stats.pages_crawled,
                stats.issues_found,
            )
        })
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, crawl_id = %crawl_id, "Storage finish task failed");
            Ok(())
        });
        if let Err(e) = finish_result {
            tracing::warn!(error = %e, crawl_id = %crawl_id, "Failed to finish crawl in storage");
        }

        let elapsed = crawl_start.elapsed();
        let snapshot = run.metrics.snapshot();

        tracing::info!(
            "Crawl complete: {} pages crawled, {} stored, {} issues, {} external skipped, {} robots blocked, {} duplicates, {} unchanged, {} modified, {} new",
            stats.pages_crawled,
            stats.pages_stored,
            stats.issues_found,
            stats.skipped_external,
            stats.skipped_robots,
            stats.skipped_duplicate,
            stats.pages_unchanged,
            stats.pages_modified,
            stats.pages_new,
        );

        CrawlOutput {
            crawl_id,
            pages_crawled: stats.pages_crawled,
            pages_stored: stats.pages_stored,
            issues_found: stats.issues_found,
            skipped_external: stats.skipped_external,
            skipped_robots: stats.skipped_robots,
            skipped_duplicate: stats.skipped_duplicate,
            pages_unchanged: stats.pages_unchanged,
            pages_modified: stats.pages_modified,
            pages_new: stats.pages_new,
            seed_domain: run.seed_domain.clone(),
            metrics: snapshot,
            elapsed,
        }
    }

    /// Build the analyzer registry based on feature flags.
    ///
    /// Delegates to [`AnalyzerRegistry::with_feature_flags`], the single
    /// registration site for the built-in analyzer set (also used by
    /// [`AnalyzerRegistry::new`]). The `ai_analyzers` and `wasm_analyzers`
    /// feature flags control the optional analyzer groups.
    fn build_analyzer_registry(&self) -> AnalyzerRegistry {
        AnalyzerRegistry::with_feature_flags(&self.config.feature_flags)
    }

    /// Get a reference to the underlying storage.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }
}

/// Get the current process RSS (Resident Set Size) in bytes.
///
/// Uses `/proc/self/statm` on Linux, returns `Err` on unsupported platforms.
/// Host plus explicit port when the URL carries a non-default one
/// (`127.0.0.1:8080`). Used as the robots.txt cache key and fetch origin so
/// origins on non-standard ports are not conflated with the default port.
fn authority_of(url: &Url) -> String {
    let host = url.host_str().unwrap_or_default();
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    }
}

fn get_process_rss_bytes() -> Result<u64, crate::CrawlError> {
    #[cfg(target_os = "linux")]
    {
        let statm = std::fs::read_to_string("/proc/self/statm")
            .map_err(|e| crate::CrawlError::Internal(format!("failed to read statm: {e}")))?;
        let fields: Vec<&str> = statm.split_whitespace().collect();
        if fields.len() >= 2 {
            let pages: u64 = fields[1]
                .parse()
                .map_err(|e| crate::CrawlError::Internal(format!("failed to parse RSS: {e}")))?;
            let page_size = 4096u64;
            return Ok(pages * page_size);
        }
    }
    Err(crate::CrawlError::Internal(
        "RSS not available on this platform".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    #[test]
    fn test_content_hashes_sha256_detects_duplicates() {
        let hashes = ContentHashes::sha256();
        assert!(hashes.insert("<html>unique body</html>"));
        assert!(!hashes.insert("<html>unique body</html>"));
        assert!(hashes.insert("<html>different body</html>"));
    }

    #[test]
    fn test_content_hashes_deterministic_detects_duplicates() {
        let hashes = ContentHashes::deterministic();
        assert!(hashes.insert("page one"));
        assert!(!hashes.insert("page one"));
        assert!(hashes.insert("page two"));
    }

    #[test]
    fn test_crawl_counters_snapshot_reflects_updates() {
        let counters = CrawlCounters::default();
        bump(&counters.pages_crawled);
        bump(&counters.pages_crawled);
        bump_by(&counters.issues_found, 7);
        let snap = counters.snapshot();
        assert_eq!(snap.pages_crawled, 2);
        assert_eq!(snap.issues_found, 7);
        assert_eq!(snap.pages_stored, 0);
    }

    #[test]
    fn test_encrypt_field_roundtrip() {
        // encrypt_field with an enabled-but-uninitialized manager falls back
        // to plaintext (encrypt fails on missing key). Full crypto roundtrip
        // is covered by encryption.rs tests.
        use crate::encryption::{EncryptionAlgorithm, EncryptionConfig, KeySource};
        let manager = EncryptionManager::new(EncryptionConfig {
            enabled: true,
            key_source: KeySource::EnvVar("TEST_CRAWL_KEY_MISSING".to_string()),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
        });
        assert!(!manager.is_initialized());
        assert_eq!(
            encrypt_field(&manager, Some("plain".to_string())),
            Some("plain".to_string())
        );
    }

    #[test]
    fn test_freshness_counters_dispatch() {
        // Sanity: enum variants are constructible and distinct via debug output.
        let variants = [
            Freshness::New,
            Freshness::Modified,
            Freshness::Unconditional,
        ];
        let rendered: Vec<String> = variants.iter().map(|f| format!("{f:?}")).collect();
        assert_eq!(rendered.len(), 3);
        assert_ne!(rendered[0], rendered[1]);
        assert_ne!(rendered[1], rendered[2]);
    }

    #[test]
    fn test_crawl_engine_config_default() {
        let config = CrawlEngineConfig::default();
        assert!(!config.enable_js_rendering);
        assert!(!config.allow_external);
        assert!(config.include_patterns.is_empty());
        assert!(config.exclude_patterns.is_empty());
        assert!(config.seed.is_none());
        assert!(config.tenant_id.is_none());
    }

    #[test]
    fn test_crawl_output_clone() {
        let output = CrawlOutput {
            crawl_id: "test".to_string(),
            pages_crawled: 10,
            pages_stored: 8,
            issues_found: 5,
            skipped_external: 2,
            skipped_robots: 1,
            skipped_duplicate: 3,
            pages_unchanged: 4,
            pages_modified: 3,
            pages_new: 3,
            seed_domain: "example.com".to_string(),
            metrics: crate::Metrics::new().snapshot(),
            elapsed: Duration::from_secs(1),
        };
        let cloned = output;
        assert_eq!(cloned.crawl_id, "test");
        assert_eq!(cloned.pages_crawled, 10);
    }

    #[tokio::test]
    async fn test_crawl_engine_new() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let config = CrawlEngineConfig::default();
        let engine = CrawlEngine::new(config, storage);
        assert_eq!(
            engine
                .storage()
                .get_stats("nonexistent")
                .unwrap()
                .total_pages,
            0
        );
    }

    #[test]
    fn test_build_analyzer_registry_parity_with_default_registry() {
        let storage = Storage::new_in_memory().unwrap();
        let engine = CrawlEngine::new(CrawlEngineConfig::default(), storage);
        let registry = engine.build_analyzer_registry();
        // The engine must register exactly the same analyzer set as
        // AnalyzerRegistry::new (single registration site), including the
        // advanced canonical analyzers the duplicated list used to omit.
        assert_eq!(
            registry.len(),
            AnalyzerRegistry::new(&CrawlConfig::default()).len()
        );
        assert_eq!(registry.len(), 31);
    }

    #[test]
    fn test_build_analyzer_registry_respects_feature_flags() {
        let storage = Storage::new_in_memory().unwrap();
        let mut config = CrawlEngineConfig::default();
        config.feature_flags.set(crate::FLAG_AI_ANALYZERS, false);
        config.feature_flags.set(crate::FLAG_WASM_ANALYZERS, false);
        let engine = CrawlEngine::new(config, storage);
        let registry = engine.build_analyzer_registry();
        // 26 base analyzers; the AI (4) and WASM (1) groups are disabled.
        assert_eq!(registry.len(), 26);
    }

    struct MockJsRenderer {
        available: bool,
        rendered_url: std::sync::Mutex<Option<String>>,
    }

    impl MockJsRenderer {
        fn new(available: bool) -> Self {
            Self {
                available,
                rendered_url: std::sync::Mutex::new(None),
            }
        }

        fn rendered_url(&self) -> Option<String> {
            self.rendered_url.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl JsRenderer for MockJsRenderer {
        fn is_available(&self) -> bool {
            self.available
        }

        async fn render(&self, url: &str) -> Result<String, String> {
            *self.rendered_url.lock().unwrap() = Some(url.to_string());
            Ok(format!(
                r#"<html><head><title>Rendered</title></head><body><div id="app">JS content for {url}</div></body></html>"#
            ))
        }
    }

    #[test]
    fn test_mock_renderer_implements_js_renderer_trait() {
        let renderer = MockJsRenderer::new(true);
        assert!(renderer.is_available());
        assert!(renderer.rendered_url().is_none());

        let renderer_unavailable = MockJsRenderer::new(false);
        assert!(!renderer_unavailable.is_available());
    }

    #[test]
    fn test_crawl_engine_config_with_js_renderer() {
        let renderer: std::sync::Arc<dyn JsRenderer> =
            std::sync::Arc::new(MockJsRenderer::new(true));
        let config = CrawlEngineConfig {
            enable_js_rendering: true,
            js_renderer: Some(renderer),
            ..Default::default()
        };
        assert!(config.enable_js_rendering);
        assert!(config.js_renderer.is_some());
        assert!(config.js_renderer.as_ref().unwrap().is_available());
    }

    #[test]
    fn test_crawl_engine_config_without_js_renderer() {
        let config = CrawlEngineConfig {
            enable_js_rendering: false,
            js_renderer: None,
            ..Default::default()
        };
        assert!(!config.enable_js_rendering);
        assert!(config.js_renderer.is_none());
    }

    #[tokio::test]
    async fn test_mock_renderer_render() {
        let renderer = MockJsRenderer::new(true);
        let result = renderer.render("https://example.com/page").await;
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("Rendered"));
        assert!(html.contains("https://example.com/page"));
        assert_eq!(
            renderer.rendered_url(),
            Some("https://example.com/page".to_string())
        );
    }

    #[test]
    fn test_js_render_decision_engine_with_renderer_config() {
        let decision_engine = crate::JsRenderDecisionEngine::new();
        let html = r#"<div id="__next">Hello</div>"#;
        let decision = decision_engine.should_render_js("https://example.com/page", Some(html));
        assert!(matches!(decision, crate::JsRenderDecision::Render { .. }));
    }
}
