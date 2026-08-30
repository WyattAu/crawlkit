use std::time::Duration;

use crate::parser::ParsedPage;
use crate::{CrawlConfig, RedirectHop};

/// Content quality analyzers for readability, entity extraction, and structured data.
pub mod content_analyzers;
/// HTTP-level analyzers for status codes, redirects, robots.txt, and SSL certificates.
pub mod http_analyzers;
/// Media and e-commerce analyzers for images, products, and shopping signals.
pub mod media_analyzers;
/// Post-crawl analyzers that inspect full crawl data for cross-page issues.
pub mod post_crawl_analyzers;
/// Security and accessibility analyzers for headers, mobile-friendliness, and WCAG compliance.
pub mod security_analyzers;
/// SEO analyzers for meta tags, headings, links, canonicals, and internationalization.
pub mod seo_analyzers;
/// Schema.org validators, one per type.
pub mod schema;
/// Social media analyzers for Open Graph, Twitter Cards, and social sharing metadata.
pub mod social_analyzers;

pub use content_analyzers::{
    ArticleAuthorValidator, ArticleDatePublishedValidator, ArticleHeadlineValidator,
    BreadcrumbActivePageValidator, BreadcrumbListDepthAnalyzer, CanonicalChainValidator,
    ContentFreshnessScorer, ContentLanguageValidator, ContentStructureAnalyzer, ContentThinAnalyzer,
    ContentQualityAnalyzer, ContentTopicCoverageAnalyzer, CourseNameValidator,
    DuplicateContentDetector, EnhancedReadabilityAnalyzer,
    EntityAnalyzer, EntityLinkingAnalyzer, ExternalLinkQualityAnalyzer, JsonLdContextValidator,
    JsonLdTypeValidator, JsonLdValidator, InternalLinkDepthAnalyzerV2, JobPostingTitleValidator,
    KeywordDensityAnalyzer,
    MetaDescriptionLengthAnalyzer, MetaRobotsValidator,
    MicrodataValidator, MobileFriendlinessScoreAnalyzer, OpenGraphVideoUrlValidator,
    OrganizationNameValidator, PageSpeedScoreAnalyzer, PersonNameValidator, RdfaValidator,
    RecipeNameValidator, SchemaIdReferenceValidator,
    SchemaNestingDepthValidator, StructuredDataValidator, TableOfContentsAnalyzer,
    TitleLengthAnalyzer, TwitterPlayerStreamValidator,
};
pub use schema::*;
pub use http_analyzers::{
    CacheHeaderAnalyzer, CompressionAnalyzer, HttpVersionAnalyzer, HttpStatusAnalyzer,
    RedirectChainAnalyzer, ResponseSizeAnalyzer, RobotsRule, RobotsTxtAnalyzer,
    ServerHeaderAnalyzer, SslCertificateValidator, TtfbAnalyzer,
};
pub use media_analyzers::{
    AggregateRatingValidator, AsyncScriptAnalyzer, BlockingStyleAnalyzer,
    ConnectionAnalyzer, CriticalResourceAnalyzer,
    EcommerceSignalsAnalyzer, FontDisplayAnalyzer, FontDisplayAnalyzerV2, FormAnalyzer, ImageAnalyzer,
    ImageAspectRatioValidator, ImageDimensionMissingAnalyzer,
    ImageFileSizeValidator, ImageInfo, ImageLazyLoadAnalyzer, ImageLazyLoadAnalyzerV2,
    PreloadHintAnalyzer, PricingSchemaValidator, ProductVariantAnalyzer,
    ResourceCountAnalyzer, ResourceSizeAnalyzer, ScriptAnalyzer, ScriptLoadAnalyzerV2,
    StylesheetAnalyzer, ThirdPartyResourceAnalyzer,
};
pub use security_analyzers::{
    AccessibilityAnalyzer, AriaLabelAnalyzer, AriaRolesAnalyzer, AriaRolesAnalyzerV2,
    AriaRequiredAttributesAnalyzer,
    CertificateTransparencyAnalyzer,
    ColorContrastAnalyzer, ColorContrastLinkAnalyzer, ColorContrastTextAnalyzer,
    ContentTypeSniffingAnalyzer,
    ContentSecurityPolicyAnalyzer, ContentSecurityPolicyAnalyzerV2, CorsMisconfigurationAnalyzer, CookieAnalyzer,
    CookieHttpOnlyFlagValidator, CookieSecureFlagValidator,
    CrossOriginIsolationAnalyzer, CrossOriginIsolationAnalyzerV2,
    CrossOriginOpenerPolicyAnalyzerV2,
    CrossOriginResourcePolicyAnalyzer, CspDirectiveValidator, DnsRebindingAnalyzer, ExpectCTAnalyzer,
    FeaturePolicyAnalyzer,
    FocusManagementAnalyzer, FocusOrderAnalyzer, FocusOrderPositiveTabindexAnalyzer,
    FontSizeAnalyzer, FormAccessibilityAnalyzerV2, FormLabelAnalyzer, FormLabelAssociationAnalyzer,
    HeadingHierarchyAnalyzerV2, HeadingLevelSkipAnalyzer, HeadingOrderAnalyzer,
    AnchorTextGenericAnalyzer,
    HstsPreloadAnalyzer, HstsPreloadListValidator,
    ImageAccessibilityAnalyzer, ImageAccessibilityAnalyzerV2, ImageAltTextAnalyzer,
    LanguageAttributeAnalyzerV2,
    LandmarkBannerAnalyzer, LandmarkMainAnalyzer, LandmarkNavAnalyzer, LandmarkRegionsAnalyzer, LinkAccessibilityAnalyzer, LinkAccessibilityAnalyzerV2, LinkTextAnalyzer,
    MixedContentAnalyzer, MixedContentFormValidator, MixedContentImageValidator, MixedContentScriptValidator,
    MobileFriendlinessChecker, PermissionPolicyAnalyzer, PermissionsPolicyAnalyzerV3,
    ReferrerPolicyAnalyzer, ReferrerPolicyAnalyzerV2, SecurityHeaderAnalyzer, SkipLinkAnalyzer, SriAnalyzer,
    StrictTransportSecurityAnalyzer, StrictTransportSecurityAnalyzerV3,
    SubresourceIntegrityAnalyzer,
    TableAccessibilityAnalyzer, TableAccessibilityAnalyzerV2, TableCaptionAnalyzer, TableCaptionPresenceAnalyzer, TableHeaderScopeAnalyzer,
    TabindexAnalyzer, TabindexAnalyzerV2,
    XContentTypeOptionsAnalyzer, XContentTypeOptionsAnalyzerV2, XSSProtectionAnalyzer, XFrameOptionsAnalyzer, XFrameOptionsAnalyzerV2,
    XPermittedCrossDomainPoliciesAnalyzer,
};
pub use seo_analyzers::{
    AnchorTextDiversityAnalyzer, CanonicalDepthAnalyzer,
    CanonicalSelfReferenceAnalyzerV2, CanonicalUrlAnalyzerV2, CanonicalUrlValidator, CharsetValidator,
    ContentQualityAnalyzerV2, ExternalLinkAnalyzerV2, ExternalLinkAuthorityAnalyzer, ExternalNofollowUnderuseValidator,
    HeadingHierarchyAnalyzer, HreflangConsistencyAnalyzer,
    HreflangReciprocalAnalyzerV2, HreflangValidatorV3,
    HreflangSelfReferenceValidator,
    HreflangValidator,
    InternalLinkAnchorAnalyzer, InternalLinkAnchorTextDiversityAnalyzer, InternalLinkAnalyzerV2, InternalLinkTopicalAnalyzer,
    InternalNofollowOveruseValidator,
    InternationalSeoAnalyzer, KeywordAnalyzer, KeywordAnalyzerV2,
    LanguageAttributeAnalyzer, LinkAnalyzer, LinkAnalyzerV2, LinkInfo,
    MetaDescriptionAnalyzerV3, MetaDescriptionLengthAnalyzerV2, MetaDescriptionPixelWidthAnalyzer, MetaTagAnalyzer,
    MixedProtocolRedirectValidator, MobileViewportAnalyzer,
    OpenSearchDescriptionValidator, OpenSearchValidator,
    PaginationAnalyzer, PaginationDepthValidator, PaginationLinkAnalyzer,
    RedirectLoopDetector, RobotsMetaAnalyzer, RobotsTxtAnalyzerV2, RobotsTxtCoverageAnalyzerV2, RobotsTxtDirectivesAnalyzer,
    RobotsTxtSizeValidator, SitemapAnalyzer, SitemapAnalyzerV2, SitemapCoverageAnalyzerV2, SitemapEntry, SitemapUrlAnalyzer,
    SitemapXmlSizeValidator, TitleAnalyzerV3, TitleKeywordAnalyzer, TitlePixelWidthAnalyzer,
    WikipediaLinkAnalyzer, WordCountAnalyzer, WordCountAnalyzerV2,
};
pub use social_analyzers::{
    OpenGraphAudioAnalyzer, OpenGraphImageValidator, OpenGraphSiteNameValidator,
    OpenGraphUrlValidator, OpenGraphVideoAnalyzer,
    SocialMediaAnalyzer, SocialPreviewOptimizer, TwitterCardTypeAnalyzer,
    TwitterPlayerValidator, TwitterSiteAnalyzer,
};
pub use post_crawl_analyzers::{
    build_post_crawl_registry, CannibalizationDetector, CrawlData,
    PostCrawlAnalyzer, PostCrawlAnalyzerRegistry, SitemapCoverageAnalyzer,
};
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
    /// in parallel using rayon. The default registry includes 134 analyzers
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
            analyzers.push(Box::new(crate::wasm_analyzers::WasmPerformanceAnalyzer::new()));
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
