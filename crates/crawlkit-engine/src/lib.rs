//! # crawlkit-core
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
//! - **28 SEO analyzers** covering meta tags, content quality, security, accessibility
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
//! let parsed = HtmlParser::parse(&result.body, &url)?;
//! # Ok(())
//! # }
//! ```
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use url::Url;

/// Advanced crawl features such as JavaScript rendering and WASM analysis.
///
/// Provides [`AlertManager`](advanced_features::AlertManager) for monitoring
/// crawl metrics and triggering notifications on threshold breaches.
pub mod advanced_canonical;
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
/// Defines the [`Analyzer`](analyzers::Analyzer) trait and provides 28+ built-in analyzers
/// covering HTTP status, redirects, canonical URLs, meta tags, headings, links,
/// images, structured data, security, accessibility, and more.
pub mod analyzers;
/// Audit trail logging for crawl operations and configuration changes.
///
/// Provides a tamper-evident append-only log with SHA-256 chaining for
/// compliance and security auditing.
pub mod audit;
/// Adapters for third-party backlink data sources (Ahrefs, GSC, Majestic).
///
/// Defines the [`BacklinkAdapter`](backlink_adapters::BacklinkAdapter) trait for
/// integrating external backlink data into crawl analysis.
pub mod backlink_adapters;
/// Backlink analysis, scoring, and reporting.
///
/// Computes PageRank-like scores from internal link graphs and produces
/// per-page backlink reports and site-wide summaries.
pub mod backlinks;
/// Backpressure controller to bound in-flight work and prevent memory blowouts.
///
/// Uses tokio semaphores and bounded channels to limit concurrent tasks,
/// ensuring the crawler stays within resource budgets.
pub mod backpressure;
/// Circuit breaker for failing HTTP endpoints to avoid cascading failures.
///
/// Per-domain circuit breakers track consecutive failures and automatically
/// stop requests to failing domains until they recover.
pub mod circuit_breaker;
/// Diff-based comparison of two crawl results.
///
/// Detects added/removed pages, status changes, title changes, content
/// changes, and Core Web Vitals regressions between crawls.
pub mod compare;
/// Deterministic replay controller for reproducible crawl runs.
///
/// Seed-based PRNG ensures that given the same input and configuration,
/// the crawler produces identical output for testing and auditing.
pub mod determinism;
/// DNS resolution cache and prefetching.
///
/// Concurrent DNS cache with configurable TTL and background prefetching
/// to reduce DNS lookup latency during high-throughput crawling.
pub mod dns;
/// TLS and encryption configuration for HTTPS requests.
///
/// Provides AES-256-GCM encryption at rest for sensitive crawl data,
/// with key management via files, environment variables, or system keyrings.
pub mod encryption;
/// Enterprise feature gating and licensing utilities.
pub mod enterprise;
/// Export of crawl data to JSON, CSV, HTML, and Markdown formats.
///
/// Configurable column selection and formatting for CSV export,
/// with streaming support for large datasets.
pub mod export;
/// Feature flag system for toggling capabilities at runtime.
///
/// Flags are immutable per-crawl session once set. Supports TOML
/// configuration and programmatic access via [`SharedFeatureFlags`](feature_flags::SharedFeatureFlags).
pub mod feature_flags;
/// HTTP client with retry, redirect following, and rate limiting.
///
/// Provides [`HttpClient`](http::HttpClient) with exponential backoff retry,
/// manual redirect tracking, user-agent rotation, and streaming responses.
pub mod http;
/// Decision engine for determining whether a page requires JavaScript rendering.
///
/// Detects SPA frameworks (Next.js, Nuxt, SvelteKit, Angular) via HTML hints
/// and URL patterns to decide when to invoke Playwright.
pub mod js_render_decision;

/// Metrics collection and observability hooks.
///
/// Provides atomic [`Metrics`](observability::Metrics) for tracking pages crawled,
/// bytes fetched, timing, and circuit breaker events with zero-allocation hot paths.
pub mod observability;
/// Playwright-based headless browser integration for JS-rendered pages.
///
/// Renders JavaScript-heavy SPAs via Playwright CLI subprocess with
/// browser context isolation, resource limits, and console/network capture.
pub mod playwright;
/// Plugin system for extending the crawler with custom analyzers.
///
/// WASM-based plugins are sandboxed via wasmtime with a well-defined
/// ABI (`crawlkit_plugin_init`, `crawlkit_plugin_analyze`, `crawlkit_plugin_alloc/free`).
pub mod plugin;

pub use plugin::{PluginError, PluginManifest, PluginMetadata, PluginRegistry, WasmPlugin};
/// Priority URL queue with depth and scope filtering.
///
/// Binary heap-based priority queue with deduplication, domain tracking,
/// and configurable scope control (allowed/blocked domains and paths).
pub mod queue;
/// Per-domain rate limiting to respect politeness constraints.
///
/// Token-bucket rate limiter with per-domain and global buckets,
/// supporting crawl-delay from robots.txt and concurrency limiting.
pub mod ratelimit;
/// Runtime resource monitoring and limit enforcement.
///
/// Tracks memory, CPU, disk, and page counts against configurable limits,
/// providing early termination when budgets are exceeded.
pub mod resource_monitor;
/// robots.txt parsing, caching, and compliance checking.
pub mod robots;
/// Real User Metrics (CrUX, GA) integration for performance data.
///
/// Fetches Core Web Vitals from Chrome UX Report API and Google Analytics,
/// merging lab and field data for comprehensive performance analysis.
pub mod rum;
/// Sitemap.xml parsing and URL discovery.
pub mod sitemap;
/// SQLite-backed persistent storage for crawl results and issues.
///
/// WAL-mode SQLite with LRU page cache, batch insert operations,
/// and memory usage tracking. Supports pages, links, issues, images,
/// structured data, and CrUX metrics.
pub mod storage;
/// WASM-based analyzers for advanced code and performance analysis.
///
/// Static pattern analysis, runtime performance analysis, and
/// Playwright-powered rendering analysis for WebAssembly content.
pub mod wasm_analyzers;

pub use ai_analyzers::{
    AiAnswerBoxAnalyzer, AiCitationEligibilityAnalyzer, AiContentStructureAnalyzer,
    AiCrawlerAccessibilityAnalyzer,
};
pub use ai_bots::{AiBot, AiBotRegistry};
pub use analyzers::{
    AccessibilityAnalyzer, AnalysisContext, Analyzer, AnalyzerRegistry, CanonicalUrlValidator,
    ContentQualityAnalyzer, EcommerceSignalsAnalyzer, EnhancedReadabilityAnalyzer, EntityAnalyzer,
    Finding, HeadingHierarchyAnalyzer, HreflangValidator, HttpStatusAnalyzer, ImageAnalyzer,
    ImageInfo, InternationalSeoAnalyzer, KeywordAnalyzer, LinkAnalyzer, LinkInfo, MetaTagAnalyzer,
    MobileFriendlinessChecker, RedirectChainAnalyzer, RobotsRule, RobotsTxtAnalyzer,
    SecurityHeaderAnalyzer, SitemapAnalyzer, SitemapEntry, SocialMediaAnalyzer, SslCertificateInfo,
    SslCertificateValidator, StructuredDataValidator, WordCountAnalyzer,
};
pub use audit::{AuditEvent, AuditEventType, AuditTrail};
pub use backlink_adapters::{
    AdapterError, AhrefsAdapter, BacklinkAdapter, BacklinkAdapterRegistry, ExternalBacklink,
    GscAdapter, MajesticAdapter,
};
pub use backlinks::{Backlink, BacklinkAnalyzer, BacklinkReport, BacklinkSummary, PageScore};
pub use backpressure::{BackpressureController, BackpressureError, BoundedPipeline};
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerRegistry, CircuitState,
};
pub use determinism::DeterminismController;
pub use dns::{DnsCache, DnsError, DnsPrefetcher};
pub use encryption::{EncryptionConfig, EncryptionError, EncryptionManager};
pub use feature_flags::{
    FeatureFlags, SharedFeatureFlags, FLAG_AI_ANALYZERS, FLAG_JS_RENDERING, FLAG_WASM_ANALYZERS,
};
pub use http::{FetchStreamReader, HttpClient, HttpClientConfig};
pub use js_render_decision::{JsRenderDecision, JsRenderDecisionEngine, SpaIndicators};

pub use observability::{Metrics, MetricsSnapshot, SharedMetrics};
pub use playwright::{
    BrowserContext, BrowserType, ConsoleMessage, NetworkRequest, PlaywrightConfig,
    PlaywrightDetector, PlaywrightError, PlaywrightRenderer, RenderedPage,
    WasmError as PlaywrightWasmError,
};
pub use resource_monitor::{ResourceLimits, ResourceMonitor, ResourceUsage};
pub use robots::RobotsTxtCache;
pub use rum::{
    CruxAdapter, CruxData, FieldMetrics, GoogleAnalyticsAdapter, LabMetrics, MergedMetrics,
    MetricDeltas, RumDataPoint, RumError,
};
pub use sitemap::SitemapCache;
pub use storage::{
    CacheStats, CrawlStats, Issue, IssueCategory, IssueFilter, Severity, StorageError,
};
pub use wasm_analyzers::{WasmPatternAnalyzer, WasmPerformanceAnalyzer, WasmRuntimeAnalyzer};

/// HTML meta tag extraction (title, description, OG, Twitter Cards, hreflang).
///
/// Provides [`MetaTags`](meta::MetaTags) with helper methods for checking
/// `noindex`/`nofollow` directives and measuring tag lengths.
pub mod meta;
/// HTML parser that extracts links, headings, images, forms, and structured data.
///
/// [`HtmlParser::parse`] produces a [`ParsedPage`] with all SEO-relevant data
/// extracted from raw HTML, including accessibility landmarks and social metadata.
pub mod parser;

pub use meta::{HreflangTag, MetaTags, OpenGraphTags, TwitterTags};
pub use parser::{
    ExtractedForm, ExtractedImage, ExtractedInput, ExtractedLink, Heading, HtmlParser, ParseError,
    ParsedPage, ScriptInfo, StreamingHtmlParser, StructuredData, StyleInfo,
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

    /// The HTTP request failed.
    #[error("request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

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
    #[error("storage error: {0}")]
    Storage(String),

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
}

/// A single hop in a redirect chain.
///
/// Records the source and destination URLs along with the HTTP status code
/// (301, 302, 307, 308) for each redirect. Multiple hops form a
/// [`RedirectChainAnalyzer`](analyzers::RedirectChainAnalyzer) input.
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
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("123"));
        let deserialized: FetchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.response_time, Duration::from_millis(123));
    }
}
