#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::manual_range_contains,
    clippy::redundant_closure,
    clippy::collapsible_if,
    clippy::unnecessary_map_or,
    clippy::default_constructed_unit_structs,
    clippy::needless_return,
    clippy::needless_range_loop,
    clippy::useless_format,
    clippy::if_same_then_else,
    clippy::derivable_impls,
    clippy::manual_pattern_char_comparison,
    clippy::manual_contains,
    clippy::collapsible_match,
    clippy::redundant_clone,
    clippy::useless_conversion
)]
use crate::analyzers::{robots_txt_star_blanket_disallows_all, AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

/// Counts real `<link rel="canonical">` elements in an HTML document.
///
/// Counting the raw string `rel="canonical"` also matched comments, code
/// samples, and documentation text — flagging single-canonical pages as
/// having duplicates.
fn count_canonical_links(body: &str) -> usize {
    let sel = scraper::Selector::parse("link").expect("static selector");
    scraper::Html::parse_document(body)
        .select(&sel)
        .filter(|el| {
            el.value().attr("rel").map_or(false, |r| {
                r.split_whitespace()
                    .any(|t| t.eq_ignore_ascii_case("canonical"))
            })
        })
        .count()
}

pub struct TitleAnalysisDeepAnalyzerV2;
impl Default for TitleAnalysisDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleAnalysisDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleAnalysisDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "title-analysis-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let title = match &ctx.page.meta.title {
            Some(t) if !t.trim().is_empty() => t.trim(),
            _ => return findings,
        };
        if title.len() < 20 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLEDEEP-V2001".to_string(),
                title: "Title critically short".to_string(),
                description: format!("{} chars, below 30-60 target.", title.len()),
                url: url.clone(),
                recommendation: "Expand to 30-60 characters.".to_string(),
            });
        }
        if title.len() > 70 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLEDEEP-V2002".to_string(),
                title: "Title excessively long".to_string(),
                description: format!("{} chars, truncates at ~60.", title.len()),
                url: url.clone(),
                recommendation: "Shorten to under 60 characters.".to_string(),
            });
        }
        // Only flag casing issues on multi-word titles with real letters:
        // acronym titles ("NASA") and numeric titles ("2026") are all-caps
        // or cased-agnostic by nature, not SEO mistakes.
        let has_cased_letters = title.chars().any(|c| c.is_alphabetic());
        let multi_word = title.split_whitespace().count() >= 2;
        if has_cased_letters
            && multi_word
            && (title.to_lowercase() == title || title.to_uppercase() == title)
        {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLEDEEP-V2003".to_string(),
                title: "Title all lowercase or uppercase".to_string(),
                description: "Affects readability and CTR.".to_string(),
                url: url.clone(),
                recommendation: "Use Title Case or sentence case.".to_string(),
            });
        }
        findings
    }
}

pub struct MetaDescriptionDeepAnalyzerV2;
impl Default for MetaDescriptionDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "meta-description-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let desc = match &ctx.page.meta.description {
            Some(d) if !d.trim().is_empty() => d.trim(),
            _ => return findings,
        };
        if desc.len() < 70 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "METADEEP-V2001".to_string(),
                title: "Description too short".to_string(),
                description: format!("{} chars, aim for 120-155.", desc.len()),
                url: url.clone(),
                recommendation: "Expand to 120-155 characters.".to_string(),
            });
        }
        if desc.len() > 160 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "METADEEP-V2002".to_string(),
                title: "Description too long".to_string(),
                description: format!("{} chars, truncates at ~155.", desc.len()),
                url: url.clone(),
                recommendation: "Shorten to under 155 characters.".to_string(),
            });
        }
        findings
    }
}

pub struct CanonicalValidationDeepAnalyzerV2;
impl Default for CanonicalValidationDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalValidationDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalValidationDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "canonical-validation-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let canonical = match &ctx.page.meta.canonical {
            Some(c) => c,
            None => return findings,
        };
        if canonical.as_str().contains('#') {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "CANDEEP-V2005".to_string(),
                title: "Canonical contains fragment".to_string(),
                description: "Fragments are ignored by search engines.".to_string(),
                url: url.clone(),
                recommendation: "Remove fragment from canonical URL.".to_string(),
            });
        }
        if canonical.as_str().contains('?') {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "CANDEEP-V2006".to_string(),
                title: "Canonical has query params".to_string(),
                description: "Query params may cause indexing issues.".to_string(),
                url: url.clone(),
                recommendation: "Canonical URLs should be parameter-free.".to_string(),
            });
        }
        findings
    }
}

pub struct SitemapCoverageDeepAnalyzerV2;
impl Default for SitemapCoverageDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapCoverageDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapCoverageDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "sitemap-coverage-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let lower = robots.to_lowercase();
            if !lower.contains("sitemap:") {
                findings.push(Finding {
                    // Declaring the sitemap in robots.txt is optional —
                    // submitting it via Search Console / ping endpoints is an
                    // equally valid route, so this is informational.
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "SITEMAPDEEP-V2001".to_string(),
                    title: "No sitemap in robots.txt".to_string(),
                    description: "Search engines may miss pages.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Sitemap: directive to robots.txt.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct RobotsTxtAnalysisDeepAnalyzerV2;
impl Default for RobotsTxtAnalysisDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtAnalysisDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtAnalysisDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "robots-txt-analysis-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let robots = match ctx.robots_txt {
            Some(r) => r,
            None => return findings,
        };
        if robots_txt_star_blanket_disallows_all(robots) {
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Seo,
                code: "ROBOTSDEEP-V2001".to_string(),
                title: "robots.txt blocks all".to_string(),
                description: "Blanket Disallow: / blocks all crawlers.".to_string(),
                url: url.clone(),
                recommendation: "Remove blanket disallow.".to_string(),
            });
        }
        findings
    }
}

pub struct InternalLinkQualityAnalyzerV2;
impl Default for InternalLinkQualityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinkQualityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalLinkQualityAnalyzerV2 {
    fn name(&self) -> &str {
        "internal-link-quality-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<_> = ctx.page.links.iter().filter(|l| !l.is_external).collect();
        if internal.is_empty() {
            return findings;
        }
        let self_links = internal.iter().filter(|l| l.href == *url).count();
        if self_links > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "INTLINKQ-V2001".to_string(),
                title: "Self-referencing links".to_string(),
                description: format!("{self_links} link(s) point to same page."),
                url: url.clone(),
                recommendation: "Remove self-referencing links.".to_string(),
            });
        }
        let nofollow = internal
            .iter()
            .filter(|l| l.rel.iter().any(|r| r == "nofollow"))
            .count();
        if nofollow == internal.len() && internal.len() > 1 {
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Seo,
                code: "INTLINKQ-V2002".to_string(),
                title: "All internal links nofollowed".to_string(),
                description: "Blocks all internal PageRank flow.".to_string(),
                url: url.clone(),
                recommendation: "Remove nofollow from internal links.".to_string(),
            });
        }
        findings
    }
}

pub struct ExternalLinkAuthorityDeepAnalyzerV2;
impl Default for ExternalLinkAuthorityDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ExternalLinkAuthorityDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ExternalLinkAuthorityDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "external-link-authority-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let external: Vec<_> = ctx.page.links.iter().filter(|l| l.is_external).collect();
        if external.is_empty() {
            return findings;
        }
        let suspicious_tlds = [".ru", ".cn", ".tk", ".ml", ".xyz", ".top"];
        let mut susp = 0;
        for link in &external {
            let h = link.href.to_lowercase();
            if suspicious_tlds.iter().any(|t| h.contains(t)) {
                susp += 1;
            }
        }
        if susp > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "EXTLINKAUTH-V2001".to_string(),
                title: "Links to suspicious TLDs".to_string(),
                description: format!("{susp} external link(s) to suspicious domains."),
                url: url.clone(),
                recommendation: "Review these links.".to_string(),
            });
        }
        findings
    }
}

pub struct TitleLengthQualityAnalyzerV2;
impl Default for TitleLengthQualityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleLengthQualityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleLengthQualityAnalyzerV2 {
    fn name(&self) -> &str {
        "title-length-quality-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let title = match &ctx.page.meta.title {
            Some(t) if !t.trim().is_empty() => t.trim(),
            _ => return findings,
        };
        let len = title.len();
        if len < 20 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLEQLT-V2001".to_string(),
                title: "Title too short".to_string(),
                description: format!("{len} chars, wastes SERP space."),
                url: url.clone(),
                recommendation: "Expand to 30-60 characters.".to_string(),
            });
        } else if len > 60 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLEQLT-V2002".to_string(),
                title: "Title may truncate".to_string(),
                description: format!("{len} chars, Google shows ~60."),
                url: url.clone(),
                recommendation: "Keep important keywords within 60 chars.".to_string(),
            });
        }
        findings
    }
}

pub struct MetaDescriptionQualityAnalyzerV2;
impl Default for MetaDescriptionQualityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionQualityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionQualityAnalyzerV2 {
    fn name(&self) -> &str {
        "meta-description-quality-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let desc = match &ctx.page.meta.description {
            Some(d) if !d.trim().is_empty() => d.trim(),
            _ => return findings,
        };
        let len = desc.len();
        if len < 50 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "METAQLT-V2001".to_string(),
                title: "Description too short".to_string(),
                description: format!("{len} chars, aim for 120-155."),
                url: url.clone(),
                recommendation: "Expand to 120-155 characters.".to_string(),
            });
        } else if len > 155 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "METAQLT-V2002".to_string(),
                title: "Description may truncate".to_string(),
                description: format!("{len} chars, Google shows ~155."),
                url: url.clone(),
                recommendation: "Shorten to under 155 characters.".to_string(),
            });
        }
        findings
    }
}

pub struct InternalLinkAnchorAnalyzerV3;
impl Default for InternalLinkAnchorAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinkAnchorAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalLinkAnchorAnalyzerV3 {
    fn name(&self) -> &str {
        "internal-link-anchor-v3"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<_> = ctx.page.links.iter().filter(|l| !l.is_external).collect();
        if internal.is_empty() {
            return findings;
        }
        let generic = [
            "click here",
            "here",
            "read more",
            "more",
            "link",
            "learn more",
        ];
        let generic_count = internal
            .iter()
            .filter(|l| generic.contains(&l.text.trim().to_lowercase().as_str()))
            .count();
        if generic_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "ANCH-V3001".to_string(),
                title: "Generic internal anchor text".to_string(),
                description: format!("{generic_count} link(s) use generic text."),
                url: url.clone(),
                recommendation: "Use keyword-rich anchor text.".to_string(),
            });
        }
        findings
    }
}

pub struct WikipediaLinkAnalyzerV3;
impl Default for WikipediaLinkAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}
impl WikipediaLinkAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WikipediaLinkAnalyzerV3 {
    fn name(&self) -> &str {
        "wikipedia-link-v3"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let wiki_count = ctx
            .page
            .links
            .iter()
            .filter(|l| l.href.contains("wikipedia.org"))
            .count();
        if wiki_count > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "WIKI-V3001".to_string(),
                title: "Wikipedia links found".to_string(),
                description: format!("{wiki_count} Wikipedia link(s). These are nofollowed."),
                url: url.clone(),
                recommendation: "Wikipedia links won't pass link equity.".to_string(),
            });
        }
        findings
    }
}

pub struct PaginationDepthAnalyzerV2;
impl Default for PaginationDepthAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl PaginationDepthAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PaginationDepthAnalyzerV2 {
    fn name(&self) -> &str {
        "pagination-depth-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        // Pagination is a property of the URL, not of the body text. Matching
        // the raw body fired on any page whose docs, comments, or scripts
        // mention "page=", "p=", or "start=".
        let lower_url = url.to_lowercase();
        let has_pagination =
            lower_url.contains("page=") || lower_url.contains("p=") || lower_url.contains("start=");
        if has_pagination {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "PAGDEP-V2001".to_string(),
                title: "Paginated URL detected".to_string(),
                description: "URL appears to be paginated.".to_string(),
                url: url.clone(),
                recommendation: "Consider rel=next/prev for paginated content.".to_string(),
            });
        }
        findings
    }
}

pub struct MixedProtocolRedirectValidatorV2;
impl Default for MixedProtocolRedirectValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedProtocolRedirectValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MixedProtocolRedirectValidatorV2 {
    fn name(&self) -> &str {
        "mixed-protocol-redirect-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.redirect_chain.is_empty() {
            return findings;
        }
        let mut https_to_http = false;
        for hop in ctx.redirect_chain {
            let from_https = hop.from.as_str().starts_with("https://");
            let to_https = hop.to.as_str().starts_with("https://");
            if from_https && !to_https {
                https_to_http = true;
            }
        }
        if https_to_http {
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Seo,
                code: "MIXPROT-V2002".to_string(),
                title: "HTTPS redirecting to HTTP".to_string(),
                description: "Security downgrade in redirect chain.".to_string(),
                url: url.clone(),
                recommendation: "Fix redirect chain to maintain HTTPS.".to_string(),
            });
        }
        findings
    }
}

pub struct InternalNofollowOveruseValidatorV2;
impl Default for InternalNofollowOveruseValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalNofollowOveruseValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalNofollowOveruseValidatorV2 {
    fn name(&self) -> &str {
        "internal-nofollow-overuse-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<_> = ctx.page.links.iter().filter(|l| !l.is_external).collect();
        if internal.is_empty() {
            return findings;
        }
        let nofollow = internal
            .iter()
            .filter(|l| l.rel.iter().any(|r| r == "nofollow"))
            .count();
        let ratio = nofollow as f64 / internal.len() as f64;
        if ratio > 0.8 && internal.len() > 3 {
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Seo,
                code: "NOFOLLOW-V2001".to_string(),
                title: "Extreme nofollow overuse".to_string(),
                description: format!(
                    "{nofollow}/{} ({:.0}%) nofollowed.",
                    internal.len(),
                    ratio * 100.0
                ),
                url: url.clone(),
                recommendation: "Remove nofollow from most internal links.".to_string(),
            });
        } else if ratio > 0.5 && internal.len() > 5 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "NOFOLLOW-V2002".to_string(),
                title: "High nofollow ratio".to_string(),
                description: format!(
                    "{nofollow}/{} ({:.0}%) nofollowed.",
                    internal.len(),
                    ratio * 100.0
                ),
                url: url.clone(),
                recommendation: "Review nofollow usage.".to_string(),
            });
        }
        findings
    }
}

pub struct SitemapXmlSizeValidatorV2;
impl Default for SitemapXmlSizeValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapXmlSizeValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapXmlSizeValidatorV2 {
    fn name(&self) -> &str {
        "sitemap-xml-size-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") || body.contains("<sitemapindex") {
                let size_kb = body.len() / 1024;
                if size_kb > 50000 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "SITEMAPSIZE-V2001".to_string(),
                        title: "Sitemap very large".to_string(),
                        description: format!("{size_kb} KB. Too large to download efficiently."),
                        url: url.clone(),
                        recommendation: "Split into multiple sitemaps.".to_string(),
                    });
                }
                let url_count = body.matches("<url>").count();
                if url_count > 50000 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "SITEMAPSIZE-V2003".to_string(),
                        title: "Sitemap exceeds URL limit".to_string(),
                        description: format!("{url_count} URLs, max is 50000."),
                        url: url.clone(),
                        recommendation: "Split into multiple sitemaps.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct RobotsTxtSizeValidatorV2;
impl Default for RobotsTxtSizeValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtSizeValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtSizeValidatorV2 {
    fn name(&self) -> &str {
        "robots-txt-size-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let robots = match ctx.robots_txt {
            Some(r) => r,
            None => return findings,
        };
        let size_kb = robots.len() / 1024;
        if size_kb > 500 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "ROBOTSSIZE-V2001".to_string(),
                title: "robots.txt very large".to_string(),
                description: format!("{size_kb} KB. Slows down crawlers."),
                url: url.clone(),
                recommendation: "Simplify rules.".to_string(),
            });
        }
        findings
    }
}

pub struct HreflangSelfReferenceValidatorV2;
impl Default for HreflangSelfReferenceValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangSelfReferenceValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangSelfReferenceValidatorV2 {
    fn name(&self) -> &str {
        "hreflang-self-reference-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() {
            return findings;
        }
        let has_self = tags.iter().any(|t| t.url.as_str() == *url);
        if !has_self {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "HREFSELF-V2001".to_string(),
                title: "Missing hreflang self-reference".to_string(),
                description: "No self-referencing hreflang tag.".to_string(),
                url: url.clone(),
                recommendation: "Add self-referencing hreflang tag.".to_string(),
            });
        }
        let mut dup = std::collections::HashMap::new();
        for t in tags {
            *dup.entry(t.lang.as_str()).or_insert(0) += 1;
        }
        for (lang, count) in &dup {
            if *count > 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "HREFSELF-V2002".to_string(),
                    title: format!("Duplicate hreflang '{lang}'"),
                    description: format!("'{lang}' declared {count} times."),
                    url: url.clone(),
                    recommendation: "Remove duplicate hreflang entries.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct OpenSearchDescriptionValidatorV2;
impl Default for OpenSearchDescriptionValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl OpenSearchDescriptionValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for OpenSearchDescriptionValidatorV2 {
    fn name(&self) -> &str {
        "opensearch-description-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            // Look for a real OpenSearch <link> element (rel="search" or the
            // OpenSearch MIME type). Raw-text matching also matched comments
            // and prose while missing single-quoted attributes.
            let sel = scraper::Selector::parse("link").expect("static selector");
            let has_opensearch = scraper::Html::parse_document(body).select(&sel).any(|el| {
                let rel_is_search = el.value().attr("rel").map_or(false, |r| {
                    r.split_whitespace()
                        .any(|t| t.eq_ignore_ascii_case("search"))
                });
                let type_is_opensearch = el.value().attr("type").map_or(false, |t| {
                    t.to_lowercase()
                        .contains("application/opensearchdescription+xml")
                });
                rel_is_search || type_is_opensearch
            });
            if !has_opensearch {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "OPDESC-V2001".to_string(),
                    title: "No OpenSearch description".to_string(),
                    description: "No OpenSearch link tag found.".to_string(),
                    url: url.clone(),
                    recommendation: "Add OpenSearch description link tag.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CanonicalDepthAnalyzerV2;
impl Default for CanonicalDepthAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalDepthAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalDepthAnalyzerV2 {
    fn name(&self) -> &str {
        "canonical-depth-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            if canonical.as_str().ends_with('/')
                && canonical.as_str() != "/"
                && !canonical.as_str().contains("index")
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "CANDEP-V2003".to_string(),
                    title: "Canonical has trailing slash".to_string(),
                    description: "Trailing slashes should match preferred format.".to_string(),
                    url: url.clone(),
                    recommendation: "Ensure canonical URL format is consistent.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MetaDescriptionLengthAnalyzerV3;
impl Default for MetaDescriptionLengthAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionLengthAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionLengthAnalyzerV3 {
    fn name(&self) -> &str {
        "meta-description-length-v3"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.description {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "META-V3001".to_string(),
                    title: "Missing meta description".to_string(),
                    description: "No meta description found.".to_string(),
                    url: url.clone(),
                    recommendation: "Write a 120-155 character meta description.".to_string(),
                });
            }
            Some(d) if d.trim().len() < 50 => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "META-V3002".to_string(),
                    title: "Description very short".to_string(),
                    description: format!("{} chars.", d.trim().len()),
                    url: url.clone(),
                    recommendation: "Expand to 120-155 characters.".to_string(),
                });
            }
            Some(d) if d.len() > 155 => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "META-V3003".to_string(),
                    title: "Description may truncate".to_string(),
                    description: format!("{} chars.", d.len()),
                    url: url.clone(),
                    recommendation: "Shorten to under 155 characters.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

pub struct TitleAnalyzerV4;
impl Default for TitleAnalyzerV4 {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleAnalyzerV4 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleAnalyzerV4 {
    fn name(&self) -> &str {
        "title-v4"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.title {
            None => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Seo,
                    code: "TITLE-V4001".to_string(),
                    title: "Missing title tag".to_string(),
                    description: "No <title> tag found.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a unique 30-60 character <title>.".to_string(),
                });
            }
            Some(t) => {
                let len = t.len();
                if len < 20 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "TITLE-V4002".to_string(),
                        title: "Title too short".to_string(),
                        description: format!("{len} chars."),
                        url: url.clone(),
                        recommendation: "Expand to 30-60 characters.".to_string(),
                    });
                }
                if len > 65 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "TITLE-V4003".to_string(),
                        title: "Title may truncate".to_string(),
                        description: format!("{len} chars."),
                        url: url.clone(),
                        recommendation: "Shorten to 60 characters.".to_string(),
                    });
                }
                if t.to_uppercase() == *t {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "TITLE-V4004".to_string(),
                        title: "ALL CAPS title".to_string(),
                        description: "Looks spammy.".to_string(),
                        url: url.clone(),
                        recommendation: "Use Title Case.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct CanonicalUrlAnalyzerV3;
impl Default for CanonicalUrlAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalUrlAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalUrlAnalyzerV3 {
    fn name(&self) -> &str {
        "canonical-url-v3"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.canonical {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "CAN-V3001".to_string(),
                    title: "Missing canonical URL".to_string(),
                    description: "No canonical tag found.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a canonical URL tag.".to_string(),
                });
            }
            Some(c) => {
                if c.as_str().is_empty() {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Seo,
                        code: "CAN-V3002".to_string(),
                        title: "Empty canonical URL".to_string(),
                        description: "Canonical has empty href.".to_string(),
                        url: url.clone(),
                        recommendation: "Set canonical URL or remove tag.".to_string(),
                    });
                } else if c.as_str().contains('#') {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "CAN-V3005".to_string(),
                        title: "Canonical has fragment".to_string(),
                        description: "Fragments are ignored.".to_string(),
                        url: url.clone(),
                        recommendation: "Remove fragment from canonical.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct HreflangValidatorV4;
impl Default for HreflangValidatorV4 {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangValidatorV4 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangValidatorV4 {
    fn name(&self) -> &str {
        "hreflang-v4"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() {
            return findings;
        }
        let has_xd = tags.iter().any(|t| t.lang == "x-default");
        if !has_xd {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "HREF-V4001".to_string(),
                title: "Missing x-default".to_string(),
                description: "No x-default hreflang tag.".to_string(),
                url: url.clone(),
                recommendation: "Add x-default hreflang tag.".to_string(),
            });
        }
        let mut seen = std::collections::HashSet::new();
        let mut dup = 0;
        for t in tags {
            if !seen.insert((&t.lang, t.url.as_str())) {
                dup += 1;
            }
        }
        if dup > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "HREF-V4003".to_string(),
                title: "Duplicate hreflang entries".to_string(),
                description: format!("{dup} duplicate(s)."),
                url: url.clone(),
                recommendation: "Remove duplicate hreflang declarations.".to_string(),
            });
        }
        findings
    }
}

pub struct SitemapAnalyzerV3;
impl Default for SitemapAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapAnalyzerV3 {
    fn name(&self) -> &str {
        "sitemap-v3"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.robots_txt.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "SITEMAP-V3001".to_string(),
                title: "No robots.txt".to_string(),
                description: "No robots.txt available.".to_string(),
                url: url.clone(),
                recommendation: "Create a robots.txt file.".to_string(),
            });
        }
        findings
    }
}

pub struct RobotsTxtAnalyzerV3;
impl Default for RobotsTxtAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtAnalyzerV3 {
    fn name(&self) -> &str {
        "robots-txt-v3"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let robots = match ctx.robots_txt {
            Some(r) => r,
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "ROBOTS-V3001".to_string(),
                    title: "No robots.txt".to_string(),
                    description: "No robots.txt available.".to_string(),
                    url: url.clone(),
                    recommendation: "Create a robots.txt.".to_string(),
                });
                return findings;
            }
        };
        if robots_txt_star_blanket_disallows_all(robots) {
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Seo,
                code: "ROBOTS-V3002".to_string(),
                title: "robots.txt blocks all".to_string(),
                description: "Blanket Disallow: /.".to_string(),
                url: url.clone(),
                recommendation: "Remove blanket disallow.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// Content V5 Analyzers (1-30)
// =========================================================================

pub struct TitleKeywordPresenceValidator;
impl Default for TitleKeywordPresenceValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleKeywordPresenceValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleKeywordPresenceValidator {
    fn name(&self) -> &str {
        "title-keyword-presence-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let title = match &ctx.page.meta.title {
            Some(t) if !t.trim().is_empty() => t.trim().to_lowercase(),
            _ => return findings,
        };
        if let Some(desc) = &ctx.page.meta.description {
            let desc_words: Vec<&str> = desc
                .split_whitespace()
                .filter(|w| w.len() > 3)
                .take(5)
                .collect();
            let missing: Vec<&str> = desc_words
                .iter()
                .filter(|w| !title.contains(*w))
                .copied()
                .collect();
            if missing.len() > 2 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "TITLEKWP-V5001".to_string(),
                    title: "Title missing description keywords".to_string(),
                    description: format!(
                        "Title doesn't contain {} description keywords.",
                        missing.len()
                    ),
                    url: url.clone(),
                    recommendation: "Include important keywords from description in title."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct TitleBrandValidator;
impl Default for TitleBrandValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleBrandValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleBrandValidator {
    fn name(&self) -> &str {
        "title-brand-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let title = match &ctx.page.meta.title {
            Some(t) if !t.trim().is_empty() => t.trim().to_lowercase(),
            _ => return findings,
        };
        let org_name = ctx.page.structured_data.iter().find_map(|sd| {
            if sd.r#type.as_deref() == Some("Organization")
                || sd.r#type.as_deref() == Some("LocalBusiness")
            {
                sd.data
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_lowercase())
            } else {
                None
            }
        });
        if let Some(brand) = org_name {
            if !brand.is_empty() && !title.contains(&brand) {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "TITLEBRAND-V5001".to_string(),
                    title: "Title missing brand name".to_string(),
                    description: format!("Title doesn't contain brand '{brand}'."),
                    url: url.clone(),
                    recommendation: "Include brand name in title for recognition.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TitleLengthValidatorV5;
impl Default for TitleLengthValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleLengthValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleLengthValidatorV5 {
    fn name(&self) -> &str {
        "title-length-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.title {
            None => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Seo,
                    code: "TITLELEN-V5001".to_string(),
                    title: "Missing title tag".to_string(),
                    description: "No <title> found.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a 30-60 character title.".to_string(),
                });
            }
            Some(t) => {
                let len = t.len();
                if len < 30 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "TITLELEN-V5002".to_string(),
                        title: "Title too short".to_string(),
                        description: format!("{len} chars, aim for 30-60."),
                        url: url.clone(),
                        recommendation: "Expand to 30-60 characters.".to_string(),
                    });
                } else if len > 60 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "TITLELEN-V5003".to_string(),
                        title: "Title may truncate".to_string(),
                        description: format!("{len} chars, Google shows ~60."),
                        url: url.clone(),
                        recommendation: "Shorten to 60 characters.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct MetaDescriptionKeywordValidator;
impl Default for MetaDescriptionKeywordValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionKeywordValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionKeywordValidator {
    fn name(&self) -> &str {
        "meta-description-keyword-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let desc = match &ctx.page.meta.description {
            Some(d) if !d.trim().is_empty() => d.trim(),
            _ => return findings,
        };
        let title = match &ctx.page.meta.title {
            Some(t) => t.as_str(),
            None => "",
        };
        if !title.is_empty() {
            let title_words: Vec<&str> = title.split_whitespace().filter(|w| w.len() > 3).collect();
            let desc_lower = desc.to_lowercase();
            let missing: Vec<&str> = title_words
                .iter()
                .filter(|w| !desc_lower.contains(&w.to_lowercase()))
                .copied()
                .collect();
            if missing.len() > 1 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "METAKEY-V5001".to_string(),
                    title: "Description missing title keywords".to_string(),
                    description: format!("Description missing {} title keywords.", missing.len()),
                    url: url.clone(),
                    recommendation: "Include title keywords in description.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MetaDescriptionUniqueValidator;
impl Default for MetaDescriptionUniqueValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionUniqueValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionUniqueValidator {
    fn name(&self) -> &str {
        "meta-description-unique-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let desc = match &ctx.page.meta.description {
            Some(d) if !d.trim().is_empty() => d.trim().to_lowercase(),
            _ => return findings,
        };
        let title = match &ctx.page.meta.title {
            Some(t) if !t.trim().is_empty() => t.trim().to_lowercase(),
            _ => return findings,
        };
        if desc == title {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "METAUNIQ-V5001".to_string(),
                title: "Description identical to title".to_string(),
                description: "Description should complement title, not duplicate it.".to_string(),
                url: url.clone(),
                recommendation: "Write a unique description that expands on the title.".to_string(),
            });
        }
        findings
    }
}

pub struct CanonicalSelfReferenceValidatorV5;
impl Default for CanonicalSelfReferenceValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalSelfReferenceValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalSelfReferenceValidatorV5 {
    fn name(&self) -> &str {
        "canonical-self-reference-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let canonical_str = canonical.as_str();
            if canonical_str != url {
                if let Ok(page_url) = url::Url::parse(url) {
                    if canonical.path() == page_url.path() && canonical.query() == page_url.query()
                    {
                        // Same path - might be a trailing slash issue, not a real problem
                    } else {
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Seo,
                            code: "CANSELF-V5001".to_string(),
                            title: "Canonical points to different URL".to_string(),
                            description: format!(
                                "Canonical '{}' differs from page URL.",
                                canonical_str
                            ),
                            url: url.clone(),
                            recommendation: "Verify canonical points to correct URL.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

pub struct CanonicalChainValidatorV5;
impl Default for CanonicalChainValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalChainValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalChainValidatorV5 {
    fn name(&self) -> &str {
        "canonical-chain-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            // Compare the parsed canonical with the page URL. The old check
            // confirmed self-reference with a raw `href="..."` body match,
            // which missed single-quoted or reordered attributes and then
            // flagged genuinely self-referencing pages as "off-page".
            let is_self = canonical.as_str() == url
                || url::Url::parse(url)
                    .ok()
                    .zip(url::Url::parse(canonical.as_str()).ok())
                    .map_or(false, |(page, can)| {
                        page.path() == can.path()
                            && page.query() == can.query()
                            && page.host_str() == can.host_str()
                    });
            if !is_self {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "CANCHAIN-V5001".to_string(),
                    title: "Canonical points off-page".to_string(),
                    description: format!(
                        "Canonical '{}' doesn't match current URL.",
                        canonical.as_str()
                    ),
                    url: url.clone(),
                    recommendation: "Ensure no canonical chains exist.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CanonicalDepthValidatorV5;
impl Default for CanonicalDepthValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalDepthValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalDepthValidatorV5 {
    fn name(&self) -> &str {
        "canonical-depth-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let depth = canonical
                .path()
                .trim_matches('/')
                .split('/')
                .filter(|s| !s.is_empty())
                .count();
            if depth > 5 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "CANDEPTH-V5001".to_string(),
                    title: "Canonical URL too deep".to_string(),
                    description: format!("{} path segments in canonical URL.", depth),
                    url: url.clone(),
                    recommendation: "Flatten URL structure if possible.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HreflangReciprocalValidatorV5;
impl Default for HreflangReciprocalValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangReciprocalValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangReciprocalValidatorV5 {
    fn name(&self) -> &str {
        "hreflang-reciprocal-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() {
            return findings;
        }
        let has_self = tags.iter().any(|t| t.url.as_str() == *url);
        if !has_self {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "HREFRECIP-V5001".to_string(),
                title: "Missing self-referencing hreflang".to_string(),
                description: "No hreflang tag points to this page.".to_string(),
                url: url.clone(),
                recommendation: "Add self-referencing hreflang tag.".to_string(),
            });
        }
        findings
    }
}

pub struct HreflangXDefaultValidator;
impl Default for HreflangXDefaultValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangXDefaultValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangXDefaultValidator {
    fn name(&self) -> &str {
        "hreflang-x-default-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() {
            return findings;
        }
        let has_xd = tags.iter().any(|t| t.lang == "x-default");
        if !has_xd {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "HREFXD-V5001".to_string(),
                title: "Missing x-default hreflang".to_string(),
                description: "No x-default hreflang tag.".to_string(),
                url: url.clone(),
                recommendation: "Add x-default hreflang for default language.".to_string(),
            });
        }
        findings
    }
}

pub struct HreflangLocaleFormatValidator;
impl Default for HreflangLocaleFormatValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangLocaleFormatValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangLocaleFormatValidator {
    fn name(&self) -> &str {
        "hreflang-locale-format-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() {
            return findings;
        }
        for tag in tags {
            if tag.lang == "x-default" {
                continue;
            }
            let lang = &tag.lang;
            if !lang.contains('-') && !lang.contains('_') && lang.len() > 3 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "HREFLOCALE-V5001".to_string(),
                    title: "Hreflang missing region".to_string(),
                    description: format!("'{lang}' should include region (e.g., en-US)."),
                    url: url.clone(),
                    recommendation: "Use format like 'en-US' instead of 'en'.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SitemapCoverageValidatorV5;
impl Default for SitemapCoverageValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapCoverageValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapCoverageValidatorV5 {
    fn name(&self) -> &str {
        "sitemap-coverage-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let lower = robots.to_lowercase();
            if !lower.contains("sitemap:") {
                findings.push(Finding {
                    // Declaring the sitemap in robots.txt is optional (see
                    // SitemapCoverageDeepAnalyzerV2) — informational, not a
                    // defect.
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "SITEMAPCOV-V5001".to_string(),
                    title: "No sitemap in robots.txt".to_string(),
                    description: "robots.txt has no Sitemap directive.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Sitemap: directive to robots.txt.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SitemapLastmodValidator;
impl Default for SitemapLastmodValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapLastmodValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapLastmodValidator {
    fn name(&self) -> &str {
        "sitemap-lastmod-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") {
                let lastmod_count = body.matches("<lastmod>").count();
                let url_count = body.matches("<url>").count();
                if url_count > 0 && lastmod_count == 0 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "SITEMAPMOD-V5001".to_string(),
                        title: "Sitemap missing lastmod".to_string(),
                        description: "No lastmod dates in sitemap.".to_string(),
                        url: url.clone(),
                        recommendation: "Add lastmod to help crawlers prioritize.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct SitemapPriorityValidator;
impl Default for SitemapPriorityValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapPriorityValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapPriorityValidator {
    fn name(&self) -> &str {
        "sitemap-priority-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") {
                let priority_count = body.matches("<priority>").count();
                let url_count = body.matches("<url>").count();
                if url_count > 0 && priority_count == 0 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "SITEMAPPRI-V5001".to_string(),
                        title: "Sitemap missing priority".to_string(),
                        description: "No priority values in sitemap.".to_string(),
                        url: url.clone(),
                        recommendation: "Add priority to indicate page importance.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct RobotsTxtDisallowValidator;
impl Default for RobotsTxtDisallowValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtDisallowValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtDisallowValidator {
    fn name(&self) -> &str {
        "robots-txt-disallow-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let robots = match ctx.robots_txt {
            Some(r) => r,
            None => return findings,
        };
        if robots_txt_star_blanket_disallows_all(robots) {
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Seo,
                code: "ROBOTSDIS-V5001".to_string(),
                title: "robots.txt blocks all crawlers".to_string(),
                description: "Disallow: / blocks everything.".to_string(),
                url: url.clone(),
                recommendation: "Remove blanket disallow.".to_string(),
            });
        }
        let disallow_count = robots
            .lines()
            .filter(|l| l.trim().to_lowercase().starts_with("disallow:"))
            .count();
        if disallow_count > 50 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "ROBOTSDIS-V5002".to_string(),
                title: "robots.txt has many Disallow rules".to_string(),
                description: format!("{disallow_count} Disallow rules."),
                url: url.clone(),
                recommendation: "Simplify robots.txt rules.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// Accessibility V5 Analyzers (66-75)
// =========================================================================

pub struct TitleMissingValidator;
impl Default for TitleMissingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleMissingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleMissingValidator {
    fn name(&self) -> &str {
        "title-missing-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx
            .page
            .meta
            .title
            .as_deref()
            .map_or(true, |t| t.is_empty())
        {
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Seo,
                code: "TITLEMISS-V6081".to_string(),
                title: "Missing title tag".to_string(),
                description: "No title tag found.".to_string(),
                url: url.clone(),
                recommendation: "Add a unique, descriptive title.".to_string(),
            });
        }
        findings
    }
}

pub struct TitleKeywordDensityValidator;
impl Default for TitleKeywordDensityValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleKeywordDensityValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleKeywordDensityValidator {
    fn name(&self) -> &str {
        "title-keyword-density-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            if let Some(desc) = &ctx.page.meta.description {
                let title_lower = title.to_lowercase();
                let words: Vec<&str> = desc.split_whitespace().collect();
                if !words.is_empty() {
                    let first = words[0].to_lowercase();
                    if !title_lower.contains(&first) && first.len() > 3 {
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Seo,
                            code: "TITLEKDEN-V6082".to_string(),
                            title: "Title keyword density low".to_string(),
                            description: format!("First description word '{first}' not in title."),
                            url: url.clone(),
                            recommendation: "Include primary keyword in title.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

pub struct TitleBrandPlacementValidator;
impl Default for TitleBrandPlacementValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleBrandPlacementValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleBrandPlacementValidator {
    fn name(&self) -> &str {
        "title-brand-placement-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            if let Some(sd) = ctx
                .page
                .structured_data
                .iter()
                .find(|s| s.r#type.as_deref() == Some("Organization"))
            {
                if let Some(brand) = sd.data.get("name").and_then(|v| v.as_str()) {
                    let title_lower = title.to_lowercase();
                    let brand_lower = brand.to_lowercase();
                    if title_lower.contains(&brand_lower) {
                        if let Some(pos) = title_lower.find(&brand_lower) {
                            if pos > 0 && pos + brand.len() < title.len() {
                                findings.push(Finding {
                                    severity: Severity::Info,
                                    category: IssueCategory::Seo,
                                    code: "TITLEBRAND-V6083".to_string(),
                                    title: "Brand not at start of title".to_string(),
                                    description: format!(
                                        "Brand '{brand}' found at position {pos} in title."
                                    ),
                                    url: url.clone(),
                                    recommendation:
                                        "Place brand at the start of title for recognition."
                                            .to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

pub struct TitlePixelWidthValidator;
impl Default for TitlePixelWidthValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitlePixelWidthValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitlePixelWidthValidator {
    fn name(&self) -> &str {
        "title-pixel-width-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            let char_width: usize = title
                .chars()
                .map(|c| if c.is_ascii() { 8 } else { 12 })
                .sum();
            if char_width > 580 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "TITLEPX-V6084".to_string(),
                    title: "Title may be truncated in SERPs".to_string(),
                    description: format!("Estimated pixel width is {char_width}, max ~580."),
                    url: url.clone(),
                    recommendation: "Shorten title to fit within SERP pixel limit.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MetaDescriptionMissingValidator;
impl Default for MetaDescriptionMissingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionMissingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionMissingValidator {
    fn name(&self) -> &str {
        "meta-description-missing-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx
            .page
            .meta
            .description
            .as_deref()
            .map_or(true, |d| d.is_empty())
        {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "METADESCMISS-V6085".to_string(),
                title: "Missing meta description".to_string(),
                description: "No meta description tag found.".to_string(),
                url: url.clone(),
                recommendation: "Add a unique, compelling meta description.".to_string(),
            });
        }
        findings
    }
}

pub struct MetaDescriptionTooShortValidator;
impl Default for MetaDescriptionTooShortValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionTooShortValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionTooShortValidator {
    fn name(&self) -> &str {
        "meta-description-too-short-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(desc) = &ctx.page.meta.description {
            if desc.len() < 70 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "METADESSSHORT-V6086".to_string(),
                    title: "Meta description too short".to_string(),
                    description: format!("Description is {} chars, recommend 120-160.", desc.len()),
                    url: url.clone(),
                    recommendation: "Expand meta description to 120-160 chars.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MetaDescriptionUniquenessValidator;
impl Default for MetaDescriptionUniquenessValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionUniquenessValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionUniquenessValidator {
    fn name(&self) -> &str {
        "meta-description-uniqueness-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let (Some(title), Some(desc)) = (&ctx.page.meta.title, &ctx.page.meta.description) {
            if title == desc {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "METADESCUNIQ-V6087".to_string(),
                    title: "Meta description matches title".to_string(),
                    description: "Title and description are identical.".to_string(),
                    url: url.clone(),
                    recommendation: "Write a unique meta description.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CanonicalMissingValidator;
impl Default for CanonicalMissingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalMissingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalMissingValidator {
    fn name(&self) -> &str {
        "canonical-missing-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.canonical.is_none() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "CANMISS-V6088".to_string(),
                title: "Missing canonical URL".to_string(),
                description: "No canonical link tag found.".to_string(),
                url: url.clone(),
                recommendation: "Add a self-referencing canonical URL.".to_string(),
            });
        }
        findings
    }
}

pub struct CanonicalSelfReferenceDeepValidator;
impl Default for CanonicalSelfReferenceDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalSelfReferenceDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalSelfReferenceDeepValidator {
    fn name(&self) -> &str {
        "canonical-self-reference-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            if canonical.as_str() != url {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "CANSELFRF-V6089".to_string(),
                    title: "Canonical does not self-reference".to_string(),
                    description: format!("Canonical points to '{}', page is '{}'.", canonical, url),
                    url: url.clone(),
                    recommendation: "Set canonical to the current page URL.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CanonicalChainDeepValidator;
impl Default for CanonicalChainDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalChainDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalChainDeepValidator {
    fn name(&self) -> &str {
        "canonical-chain-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            // Count real <link rel="canonical"> elements; counting the raw
            // string also matched code samples and docs that mention the
            // attribute, flagging single-canonical pages as duplicates.
            let canonical_count = count_canonical_links(body);
            if canonical_count > 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "CANCHAIN-V6090".to_string(),
                    title: "Multiple canonical tags".to_string(),
                    description: format!("{canonical_count} canonical tags found."),
                    url: url.clone(),
                    recommendation: "Use only one canonical tag per page.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CanonicalDepthDeepValidator;
impl Default for CanonicalDepthDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalDepthDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalDepthDeepValidator {
    fn name(&self) -> &str {
        "canonical-depth-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let path = canonical.path();
            let depth = path.trim_matches('/').matches('/').count();
            if depth > 4 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "CANDEEP-V6091".to_string(),
                    title: "Canonical URL path deeply nested".to_string(),
                    description: format!("Canonical path has {} levels.", depth + 1),
                    url: url.clone(),
                    recommendation: "Consider flattening URL structure.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HreflangMissingValidator;
impl Default for HreflangMissingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangMissingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangMissingValidator {
    fn name(&self) -> &str {
        "hreflang-missing-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.hreflang.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "HREFMISS-V6092".to_string(),
                title: "No hreflang tags".to_string(),
                description: "No hreflang link tags found.".to_string(),
                url: url.clone(),
                recommendation: "Add hreflang tags for international targeting.".to_string(),
            });
        }
        findings
    }
}

pub struct HreflangReciprocalDeepValidator;
impl Default for HreflangReciprocalDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangReciprocalDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangReciprocalDeepValidator {
    fn name(&self) -> &str {
        "hreflang-reciprocal-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.hreflang.len() > 1 {
            let langs: Vec<&str> = ctx
                .page
                .meta
                .hreflang
                .iter()
                .map(|h| h.lang.as_str())
                .collect();
            let unique: std::collections::HashSet<&str> = langs.iter().copied().collect();
            if unique.len() < langs.len() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "HREFRECIP-V6093".to_string(),
                    title: "Duplicate hreflang languages".to_string(),
                    description: format!(
                        "{} hreflang entries but {} unique languages.",
                        langs.len(),
                        unique.len()
                    ),
                    url: url.clone(),
                    recommendation: "Remove duplicate hreflang entries.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HreflangXDefaultMissingValidator;
impl Default for HreflangXDefaultMissingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangXDefaultMissingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangXDefaultMissingValidator {
    fn name(&self) -> &str {
        "hreflang-x-default-missing-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.meta.hreflang.is_empty() {
            let has_xdefault = ctx.page.meta.hreflang.iter().any(|h| h.lang == "x-default");
            if !has_xdefault {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "HREFXD-V6094".to_string(),
                    title: "Missing hreflang x-default".to_string(),
                    description: "No x-default hreflang tag found.".to_string(),
                    url: url.clone(),
                    recommendation: "Add hreflang x-default for fallback language.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HreflangLocaleFormatDeepValidator;
impl Default for HreflangLocaleFormatDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangLocaleFormatDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangLocaleFormatDeepValidator {
    fn name(&self) -> &str {
        "hreflang-locale-format-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for h in &ctx.page.meta.hreflang {
            if h.lang == "x-default" {
                continue;
            }
            if !h.lang.contains('-') && h.lang.len() > 2 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "HREFFMT-V6095".to_string(),
                    title: "Hreflang locale format".to_string(),
                    description: format!("'{}' may not follow ISO 639-1/BCP 47 format.", h.lang),
                    url: url.clone(),
                    recommendation: "Use ISO 639-1 format (e.g., 'en', 'en-US').".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SitemapMissingValidator;
impl Default for SitemapMissingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapMissingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapMissingValidator {
    fn name(&self) -> &str {
        "sitemap-missing-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            if !robots.to_lowercase().contains("sitemap:") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "SITEMAPMISS-V6096".to_string(),
                    title: "No sitemap in robots.txt".to_string(),
                    description: "robots.txt doesn't reference a sitemap.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Sitemap directive to robots.txt.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SitemapLastmodFormatValidator;
impl Default for SitemapLastmodFormatValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapLastmodFormatValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapLastmodFormatValidator {
    fn name(&self) -> &str {
        "sitemap-lastmod-format-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") || body.contains("<sitemapindex") {
                let lower = body.to_lowercase();
                let lastmod_count = lower.matches("<lastmod>").count();
                let valid_iso =
                    body.matches("<lastmod>20").count() + body.matches("<lastmod>19").count();
                if lastmod_count > 0 && valid_iso < lastmod_count / 2 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "SITEMAPLMFMT-V6097".to_string(),
                        title: "Sitemap lastmod format issues".to_string(),
                        description: format!("{lastmod_count} lastmod entries, many not ISO 8601."),
                        url: url.clone(),
                        recommendation: "Use W3C Datetime format for lastmod.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct SitemapPriorityRangeValidator;
impl Default for SitemapPriorityRangeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapPriorityRangeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapPriorityRangeValidator {
    fn name(&self) -> &str {
        "sitemap-priority-range-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") {
                for line in body.lines() {
                    if line.contains("<priority>") {
                        if let Some(start) = line.find("<priority>") {
                            let after = &line[start + 9..];
                            if let Some(end) = after.find("</priority>") {
                                let val = &after[..end];
                                if let Ok(p) = val.parse::<f64>() {
                                    if p < 0.0 || p > 1.0 {
                                        findings.push(Finding {
                                            severity: Severity::Warning,
                                            category: IssueCategory::Seo,
                                            code: "SITEMAPPRI-V6098".to_string(),
                                            title: "Sitemap priority out of range".to_string(),
                                            description: format!(
                                                "Priority {val} is outside 0.0-1.0."
                                            ),
                                            url: url.clone(),
                                            recommendation: "Set priority between 0.0 and 1.0."
                                                .to_string(),
                                        });
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

pub struct RobotsTxtEmptyValidator;
impl Default for RobotsTxtEmptyValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtEmptyValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtEmptyValidator {
    fn name(&self) -> &str {
        "robots-txt-empty-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            if robots.trim().is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "ROBOTSEMPTY-V6099".to_string(),
                    title: "Empty robots.txt".to_string(),
                    description: "robots.txt file is empty.".to_string(),
                    url: url.clone(),
                    recommendation: "Add directives to robots.txt.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct RobotsTxtDisallowDepthValidator;
impl Default for RobotsTxtDisallowDepthValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtDisallowDepthValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtDisallowDepthValidator {
    fn name(&self) -> &str {
        "robots-txt-disallow-depth-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let deep_blocks = robots
                .lines()
                .filter(|l| l.starts_with("Disallow:") || l.starts_with("disallow:"))
                .filter(|l| l.matches('/').count() > 5)
                .count();
            if deep_blocks > 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "ROBOTSDEPTH-V6100".to_string(),
                    title: "Deep disallow paths in robots.txt".to_string(),
                    description: format!("{deep_blocks} disallow rule(s) with deep path depth."),
                    url: url.clone(),
                    recommendation: "Simplify disallow rules to directory level.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct RobotsTxtWildcardDisallowValidator;
impl Default for RobotsTxtWildcardDisallowValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtWildcardDisallowValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtWildcardDisallowValidator {
    fn name(&self) -> &str {
        "robots-txt-wildcard-disallow-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            if robots_txt_star_blanket_disallows_all(robots) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ROBOTSWILD-V6101".to_string(),
                    title: "robots.txt blocks all crawlers".to_string(),
                    description: "Disallow: / blocks all crawling.".to_string(),
                    url: url.clone(),
                    recommendation: "Review disallow rules to avoid blocking everything."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct RobotsTxtMissingUserAgentValidator;
impl Default for RobotsTxtMissingUserAgentValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtMissingUserAgentValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtMissingUserAgentValidator {
    fn name(&self) -> &str {
        "robots-txt-missing-user-agent-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            if !robots.to_lowercase().contains("user-agent:") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ROBOTSUA-V6102".to_string(),
                    title: "robots.txt missing User-agent".to_string(),
                    description: "No User-agent directive found.".to_string(),
                    url: url.clone(),
                    recommendation: "Add User-agent directive to robots.txt.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct InternalLinksDiversityValidator;
impl Default for InternalLinksDiversityValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinksDiversityValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalLinksDiversityValidator {
    fn name(&self) -> &str {
        "internal-links-diversity-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.is_external)
            .map(|l| l.text.trim())
            .filter(|t| !t.is_empty())
            .collect();
        if internal.len() > 5 {
            let unique_texts: std::collections::HashSet<&str> = internal.iter().copied().collect();
            if unique_texts.len() < internal.len() / 2 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Links,
                    code: "INTDIV-V6103".to_string(),
                    title: "Low internal link text diversity".to_string(),
                    description: format!(
                        "{} internal links but only {} unique texts.",
                        internal.len(),
                        unique_texts.len()
                    ),
                    url: url.clone(),
                    recommendation: "Diversify anchor text for internal links.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct InternalLinksDepthDistributionValidator;
impl Default for InternalLinksDepthDistributionValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinksDepthDistributionValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalLinksDepthDistributionValidator {
    fn name(&self) -> &str {
        "internal-links-depth-distribution-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.is_external)
            .map(|l| l.href.as_str())
            .collect();
        if internal.len() > 10 {
            let base = url::Url::parse(url).ok();
            if let Some(base) = base {
                let deep = internal
                    .iter()
                    .filter(|href| {
                        url::Url::parse(href).ok().map_or(false, |u| {
                            u.path().matches('/').count() > 4 && u.host_str() == base.host_str()
                        })
                    })
                    .count();
                if deep > internal.len() / 3 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Links,
                        code: "INTDEPTH-V6104".to_string(),
                        title: "Many deep internal links".to_string(),
                        description: format!(
                            "{deep}/{} internal links point to deeply nested URLs.",
                            internal.len()
                        ),
                        url: url.clone(),
                        recommendation: "Flatten URL structure for important pages.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct ExternalLinksNofollowAnalysisValidator;
impl Default for ExternalLinksNofollowAnalysisValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ExternalLinksNofollowAnalysisValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ExternalLinksNofollowAnalysisValidator {
    fn name(&self) -> &str {
        "external-links-nofollow-analysis-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let ext_total = ctx.page.links.iter().filter(|l| l.is_external).count();
        if ext_total > 3 {
            let nofollow_ext = ctx
                .page
                .links
                .iter()
                .filter(|l| l.is_external && l.rel.iter().any(|r| r == "nofollow"))
                .count();
            if nofollow_ext == 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Links,
                    code: "EXTNOFOLLOW-V6106".to_string(),
                    title: "No nofollow on external links".to_string(),
                    description: format!("{} external links, none nofollowed.", ext_total),
                    url: url.clone(),
                    recommendation: "Consider nofollow for untrusted external links.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TitleMissingDeepValidator;
impl Default for TitleMissingDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleMissingDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleMissingDeepValidator {
    fn name(&self) -> &str {
        "title-missing-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.title {
            None => {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Seo,
                    code: "TITLEMISS001".to_string(),
                    title: "Missing title tag".to_string(),
                    description: "No title tag found on page.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a unique, descriptive title tag.".to_string(),
                });
            }
            Some(title) if title.trim().is_empty() => {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Seo,
                    code: "TITLEMISS001".to_string(),
                    title: "Empty title tag".to_string(),
                    description: "Title tag exists but is empty.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a unique, descriptive title tag.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

pub struct TitleKeywordDensityDeepValidator;
impl Default for TitleKeywordDensityDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleKeywordDensityDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleKeywordDensityDeepValidator {
    fn name(&self) -> &str {
        "title-keyword-density-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let (Some(title), Some(desc)) = (&ctx.page.meta.title, &ctx.page.meta.description) {
            let title_words: Vec<&str> = title.split_whitespace().collect();
            let desc_words: Vec<&str> = desc.split_whitespace().collect();
            if !title_words.is_empty() && !desc_words.is_empty() {
                let overlap = title_words
                    .iter()
                    .filter(|tw| desc_words.iter().any(|dw| tw.eq_ignore_ascii_case(dw)))
                    .count();
                let ratio = overlap as f64 / title_words.len() as f64;
                if ratio < 0.2 && title_words.len() > 2 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "TITLEKDEN001".to_string(),
                        title: "Low title-description keyword overlap".to_string(),
                        description: format!(
                            "Only {overlap}/{} title words appear in description.",
                            title_words.len()
                        ),
                        url: url.clone(),
                        recommendation: "Include key title words in meta description.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct TitleBrandPlacementDeepValidator;
impl Default for TitleBrandPlacementDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleBrandPlacementDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleBrandPlacementDeepValidator {
    fn name(&self) -> &str {
        "title-brand-placement-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            let title_lower = title.to_lowercase();
            if title_lower.ends_with(" | ")
                || title_lower.ends_with(" - ")
                || title_lower.ends_with(" – ")
                || title_lower.ends_with(" — ")
            {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "TITLEBRAND001".to_string(), title: "Title ends with separator".to_string(), description: "Title ends with a brand separator which may indicate brand is at the end.".to_string(), url: url.clone(), recommendation: "Consider placing the brand name at the beginning of the title.".to_string() });
            }
            if title.contains(" | ") || title.contains(" - ") {
                let parts: Vec<&str> = title.splitn(2, |c| c == '|' || c == '-').collect();
                if parts.len() == 2 {
                    let first = parts[0].trim();
                    let second = parts[1].trim();
                    if first.len() < 5 && second.len() > 20 {
                        findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "TITLEBRAND001".to_string(), title: "Short brand prefix in title".to_string(), description: format!("Brand '{first}' takes minimal space; primary content is after separator."), url: url.clone(), recommendation: "Ensure the primary keyword appears before the brand in the title.".to_string() });
                    }
                }
            }
        }
        findings
    }
}

pub struct MetaDescriptionMissingDeepValidator;
impl Default for MetaDescriptionMissingDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionMissingDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionMissingDeepValidator {
    fn name(&self) -> &str {
        "meta-description-missing-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.description {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "METADESCMISS001".to_string(),
                    title: "Missing meta description".to_string(),
                    description: "No meta description tag found.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a unique, compelling meta description.".to_string(),
                });
            }
            Some(desc) if desc.trim().is_empty() => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "METADESCMISS001".to_string(),
                    title: "Empty meta description".to_string(),
                    description: "Meta description tag exists but is empty.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a unique, compelling meta description.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

pub struct MetaDescriptionTooShortDeepValidator;
impl Default for MetaDescriptionTooShortDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionTooShortDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionTooShortDeepValidator {
    fn name(&self) -> &str {
        "meta-description-too-short-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(desc) = &ctx.page.meta.description {
            if !desc.is_empty() && desc.len() < 70 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "METADESSSHORT001".to_string(),
                    title: "Meta description too short".to_string(),
                    description: format!(
                        "Description is {} chars, recommended 120-160.",
                        desc.len()
                    ),
                    url: url.clone(),
                    recommendation: "Expand meta description to 120-160 characters.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MetaDescriptionUniquenessDeepValidator;
impl Default for MetaDescriptionUniquenessDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionUniquenessDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionUniquenessDeepValidator {
    fn name(&self) -> &str {
        "meta-description-uniqueness-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let (Some(title), Some(desc)) = (&ctx.page.meta.title, &ctx.page.meta.description) {
            if title == desc {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "METADESCUNIQ001".to_string(),
                    title: "Meta description identical to title".to_string(),
                    description: "Title and meta description are exactly the same.".to_string(),
                    url: url.clone(),
                    recommendation: "Write a unique meta description that complements the title."
                        .to_string(),
                });
            }
            if !desc.is_empty() && title.len() > 10 {
                let title_lower = title.to_lowercase();
                let desc_lower = desc.to_lowercase();
                if title_lower == desc_lower.chars().take(title.len()).collect::<String>() {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "METADESCUNIQ001".to_string(),
                        title: "Meta description starts with title".to_string(),
                        description: "Description begins with the exact title text.".to_string(),
                        url: url.clone(),
                        recommendation:
                            "Write a distinct description that adds value beyond the title."
                                .to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct CanonicalMissingDeepValidator;
impl Default for CanonicalMissingDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalMissingDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalMissingDeepValidator {
    fn name(&self) -> &str {
        "canonical-missing-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.canonical.is_none() {
            let is_paginated =
                url.contains("page=") || url.contains("/page/") || url.contains("?p=");
            if is_paginated {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "CANMISS001".to_string(), title: "Missing canonical on paginated page".to_string(), description: "Paginated page has no canonical URL.".to_string(), url: url.clone(), recommendation: "Add a canonical URL pointing to the current paginated URL or the first page.".to_string() });
            } else {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "CANMISS001".to_string(),
                    title: "Missing canonical URL".to_string(),
                    description: "No canonical link tag found.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a self-referencing canonical URL.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CanonicalSelfReferenceDeepDeepValidator;
impl Default for CanonicalSelfReferenceDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalSelfReferenceDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalSelfReferenceDeepDeepValidator {
    fn name(&self) -> &str {
        "canonical-self-reference-deep-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let canonical_str = canonical.as_str();
            if canonical_str != url {
                let same_path = canonical.path()
                    == url::Url::parse(url)
                        .map(|u| u.path().to_string())
                        .unwrap_or_default();
                if same_path {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "CANSELFRF001".to_string(),
                        title: "Canonical has different scheme/host".to_string(),
                        description: format!(
                            "Canonical '{}' differs from page URL '{}' in scheme/host.",
                            canonical_str, url
                        ),
                        url: url.clone(),
                        recommendation: "Ensure canonical uses the exact same URL as the page."
                            .to_string(),
                    });
                } else {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "CANSELFRF001".to_string(),
                        title: "Canonical does not self-reference".to_string(),
                        description: format!(
                            "Canonical points to '{}', page is '{}'.",
                            canonical_str, url
                        ),
                        url: url.clone(),
                        recommendation: "Set canonical to the current page URL.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct CanonicalChainDeepDeepValidator;
impl Default for CanonicalChainDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalChainDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalChainDeepDeepValidator {
    fn name(&self) -> &str {
        "canonical-chain-deep-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            // See CanonicalChainDeepValidator: count real link elements.
            let canonical_count = count_canonical_links(body);
            if canonical_count > 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "CANCHAIN001".to_string(),
                    title: "Multiple canonical tags".to_string(),
                    description: format!("{canonical_count} canonical tags found in HTML."),
                    url: url.clone(),
                    recommendation: "Use only one canonical tag per page.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HreflangMissingDeepValidator;
impl Default for HreflangMissingDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangMissingDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangMissingDeepValidator {
    fn name(&self) -> &str {
        "hreflang-missing-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.hreflang.is_empty() {
            let has_html_lang = ctx.page.has_lang_attribute;
            let has_multilingual_signal = url.contains("/en/")
                || url.contains("/fr/")
                || url.contains("/de/")
                || url.contains("/es/")
                || url.contains("/ja/");
            if has_multilingual_signal && !has_html_lang {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "HREFMISS001".to_string(),
                    title: "Missing hreflang on multilingual site".to_string(),
                    description:
                        "URL pattern suggests multilingual content but no hreflang tags found."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add hreflang tags for all language variants.".to_string(),
                });
            } else if !has_multilingual_signal {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "HREFMISS001".to_string(),
                    title: "No hreflang tags".to_string(),
                    description: "No hreflang annotations found on page.".to_string(),
                    url: url.clone(),
                    recommendation: "Add hreflang tags if serving content in multiple languages."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct HreflangReciprocalDeepDeepValidator;
impl Default for HreflangReciprocalDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangReciprocalDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangReciprocalDeepDeepValidator {
    fn name(&self) -> &str {
        "hreflang-reciprocal-deep-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.hreflang.is_empty() {
            return findings;
        }
        let mut lang_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for h in &ctx.page.meta.hreflang {
            *lang_counts.entry(h.lang.clone()).or_insert(0) += 1;
        }
        for (lang, count) in &lang_counts {
            if *count > 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "HREFRECIP001".to_string(),
                    title: format!("Duplicate hreflang for '{lang}'"),
                    description: format!(
                        "hreflang '{lang}' appears {count} times on the same page."
                    ),
                    url: url.clone(),
                    recommendation: "Each language should have only one hreflang tag per page."
                        .to_string(),
                });
            }
        }
        let has_x_default = ctx.page.meta.hreflang.iter().any(|h| h.lang == "x-default");
        if !has_x_default && ctx.page.meta.hreflang.len() > 1 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "HREFRECIP001".to_string(),
                title: "Missing x-default hreflang".to_string(),
                description: "Multiple hreflang tags but no x-default variant.".to_string(),
                url: url.clone(),
                recommendation: "Add an x-default hreflang tag for the fallback language."
                    .to_string(),
            });
        }
        findings
    }
}

pub struct HreflangXDefaultMissingDeepValidator;
impl Default for HreflangXDefaultMissingDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangXDefaultMissingDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangXDefaultMissingDeepValidator {
    fn name(&self) -> &str {
        "hreflang-x-default-missing-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.meta.hreflang.is_empty() {
            let has_x_default = ctx.page.meta.hreflang.iter().any(|h| h.lang == "x-default");
            if !has_x_default {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "HREFXD001".to_string(),
                    title: "Missing hreflang x-default".to_string(),
                    description: "No x-default hreflang tag found.".to_string(),
                    url: url.clone(),
                    recommendation:
                        "Add an x-default hreflang tag for the fallback/default language."
                            .to_string(),
                });
            }
        }
        findings
    }
}

pub struct SitemapMissingDeepValidator;
impl Default for SitemapMissingDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapMissingDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapMissingDeepValidator {
    fn name(&self) -> &str {
        "sitemap-missing-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let has_sitemap = robots
                .lines()
                .any(|l| l.to_lowercase().starts_with("sitemap:"));
            if !has_sitemap {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "SITEMAPMISS001".to_string(),
                    title: "No sitemap in robots.txt".to_string(),
                    description: "robots.txt does not reference any sitemap.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Sitemap: directive to robots.txt.".to_string(),
                });
            }
        } else {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "SITEMAPMISS001".to_string(),
                title: "No robots.txt available".to_string(),
                description: "Could not fetch robots.txt to check for sitemap reference."
                    .to_string(),
                url: url.clone(),
                recommendation: "Ensure robots.txt is accessible and references your sitemap."
                    .to_string(),
            });
        }
        findings
    }
}

pub struct RobotsTxtEmptyDeepValidator;
impl Default for RobotsTxtEmptyDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtEmptyDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtEmptyDeepValidator {
    fn name(&self) -> &str {
        "robots-txt-empty-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let trimmed = robots.trim();
            if trimmed.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ROBOTSEMPTY001".to_string(),
                    title: "Empty robots.txt".to_string(),
                    description: "robots.txt file is empty.".to_string(),
                    url: url.clone(),
                    recommendation: "Add at least a User-agent directive to robots.txt."
                        .to_string(),
                });
            } else {
                let has_user_agent = trimmed
                    .lines()
                    .any(|l| l.to_lowercase().starts_with("user-agent:"));
                if !has_user_agent {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "ROBOTSEMPTY001".to_string(),
                        title: "robots.txt missing User-agent".to_string(),
                        description: "robots.txt exists but has no User-agent directive."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add User-agent: * directive to robots.txt.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct InternalLinksDiversityDeepValidator;
impl Default for InternalLinksDiversityDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinksDiversityDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalLinksDiversityDeepValidator {
    fn name(&self) -> &str {
        "internal-links-diversity-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<&crate::parser::ExtractedLink> =
            ctx.page.links.iter().filter(|l| !l.is_external).collect();
        if internal.len() < 3 {
            return findings;
        }
        let unique_targets: std::collections::HashSet<&str> =
            internal.iter().map(|l| l.href.as_str()).collect();
        let unique_texts: std::collections::HashSet<&str> =
            internal.iter().map(|l| l.text.trim()).collect();
        if !unique_targets.is_empty() {
            let diversity_ratio = unique_targets.len() as f64 / internal.len() as f64;
            if diversity_ratio < 0.5 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "INTDIV001".to_string(),
                    title: "Low internal link diversity".to_string(),
                    description: format!(
                        "Only {}/{} internal links point to unique targets.",
                        unique_targets.len(),
                        internal.len()
                    ),
                    url: url.clone(),
                    recommendation: "Diversify internal link targets to distribute link equity."
                        .to_string(),
                });
            }
        }
        if !unique_texts.is_empty() {
            let text_diversity = unique_texts.len() as f64 / internal.len() as f64;
            if text_diversity < 0.3 && internal.len() > 5 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "INTDIV001".to_string(),
                    title: "Low internal link anchor text diversity".to_string(),
                    description: format!(
                        "{}/{} unique anchor texts.",
                        unique_texts.len(),
                        internal.len()
                    ),
                    url: url.clone(),
                    recommendation: "Use varied, descriptive anchor text for internal links."
                        .to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// V8 Content Validators (Dataset, HowTo, Recipe deep validators)
// =========================================================================

pub struct InternalLinksDepthDeepValidator;
impl Default for InternalLinksDepthDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinksDepthDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalLinksDepthDeepValidator {
    fn name(&self) -> &str {
        "internal-links-depth-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.is_external)
            .map(|l| l.href.as_str())
            .collect();
        if internal.len() > 5 {
            let base = url::Url::parse(url).ok();
            if let Some(base) = base {
                let base_host = base.host_str().unwrap_or("");
                let depth_counts: std::collections::HashMap<usize, usize> = internal
                    .iter()
                    .filter_map(|href| {
                        url::Url::parse(href).ok().and_then(|u| {
                            if u.host_str() == Some(base_host) {
                                let depth = u
                                    .path()
                                    .trim_matches('/')
                                    .split('/')
                                    .filter(|s| !s.is_empty())
                                    .count();
                                Some(depth)
                            } else {
                                None
                            }
                        })
                    })
                    .fold(std::collections::HashMap::new(), |mut acc, d| {
                        *acc.entry(d).or_insert(0) += 1;
                        acc
                    });
                let deep_pages: usize = depth_counts
                    .iter()
                    .filter(|(&d, _)| d > 4)
                    .map(|(_, &c)| c)
                    .sum();
                if deep_pages > internal.len() / 4 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "INTDEPTH001".to_string(),
                        title: "Many deeply nested internal links".to_string(),
                        description: format!(
                            "{}/{} internal links point to URLs with 4+ path segments.",
                            deep_pages,
                            internal.len()
                        ),
                        url: url.clone(),
                        recommendation: "Flatten URL structure for important pages.".to_string(),
                    });
                }
                let max_depth = depth_counts.keys().copied().max().unwrap_or(0);
                if max_depth > 6 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "INTDEPTH002".to_string(),
                        title: "Very deep internal link targets".to_string(),
                        description: format!(
                            "Deepest internal link has {} path segments.",
                            max_depth
                        ),
                        url: url.clone(),
                        recommendation: "Reduce URL depth for better SEO.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct TitleMissingDeepDeepValidator;
impl Default for TitleMissingDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleMissingDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleMissingDeepDeepValidator {
    fn name(&self) -> &str {
        "title-missing-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.title {
            None => {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Seo,
                    code: "TITLEMISS-V2001".to_string(),
                    title: "Missing title tag (deep-deep)".to_string(),
                    description: "No title tag found on page in deep analysis.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a unique, descriptive title tag.".to_string(),
                });
            }
            Some(title) if title.trim().is_empty() => {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Seo,
                    code: "TITLEMISS-V2001".to_string(),
                    title: "Empty title tag (deep-deep)".to_string(),
                    description: "Title tag exists but is empty in deep analysis.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a unique, descriptive title tag.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

pub struct TitleKeywordDensityDeepDeepValidator;
impl Default for TitleKeywordDensityDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleKeywordDensityDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleKeywordDensityDeepDeepValidator {
    fn name(&self) -> &str {
        "title-keyword-density-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let (Some(title), Some(desc)) = (&ctx.page.meta.title, &ctx.page.meta.description) {
            let title_words: Vec<&str> = title.split_whitespace().collect();
            let desc_words: Vec<&str> = desc.split_whitespace().collect();
            if !title_words.is_empty() && !desc_words.is_empty() {
                let overlap = title_words
                    .iter()
                    .filter(|tw| desc_words.iter().any(|dw| tw.eq_ignore_ascii_case(dw)))
                    .count();
                let ratio = overlap as f64 / title_words.len() as f64;
                if ratio < 0.2 && title_words.len() > 2 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "TITLEKDEN-V2001".to_string(),
                        title: "Low title-description keyword overlap (deep-deep)".to_string(),
                        description: format!(
                            "Only {overlap}/{} title words appear in description in deep analysis.",
                            title_words.len()
                        ),
                        url: url.clone(),
                        recommendation: "Include key title words in meta description.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct TitleBrandPlacementDeepDeepValidator;
impl Default for TitleBrandPlacementDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleBrandPlacementDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleBrandPlacementDeepDeepValidator {
    fn name(&self) -> &str {
        "title-brand-placement-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            let title_lower = title.to_lowercase();
            if title_lower.ends_with(" | ")
                || title_lower.ends_with(" - ")
                || title_lower.ends_with(" – ")
                || title_lower.ends_with(" — ")
            {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "TITLEBRAND-V2001".to_string(), title: "Title ends with separator (deep-deep)".to_string(), description: "Title ends with a brand separator in deep analysis.".to_string(), url: url.clone(), recommendation: "Consider placing the brand name at the beginning of the title.".to_string() });
            }
        }
        findings
    }
}

pub struct MetaDescriptionMissingDeepDeepValidator;
impl Default for MetaDescriptionMissingDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionMissingDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionMissingDeepDeepValidator {
    fn name(&self) -> &str {
        "meta-description-missing-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.description {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "METADESCMISS-V2001".to_string(),
                    title: "Missing meta description (deep-deep)".to_string(),
                    description: "No meta description tag found in deep analysis.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a unique, compelling meta description.".to_string(),
                });
            }
            Some(desc) if desc.trim().is_empty() => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "METADESCMISS-V2001".to_string(),
                    title: "Empty meta description (deep-deep)".to_string(),
                    description: "Meta description tag exists but is empty in deep analysis."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a unique, compelling meta description.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

pub struct MetaDescriptionTooShortDeepDeepValidator;
impl Default for MetaDescriptionTooShortDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionTooShortDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionTooShortDeepDeepValidator {
    fn name(&self) -> &str {
        "meta-description-too-short-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(desc) = &ctx.page.meta.description {
            if !desc.is_empty() && desc.len() < 70 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "METADESSSHORT-V2001".to_string(),
                    title: "Meta description too short (deep-deep)".to_string(),
                    description: format!(
                        "Description is {} chars, recommended 120-160 in deep analysis.",
                        desc.len()
                    ),
                    url: url.clone(),
                    recommendation: "Expand meta description to 120-160 characters.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MetaDescriptionUniquenessDeepDeepValidator;
impl Default for MetaDescriptionUniquenessDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionUniquenessDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionUniquenessDeepDeepValidator {
    fn name(&self) -> &str {
        "meta-description-uniqueness-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let (Some(title), Some(desc)) = (&ctx.page.meta.title, &ctx.page.meta.description) {
            if title == desc {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "METADESCUNIQ-V2001".to_string(),
                    title: "Meta description identical to title (deep-deep)".to_string(),
                    description:
                        "Title and meta description are exactly the same in deep analysis."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Write a unique meta description that complements the title."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct CanonicalMissingDeepDeepValidator;
impl Default for CanonicalMissingDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalMissingDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalMissingDeepDeepValidator {
    fn name(&self) -> &str {
        "canonical-missing-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.canonical.is_none() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "CANMISS-V2001".to_string(),
                title: "Missing canonical URL (deep-deep)".to_string(),
                description: "No canonical link tag found in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add a self-referencing canonical URL.".to_string(),
            });
        }
        findings
    }
}

pub struct CanonicalSelfReferenceDeepDeepDeepValidator;
impl Default for CanonicalSelfReferenceDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalSelfReferenceDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalSelfReferenceDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "canonical-self-reference-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let canonical_str = canonical.as_str();
            if canonical_str != url {
                let same_path = canonical.path()
                    == url::Url::parse(url)
                        .map(|u| u.path().to_string())
                        .unwrap_or_default();
                if same_path {
                    findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "CANSELFRF-V2001".to_string(), title: "Canonical has different scheme/host (deep-deep-deep)".to_string(), description: format!("Canonical '{}' differs from page URL '{}' in scheme/host in deep analysis.", canonical_str, url), url: url.clone(), recommendation: "Ensure canonical uses the exact same URL as the page.".to_string() });
                } else {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "CANSELFRF-V2001".to_string(),
                        title: "Canonical does not self-reference (deep-deep-deep)".to_string(),
                        description: format!(
                            "Canonical points to '{}', page is '{}' in deep analysis.",
                            canonical_str, url
                        ),
                        url: url.clone(),
                        recommendation: "Set canonical to the current page URL.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct CanonicalChainDeepDeepDeepValidator;
impl Default for CanonicalChainDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalChainDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalChainDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "canonical-chain-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            // See CanonicalChainDeepValidator: count real link elements.
            let canonical_count = count_canonical_links(body);
            if canonical_count > 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "CANCHAIN-V2001".to_string(),
                    title: "Multiple canonical tags (deep-deep-deep)".to_string(),
                    description: format!(
                        "{canonical_count} canonical tags found in HTML in deep analysis."
                    ),
                    url: url.clone(),
                    recommendation: "Use only one canonical tag per page.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HreflangMissingDeepDeepValidator;
impl Default for HreflangMissingDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangMissingDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangMissingDeepDeepValidator {
    fn name(&self) -> &str {
        "hreflang-missing-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.hreflang.is_empty() {
            let has_x_robots_tag = ctx.headers.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case("X-Robots-Tag") && v.to_lowercase().contains("hreflang")
            });
            if !has_x_robots_tag {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "HREFMISS-V2001".to_string(),
                    title: "Missing hreflang tags (deep-deep)".to_string(),
                    description: "No hreflang link tags found in deep analysis.".to_string(),
                    url: url.clone(),
                    recommendation: "Add hreflang tags for multi-language sites.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HreflangReciprocalDeepDeepDeepValidator;
impl Default for HreflangReciprocalDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangReciprocalDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangReciprocalDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "hreflang-reciprocal-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.hreflang.len() >= 2 {
            for h in &ctx.page.meta.hreflang {
                let has_return = ctx
                    .page
                    .meta
                    .hreflang
                    .iter()
                    .any(|other| other.lang == h.lang && other.url != h.url);
                if !has_return {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "HREFRECIP-V2001".to_string(),
                        title: "Missing reciprocal hreflang (deep-deep-deep)".to_string(),
                        description: format!(
                            "No return hreflang found for '{}' in deep analysis.",
                            h.lang
                        ),
                        url: url.clone(),
                        recommendation: "Ensure all hreflang tags have reciprocal links."
                            .to_string(),
                    });
                    break;
                }
            }
        }
        findings
    }
}

pub struct HreflangXDefaultMissingDeepDeepValidator;
impl Default for HreflangXDefaultMissingDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangXDefaultMissingDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangXDefaultMissingDeepDeepValidator {
    fn name(&self) -> &str {
        "hreflang-x-default-missing-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.meta.hreflang.is_empty() {
            let has_x_default = ctx
                .page
                .meta
                .hreflang
                .iter()
                .any(|h| h.lang.to_lowercase().contains("x-default"));
            if !has_x_default {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "HREFXD-V2001".to_string(),
                    title: "Missing hreflang x-default (deep-deep)".to_string(),
                    description: "No x-default hreflang found in deep analysis.".to_string(),
                    url: url.clone(),
                    recommendation: "Add an x-default hreflang tag for the fallback page."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct SitemapMissingDeepDeepValidator;
impl Default for SitemapMissingDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapMissingDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapMissingDeepDeepValidator {
    fn name(&self) -> &str {
        "sitemap-missing-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let has_sitemap_ref =
                lower.contains("sitemap.xml") || lower.contains("sitemap_index.xml");
            if !has_sitemap_ref && ctx.page.meta.canonical.is_some() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "SITEMAPMISS-V2001".to_string(),
                    title: "No sitemap reference (deep-deep)".to_string(),
                    description: "Page does not reference a sitemap in deep analysis.".to_string(),
                    url: url.clone(),
                    recommendation: "Add sitemap reference or ensure sitemap.xml exists at root."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct RobotsTxtEmptyDeepDeepValidator;
impl Default for RobotsTxtEmptyDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtEmptyDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtEmptyDeepDeepValidator {
    fn name(&self) -> &str {
        "robots-txt-empty-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            if robots.trim().is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "ROBOTSEMPTY-V2001".to_string(),
                    title: "Empty robots.txt (deep-deep)".to_string(),
                    description: "robots.txt is present but empty in deep analysis.".to_string(),
                    url: url.clone(),
                    recommendation: "Add basic rules to robots.txt.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct InternalLinksDiversityDeepDeepValidator;
impl Default for InternalLinksDiversityDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinksDiversityDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalLinksDiversityDeepDeepValidator {
    fn name(&self) -> &str {
        "internal-links-diversity-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.is_external)
            .map(|l| l.href.as_str())
            .collect();
        if internal.len() > 5 {
            let unique: std::collections::HashSet<&str> = internal.iter().copied().collect();
            let ratio = unique.len() as f64 / internal.len() as f64;
            if ratio < 0.5 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "INTDIV-V2001".to_string(),
                    title: "Low internal link diversity (deep-deep)".to_string(),
                    description: format!(
                        "{}/{} ({:.0}%) internal links are unique in deep analysis.",
                        unique.len(),
                        internal.len(),
                        ratio * 100.0
                    ),
                    url: url.clone(),
                    recommendation: "Increase the diversity of internal link targets.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct InternalLinksDepthDeepDeepValidator;
impl Default for InternalLinksDepthDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinksDepthDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalLinksDepthDeepDeepValidator {
    fn name(&self) -> &str {
        "internal-links-depth-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.is_external)
            .map(|l| l.href.as_str())
            .collect();
        if internal.len() > 5 {
            let base = url::Url::parse(url).ok();
            if let Some(base) = base {
                let base_host = base.host_str().unwrap_or("");
                let deep_count = internal
                    .iter()
                    .filter_map(|href| {
                        url::Url::parse(href).ok().and_then(|u| {
                            if u.host_str() == Some(base_host) {
                                let depth = u
                                    .path()
                                    .trim_matches('/')
                                    .split('/')
                                    .filter(|s| !s.is_empty())
                                    .count();
                                if depth > 4 {
                                    Some(depth)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                    })
                    .count();
                if deep_count > internal.len() / 4 {
                    findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "INTDEPTH-V2001".to_string(), title: "Many deeply nested internal links (deep-deep)".to_string(), description: format!("{}/{} internal links point to URLs with 4+ path segments in deep analysis.", deep_count, internal.len()), url: url.clone(), recommendation: "Flatten URL structure for important pages.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct MetaDescriptionLengthDeepValidator;
impl Default for MetaDescriptionLengthDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionLengthDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MetaDescriptionLengthDeepValidator {
    fn name(&self) -> &str {
        "meta-description-length-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(desc) = &ctx.page.meta.description {
            let len = desc.len();
            if len > 160 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "METALEN-V2001".to_string(),
                    title: "Meta description too long (deep)".to_string(),
                    description: format!(
                        "Description is {len} chars, recommended 120-160 in deep analysis."
                    ),
                    url: url.clone(),
                    recommendation: "Shorten meta description to 120-160 characters.".to_string(),
                });
            } else if len < 70 && len > 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "METALEN-V2002".to_string(),
                    title: "Meta description too short (deep)".to_string(),
                    description: format!(
                        "Description is {len} chars, recommended 120-160 in deep analysis."
                    ),
                    url: url.clone(),
                    recommendation: "Expand meta description to 120-160 characters.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TitleLengthDeepValidator;
impl Default for TitleLengthDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleLengthDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TitleLengthDeepValidator {
    fn name(&self) -> &str {
        "title-length-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            let len = title.len();
            if len > 60 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "TITLELEN-V2001".to_string(),
                    title: "Title too long (deep)".to_string(),
                    description: format!(
                        "Title is {len} chars, recommended 30-60 in deep analysis."
                    ),
                    url: url.clone(),
                    recommendation: "Shorten title to 30-60 characters.".to_string(),
                });
            } else if len < 20 && len > 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "TITLELEN-V2002".to_string(),
                    title: "Title too short (deep)".to_string(),
                    description: format!(
                        "Title is {len} chars, recommended 30-60 in deep analysis."
                    ),
                    url: url.clone(),
                    recommendation: "Expand title to 30-60 characters.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SitemapCoverageDeepDeepValidator;
impl Default for SitemapCoverageDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SitemapCoverageDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SitemapCoverageDeepDeepValidator {
    fn name(&self) -> &str {
        "sitemap-coverage-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let lower = robots.to_lowercase();
            let has_sitemap = lower.contains("sitemap:");
            if !has_sitemap {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "SITEMAPDEEP-V2001".to_string(),
                    title: "No sitemap in robots.txt (deep-deep)".to_string(),
                    description: "robots.txt does not reference a sitemap in deep analysis."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add Sitemap: directive to robots.txt.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct RobotsTxtAnalysisDeepDeepValidator;
impl Default for RobotsTxtAnalysisDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtAnalysisDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RobotsTxtAnalysisDeepDeepValidator {
    fn name(&self) -> &str {
        "robots-txt-analysis-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let lower = robots.to_lowercase();
            if !lower.contains("user-agent:") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "ROBOTSDEEP-V2001".to_string(),
                    title: "Missing User-agent in robots.txt (deep-deep)".to_string(),
                    description:
                        "robots.txt does not contain User-agent directive in deep analysis."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add User-agent directive to robots.txt.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct InternalLinkQualityDeepValidator;
impl Default for InternalLinkQualityDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinkQualityDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InternalLinkQualityDeepValidator {
    fn name(&self) -> &str {
        "internal-link-quality-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<&crate::parser::ExtractedLink> =
            ctx.page.links.iter().filter(|l| !l.is_external).collect();
        if internal.len() > 3 {
            let nofollow_count = internal
                .iter()
                .filter(|l| l.rel.iter().any(|r| r == "nofollow"))
                .count();
            if nofollow_count as f64 / internal.len() as f64 > 0.5 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "INTLINKQ-V2001".to_string(),
                    title: "High nofollow internal link ratio (deep)".to_string(),
                    description: format!(
                        "{nofollow_count}/{} internal links are nofollowed in deep analysis.",
                        internal.len()
                    ),
                    url: url.clone(),
                    recommendation: "Review nofollow usage on internal links.".to_string(),
                });
            }
            let empty_text = internal
                .iter()
                // Image links are accessible via their img alt text; only
                // links missing text, aria-label, AND img alt are gaps.
                .filter(|l| {
                    l.text.trim().is_empty()
                        && l.aria_label.is_none()
                        && l.img_alt.as_ref().map_or(true, |a| a.trim().is_empty())
                })
                .count();
            if empty_text > 0 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "INTLINKQ-V2002".to_string(), title: "Internal links without anchor text (deep)".to_string(), description: format!("{empty_text} internal link(s) have no text or aria-label in deep analysis."), url: url.clone(), recommendation: "Add descriptive text to all internal links.".to_string() });
            }
        }
        findings
    }
}

pub struct ExternalLinkAuthorityDeepDeepValidator;
impl Default for ExternalLinkAuthorityDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ExternalLinkAuthorityDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ExternalLinkAuthorityDeepDeepValidator {
    fn name(&self) -> &str {
        "external-link-authority-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let external: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| l.is_external)
            .map(|l| l.href.as_str())
            .collect();
        if external.len() > 5 {
            let low_authority_tlds = [
                ".ru", ".cn", ".tk", ".ml", ".xyz", ".top", ".buzz", ".click",
            ];
            let suspicious = external
                .iter()
                .filter(|href| low_authority_tlds.iter().any(|tld| href.contains(tld)))
                .count();
            if suspicious > external.len() / 3 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "EXTLINKAUTH-V2001".to_string(),
                    title: "Many low-authority external links (deep-deep)".to_string(),
                    description: format!(
                        "{}/{} external links to low-authority TLDs in deep analysis.",
                        suspicious,
                        external.len()
                    ),
                    url: url.clone(),
                    recommendation: "Link to more authoritative external sources.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HreflangLocaleFormatDeepDeepValidator;
impl Default for HreflangLocaleFormatDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangLocaleFormatDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HreflangLocaleFormatDeepDeepValidator {
    fn name(&self) -> &str {
        "hreflang-locale-format-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for h in &ctx.page.meta.hreflang {
            let locale = &h.lang;
            if locale != "x-default" {
                let lang_parts: Vec<&str> = locale.split('-').collect();
                if lang_parts.is_empty() || lang_parts[0].len() < 2 || lang_parts[0].len() > 3 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "HREFFMT-V2001".to_string(),
                        title: "Invalid hreflang locale format (deep-deep)".to_string(),
                        description: format!(
                            "Locale '{locale}' has unexpected format in deep analysis."
                        ),
                        url: url.clone(),
                        recommendation: "Use ISO 639-1 language codes (e.g., 'en', 'en-US')."
                            .to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct CanonicalDepthDeepDeepValidator;
impl Default for CanonicalDepthDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalDepthDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CanonicalDepthDeepDeepValidator {
    fn name(&self) -> &str {
        "canonical-depth-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            if let Ok(parsed) = url::Url::parse(canonical.as_str()) {
                let path_depth = parsed
                    .path()
                    .trim_matches('/')
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .count();
                if path_depth > 5 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "CANDEEP-V2001".to_string(),
                        title: "Canonical URL very deep (deep-deep)".to_string(),
                        description: format!(
                            "Canonical path has {path_depth} segments in deep analysis."
                        ),
                        url: url.clone(),
                        recommendation: "Consider using a shorter canonical URL.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

// =========================================================================
// V8 Accessibility Validators (20)
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::StructuredData;

    fn make_page(url: &str) -> crate::parser::ParsedPage {
        crate::parser::ParsedPage {
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

    fn make_ctx<'a>(
        page: &'a crate::parser::ParsedPage,
        body: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    #[test]
    fn test_title_deep_v2() {
        assert!(TitleAnalysisDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_meta_deep_v2() {
        assert!(MetaDescriptionDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_canonical_deep_v2() {
        assert!(CanonicalValidationDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_sitemap_deep_v2() {
        assert!(SitemapCoverageDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_robots_deep_v2() {
        assert!(RobotsTxtAnalysisDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_int_link_v2() {
        assert!(InternalLinkQualityAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_ext_link_v2() {
        assert!(ExternalLinkAuthorityDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_title_len_v2() {
        assert!(TitleLengthQualityAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_meta_desc_v2() {
        assert!(MetaDescriptionQualityAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_anchor_v3() {
        assert!(InternalLinkAnchorAnalyzerV3::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_wiki_v3() {
        assert!(WikipediaLinkAnalyzerV3::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_pagination_v2() {
        assert!(PaginationDepthAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_mixed_proto_v2() {
        assert!(MixedProtocolRedirectValidatorV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_nofollow_v2() {
        assert!(InternalNofollowOveruseValidatorV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_sitemap_size_v2() {
        assert!(SitemapXmlSizeValidatorV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_robots_size_v2() {
        assert!(RobotsTxtSizeValidatorV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_hreflang_self_v2() {
        assert!(HreflangSelfReferenceValidatorV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_opensearch_v2() {
        assert!(OpenSearchDescriptionValidatorV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_canonical_depth_v2() {
        assert!(CanonicalDepthAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_meta_len_v3() {
        let f = MetaDescriptionLengthAnalyzerV3::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "META-V3001");
    }
    #[test]
    fn test_title_v4() {
        let f = TitleAnalyzerV4::new().analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "TITLE-V4001");
    }
    #[test]
    fn test_canonical_v3() {
        let f = CanonicalUrlAnalyzerV3::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "CAN-V3001");
    }
    #[test]
    fn test_hreflang_v4() {
        assert!(HreflangValidatorV4::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_sitemap_v3() {
        let f =
            SitemapAnalyzerV3::new().analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 1);
    }
    #[test]
    fn test_robots_v3() {
        let f =
            RobotsTxtAnalyzerV3::new().analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 1);
    }

    // ===== Content V5 Tests =====
    #[test]
    fn test_title_keyword_v5() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Amazing Widgets Online".into());
        p.meta.description = Some("Buying amazing widgets online today".into());
        assert!(TitleKeywordPresenceValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_brand_v5() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Best Products".into());
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Organization".into()),
            data: serde_json::json!({"name": "Acme Corp"}),
        }];
        let f = TitleBrandValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_title_length_v5_missing() {
        let f = TitleLengthValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f[0].code, "TITLELEN-V5001");
    }
    #[test]
    fn test_title_length_v5_short() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Hi".into());
        let f = TitleLengthValidatorV5::new().analyze(&make_ctx(&p, None));
        assert_eq!(f[0].code, "TITLELEN-V5002");
    }
    #[test]
    fn test_title_length_v5_ok() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("This is a perfectly normal length title".into());
        assert!(TitleLengthValidatorV5::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_meta_keyword_v5() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Amazing Widgets".into());
        p.meta.description = Some("Buy the best gadgets and gizmos today".into());
        let f = MetaDescriptionKeywordValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_meta_unique_v5() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Hello World".into());
        p.meta.description = Some("Hello World".into());
        let f = MetaDescriptionUniqueValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_meta_unique_v5_different() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Hello World".into());
        p.meta.description = Some("A completely different description about things".into());
        assert!(MetaDescriptionUniqueValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_canonical_self_v5_diff() {
        let mut p = make_page("https://example.com/page");
        p.meta.canonical = Some(url::Url::parse("https://other.com/different-page").unwrap());
        let f = CanonicalSelfReferenceValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_canonical_self_v5_same() {
        let mut p = make_page("https://example.com/page");
        p.meta.canonical = Some(url::Url::parse("https://example.com/page").unwrap());
        assert!(CanonicalSelfReferenceValidatorV5::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_canonical_chain_v5() {
        let mut p = make_page("https://example.com");
        p.meta.canonical = Some(url::Url::parse("https://other.com").unwrap());
        let f = CanonicalChainValidatorV5::new().analyze(&make_ctx(
            &p,
            Some("<html><head><link rel=\"canonical\" href=\"https://other.com\"></head></html>"),
        ));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_canonical_depth_v5() {
        let mut p = make_page("https://example.com");
        p.meta.canonical = Some(url::Url::parse("https://example.com/a/b/c/d/e/f").unwrap());
        let f = CanonicalDepthValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hreflang_reciprocal_v5() {
        let mut p = make_page("https://example.com/page");
        p.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "fr".into(),
            url: url::Url::parse("https://example.com/page/fr").unwrap(),
        }];
        let f = HreflangReciprocalValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hreflang_xd_v5() {
        let mut p = make_page("https://example.com/page");
        p.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".into(),
            url: url::Url::parse("https://example.com/page/en").unwrap(),
        }];
        let f = HreflangXDefaultValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hreflang_locale_v5() {
        let mut p = make_page("https://example.com/page");
        p.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "english".into(),
            url: url::Url::parse("https://example.com/page/en").unwrap(),
        }];
        let f = HreflangLocaleFormatValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_sitemap_coverage_v5() {
        let robots = "User-agent: *\nDisallow: /admin";
        let p = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some(robots),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = SitemapCoverageValidatorV5::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_sitemap_lastmod_v5() {
        let body = "<urlset><url><loc>https://example.com</loc></url></urlset>";
        let f = SitemapLastmodValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), Some(body)));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_sitemap_priority_v5() {
        let body = "<urlset><url><loc>https://example.com</loc></url></urlset>";
        let f = SitemapPriorityValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), Some(body)));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_robots_disallow_v5() {
        let robots = "User-agent: *\nDisallow: /";
        let p = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some(robots),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = RobotsTxtDisallowValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }

    // ===== Accessibility V5 Tests =====
    #[test]
    fn test_title_missing() {
        let f = TitleMissingValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TITLEMISS-V6081");
    }
    #[test]
    fn test_title_present() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Good Title".into());
        assert!(TitleMissingValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_keyword_density() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Amazing Widgets Online".into());
        p.meta.description = Some("Amazing widgets available online today".into());
        assert!(TitleKeywordDensityValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_brand() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Best Products by Acme Corp".into());
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Organization".into()),
            data: serde_json::json!({"name": "Acme Corp"}),
        }];
        assert!(TitleBrandPlacementValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_pixel_width() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Short Title".into());
        assert!(TitlePixelWidthValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_pixel_width_long() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("A".repeat(100).into());
        let f = TitlePixelWidthValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_meta_desc_missing() {
        let f = MetaDescriptionMissingValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_meta_desc_short() {
        let mut p = make_page("https://example.com");
        p.meta.description = Some("Short".into());
        let f = MetaDescriptionTooShortValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_meta_desc_unique() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Hello World".into());
        p.meta.description = Some("Hello World".into());
        let f = MetaDescriptionUniquenessValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_canonical_missing() {
        let f = CanonicalMissingValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_canonical_self_ref() {
        let mut p = make_page("https://example.com/page");
        p.meta.canonical = Some(url::Url::parse("https://other.com/different").unwrap());
        let f = CanonicalSelfReferenceDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_canonical_self_ok() {
        let mut p = make_page("https://example.com/page");
        p.meta.canonical = Some(url::Url::parse("https://example.com/page").unwrap());
        assert!(CanonicalSelfReferenceDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_canonical_chain() {
        let f = CanonicalChainDeepValidator::new().analyze(&make_ctx(&make_page("https://example.com"), Some("<html><head><link rel=\"canonical\" href=\"https://example.com\"><link rel=\"canonical\" href=\"https://example.com/other\"></head></html>")));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_canonical_depth() {
        let mut p = make_page("https://example.com");
        p.meta.canonical = Some(url::Url::parse("https://example.com/a/b/c/d/e/f").unwrap());
        let f = CanonicalDepthDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hreflang_missing() {
        let f = HreflangMissingValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HREFMISS-V6092");
    }
    #[test]
    fn test_hreflang_not_missing() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".into(),
            url: url::Url::parse("https://example.com/en").unwrap(),
        }];
        assert!(HreflangMissingValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_hreflang_reciprocal_dup() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "fr".into(),
                url: url::Url::parse("https://example.com/fr").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "fr".into(),
                url: url::Url::parse("https://example.com/fr2").unwrap(),
            },
        ];
        let f = HreflangReciprocalDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hreflang_xdefault() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".into(),
            url: url::Url::parse("https://example.com/en").unwrap(),
        }];
        let f = HreflangXDefaultMissingValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_sitemap_missing() {
        let p = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some("User-agent: *\nDisallow: /admin"),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = SitemapMissingValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_robots_txt_empty() {
        let p = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some(""),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = RobotsTxtEmptyValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_robots_txt_deep_disallow() {
        let p = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some("User-agent: *\nDisallow: /a/b/c/d/e/f"),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = RobotsTxtDisallowDepthValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_robots_txt_block_all() {
        let p = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some("User-agent: *\nDisallow: /"),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = RobotsTxtWildcardDisallowValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_robots_txt_no_ua() {
        let p = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some("Disallow: /admin"),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = RobotsTxtMissingUserAgentValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_int_links_diversity() {
        let p = make_page("https://example.com");
        assert!(InternalLinksDiversityValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_ext_links_nofollow() {
        let p = make_page("https://example.com");
        assert!(ExternalLinksNofollowAnalysisValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== V6 Accessibility Validators Tests =====
    #[test]
    fn test_title_missing_deep() {
        let f = TitleMissingDeepValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TITLEMISS001");
    }
    #[test]
    fn test_title_present_deep() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Good Title".into());
        assert!(TitleMissingDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_keyword_density_deep() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Amazing Widgets Online".into());
        p.meta.description = Some("Amazing widgets available online today".into());
        assert!(TitleKeywordDensityDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_keyword_density_low() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("XYZ Corp Products".into());
        p.meta.description = Some("Buy amazing things today".into());
        let f = TitleKeywordDensityDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TITLEKDEN001");
    }
    #[test]
    fn test_title_brand_deep_ok() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Acme Corp - Best Products".into());
        assert!(TitleBrandPlacementDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_brand_deep_short_prefix() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Co - Amazing Widgets That Are Really Great And Wonderful".into());
        let f = TitleBrandPlacementDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TITLEBRAND001");
    }
    #[test]
    fn test_meta_desc_missing_deep() {
        let f = MetaDescriptionMissingDeepValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "METADESCMISS001");
    }
    #[test]
    fn test_meta_desc_present_deep() {
        let mut p = make_page("https://example.com");
        p.meta.description = Some("A good description".into());
        assert!(MetaDescriptionMissingDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_meta_desc_short_deep() {
        let mut p = make_page("https://example.com");
        p.meta.description = Some("Short".into());
        let f = MetaDescriptionTooShortDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "METADESSSHORT001");
    }
    #[test]
    fn test_meta_desc_short_ok() {
        let mut p = make_page("https://example.com");
        p.meta.description = Some("This is a sufficiently long meta description that should pass the minimum length check for testing purposes.".into());
        assert!(MetaDescriptionTooShortDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_meta_desc_unique_deep() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Hello World".into());
        p.meta.description = Some("Hello World".into());
        let f = MetaDescriptionUniquenessDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "METADESCUNIQ001");
    }
    #[test]
    fn test_meta_desc_unique_ok() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Hello World".into());
        p.meta.description = Some("A comprehensive description of the page content.".into());
        assert!(MetaDescriptionUniquenessDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_canonical_missing_deep() {
        let f = CanonicalMissingDeepValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CANMISS001");
    }
    #[test]
    fn test_canonical_present_deep() {
        let mut p = make_page("https://example.com");
        p.meta.canonical = Some(url::Url::parse("https://example.com").unwrap());
        assert!(CanonicalMissingDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_canonical_self_ref_deep() {
        let mut p = make_page("https://example.com/page");
        p.meta.canonical = Some(url::Url::parse("https://example.com/other").unwrap());
        let f = CanonicalSelfReferenceDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CANSELFRF001");
    }
    #[test]
    fn test_canonical_self_ok_deep() {
        let mut p = make_page("https://example.com/page");
        p.meta.canonical = Some(url::Url::parse("https://example.com/page").unwrap());
        assert!(CanonicalSelfReferenceDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_canonical_chain_deep_deep() {
        let body = "<html><head><link rel=\"canonical\" href=\"https://example.com\"><link rel=\"canonical\" href=\"https://example.com/other\"></head></html>";
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: Some(body),
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CanonicalChainDeepDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CANCHAIN001");
    }
    #[test]
    fn test_canonical_chain_ok_deep() {
        let body =
            "<html><head><link rel=\"canonical\" href=\"https://example.com\"></head></html>";
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: Some(body),
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CanonicalChainDeepDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_hreflang_missing_deep() {
        let f = HreflangMissingDeepValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com/fr/page"), None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HREFMISS001");
    }
    #[test]
    fn test_hreflang_present_deep() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".into(),
            url: url::Url::parse("https://example.com/en").unwrap(),
        }];
        assert!(HreflangMissingDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_hreflang_reciprocal_dup_deep() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "fr".into(),
                url: url::Url::parse("https://example.com/fr").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "fr".into(),
                url: url::Url::parse("https://example.com/fr2").unwrap(),
            },
        ];
        let f = HreflangReciprocalDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hreflang_xdefault_missing_deep() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".into(),
            url: url::Url::parse("https://example.com/en").unwrap(),
        }];
        let f = HreflangXDefaultMissingDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HREFXD001");
    }
    #[test]
    fn test_hreflang_xdefault_ok_deep() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".into(),
                url: url::Url::parse("https://example.com/en").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "x-default".into(),
                url: url::Url::parse("https://example.com").unwrap(),
            },
        ];
        assert!(HreflangXDefaultMissingDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_sitemap_missing_deep() {
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some("User-agent: *\nDisallow: /admin"),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = SitemapMissingDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "SITEMAPMISS001");
    }
    #[test]
    fn test_sitemap_ok_deep() {
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some("User-agent: *\nSitemap: https://example.com/sitemap.xml"),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(SitemapMissingDeepValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_robots_txt_empty_deep() {
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some(""),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = RobotsTxtEmptyDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "ROBOTSEMPTY001");
    }
    #[test]
    fn test_robots_txt_ok_deep() {
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: Some("User-agent: *\nDisallow: /admin"),
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(RobotsTxtEmptyDeepValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_int_links_diversity_deep() {
        let p = make_page("https://example.com");
        assert!(InternalLinksDiversityDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_links_diversity_low() {
        let mut p = make_page("https://example.com");
        for _ in 0..10 {
            p.links.push(crate::parser::ExtractedLink {
                href: "https://example.com/same".into(),
                text: "same".into(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            });
        }
        let f = InternalLinksDiversityDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "INTDIV001");
    }

    // ===== DatasetMissingDescriptionValidator tests =====
    #[test]
    fn test_int_depth_no_links() {
        let p = make_page("https://example.com");
        assert!(InternalLinksDepthDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_depth_few_links() {
        let mut p = make_page("https://example.com");
        p.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/about".into(),
            text: "About".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        assert!(InternalLinksDepthDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_depth_shallow_links() {
        let mut p = make_page("https://example.com");
        p.links = (0..10)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        assert!(InternalLinksDepthDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_depth_deep_links() {
        let mut p = make_page("https://example.com");
        p.links = (0..12)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/a/b/c/d/e/f/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let f = InternalLinksDepthDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_int_depth_mixed_internal_external() {
        let mut p = make_page("https://example.com");
        let mut links: Vec<crate::parser::ExtractedLink> = (0..12)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/a/b/c/d/e/f/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        links.push(crate::parser::ExtractedLink {
            href: "https://other.com/deep/page".into(),
            text: "Ext".into(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        });
        p.links = links;
        let f = InternalLinksDepthDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_int_depth_no_structured_data() {
        let p = make_page("https://example.com");
        assert!(InternalLinksDepthDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_depth_multiple_findings() {
        let mut p = make_page("https://example.com");
        p.links = (0..20)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/a/b/c/d/e/f/g/h/i/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let f = InternalLinksDepthDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_int_depth_empty_links() {
        let mut p = make_page("https://example.com");
        p.links = vec![];
        assert!(InternalLinksDepthDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_depth_external_only() {
        let mut p = make_page("https://example.com");
        p.links = (0..10)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://other.com/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        assert!(InternalLinksDepthDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== ExternalLinksAuthorityScoreDeepValidator tests =====
    #[test]
    fn test_title_miss_dd_v8_none() {
        let p = make_page("https://example.com");
        let f = TitleMissingDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TITLEMISS-V2001");
    }
    #[test]
    fn test_title_miss_dd_v8_empty() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("".to_string());
        let f = TitleMissingDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_title_miss_dd_v8_present() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("My Title".to_string());
        assert!(TitleMissingDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_kden_dd_v8_no_overlap() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Amazing Super Widget".to_string());
        p.meta.description = Some("Completely different text here".to_string());
        let f = TitleKeywordDensityDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_title_kden_dd_v8_good_overlap() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Amazing Widget".to_string());
        p.meta.description = Some("Buy the amazing widget".to_string());
        assert!(TitleKeywordDensityDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_brand_dd_v8_ends_sep() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("My Page | ".to_string());
        let f = TitleBrandPlacementDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_title_brand_dd_v8_no_sep() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("My Page Brand".to_string());
        assert!(TitleBrandPlacementDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_meta_miss_dd_v8_none() {
        let p = make_page("https://example.com");
        let f = MetaDescriptionMissingDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "METADESCMISS-V2001");
    }
    #[test]
    fn test_meta_miss_dd_v8_present() {
        let mut p = make_page("https://example.com");
        p.meta.description = Some("A description".to_string());
        assert!(MetaDescriptionMissingDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_meta_short_dd_v8_short() {
        let mut p = make_page("https://example.com");
        p.meta.description = Some("Short".to_string());
        let f = MetaDescriptionTooShortDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_meta_short_dd_v8_ok() {
        let mut p = make_page("https://example.com");
        p.meta.description = Some("A good length description that is between 120 and 160 characters long for testing purposes.".to_string());
        assert!(MetaDescriptionTooShortDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_meta_uniq_dd_v8_same() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Same Title".to_string());
        p.meta.description = Some("Same Title".to_string());
        let f = MetaDescriptionUniquenessDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_meta_uniq_dd_v8_diff() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("Title".to_string());
        p.meta.description = Some("Different description".to_string());
        assert!(MetaDescriptionUniquenessDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_can_miss_dd_v8_none() {
        let p = make_page("https://example.com");
        let f = CanonicalMissingDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CANMISS-V2001");
    }
    #[test]
    fn test_can_miss_dd_v8_present() {
        let mut p = make_page("https://example.com");
        p.meta.canonical = Some("https://example.com/".to_string().parse().unwrap());
        assert!(CanonicalMissingDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_can_selfref_ddd_v8_self() {
        let mut p = make_page("https://example.com/page");
        p.meta.canonical = Some("https://example.com/page".parse().unwrap());
        assert!(CanonicalSelfReferenceDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_can_selfref_ddd_v8_diff() {
        let mut p = make_page("https://example.com/page");
        p.meta.canonical = Some("https://example.com/other".parse().unwrap());
        let f = CanonicalSelfReferenceDeepDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_can_chain_ddd_v8_one() {
        let p = make_page("https://example.com");
        assert!(CanonicalChainDeepDeepDeepValidator::new()
            .analyze(&make_ctx(
                &p,
                Some("<html><head><link rel='canonical' href='https://example.com'></head></html>")
            ))
            .is_empty());
    }
    #[test]
    fn test_can_chain_ddd_v8_multiple() {
        let p = make_page("https://example.com");
        let f = CanonicalChainDeepDeepDeepValidator::new().analyze(&make_ctx(&p, Some("<html><head><link rel='canonical' href='https://a.com'><link rel='canonical' href='https://b.com'></head></html>")));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_href_miss_dd_v8_empty() {
        let p = make_page("https://example.com");
        let f = HreflangMissingDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_href_miss_dd_v8_present() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".into(),
            url: "https://example.com/en".parse().unwrap(),
        }];
        assert!(HreflangMissingDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_sitemap_miss_dd_v8_no_ref() {
        let mut p = make_page("https://example.com/page");
        p.meta.canonical = Some("https://example.com/page".parse().unwrap());
        let f =
            SitemapMissingDeepDeepValidator::new().analyze(&make_ctx(&p, Some("<html></html>")));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_robots_empty_dd_v8_empty() {
        let p = make_page("https://example.com");
        let mut ctx = make_ctx(&p, None);
        ctx.robots_txt = Some("");
        let f = RobotsTxtEmptyDeepDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_robots_empty_dd_v8_content() {
        let p = make_page("https://example.com");
        let mut ctx = make_ctx(&p, None);
        ctx.robots_txt = Some("User-agent: *\nDisallow: /admin");
        assert!(RobotsTxtEmptyDeepDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_robots_empty_dd_v8_none() {
        let p = make_page("https://example.com");
        assert!(RobotsTxtEmptyDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_div_dd_v8_few() {
        let mut p = make_page("https://example.com");
        p.links = (0..3)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/page{i}"),
                text: format!("L{i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        assert!(InternalLinksDiversityDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_div_dd_v8_low() {
        let mut p = make_page("https://example.com");
        p.links = (0..10)
            .map(|_| crate::parser::ExtractedLink {
                href: "https://example.com/same".into(),
                text: "Same".into(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let f = InternalLinksDiversityDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_int_depth_dd_v8_few() {
        let mut p = make_page("https://example.com");
        p.links = (0..3)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/page{i}"),
                text: format!("L{i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        assert!(InternalLinksDepthDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_depth_dd_v8_deep() {
        let mut p = make_page("https://example.com");
        p.links = (0..12)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/a/b/c/d/e/f/page{i}"),
                text: format!("L{i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let f = InternalLinksDepthDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_meta_len_deep_v8_long() {
        let mut p = make_page("https://example.com");
        p.meta.description = Some("A".repeat(200));
        let f = MetaDescriptionLengthDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "METALEN-V2001");
    }
    #[test]
    fn test_meta_len_deep_v8_ok() {
        let mut p = make_page("https://example.com");
        p.meta.description = Some("A good length meta description that falls within the recommended range of 120 to 160 characters.".to_string());
        assert!(MetaDescriptionLengthDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_title_len_deep_v8_long() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("A".repeat(80));
        let f = TitleLengthDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TITLELEN-V2001");
    }
    #[test]
    fn test_title_len_deep_v8_ok() {
        let mut p = make_page("https://example.com");
        p.meta.title = Some("A Good Title That Is Of Length".to_string());
        assert!(TitleLengthDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_sitemap_deep_dd_v8_no_robots() {
        let p = make_page("https://example.com");
        assert!(SitemapCoverageDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_sitemap_deep_dd_v8_no_sitemap_in_robots() {
        let p = make_page("https://example.com");
        let mut ctx = make_ctx(&p, None);
        ctx.robots_txt = Some("User-agent: *\nDisallow: /admin");
        let f = SitemapCoverageDeepDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_sitemap_deep_dd_v8_sitemap_in_robots() {
        let p = make_page("https://example.com");
        let mut ctx = make_ctx(&p, None);
        ctx.robots_txt = Some("User-agent: *\nSitemap: https://example.com/sitemap.xml");
        assert!(SitemapCoverageDeepDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_robots_deep_dd_v8_no_ua() {
        let p = make_page("https://example.com");
        let mut ctx = make_ctx(&p, None);
        ctx.robots_txt = Some("Disallow: /admin");
        let f = RobotsTxtAnalysisDeepDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_robots_deep_dd_v8_has_ua() {
        let p = make_page("https://example.com");
        let mut ctx = make_ctx(&p, None);
        ctx.robots_txt = Some("User-agent: *\nDisallow: /admin");
        assert!(RobotsTxtAnalysisDeepDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_int_lq_deep_v8_few() {
        let mut p = make_page("https://example.com");
        p.links = (0..3)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/page{i}"),
                text: format!("L{i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        assert!(InternalLinkQualityDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_int_lq_deep_v8_nofollow() {
        let mut p = make_page("https://example.com");
        p.links = (0..6)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/page{i}"),
                text: format!("L{i}"),
                rel: vec!["nofollow".into()],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let f = InternalLinkQualityDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_ext_auth_dd_v8_few() {
        let mut p = make_page("https://example.com");
        p.links = vec![crate::parser::ExtractedLink {
            href: "https://other.com/page".into(),
            text: "L".into(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        assert!(ExternalLinkAuthorityDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_ext_auth_dd_v8_suspicious() {
        let mut p = make_page("https://example.com");
        p.links = (0..8)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://link{i}.ru/page"),
                text: format!("L{i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let f = ExternalLinkAuthorityDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_href_fmt_dd_v8_valid() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".into(),
                url: "https://example.com/en".parse().unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "fr".into(),
                url: "https://example.com/fr".parse().unwrap(),
            },
        ];
        assert!(HreflangLocaleFormatDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_href_fmt_dd_v8_invalid() {
        let mut p = make_page("https://example.com");
        p.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "english".into(),
            url: "https://example.com".parse().unwrap(),
        }];
        let f = HreflangLocaleFormatDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_can_depth_dd_v8_shallow() {
        let mut p = make_page("https://example.com");
        p.meta.canonical = Some("https://example.com/page".parse().unwrap());
        assert!(CanonicalDepthDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== V8 Accessibility Validators (20) tests =====
}
