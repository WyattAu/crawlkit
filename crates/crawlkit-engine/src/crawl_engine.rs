use crate::analyzers::AnalyzerRegistry;
use crate::encryption::EncryptionManager;
use crate::http::HttpClient;
use crate::queue::{Priority, UrlQueue};
use crate::ratelimit::RateLimiter;
use crate::robots::RobotsTxtCache;
use crate::sitemap::SitemapCache;
use crate::storage::{Issue, PageData, Storage};
use crate::{
    BackpressureController, CircuitBreakerRegistry, CrawlConfig, DeterminismController,
    FeatureFlags, Metrics, RedirectHop, ResourceMonitor,
};
use chrono::Utc;
use rusqlite::params;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;

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

        // Start crawl in storage
        let crawl_id = self
            .storage
            .start_crawl(start_url, None)
            .map_err(|e| crate::CrawlError::Storage(e.to_string()))?;

        // Initialize components
        let concurrency = cfg.concurrency.unwrap_or(cfg.crawl_config.concurrency);
        let http_client = HttpClient::from_crawl_config(&cfg.crawl_config)?;
        let http_client = Arc::new(http_client);

        let robots_cache = Arc::new(RobotsTxtCache::new(http_client.clone(), &cfg.crawl_config));
        let sitemap_cache = Arc::new(SitemapCache::new(http_client.clone()));

        let metrics = Metrics::new();
        let resource_monitor = ResourceMonitor::with_default_limits();
        let circuit_breaker_registry = CircuitBreakerRegistry::with_default_config();
        let backpressure = BackpressureController::new(concurrency);

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

        // Seed the queue
        {
            let q = queue.lock().await;
            q.push(seed_url.clone(), 0, Priority::HIGH);
        }

        // Discover and queue sitemap URLs for the seed domain
        let mut known_sitemap_urls: HashSet<String> = HashSet::new();
        {
            let sitemap_urls = robots_cache
                .sitemaps(seed_url.scheme(), seed_url.host_str().unwrap_or(""))
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

        let mut pages_crawled: usize = 0;
        let mut pages_stored: usize = 0;
        let mut issues_found: usize = 0;
        let mut skipped_external: usize = 0;
        let mut skipped_robots: usize = 0;
        let mut skipped_duplicate: usize = 0;
        let mut pages_unchanged: usize = 0;
        let mut pages_modified: usize = 0;
        let mut pages_new: usize = 0;
        let mut visited: HashSet<String> = HashSet::new();
        let mut content_hashes_string: HashSet<String> = HashSet::new();
        let mut content_hashes_u64: HashSet<u64> = HashSet::new();
        let use_deterministic_hash = determinism.is_some();
        let crawl_start = std::time::Instant::now();

        // Crawl loop
        while pages_crawled < max_pages {
            // Check time budget
            if let Some(max_time) = cfg.crawl_config.max_time {
                if crawl_start.elapsed() >= max_time {
                    tracing::info!("Crawl time limit reached: {max_time:?}");
                    break;
                }
            }

            // Check resource limits (every 100 pages)
            if pages_crawled.is_multiple_of(100) && pages_crawled > 0 {
                if let Ok(rss_bytes) = get_process_rss_bytes() {
                    let usage = crate::ResourceUsage {
                        memory_bytes: rss_bytes,
                        pages_processed: pages_crawled,
                        elapsed: crawl_start.elapsed(),
                        ..Default::default()
                    };
                    resource_monitor.update(usage);
                    let exceeded = resource_monitor.exceeded_limits();
                    if !exceeded.is_empty() {
                        tracing::warn!("Resource limits exceeded: {:?}", exceeded);
                        metrics.record_resource_limit_hit();
                        break;
                    }
                }
            }

            // Pop URL from queue
            let entry = {
                let q = queue.lock().await;
                q.pop()
            };

            let entry = match entry {
                Some(e) => e,
                None => break,
            };

            if visited.contains(&entry.url.to_string()) {
                continue;
            }
            visited.insert(entry.url.to_string());

            // Robots.txt check
            let robots_raw;
            if cfg.crawl_config.respect_robots_txt {
                let domain = entry.url.host_str().unwrap_or("");
                let scheme = entry.url.scheme();
                if robots_cache
                    .is_disallowed(scheme, domain, entry.url.path())
                    .await
                {
                    tracing::debug!("Blocked by robots.txt: {}", entry.url);
                    skipped_robots += 1;
                    continue;
                }
                if let Some(delay_secs) = robots_cache.crawl_delay(scheme, domain).await {
                    rate_limiter.set_crawl_delay(domain, Duration::from_secs_f64(delay_secs));
                }
                robots_raw = robots_cache.raw_content(scheme, domain).await;

                // Discover sitemaps for this domain (first visit only)
                if !domain.is_empty()
                    && !known_sitemap_urls.contains(&format!("{scheme}://{domain}"))
                {
                    known_sitemap_urls.insert(format!("{scheme}://{domain}"));
                    let sitemap_urls = robots_cache.sitemaps(scheme, domain).await;
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
                metrics.record_page_skipped_circuit_breaker();
                continue;
            }

            // Acquire backpressure permit
            let _permit = match backpressure.acquire().await {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Backpressure acquire failed: {}", e);
                    continue;
                }
            };

            // Fetch
            let start = std::time::Instant::now();
            let result = if cfg.incremental && !cfg.force {
                // Check for existing ETag/Last-Modified from a previous crawl
                let previous = match self
                    .storage
                    .get_page_conditional(&crawl_id, entry.url.as_str())
                {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(error = %e, url = %entry.url, "Failed to get page conditional");
                        None
                    }
                };

                let cross_previous = match self.storage.get_latest_conditional(entry.url.as_str()) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(error = %e, url = %entry.url, "Failed to get latest conditional");
                        None
                    }
                };

                // Prefer same-crawl result (has page_id for 304 updates),
                // fall back to cross-crawl for conditional headers only.
                let (existing_etag, existing_lm) = if let Some((_, ref etag, ref lm)) = previous {
                    (etag.as_deref(), lm.as_deref())
                } else if let Some((ref etag, ref lm)) = cross_previous {
                    (etag.as_deref(), lm.as_deref())
                } else {
                    (None, None)
                };

                match http_client
                    .fetch_conditional(&entry.url, existing_etag, existing_lm)
                    .await
                {
                    Ok(r) if r.status_code == 304 => {
                        // Not Modified — skip analysis but update access timestamp
                        pages_unchanged += 1;
                        if let Some((page_id, _, _)) = previous {
                            // Update the fetched_at timestamp for the existing page
                            let conn = self.storage.conn();
                            let _ = conn.execute(
                                "UPDATE pages SET fetched_at = ?1 WHERE id = ?2",
                                params![Utc::now().to_rfc3339(), page_id],
                            );
                        }
                        tracing::debug!("304 Not Modified: {}", entry.url);
                        continue;
                    }
                    Ok(r) => {
                        if previous.is_some() {
                            pages_modified += 1;
                        } else {
                            pages_new += 1;
                        }
                        breaker.record_success();
                        r
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch {}: {}", entry.url, e);
                        let was_allowed = breaker.is_allowed();
                        breaker.record_failure();
                        if was_allowed && !breaker.is_allowed() {
                            metrics.record_circuit_breaker_trip();
                        }
                        metrics.record_page_failure();
                        continue;
                    }
                }
            } else {
                match http_client.fetch(&entry.url).await {
                    Ok(r) => {
                        breaker.record_success();
                        r
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch {}: {}", entry.url, e);
                        let was_allowed = breaker.is_allowed();
                        breaker.record_failure();
                        if was_allowed && !breaker.is_allowed() {
                            metrics.record_circuit_breaker_trip();
                        }
                        metrics.record_page_failure();
                        continue;
                    }
                }
            };
            let fetch_time = start.elapsed();

            // Content-hash deduplication
            {
                let is_duplicate = if use_deterministic_hash {
                    let hash = DeterminismController::content_hash(&result.body);
                    !content_hashes_u64.insert(hash)
                } else {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(result.body.as_bytes());
                    let hash_result = hasher.finalize();
                    let hash: String = hash_result.iter().map(|b| format!("{b:02x}")).collect();
                    !content_hashes_string.insert(hash)
                };
                if is_duplicate {
                    tracing::debug!("Skipping duplicate content: {}", entry.url);
                    skipped_duplicate += 1;
                    continue;
                }
            }

            pages_crawled += 1;

            // Parse HTML
            let mut body_text = result.body.clone();
            let mut parsed = match crate::HtmlParser::parse(&body_text, &entry.url) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}", entry.url, e);
                    continue;
                }
            };

            // JS rendering: consult decision engine and render if needed
            if cfg.enable_js_rendering {
                let js_decision_engine = crate::JsRenderDecisionEngine::new();
                let decision =
                    js_decision_engine.should_render_js(entry.url.as_ref(), Some(&body_text));
                match decision {
                    crate::JsRenderDecision::Render { reason } => {
                        tracing::info!("JS render decision for {}: {}", entry.url, reason);
                        if let Some(ref renderer) = cfg.js_renderer {
                            if renderer.is_available() {
                                match tokio::time::timeout(
                                    Duration::from_secs(30),
                                    renderer.render(entry.url.as_str()),
                                )
                                .await
                                {
                                    Ok(Ok(rendered_html)) => {
                                        body_text = rendered_html;
                                        match crate::HtmlParser::parse(&body_text, &entry.url) {
                                            Ok(re_parsed) => parsed = re_parsed,
                                            Err(e) => {
                                                tracing::warn!(
                                                    "Failed to re-parse rendered {}: {}",
                                                    entry.url,
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Ok(Err(e)) => {
                                        tracing::warn!("JS render failed for {}: {}", entry.url, e);
                                    }
                                    Err(_) => {
                                        tracing::warn!("JS render timed out for {}", entry.url);
                                    }
                                }
                            } else {
                                tracing::warn!(
                                    "JS renderer not available, using static HTML: {}",
                                    entry.url
                                );
                            }
                        }
                    }
                    crate::JsRenderDecision::Skip { reason } => {
                        tracing::debug!("JS skip for {}: {}", entry.url, reason);
                    }
                }
            }

            // Run analyzers
            let headers_vec: Vec<(String, String)> = result.headers.clone();
            let empty_chain: Vec<RedirectHop> = vec![];
            let robots_ref = if robots_raw.is_empty() {
                None
            } else {
                Some(robots_raw.as_str())
            };
            let ctx = crate::analyzers::AnalysisContext {
                page: &parsed,
                body: Some(&body_text),
                status_code: Some(result.status_code),
                headers: &headers_vec,
                response_time: Some(fetch_time),
                redirect_chain: &empty_chain,
                robots_txt: robots_ref,
            };
            let analysis_start = std::time::Instant::now();
            let findings = analyzer_registry.analyze(&ctx, &cfg.crawl_config);
            let analysis_time = analysis_start.elapsed();

            issues_found += findings.len();

            // Store page
            let page_id = uuid::Uuid::new_v4().to_string();
            let mut page_data = PageData {
                id: page_id.clone(),
                url: entry.url.clone(),
                final_url: result.final_url.clone(),
                status_code: result.status_code,
                title: parsed.meta.title.clone(),
                description: parsed.meta.description.clone(),
                canonical_url: parsed.meta.canonical.clone(),
                word_count: Some(parsed.word_count),
                load_time_ms: Some(fetch_time.as_millis() as u64),
                body_size: Some(result.body.len()),
                fetched_at: Utc::now(),
                links: parsed
                    .links
                    .iter()
                    .filter_map(|l| Url::parse(&l.href).ok())
                    .collect(),
                tenant_id: cfg.tenant_id.clone(),
                etag: result.etag.clone(),
                last_modified: result.last_modified.clone(),
                cwv_lcp: None,
                cwv_cls: None,
                cwv_inp: None,
            };

            // Measure Core Web Vitals if JS rendering is enabled
            if cfg.enable_js_rendering {
                let measurer = crate::web_vitals::WebVitalsMeasurer::new();
                match measurer.measure(entry.url.as_ref()).await {
                    Ok(vitals) => {
                        page_data.cwv_lcp = vitals.lcp;
                        page_data.cwv_cls = vitals.cls;
                        page_data.cwv_inp = vitals.inp;
                        tracing::debug!(
                            url = %entry.url,
                            lcp = ?vitals.lcp,
                            cls = ?vitals.cls,
                            inp = ?vitals.inp,
                            "CWV measured"
                        );
                    }
                    Err(e) => {
                        tracing::debug!("CWV measurement skipped for {}: {}", entry.url, e);
                    }
                }
            }

            // Encrypt sensitive fields if encryption is enabled
            if let Some(ref encryption) = cfg.encryption {
                if encryption.is_enabled() {
                    if let Some(title) = page_data.title.take() {
                        if let Ok(encrypted) = encryption.encrypt(title.as_bytes()) {
                            page_data.title = Some(format!(
                                "enc:{}",
                                encrypted
                                    .iter()
                                    .map(|b| format!("{b:02x}"))
                                    .collect::<String>()
                            ));
                        } else {
                            page_data.title = Some(title);
                        }
                    }
                    if let Some(desc) = page_data.description.take() {
                        if let Ok(encrypted) = encryption.encrypt(desc.as_bytes()) {
                            page_data.description = Some(format!(
                                "enc:{}",
                                encrypted
                                    .iter()
                                    .map(|b| format!("{b:02x}"))
                                    .collect::<String>()
                            ));
                        } else {
                            page_data.description = Some(desc);
                        }
                    }
                }
            }

            if let Err(e) = self.storage.insert_page(&crawl_id, &page_data) {
                tracing::warn!("Failed to store page {}: {}", entry.url, e);
            } else {
                pages_stored += 1;
            }

            // Store findings
            for finding in &findings {
                let issue = Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    page_id: page_data.id.clone(),
                    category: finding.category.clone(),
                    severity: finding.severity.clone(),
                    code: finding.code.clone(),
                    title: finding.title.clone(),
                    description: finding.description.clone(),
                    element: None,
                    recommendation: finding.recommendation.clone(),
                    tenant_id: cfg.tenant_id.clone(),
                };
                if let Err(e) = self.storage.insert_issue(&issue) {
                    tracing::warn!("Failed to store issue: {}", e);
                }
            }

            // Record metrics
            metrics.record_page_success(
                result.body.len() as u64,
                fetch_time.as_micros() as u64,
                analysis_time.as_micros() as u64,
                0, // storage_time not tracked here
                findings.len() as u64,
            );
            resource_monitor.record_page();

            // Invoke callback
            if let Some(ref cb) = on_page {
                cb(entry.url.as_ref(), &page_id, findings.len());
            }

            // Extract and queue new links
            for link in &parsed.links {
                let link_url = match Url::parse(&link.href) {
                    Ok(u) => u,
                    Err(_) => continue,
                };
                if visited.contains(&link_url.to_string()) {
                    continue;
                }
                let is_internal = link_url.host_str() == Some(&seed_domain);

                // Enforce domain filtering
                if !is_internal && !cfg.allow_external {
                    skipped_external += 1;
                    continue;
                }

                if !cfg.include_patterns.is_empty()
                    && !cfg.include_patterns.iter().any(|p| link.href.contains(p))
                {
                    continue;
                }
                if cfg.exclude_patterns.iter().any(|p| link.href.contains(p)) {
                    continue;
                }

                let priority = if is_internal {
                    Priority::NORMAL
                } else {
                    Priority::LOW
                };

                // Enforce depth budget
                if let Some(max_depth) = cfg.crawl_config.max_depth {
                    if entry.depth + 1 > max_depth {
                        continue;
                    }
                }

                let q = queue.lock().await;
                q.push(link_url, entry.depth + 1, priority);
            }
        }

        // Finish crawl in storage
        if let Err(e) = self
            .storage
            .finish_crawl(&crawl_id, pages_crawled, issues_found)
        {
            tracing::warn!(error = %e, crawl_id = %crawl_id, "Failed to finish crawl in storage");
        }

        let elapsed = crawl_start.elapsed();
        let snapshot = metrics.snapshot();

        tracing::info!(
            "Crawl complete: {} pages crawled, {} stored, {} issues, {} external skipped, {} robots blocked, {} duplicates, {} unchanged, {} modified, {} new",
            pages_crawled,
            pages_stored,
            issues_found,
            skipped_external,
            skipped_robots,
            skipped_duplicate,
            pages_unchanged,
            pages_modified,
            pages_new,
        );

        Ok(CrawlOutput {
            crawl_id,
            pages_crawled,
            pages_stored,
            issues_found,
            skipped_external,
            skipped_robots,
            skipped_duplicate,
            pages_unchanged,
            pages_modified,
            pages_new,
            seed_domain,
            metrics: snapshot,
            elapsed,
        })
    }

    /// Build the analyzer registry based on feature flags.
    fn build_analyzer_registry(&self) -> AnalyzerRegistry {
        let flags = &self.config.feature_flags;

        let mut analyzers: Vec<Box<dyn crate::analyzers::Analyzer>> = vec![
            Box::new(crate::HttpStatusAnalyzer::new()),
            Box::new(crate::RedirectChainAnalyzer::new()),
            Box::new(crate::CanonicalUrlValidator::new()),
            Box::new(crate::HreflangValidator::new()),
            Box::new(crate::SitemapAnalyzer::empty()),
            Box::new(crate::RobotsTxtAnalyzer::empty()),
            Box::new(crate::MetaTagAnalyzer::new()),
            Box::new(crate::HeadingHierarchyAnalyzer::new()),
            Box::new(crate::LinkAnalyzer::new()),
            Box::new(crate::ImageAnalyzer::new()),
            Box::new(crate::StructuredDataValidator::new()),
            Box::new(crate::ContentQualityAnalyzer::new()),
            Box::new(crate::WordCountAnalyzer::new()),
            Box::new(crate::SecurityHeaderAnalyzer::new()),
            Box::new(crate::SslCertificateValidator::empty()),
            Box::new(crate::MobileFriendlinessChecker::new()),
            Box::new(crate::AccessibilityAnalyzer::new()),
            Box::new(crate::SocialMediaAnalyzer::new()),
            Box::new(crate::EntityAnalyzer::new()),
            Box::new(crate::EnhancedReadabilityAnalyzer::new()),
            Box::new(crate::KeywordAnalyzer::new()),
            Box::new(crate::EcommerceSignalsAnalyzer::new()),
            Box::new(crate::InternationalSeoAnalyzer::new()),
        ];

        if flags.get(crate::FLAG_AI_ANALYZERS) {
            analyzers.push(Box::new(crate::AiCrawlerAccessibilityAnalyzer::new()));
            analyzers.push(Box::new(crate::AiContentStructureAnalyzer::new()));
            analyzers.push(Box::new(crate::AiCitationEligibilityAnalyzer::new()));
            analyzers.push(Box::new(crate::AiAnswerBoxAnalyzer::new()));
        }

        if flags.get(crate::FLAG_WASM_ANALYZERS) {
            analyzers.push(Box::new(crate::WasmPatternAnalyzer::new()));
        }

        AnalyzerRegistry::with_analyzers(analyzers)
    }

    /// Get a reference to the underlying storage.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }
}

/// Get the current process RSS (Resident Set Size) in bytes.
///
/// Uses `/proc/self/statm` on Linux, returns `Err` on unsupported platforms.
fn get_process_rss_bytes() -> Result<u64, crate::CrawlError> {
    #[cfg(target_os = "linux")]
    {
        let statm = std::fs::read_to_string("/proc/self/statm")
            .map_err(|e| crate::CrawlError::Storage(e.to_string()))?;
        let fields: Vec<&str> = statm.split_whitespace().collect();
        if fields.len() >= 2 {
            let pages: u64 = fields[1]
                .parse()
                .map_err(|e| crate::CrawlError::Storage(format!("Failed to parse RSS: {e}")))?;
            let page_size = 4096u64;
            return Ok(pages * page_size);
        }
    }
    Err(crate::CrawlError::Storage(
        "RSS not available on this platform".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

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
