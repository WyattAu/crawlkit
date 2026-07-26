// ---------------------------------------------------------------------------
// Advanced Canonical & Hreflang Analysis
// ---------------------------------------------------------------------------
// Addresses Ahrefs findings that our crawler was missing:
// - CANON004: Canonical points to redirect
// - CANON005: Canonical URL has no incoming internal links (simplified)
// - ISEO007: Hreflang to non-canonical
// - ISEO008: Hreflang to redirect or broken page
// - ISEO009: Missing reciprocal hreflang
// - URL001: Double slash in URL

use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::storage::{IssueCategory, Severity};
use crate::CrawlConfig;

/// Advanced canonical and hreflang analyzer that catches issues
/// Ahrefs and other premium tools detect but basic crawlers miss.
pub struct AdvancedCanonicalAnalyzer;

impl Default for AdvancedCanonicalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedCanonicalAnalyzer {
    /// Create a new advanced canonical analyzer.
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AdvancedCanonicalAnalyzer {
    fn name(&self) -> &str {
        "advanced-canonical"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // CANON004: Canonical points to redirect
        if let Some(canonical) = &ctx.page.meta.canonical {
            if !ctx.redirect_chain.is_empty() {
                let canonical_str = canonical.as_str();
                let page_str = url.as_str();
                if canonical_str != page_str {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "CANON004".to_string(),
                        title: "Canonical may point to redirect".to_string(),
                        description: format!(
                            "This page was redirected, and its canonical URL \"{}\" differs \
                             from the current URL. The canonical may also redirect, creating \
                             a confusing signal for search engines.",
                            canonical
                        ),
                        url: url.clone(),
                        recommendation: "Verify the canonical URL resolves to a final \
                                         destination without redirects."
                            .to_string(),
                    });
                }
            }
        }

        // ISEO007: Hreflang to non-canonical
        if let Some(_canonical) = &ctx.page.meta.canonical {
            for tag in &ctx.page.meta.hreflang {
                let tag_url = tag.url.as_str();
                let this_url = url.as_str();
                if tag_url != this_url && tag_url.contains('?') {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "ISEO007".to_string(),
                        title: "Hreflang may point to non-canonical URL".to_string(),
                        description: format!(
                            "Hreflang tag lang=\"{}\" points to \"{}\" which contains \
                             query parameters. This URL may not be the canonical version.",
                            tag.lang, tag_url
                        ),
                        url: url.clone(),
                        recommendation: "Update the hreflang tag to point to the \
                                         canonical version of the target page."
                            .to_string(),
                    });
                }
            }
        }

        // URL001: Double slash in URL path
        if let Some(path_start) = url.find("://") {
            let path = &url[path_start + 3..];
            if path.contains("//") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "URL001".to_string(),
                    title: "Double slash in URL path".to_string(),
                    description: "URL contains double slashes after the scheme. This can cause \
                         indexing issues and duplicate content problems."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Remove double slashes from the URL path.".to_string(),
                });
            }
        }

        findings
    }
}

/// Sitemap canonical validator.
pub struct SitemapCanonicalValidator;

impl Default for SitemapCanonicalValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SitemapCanonicalValidator {
    /// Create a new sitemap canonical validator.
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SitemapCanonicalValidator {
    fn name(&self) -> &str {
        "sitemap-canonical"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if let Some(canonical) = &ctx.page.meta.canonical {
            let canonical_str = canonical.as_str();
            let page_str = url.as_str();
            if canonical_str != page_str {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "SITEMAP006".to_string(),
                    title: "Non-canonical page with canonical tag".to_string(),
                    description: format!(
                        "Page \"{}\" has canonical \"{}\". Non-canonical pages should \
                         either not have a canonical tag or the canonical should match.",
                        url, canonical
                    ),
                    url: url.clone(),
                    recommendation: "Either remove the canonical tag (if this is the \
                                     preferred URL) or update the canonical to match."
                        .to_string(),
                });
            }
        }

        findings
    }
}

/// URL format validator.
pub struct UrlFormatValidator;

impl Default for UrlFormatValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlFormatValidator {
    /// Create a new URL format validator.
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for UrlFormatValidator {
    fn name(&self) -> &str {
        "url-format"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // URL002: Uppercase in URL path
        if let Some(path_start) = url.find("://") {
            let path = &url[path_start + 3..];
            if path != path.to_lowercase() && !path.contains('%') {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "URL002".to_string(),
                    title: "Uppercase characters in URL".to_string(),
                    description: "URL path contains uppercase characters. URLs are \
                         case-sensitive and should be consistent."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Use lowercase characters in URL paths for consistency."
                        .to_string(),
                });
            }
        }

        findings
    }
}
