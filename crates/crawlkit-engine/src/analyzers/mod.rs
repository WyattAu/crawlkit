use std::time::Duration;

use crate::parser::ParsedPage;
use crate::{CrawlConfig, RedirectHop};

/// Content quality analyzers for readability, entity extraction, and structured data.
pub mod content_analyzers;
/// HTTP-level analyzers for status codes, redirects, robots.txt, and SSL certificates.
pub mod http_analyzers;
/// Media and e-commerce analyzers for images, products, and shopping signals.
pub mod media_analyzers;
/// Security and accessibility analyzers for headers, mobile-friendliness, and WCAG compliance.
pub mod security_analyzers;
/// SEO analyzers for meta tags, headings, links, canonicals, and internationalization.
pub mod seo_analyzers;
/// Social media analyzers for Open Graph, Twitter Cards, and social sharing metadata.
pub mod social_analyzers;

pub use content_analyzers::{
    BreadcrumbsValidator, ContentThinAnalyzer, ContentQualityAnalyzer, CouponSchemaValidator,
    DatasetSchemaValidator, DuplicateContentDetector, EnhancedReadabilityAnalyzer,
    EntityAnalyzer, EntityLinkingAnalyzer, EventSchemaValidator, FaqSchemaValidator,
    HowToSchemaValidator, JsonLdValidator, LocalBusinessSchemaValidator, MetaDescriptionLengthAnalyzer,
    MicrodataValidator, OfferAvailabilityAnalyzer, ReviewSchemaValidator, RdfaValidator,
    ShippingSchemaValidator, SoftwareApplicationValidator, SpeakableSchemaValidator,
    SpecialAnnouncementSchemaValidator, StructuredDataValidator, TableOfContentsAnalyzer,
    TitleLengthAnalyzer, VideoSchemaValidator,
};
pub use http_analyzers::{
    CacheHeaderAnalyzer, HttpStatusAnalyzer, RedirectChainAnalyzer, ResponseSizeAnalyzer,
    RobotsRule, RobotsTxtAnalyzer, SslCertificateValidator, TtfbAnalyzer,
};
pub use media_analyzers::{
    AggregateRatingValidator, CriticalResourceAnalyzer, EcommerceSignalsAnalyzer, ImageAnalyzer,
    ImageInfo, PricingSchemaValidator, ProductVariantAnalyzer, ResourceCountAnalyzer,
};
pub use security_analyzers::{
    AccessibilityAnalyzer, ColorContrastAnalyzer, CrossOriginIsolationAnalyzer,
    FocusOrderAnalyzer, FontSizeAnalyzer, HstsPreloadAnalyzer, MobileFriendlinessChecker,
    PermissionPolicyAnalyzer, SecurityHeaderAnalyzer, SriAnalyzer,
};
pub use seo_analyzers::{
    CanonicalDepthAnalyzer, CanonicalUrlValidator, CharsetValidator, HeadingHierarchyAnalyzer,
    HreflangConsistencyAnalyzer, HreflangValidator, InternationalSeoAnalyzer, KeywordAnalyzer,
    LanguageAttributeAnalyzer, LinkAnalyzer, LinkInfo, MetaTagAnalyzer, MobileViewportAnalyzer,
    OpenSearchValidator, PaginationAnalyzer, RobotsMetaAnalyzer, SitemapAnalyzer, SitemapEntry,
    WordCountAnalyzer,
};
pub use social_analyzers::{OpenGraphImageValidator, SocialMediaAnalyzer, TwitterPlayerValidator};
pub use crate::advanced_canonical::{CanonicalChainDetector, HreflangReciprocalValidator};

/// Check if a URL path is a utility/page that should be excluded from analysis.
fn is_utility_page(url: &str) -> bool {
    let lower = url.to_lowercase();
    [
        "/login",
        "/register",
        "/signup",
        "/signin",
        "/auth",
        "/admin",
        "/dashboard",
        "/settings",
        "/account",
        "/search",
        "/sitemap",
        "/robots.txt",
        "/wp-admin",
        "/wp-login.php",
        "/cgi-bin",
        "/_next",
        "/_nuxt",
        "/compare",
        "/wishlist",
        "/cart",
        "/checkout",
        "/forgot",
        "/contact",
        "/about",
    ]
    .iter()
    .any(|p| lower.contains(p))
}

/// Count sentences in text (by punctuation).
fn count_sentences(text: &str) -> usize {
    if text.trim().is_empty() {
        return 1;
    }
    text.chars()
        .filter(|&c| c == '.' || c == '!' || c == '?')
        .count()
        .max(1)
}

/// Count syllables in a word (approximate heuristic).
fn count_syllables(word: &str) -> usize {
    let word = word.to_lowercase();
    let chars: Vec<char> = word.chars().collect();
    if chars.is_empty() {
        return 0;
    }
    if chars.len() <= 2 {
        return 1;
    }

    let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
    let mut count = 0;
    let mut prev_vowel = false;

    for &c in &chars {
        let is_vowel = vowels.contains(&c);
        if is_vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = is_vowel;
    }

    if chars.last() == Some(&'e') && count > 1 {
        count -= 1;
    }

    count.max(1)
}

/// Flesch-Kincaid grade level.
fn flesch_kincaid_grade(words: usize, sentences: usize, syllables: usize) -> f64 {
    if words == 0 || sentences == 0 {
        return 0.0;
    }
    0.39 * (words as f64 / sentences as f64) + 11.8 * (syllables as f64 / words as f64) - 15.59
}

/// Flesch Reading Ease score (0-100, higher = easier).
fn flesch_reading_ease(words: usize, sentences: usize, syllables: usize) -> f64 {
    if words == 0 || sentences == 0 {
        return 0.0;
    }
    let score = 206.835
        - 1.015 * (words as f64 / sentences as f64)
        - 84.6 * (syllables as f64 / words as f64);
    score.clamp(0.0, 100.0)
}

/// Pre-fetched TLS certificate information for offline validation.
#[derive(Debug, Clone)]
pub struct SslCertificateInfo {
    /// The subject Common Name (CN) or first SAN entry.
    pub subject: Option<String>,
    /// The certificate issuer.
    pub issuer: Option<String>,
    /// Subject Alternative Names (DNS names, IP addresses).
    pub san_entries: Vec<String>,
    /// Certificate valid-from timestamp (ISO 8601).
    pub not_before: Option<String>,
    /// Certificate expiry timestamp (ISO 8601).
    pub not_after: Option<String>,
    /// Whether the certificate chain validated successfully.
    pub is_valid_chain: bool,
    /// Whether the certificate is self-signed.
    pub is_self_signed: bool,
    /// The signature algorithm (e.g. "SHA256withRSA").
    pub signature_algorithm: Option<String>,
}

/// Context for analyzing a page, bundling parsed HTML with HTTP metadata.
///
/// Passed to each [`Analyzer`] to provide all the data needed for analysis.
/// The context borrows the parsed page and response metadata to avoid
/// unnecessary cloning.
pub struct AnalysisContext<'a> {
    /// The parsed page content.
    pub page: &'a ParsedPage,
    /// Raw response body text (if available).
    pub body: Option<&'a str>,
    /// HTTP status code (if fetched).
    pub status_code: Option<u16>,
    /// Response headers.
    pub headers: &'a [(String, String)],
    /// Response time (if measured).
    pub response_time: Option<Duration>,
    /// Redirect chain hops (if any).
    pub redirect_chain: &'a [RedirectHop],
    /// Pre-fetched robots.txt content for this page's domain (if available).
    pub robots_txt: Option<&'a str>,
    /// Response body size in bytes (uncompressed).
    pub body_size: Option<usize>,
    /// Compressed response size in bytes (if Content-Length available).
    pub compressed_size: Option<usize>,
    /// Server header value (e.g., "nginx/1.24.0").
    pub server: Option<&'a str>,
    /// Content-Type header value (e.g., "text/html; charset=utf-8").
    pub content_type: Option<&'a str>,
}

/// A finding/issue detected by an analyzer.
///
/// Represents a single SEO or technical issue found during page analysis.
/// Each finding has a severity, category, machine-readable code, and
/// a human-readable recommendation for fixing the issue.
pub use crawlkit_types::Finding;

/// Trait for page analyzers.
///
/// Implement this trait to create custom SEO analyzers. Each analyzer
/// receives an [`AnalysisContext`] and returns a list of [`Finding`]s.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::analyzers::{Analyzer, AnalysisContext, Finding};
/// use crawlkit_engine::CrawlConfig;
///
/// struct MyAnalyzer;
///
/// impl Analyzer for MyAnalyzer {
///     fn name(&self) -> &str { "my-analyzer" }
///     fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
///         vec![]
///     }
/// }
/// ```
pub trait Analyzer: Send + Sync {
    /// Returns the human-readable name of this analyzer.
    fn name(&self) -> &str;

    /// Analyze a page and return any findings/issues.
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding>;
}

/// Stop words for keyword density analysis (common English words).
pub(crate) const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "is",
    "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does", "did", "will",
    "would", "could", "should", "may", "might", "shall", "can", "it", "its", "this", "that",
    "these", "those", "i", "you", "he", "she", "we", "they", "me", "him", "her", "us", "them",
    "my", "your", "his", "our", "their", "what", "which", "who", "whom", "where", "when", "why",
    "how", "all", "each", "every", "both", "few", "more", "most", "other", "some", "such", "no",
    "nor", "not", "only", "own", "same", "so", "than", "too", "very", "just", "about", "above",
    "after", "again", "also", "as", "before", "between", "down", "from", "here", "if", "into",
    "then", "there", "through", "under", "until", "up",
];

// ---------------------------------------------------------------------------
// Analyzer Registry
// ---------------------------------------------------------------------------

/// Registry of SEO analyzers that can be run against crawled pages.
///
/// Manages a collection of [`Analyzer`] implementations and runs them
/// in parallel using rayon. The default registry includes 39 analyzers
/// covering HTTP, SEO, content, links, images, security, accessibility,
/// and AI-specific checks.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::{CrawlConfig, analyzers::AnalyzerRegistry};
///
/// let registry = AnalyzerRegistry::new(&CrawlConfig::default());
/// assert!(registry.len() > 20);
/// ```
pub struct AnalyzerRegistry {
    analyzers: Vec<Box<dyn Analyzer>>,
}

impl AnalyzerRegistry {
    /// Create a registry with all default analyzers.
    ///
    /// Registers 39 built-in analyzers covering HTTP status, redirects,
    /// canonical URLs, meta tags, headings, links, images, structured data,
    /// security, accessibility, social media, AI crawlers, and WASM patterns.
    pub fn new(_config: &CrawlConfig) -> Self {
        Self::build_registry(true, true)
    }

    /// Create a registry with the default analyzers, honouring runtime
    /// feature flags.
    ///
    /// This delegates to the same single registration site as
    /// [`AnalyzerRegistry::new`]; [`crate::FLAG_AI_ANALYZERS`] and
    /// [`crate::FLAG_WASM_ANALYZERS`] control whether the optional AI and
    /// WASM analyzer groups are included.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crawlkit_engine::analyzers::AnalyzerRegistry;
    /// use crawlkit_engine::{FeatureFlags, FLAG_AI_ANALYZERS};
    ///
    /// let mut flags = FeatureFlags::default();
    /// flags.set(FLAG_AI_ANALYZERS, false);
    /// let registry = AnalyzerRegistry::with_feature_flags(&flags);
    /// assert!(registry.len() > 20);
    /// ```
    #[cfg(feature = "full")]
    pub fn with_feature_flags(flags: &crate::FeatureFlags) -> Self {
        Self::build_registry(
            flags.get(crate::FLAG_AI_ANALYZERS),
            flags.get(crate::FLAG_WASM_ANALYZERS),
        )
    }

    /// Single registration site for all built-in analyzers.
    ///
    /// `include_ai` and `include_wasm` toggle the optional analyzer groups;
    /// every other analyzer is always registered.
    fn build_registry(include_ai: bool, include_wasm: bool) -> Self {
        let mut analyzers: Vec<Box<dyn Analyzer>> = vec![
            Box::new(HttpStatusAnalyzer::new()),
            Box::new(RedirectChainAnalyzer::new()),
            Box::new(CanonicalUrlValidator::new()),
            Box::new(HreflangValidator::new()),
            Box::new(SitemapAnalyzer::empty()),
            Box::new(RobotsTxtAnalyzer::empty()),
            Box::new(MetaTagAnalyzer::new()),
            Box::new(HeadingHierarchyAnalyzer::new()),
            Box::new(LinkAnalyzer::new()),
            Box::new(ImageAnalyzer::new()),
            Box::new(StructuredDataValidator::new()),
            Box::new(ContentQualityAnalyzer::new()),
            Box::new(WordCountAnalyzer::new()),
            Box::new(SecurityHeaderAnalyzer::new()),
            Box::new(SslCertificateValidator::empty()),
            Box::new(MobileFriendlinessChecker::new()),
            Box::new(AccessibilityAnalyzer::new()),
            Box::new(SocialMediaAnalyzer::new()),
            Box::new(EntityAnalyzer::new()),
            Box::new(EnhancedReadabilityAnalyzer::new()),
            Box::new(KeywordAnalyzer::new()),
            Box::new(EcommerceSignalsAnalyzer::new()),
            Box::new(InternationalSeoAnalyzer::new()),
            // Advanced canonical & hreflang analysis
            Box::new(crate::advanced_canonical::AdvancedCanonicalAnalyzer::new()),
            Box::new(crate::advanced_canonical::SitemapCanonicalValidator::new()),
            Box::new(crate::advanced_canonical::UrlFormatValidator::new()),
            // RDFa, Microdata, Entity linking, Shipping, Availability, and Coupon validators
            Box::new(RdfaValidator::new()),
            Box::new(MicrodataValidator::new()),
            Box::new(EntityLinkingAnalyzer::new()),
            Box::new(ShippingSchemaValidator::new()),
            Box::new(OfferAvailabilityAnalyzer::new()),
            Box::new(CouponSchemaValidator::new()),
            // Advanced canonical chain and hreflang reciprocal validators
            Box::new(CanonicalChainDetector::new()),
            Box::new(HreflangReciprocalValidator::new()),
            // Pagination, OG image, OpenSearch
            Box::new(PaginationAnalyzer::new()),
            Box::new(OpenGraphImageValidator::new()),
            Box::new(OpenSearchValidator::new()),
            // Performance: response size, TTFB, cache headers, resource count
            Box::new(ResponseSizeAnalyzer::new()),
            Box::new(TtfbAnalyzer::new()),
            Box::new(CacheHeaderAnalyzer::new()),
            Box::new(ResourceCountAnalyzer::new()),
            // Accessibility: critical resources, font size, color contrast, focus order
            Box::new(CriticalResourceAnalyzer::new()),
            Box::new(FontSizeAnalyzer::new()),
            Box::new(ColorContrastAnalyzer::new()),
            Box::new(FocusOrderAnalyzer::new()),
            // E-commerce: product variants, pricing, aggregate rating
            Box::new(ProductVariantAnalyzer::new()),
            Box::new(PricingSchemaValidator::new()),
            Box::new(AggregateRatingValidator::new()),
            // Social: Twitter player
            Box::new(TwitterPlayerValidator::new()),
            // Content: breadcrumbs, duplicate, event/review/video schema, ToC, local business
            Box::new(BreadcrumbsValidator::new()),
            Box::new(DuplicateContentDetector::new()),
            Box::new(EventSchemaValidator::new()),
            Box::new(ReviewSchemaValidator::new()),
            Box::new(VideoSchemaValidator::new()),
            Box::new(TableOfContentsAnalyzer::new()),
            Box::new(LocalBusinessSchemaValidator::new()),
            // International SEO: language, hreflang consistency, charset, robots, canonical depth, mobile viewport
            Box::new(LanguageAttributeAnalyzer::new()),
            Box::new(HreflangConsistencyAnalyzer::new()),
            Box::new(CharsetValidator::new()),
            Box::new(RobotsMetaAnalyzer::new()),
            Box::new(CanonicalDepthAnalyzer::new()),
            Box::new(MobileViewportAnalyzer::new()),
            // FAQ, HowTo, Speakable, Dataset, SpecialAnnouncement, SoftwareApplication validators
            Box::new(FaqSchemaValidator::new()),
            Box::new(HowToSchemaValidator::new()),
            Box::new(SpeakableSchemaValidator::new()),
            Box::new(DatasetSchemaValidator::new()),
            Box::new(SpecialAnnouncementSchemaValidator::new()),
            Box::new(SoftwareApplicationValidator::new()),
            // Security: HSTS preload, SRI, Permission-Policy, Cross-Origin Isolation
            Box::new(HstsPreloadAnalyzer::new()),
            Box::new(SriAnalyzer::new()),
            Box::new(PermissionPolicyAnalyzer::new()),
            Box::new(CrossOriginIsolationAnalyzer::new()),
            // Content: JSON-LD validator, meta description length, title length, thin content
            Box::new(JsonLdValidator::new()),
            Box::new(MetaDescriptionLengthAnalyzer::new()),
            Box::new(TitleLengthAnalyzer::new()),
            Box::new(ContentThinAnalyzer::new()),
        ];

        if include_ai {
            // Phase 8: AI Search Optimization Analyzers
            analyzers.push(Box::new(
                crate::ai_analyzers::AiCrawlerAccessibilityAnalyzer::new(),
            ));
            analyzers.push(Box::new(
                crate::ai_analyzers::AiContentStructureAnalyzer::new(),
            ));
            analyzers.push(Box::new(
                crate::ai_analyzers::AiCitationEligibilityAnalyzer::new(),
            ));
            analyzers.push(Box::new(crate::ai_analyzers::AiAnswerBoxAnalyzer::new()));
        }

        // Phase 8: WASM Error Detection Analyzers
        #[cfg(feature = "full")]
        if include_wasm {
            analyzers.push(Box::new(crate::wasm_analyzers::WasmPatternAnalyzer::new()));
        }
        #[cfg(not(feature = "full"))]
        let _ = include_wasm;

        Self { analyzers }
    }

    /// Create a registry with custom analyzers.
    ///
    /// Use this when you want full control over which analyzers are run,
    /// without the default set.
    pub fn with_analyzers(analyzers: Vec<Box<dyn Analyzer>>) -> Self {
        Self { analyzers }
    }

    /// Add an analyzer to the registry.
    ///
    /// Custom analyzers are appended to the end and run after all
    /// previously registered analyzers.
    pub fn register(&mut self, analyzer: Box<dyn Analyzer>) {
        self.analyzers.push(analyzer);
    }

    /// Run all analyzers on a page and collect findings.
    ///
    /// Analyzers run in parallel via rayon when the `full` feature is enabled,
    /// or sequentially under `wasm`. Returns a flat list of all [`Finding`]s
    /// from all analyzers, canonically ordered by `(code, url)` so the
    /// (potentially nondeterministic) analyzer completion order cannot
    /// affect downstream output.
    pub fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        #[cfg(feature = "full")]
        let mut findings: Vec<Finding> = {
            use rayon::prelude::*;
            self.analyzers
                .par_iter()
                .flat_map(|a| a.analyze(ctx))
                .collect()
        };
        #[cfg(not(feature = "full"))]
        let mut findings: Vec<Finding> =
            self.analyzers.iter().flat_map(|a| a.analyze(ctx)).collect();

        // Canonical ordering: stable sort by (code, url) so identical input
        // always produces identical output regardless of parallel scheduling.
        findings.sort_by(|a, b| a.code.cmp(&b.code).then_with(|| a.url.cmp(&b.url)));
        findings
    }

    /// Returns the number of registered analyzers.
    pub fn len(&self) -> usize {
        self.analyzers.len()
    }

    /// Returns true if no analyzers are registered.
    pub fn is_empty(&self) -> bool {
        self.analyzers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ExtractedImage, ExtractedLink, Heading, StructuredData};
    use crate::types::{IssueCategory, Severity};
    use std::collections::{HashMap, HashSet};
    use url::Url;

    fn default_config() -> CrawlConfig {
        CrawlConfig::default()
    }

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        }
    }

    // ---- HttpStatusAnalyzer ----

    #[test]
    fn test_http_status_200() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = HttpStatusAnalyzer::new().analyze(&ctx);
        // Should have info about status category
        assert!(findings.iter().any(|f| f.code == "HTTP006"));
    }

    #[test]
    fn test_http_status_404() {
        let page = make_page("https://example.com/missing");
        let ctx = make_ctx(&page, Some(404));
        let findings = HttpStatusAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTP004"));
    }

    #[test]
    fn test_http_status_500() {
        let page = make_page("https://example.com/error");
        let ctx = make_ctx(&page, Some(500));
        let findings = HttpStatusAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTP005"));
        assert!(findings.iter().any(|f| f.severity == Severity::Critical));
    }

    #[test]
    fn test_http_status_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = HttpStatusAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTP001"));
    }

    #[test]
    fn test_http_status_soft_404_empty_body() {
        let mut page = make_page("https://example.com/soft404");
        page.word_count = 0;
        let ctx = make_ctx(&page, Some(200));
        let findings = HttpStatusAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTP003"));
    }

    #[test]
    fn test_http_status_slow_response() {
        let page = make_page("https://example.com/slow");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: Some(Duration::from_secs(10)),
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = HttpStatusAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTP002"));
    }

    // ---- RedirectChainAnalyzer ----

    #[test]
    fn test_redirect_no_hops() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = RedirectChainAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_redirect_long_chain() {
        let hops: Vec<RedirectHop> = (0..7)
            .map(|i| RedirectHop {
                from: Url::parse(&format!("https://example.com/page{i}")).unwrap(),
                to: Url::parse(&format!("https://example.com/page{}", i + 1)).unwrap(),
                status_code: 301,
            })
            .collect();
        let page = make_page("https://example.com/page0");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &hops,
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = RedirectChainAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "REDIR001"));
    }

    #[test]
    fn test_redirect_loop() {
        let hops = vec![
            RedirectHop {
                from: Url::parse("https://example.com/a").unwrap(),
                to: Url::parse("https://example.com/b").unwrap(),
                status_code: 301,
            },
            RedirectHop {
                from: Url::parse("https://example.com/b").unwrap(),
                to: Url::parse("https://example.com/a").unwrap(),
                status_code: 301,
            },
        ];
        let page = make_page("https://example.com/a");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &hops,
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = RedirectChainAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "REDIR002"));
    }

    #[test]
    fn test_redirect_mixed_protocol() {
        let hops = vec![RedirectHop {
            from: Url::parse("http://example.com/page").unwrap(),
            to: Url::parse("https://example.com/page").unwrap(),
            status_code: 301,
        }];
        let page = make_page("http://example.com/page");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &hops,
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = RedirectChainAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "REDIR003"));
    }

    #[test]
    fn test_redirect_single_hop() {
        let hops = vec![RedirectHop {
            from: Url::parse("https://example.com/old").unwrap(),
            to: Url::parse("https://example.com/new").unwrap(),
            status_code: 301,
        }];
        let page = make_page("https://example.com/old");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &hops,
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = RedirectChainAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "REDIR004"));
    }

    // ---- CanonicalUrlValidator ----

    #[test]
    fn test_canonical_missing() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalUrlValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CANON001"));
    }

    #[test]
    fn test_canonical_self_referencing() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/page").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalUrlValidator::new().analyze(&ctx);
        // Self-referencing is fine — no mismatch finding
        assert!(!findings.iter().any(|f| f.code == "CANON003"));
    }

    #[test]
    fn test_canonical_mismatch() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/other").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalUrlValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CANON003"));
    }

    // ---- HreflangValidator ----

    #[test]
    fn test_hreflang_no_tags() {
        let page = make_page("https://example.com/en");
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hreflang_missing_x_default() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "fr".to_string(),
                url: Url::parse("https://example.com/fr").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HREF001"));
    }

    #[test]
    fn test_hreflang_invalid_locale() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "invalid-locale-code-too-long".to_string(),
                url: Url::parse("https://example.com/invalid").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "x-default".to_string(),
                url: Url::parse("https://example.com").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HREF002"));
    }

    #[test]
    fn test_hreflang_duplicate_language() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en-uk").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "x-default".to_string(),
                url: Url::parse("https://example.com").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HREF003"));
    }

    #[test]
    fn test_hreflang_valid_with_x_default() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "fr".to_string(),
                url: Url::parse("https://example.com/fr").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "x-default".to_string(),
                url: Url::parse("https://example.com").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangValidator::new().analyze(&ctx);
        // No errors for valid setup
        assert!(!findings.iter().any(|f| f.severity == Severity::Error));
    }

    // ---- SitemapAnalyzer ----

    #[test]
    fn test_sitemap_no_data() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapAnalyzer::empty().analyze(&ctx);
        // Empty-data analyzers must not emit per-page noise findings.
        assert!(findings.is_empty());
    }

    #[test]
    fn test_sitemap_url_not_found() {
        let mut known = HashSet::new();
        known.insert("https://example.com/other".to_string());
        let analyzer = SitemapAnalyzer::new(known, Vec::new());
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP002"));
    }

    #[test]
    fn test_sitemap_url_found() {
        let mut known = HashSet::new();
        known.insert("https://example.com/page".to_string());
        let analyzer = SitemapAnalyzer::new(known, Vec::new());
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SITEMAP002"));
    }

    #[test]
    fn test_sitemap_invalid_lastmod() {
        let mut known = HashSet::new();
        known.insert("https://example.com/page".to_string());
        let entries = vec![SitemapEntry {
            url: "https://example.com/page".to_string(),
            lastmod: Some("not-a-date".to_string()),
            changefreq: None,
            priority: None,
        }];
        let analyzer = SitemapAnalyzer::new(known, entries);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP003"));
    }

    #[test]
    fn test_sitemap_invalid_changefreq() {
        let mut known = HashSet::new();
        known.insert("https://example.com/page".to_string());
        let entries = vec![SitemapEntry {
            url: "https://example.com/page".to_string(),
            lastmod: None,
            changefreq: Some("sometimes".to_string()),
            priority: None,
        }];
        let analyzer = SitemapAnalyzer::new(known, entries);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP004"));
    }

    #[test]
    fn test_sitemap_invalid_priority() {
        let mut known = HashSet::new();
        known.insert("https://example.com/page".to_string());
        let entries = vec![SitemapEntry {
            url: "https://example.com/page".to_string(),
            lastmod: None,
            changefreq: None,
            priority: Some(2.5),
        }];
        let analyzer = SitemapAnalyzer::new(known, entries);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP005"));
    }

    #[test]
    fn test_sitemap_valid_metadata() {
        let mut known = HashSet::new();
        known.insert("https://example.com/page".to_string());
        let entries = vec![SitemapEntry {
            url: "https://example.com/page".to_string(),
            lastmod: Some("2024-01-15T10:30:00Z".to_string()),
            changefreq: Some("weekly".to_string()),
            priority: Some(0.8),
        }];
        let analyzer = SitemapAnalyzer::new(known, entries);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        // No errors for valid metadata
        assert!(!findings.iter().any(|f| f.severity == Severity::Error));
    }

    // ---- RobotsTxtAnalyzer ----

    #[test]
    fn test_robots_empty() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsTxtAnalyzer::empty().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_disallowed() {
        let rules = vec![RobotsRule {
            user_agent: "*".to_string(),
            disallowed_paths: vec!["/admin".to_string()],
            allowed_paths: Vec::new(),
            crawl_delay: None,
            sitemaps: Vec::new(),
        }];
        let analyzer = RobotsTxtAnalyzer::new(rules, Vec::new());
        let page = make_page("https://example.com/admin/secret");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOT002"));
    }

    #[test]
    fn test_robots_allowed() {
        let rules = vec![RobotsRule {
            user_agent: "*".to_string(),
            disallowed_paths: vec!["/admin".to_string()],
            allowed_paths: Vec::new(),
            crawl_delay: None,
            sitemaps: Vec::new(),
        }];
        let analyzer = RobotsTxtAnalyzer::new(rules, Vec::new());
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ROBOT002"));
    }

    #[test]
    fn test_robots_allow_override() {
        let rules = vec![RobotsRule {
            user_agent: "*".to_string(),
            disallowed_paths: vec!["/admin".to_string()],
            allowed_paths: vec!["/admin/public".to_string()],
            crawl_delay: None,
            sitemaps: Vec::new(),
        }];
        let analyzer = RobotsTxtAnalyzer::new(rules, Vec::new());
        let page = make_page("https://example.com/admin/public/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOT001"));
    }

    #[test]
    fn test_robots_high_crawl_delay() {
        let rules = vec![RobotsRule {
            user_agent: "*".to_string(),
            disallowed_paths: Vec::new(),
            allowed_paths: Vec::new(),
            crawl_delay: Some(20.0),
            sitemaps: Vec::new(),
        }];
        let analyzer = RobotsTxtAnalyzer::new(rules, Vec::new());
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOT003"));
    }

    #[test]
    fn test_robots_invalid_sitemap_url() {
        let rules = vec![RobotsRule {
            user_agent: "*".to_string(),
            disallowed_paths: Vec::new(),
            allowed_paths: Vec::new(),
            crawl_delay: None,
            sitemaps: Vec::new(),
        }];
        let analyzer = RobotsTxtAnalyzer::new(rules, vec!["not-a-url".to_string()]);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOT004"));
    }

    // ---- MetaTagAnalyzer ----

    #[test]
    fn test_meta_missing_title() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META001"));
    }

    #[test]
    fn test_meta_title_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Hi".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META002"));
    }

    #[test]
    fn test_meta_title_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(80));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META003"));
    }

    #[test]
    fn test_meta_title_just_right() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(45));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "META002"));
        assert!(!findings.iter().any(|f| f.code == "META003"));
    }

    #[test]
    fn test_meta_missing_description() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META004"));
    }

    #[test]
    fn test_meta_description_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("Short".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META005"));
    }

    #[test]
    fn test_meta_description_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(200));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META006"));
    }

    #[test]
    fn test_meta_missing_og_tags() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        // Should flag og:title, og:description, og:image, og:url, og:type
        let og_codes: Vec<&str> = findings
            .iter()
            .filter(|f| f.code == "SOCIAL006")
            .map(|f| f.title.as_str())
            .collect();
        assert!(!og_codes.is_empty());
    }

    #[test]
    fn test_meta_missing_twitter_tags() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        // Should flag twitter:card, twitter:title, twitter:image
        let tw_count = findings.iter().filter(|f| f.code == "SOCIAL007").count();
        assert!(tw_count >= 1);
    }

    #[test]
    fn test_meta_missing_viewport() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META009"));
    }

    #[test]
    fn test_meta_complete_tags() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Perfect Title for SEO".to_string());
        page.meta.description = Some("A".repeat(140));
        page.meta.viewport = Some("width=device-width".to_string());
        page.meta.og.title = Some("OG Title".to_string());
        page.meta.og.image = Some("https://example.com/img.png".to_string());
        page.meta.og.url = Some("https://example.com".to_string());
        page.meta.og.r#type = Some("website".to_string());
        page.meta.twitter.card = Some("summary_large_image".to_string());
        page.meta.twitter.title = Some("Twitter Title".to_string());
        page.meta.twitter.image = Some("https://example.com/tw.png".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx);
        // Should have no errors or warnings about missing tags
        assert!(!findings
            .iter()
            .any(|f| f.code == "META001" || f.code == "META004"));
    }

    // ---- HeadingHierarchyAnalyzer ----

    #[test]
    fn test_heading_no_headings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HEAD001"));
    }

    #[test]
    fn test_heading_missing_h1() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 2,
                text: "Section".to_string(),
                length: 7,
            },
            Heading {
                level: 3,
                text: "Sub".to_string(),
                length: 3,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HEAD002"));
    }

    #[test]
    fn test_heading_multiple_h1() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "First".to_string(),
                length: 5,
            },
            Heading {
                level: 1,
                text: "Second".to_string(),
                length: 6,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HEAD003"));
    }

    #[test]
    fn test_heading_skipped_level() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "Title".to_string(),
                length: 5,
            },
            Heading {
                level: 3,
                text: "Skipped H2".to_string(),
                length: 10,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HEAD004"));
    }

    #[test]
    fn test_heading_valid_hierarchy() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "Main".to_string(),
                length: 4,
            },
            Heading {
                level: 2,
                text: "Section".to_string(),
                length: 7,
            },
            Heading {
                level: 2,
                text: "Section 2".to_string(),
                length: 9,
            },
            Heading {
                level: 3,
                text: "Sub".to_string(),
                length: 3,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "HEAD004"));
    }

    #[test]
    fn test_heading_deep_hierarchy() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "H1".to_string(),
                length: 2,
            },
            Heading {
                level: 2,
                text: "H2".to_string(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3".to_string(),
                length: 2,
            },
            Heading {
                level: 4,
                text: "H4".to_string(),
                length: 2,
            },
            Heading {
                level: 5,
                text: "H5".to_string(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HEAD005"));
    }

    // ---- AnalyzerRegistry ----

    #[test]
    fn test_registry_default() {
        let config = default_config();
        let registry = AnalyzerRegistry::new(&config);
        assert_eq!(registry.len(), 81);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_analyze() {
        let config = default_config();
        let registry = AnalyzerRegistry::new(&config);
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Good Title Here for SEO".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = registry.analyze(&ctx);
        // Should produce findings from multiple analyzers
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_registry_custom() {
        struct DummyAnalyzer;
        impl Analyzer for DummyAnalyzer {
            fn name(&self) -> &str {
                "dummy"
            }
            fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
                vec![Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Custom("test".to_string()),
                    code: "DUMMY001".to_string(),
                    title: "Dummy finding".to_string(),
                    description: "Test".to_string(),
                    url: String::new(),
                    recommendation: "None".to_string(),
                }]
            }
        }
        let mut registry = AnalyzerRegistry::with_analyzers(Vec::new());
        registry.register(Box::new(DummyAnalyzer));
        assert_eq!(registry.len(), 1);

        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = registry.analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "DUMMY001");
    }

    #[test]
    fn test_registry_analyze_sorts_findings_canonically() {
        struct UnsortedAnalyzer;
        impl Analyzer for UnsortedAnalyzer {
            fn name(&self) -> &str {
                "unsorted"
            }
            fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
                let finding = |code: &str, url: &str| Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Custom("test".to_string()),
                    code: code.to_string(),
                    title: format!("finding {code}"),
                    description: "Test".to_string(),
                    url: url.to_string(),
                    recommendation: "None".to_string(),
                };
                // Deliberately unsorted by (code, url).
                vec![
                    finding("ZZZ001", ctx.page.url.as_str()),
                    finding("AAA002", "https://example.com/b"),
                    finding("AAA002", "https://example.com/a"),
                    finding("AAA001", ctx.page.url.as_str()),
                ]
            }
        }
        let registry = AnalyzerRegistry::with_analyzers(vec![Box::new(UnsortedAnalyzer)]);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = registry.analyze(&ctx);

        let keys: Vec<(String, String)> = findings
            .iter()
            .map(|f| (f.code.clone(), f.url.clone()))
            .collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys, "findings must be ordered by (code, url)");
    }

    #[test]
    fn test_registry_analyze_repeated_calls_identical_order() {
        let config = default_config();
        let registry = AnalyzerRegistry::new(&config);
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let first: Vec<(String, String)> = registry
            .analyze(&ctx)
            .iter()
            .map(|f| (f.code.clone(), f.title.clone()))
            .collect();
        let second: Vec<(String, String)> = registry
            .analyze(&ctx)
            .iter()
            .map(|f| (f.code.clone(), f.title.clone()))
            .collect();
        assert_eq!(first, second);
    }

    // ---- Edge cases for locale validation ----

    #[test]
    fn test_valid_locales() {
        assert!(HreflangValidator::is_valid_locale("en"));
        assert!(HreflangValidator::is_valid_locale("fr"));
        assert!(HreflangValidator::is_valid_locale("de"));
        assert!(HreflangValidator::is_valid_locale("en-US"));
        assert!(HreflangValidator::is_valid_locale("fr-CA"));
        assert!(HreflangValidator::is_valid_locale("zh-CN"));
        assert!(HreflangValidator::is_valid_locale("x-default"));
    }

    #[test]
    fn test_invalid_locales() {
        assert!(!HreflangValidator::is_valid_locale("e"));
        assert!(!HreflangValidator::is_valid_locale("english"));
        assert!(!HreflangValidator::is_valid_locale("en-us-extra"));
        assert!(!HreflangValidator::is_valid_locale("123"));
    }

    // ---- Edge cases for soft 404 detection ----

    #[test]
    fn test_soft_404_indicators() {
        assert!(HttpStatusAnalyzer::is_soft_404(
            "<html><body>Page Not Found</body></html>"
        ));
        assert!(HttpStatusAnalyzer::is_soft_404(
            "Error 404 — The page you requested does not exist."
        ));
        assert!(HttpStatusAnalyzer::is_soft_404(
            "Sorry, we couldn't find the page you're looking for."
        ));
        assert!(!HttpStatusAnalyzer::is_soft_404(
            "<html><body>Welcome to our site</body></html>"
        ));
    }

    // ---- Edge cases for robots.txt path matching ----

    #[test]
    fn test_robots_path_matching() {
        assert!(RobotsTxtAnalyzer::is_disallowed(
            "/admin/secret",
            &["/admin".to_string()]
        ));
        assert!(!RobotsTxtAnalyzer::is_disallowed(
            "/page",
            &["/admin".to_string()]
        ));
        assert!(RobotsTxtAnalyzer::is_allowed(
            "/admin/public",
            &["/admin/public".to_string()]
        ));
        assert!(!RobotsTxtAnalyzer::is_allowed(
            "/admin/secret",
            &["/admin/public".to_string()]
        ));
    }

    // ---- Finding struct ----

    #[test]
    fn test_finding_creation() {
        let finding = Finding {
            severity: Severity::Warning,
            category: IssueCategory::Seo,
            code: "TEST001".to_string(),
            title: "Test finding".to_string(),
            description: "A test finding for unit tests".to_string(),
            url: "https://example.com".to_string(),
            recommendation: "Fix it".to_string(),
        };
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.category, IssueCategory::Seo);
    }

    // ---- Sitemap edge cases ----

    #[test]
    fn test_sitemap_valid_lastmod_formats() {
        assert!(SitemapAnalyzer::is_valid_lastmod("2024-01-15T10:30:00Z"));
        assert!(SitemapAnalyzer::is_valid_lastmod("2024-01-15"));
        assert!(!SitemapAnalyzer::is_valid_lastmod("lastweek"));
    }

    #[test]
    fn test_sitemap_valid_changefreq() {
        assert!(SitemapAnalyzer::is_valid_changefreq("daily"));
        assert!(SitemapAnalyzer::is_valid_changefreq("weekly"));
        assert!(SitemapAnalyzer::is_valid_changefreq("never"));
        assert!(!SitemapAnalyzer::is_valid_changefreq("sometimes"));
        assert!(!SitemapAnalyzer::is_valid_changefreq("often"));
    }

    #[test]
    fn test_sitemap_valid_priority() {
        assert!(SitemapAnalyzer::is_valid_priority(0.0));
        assert!(SitemapAnalyzer::is_valid_priority(0.5));
        assert!(SitemapAnalyzer::is_valid_priority(1.0));
        assert!(!SitemapAnalyzer::is_valid_priority(-0.1));
        assert!(!SitemapAnalyzer::is_valid_priority(1.1));
    }

    // ---- Full analysis integration ----

    #[test]
    fn test_full_analysis_minimal_page() {
        let config = default_config();
        let registry = AnalyzerRegistry::new(&config);
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = registry.analyze(&ctx);

        // A minimal page should produce several findings
        let codes: Vec<&str> = findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"META001")); // missing title
        assert!(codes.contains(&"META004")); // missing description
        assert!(codes.contains(&"CANON001")); // missing canonical
        assert!(codes.contains(&"HEAD001")); // no headings
    }

    #[test]
    fn test_full_analysis_well_optimized_page() {
        let config = default_config();
        let registry = AnalyzerRegistry::new(&config);

        let mut page = make_page("https://example.com/page");
        page.meta.title = Some("Optimized Page Title for Search".to_string());
        page.meta.description = Some("A".repeat(145));
        page.meta.canonical = Some(Url::parse("https://example.com/page").unwrap());
        page.meta.viewport = Some("width=device-width".to_string());
        page.meta.og.title = Some("OG Title".to_string());
        page.meta.og.image = Some("https://example.com/img.png".to_string());
        page.meta.og.url = Some("https://example.com/page".to_string());
        page.meta.og.r#type = Some("article".to_string());
        page.meta.twitter.card = Some("summary_large_image".to_string());
        page.meta.twitter.title = Some("Twitter Title".to_string());
        page.meta.twitter.image = Some("https://example.com/tw.png".to_string());
        page.headings = vec![
            Heading {
                level: 1,
                text: "Main Topic".to_string(),
                length: 10,
            },
            Heading {
                level: 2,
                text: "Section".to_string(),
                length: 7,
            },
        ];
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        page.has_main_landmark = true;
        page.has_nav_landmark = true;
        page.has_skip_link = true;
        page.word_count = 500;

        let ctx = make_ctx(&page, Some(200));
        let findings = registry.analyze(&ctx);

        // Should have few/no errors
        let errors: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.severity == Severity::Error || f.severity == Severity::Critical)
            .collect();
        assert!(
            errors.is_empty(),
            "Well-optimized page should have no errors: {:?}",
            errors.iter().map(|f| &f.code).collect::<Vec<_>>()
        );
    }

    // =========================================================================
    // LinkAnalyzer tests
    // =========================================================================

    #[test]
    fn test_link_analyzer_no_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LINK001"));
        let link001 = findings.iter().find(|f| f.code == "LINK001").unwrap();
        assert!(link001.description.contains("Internal: 0"));
        assert!(link001.description.contains("External: 0"));
    }

    #[test]
    fn test_link_analyzer_internal_external_counts() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/about".to_string(),
                text: "About".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/contact".to_string(),
                text: "Contact".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "https://external.com".to_string(),
                text: "External".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx);
        let link001 = findings.iter().find(|f| f.code == "LINK001").unwrap();
        assert!(link001.description.contains("Internal: 2"));
        assert!(link001.description.contains("External: 1"));
    }

    #[test]
    fn test_link_analyzer_nofollow_detection() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://external.com".to_string(),
            text: "Nofollow link".to_string(),
            rel: vec!["nofollow".to_string()],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LINK003"));
    }

    #[test]
    fn test_link_analyzer_empty_anchor_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: String::new(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LINK004"));
    }

    #[test]
    fn test_link_analyzer_short_anchor_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "Go".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LINK005"));
    }

    #[test]
    fn test_link_analyzer_broken_page_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/broken".to_string(),
            text: "Broken link".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(404));
        let findings = LinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LINK002"));
    }

    #[test]
    fn test_link_analyzer_orphan_page() {
        let mut inbound = HashMap::new();
        inbound.insert("https://example.com/orphan".to_string(), 0);
        let analyzer = LinkAnalyzer::with_inbound_links(inbound);
        let page = make_page("https://example.com/orphan");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LINK006"));
    }

    #[test]
    fn test_link_analyzer_not_orphan() {
        let mut inbound = HashMap::new();
        inbound.insert("https://example.com/page".to_string(), 3);
        let analyzer = LinkAnalyzer::with_inbound_links(inbound);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LINK006"));
    }

    #[test]
    fn test_link_analyzer_no_nofollow_when_absent() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "Go".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LINK003"));
    }

    // =========================================================================
    // ImageAnalyzer tests
    // =========================================================================

    #[test]
    fn test_image_analyzer_no_images() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_image_analyzer_missing_alt_text() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/img.png".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: false,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "IMG001"));
    }

    #[test]
    fn test_image_analyzer_with_alt_text() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/img.png".to_string(),
            alt: "A photo".to_string(),
            width: Some(100),
            height: Some(200),
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "IMG001"));
    }

    #[test]
    fn test_image_analyzer_lazy_loading() {
        let mut page = make_page("https://example.com");
        page.images = vec![
            ExtractedImage {
                src: "/a.png".to_string(),
                alt: "A".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: true,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/b.png".to_string(),
                alt: "B".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: true,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/c.png".to_string(),
                alt: "C".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "IMG003"));
        let img003 = findings.iter().find(|f| f.code == "IMG003").unwrap();
        assert!(img003.description.contains("2 of 3"));
    }

    #[test]
    fn test_image_analyzer_missing_dimensions() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/no-dims.png".to_string(),
            alt: "No dims".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "IMG004"));
    }

    #[test]
    fn test_image_analyzer_detect_format() {
        assert_eq!(
            ImageAnalyzer::detect_format("photo.jpg"),
            Some("jpeg".into())
        );
        assert_eq!(
            ImageAnalyzer::detect_format("photo.jpeg"),
            Some("jpeg".into())
        );
        assert_eq!(ImageAnalyzer::detect_format("pic.png"), Some("png".into()));
        assert_eq!(ImageAnalyzer::detect_format("anim.gif"), Some("gif".into()));
        assert_eq!(
            ImageAnalyzer::detect_format("modern.webp"),
            Some("webp".into())
        );
        assert_eq!(
            ImageAnalyzer::detect_format("new.avif"),
            Some("avif".into())
        );
        assert_eq!(ImageAnalyzer::detect_format("icon.svg"), Some("svg".into()));
        assert_eq!(
            ImageAnalyzer::detect_format("photo.jpg?v=1"),
            Some("jpeg".into())
        );
        assert_eq!(ImageAnalyzer::detect_format("no-ext"), None);
    }

    // =========================================================================
    // StructuredDataValidator tests
    // =========================================================================

    #[test]
    fn test_structured_data_no_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = StructuredDataValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SD001"));
    }

    #[test]
    fn test_structured_data_missing_context() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: None,
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "Test"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = StructuredDataValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SD002"));
    }

    #[test]
    fn test_structured_data_wrong_context() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://example.com/schema".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = StructuredDataValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SD003"));
    }

    #[test]
    fn test_structured_data_missing_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: None,
            data: serde_json::json!({"@context": "https://schema.org"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = StructuredDataValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SD004"));
    }

    #[test]
    fn test_structured_data_unknown_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("CustomWidget".to_string()),
            data: serde_json::json!({"@context": "https://schema.org", "@type": "CustomWidget"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = StructuredDataValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SD005"));
    }

    #[test]
    fn test_structured_data_missing_required_properties() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = StructuredDataValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SD006"));
        let sd006 = findings.iter().find(|f| f.code == "SD006").unwrap();
        assert!(sd006.description.contains("headline"));
        assert!(sd006.description.contains("author"));
    }

    #[test]
    fn test_structured_data_valid_article() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": "Test Article",
                "author": "John Doe"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = StructuredDataValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SD006"));
    }

    #[test]
    fn test_structured_data_valid_product() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = StructuredDataValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SD006"));
    }

    #[test]
    fn test_structured_data_schema_org_context() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("schema.org".to_string()),
            r#type: Some("WebSite".to_string()),
            data: serde_json::json!({
                "@context": "schema.org",
                "@type": "WebSite",
                "name": "My Site"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = StructuredDataValidator::new().analyze(&ctx);
        // "schema.org" without https is accepted as valid
        assert!(!findings.iter().any(|f| f.code == "SD003"));
    }

    // =========================================================================
    // ContentQualityAnalyzer tests
    // =========================================================================

    #[test]
    fn test_content_quality_empty_page() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentQualityAnalyzer::new().analyze(&ctx);
        // Should flag zero content
        assert!(findings.iter().any(|f| f.code == "CQ003"));
    }

    #[test]
    fn test_content_quality_thin_content() {
        let mut page = make_page("https://example.com");
        page.word_count = 150;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentQualityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CQ004"));
    }

    #[test]
    fn test_content_quality_long_form() {
        let mut page = make_page("https://example.com");
        page.word_count = 5000;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentQualityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CQ005"));
    }

    #[test]
    fn test_content_quality_readability_score() {
        let mut page = make_page("https://example.com");
        page.word_count = 500;
        page.headings = vec![Heading {
            level: 1,
            text: "This is a simple heading for testing readability scores in content analysis"
                .to_string(),
            length: 66,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentQualityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CQ001"));
    }

    #[test]
    fn test_content_quality_keyword_density() {
        let mut page = make_page("https://example.com");
        page.word_count = 500;
        page.headings = vec![
            Heading {
                level: 1,
                text: "Rust Programming Language Tutorial".to_string(),
                length: 33,
            },
            Heading {
                level: 2,
                text: "Rust Basics for Beginners".to_string(),
                length: 24,
            },
            Heading {
                level: 2,
                text: "Advanced Rust Programming".to_string(),
                length: 24,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentQualityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CQ002"));
        let cq002 = findings.iter().find(|f| f.code == "CQ002").unwrap();
        assert!(cq002.description.contains("rust"));
    }

    #[test]
    fn test_content_quality_syllable_counting() {
        assert_eq!(count_syllables("cat"), 1);
        assert_eq!(count_syllables("hello"), 2);
        assert_eq!(count_syllables("beautiful"), 3);
        assert_eq!(count_syllables("a"), 1);
        assert_eq!(count_syllables(""), 0);
    }

    /// Direct fixtures for flesch_kincaid_grade — hand-computed from the
    /// formula 0.39*(w/s) + 11.8*(syl/w) - 15.59. Added after the
    /// cargo-mutants baseline showed arithmetic-operator mutants surviving
    /// in these functions (53.6% kill score).
    #[test]
    fn test_flesch_kincaid_grade_fixtures() {
        let close = |a: f64, b: f64| assert!((a - b).abs() < 0.01, "{a} vs {b}");

        // Degenerate inputs short-circuit to 0.0
        assert_eq!(flesch_kincaid_grade(0, 0, 0), 0.0);
        assert_eq!(flesch_kincaid_grade(100, 0, 50), 0.0);
        assert_eq!(flesch_kincaid_grade(0, 5, 50), 0.0);

        // 100 words, 5 sentences, 150 syllables:
        // 0.39*20 + 11.8*1.5 - 15.59 = 7.8 + 17.7 - 15.59 = 9.91
        close(flesch_kincaid_grade(100, 5, 150), 9.91);

        // 30 words, 2 sentences, 45 syllables:
        // 0.39*15 + 11.8*1.5 - 15.59 = 5.85 + 17.7 - 15.59 = 7.96
        close(flesch_kincaid_grade(30, 2, 45), 7.96);

        // Monosyllabic short sentences: 10 words, 1 sentence, 10 syllables:
        // 3.9 + 11.8 - 15.59 = 0.11
        close(flesch_kincaid_grade(10, 1, 10), 0.11);
    }

    /// Direct fixtures for flesch_reading_ease — hand-computed from
    /// 206.835 - 1.015*(w/s) - 84.6*(syl/w), clamped to [0, 100].
    #[test]
    fn test_flesch_reading_ease_fixtures() {
        let close = |a: f64, b: f64| assert!((a - b).abs() < 0.01, "{a} vs {b}");

        assert_eq!(flesch_reading_ease(0, 0, 0), 0.0);

        // 100 words, 5 sentences, 150 syllables:
        // 206.835 - 20.3 - 126.9 = 59.635 (standard difficulty)
        close(flesch_reading_ease(100, 5, 150), 59.635);

        // Easy monosyllabic prose: 60 words, 2 sentences, 60 syllables:
        // 206.835 - 30.45 - 84.6 = 91.785
        close(flesch_reading_ease(60, 2, 60), 91.785);

        // Clamp at floor: 2 words, 1 sentence, 12 syllables:
        // 206.835 - 2.03 - 507.6 = -302.795 → 0.0
        assert_eq!(flesch_reading_ease(2, 1, 12), 0.0);

        // Clamp at ceiling: 50 words, 50 sentences, 50 syllables:
        // 206.835 - 1.015 - 84.6 = 121.22 → 100.0
        assert_eq!(flesch_reading_ease(50, 50, 50), 100.0);
    }

    #[test]
    fn test_content_quality_sentence_counting() {
        assert_eq!(count_sentences("Hello world."), 1);
        assert_eq!(
            count_sentences("First sentence. Second sentence! Third?"),
            3
        );
        assert_eq!(count_sentences(""), 1);
        assert_eq!(count_sentences("   "), 1);
    }

    #[test]
    fn test_content_quality_flesch_kincaid() {
        // Simple text should score higher
        let score = flesch_reading_ease(100, 5, 150);
        assert!(score > 50.0);

        // Complex text should score lower
        let score = flesch_reading_ease(100, 20, 300);
        assert!(score < 50.0);

        // Zero words should return 0
        assert_eq!(flesch_reading_ease(0, 0, 0), 0.0);
    }

    // =========================================================================
    // WordCountAnalyzer tests
    // =========================================================================

    #[test]
    fn test_word_count_analyzer_empty() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = WordCountAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WC001"));
        assert!(findings.iter().any(|f| f.code == "WC002"));
    }

    #[test]
    fn test_word_count_analyzer_with_content() {
        let mut page = make_page("https://example.com");
        page.word_count = 150;
        page.headings = vec![Heading {
            level: 1,
            text: "A page with some words".to_string(),
            length: 22,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WordCountAnalyzer::new().analyze(&ctx);
        let wc001 = findings.iter().find(|f| f.code == "WC001").unwrap();
        assert!(wc001.description.contains("Words: 150"));
    }

    #[test]
    fn test_word_count_analyzer_very_low() {
        let mut page = make_page("https://example.com");
        page.word_count = 50;
        page.headings = vec![Heading {
            level: 1,
            text: "Short".to_string(),
            length: 5,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WordCountAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WC003"));
    }

    #[test]
    fn test_word_count_analyzer_long_sentences() {
        let mut page = make_page("https://example.com");
        // 100 words in 2 sentences = 50 avg → fires (>25)
        page.word_count = 100;
        page.sentence_count = 2;
        let ctx = make_ctx(&page, Some(200));
        let findings = WordCountAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WC004"));
    }

    /// Regression (kingstonpeptides.com dogfood): the old implementation
    /// divided full-page words by heading-only sentence counts, reporting
    /// absurd averages (147-190 "words/sentence") on every page. With a
    /// consistent corpus, a chrome-heavy 150-word page across ~12 short
    /// sentences averages ~12.5 and must NOT fire.
    #[test]
    fn test_word_count_analyzer_consistent_corpus_no_false_positive() {
        let mut page = make_page("https://example.com");
        page.word_count = 150;
        page.sentence_count = 12;
        let ctx = make_ctx(&page, Some(200));
        let findings = WordCountAnalyzer::new().analyze(&ctx);
        assert!(
            findings.iter().all(|f| f.code != "WC004"),
            "150 words / 12 sentences (avg 12.5) must not be a long-sentence page: {:?}",
            findings.iter().map(|f| f.code.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_word_count_analyzer_zero_sentences_treated_as_one() {
        let mut page = make_page("https://example.com");
        // Label-style page: words but no sentence terminators → one
        // implicit sentence, so the average equals the word count.
        page.word_count = 30;
        page.sentence_count = 0;
        let ctx = make_ctx(&page, Some(200));
        let findings = WordCountAnalyzer::new().analyze(&ctx);
        let wc001 = findings
            .iter()
            .find(|f| f.code == "WC001")
            .expect("stats finding");
        assert!(wc001.description.contains("Avg words/sentence: 30.0"));
    }

    #[test]
    fn test_word_count_analyzer_normal_content() {
        let mut page = make_page("https://example.com");
        page.word_count = 500;
        page.headings = vec![
            Heading {
                level: 1,
                text: "Main Title".to_string(),
                length: 10,
            },
            Heading {
                level: 2,
                text: "Section One".to_string(),
                length: 11,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = WordCountAnalyzer::new().analyze(&ctx);
        // Should have WC001 but not WC002 or WC003
        assert!(findings.iter().any(|f| f.code == "WC001"));
        assert!(!findings.iter().any(|f| f.code == "WC002"));
        assert!(!findings.iter().any(|f| f.code == "WC003"));
    }

    // =========================================================================
    // SecurityHeaderAnalyzer tests
    // =========================================================================

    #[test]
    fn test_security_headers_none_present() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        // Should flag missing CSP, HSTS, XFO, XCTO
        assert!(findings.iter().any(|f| f.code == "SEC001"));
        assert!(findings.iter().any(|f| f.code == "SEC002"));
        assert!(findings.iter().any(|f| f.code == "SEC003"));
        assert!(findings.iter().any(|f| f.code == "SEC005"));
        // Should have a score
        assert!(findings.iter().any(|f| f.code == "SEC012"));
    }

    #[test]
    fn test_security_headers_all_present() {
        let headers = vec![
            (
                "Content-Security-Policy".to_string(),
                "default-src 'self'".to_string(),
            ),
            (
                "Strict-Transport-Security".to_string(),
                "max-age=31536000; includeSubDomains; preload".to_string(),
            ),
            ("X-Frame-Options".to_string(), "DENY".to_string()),
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
            (
                "Referrer-Policy".to_string(),
                "strict-origin-when-cross-origin".to_string(),
            ),
            (
                "Permissions-Policy".to_string(),
                "camera=(), microphone=(), geolocation=()".to_string(),
            ),
            (
                "Cross-Origin-Embedder-Policy".to_string(),
                "require-corp".to_string(),
            ),
            (
                "Cross-Origin-Opener-Policy".to_string(),
                "same-origin".to_string(),
            ),
            (
                "Cross-Origin-Resource-Policy".to_string(),
                "same-origin".to_string(),
            ),
        ];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        // Should not flag any missing headers
        assert!(!findings.iter().any(|f| f.code == "SEC001"));
        assert!(!findings.iter().any(|f| f.code == "SEC002"));
        assert!(!findings.iter().any(|f| f.code == "SEC003"));
        assert!(!findings.iter().any(|f| f.code == "SEC005"));
        // Score should be high
        let score_finding = findings.iter().find(|f| f.code == "SEC012").unwrap();
        assert!(score_finding.description.contains("100/100"));
    }

    #[test]
    fn test_security_headers_invalid_xfo() {
        let headers = vec![(
            "X-Frame-Options".to_string(),
            "ALLOW-FROM https://example.com".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SEC004"));
    }

    #[test]
    fn test_security_headers_invalid_xcto() {
        let headers = vec![("X-Content-Type-Options".to_string(), "sniff".to_string())];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SEC006"));
    }

    #[test]
    fn test_security_headers_xfo_sameorigin() {
        let headers = vec![("X-Frame-Options".to_string(), "SAMEORIGIN".to_string())];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        // SAMEORIGIN is valid, should not flag SEC003 or SEC004
        assert!(!findings.iter().any(|f| f.code == "SEC003"));
        assert!(!findings.iter().any(|f| f.code == "SEC004"));
    }

    #[test]
    fn test_security_headers_hsts_weak_max_age() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=300".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SEC014"));
    }

    #[test]
    fn test_security_headers_hsts_missing_max_age() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "includeSubDomains".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SEC014"));
    }

    #[test]
    fn test_security_headers_invalid_csp() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "invalid-value".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SEC013"));
    }

    #[test]
    fn test_security_headers_valid_csp() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "default-src 'self'; script-src 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SEC013"));
    }

    #[test]
    fn test_security_headers_case_insensitive_lookup() {
        let headers = vec![
            (
                "content-security-policy".to_string(),
                "default-src 'self'".to_string(),
            ),
            (
                "strict-transport-security".to_string(),
                "max-age=63072000".to_string(),
            ),
            ("x-frame-options".to_string(), "DENY".to_string()),
            ("x-content-type-options".to_string(), "nosniff".to_string()),
        ];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        // Lowercase header names should still be found
        assert!(!findings.iter().any(|f| f.code == "SEC001"));
        assert!(!findings.iter().any(|f| f.code == "SEC002"));
        assert!(!findings.iter().any(|f| f.code == "SEC003"));
        assert!(!findings.iter().any(|f| f.code == "SEC005"));
    }

    #[test]
    fn test_security_headers_score_decreases_with_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
        let score_finding = findings.iter().find(|f| f.code == "SEC012").unwrap();
        // With multiple missing headers, score should be well below 100
        assert!(score_finding.description.contains("/100"));
        // Parse the score
        let score_str = score_finding
            .description
            .split_whitespace()
            .find(|s| s.contains("/100"))
            .unwrap();
        let score_num: u32 = score_str.split('/').next().unwrap().parse().unwrap();
        assert!(score_num < 90);
    }

    // =========================================================================
    // SslCertificateValidator tests
    // =========================================================================

    #[test]
    fn test_ssl_no_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SslCertificateValidator::empty().analyze(&ctx);
        // Empty-data analyzers must not emit per-page noise findings.
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ssl_expired_certificate() {
        let cert = SslCertificateInfo {
            subject: Some("example.com".to_string()),
            issuer: Some("Let's Encrypt".to_string()),
            san_entries: vec!["example.com".to_string()],
            not_before: Some("2024-01-01T00:00:00Z".to_string()),
            not_after: Some("2025-01-01T00:00:00Z".to_string()),
            is_valid_chain: true,
            is_self_signed: false,
            signature_algorithm: Some("SHA256withRSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SSL001"));
    }

    #[test]
    fn test_ssl_expiring_soon() {
        let cert = SslCertificateInfo {
            subject: Some("example.com".to_string()),
            issuer: Some("Let's Encrypt".to_string()),
            san_entries: vec!["example.com".to_string()],
            not_before: Some("2025-06-01T00:00:00Z".to_string()),
            not_after: Some("2025-08-01T00:00:00Z".to_string()),
            is_valid_chain: true,
            is_self_signed: false,
            signature_algorithm: Some("SHA256withRSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        // Should NOT be expired but should be flagged as expiring soon (or not, depending on
        // the current date relative to 2025-08-01)
        let has_expiry_finding = findings
            .iter()
            .any(|f| f.code == "SSL001" || f.code == "SSL002");
        // Given today is 2026, this cert IS expired
        assert!(has_expiry_finding);
    }

    #[test]
    fn test_ssl_valid_certificate() {
        let cert = SslCertificateInfo {
            subject: Some("example.com".to_string()),
            issuer: Some("Let's Encrypt".to_string()),
            san_entries: vec!["example.com".to_string(), "www.example.com".to_string()],
            not_before: Some("2025-01-01T00:00:00Z".to_string()),
            not_after: Some("2027-01-01T00:00:00Z".to_string()),
            is_valid_chain: true,
            is_self_signed: false,
            signature_algorithm: Some("SHA256withRSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        // Should not have critical or error findings
        assert!(!findings.iter().any(|f| f.severity == Severity::Critical));
        assert!(!findings.iter().any(|f| f.severity == Severity::Error));
        // Should have the summary finding
        assert!(findings.iter().any(|f| f.code == "SSL008"));
    }

    #[test]
    fn test_ssl_invalid_chain() {
        let cert = SslCertificateInfo {
            subject: Some("example.com".to_string()),
            issuer: Some("Unknown CA".to_string()),
            san_entries: vec!["example.com".to_string()],
            not_before: Some("2025-01-01T00:00:00Z".to_string()),
            not_after: Some("2027-01-01T00:00:00Z".to_string()),
            is_valid_chain: false,
            is_self_signed: false,
            signature_algorithm: Some("SHA256withRSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SSL003"));
    }

    #[test]
    fn test_ssl_self_signed() {
        let cert = SslCertificateInfo {
            subject: Some("localhost".to_string()),
            issuer: Some("localhost".to_string()),
            san_entries: vec!["localhost".to_string()],
            not_before: Some("2025-01-01T00:00:00Z".to_string()),
            not_after: Some("2027-01-01T00:00:00Z".to_string()),
            is_valid_chain: true,
            is_self_signed: true,
            signature_algorithm: Some("SHA256withRSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://localhost");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SSL006"));
    }

    #[test]
    fn test_ssl_subject_mismatch() {
        let cert = SslCertificateInfo {
            subject: Some("wrong.example.com".to_string()),
            issuer: Some("Let's Encrypt".to_string()),
            san_entries: vec!["wrong.example.com".to_string()],
            not_before: Some("2025-01-01T00:00:00Z".to_string()),
            not_after: Some("2027-01-01T00:00:00Z".to_string()),
            is_valid_chain: true,
            is_self_signed: false,
            signature_algorithm: Some("SHA256withRSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SSL004"));
    }

    #[test]
    fn test_ssl_wildcard_match() {
        let cert = SslCertificateInfo {
            subject: Some("*.example.com".to_string()),
            issuer: Some("Let's Encrypt".to_string()),
            san_entries: vec!["*.example.com".to_string()],
            not_before: Some("2025-01-01T00:00:00Z".to_string()),
            not_after: Some("2027-01-01T00:00:00Z".to_string()),
            is_valid_chain: true,
            is_self_signed: false,
            signature_algorithm: Some("SHA256withRSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://sub.example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SSL004"));
    }

    #[test]
    fn test_ssl_weak_algorithm() {
        let cert = SslCertificateInfo {
            subject: Some("example.com".to_string()),
            issuer: Some("Let's Encrypt".to_string()),
            san_entries: vec!["example.com".to_string()],
            not_before: Some("2025-01-01T00:00:00Z".to_string()),
            not_after: Some("2027-01-01T00:00:00Z".to_string()),
            is_valid_chain: true,
            is_self_signed: false,
            signature_algorithm: Some("SHA1withRSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SSL005"));
    }

    #[test]
    fn test_ssl_strong_algorithm() {
        let cert = SslCertificateInfo {
            subject: Some("example.com".to_string()),
            issuer: Some("Let's Encrypt".to_string()),
            san_entries: vec!["example.com".to_string()],
            not_before: Some("2025-01-01T00:00:00Z".to_string()),
            not_after: Some("2027-01-01T00:00:00Z".to_string()),
            is_valid_chain: true,
            is_self_signed: false,
            signature_algorithm: Some("SHA256withECDSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SSL005"));
    }

    #[test]
    fn test_ssl_san_match() {
        let cert = SslCertificateInfo {
            subject: Some("other.com".to_string()),
            issuer: Some("Let's Encrypt".to_string()),
            san_entries: vec!["other.com".to_string(), "example.com".to_string()],
            not_before: Some("2025-01-01T00:00:00Z".to_string()),
            not_after: Some("2027-01-01T00:00:00Z".to_string()),
            is_valid_chain: true,
            is_self_signed: false,
            signature_algorithm: Some("SHA256withRSA".to_string()),
        };
        let validator = SslCertificateValidator::new(Some(cert));
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = validator.analyze(&ctx);
        // example.com is in SANs, so no mismatch
        assert!(!findings.iter().any(|f| f.code == "SSL004"));
    }

    // =========================================================================
    // MobileFriendlinessChecker tests
    // =========================================================================

    #[test]
    fn test_mobile_missing_viewport() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOB001"));
    }

    #[test]
    fn test_mobile_optimal_viewport() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "MOB001"));
        assert!(!findings.iter().any(|f| f.code == "MOB002"));
        assert!(!findings.iter().any(|f| f.code == "MOB003"));
        assert!(!findings.iter().any(|f| f.code == "MOB004"));
    }

    #[test]
    fn test_mobile_user_scalable_no() {
        let mut page = make_page("https://example.com");
        page.meta.viewport =
            Some("width=device-width, initial-scale=1, user-scalable=no".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOB004"));
    }

    #[test]
    fn test_mobile_maximum_scale_restricted() {
        let mut page = make_page("https://example.com");
        page.meta.viewport =
            Some("width=device-width, initial-scale=1, maximum-scale=1.0".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOB005"));
    }

    #[test]
    fn test_mobile_fixed_width() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=980".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOB003"));
    }

    #[test]
    fn test_mobile_missing_width_directive() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOB002"));
    }

    #[test]
    fn test_mobile_non_standard_initial_scale() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=2.0".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOB009"));
    }

    #[test]
    fn test_mobile_parse_viewport() {
        let vp = MobileFriendlinessChecker::parse_viewport(
            "width=device-width, initial-scale=1, user-scalable=no",
        );
        assert_eq!(vp.get("width").unwrap(), "device-width");
        assert_eq!(vp.get("initial-scale").unwrap(), "1");
        assert_eq!(vp.get("user-scalable").unwrap(), "no");
    }

    #[test]
    fn test_mobile_parse_viewport_empty() {
        let vp = MobileFriendlinessChecker::parse_viewport("");
        assert!(vp.is_empty());
    }

    #[test]
    fn test_mobile_both_user_scalable_and_max_scale() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some(
            "width=device-width, initial-scale=1, user-scalable=no, maximum-scale=1.0".to_string(),
        );
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOB004"));
        assert!(findings.iter().any(|f| f.code == "MOB005"));
    }

    // =========================================================================
    // AccessibilityAnalyzer tests
    // =========================================================================

    #[test]
    fn test_a11y_images_missing_alt() {
        let mut page = make_page("https://example.com");
        page.images = vec![
            ExtractedImage {
                src: "/a.png".to_string(),
                alt: String::new(),
                width: None,
                height: None,
                has_alt: false,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/b.jpg".to_string(),
                alt: "Good alt".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y001"));
        let f = findings.iter().find(|f| f.code == "A11Y001").unwrap();
        assert!(f.description.contains("/a.png"));
    }

    #[test]
    fn test_a11y_images_all_have_alt() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/a.png".to_string(),
            alt: "Description".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "A11Y001"));
    }

    #[test]
    fn test_a11y_no_headings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y002"));
    }

    #[test]
    fn test_a11y_missing_h1() {
        let mut page = make_page("https://example.com");
        page.headings = vec![Heading {
            level: 2,
            text: "Section".to_string(),
            length: 7,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y003"));
    }

    #[test]
    fn test_a11y_multiple_h1() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "First".to_string(),
                length: 5,
            },
            Heading {
                level: 1,
                text: "Second".to_string(),
                length: 6,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y004"));
    }

    #[test]
    fn test_a11y_skipped_heading_level() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "H1".to_string(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3 skipped H2".to_string(),
                length: 13,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y005"));
    }

    #[test]
    fn test_a11y_valid_heading_hierarchy() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "H1".to_string(),
                length: 2,
            },
            Heading {
                level: 2,
                text: "H2".to_string(),
                length: 2,
            },
            Heading {
                level: 2,
                text: "H2b".to_string(),
                length: 3,
            },
            Heading {
                level: 3,
                text: "H3".to_string(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "A11Y005"));
    }

    #[test]
    fn test_a11y_missing_main_landmark() {
        let mut page = make_page("https://example.com");
        page.has_main_landmark = false;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y006"));
    }

    #[test]
    fn test_a11y_has_main_landmark() {
        let mut page = make_page("https://example.com");
        page.has_main_landmark = true;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "A11Y006"));
    }

    #[test]
    fn test_a11y_empty_link_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: String::new(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y009"));
    }

    #[test]
    fn test_a11y_vague_link_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "click here".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y010"));
    }

    #[test]
    fn test_a11y_good_link_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/pricing".to_string(),
            text: "View pricing details".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "A11Y009"));
        assert!(!findings.iter().any(|f| f.code == "A11Y010"));
    }

    #[test]
    fn test_a11y_form_input_missing_label() {
        use crate::parser::ExtractedInput;
        let mut page = make_page("https://example.com");
        page.forms = vec![crate::parser::ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![ExtractedInput {
                input_type: Some("text".to_string()),
                name: Some("email".to_string()),
                id: None,
                has_label: false,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: Some("Enter email".to_string()),
                required: true,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y011"));
    }

    #[test]
    fn test_a11y_form_input_with_label() {
        use crate::parser::ExtractedInput;
        let mut page = make_page("https://example.com");
        page.forms = vec![crate::parser::ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![ExtractedInput {
                input_type: Some("text".to_string()),
                name: Some("email".to_string()),
                id: Some("email-input".to_string()),
                has_label: true,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: true,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "A11Y011"));
    }

    #[test]
    fn test_a11y_positive_tabindex() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y012"));
    }

    #[test]
    fn test_a11y_no_positive_tabindex() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = false;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "A11Y012"));
    }

    #[test]
    fn test_a11y_table_missing_headers() {
        let mut page = make_page("https://example.com");
        page.tables_total = 2;
        page.tables_with_headers = 1;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y014"));
        let f = findings.iter().find(|f| f.code == "A11Y014").unwrap();
        assert!(f.description.contains("1 of 2"));
    }

    #[test]
    fn test_a11y_table_with_headers() {
        let mut page = make_page("https://example.com");
        page.tables_total = 2;
        page.tables_with_headers = 2;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "A11Y014"));
    }

    #[test]
    fn test_a11y_missing_lang_attribute() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = false;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "A11Y016"));
    }

    #[test]
    fn test_a11y_has_lang_attribute() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "A11Y016"));
    }

    #[test]
    fn test_a11y_no_images_no_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        // Should not have A11Y001 (images missing alt) when there are no images
        assert!(!findings.iter().any(|f| f.code == "A11Y001"));
    }

    #[test]
    fn test_a11y_well_accessible_page() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "Page Title".to_string(),
                length: 10,
            },
            Heading {
                level: 2,
                text: "Section".to_string(),
                length: 7,
            },
        ];
        page.has_main_landmark = true;
        page.has_nav_landmark = true;
        page.has_skip_link = true;
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "click here".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx);
        // Well-accessible page should have no errors
        let errors: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Unexpected errors: {:?}",
            errors.iter().map(|f| &f.code).collect::<Vec<_>>()
        );
    }

    // =========================================================================
    // SocialMediaAnalyzer tests
    // =========================================================================

    #[test]
    fn test_social_og_image_no_dimensions() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = Some("https://example.com/og.png".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SOCIAL001"));
    }

    #[test]
    fn test_social_og_image_adequate_dimensions() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = Some("https://example.com/og.png".to_string());
        page.og_image_width = Some(1200);
        page.og_image_height = Some(630);
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SOCIAL001"));
        assert!(!findings.iter().any(|f| f.code == "SOCIAL002"));
        assert!(!findings.iter().any(|f| f.code == "SOCIAL003"));
    }

    #[test]
    fn test_social_og_image_too_small() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = Some("https://example.com/og.png".to_string());
        page.og_image_width = Some(600);
        page.og_image_height = Some(315);
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SOCIAL002"));
        assert!(findings.iter().any(|f| f.code == "SOCIAL003"));
    }

    #[test]
    fn test_social_og_image_width_only_too_narrow() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = Some("https://example.com/og.png".to_string());
        page.og_image_width = Some(800);
        page.og_image_height = Some(630);
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SOCIAL002"));
        assert!(!findings.iter().any(|f| f.code == "SOCIAL003"));
    }

    #[test]
    fn test_social_missing_twitter_card() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SOCIAL004"));
    }

    #[test]
    fn test_social_valid_twitter_card() {
        let mut page = make_page("https://example.com");
        page.meta.twitter.card = Some("summary_large_image".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SOCIAL004"));
        assert!(!findings.iter().any(|f| f.code == "SOCIAL005"));
    }

    #[test]
    fn test_social_invalid_twitter_card() {
        let mut page = make_page("https://example.com");
        page.meta.twitter.card = Some("invalid_type".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SOCIAL005"));
    }

    #[test]
    fn test_social_incomplete_og_tags() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        // Missing og:title, og:description, og:image, og:url, og:type
        assert!(findings.iter().any(|f| f.code == "SOCIAL006"));
        let f = findings.iter().find(|f| f.code == "SOCIAL006").unwrap();
        assert!(f.description.contains("og:title"));
    }

    #[test]
    fn test_social_complete_og_tags() {
        let mut page = make_page("https://example.com");
        page.meta.og.title = Some("Title".to_string());
        page.meta.og.description = Some("Description".to_string());
        page.meta.og.image = Some("https://example.com/og.png".to_string());
        page.meta.og.url = Some("https://example.com".to_string());
        page.meta.og.r#type = Some("website".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SOCIAL006"));
    }

    #[test]
    fn test_social_completeness_score() {
        let mut page = make_page("https://example.com");
        page.meta.og.title = Some("Title".to_string());
        page.meta.og.description = Some("Description".to_string());
        page.meta.og.image = Some("https://example.com/og.png".to_string());
        page.meta.og.url = Some("https://example.com".to_string());
        page.meta.og.r#type = Some("website".to_string());
        page.meta.twitter.card = Some("summary_large_image".to_string());
        page.meta.twitter.title = Some("Title".to_string());
        page.meta.twitter.description = Some("Desc".to_string());
        page.meta.twitter.image = Some("https://example.com/tw.png".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        let score = findings.iter().find(|f| f.code == "SOCIAL008").unwrap();
        assert!(score.description.contains("8/8"));
    }

    #[test]
    fn test_social_no_og_image_no_dimension_warning() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = None;
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        // No OG image = no dimension warning needed
        assert!(!findings.iter().any(|f| f.code == "SOCIAL001"));
    }

    #[test]
    fn test_social_all_twitter_cards_valid() {
        for card_type in &["summary", "summary_large_image", "app", "player"] {
            let mut page = make_page("https://example.com");
            page.meta.twitter.card = Some(card_type.to_string());
            let ctx = make_ctx(&page, Some(200));
            let findings = SocialMediaAnalyzer::new().analyze(&ctx);
            assert!(
                !findings.iter().any(|f| f.code == "SOCIAL005"),
                "Card type {card_type} should be valid"
            );
        }
    }

    // =========================================================================
    // EntityAnalyzer tests
    // =========================================================================

    #[test]
    fn test_entity_analyzer_empty_page() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = EntityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ENTITY005"));
        assert!(findings.iter().any(|f| f.code == "ENTITY006"));
    }

    #[test]
    fn test_entity_detect_people() {
        let text = "Written by Dr. John Smith and edited by Prof. Jane Doe.";
        let people = EntityAnalyzer::detect_people(text);
        assert!(!people.is_empty());
    }

    #[test]
    fn test_entity_detect_organizations() {
        let text = "This product is made by Acme Corporation and Widget Inc.";
        let orgs = EntityAnalyzer::detect_organizations(text);
        assert!(!orgs.is_empty());
    }

    #[test]
    fn test_entity_detect_locations() {
        let text = "The city of London and the mountain region are beautiful.";
        let locs = EntityAnalyzer::detect_locations(text);
        assert!(!locs.is_empty());
    }

    #[test]
    fn test_entity_topics() {
        let headings = vec![
            Heading {
                level: 1,
                text: "Rust Programming Language Tutorial".to_string(),
                length: 33,
            },
            Heading {
                level: 2,
                text: "Advanced Rust Programming Concepts".to_string(),
                length: 35,
            },
        ];
        let topics = EntityAnalyzer::detect_topics(&headings, 500);
        assert!(topics
            .iter()
            .any(|t| t.contains("rust") || t.contains("programming")));
    }

    #[test]
    fn test_entity_sentiment_positive() {
        let (score, label) = EntityAnalyzer::analyze_sentiment(
            "This is a great and amazing product, truly wonderful and excellent!",
        );
        assert!(score > 0.0);
        assert_eq!(label, "positive");
    }

    #[test]
    fn test_entity_sentiment_negative() {
        let (score, label) = EntityAnalyzer::analyze_sentiment(
            "This is a terrible and horrible product, truly awful and bad!",
        );
        assert!(score < 0.0);
        assert_eq!(label, "negative");
    }

    #[test]
    fn test_entity_sentiment_neutral() {
        let (score, label) =
            EntityAnalyzer::analyze_sentiment("The page contains information about the topic.");
        assert_eq!(label, "neutral");
        assert!((-0.05..=0.05).contains(&score));
    }

    #[test]
    fn test_entity_analyzer_with_people() {
        let mut page = make_page("https://example.com");
        page.headings = vec![Heading {
            level: 1,
            text: "Interview with Dr. Alice Smith".to_string(),
            length: 29,
        }];
        page.word_count = 500;
        let ctx = make_ctx(&page, Some(200));
        let findings = EntityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ENTITY001"));
    }

    // =========================================================================
    // EnhancedReadabilityAnalyzer tests
    // =========================================================================

    #[test]
    fn test_readability_empty_page() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_readability_simple_text() {
        let mut page = make_page("https://example.com");
        page.word_count = 100;
        page.headings = vec![Heading {
            level: 1,
            text: "The cat sat on the mat. The dog ran in the park.".to_string(),
            length: 48,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "READ001"));
        assert!(findings.iter().any(|f| f.code == "READ005"));
        let fre = findings.iter().find(|f| f.code == "READ005").unwrap();
        assert!(fre.description.contains("Score:"));
    }

    #[test]
    fn test_readability_complex_text() {
        let mut page = make_page("https://example.com");
        page.word_count = 200;
        page.headings = vec![Heading {
            level: 1,
            text: "The implementation of sophisticated algorithmic methodologies \
                   necessitates comprehensive understanding of computational complexity \
                   and theoretical frameworks"
                .to_string(),
            length: 150,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "READ001"));
        let fk = findings.iter().find(|f| f.code == "READ001").unwrap();
        assert!(fk.description.contains("Grade level:"));
    }

    #[test]
    fn test_readability_syllable_counting() {
        assert_eq!(count_syllables("cat"), 1);
        assert_eq!(count_syllables("hello"), 2);
        assert_eq!(count_syllables("beautiful"), 3);
        assert_eq!(count_syllables("a"), 1);
        assert_eq!(count_syllables(""), 0);
    }

    #[test]
    fn test_readability_indices() {
        let mut page = make_page("https://example.com");
        page.word_count = 500;
        page.headings = vec![Heading {
            level: 1,
            text: "A comprehensive guide to understanding modern web development \
                   practices and techniques for beginners and experienced developers"
                .to_string(),
            length: 140,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "READ001"));
        assert!(findings.iter().any(|f| f.code == "READ002"));
        assert!(findings.iter().any(|f| f.code == "READ003"));
        assert!(findings.iter().any(|f| f.code == "READ004"));
        assert!(findings.iter().any(|f| f.code == "READ005"));
    }

    #[test]
    fn test_readability_grade_label() {
        assert_eq!(
            EnhancedReadabilityAnalyzer::grade_label(0.5),
            "kindergarten"
        );
        assert_eq!(
            EnhancedReadabilityAnalyzer::grade_label(4.0),
            "elementary school"
        );
        assert_eq!(
            EnhancedReadabilityAnalyzer::grade_label(7.0),
            "middle school"
        );
        assert_eq!(
            EnhancedReadabilityAnalyzer::grade_label(10.0),
            "high school"
        );
        assert_eq!(EnhancedReadabilityAnalyzer::grade_label(14.0), "college");
        assert_eq!(
            EnhancedReadabilityAnalyzer::grade_label(18.0),
            "postgraduate"
        );
    }

    #[test]
    fn test_readability_ease_label() {
        assert_eq!(
            EnhancedReadabilityAnalyzer::reading_ease_label(95.0),
            "very easy"
        );
        assert_eq!(
            EnhancedReadabilityAnalyzer::reading_ease_label(85.0),
            "easy"
        );
        assert_eq!(
            EnhancedReadabilityAnalyzer::reading_ease_label(75.0),
            "fairly easy"
        );
        assert_eq!(
            EnhancedReadabilityAnalyzer::reading_ease_label(65.0),
            "standard"
        );
        assert_eq!(
            EnhancedReadabilityAnalyzer::reading_ease_label(55.0),
            "fairly difficult"
        );
        assert_eq!(
            EnhancedReadabilityAnalyzer::reading_ease_label(35.0),
            "difficult"
        );
        assert_eq!(
            EnhancedReadabilityAnalyzer::reading_ease_label(20.0),
            "very difficult"
        );
    }

    // =========================================================================
    // KeywordAnalyzer tests
    // =========================================================================

    #[test]
    fn test_keyword_analyzer_empty_page() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = KeywordAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_keyword_analyzer_with_content() {
        let mut page = make_page("https://example.com");
        page.word_count = 500;
        page.headings = vec![
            Heading {
                level: 1,
                text: "Rust Programming Language Tutorial for Beginners".to_string(),
                length: 48,
            },
            Heading {
                level: 2,
                text: "Learn Rust Programming Today".to_string(),
                length: 28,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = KeywordAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "KW001"));
        assert!(findings.iter().any(|f| f.code == "KW002"));
    }

    #[test]
    fn test_keyword_tokenize() {
        let tokens = KeywordAnalyzer::tokenize("Rust Programming Language Tutorial");
        assert!(tokens.contains(&"rust".to_string()));
        assert!(tokens.contains(&"programming".to_string()));
        assert!(tokens.contains(&"language".to_string()));
        assert!(tokens.contains(&"tutorial".to_string()));
    }

    #[test]
    fn test_keyword_tf() {
        let tokens = vec![
            "rust".to_string(),
            "programming".to_string(),
            "rust".to_string(),
            "language".to_string(),
        ];
        let tf = KeywordAnalyzer::compute_tf(&tokens);
        assert!(*tf.get("rust").unwrap() > 0.4);
    }

    #[test]
    fn test_keyword_density() {
        let tokens = vec![
            "rust".to_string(),
            "programming".to_string(),
            "rust".to_string(),
        ];
        let density = KeywordAnalyzer::keyword_density(&tokens, 100);
        assert!(*density.get("rust").unwrap() > 1.5);
    }

    #[test]
    fn test_keyword_prominent_detection() {
        let mut density = HashMap::new();
        density.insert("rust".to_string(), 3.5);
        density.insert("programming".to_string(), 1.0);
        let prominent = KeywordAnalyzer::detect_prominent_keywords(&density);
        assert_eq!(prominent.len(), 1);
        assert_eq!(prominent[0].0, "rust");
    }

    #[test]
    fn test_keyword_cooccurrence() {
        let tokens = vec![
            "rust".to_string(),
            "programming".to_string(),
            "language".to_string(),
            "rust".to_string(),
            "programming".to_string(),
        ];
        let cooccur = KeywordAnalyzer::cooccurrence(&tokens, 2);
        assert!(!cooccur.is_empty());
    }

    #[test]
    fn test_keyword_analyzer_prominent_warning() {
        let mut page = make_page("https://example.com");
        page.word_count = 100;
        page.headings = vec![Heading {
            level: 1,
            text: "rust rust rust rust rust rust rust rust rust rust rust".to_string(),
            length: 55,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = KeywordAnalyzer::new().analyze(&ctx);
        let kw003 = findings.iter().find(|f| f.code == "KW003");
        assert!(kw003.is_some());
        assert_eq!(kw003.unwrap().severity, Severity::Warning);
    }

    // =========================================================================
    // EcommerceSignalsAnalyzer tests
    // =========================================================================

    #[test]
    fn test_ecom_empty_page() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ecom_product_schema() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget",
                "offers": {
                    "@type": "Offer",
                    "price": "29.99",
                    "priceCurrency": "USD",
                    "availability": "https://schema.org/InStock"
                }
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ECOM001"));
        assert!(findings.iter().any(|f| f.code == "ECOM002"));
        assert!(findings.iter().any(|f| f.code == "ECOM003"));
        assert!(findings.iter().any(|f| f.code == "ECOM005"));
    }

    #[test]
    fn test_ecom_product_with_reviews() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget",
                "aggregateRating": {
                    "@type": "AggregateRating",
                    "ratingValue": "4.5",
                    "reviewCount": "120"
                }
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ECOM004"));
    }

    #[test]
    fn test_ecom_price_without_product() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer",
                "price": "19.99",
                "priceCurrency": "USD"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ECOM006"));
    }

    #[test]
    fn test_ecom_no_price() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ECOM001"));
        assert!(!findings.iter().any(|f| f.code == "ECOM002"));
    }

    #[test]
    fn test_ecom_non_product_schema() {
        let mut page = make_page("https://example.com/article");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": "Test"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // =========================================================================
    // InternationalSeoAnalyzer tests
    // =========================================================================

    #[test]
    fn test_iseo_empty_page() {
        let mut page = make_page("https://example.com");
        page.html_lang = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ISEO004"));
    }

    #[test]
    fn test_iseo_detect_locale_from_url() {
        assert_eq!(
            InternationalSeoAnalyzer::detect_locale_from_url("https://example.com/en/page"),
            Some("en".to_string())
        );
        assert_eq!(
            InternationalSeoAnalyzer::detect_locale_from_url("https://example.com/fr/page"),
            Some("fr".to_string())
        );
        assert_eq!(
            InternationalSeoAnalyzer::detect_locale_from_url("https://example.com/en-US/page"),
            Some("en-US".to_string())
        );
        assert_eq!(
            InternationalSeoAnalyzer::detect_locale_from_url("https://example.com/page"),
            None
        );
    }

    #[test]
    fn test_iseo_is_locale_segment() {
        assert!(InternationalSeoAnalyzer::is_locale_segment("en"));
        assert!(InternationalSeoAnalyzer::is_locale_segment("fr"));
        assert!(InternationalSeoAnalyzer::is_locale_segment("en-US"));
        assert!(InternationalSeoAnalyzer::is_locale_segment("zh-CN"));
        assert!(!InternationalSeoAnalyzer::is_locale_segment("page"));
        assert!(!InternationalSeoAnalyzer::is_locale_segment("e"));
    }

    #[test]
    fn test_iseo_hreflang_url_locale_mismatch() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "fr".to_string(),
            url: Url::parse("https://example.com/en/about").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ISEO001"));
    }

    #[test]
    fn test_iseo_missing_x_default() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "fr".to_string(),
                url: Url::parse("https://example.com/fr").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ISEO002"));
    }

    #[test]
    fn test_iseo_duplicate_language() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en-uk").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ISEO003"));
    }

    #[test]
    fn test_iseo_multilang_content() {
        // Multilingual presence is deliberately NOT a finding (noise on
        // correctly configured sites); only hreflang *validation* findings
        // fire.
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "x-default".to_string(),
                url: Url::parse("https://example.com").unwrap(),
            },
        ];
        page.html_lang = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().all(|f| f.code != "ISEO006"));
    }

    #[test]
    fn test_iseo_valid_hreflang_with_xdefault() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "fr".to_string(),
                url: Url::parse("https://example.com/fr").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "x-default".to_string(),
                url: Url::parse("https://example.com").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.severity == Severity::Error));
    }

    #[test]
    fn test_iseo_locale_detection_from_url() {
        let locale = InternationalSeoAnalyzer::detect_locale_from_url(
            "https://example.com/de/products/widget",
        );
        assert_eq!(locale, Some("de".to_string()));
    }

    #[test]
    fn test_iseo_no_locale_no_hreflang() {
        let mut page = make_page("https://example.com/products");
        page.html_lang = None;
        let ctx = make_ctx(&page, Some(200));
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ISEO004"));
    }
}

/// Direct fixtures for KeywordAnalyzer::compute_tfidf — hand-computed from
/// idf = ln(N/df) + 1, tfidf = tf * idf. Added after the mutants pass on
/// seo_analyzers.rs showed arithmetic mutants surviving here (same class
/// as the flesch gaps fixed in 4.0.0).
#[test]
fn test_compute_tfidf_fixtures() {
    use std::collections::HashMap;

    let tf: HashMap<String, f64> = [("alpha".to_string(), 0.5)].into();
    // Two "documents" in the corpus; alpha appears in one with tf 0.5.
    let corpus: HashMap<String, f64> =
        [("alpha".to_string(), 0.5), ("beta".to_string(), 0.25)].into();

    let out = KeywordAnalyzer::compute_tfidf(&tf, &corpus);
    let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} vs {b}");
    // idf = ln(2/0.5)+1 = ln(4)+1 = 2.386294361...; tfidf = 0.5 * that.
    close(out["alpha"], 0.5 * ((2.0f64 / 0.5).ln() + 1.0));

    // Term absent from the corpus: idf falls back to 1.0 -> tfidf = tf.
    let tf2: HashMap<String, f64> = [("gamma".to_string(), 0.25)].into();
    let out2 = KeywordAnalyzer::compute_tfidf(&tf2, &corpus);
    close(out2["gamma"], 0.25);

    // Empty corpus (total_docs clamps to 1): df 0 -> idf 1.0 -> tfidf = tf.
    let out3 = KeywordAnalyzer::compute_tfidf(&tf2, &HashMap::new());
    close(out3["gamma"], 0.25);

    // Higher df (more common term) must yield a strictly smaller idf.
    let common: HashMap<String, f64> = [("alpha".to_string(), 2.0)].into();
    let out_common = KeywordAnalyzer::compute_tfidf(&tf, &common);
    // idf = ln(1/2)+1 < 1 -> score below plain tf.
    assert!(out_common["alpha"] < 0.5);
}

/// Direct fixtures for KeywordAnalyzer::keyword_density (percent = count /
/// total * 100) and its zero/empty guards.
#[test]
fn test_keyword_density_fixtures() {
    let tokens: Vec<String> = ["a", "a", "b"].iter().map(|s| s.to_string()).collect();
    let d = KeywordAnalyzer::keyword_density(&tokens, 4);
    let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} vs {b}");
    close(d["a"], 50.0); // 2/4*100
    close(d["b"], 25.0); // 1/4*100

    // Zero total words: empty map, never a division by zero.
    assert!(KeywordAnalyzer::keyword_density(&tokens, 0).is_empty());

    // Density threshold handoff: 1.5% boundary is inclusive.
    let d2 = KeywordAnalyzer::keyword_density(&["x".to_string()], 66);
    close(d2["x"], 100.0 / 66.0);
}

/// Direct fixtures for HreflangValidator::is_valid_locale — every branch
/// (x-default, language-only length/charset bounds, language-region with
/// alpha-2 and numeric-4 regions, multi-segment rejection).
#[test]
fn test_is_valid_locale_fixtures() {
    use seo_analyzers::HreflangValidator;
    let _v = HreflangValidator::new();

    // x-default passes unconditionally.
    assert!(HreflangValidator::is_valid_locale("x-default"));

    // Language-only: 2-3 ascii letters.
    assert!(HreflangValidator::is_valid_locale("en"));
    assert!(HreflangValidator::is_valid_locale("dan"));
    assert!(!HreflangValidator::is_valid_locale("e")); // too short
    assert!(!HreflangValidator::is_valid_locale("engs")); // too long
    assert!(!HreflangValidator::is_valid_locale("e1")); // non-alpha

    // Language-Region: alpha-2 or numeric-4 (UN M49).
    assert!(HreflangValidator::is_valid_locale("en-US"));
    assert!(HreflangValidator::is_valid_locale("pt-BR"));
    assert!(HreflangValidator::is_valid_locale("es-419")); // UN M49 Latin America
    assert!(HreflangValidator::is_valid_locale("en-001")); // UN M49 world
    assert!(!HreflangValidator::is_valid_locale("en-USA")); // 3-letter region
    assert!(!HreflangValidator::is_valid_locale("en-1")); // 1-digit region
    assert!(!HreflangValidator::is_valid_locale("en-4199")); // 4-digit region (not M49)
    assert!(!HreflangValidator::is_valid_locale("e1-US")); // bad language half
    assert!(!HreflangValidator::is_valid_locale("en-US-variant")); // 3 segments
}

/// Direct fixtures for SitemapAnalyzer::is_valid_lastmod — full-year scan
/// window, separator/digit requirements, and the short-input guard.
#[test]
fn test_is_valid_lastmod_fixtures() {
    use seo_analyzers::SitemapAnalyzer;

    // Canonical forms.
    assert!(SitemapAnalyzer::is_valid_lastmod("2024-01"));
    assert!(SitemapAnalyzer::is_valid_lastmod("2024-01-15"));
    assert!(SitemapAnalyzer::is_valid_lastmod(
        "2024-01-15T10:30:00+00:00"
    ));

    // Too short for the YYYY-MM scan window.
    assert!(!SitemapAnalyzer::is_valid_lastmod(""));
    assert!(!SitemapAnalyzer::is_valid_lastmod("2024"));
    assert!(!SitemapAnalyzer::is_valid_lastmod("202-01"));

    // Window-internal separator/digit defects.
    assert!(!SitemapAnalyzer::is_valid_lastmod("202x-01")); // year digit
    assert!(!SitemapAnalyzer::is_valid_lastmod("2024x01")); // separator
    assert!(!SitemapAnalyzer::is_valid_lastmod("2024-xy")); // month digits

    // Valid date appearing after a prefix (scan finds it anywhere).
    assert!(SitemapAnalyzer::is_valid_lastmod("modified:2024-06"));
}

// =========================================================================
// HstsPreloadAnalyzer tests
// =========================================================================

#[cfg(test)]
mod hsts_preload_tests {
    use super::*;
    use crate::parser::ParsedPage;
    use crate::meta::MetaTags;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx_with_headers<'a>(
        page: &'a ParsedPage,
        headers: &'a [(String, String)],
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        }
    }

    #[test]
    fn test_hsts_no_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &[]);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hsts_missing_include_subdomains() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HSTS001"));
    }

    #[test]
    fn test_hsts_missing_preload() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; includeSubDomains".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HSTS002"));
        assert!(!findings.iter().any(|f| f.code == "HSTS001"));
    }

    #[test]
    fn test_hsts_max_age_too_low() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=300; includeSubDomains; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HSTS003"));
    }

    #[test]
    fn test_hsts_perfect() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=63072000; includeSubDomains; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hsts_max_age_exact_31536000() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; includeSubDomains; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "HSTS003"));
    }

    #[test]
    fn test_hsts_case_insensitive_directives() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; INCLUDESUBDOMAINS; PRELOAD".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hsts_all_issues() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=100".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HSTS001"));
        assert!(findings.iter().any(|f| f.code == "HSTS002"));
        assert!(findings.iter().any(|f| f.code == "HSTS003"));
    }

    #[test]
    fn test_hsts_max_age_missing() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "includeSubDomains".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        // includeSubDomains is present, so HSTS001 should NOT fire
        assert!(!findings.iter().any(|f| f.code == "HSTS001"));
        assert!(findings.iter().any(|f| f.code == "HSTS002"));
    }

    #[test]
    fn test_hsts_partial_include_subdomains() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HSTS001"));
        assert!(!findings.iter().any(|f| f.code == "HSTS002"));
    }

    #[test]
    fn test_hsts_only_preload_missing() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=63072000; includeSubDomains".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "HSTS001"));
        assert!(findings.iter().any(|f| f.code == "HSTS002"));
        assert!(!findings.iter().any(|f| f.code == "HSTS003"));
    }

    #[test]
    fn test_hsts_max_age_just_below() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31535999; includeSubDomains; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HSTS003"));
    }

    #[test]
    fn test_hsts_max_age_very_high() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=315360000; includeSubDomains; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hsts_lowercase_header_name() {
        let headers = vec![(
            "strict-transport-security".to_string(),
            "max-age=31536000; includeSubDomains; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hsts_directives_case_insensitive() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "Max-Age=31536000; includesubdomains; PRELOAD".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}

// =========================================================================
// SriAnalyzer tests
// =========================================================================

#[cfg(test)]
mod sri_tests {
    use super::*;
    use crate::parser::{ParsedPage, ScriptInfo, StyleInfo};
    use crate::meta::MetaTags;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        }
    }

    #[test]
    fn test_sri_no_scripts_or_styles() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_sri_local_script_no_issue() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("/app.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SRI001"));
    }

    #[test]
    fn test_sri_external_script_without_integrity() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://cdn.example.com/lib.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SRI001"));
    }

    #[test]
    fn test_sri_external_script_with_integrity() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://cdn.example.com/lib.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: true,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SRI001"));
    }

    #[test]
    fn test_sri_external_stylesheet_without_integrity() {
        let mut page = make_page("https://example.com");
        page.styles = vec![StyleInfo {
            href: Some("https://cdn.example.com/style.css".to_string()),
            media: None,
            is_inline: false,
            has_integrity: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SRI002"));
    }

    #[test]
    fn test_sri_external_stylesheet_with_integrity() {
        let mut page = make_page("https://example.com");
        page.styles = vec![StyleInfo {
            href: Some("https://cdn.example.com/style.css".to_string()),
            media: None,
            is_inline: false,
            has_integrity: true,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SRI002"));
    }

    #[test]
    fn test_sri_inline_style_no_issue() {
        let mut page = make_page("https://example.com");
        page.styles = vec![StyleInfo {
            href: None,
            media: None,
            is_inline: true,
            has_integrity: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_sri_multiple_external_scripts_mixed() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            ScriptInfo {
                src: Some("https://cdn.example.com/a.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
            },
            ScriptInfo {
                src: Some("https://cdn.example.com/b.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: true,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SRI001"));
        let finding = findings.iter().find(|f| f.code == "SRI001").unwrap();
        assert!(finding.description.contains("1 external script(s)"));
    }

    #[test]
    fn test_sri_protocol_relative_url() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("//cdn.example.com/lib.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SRI001"));
    }

    #[test]
    fn test_sri_both_scripts_and_styles() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://cdn.example.com/app.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        }];
        page.styles = vec![StyleInfo {
            href: Some("https://cdn.example.com/style.css".to_string()),
            media: None,
            is_inline: false,
            has_integrity: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SRI001"));
        assert!(findings.iter().any(|f| f.code == "SRI002"));
    }

    #[test]
    fn test_sri_no_cross_origin_scripts_all_local() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            ScriptInfo {
                src: Some("/js/a.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
            },
            ScriptInfo {
                src: Some("/js/b.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SRI001"));
    }

    #[test]
    fn test_sri_no_cross_origin_stylesheets() {
        let mut page = make_page("https://example.com");
        page.styles = vec![
            StyleInfo {
                href: Some("/css/a.css".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            },
            StyleInfo {
                href: None,
                media: None,
                is_inline: true,
                has_integrity: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SRI002"));
    }

    #[test]
    fn test_sri_only_external_without_integrity_counted() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            ScriptInfo {
                src: Some("https://cdn.example.com/a.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
            },
            ScriptInfo {
                src: Some("https://cdn.example.com/b.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
            },
            ScriptInfo {
                src: Some("/local.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SRI001"));
        let finding = findings.iter().find(|f| f.code == "SRI001").unwrap();
        assert!(finding.description.contains("2 external script(s)"));
    }

    #[test]
    fn test_sri_all_with_integrity_no_findings() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://cdn.example.com/lib.js".to_string()),
            r#async: true,
            defer: false,
            script_type: None,
            has_integrity: true,
        }];
        page.styles = vec![StyleInfo {
            href: Some("https://cdn.example.com/style.css".to_string()),
            media: Some("screen".to_string()),
            is_inline: false,
            has_integrity: true,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_sri_only_local_no_findings() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("/app.js".to_string()),
            r#async: false,
            defer: true,
            script_type: None,
            has_integrity: false,
        }];
        page.styles = vec![StyleInfo {
            href: Some("/style.css".to_string()),
            media: None,
            is_inline: false,
            has_integrity: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SriAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}

// =========================================================================
// PermissionPolicyAnalyzer tests
// =========================================================================

#[cfg(test)]
mod permission_policy_tests {
    use super::*;
    use crate::parser::ParsedPage;
    use crate::meta::MetaTags;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx_with_headers<'a>(
        page: &'a ParsedPage,
        headers: &'a [(String, String)],
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        }
    }

    #[test]
    fn test_perm_no_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &[]);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERM001"));
    }

    #[test]
    fn test_perm_camera_not_restricted() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=*".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERM002"));
    }

    #[test]
    fn test_perm_microphone_not_restricted() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "microphone=*".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERM002"));
    }

    #[test]
    fn test_perm_camera_restricted() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=()".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PERM002"));
        assert!(!findings.iter().any(|f| f.code == "PERM001"));
    }

    #[test]
    fn test_perm_all_restricted() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=(), microphone=(), geolocation=()".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_perm_camera_self_restricted() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=(self)".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PERM002"));
    }

    #[test]
    fn test_perm_multiple_features_mixed() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=(), microphone=*, geolocation=()".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        // camera restricted, geolocation restricted, but microphone not
        let perm002: Vec<&Finding> = findings.iter().filter(|f| f.code == "PERM002").collect();
        assert_eq!(perm002.len(), 1);
    }

    #[test]
    fn test_perm_lowercase_header_name() {
        let headers = vec![(
            "permissions-policy".to_string(),
            "camera=(), microphone=()".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_perm_no_issue_when_feature_not_mentioned() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "geolocation=()".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PERM002"));
    }

    #[test]
    fn test_perm_empty_policy_value() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        // Empty value means no header effectively, but technically present
        assert!(!findings.iter().any(|f| f.code == "PERM001"));
        assert!(!findings.iter().any(|f| f.code == "PERM002"));
    }

    #[test]
    fn test_perm_only_microphone_unrestricted() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=(), microphone=*, geolocation=(), gyroscope=()".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        let perm002: Vec<&Finding> = findings.iter().filter(|f| f.code == "PERM002").collect();
        assert_eq!(perm002.len(), 1);
        assert!(perm002[0].description.contains("microphone"));
    }

    #[test]
    fn test_perm_both_unrestricted() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=*, microphone=*".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        let perm002: Vec<&Finding> = findings.iter().filter(|f| f.code == "PERM002").collect();
        assert_eq!(perm002.len(), 2);
    }

    #[test]
    fn test_perm_multiple_restricted_features() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=(), microphone=(), geolocation=(), gyroscope=()".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_perm_with_other_directives() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "accelerometer=(), camera=(), geolocation=(), gyroscope=(), \
             magnetometer=(), microphone=(), payment=(), usb=()"
                .to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_perm_camera_with_self_and_unrestricted() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=(self), microphone=*".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
        let perm002: Vec<&Finding> = findings.iter().filter(|f| f.code == "PERM002").collect();
        assert_eq!(perm002.len(), 1);
        assert!(perm002[0].description.contains("microphone"));
    }
}

// =========================================================================
// CrossOriginIsolationAnalyzer tests
// =========================================================================

#[cfg(test)]
mod cross_origin_isolation_tests {
    use super::*;
    use crate::parser::ParsedPage;
    use crate::meta::MetaTags;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx_with_headers<'a>(
        page: &'a ParsedPage,
        headers: &'a [(String, String)],
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
        }
    }

    #[test]
    fn test_co_no_headers() {
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &[]);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COEP001"));
        assert!(findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_co_coep_present() {
        let headers = vec![(
            "Cross-Origin-Embedder-Policy".to_string(),
            "require-corp".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "COEP001"));
        assert!(findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_co_coop_present() {
        let headers = vec![(
            "Cross-Origin-Opener-Policy".to_string(),
            "same-origin".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COEP001"));
        assert!(!findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_co_both_present() {
        let headers = vec![
            (
                "Cross-Origin-Embedder-Policy".to_string(),
                "require-corp".to_string(),
            ),
            (
                "Cross-Origin-Opener-Policy".to_string(),
                "same-origin".to_string(),
            ),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_co_case_insensitive_header_names() {
        let headers = vec![
            (
                "cross-origin-embedder-policy".to_string(),
                "require-corp".to_string(),
            ),
            (
                "cross-origin-opener-policy".to_string(),
                "same-origin".to_string(),
            ),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_co_only_coep_present() {
        let headers = vec![
            (
                "Cross-Origin-Embedder-Policy".to_string(),
                "require-corp".to_string(),
            ),
            (
                "Cross-Origin-Resource-Policy".to_string(),
                "same-origin".to_string(),
            ),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "COEP001"));
        assert!(findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_co_only_coop_present() {
        let headers = vec![
            (
                "Cross-Origin-Opener-Policy".to_string(),
                "same-origin".to_string(),
            ),
            (
                "Cross-Origin-Resource-Policy".to_string(),
                "same-origin".to_string(),
            ),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COEP001"));
        assert!(!findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_co_corp_only_no_coep_no_coop() {
        let headers = vec![(
            "Cross-Origin-Resource-Policy".to_string(),
            "same-origin".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COEP001"));
        assert!(findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_co_coep_wrong_value_still_present() {
        let headers = vec![(
            "Cross-Origin-Embedder-Policy".to_string(),
            "unsafe-none".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        // Only checks presence, not value
        assert!(!findings.iter().any(|f| f.code == "COEP001"));
    }

    #[test]
    fn test_co_all_cross_origin_headers_present() {
        let headers = vec![
            (
                "Cross-Origin-Embedder-Policy".to_string(),
                "require-corp".to_string(),
            ),
            (
                "Cross-Origin-Opener-Policy".to_string(),
                "same-origin".to_string(),
            ),
            (
                "Cross-Origin-Resource-Policy".to_string(),
                "same-site".to_string(),
            ),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_co_only_coep_wrong_value() {
        let headers = vec![(
            "Cross-Origin-Embedder-Policy".to_string(),
            "unsafe-none".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "COEP001"));
        assert!(findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_co_mixed_case_header_names() {
        let headers = vec![
            (
                "cross-origin-embedder-Policy".to_string(),
                "require-corp".to_string(),
            ),
            (
                "Cross-Origin-OPENER-policy".to_string(),
                "same-origin".to_string(),
            ),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_headers(&page, &headers);
        let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}
