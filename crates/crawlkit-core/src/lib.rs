//! Core library for crawlkit — an SEO site crawler and analyzer.
//!
//! This crate provides the foundational types, HTTP fetching, HTML parsing,
//! SEO analyzers, crawl queue, storage, and observability primitives used by
//! the crawlkit CLI and API server.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use url::Url;

/// Advanced crawl features such as JavaScript rendering and WASM analysis.
pub mod advanced_features;
/// AI-powered page content analyzers (answer boxes, citations, accessibility).
pub mod ai_analyzers;
/// Registry of known AI bot user-agents and crawler identification.
pub mod ai_bots;
/// SEO analysis engine with pluggable analyzers (title, meta, links, etc.).
pub mod analyzers;
/// Audit trail logging for crawl operations and configuration changes.
pub mod audit;
/// Adapters for third-party backlink data sources (Ahrefs, GSC, Majestic).
pub mod backlink_adapters;
/// Backlink analysis, scoring, and reporting.
pub mod backlinks;
/// Backpressure controller to bound in-flight work and prevent memory blowouts.
pub mod backpressure;
/// Circuit breaker for failing HTTP endpoints to avoid cascading failures.
pub mod circuit_breaker;
/// Diff-based comparison of two crawl results.
pub mod compare;
/// Deterministic replay controller for reproducible crawl runs.
pub mod determinism;
/// DNS resolution cache and prefetching.
pub mod dns;
/// TLS and encryption configuration for HTTPS requests.
pub mod encryption;
/// Enterprise feature gating and licensing utilities.
pub mod enterprise;
/// Export of crawl data to JSON, CSV, HTML, and Markdown formats.
pub mod export;
/// Feature flag system for toggling capabilities at runtime.
pub mod feature_flags;
/// HTTP client with retry, redirect following, and rate limiting.
pub mod http;
/// Decision engine for determining whether a page requires JavaScript rendering.
pub mod js_render_decision;
/// Directed graph of inter-page links for site structure analysis.
pub mod link_graph;
/// Metrics collection and observability hooks.
pub mod observability;
/// Playwright-based headless browser integration for JS-rendered pages.
pub mod playwright;
/// Plugin system for extending the crawler with custom analyzers.
pub mod plugin;
/// Priority URL queue with depth and scope filtering.
pub mod queue;
/// Per-domain rate limiting to respect politeness constraints.
pub mod ratelimit;
/// Runtime resource monitoring and limit enforcement.
pub mod resource_monitor;
/// robots.txt parsing, caching, and compliance checking.
pub mod robots;
/// Real User Metrics (CrUX, GA) integration for performance data.
pub mod rum;
/// Sitemap.xml parsing and URL discovery.
pub mod sitemap;
/// SQLite-backed persistent storage for crawl results and issues.
pub mod storage;
/// WASM-based analyzers for advanced code and performance analysis.
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
pub use link_graph::LinkGraph;
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
pub mod meta;
/// HTML parser that extracts links, headings, images, forms, and structured data.
pub mod parser;

pub use meta::{HreflangTag, MetaTags, OpenGraphTags, TwitterTags};
pub use parser::{
    ExtractedForm, ExtractedImage, ExtractedInput, ExtractedLink, Heading, HtmlParser, ParseError,
    ParsedPage, ScriptInfo, StructuredData, StyleInfo,
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
            start_url: Url::parse("https://example.com").expect("valid default URL"),
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
