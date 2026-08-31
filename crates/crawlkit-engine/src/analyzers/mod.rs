use std::time::Duration;

use crate::parser::ParsedPage;
use crate::{CrawlConfig, RedirectHop};

/// Deep accessibility analyzers for landmarks, structure, forms, media, focus, and language.
pub mod accessibility_deep_analyzers;
/// Accessibility analyzers for color contrast and keyboard focus order.
pub mod accessibility_visual_analyzers;
/// ARIA accessibility analyzers.
pub mod aria_analyzers;
/// ARIA label accessibility analyzer.
pub mod aria_label_analyzers;
/// Basic accessibility analyzers for forms, links, images, and ARIA roles.
pub mod basic_accessibility_analyzers;
/// Content quality analyzers for readability, entity extraction, and structured data.
pub mod content_analyzers;
/// Cookie analyzers: Set-Cookie flag checks (extracted from security_analyzers).
pub mod cookies;
/// Deep security-header analyzers extracted from the security monolith.
pub mod deep_security_header_analyzers;
/// DNS rebinding, Subresource Integrity, and CORS security analyzers.
pub mod dns_sri_cors_analyzers;
/// Focus-management accessibility analyzers.
pub mod focus_management_analyzers;
/// Font-size and line-height accessibility analyzers.
pub mod font_analyzers;
/// Form accessibility analyzers.
pub mod form_analyzers;
/// Heading hierarchy accessibility analyzers.
pub mod heading_analyzers;
/// HSTS preload readiness analyzer (extracted from security_analyzers).
pub mod hsts_analyzer;
/// HTTP-level analyzers for status codes, redirects, robots.txt, and SSL certificates.
pub mod http_analyzers;
/// Image accessibility analyzers.
pub mod image_accessibility_analyzers;
/// Landmark accessibility analyzers.
pub mod landmark_analyzers;
/// HTML language-attribute accessibility analyzers.
pub mod language_accessibility_analyzers;
/// Link accessibility analyzers.
pub mod link_accessibility_analyzers;
/// Media and e-commerce analyzers for images, products, and shopping signals.
pub mod media_analyzers;
/// Mixed-content and HSTS preload-readiness analyzers extracted from the security monolith.
pub mod mixed_content_analyzers;
/// Mobile-friendliness analyzer for viewport and zoom accessibility checks.
pub mod mobile_analyzers;
/// Permissions-Policy V2 security analyzer.
pub mod permissions_policy_v2_analyzers;
/// Post-crawl analyzers that inspect full crawl data for cross-page issues.
pub mod post_crawl_analyzers;
/// Schema.org validators, one per type.
pub mod schema;
/// Security and accessibility analyzers for headers, mobile-friendliness, and WCAG compliance.
pub mod security_analyzers;
/// Security header analyzers: SRI, Permission-Policy, Cross-Origin isolation
/// (extracted from security_analyzers).
pub mod security_header_analyzers;
/// Versioned security-header analyzers extracted from the security monolith.
pub mod security_header_v2_analyzers;
/// SEO analyzers for meta tags, headings, links, canonicals, and internationalization.
pub mod seo_analyzers;
/// Skip-navigation link accessibility analyzer.
pub mod skip_link_analyzers;
/// Social media analyzers for Open Graph, Twitter Cards, and social sharing metadata.
pub mod social_analyzers;
/// Strict-Transport-Security / COOP / COEP / Feature-Policy / Expect-CT / CT
/// analyzers (extracted from security_analyzers).
pub mod sts_analyzers;
/// Tabindex accessibility analyzer.
pub mod tabindex_analyzers;
/// Table accessibility analyzers.
pub mod table_analyzers;
/// Table caption accessibility analyzer.
pub mod table_caption_analyzers;
/// V2/V3/V4 analyzers for content scoring, security deep analysis, and SEO analysis.
pub mod v2_analyzers;
/// X-header analyzers: X-Content-Type-Options, X-Permitted-Cross-Domain-Policies,
/// Cross-Origin-Resource-Policy (extracted from security_analyzers).
pub mod x_header_analyzers;

pub use crate::advanced_canonical::{CanonicalChainDetector, HreflangReciprocalValidator};
pub use accessibility_deep_analyzers::{
    AriaLandmarksAnalyzer, FocusManagementDeepAnalyzer, FormLabelsDeepAnalyzer,
    HeadingHierarchyDeepAnalyzer, ImageAltTextDeepAnalyzer, LanguageAttributesDeepAnalyzer,
    LinkTextQualityAnalyzer, TableAccessibilityDeepAnalyzer,
};
pub use accessibility_visual_analyzers::{ColorContrastAnalyzer, FocusOrderAnalyzer};
pub use aria_analyzers::AriaRolesAnalyzer;
pub use aria_label_analyzers::AriaLabelAnalyzer;
pub use basic_accessibility_analyzers::{
    AriaRoleAnalyzer, FormInputLabelAnalyzer, ImageAltTextAnalyzer, LinkTextAnalyzer,
};
pub use content_analyzers::{
    ArticleAuthorValidator, ArticleDatePublishedValidator, ArticleHeadlineValidator,
    ArticleQualityAnalyzer, BreadcrumbActivePageValidator, BreadcrumbListDepthAnalyzer,
    CanonicalChainValidator, CanonicalConsistencyAnalyzer, ContentDepthAnalyzer,
    ContentFreshnessScorer, ContentFreshnessSignalAnalyzer, ContentLanguageValidator,
    ContentQualityAnalyzer, ContentReadabilityScorer, ContentStructureAnalyzer,
    ContentThinAnalyzer, ContentTopicCoverageAnalyzer, CourseNameValidator,
    DuplicateContentDetector, EnhancedReadabilityAnalyzer, EntityAnalyzer, EntityLinkingAnalyzer,
    ExternalLinkQualityAnalyzer, HeadingCoverageAnalyzer, HreflangNetworkValidator,
    InternalLinkDepthAnalyzerV2, JobPostingTitleValidator, JsonLdContextValidator,
    JsonLdTypeValidator, JsonLdValidator, KeywordDensityAnalyzer, KeywordProminenceAnalyzer,
    MetaDescriptionLengthAnalyzer, MetaRobotsValidationAnalyzer, MetaRobotsValidator,
    MicrodataValidator, MobileFriendlinessScoreAnalyzer, OpenGraphVideoUrlValidator,
    OrganizationNameValidator, PageImportanceAnalyzer, PageSpeedScoreAnalyzer, PersonNameValidator,
    RdfaValidator, RecipeNameValidator, SchemaIdReferenceValidator, SchemaNestingDepthValidator,
    StructuredDataValidator, TableOfContentsAnalyzer, TitleLengthAnalyzer,
    TwitterPlayerStreamValidator,
};
pub use cookies::{
    CookieAnalyzer, CookieHttpOnlyFlagValidator, CookieSecureFlagValidator,
    CookieSecurityFlagAnalyzer,
};
pub use deep_security_header_analyzers::{
    CrossOriginIsolationDeepAnalyzer, PermissionsPolicyDeepAnalyzer, ReferrerPolicyDeepAnalyzer,
    XContentTypeOptionsDeepAnalyzer, XFrameOptionsDeepAnalyzer,
};
pub use dns_sri_cors_analyzers::{
    CorsMisconfigurationAnalyzer, DnsRebindingAnalyzer, SubresourceIntegrityAnalyzer,
};
pub use focus_management_analyzers::FocusManagementAnalyzer;
pub use font_analyzers::FontSizeAnalyzer;
pub use form_analyzers::FormLabelAnalyzer;
pub use heading_analyzers::HeadingOrderAnalyzer;
pub use hsts_analyzer::HstsPreloadAnalyzer;
pub use http_analyzers::{
    CacheHeaderAnalyzer, CompressionAnalyzer, HttpStatusAnalyzer, HttpVersionAnalyzer,
    RedirectChainAnalyzer, ResponseSizeAnalyzer, RobotsRule, RobotsTxtAnalyzer,
    ServerHeaderAnalyzer, SslCertificateValidator, TtfbAnalyzer,
};
pub use image_accessibility_analyzers::ImageAccessibilityAnalyzer;
pub use landmark_analyzers::LandmarkRegionsAnalyzer;
pub use link_accessibility_analyzers::LinkAccessibilityAnalyzer;
pub use media_analyzers::{
    AggregateRatingValidator, AsyncScriptAnalyzer, BlockingStyleAnalyzer, ConnectionAnalyzer,
    CriticalResourceAnalyzer, EcommerceSignalsAnalyzer, FontDisplayAnalyzer, FontDisplayAnalyzerV2,
    FormAnalyzer, ImageAnalyzer, ImageAspectRatioValidator, ImageDimensionMissingAnalyzer,
    ImageFileSizeValidator, ImageInfo, ImageLazyLoadAnalyzer, ImageLazyLoadAnalyzerV2,
    PreloadHintAnalyzer, PricingSchemaValidator, ProductVariantAnalyzer, ResourceCountAnalyzer,
    ResourceSizeAnalyzer, ScriptAnalyzer, ScriptLoadAnalyzerV2, StylesheetAnalyzer,
    ThirdPartyResourceAnalyzer,
};
pub use mixed_content_analyzers::{HstsPreloadReadinessAnalyzer, MixedContentDetectionAnalyzer};
pub use mobile_analyzers::MobileFriendlinessChecker;
pub use permissions_policy_v2_analyzers::PermissionsPolicyAnalyzerV2;
pub use post_crawl_analyzers::{
    build_post_crawl_registry, CannibalizationDetector, CrawlData, PostCrawlAnalyzer,
    PostCrawlAnalyzerRegistry, SitemapCoverageAnalyzer,
};
pub use schema::*;
pub use security_analyzers::{
    AccessibilityAnalyzer, AnchorTextGenericAnalyzer, AriaRequiredAttributesAnalyzer,
    AriaRolesAnalyzerV2, ColorContrastLinkAnalyzer, ColorContrastTextAnalyzer,
    ContentSecurityPolicyAnalyzerV2, CrossOriginIsolationAnalyzerV2,
    CrossOriginOpenerPolicyAnalyzerV2, CspDirectiveValidator, FocusOrderPositiveTabindexAnalyzer,
    FormAccessibilityAnalyzerV2, FormLabelAssociationAnalyzer, HeadingHierarchyAnalyzerV2,
    HeadingLevelSkipAnalyzer, ImageAccessibilityAnalyzerV2, LandmarkBannerAnalyzer,
    LandmarkMainAnalyzer, LandmarkNavAnalyzer, LanguageAttributeAnalyzerV2,
    LinkAccessibilityAnalyzerV2, MixedContentFormValidator, MixedContentImageValidator,
    MixedContentScriptValidator, PermissionsPolicyAnalyzerV3, ReferrerPolicyAnalyzerV2,
    SecurityHeaderAnalyzer, StrictTransportSecurityAnalyzerV3, TabindexAnalyzerV2,
    TableAccessibilityAnalyzerV2, TableCaptionPresenceAnalyzer, TableHeaderScopeAnalyzer,
    XContentTypeOptionsAnalyzerV2, XFrameOptionsAnalyzerV2,
};
pub use security_header_analyzers::{
    ContentSecurityPolicyAnalyzer, CrossOriginIsolationAnalyzer, MixedContentAnalyzer,
    PermissionPolicyAnalyzer, ReferrerPolicyAnalyzer, SriAnalyzer, XFrameOptionsAnalyzer,
};
pub use security_header_v2_analyzers::{
    ContentTypeSniffingAnalyzerV2, HstsPreloadListValidator, StrictTransportSecurityAnalyzerV2,
    XssProtectionAnalyzerV2,
};
pub use seo_analyzers::{
    AnchorTextDiversityAnalyzer, CanonicalDepthAnalyzer, CanonicalSelfReferenceAnalyzerV2,
    CanonicalUrlAnalyzerV2, CanonicalUrlValidator, CanonicalValidationDeepAnalyzer,
    CharsetValidator, ContentQualityAnalyzerV2, ExternalLinkAnalyzerV2,
    ExternalLinkAuthorityAnalyzer, ExternalLinkAuthorityDeepAnalyzer,
    ExternalNofollowUnderuseValidator, HeadingHierarchyAnalyzer, HreflangConsistencyAnalyzer,
    HreflangReciprocalAnalyzerV2, HreflangSelfReferenceValidator, HreflangValidator,
    HreflangValidatorV3, InternalLinkAnalyzerV2, InternalLinkAnchorAnalyzer,
    InternalLinkAnchorTextDiversityAnalyzer, InternalLinkQualityAnalyzer,
    InternalLinkTopicalAnalyzer, InternalNofollowOveruseValidator, InternationalSeoAnalyzer,
    KeywordAnalyzer, KeywordAnalyzerV2, LanguageAttributeAnalyzer, LinkAnalyzer, LinkAnalyzerV2,
    LinkInfo, MetaDescriptionAnalyzerV3, MetaDescriptionDeepAnalyzer,
    MetaDescriptionLengthAnalyzerV2, MetaDescriptionPixelWidthAnalyzer, MetaTagAnalyzer,
    MixedProtocolRedirectValidator, MobileViewportAnalyzer, OpenSearchDescriptionValidator,
    OpenSearchValidator, PaginationAnalyzer, PaginationDepthValidator, PaginationLinkAnalyzer,
    RedirectLoopDetector, RobotsMetaAnalyzer, RobotsTxtAnalysisDeepAnalyzer, RobotsTxtAnalyzerV2,
    RobotsTxtCoverageAnalyzerV2, RobotsTxtDirectivesAnalyzer, RobotsTxtSizeValidator,
    SitemapAnalyzer, SitemapAnalyzerV2, SitemapCoverageAnalyzerV2, SitemapCoverageDeepAnalyzer,
    SitemapEntry, SitemapUrlAnalyzer, SitemapXmlSizeValidator, TitleAnalysisDeepAnalyzer,
    TitleAnalyzerV3, TitleKeywordAnalyzer, TitlePixelWidthAnalyzer, WikipediaLinkAnalyzer,
    WordCountAnalyzer, WordCountAnalyzerV2,
};
pub use skip_link_analyzers::SkipLinkAnalyzer;
pub use social_analyzers::{
    OpenGraphAudioAnalyzer, OpenGraphImageValidator, OpenGraphSiteNameValidator,
    OpenGraphUrlValidator, OpenGraphVideoAnalyzer, SocialMediaAnalyzer, SocialPreviewOptimizer,
    TwitterCardTypeAnalyzer, TwitterPlayerValidator, TwitterSiteAnalyzer,
};
pub use sts_analyzers::{
    CertificateTransparencyAnalyzer, ContentTypeSniffingAnalyzer, CorsPolicyAnalyzer,
    CrossOriginEmbedderPolicyAnalyzer, CrossOriginOpenerPolicyAnalyzer, CspDirectiveAnalyzer,
    ExpectCTAnalyzer, FeaturePolicyAnalyzer, PermissionsPolicyAnalyzerNew,
    StrictTransportSecurityAnalyzer, XSSProtectionAnalyzer,
};
pub use tabindex_analyzers::TabindexAnalyzer;
pub use table_analyzers::TableAccessibilityAnalyzer;
pub use table_caption_analyzers::TableCaptionAnalyzer;
pub use v2_analyzers::*;
pub use x_header_analyzers::{
    CrossOriginResourcePolicyAnalyzer, XContentTypeOptionsAnalyzer,
    XPermittedCrossDomainPoliciesAnalyzer,
};

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
pub(crate) fn count_sentences(text: &str) -> usize {
    if text.trim().is_empty() {
        return 1;
    }
    text.chars()
        .filter(|&c| c == '.' || c == '!' || c == '?')
        .count()
        .max(1)
}

/// Count syllables in a word (approximate heuristic).
pub(crate) fn count_syllables(word: &str) -> usize {
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
pub(crate) fn flesch_kincaid_grade(words: usize, sentences: usize, syllables: usize) -> f64 {
    if words == 0 || sentences == 0 {
        return 0.0;
    }
    0.39 * (words as f64 / sentences as f64) + 11.8 * (syllables as f64 / words as f64) - 15.59
}

/// Flesch Reading Ease score (0-100, higher = easier).
pub(crate) fn flesch_reading_ease(words: usize, sentences: usize, syllables: usize) -> f64 {
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
    /// Rendered page data from JS rendering (if available).
    pub rendered: Option<&'a crate::playwright::RenderedPage>,
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
/// in parallel using rayon. The default registry includes 423 analyzers
/// covering HTTP, SEO, content, links, images, security, accessibility,
/// social media, AI-specific checks, and structured data validation.
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
            // Additional SEO
            Box::new(InternalLinkAnchorAnalyzer::new()),
            Box::new(WikipediaLinkAnalyzer::new()),
            Box::new(AnchorTextDiversityAnalyzer::new()),
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
            // Schema validators: Article, Organization, Person, JobPosting, Course, Recipe
            Box::new(ArticleSchemaValidator::new()),
            Box::new(OrganizationSchemaValidator::new()),
            Box::new(PersonSchemaValidator::new()),
            Box::new(JobPostingSchemaValidator::new()),
            Box::new(CourseSchemaValidator::new()),
            Box::new(RecipeSchemaValidator::new()),
            // Content freshness and breadcrumb depth
            Box::new(ContentFreshnessScorer::new()),
            Box::new(BreadcrumbListDepthAnalyzer::new()),
            // SEO: SERP pixel width analysis
            Box::new(TitlePixelWidthAnalyzer::new()),
            Box::new(MetaDescriptionPixelWidthAnalyzer::new()),
            // SEO: internal link topical relevance
            Box::new(InternalLinkTopicalAnalyzer::new()),
            // Content: topic coverage analysis
            Box::new(ContentTopicCoverageAnalyzer::new()),
            // Social: social preview optimizer
            Box::new(SocialPreviewOptimizer::new()),
            // Local business NAP consistency
            Box::new(LocalBusinessNapAnalyzer::new()),
            // Security: CSP, Referrer-Policy, X-Frame-Options deep analysis
            Box::new(ContentSecurityPolicyAnalyzer::new()),
            Box::new(ReferrerPolicyAnalyzer::new()),
            Box::new(XFrameOptionsAnalyzer::new()),
            // SEO: robots.txt directives, sitemap URL analysis
            Box::new(RobotsTxtDirectivesAnalyzer::new()),
            Box::new(SitemapUrlAnalyzer::new()),
            // Social: Open Graph video, Twitter card type
            Box::new(OpenGraphVideoAnalyzer::new()),
            Box::new(TwitterCardTypeAnalyzer::new()),
            // HTTP: compression analysis
            Box::new(CompressionAnalyzer::new()),
            // Content: new schema validators (WebPage, Service, ItemList, Offer, AggregateOffer, Brand, Occupation, Quest, Action, Playbook)
            Box::new(WebPageSchemaValidator::new()),
            Box::new(ServiceSchemaValidator::new()),
            Box::new(ItemListSchemaValidator::new()),
            Box::new(OfferSchemaValidator::new()),
            Box::new(AggregateOfferSchemaValidator::new()),
            Box::new(BrandSchemaValidator::new()),
            Box::new(OccupationSchemaValidator::new()),
            Box::new(QuestSchemaValidator::new()),
            Box::new(ActionSchemaValidator::new()),
            Box::new(PlaybookSchemaValidator::new()),
            // Security: X-Content-Type-Options, X-Permitted-Cross-Domain-Policies, CORP
            Box::new(XContentTypeOptionsAnalyzer::new()),
            Box::new(XPermittedCrossDomainPoliciesAnalyzer::new()),
            Box::new(CrossOriginResourcePolicyAnalyzer::new()),
            // SEO: meta robots, canonical self-reference, pagination rel, hreflang self-reference, OpenSearch description
            Box::new(HreflangSelfReferenceValidator::new()),
            Box::new(OpenSearchDescriptionValidator::new()),
            // Social: OG audio, twitter:site
            Box::new(OpenGraphAudioAnalyzer::new()),
            Box::new(TwitterSiteAnalyzer::new()),
            // HTTP: version and server header analysis
            Box::new(HttpVersionAnalyzer::new()),
            Box::new(ServerHeaderAnalyzer::new()),
            // Performance: preload hints, async scripts, lazy loading, font display
            Box::new(PreloadHintAnalyzer::new()),
            Box::new(AsyncScriptAnalyzer::new()),
            Box::new(ImageLazyLoadAnalyzer::new()),
            Box::new(FontDisplayAnalyzer::new()),
            Box::new(ResourceSizeAnalyzer::new()),
            Box::new(ConnectionAnalyzer::new()),
            // Security: mixed content and cookies
            Box::new(MixedContentAnalyzer::new()),
            Box::new(CookieAnalyzer::new()),
            // Accessibility: landmark regions, heading order, form labels, table accessibility
            Box::new(LandmarkRegionsAnalyzer::new()),
            Box::new(HeadingOrderAnalyzer::new()),
            Box::new(FormLabelAnalyzer::new()),
            Box::new(TableAccessibilityAnalyzer::new()),
            // Accessibility: link, image, ARIA roles, focus management, language attribute
            Box::new(LinkAccessibilityAnalyzer::new()),
            Box::new(ImageAccessibilityAnalyzer::new()),
            Box::new(AriaRolesAnalyzer::new()),
            Box::new(FocusManagementAnalyzer::new()),
            // Schema validators: Apartment, Car, MusicAlbum, TVSeries, Movie
            Box::new(ApartmentSchemaValidator::new()),
            Box::new(CarSchemaValidator::new()),
            Box::new(MusicAlbumSchemaValidator::new()),
            Box::new(TVSeriesSchemaValidator::new()),
            Box::new(MovieSchemaValidator::new()),
            // Schema validators: GovernmentService, HealthPlan, Invoice, Permit, Plan
            Box::new(GovernmentServiceSchemaValidator::new()),
            Box::new(HealthPlanSchemaValidator::new()),
            Box::new(InvoiceSchemaValidator::new()),
            Box::new(PermitSchemaValidator::new()),
            Box::new(PlanSchemaValidator::new()),
            // Schema validators: ProductModel, ResearchProject, Schedule, Trip, WorkersUnion
            Box::new(ProductModelSchemaValidator::new()),
            Box::new(ResearchProjectSchemaValidator::new()),
            Box::new(ScheduleSchemaValidator::new()),
            Box::new(TripSchemaValidator::new()),
            Box::new(WorkersUnionSchemaValidator::new()),
            // Schema validators: WebAPI, Wearable, WebPageElement, WebSite, Worker
            Box::new(WebAPISchemaValidator::new()),
            Box::new(WearableSchemaValidator::new()),
            Box::new(WebPageElementSchemaValidator::new()),
            Box::new(WebSiteSchemaValidator::new()),
            Box::new(WorkerSchemaValidator::new()),
            // Additional schema validators
            Box::new(LocalBusinessHoursValidator::new()),
            Box::new(ProductReviewValidator::new()),
            Box::new(EventLocationValidator::new()),
            Box::new(OrganizationLogoValidator::new()),
            Box::new(PersonJobTitleValidator::new()),
            Box::new(RecipeNutritionValidator::new()),
            Box::new(CourseProviderValidator::new()),
            Box::new(JobPostingSalaryValidator::new()),
            // Media analyzers
            Box::new(ImageAspectRatioValidator::new()),
            Box::new(ImageFileSizeValidator::new()),
            Box::new(ScriptAnalyzer::new()),
            Box::new(StylesheetAnalyzer::new()),
            Box::new(FormAnalyzer::new()),
            // Security: HSTS, XSS protection, content-type sniffing
            Box::new(StrictTransportSecurityAnalyzer::new()),
            Box::new(XSSProtectionAnalyzer::new()),
            Box::new(ContentTypeSniffingAnalyzer::new()),
            Box::new(FeaturePolicyAnalyzer::new()),
            Box::new(ExpectCTAnalyzer::new()),
            Box::new(CertificateTransparencyAnalyzer::new()),
            // Content analyzers: OG video URL, Twitter player stream, schema nesting, schema @id ref, breadcrumb active page, content language
            Box::new(OpenGraphVideoUrlValidator::new()),
            Box::new(TwitterPlayerStreamValidator::new()),
            Box::new(SchemaNestingDepthValidator::new()),
            Box::new(SchemaIdReferenceValidator::new()),
            Box::new(BreadcrumbActivePageValidator::new()),
            Box::new(ContentLanguageValidator::new()),
            // SEO analyzers: pagination depth, redirect loop, mixed protocol, nofollow overuse/underuse, sitemap size, robots.txt size
            Box::new(PaginationDepthValidator::new()),
            Box::new(RedirectLoopDetector::new()),
            Box::new(MixedProtocolRedirectValidator::new()),
            Box::new(InternalNofollowOveruseValidator::new()),
            Box::new(ExternalNofollowUnderuseValidator::new()),
            Box::new(SitemapXmlSizeValidator::new()),
            Box::new(RobotsTxtSizeValidator::new()),
            // Schema validators: video duration, product availability, FAQ page entity, HowTo step count, event start date, local business NPI, org sameAs
            Box::new(VideoObjectDurationValidator::new()),
            Box::new(ProductAvailabilityValidator::new()),
            Box::new(FaqPageEntityValidator::new()),
            Box::new(HowToStepCountValidator::new()),
            Box::new(EventStartDateValidator::new()),
            Box::new(LocalBusinessNpiValidator::new()),
            Box::new(OrganizationSameAsValidator::new()),
            // Security: DNS rebinding, SRI scripts, CORS misconfiguration
            Box::new(DnsRebindingAnalyzer::new()),
            Box::new(SubresourceIntegrityAnalyzer::new()),
            Box::new(CorsMisconfigurationAnalyzer::new()),
            // Performance: third-party resources, blocking styles, image dimensions
            Box::new(ThirdPartyResourceAnalyzer::new()),
            Box::new(BlockingStyleAnalyzer::new()),
            Box::new(ImageDimensionMissingAnalyzer::new()),
            // Accessibility: ARIA labels, table caption, skip link, tabindex
            Box::new(AriaLabelAnalyzer::new()),
            Box::new(TableCaptionAnalyzer::new()),
            Box::new(SkipLinkAnalyzer::new()),
            Box::new(TabindexAnalyzer::new()),
            // Security: HSTS preload list, CSP directives, cookie flags, mixed content (form/script/image)
            Box::new(HstsPreloadListValidator::new()),
            Box::new(CspDirectiveValidator::new()),
            Box::new(CookieSecureFlagValidator::new()),
            Box::new(CookieHttpOnlyFlagValidator::new()),
            Box::new(MixedContentFormValidator::new()),
            Box::new(MixedContentScriptValidator::new()),
            Box::new(MixedContentImageValidator::new()),
            // Accessibility: landmarks, heading skip, form labels, table scope/caption, anchor text, ARIA, tabindex, contrast
            Box::new(LandmarkMainAnalyzer::new()),
            Box::new(LandmarkNavAnalyzer::new()),
            Box::new(LandmarkBannerAnalyzer::new()),
            Box::new(HeadingLevelSkipAnalyzer::new()),
            Box::new(FormLabelAssociationAnalyzer::new()),
            Box::new(TableHeaderScopeAnalyzer::new()),
            Box::new(TableCaptionPresenceAnalyzer::new()),
            Box::new(AnchorTextGenericAnalyzer::new()),
            Box::new(AriaRequiredAttributesAnalyzer::new()),
            Box::new(FocusOrderPositiveTabindexAnalyzer::new()),
            Box::new(ColorContrastTextAnalyzer::new()),
            Box::new(ColorContrastLinkAnalyzer::new()),
            // SEO: meta description pixel width, title keyword, canonical self-ref, hreflang reciprocal, anchor diversity, external authority, sitemap/robots coverage, pagination
            Box::new(MetaDescriptionLengthAnalyzerV2::new()),
            Box::new(TitleKeywordAnalyzer::new()),
            Box::new(CanonicalSelfReferenceAnalyzerV2::new()),
            Box::new(HreflangReciprocalAnalyzerV2::new()),
            Box::new(InternalLinkAnchorTextDiversityAnalyzer::new()),
            Box::new(ExternalLinkAuthorityAnalyzer::new()),
            Box::new(SitemapCoverageAnalyzerV2::empty()),
            Box::new(RobotsTxtCoverageAnalyzerV2::new()),
            Box::new(PaginationLinkAnalyzer::new()),
            // Schema validators V2: Book, Movie, TVEpisode, MusicRecording, Service, HealthPlan
            Box::new(BookSchemaValidatorV2::new()),
            Box::new(MovieSchemaValidatorV2::new()),
            Box::new(TVSeriesEpisodeSchemaValidator::new()),
            Box::new(MusicRecordingSchemaValidator::new()),
            Box::new(ServiceSchemaValidatorV2::new()),
            Box::new(HealthPlanSchemaValidatorV2::new()),
            // Schema validators V2: Invoice, Permit, Plan, ResearchProject, Schedule, Trip
            Box::new(InvoiceSchemaValidatorV2::new()),
            Box::new(PermitSchemaValidatorV2::new()),
            Box::new(PlanSchemaValidatorV2::new()),
            Box::new(ResearchProjectSchemaValidatorV2::new()),
            Box::new(ScheduleSchemaValidatorV2::new()),
            Box::new(TripSchemaValidatorV2::new()),
            // Schema validators V2: WorkersUnion, WebAPI, Wearable, WebPageElement, WebSite, Worker
            Box::new(WorkersUnionSchemaValidatorV2::new()),
            Box::new(WebAPISchemaValidatorV2::new()),
            Box::new(WearableSchemaValidatorV2::new()),
            Box::new(WebPageElementSchemaValidatorV2::new()),
            Box::new(WebSiteSchemaValidatorV2::new()),
            Box::new(WorkerSchemaValidatorV2::new()),
            // Content analyzers: JsonLdContext, JsonLdType, MetaRobots, CanonicalChain
            Box::new(JsonLdContextValidator::new()),
            Box::new(JsonLdTypeValidator::new()),
            Box::new(MetaRobotsValidator::new()),
            Box::new(CanonicalChainValidator::new()),
            // Content analyzers: InternalLinkDepth, ExternalLinkQuality, ContentStructure
            Box::new(InternalLinkDepthAnalyzerV2::new()),
            Box::new(ExternalLinkQualityAnalyzer::new()),
            Box::new(ContentStructureAnalyzer::new()),
            // Content analyzers: KeywordDensity, PageSpeed, MobileFriendliness
            Box::new(KeywordDensityAnalyzer::new()),
            Box::new(PageSpeedScoreAnalyzer::new()),
            Box::new(MobileFriendlinessScoreAnalyzer::new()),
            // Schema validators V2: Apartment, Car, MusicAlbum, TVSeries, GovernmentService, ProductModel, Occupation, Action, Playbook
            Box::new(ApartmentSchemaValidatorV2::new()),
            Box::new(CarSchemaValidatorV2::new()),
            Box::new(MusicAlbumSchemaValidatorV2::new()),
            Box::new(TVSeriesSchemaValidatorV2::new()),
            Box::new(GovernmentServiceSchemaValidatorV2::new()),
            Box::new(ProductModelSchemaValidatorV2::new()),
            Box::new(OccupationSchemaValidatorV2::new()),
            Box::new(ActionSchemaValidatorV2::new()),
            Box::new(PlaybookSchemaValidatorV2::new()),
            // Schema validators: CreativeWork, Playlist, FoodEstablishment, LodgingBusiness, SportsActivityLocation, CivicStructure
            Box::new(CreativeWorkSchemaValidator::new()),
            Box::new(PlaylistSchemaValidator::new()),
            Box::new(FoodEstablishmentSchemaValidator::new()),
            Box::new(LodgingBusinessSchemaValidator::new()),
            Box::new(SportsActivityLocationSchemaValidator::new()),
            Box::new(CivicStructureSchemaValidator::new()),
            // Schema validators: Landform, LandmarksOrHistoricalBuildings, TouristAttraction, TouristDestination
            Box::new(LandformSchemaValidator::new()),
            Box::new(LandmarksOrHistoricalBuildingsSchemaValidator::new()),
            Box::new(TouristAttractionSchemaValidator::new()),
            Box::new(TouristDestinationSchemaValidator::new()),
            // Schema validators: SportsEvent, EducationalOrganization, NGO, PerformingArtsSeries, BroadcastEvent
            Box::new(SportsEventSchemaValidator::new()),
            Box::new(EducationalOrganizationSchemaValidator::new()),
            Box::new(NGOSchemaValidator::new()),
            Box::new(PerformingArtsSeriesSchemaValidator::new()),
            Box::new(BroadcastEventSchemaValidator::new()),
            // Content analyzers: Article author/date/headline, Organization, Person, JobPosting title, Course name, Recipe name
            Box::new(ArticleAuthorValidator::new()),
            Box::new(ArticleDatePublishedValidator::new()),
            Box::new(ArticleHeadlineValidator::new()),
            Box::new(OrganizationNameValidator::new()),
            Box::new(PersonNameValidator::new()),
            Box::new(JobPostingTitleValidator::new()),
            Box::new(CourseNameValidator::new()),
            Box::new(RecipeNameValidator::new()),
            // Security V2/V3: Referrer-Policy, X-Frame-Options, CSP script-src, HSTS includeSubDomains, X-Content-Type-Options, Permissions-Policy camera, COEP, COOP
            Box::new(ReferrerPolicyAnalyzerV2::new()),
            Box::new(XFrameOptionsAnalyzerV2::new()),
            Box::new(ContentSecurityPolicyAnalyzerV2::new()),
            Box::new(StrictTransportSecurityAnalyzerV3::new()),
            Box::new(XContentTypeOptionsAnalyzerV2::new()),
            Box::new(PermissionsPolicyAnalyzerV3::new()),
            Box::new(CrossOriginIsolationAnalyzerV2::new()),
            Box::new(CrossOriginOpenerPolicyAnalyzerV2::new()),
            // Accessibility V2: Tabindex, Link, Image, Form, Table, ARIA roles, Heading hierarchy, Language
            Box::new(TabindexAnalyzerV2::new()),
            Box::new(LinkAccessibilityAnalyzerV2::new()),
            Box::new(ImageAccessibilityAnalyzerV2::new()),
            Box::new(FormAccessibilityAnalyzerV2::new()),
            Box::new(TableAccessibilityAnalyzerV2::new()),
            Box::new(AriaRolesAnalyzerV2::new()),
            Box::new(HeadingHierarchyAnalyzerV2::new()),
            Box::new(LanguageAttributeAnalyzerV2::new()),
            // SEO V2/V3: Meta description, Title, Canonical, Hreflang, Sitemap, Robots.txt, Internal/External links, Keyword, ContentQuality, WordCount, Link
            Box::new(MetaDescriptionAnalyzerV3::new()),
            Box::new(TitleAnalyzerV3::new()),
            Box::new(CanonicalUrlAnalyzerV2::new()),
            Box::new(HreflangValidatorV3::new()),
            Box::new(SitemapAnalyzerV2::new()),
            Box::new(RobotsTxtAnalyzerV2::new()),
            Box::new(InternalLinkAnalyzerV2::new()),
            Box::new(ExternalLinkAnalyzerV2::new()),
            Box::new(KeywordAnalyzerV2::new()),
            Box::new(ContentQualityAnalyzerV2::new()),
            Box::new(WordCountAnalyzerV2::new()),
            Box::new(LinkAnalyzerV2::new()),
            // Performance V2: Image lazy load, Script load, Font display
            Box::new(ImageLazyLoadAnalyzerV2::new()),
            Box::new(ScriptLoadAnalyzerV2::new()),
            Box::new(FontDisplayAnalyzerV2::new()),
            // Content analyzers: ArticleQuality, ContentDepth, HeadingCoverage, KeywordProminence, etc.
            Box::new(ArticleQualityAnalyzer::new()),
            Box::new(ContentDepthAnalyzer::new()),
            Box::new(HeadingCoverageAnalyzer::new()),
            Box::new(KeywordProminenceAnalyzer::new()),
            Box::new(ContentFreshnessSignalAnalyzer::new()),
            Box::new(MetaRobotsValidationAnalyzer::new()),
            Box::new(CanonicalConsistencyAnalyzer::new()),
            Box::new(HreflangNetworkValidator::new()),
            Box::new(ContentReadabilityScorer::new()),
            Box::new(PageImportanceAnalyzer::new()),
            // Security: CSP directive, CORS policy, cookie flags, mixed content detection, HSTS preload readiness
            Box::new(CspDirectiveAnalyzer::new()),
            Box::new(CorsPolicyAnalyzer::new()),
            Box::new(CookieSecurityFlagAnalyzer::new()),
            Box::new(MixedContentDetectionAnalyzer::new()),
            Box::new(HstsPreloadReadinessAnalyzer::new()),
            // Security: X-Content-Type deep, Referrer-Policy deep, X-Frame-Options deep, Permissions-Policy deep, COI deep
            Box::new(XContentTypeOptionsDeepAnalyzer::new()),
            Box::new(ReferrerPolicyDeepAnalyzer::new()),
            Box::new(XFrameOptionsDeepAnalyzer::new()),
            Box::new(PermissionsPolicyDeepAnalyzer::new()),
            Box::new(CrossOriginIsolationDeepAnalyzer::new()),
            // Accessibility: ARIA landmarks, heading hierarchy deep, form labels deep, table accessibility deep
            Box::new(AriaLandmarksAnalyzer::new()),
            Box::new(HeadingHierarchyDeepAnalyzer::new()),
            Box::new(FormLabelsDeepAnalyzer::new()),
            Box::new(TableAccessibilityDeepAnalyzer::new()),
            // Accessibility: link text quality, image alt text deep, focus management deep, language attributes deep
            Box::new(LinkTextQualityAnalyzer::new()),
            Box::new(ImageAltTextDeepAnalyzer::new()),
            Box::new(FocusManagementDeepAnalyzer::new()),
            Box::new(LanguageAttributesDeepAnalyzer::new()),
            // SEO: title analysis deep, meta description deep, canonical validation deep, sitemap coverage deep, robots.txt deep, internal/external link quality
            Box::new(TitleAnalysisDeepAnalyzer::new()),
            Box::new(MetaDescriptionDeepAnalyzer::new()),
            Box::new(CanonicalValidationDeepAnalyzer::new()),
            Box::new(SitemapCoverageDeepAnalyzer::new()),
            Box::new(RobotsTxtAnalysisDeepAnalyzer::new()),
            Box::new(InternalLinkQualityAnalyzer::new()),
            Box::new(ExternalLinkAuthorityDeepAnalyzer::new()),
            // V2/V3/V4 content score analyzers
            Box::new(DuplicateContentDetectorV2::new()),
            Box::new(ContentFreshnessScoreAnalyzer::new()),
            Box::new(HeadingStructureScoreAnalyzer::new()),
            Box::new(LinkQualityScoreAnalyzer::new()),
            Box::new(SchemaCoverageScoreAnalyzer::new()),
            Box::new(SecurityScoreAnalyzer::new()),
            Box::new(AccessibilityScoreAnalyzer::new()),
            // V2 security analyzers
            Box::new(CspDirectiveAnalyzerV2::new()),
            Box::new(CorsPolicyAnalyzerV2::new()),
            Box::new(CookieSecurityFlagAnalyzerV2::new()),
            Box::new(MixedContentDetectionAnalyzerV2::new()),
            Box::new(HstsPreloadReadinessAnalyzerV2::new()),
            Box::new(XContentTypeOptionsDeepAnalyzerV2::new()),
            Box::new(ReferrerPolicyDeepAnalyzerV2::new()),
            Box::new(XFrameOptionsDeepAnalyzerV2::new()),
            Box::new(PermissionsPolicyDeepAnalyzerV2::new()),
            Box::new(CrossOriginIsolationDeepAnalyzerV2::new()),
            Box::new(AriaLandmarksAnalyzerV2::new()),
            Box::new(HeadingHierarchyDeepAnalyzerV2::new()),
            Box::new(FormLabelsDeepAnalyzerV2::new()),
            Box::new(TableAccessibilityDeepAnalyzerV2::new()),
            Box::new(LinkTextQualityAnalyzerV2::new()),
            Box::new(ImageAltTextDeepAnalyzerV2::new()),
            Box::new(FocusManagementDeepAnalyzerV2::new()),
            Box::new(LanguageAttributesDeepAnalyzerV2::new()),
            Box::new(ColorContrastTextAnalyzerV2::new()),
            Box::new(ColorContrastLinkAnalyzerV2::new()),
            Box::new(AnchorTextGenericAnalyzerV2::new()),
            Box::new(TableCaptionPresenceAnalyzerV2::new()),
            Box::new(TableHeaderScopeAnalyzerV2::new()),
            Box::new(FormLabelAssociationAnalyzerV2::new()),
            Box::new(AriaRequiredAttributesAnalyzerV2::new()),
            // V2/V3/V4 SEO analyzers
            Box::new(TitleAnalysisDeepAnalyzerV2::new()),
            Box::new(MetaDescriptionDeepAnalyzerV2::new()),
            Box::new(CanonicalValidationDeepAnalyzerV2::new()),
            Box::new(SitemapCoverageDeepAnalyzerV2::new()),
            Box::new(RobotsTxtAnalysisDeepAnalyzerV2::new()),
            Box::new(InternalLinkQualityAnalyzerV2::new()),
            Box::new(ExternalLinkAuthorityDeepAnalyzerV2::new()),
            Box::new(TitleLengthQualityAnalyzerV2::new()),
            Box::new(MetaDescriptionQualityAnalyzerV2::new()),
            Box::new(InternalLinkAnchorAnalyzerV3::new()),
            Box::new(WikipediaLinkAnalyzerV3::new()),
            Box::new(PaginationDepthAnalyzerV2::new()),
            Box::new(MixedProtocolRedirectValidatorV2::new()),
            Box::new(InternalNofollowOveruseValidatorV2::new()),
            Box::new(SitemapXmlSizeValidatorV2::new()),
            Box::new(RobotsTxtSizeValidatorV2::new()),
            Box::new(HreflangSelfReferenceValidatorV2::new()),
            Box::new(OpenSearchDescriptionValidatorV2::new()),
            Box::new(CanonicalDepthAnalyzerV2::new()),
            Box::new(MetaDescriptionLengthAnalyzerV3::new()),
            Box::new(TitleAnalyzerV4::new()),
            Box::new(CanonicalUrlAnalyzerV3::new()),
            Box::new(HreflangValidatorV4::new()),
            Box::new(SitemapAnalyzerV3::new()),
            Box::new(RobotsTxtAnalyzerV3::new()),
            // V5 Content Analyzers (1-30)
            Box::new(ArticleWordCountAnalyzer::new()),
            Box::new(ArticleAuthorUrlValidator::new()),
            Box::new(ArticleDateModifiedValidator::new()),
            Box::new(OrganizationUrlValidatorV5::new()),
            Box::new(OrganizationLogoUrlValidator::new()),
            Box::new(OrganizationContactValidator::new()),
            Box::new(PersonUrlValidator::new()),
            Box::new(JobPostingEmploymentTypeValidator::new()),
            Box::new(JobPostingSalaryCurrencyValidator::new()),
            Box::new(JobPostingValidThroughValidator::new()),
            Box::new(CourseDescriptionValidator::new()),
            Box::new(CourseProviderNameValidator::new()),
            Box::new(RecipePrepTimeValidator::new()),
            Box::new(RecipeIngredientsValidator::new()),
            Box::new(ProductPriceValidator::new()),
            Box::new(ProductImageValidatorV5::new()),
            Box::new(BreadcrumbItemCountValidator::new()),
            Box::new(BreadcrumbUrlValidator::new()),
            Box::new(EventOrganizerValidatorV5::new()),
            Box::new(EventPerformerValidator::new()),
            Box::new(VideoThumbnailValidator::new()),
            Box::new(VideoDurationFormatValidator::new()),
            Box::new(SoftwareOffersValidatorV5::new()),
            Box::new(SoftwareScreenshotValidator::new()),
            Box::new(FAQAnswerLengthValidator::new()),
            Box::new(HowToNameValidator::new()),
            Box::new(HowToStepDescriptionValidator::new()),
            Box::new(DatasetLicenseValidator::new()),
            Box::new(DatasetDistributionValidator::new()),
            // V5 Security Analyzers (31-50)
            Box::new(CspScriptSrcValidator::new()),
            Box::new(CspStyleSrcValidator::new()),
            Box::new(CspFrameAncestorsValidator::new()),
            Box::new(HstsMaxAgeValidator::new()),
            Box::new(HstsIncludeSubDomainsValidator::new()),
            Box::new(HstsPreloadValidatorV5::new()),
            Box::new(XContentTypeOptionsValidatorV5::new()),
            Box::new(ReferrerPolicyValidatorV5::new()),
            Box::new(XFrameOptionsValidatorV5::new()),
            Box::new(PermissionsPolicyCameraValidator::new()),
            Box::new(PermissionsPolicyMicrophoneValidator::new()),
            Box::new(PermissionsPolicyGeolocationValidator::new()),
            Box::new(CoepValidator::new()),
            Box::new(CoopValidator::new()),
            Box::new(CookieSecureFlagValidatorV5::new()),
            Box::new(CookieHttpOnlyFlagValidatorV5::new()),
            Box::new(CookieSameSiteValidator::new()),
            Box::new(MixedContentScriptValidatorV5::new()),
            Box::new(MixedContentStylesheetValidator::new()),
            Box::new(SriValidator::new()),
            // V5 SEO Analyzers (51-65)
            Box::new(TitleKeywordPresenceValidator::new()),
            Box::new(TitleBrandValidator::new()),
            Box::new(TitleLengthValidatorV5::new()),
            Box::new(MetaDescriptionKeywordValidator::new()),
            Box::new(MetaDescriptionUniqueValidator::new()),
            Box::new(CanonicalSelfReferenceValidatorV5::new()),
            Box::new(CanonicalChainValidatorV5::new()),
            Box::new(CanonicalDepthValidatorV5::new()),
            Box::new(HreflangReciprocalValidatorV5::new()),
            Box::new(HreflangXDefaultValidator::new()),
            Box::new(HreflangLocaleFormatValidator::new()),
            Box::new(SitemapCoverageValidatorV5::new()),
            Box::new(SitemapLastmodValidator::new()),
            Box::new(SitemapPriorityValidator::new()),
            Box::new(RobotsTxtDisallowValidator::new()),
            // V5 Accessibility Analyzers (66-75)
            Box::new(LandmarkMainValidatorV5::new()),
            Box::new(LandmarkNavValidatorV5::new()),
            Box::new(LandmarkBannerValidatorV5::new()),
            Box::new(HeadingSkipLevelsValidator::new()),
            Box::new(HeadingMultipleH1Validator::new()),
            Box::new(FormLabelAssociationValidatorV5::new()),
            Box::new(FormRequiredFieldsValidator::new()),
            Box::new(TableHeadersValidatorV5::new()),
            Box::new(TableCaptionValidatorV5::new()),
            Box::new(TableScopeValidator::new()),
            // V5 Performance Analyzers (76-80)
            Box::new(PreconnectHintValidator::new()),
            Box::new(DnsPrefetchHintValidator::new()),
            Box::new(ScriptAsyncDeferValidator::new()),
            Box::new(ImageLazyLoadingValidatorV5::new()),
            Box::new(ImageModernFormatValidator::new()),
            Box::new(ImageDimensionsValidatorV5::new()),
            // V6 Content Validators (1-60)
            Box::new(CreativeWorkMissingNameValidator::new()),
            Box::new(CreativeWorkMissingDescriptionValidator::new()),
            Box::new(CreativeWorkMissingDateCreatedValidator::new()),
            Box::new(PlaylistMissingNumberOfItemsValidator::new()),
            Box::new(FoodEstablishmentMissingMenuValidator::new()),
            Box::new(FoodEstablishmentMissingServesCuisineValidator::new()),
            Box::new(LodgingBusinessMissingStarRatingValidator::new()),
            Box::new(LodgingBusinessMissingAmenityFeatureValidator::new()),
            Box::new(SportsActivityLocationMissingSportValidator::new()),
            Box::new(CivicStructureMissingNameValidator::new()),
            Box::new(LandformMissingNameValidator::new()),
            Box::new(LandmarkMissingNameValidator::new()),
            Box::new(TouristAttractionMissingNameValidator::new()),
            Box::new(TouristDestinationMissingNameValidator::new()),
            Box::new(SportsEventMissingSportValidator::new()),
            Box::new(SportsEventMissingNameValidator::new()),
            Box::new(EducationalOrganizationMissingNameValidator::new()),
            Box::new(NGOMissingNameValidator::new()),
            Box::new(PerformingArtsSeriesMissingNameValidator::new()),
            Box::new(BroadcastEventMissingNameValidator::new()),
            Box::new(ProductMissingBrandValidator::new()),
            Box::new(ProductMissingCategoryValidator::new()),
            Box::new(ProductMissingReviewValidator::new()),
            Box::new(BookMissingAuthorValidator::new()),
            Box::new(BookMissingIsbnValidator::new()),
            Box::new(BookMissingDatePublishedValidator::new()),
            Box::new(MovieMissingDirectorValidator::new()),
            Box::new(MovieMissingDurationValidator::new()),
            Box::new(MovieMissingDateCreatedValidator::new()),
            Box::new(TVSeriesMissingNumberOfSeasonsValidator::new()),
            Box::new(TVSeriesMissingEpisodeValidator::new()),
            Box::new(MusicRecordingMissingByArtistValidator::new()),
            Box::new(MusicRecordingMissingAlbumValidator::new()),
            Box::new(ServiceMissingAreaServedValidator::new()),
            Box::new(ServiceMissingProviderValidator::new()),
            Box::new(HealthPlanMissingProviderValidator::new()),
            Box::new(HealthPlanMissingCoverageAreaValidator::new()),
            Box::new(InvoiceMissingAccountValidator::new()),
            Box::new(InvoiceMissingPaymentDueDateValidator::new()),
            Box::new(PermitMissingPermitNumberValidator::new()),
            Box::new(PermitMissingIssuedByValidator::new()),
            Box::new(PlanMissingDescriptionValidator::new()),
            Box::new(PlanMissingAboutValidator::new()),
            Box::new(ResearchProjectMissingAboutValidator::new()),
            Box::new(ResearchProjectMissingFunderValidator::new()),
            Box::new(ScheduleMissingTimezoneValidator::new()),
            Box::new(TripMissingItineraryValidator::new()),
            Box::new(WorkersUnionMissingNameValidator::new()),
            Box::new(WebAPIMissingDocumentationValidator::new()),
            Box::new(WearableMissingDeviceTypeValidator::new()),
            Box::new(WebPageElementMissingNameValidator::new()),
            Box::new(WorkerMissingJobTitleValidator::new()),
            Box::new(CreativeWorkMissingLicenseValidator::new()),
            Box::new(ProductMissingSkuValidator::new()),
            Box::new(BookMissingPublisherValidator::new()),
            Box::new(MovieMissingActorValidator::new()),
            Box::new(ServiceMissingServiceTypeValidator::new()),
            Box::new(EventMissingStartDateValidator::new()),
            Box::new(EventMissingLocationValidator::new()),
            Box::new(LocalBusinessMissingGeoValidator::new()),
            // V6 Security Validators (61-80)
            Box::new(CspConnectSrcValidator::new()),
            Box::new(CspFontSrcValidator::new()),
            Box::new(HstsMaxAgeThresholdValidator::new()),
            Box::new(HstsPreloadListCheckValidator::new()),
            Box::new(CookieSecureFlagDeepValidator::new()),
            Box::new(CookieHttpOnlyFlagDeepValidator::new()),
            Box::new(CookieSameSiteDeepValidator::new()),
            Box::new(MixedContentIframeValidator::new()),
            Box::new(CorsWildcardValidator::new()),
            Box::new(CorsMissingHeaderValidator::new()),
            Box::new(ReferrerPolicyStrictValidator::new()),
            Box::new(XFrameOptionsMissingValidator::new()),
            Box::new(PermissionsPolicyPaymentValidator::new()),
            Box::new(PermissionsPolicyFullscreenValidator::new()),
            Box::new(PermissionsPolicyXrVrValidator::new()),
            Box::new(CoepRequireCorpValidator::new()),
            Box::new(CoopSameOriginValidator::new()),
            Box::new(CspObjectSrcNoneValidator::new()),
            Box::new(CspBaseUriSelfValidator::new()),
            Box::new(CspFormActionSelfValidator::new()),
            // V6 SEO Validators (81-106)
            Box::new(TitleMissingValidator::new()),
            Box::new(TitleKeywordDensityValidator::new()),
            Box::new(TitleBrandPlacementValidator::new()),
            Box::new(TitlePixelWidthValidator::new()),
            Box::new(MetaDescriptionMissingValidator::new()),
            Box::new(MetaDescriptionTooShortValidator::new()),
            Box::new(MetaDescriptionUniquenessValidator::new()),
            Box::new(CanonicalMissingValidator::new()),
            Box::new(CanonicalSelfReferenceDeepValidator::new()),
            Box::new(CanonicalChainDeepValidator::new()),
            Box::new(CanonicalDepthDeepValidator::new()),
            Box::new(HreflangMissingValidator::new()),
            Box::new(HreflangReciprocalDeepValidator::new()),
            Box::new(HreflangXDefaultMissingValidator::new()),
            Box::new(HreflangLocaleFormatDeepValidator::new()),
            Box::new(SitemapMissingValidator::new()),
            Box::new(SitemapLastmodFormatValidator::new()),
            Box::new(SitemapPriorityRangeValidator::new()),
            Box::new(RobotsTxtEmptyValidator::new()),
            Box::new(RobotsTxtDisallowDepthValidator::new()),
            Box::new(RobotsTxtWildcardDisallowValidator::new()),
            Box::new(RobotsTxtMissingUserAgentValidator::new()),
            Box::new(InternalLinksDiversityValidator::new()),
            Box::new(InternalLinksDepthDistributionValidator::new()),
            Box::new(ExternalLinksAuthorityScoreValidator::new()),
            Box::new(ExternalLinksNofollowAnalysisValidator::new()),
            // V6 Accessibility Validators (107-125)
            Box::new(HeadingH1CountValidator::new()),
            Box::new(HeadingDepthAnalysisValidator::new()),
            Box::new(FormRequiredFieldsDeepValidator::new()),
            Box::new(TableHeadersScopeValidator::new()),
            Box::new(TableCaptionMissingDeepValidator::new()),
            Box::new(LinkTextGenericDeepValidator::new()),
            Box::new(LinkTextEmptyDeepValidator::new()),
            Box::new(LinkTextDuplicateValidator::new()),
            Box::new(ImageAltMissingDeepValidator::new()),
            Box::new(ImageAltEmptyDeepValidator::new()),
            Box::new(ImageAltDecorativePatternValidator::new()),
            Box::new(FocusTabindexPositiveValidator::new()),
            Box::new(FocusTrapMissingValidator::new()),
            Box::new(HeadingSkipLevelsDeepValidator::new()),
            Box::new(HeadingEmptyValidator::new()),
            Box::new(FormFieldsetLegendValidator::new()),
            Box::new(LandmarkContentinfoValidator::new()),
            Box::new(LandmarkComplementaryValidator::new()),
            // V7 Content Validators (1-20)
            Box::new(ApartmentMissingNumberOfRoomsValidator::new()),
            Box::new(CarMissingModelValidator::new()),
            Box::new(CarMissingManufacturerValidator::new()),
            Box::new(MusicAlbumMissingNumberOfTracksValidator::new()),
            Box::new(TVSeriesMissingNumberOfEpisodesValidator::new()),
            Box::new(MovieMissingAggregateRatingValidator::new()),
            Box::new(GovernmentServiceMissingServiceAreaValidator::new()),
            Box::new(HealthPlanMissingHealthPlanIdValidator::new()),
            Box::new(InvoiceMissingPaymentStatusValidator::new()),
            Box::new(PermitMissingValidFromValidator::new()),
            Box::new(PlanMissingBenefitsValidator::new()),
            Box::new(ResearchProjectMissingFundingRecognizerValidator::new()),
            Box::new(ScheduleMissingRepeatFrequencyValidator::new()),
            Box::new(TripMissingDepartureTimeValidator::new()),
            Box::new(WorkersUnionMissingMemberCountValidator::new()),
            Box::new(WebAPIDocumentationMissingValidator::new()),
            Box::new(WearableMissingBatteryLifeValidator::new()),
            Box::new(WebPageElementMissingAccessibleNameValidator::new()),
            Box::new(WorkerMissingOccupationValidator::new()),
            Box::new(BookMissingBookFormatValidator::new()),
            // V7 Security Validators (21-35)
            Box::new(CspScriptSrcSelfValidator::new()),
            Box::new(CspStyleSrcSelfValidator::new()),
            Box::new(CspConnectSrcAnalysisValidator::new()),
            Box::new(CspFontSrcAnalysisValidator::new()),
            Box::new(CspObjectSrcNoneDeepValidator::new()),
            Box::new(CspBaseUriSelfDeepValidator::new()),
            Box::new(CspFormActionSelfDeepValidator::new()),
            Box::new(HstsPreloadReadyDeepValidator::new()),
            Box::new(HstsMaxAgeDeepValidator::new()),
            Box::new(CookieSecureDeepDeepValidator::new()),
            Box::new(CookieHttpOnlyDeepDeepValidator::new()),
            Box::new(CookieSameSiteDeepDeepValidator::new()),
            Box::new(MixedContentIframeDeepValidator::new()),
            Box::new(CorsWildcardDeepValidator::new()),
            Box::new(ReferrerPolicyStrictDeepValidator::new()),
            // V7 SEO Validators (36-50)
            Box::new(TitleMissingDeepValidator::new()),
            Box::new(TitleKeywordDensityDeepValidator::new()),
            Box::new(TitleBrandPlacementDeepValidator::new()),
            Box::new(MetaDescriptionMissingDeepValidator::new()),
            Box::new(MetaDescriptionTooShortDeepValidator::new()),
            Box::new(MetaDescriptionUniquenessDeepValidator::new()),
            Box::new(CanonicalMissingDeepValidator::new()),
            Box::new(CanonicalSelfReferenceDeepDeepValidator::new()),
            Box::new(CanonicalChainDeepDeepValidator::new()),
            Box::new(HreflangMissingDeepValidator::new()),
            Box::new(HreflangReciprocalDeepDeepValidator::new()),
            Box::new(HreflangXDefaultMissingDeepValidator::new()),
            Box::new(SitemapMissingDeepValidator::new()),
            Box::new(RobotsTxtEmptyDeepValidator::new()),
            Box::new(InternalLinksDiversityDeepValidator::new()),
            // V8 Content Validators (Dataset, HowTo, Recipe)
            Box::new(DatasetMissingDescriptionValidator::new()),
            Box::new(DatasetMissingDistributionValidator::new()),
            Box::new(HowToMissingNameValidator::new()),
            Box::new(HowToMissingStepValidator::new()),
            Box::new(RecipeMissingCookTimeValidator::new()),
            // V8 Security Validators (Permissions-Policy deep)
            Box::new(PermissionsPolicyPaymentDeepValidator::new()),
            Box::new(PermissionsPolicyFullscreenDeepValidator::new()),
            Box::new(PermissionsPolicyXrVrDeepValidator::new()),
            // V8 SEO Validators (Internal/External links deep)
            Box::new(InternalLinksDepthDeepValidator::new()),
            Box::new(ExternalLinksAuthorityScoreDeepValidator::new()),
            // V8 Content Validators (30)
            Box::new(ApartmentMissingNumberOfRoomsValidatorV2::new()),
            Box::new(CarMissingModelValidatorV2::new()),
            Box::new(MusicAlbumMissingTracksValidatorV2::new()),
            Box::new(TVSeriesMissingEpisodesValidatorV2::new()),
            Box::new(MovieMissingDirectorValidatorV2::new()),
            Box::new(BookMissingAuthorValidatorV2::new()),
            Box::new(BookMissingIsbnValidatorV2::new()),
            Box::new(BookMissingDatePublishedValidatorV2::new()),
            Box::new(ServiceMissingProviderValidatorV2::new()),
            Box::new(HealthPlanMissingProviderValidatorV2::new()),
            Box::new(InvoiceMissingAccountValidatorV2::new()),
            Box::new(PermitMissingPermitNumberValidatorV2::new()),
            Box::new(PlanMissingDescriptionValidatorV2::new()),
            Box::new(ResearchProjectMissingAboutValidatorV2::new()),
            Box::new(ScheduleMissingTimezoneValidatorV2::new()),
            Box::new(TripMissingItineraryValidatorV2::new()),
            Box::new(WorkersUnionMissingNameValidatorV2::new()),
            Box::new(WebAPIMissingDocumentationValidatorV2::new()),
            Box::new(WearableMissingDeviceTypeValidatorV2::new()),
            Box::new(WebPageElementMissingNameValidatorV2::new()),
            Box::new(WorkerMissingJobTitleValidatorV2::new()),
            Box::new(ArticleMissingHeadlineValidatorV2::new()),
            Box::new(ArticleMissingDatePublishedValidatorV2::new()),
            Box::new(ArticleMissingAuthorValidatorV2::new()),
            Box::new(OrganizationMissingNameValidatorV2::new()),
            Box::new(PersonMissingNameValidatorV2::new()),
            Box::new(JobPostingMissingTitleValidatorV2::new()),
            Box::new(CourseMissingNameValidatorV2::new()),
            Box::new(RecipeMissingNameValidatorV2::new()),
            Box::new(LocalBusinessMissingNpiValidatorV2::new()),
            // V8 Security Validators (25)
            Box::new(CspScriptSrcSelfDeepValidator::new()),
            Box::new(CspStyleSrcSelfDeepValidator::new()),
            Box::new(CspConnectSrcDeepValidator::new()),
            Box::new(CspFontSrcDeepValidator::new()),
            Box::new(CspObjectSrcNoneDeepDeepValidator::new()),
            Box::new(CspBaseUriSelfDeepDeepValidator::new()),
            Box::new(CspFormActionSelfDeepDeepValidator::new()),
            Box::new(HstsPreloadReadyDeepDeepValidator::new()),
            Box::new(HstsMaxAgeDeepDeepValidator::new()),
            Box::new(CookieSecureDeepDeepDeepValidator::new()),
            Box::new(CookieHttpOnlyDeepDeepDeepValidator::new()),
            Box::new(CookieSameSiteDeepDeepDeepValidator::new()),
            Box::new(MixedContentIframeDeepDeepValidator::new()),
            Box::new(CorsWildcardDeepDeepValidator::new()),
            Box::new(ReferrerPolicyStrictDeepDeepValidator::new()),
            Box::new(XFrameOptionsDeepDeepValidator::new()),
            Box::new(PermissionsPolicyDeepDeepValidator::new()),
            Box::new(CrossOriginIsolationDeepDeepValidator::new()),
            Box::new(AriaLandmarksDeepValidator::new()),
            Box::new(HeadingHierarchyDeepDeepValidator::new()),
            Box::new(FormLabelsDeepDeepValidator::new()),
            Box::new(TableAccessibilityDeepDeepValidator::new()),
            Box::new(LinkTextQualityDeepValidator::new()),
            Box::new(ImageAltTextDeepDeepValidator::new()),
            Box::new(FocusManagementDeepDeepValidator::new()),
            // V8 SEO Validators (25)
            Box::new(TitleMissingDeepDeepValidator::new()),
            Box::new(TitleKeywordDensityDeepDeepValidator::new()),
            Box::new(TitleBrandPlacementDeepDeepValidator::new()),
            Box::new(MetaDescriptionMissingDeepDeepValidator::new()),
            Box::new(MetaDescriptionTooShortDeepDeepValidator::new()),
            Box::new(MetaDescriptionUniquenessDeepDeepValidator::new()),
            Box::new(CanonicalMissingDeepDeepValidator::new()),
            Box::new(CanonicalSelfReferenceDeepDeepDeepValidator::new()),
            Box::new(CanonicalChainDeepDeepDeepValidator::new()),
            Box::new(HreflangMissingDeepDeepValidator::new()),
            Box::new(HreflangReciprocalDeepDeepDeepValidator::new()),
            Box::new(HreflangXDefaultMissingDeepDeepValidator::new()),
            Box::new(SitemapMissingDeepDeepValidator::new()),
            Box::new(RobotsTxtEmptyDeepDeepValidator::new()),
            Box::new(InternalLinksDiversityDeepDeepValidator::new()),
            Box::new(ExternalLinksAuthorityScoreDeepDeepValidator::new()),
            Box::new(InternalLinksDepthDeepDeepValidator::new()),
            Box::new(MetaDescriptionLengthDeepValidator::new()),
            Box::new(TitleLengthDeepValidator::new()),
            Box::new(SitemapCoverageDeepDeepValidator::new()),
            Box::new(RobotsTxtAnalysisDeepDeepValidator::new()),
            Box::new(InternalLinkQualityDeepValidator::new()),
            Box::new(ExternalLinkAuthorityDeepDeepValidator::new()),
            Box::new(HreflangLocaleFormatDeepDeepValidator::new()),
            Box::new(CanonicalDepthDeepDeepValidator::new()),
            // V8 Accessibility Validators (20)
            Box::new(LandmarkMainDeepValidator::new()),
            Box::new(LandmarkNavDeepValidator::new()),
            Box::new(LandmarkBannerDeepValidator::new()),
            Box::new(HeadingSkipLevelsDeepDeepValidator::new()),
            Box::new(FormLabelsDeepDeepDeepValidator::new()),
            Box::new(TableAccessibilityDeepDeepDeepValidator::new()),
            Box::new(LinkTextQualityDeepDeepValidator::new()),
            Box::new(ImageAltTextDeepDeepDeepValidator::new()),
            Box::new(FocusManagementDeepDeepDeepValidator::new()),
            Box::new(LanguageAttributesDeepDeepValidator::new()),
            Box::new(ColorContrastTextDeepValidator::new()),
            Box::new(ColorContrastLinkDeepValidator::new()),
            Box::new(AnchorTextGenericDeepValidator::new()),
            Box::new(TableCaptionPresenceDeepValidator::new()),
            Box::new(TableHeaderScopeDeepValidator::new()),
            Box::new(FormLabelAssociationDeepValidator::new()),
            Box::new(AriaRequiredAttributesDeepValidator::new()),
            Box::new(HeadingHierarchyDeepDeepDeepValidator::new()),
            Box::new(LandmarkContentinfoDeepValidator::new()),
            Box::new(LandmarkComplementaryDeepValidator::new()),
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
            analyzers.push(Box::new(crate::wasm_analyzers::WasmRuntimeAnalyzer::new()));
            analyzers.push(Box::new(
                crate::wasm_analyzers::WasmPerformanceAnalyzer::new(),
            ));
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
mod tests;
