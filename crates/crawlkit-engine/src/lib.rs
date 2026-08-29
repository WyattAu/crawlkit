//! # crawlkit-engine
//!
//! Core library for crawlkit — a high-performance Rust web crawler for SEO analysis.
//!
//! ## Overview
//!
//! This crate provides the foundational types, HTTP fetching, HTML parsing,
//! SEO analyzers, crawl queue, storage, and observability primitives used by
//! the crawlkit CLI and API server.
//!
//! ## Features
//!
//! - **31 SEO analyzers** covering meta tags, content quality, security, accessibility
//! - **Async HTTP/2** fetching with retry, redirect tracking, rate limiting
//! - **HTML parsing** with link, heading, image, and structured data extraction
//! - **SQLite storage** with WAL mode and batch operations
//! - **Observability** with atomic metrics and OpenTelemetry support
//! - **Plugin system** with WASM sandboxing for third-party extensions
//! - **Encryption at rest** with AES-256-GCM
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use crawlkit_engine::{CrawlConfig, HttpClient, HtmlParser};
//! use crawlkit_engine::analyzers::AnalyzerRegistry;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = CrawlConfig::default();
//! let client = HttpClient::from_crawl_config(&config)?;
//! let registry = AnalyzerRegistry::new(&config);
//!
//! let url = url::Url::parse("https://example.com")?;
//! let result = client.fetch(&url).await?;
//! let parsed = HtmlParser::parse(&result.body, &url);
//! # Ok(())
//! # }
//! ```
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use url::Url;

/// Advanced canonical URL analysis and validation.
///
/// Detects canonical conflicts, duplicate content, and
/// provides recommendations for canonical URL best practices.
pub mod advanced_canonical;
/// Advanced crawl features such as JavaScript rendering and WASM analysis.
#[cfg(feature = "full")]
pub mod advanced_features;
/// AI-powered page content analyzers for answer boxes, citations, and crawler accessibility.
///
/// These analyzers detect AI-specific SEO opportunities and issues, such as
/// whether AI crawlers can access the site and whether content is structured
/// for AI extraction.
pub mod ai_analyzers;
/// Registry of known AI bot user-agents and crawler identification.
///
/// Contains a static list of AI crawlers (GPTBot, ClaudeBot, etc.) with metadata
/// for robots.txt analysis and AI accessibility scoring.
pub mod ai_bots;
/// SEO analysis engine with pluggable analyzers (title, meta, links, etc.).
///
/// Defines the Analyzer trait and provides 31 built-in analyzers
/// covering HTTP status, redirects, canonical URLs, meta tags, headings, links,
/// images, structured data, security, accessibility, and more.
pub mod analyzers;
/// SSRF (Server-Side Request Forgery) validation for URLs.
///
/// Shared validation used by both the plugin network guard and the API
/// crawl-submission endpoint to ensure URLs point to public, routable
/// HTTP(S) targets.
pub mod ssrf;
/// Article content generation from crawled page data.
///
/// Extracts and formats article content from web pages for
/// SEO-optimized content creation.
#[cfg(feature = "full")]
pub mod article_generator;
/// Audit trail logging for crawl operations and configuration changes.
///
/// Provides a tamper-evident append-only log with SHA-256 chaining for
/// compliance and security auditing.
#[cfg(feature = "full")]
pub mod audit;
/// API access logging for SOC 2 compliance.
///
/// Records every API access with user identity, timestamp, action,
/// resource, and outcome. Supports querying by user, action prefix,
/// time range, and success/failure status.
#[cfg(feature = "full")]
pub mod access_log;
/// Adapters for third-party backlink data sources (Ahrefs, GSC, Majestic).
///
/// Defines the BacklinkAdapter trait for
/// integrating external backlink data into crawl analysis.
#[cfg(feature = "full")]
pub mod backlink_adapters;
/// Backlink analysis, scoring, and reporting.
///
/// Computes PageRank-like scores from internal link graphs and produces
/// per-page backlink reports and site-wide summaries.
#[cfg(feature = "full")]
pub mod backlinks;
/// Backpressure controller to bound in-flight work and prevent memory blowouts.
///
/// Uses tokio semaphores and bounded channels to limit concurrent tasks,
/// ensuring the crawler stays within resource budgets.
#[cfg(feature = "unstable")]
pub mod backpressure;
/// Circuit breaker for failing HTTP endpoints to avoid cascading failures.
///
/// Per-domain circuit breakers track consecutive failures and automatically
/// stop requests to failing domains until they recover.
#[cfg(feature = "full")]
pub mod circuit_breaker;
/// Diff-based comparison of two crawl results.
///
/// Detects added/removed pages, status changes, title changes, content
/// changes, and Core Web Vitals regressions between crawls.
#[cfg(feature = "full")]
pub mod compare;
/// Content gap analysis for identifying missing topics and keywords.
///
/// Compares crawled content against target keywords to find
/// opportunities for new content creation.
#[cfg(feature = "full")]
pub mod content_gap;
/// Distributed crawling coordination: leader election and URL partitioning.
///
/// Provides [`CrawlCoordinator`] for partitioning URLs across multiple
/// crawler instances by domain hash, and defines [`PartitionStrategy`]
/// for hash-based or range-based partitioning.
#[cfg(feature = "full")]
pub mod coordination;
/// CrUX (Chrome User Experience Report) field data client.
///
/// Fetches real-world Core Web Vitals (LCP, CLS, INP, FCP, TTFB) from the
/// CrUX API for origin-level performance data.
#[cfg(feature = "full")]
pub mod crux;
/// Crawl engine that encapsulates the shared crawl loop for CLI and API consumers.
#[cfg(feature = "full")]
pub mod crawl_engine;
/// Deterministic replay controller for reproducible crawl runs.
///
/// Seed-based PRNG ensures that given the same input and configuration,
/// the crawler produces identical output for testing and auditing.
#[cfg(feature = "full")]
pub mod determinism;
/// Redis-backed distributed URL queue for multi-instance crawling.
///
/// Provides [`DistributedQueue`] for sharing crawl queues across multiple
/// crawler instances via Redis sorted sets. Each queue is namespaced by
/// a crawl ID to prevent collisions between different crawl sessions.
#[cfg(feature = "unstable")]
pub mod distributed_queue;
/// DNS resolution cache and prefetching.
///
/// Concurrent DNS cache with configurable TTL and background prefetching
/// to reduce DNS lookup latency during high-throughput crawling.
#[cfg(feature = "full")]
pub mod dns;
/// TLS and encryption configuration for HTTPS requests.
///
/// Provides AES-256-GCM encryption at rest for sensitive crawl data,
/// with key management via files, environment variables, or system keyrings.
#[cfg(feature = "full")]
pub mod encryption;
/// Enterprise feature gating and licensing utilities.
#[cfg(feature = "unstable")]
pub mod enterprise;
/// Export of crawl data to JSON, CSV, HTML, and Markdown formats.
///
/// Configurable column selection and formatting for CSV export,
/// with streaming support for large datasets.
#[cfg(feature = "full")]
pub mod export;
/// Feature flag system for toggling capabilities at runtime.
///
/// Flags are immutable per-crawl session once set. Supports TOML
/// configuration and programmatic access via SharedFeatureFlags.
#[cfg(feature = "full")]
pub mod feature_flags;
/// HTTP client with retry, redirect following, and rate limiting.
///
/// Provides HttpClient with exponential backoff retry,
/// manual redirect tracking, user-agent rotation, and streaming responses.
#[cfg(feature = "full")]
pub mod http;
/// Decision engine for determining whether a page requires JavaScript rendering.
///
/// Detects SPA frameworks (Next.js, Nuxt, SvelteKit, Angular) via HTML hints
/// and URL patterns to decide when to invoke Playwright.
#[cfg(feature = "full")]
pub mod js_render_decision;
/// Post-crawl analysis for cross-page SEO checks.
///
/// Runs after a crawl completes to detect site-wide issues like
/// canonical conflicts, redirect chains, and orphan pages.
#[cfg(feature = "full")]
pub mod post_crawl;
/// Monitoring: delta analysis for scheduled crawl comparison.
///
/// Compares two crawl results and determines whether significant changes
/// occurred, producing a [`MonitoringResult`](monitoring::MonitoringResult)
/// that can drive alerting and webhook delivery.
#[cfg(feature = "full")]
pub mod monitoring;
/// Search query tracking and SERP analysis.
///
/// Tracks search engine result pages (SERPs) for target keywords
/// and analyzes ranking positions and changes.
#[cfg(feature = "full")]
pub mod query_tracker;

/// Native plugin loading via dynamic linking (libloading).
///
/// Provides [`NativePlugin`](native_plugin::NativePlugin) for loading
/// shared libraries that implement the crawlkit native plugin ABI.
#[cfg(feature = "unstable")]
pub mod native_plugin;
/// Metrics collection and observability hooks.
///
/// Provides atomic Metrics for tracking pages crawled,
/// bytes fetched, timing, and circuit breaker events with zero-allocation hot paths.
#[cfg(feature = "full")]
pub mod observability;
/// Playwright-based headless browser integration for JS-rendered pages.
///
/// Renders JavaScript-heavy SPAs via Playwright CLI subprocess with
/// browser context isolation, resource limits, and console/network capture.
#[cfg(feature = "full")]
pub mod playwright;
/// Plugin system for extending the crawler with custom analyzers.
///
/// WASM-based plugins are sandboxed via wasmtime with a well-defined
/// ABI (`crawlkit_plugin_init`, `crawlkit_plugin_analyze`, `crawlkit_plugin_alloc/free`).
#[cfg(feature = "full")]
pub mod plugin;
pub mod plugin_index;
pub mod plugin_runtime;

#[cfg(feature = "full")]
pub use plugin::{
    ManifestError, PluginError, PluginInstance, PluginKind, PluginManifest, PluginMetadata,
    PluginRegistry, WasmConfig, WasmPlugin,
};
#[cfg(all(feature = "full", feature = "wasi-preview2"))]
pub use plugin::WasiPlugin;
pub use plugin_index::{
    install_plugin, list_installed_plugins, parse_plugin_index, PluginIndexEntry, PluginIndexError,
};
/// PostgreSQL-backed storage for crawl data.
///
/// Uses sqlx with connection pooling for async database access.
/// Requires the `postgres` feature to be enabled.
#[cfg(feature = "postgres")]
pub mod pg_storage;
/// Priority URL queue with depth and scope filtering.
///
/// Binary heap-based priority queue with deduplication, domain tracking,
/// and configurable scope control (allowed/blocked domains and paths).
#[cfg(feature = "full")]
pub mod queue;
/// Shared queue trait for pluggable queue backends.
///
/// Defines [`Queue`](queue_trait::Queue) for swapping between in-memory
/// and distributed queue implementations.
#[cfg(feature = "full")]
pub mod queue_trait;
/// Per-domain rate limiting to respect politeness constraints.
///
/// Token-bucket rate limiter with per-domain and global buckets,
/// supporting crawl-delay from robots.txt and concurrency limiting.
#[cfg(feature = "full")]
pub mod ratelimit;
/// Runtime resource monitoring and limit enforcement.
///
/// Tracks memory, CPU, disk, and page counts against configurable limits,
/// providing early termination when budgets are exceeded.
#[cfg(feature = "full")]
pub mod resource_monitor;
/// robots.txt parsing, caching, and compliance checking.
#[cfg(feature = "full")]
pub mod robots;
/// Real User Metrics (CrUX, GA) integration for performance data.
///
/// Fetches Core Web Vitals from Chrome UX Report API and Google Analytics,
/// merging lab and field data for comprehensive performance analysis.
#[cfg(feature = "full")]
pub mod rum;
/// Sitemap.xml parsing and URL discovery.
#[cfg(feature = "full")]
pub mod sitemap;
/// SQLite-backed persistent storage for crawl results and issues.
///
/// WAL-mode SQLite with LRU page cache, batch insert operations,
/// and memory usage tracking. Supports pages, links, issues, images,
/// structured data, and CrUX metrics.
#[cfg(feature = "full")]
pub mod storage;
/// Trait-based abstraction for storage backends.
///
/// Defines [`StorageBackend`](storage_trait::StorageBackend) for pluggable
/// storage implementations (SQLite, in-memory, distributed, etc.).
#[cfg(feature = "full")]
pub mod storage_trait;
/// Common types re-exported for backward compatibility.
pub mod types;
/// WASM-based analyzers for advanced code and performance analysis.
///
/// Static pattern analysis, runtime performance analysis, and
/// Playwright-powered rendering analysis for WebAssembly content.
#[cfg(feature = "full")]
pub mod wasm_analyzers;
/// Core Web Vitals measurement via Chrome DevTools Protocol.
///
/// Injects [`PerformanceObserver`](web_vitals::CWV_OBSERVER_SCRIPT) scripts
/// into Playwright-rendered pages to capture LCP, CLS, INP, FCP, and TTFB.
#[cfg(feature = "full")]
pub mod web_vitals;
/// LLM-powered post-crawl analysis plugin (user-brings-own-key).
///
/// Configurable with provider, model, and prompt templates. Supports
/// OpenAI and Anthropic APIs via [`LlmConfig`](llm_analyzer::LlmConfig).
#[cfg(feature = "full")]
pub mod llm_analyzer;
/// Google Search Console API client.
///
/// Full integration with GSC Search Analytics, URL Inspection,
/// and site management APIs.
#[cfg(feature = "full")]
pub mod gsc;
/// Historical trend analysis across multiple crawl snapshots.
///
/// Computes time-series trends for pages crawled, issues found,
/// and health scores, with linear regression for trend direction.
#[cfg(feature = "full")]
pub mod trends;
/// Prioritized insights engine that ranks post-crawl findings by impact and effort.
///
/// Aggregates findings across all crawled pages, computes impact scores
/// based on severity and prevalence, estimates fix effort, and produces
/// a ranked list of actionable insights — the key differentiator that
/// tools like Sitebulb and Lumar offer.
pub mod insights;

pub use ai_analyzers::{
    AiAnswerBoxAnalyzer, AiCitationEligibilityAnalyzer, AiContentStructureAnalyzer,
    AiCrawlerAccessibilityAnalyzer,
};
pub use ai_bots::{AiBot, AiBotRegistry};
pub use analyzers::{
    AccessibilityAnalyzer, AnalysisContext, Analyzer, AnalyzerRegistry, CacheHeaderAnalyzer,
    CanonicalUrlValidator, ContentQualityAnalyzer, CrawlData, EcommerceSignalsAnalyzer,
    EnhancedReadabilityAnalyzer, EntityAnalyzer, Finding, HeadingHierarchyAnalyzer,
    HreflangValidator, HttpStatusAnalyzer, ImageAnalyzer, ImageInfo, InternationalSeoAnalyzer,
    KeywordAnalyzer, LinkAnalyzer, LinkInfo, MetaTagAnalyzer, MobileFriendlinessChecker,
    PostCrawlAnalyzer, PostCrawlAnalyzerRegistry, RedirectChainAnalyzer, ResourceCountAnalyzer,
    ResponseSizeAnalyzer, RobotsRule, RobotsTxtAnalyzer, SecurityHeaderAnalyzer, SitemapAnalyzer,
    SitemapEntry, SocialMediaAnalyzer, SslCertificateInfo, SslCertificateValidator,
    StructuredDataValidator, TtfbAnalyzer, WordCountAnalyzer,
};
#[cfg(feature = "full")]
pub use audit::{AuditEvent, AuditEventType, AuditTrail};
#[cfg(feature = "full")]
pub use access_log::{AccessLogEntry, AccessLogFilter, AccessLogger};
#[cfg(feature = "full")]
pub use backlink_adapters::{
    AdapterError, AhrefsAdapter, BacklinkAdapter, BacklinkAdapterRegistry, ExternalBacklink,
    GscAdapter, MajesticAdapter,
};
#[cfg(feature = "full")]
pub use gsc::{GscAnalytics, GscClient, GscError, GscRow, UrlInspection};
#[cfg(feature = "full")]
pub use trends::{
    analyze_trends, compute_health_score, trend_to_json, trend_to_markdown, CrawlSnapshot,
    TrendAnalysis, TrendDirection, TrendError, TrendPoint, TrendSummary,
};
#[cfg(feature = "full")]
pub use backlinks::{Backlink, BacklinkAnalyzer, BacklinkReport, BacklinkSummary, PageScore};
#[cfg(feature = "unstable")]
pub use backpressure::{BackpressureController, BackpressureError, BoundedPipeline};
#[cfg(feature = "full")]
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerRegistry, CircuitState,
};
#[cfg(feature = "full")]
pub use determinism::DeterminismController;
#[cfg(feature = "full")]
pub use dns::{DnsCache, DnsError, DnsPrefetcher};
#[cfg(feature = "full")]
pub use encryption::{EncryptionConfig, EncryptionError, EncryptionManager};
#[cfg(feature = "full")]
pub use feature_flags::{
    FeatureFlags, SharedFeatureFlags, FLAG_AI_ANALYZERS, FLAG_JS_RENDERING, FLAG_WASM_ANALYZERS,
};
#[cfg(feature = "full")]
pub use http::{FetchStreamReader, HttpClient, HttpClientConfig};
#[cfg(feature = "full")]
pub use js_render_decision::{JsRenderDecision, JsRenderDecisionEngine, SpaIndicators};

#[cfg(feature = "full")]
pub use observability::{Metrics, MetricsSnapshot, SharedMetrics};
#[cfg(all(feature = "full", feature = "observability"))]
pub use observability::otel;
#[cfg(feature = "postgres")]
pub use pg_storage::PgStorage;
#[cfg(feature = "full")]
pub use playwright::{
    BrowserContext, BrowserType, ConsoleMessage, NetworkRequest, PlaywrightConfig,
    PlaywrightDetector, PlaywrightError, PlaywrightRenderer, RenderedPage,
    WasmError as PlaywrightWasmError,
};
#[cfg(feature = "full")]
pub use resource_monitor::{ResourceLimits, ResourceMonitor, ResourceUsage};
#[cfg(feature = "full")]
pub use robots::RobotsTxtCache;
#[cfg(feature = "full")]
pub use crux::{CruxClient, CruxError, CruxFieldData};
#[cfg(feature = "full")]
pub use rum::{
    CruxAdapter, CruxData, FieldMetrics, GoogleAnalyticsAdapter, LabMetrics, MergedMetrics,
    MetricDeltas, RumDataPoint, RumError,
};
#[cfg(feature = "full")]
pub use sitemap::SitemapCache;
#[cfg(feature = "full")]
pub use storage::{
    CacheStats, CrawlStats, Issue, IssueCategory, IssueFilter, Severity, StorageError,
};
#[cfg(feature = "full")]
pub use storage_trait::{new_in_memory_backend, StorageBackend};
#[cfg(feature = "full")]
pub use wasm_analyzers::{WasmPatternAnalyzer, WasmPerformanceAnalyzer, WasmRuntimeAnalyzer};
#[cfg(feature = "full")]
pub use web_vitals::{WebVitals, WebVitalsError, WebVitalsMeasurer};

/// Declarative custom extraction engine.
///
/// Applies user-defined CSS selector + regex rules to each crawled page,
/// extracting structured fields for storage and export. Configuration
/// is driven by `[[extraction.rules]]` entries in `crawlkit.toml`.
pub mod extraction;
/// HTML meta tag extraction (title, description, OG, Twitter Cards, hreflang).
///
/// Provides MetaTags with helper methods for checking
/// `noindex`/`nofollow` directives and measuring tag lengths.
pub mod meta;
/// HTML parser that extracts links, headings, images, forms, and structured data.
///
/// [`HtmlParser::parse`] produces a [`ParsedPage`] with all SEO-relevant data
/// extracted from raw HTML, including accessibility landmarks and social metadata.
pub mod parser;
/// Web server access log parsing for Nginx/Apache combined and JSON formats.
pub mod log_parser;
/// Log analysis: crawler breakdown, status codes, top URLs, and error reporting.
pub mod log_analyzer;

pub use meta::{HreflangTag, MetaTags, OpenGraphTags, TwitterTags};
pub use parser::{
    ExtractedForm, ExtractedImage, ExtractedInput, ExtractedLink, Heading, HtmlParser, ParsedPage,
    ParserEvent, ScriptInfo, StreamingHtmlParser, StructuredData, StyleInfo,
};

mod duration_ms {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ms = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(ms))
    }
}

mod opt_duration_ms {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.map(|d| d.as_millis()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ms = Option::<u64>::deserialize(deserializer)?;
        Ok(ms.map(Duration::from_millis))
    }
}

/// Errors that can occur during crawl operations.
///
/// This is the primary error type for the crawlkit crate. It wraps
/// lower-level errors from URL parsing, HTTP requests, robots.txt,
/// and storage operations.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::CrawlError;
///
/// let err = CrawlError::TooManyRedirects(20);
/// assert!(err.to_string().contains("20"));
/// ```
#[derive(Debug, Error)]
pub enum CrawlError {
    /// The URL could not be parsed.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// The HTTP request failed (full mode — reqwest error).
    #[cfg(feature = "full")]
    #[error("request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    /// The HTTP request failed (WASM mode — string error).
    #[cfg(not(feature = "full"))]
    #[error("request failed: {0}")]
    RequestFailed(String),

    /// The URL exceeded the maximum redirect limit.
    #[error("too many redirects ({0})")]
    TooManyRedirects(usize),

    /// The URL is excluded by robots.txt disallow rules.
    #[error("blocked by robots.txt: {0}")]
    BlockedByRobotsTxt(String),

    /// The URL is outside the allowed domain scope.
    #[error("out of scope: {0}")]
    OutOfScope(String),

    /// A storage/database error occurred.
    #[cfg(feature = "full")]
    #[error("storage error: {0}")]
    Storage(#[from] crate::storage::StorageError),

    /// An unexpected internal failure (I/O, environment, or subsystem
    /// errors that are not storage-related).
    #[error("internal error: {0}")]
    Internal(String),

    /// All retry attempts were exhausted.
    #[error("max retries exceeded after {0} attempts")]
    MaxRetriesExceeded(usize),
}

/// Configuration for a crawl session.
///
/// Controls the starting URL, crawl limits, politeness settings, and
/// URL filtering patterns. Implements `Default` with sensible values
/// for most use cases.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::CrawlConfig;
/// use std::time::Duration;
///
/// let config = CrawlConfig {
///     max_pages: 500,
///     request_delay: Duration::from_millis(200),
///     concurrency: 8,
///     ..Default::default()
/// };
/// assert_eq!(config.max_pages, 500);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    /// The starting URL to crawl.
    pub start_url: Url,

    /// Maximum number of pages to crawl.
    pub max_pages: usize,

    /// Maximum crawl duration. `None` means no time limit.
    #[serde(default, with = "opt_duration_ms")]
    pub max_time: Option<Duration>,

    /// Maximum crawl depth from the starting URL. `None` means no depth limit.
    pub max_depth: Option<usize>,

    /// Delay between requests to the same domain.
    #[serde(with = "duration_ms")]
    pub request_delay: Duration,

    /// Number of concurrent fetchers per domain.
    pub concurrency: usize,

    /// HTTP request timeout.
    #[serde(with = "duration_ms")]
    pub request_timeout: Duration,

    /// User-Agent string to send with requests.
    pub user_agent: String,

    /// Maximum number of redirects to follow.
    pub max_redirects: usize,

    /// Whether to respect robots.txt directives.
    pub respect_robots_txt: bool,

    /// Allowed URL patterns (glob-style).
    pub allowed_patterns: Vec<String>,

    /// Disallowed URL patterns (glob-style).
    pub disallowed_patterns: Vec<String>,

    /// Custom extraction rules applied to each crawled page.
    #[serde(default)]
    pub extraction: extraction::ExtractionConfig,

    /// Data retention policy: automatically purge crawls older than this
    /// many days. `None` means data is retained indefinitely.
    #[serde(default)]
    pub data_retention_days: Option<u32>,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            start_url: Url::parse("https://example.com")
                .unwrap_or_else(|_| unreachable!("static URL string is always valid")),
            max_pages: 100,
            max_time: None,
            max_depth: None,
            request_delay: Duration::from_millis(500),
            concurrency: 4,
            request_timeout: Duration::from_secs(30),
            user_agent: format!("crawlkit/{}", env!("CARGO_PKG_VERSION")),
            max_redirects: 20,
            respect_robots_txt: true,
            allowed_patterns: Vec::new(),
            disallowed_patterns: Vec::new(),
            extraction: extraction::ExtractionConfig::default(),
            data_retention_days: None,
        }
    }
}

/// Represents a single URL discovered during crawling.
///
/// Tracks the original URL, its canonical form, the referring page,
/// crawl depth, and discovery timestamp. Used by the crawl queue
/// and storage layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlEntry {
    /// The original URL as discovered.
    pub url: Url,

    /// The canonical URL after normalization.
    pub canonical_url: Url,

    /// The page that linked to this URL.
    pub referrer: Option<Url>,

    /// The depth from the starting URL (0 = start).
    pub depth: usize,

    /// When this URL was discovered.
    pub discovered_at: DateTime<Utc>,
}

/// The result of fetching a URL.
///
/// Contains the final URL (after redirects), HTTP status, response headers,
/// body content, timing, and size information. This is the primary output
/// of [`HttpClient::fetch`](http::HttpClient::fetch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    /// The final URL after following redirects.
    pub final_url: Url,

    /// HTTP status code.
    pub status_code: u16,

    /// Response headers.
    pub headers: Vec<(String, String)>,

    /// The response body as a UTF-8 string.
    pub body: String,

    /// Time taken for the request.
    #[serde(with = "duration_ms")]
    pub response_time: Duration,

    /// Size of the response body in bytes.
    pub body_size: usize,

    /// When the request was made.
    pub fetched_at: DateTime<Utc>,

    /// The ETag header value, if present.
    pub etag: Option<String>,

    /// The Last-Modified header value, if present.
    pub last_modified: Option<String>,
}

/// A single hop in a redirect chain.
///
/// Records the source and destination URLs along with the HTTP status code
/// (301, 302, 307, 308) for each redirect. Multiple hops form a
/// RedirectChainAnalyzer input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectHop {
    /// The URL that redirected.
    pub from: Url,

    /// The URL redirected to.
    pub to: Url,

    /// The HTTP status code (301, 302, 307, 308).
    pub status_code: u16,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_crawl_config_default() {
        let config = CrawlConfig::default();
        assert_eq!(config.max_pages, 100);
        assert_eq!(config.concurrency, 4);
        assert_eq!(config.max_redirects, 20);
        assert!(config.respect_robots_txt);
    }

    #[test]
    fn test_crawl_config_serialization() {
        let config = CrawlConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: CrawlConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.max_pages, deserialized.max_pages);
        assert_eq!(config.request_delay, deserialized.request_delay);
        assert_eq!(config.request_timeout, deserialized.request_timeout);
    }

    #[test]
    fn test_url_entry_serialization() {
        let entry = UrlEntry {
            url: Url::parse("https://example.com").unwrap(),
            canonical_url: Url::parse("https://example.com").unwrap(),
            referrer: None,
            depth: 0,
            discovered_at: Utc::now(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: UrlEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.url, deserialized.url);
        assert_eq!(entry.depth, deserialized.depth);
    }

    #[test]
    fn test_redirect_hops() {
        let hops = [
            RedirectHop {
                from: Url::parse("https://example.com/old").unwrap(),
                to: Url::parse("https://example.com/mid").unwrap(),
                status_code: 301,
            },
            RedirectHop {
                from: Url::parse("https://example.com/mid").unwrap(),
                to: Url::parse("https://example.com/new").unwrap(),
                status_code: 302,
            },
        ];

        assert_eq!(hops.len(), 2);
        assert_eq!(hops[0].status_code, 301);
        assert_eq!(hops[1].status_code, 302);
    }

    #[test]
    fn test_fetch_result_serialization() {
        let result = FetchResult {
            final_url: Url::parse("https://example.com").unwrap(),
            status_code: 200,
            headers: vec![("content-type".into(), "text/html".into())],
            body: "<html></html>".into(),
            response_time: Duration::from_millis(123),
            body_size: 14,
            fetched_at: Utc::now(),
            etag: Some("\"abc123\"".into()),
            last_modified: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("123"));
        let deserialized: FetchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.response_time, Duration::from_millis(123));
    }
}
