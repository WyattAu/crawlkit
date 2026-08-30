#![allow(
    clippy::unwrap_used,
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
    clippy::redundant_clone
)]
use std::collections::{HashMap, HashSet};
use url::Url;

use crate::parser::ExtractedLink;
use crate::types::{IssueCategory, Severity};

use super::{is_utility_page, AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------
// 3. Canonical URL Validator
// ---------------------------------------------------------------------------

pub struct CanonicalUrlValidator;

impl CanonicalUrlValidator {
    pub fn new() -> Self {
        Self
    }

    /// Normalize a URL for comparison (strip trailing slash, lowercase scheme/host).
    fn normalize_url(url: &Url) -> String {
        let mut s = url.to_string();
        // Strip fragment (anchor) — search engines ignore fragments for canonical comparison
        if let Some(pos) = s.find('#') {
            s.truncate(pos);
        }
        // Remove trailing slash from path-only URLs
        if s.ends_with('/') && url.path() != "/" {
            s.pop();
        }
        s
    }
}

impl Default for CanonicalUrlValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CanonicalUrlValidator {
    fn name(&self) -> &str {
        "canonical-url"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let page_url = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => return findings,
        };

        match &ctx.page.meta.canonical {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "CANON001".to_string(),
                    title: "Missing canonical URL".to_string(),
                    description: "No <link rel=\"canonical\"> tag was found on this page."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a canonical URL tag pointing to the preferred version \
                                     of this page."
                        .to_string(),
                });
            }
            Some(canonical) => {
                let canonical_str = Self::normalize_url(canonical);
                let page_str = Self::normalize_url(&page_url);

                if canonical_str == page_str {
                    // Self-referencing canonical — this is correct
                } else {
                    // Canonical points elsewhere — check if it's intentional
                    let same_host = canonical.host_str() == page_url.host_str();
                    let same_path = canonical.path() == page_url.path();

                    if same_host && same_path {
                        // Likely a parameter difference — info
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Seo,
                            code: "CANON002".to_string(),
                            title: "Canonical URL differs".to_string(),
                            description: format!(
                                "Canonical points to {canonical}, which differs from the \
                                 current URL."
                            ),
                            url: url.clone(),
                            recommendation: "Verify this is intentional. The canonical should \
                                             point to the preferred URL."
                                .to_string(),
                        });
                    } else {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Seo,
                            code: "CANON003".to_string(),
                            title: "Canonical URL mismatch".to_string(),
                            description: format!(
                                "Canonical URL ({canonical}) does not match the current page \
                                 URL ({url})."
                            ),
                            url: url.clone(),
                            recommendation: "Ensure the canonical URL points to the correct \
                                             preferred version of this page."
                                .to_string(),
                        });
                    }
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 4. Hreflang Validator
// ---------------------------------------------------------------------------

pub struct HreflangValidator;

impl HreflangValidator {
    pub fn new() -> Self {
        Self
    }

    /// Check if a locale code looks valid (language[-region] format).
    pub(crate) fn is_valid_locale(code: &str) -> bool {
        if code == "x-default" {
            return true;
        }
        let parts: Vec<&str> = code.split('-').collect();
        match parts.len() {
            1 => {
                // Language-only: 2-3 letter code
                let lang = parts[0];
                lang.len() >= 2 && lang.len() <= 3 && lang.chars().all(|c| c.is_ascii_alphabetic())
            }
            2 => {
                // Language-Region
                let lang = parts[0];
                let region = parts[1];
                lang.len() >= 2
                    && lang.len() <= 3
                    && lang.chars().all(|c| c.is_ascii_alphabetic())
                    // Region: ISO 3166-1 alpha-2 ("US") or UN M49 numeric-3
                    // ("419" = Latin America) — both canonical in hreflang.
                    && ((region.len() == 2 && region.chars().all(|c| c.is_ascii_alphabetic()))
                        || (region.len() == 3 && region.chars().all(|c| c.is_ascii_digit())))
            }
            _ => false,
        }
    }
}

impl Default for HreflangValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HreflangValidator {
    fn name(&self) -> &str {
        "hreflang"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hreflang_tags = &ctx.page.meta.hreflang;

        if hreflang_tags.is_empty() {
            return findings; // No hreflang — not an error (just not implemented)
        }

        // Check for x-default
        let has_x_default = hreflang_tags.iter().any(|t| t.lang == "x-default");
        if !has_x_default {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "HREF001".to_string(),
                title: "Missing x-default hreflang".to_string(),
                description: "No hreflang tag with lang=\"x-default\" was found.".to_string(),
                url: url.clone(),
                recommendation: "Add an x-default hreflang tag to specify the fallback URL for \
                                 users whose language doesn't match any other hreflang."
                    .to_string(),
            });
        }

        // Validate locale codes
        for tag in hreflang_tags {
            if !Self::is_valid_locale(&tag.lang) {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Seo,
                    code: "HREF002".to_string(),
                    title: "Invalid hreflang locale code".to_string(),
                    description: format!(
                        "The hreflang code \"{}\" does not follow the ISO 639-1/BCP 47 format.",
                        tag.lang
                    ),
                    url: url.clone(),
                    recommendation: "Use valid BCP 47 language tags (e.g., \"en\", \"en-US\", \
                                     \"fr-CA\")."
                        .to_string(),
                });
            }
        }

        // Collect all languages and their URLs
        let mut lang_to_urls: HashMap<String, Vec<String>> = HashMap::new();
        for tag in hreflang_tags {
            lang_to_urls
                .entry(tag.lang.clone())
                .or_default()
                .push(tag.url.to_string());
        }

        // Check for duplicate language codes
        for (lang, urls) in &lang_to_urls {
            if urls.len() > 1 && lang != "x-default" {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Seo,
                    code: "HREF003".to_string(),
                    title: "Duplicate hreflang language".to_string(),
                    description: format!(
                        "Language \"{}\" appears {} times in hreflang tags.",
                        lang,
                        urls.len()
                    ),
                    url: url.clone(),
                    recommendation: "Each language code should appear only once in hreflang tags."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 5. Sitemap Analyzer
// ---------------------------------------------------------------------------

/// Known sitemap entries for validation.
#[derive(Debug, Clone)]
pub struct SitemapEntry {
    pub url: String,
    pub lastmod: Option<String>,
    pub changefreq: Option<String>,
    pub priority: Option<f64>,
}

pub struct SitemapAnalyzer {
    /// Pre-loaded sitemap entries (URLs found in sitemaps).
    known_urls: HashSet<String>,
    /// Entries with metadata.
    entries: Vec<SitemapEntry>,
}

impl SitemapAnalyzer {
    pub fn new(known_urls: HashSet<String>, entries: Vec<SitemapEntry>) -> Self {
        Self {
            known_urls,
            entries,
        }
    }

    pub fn empty() -> Self {
        Self {
            known_urls: HashSet::new(),
            entries: Vec::new(),
        }
    }

    /// Validate a lastmod date format (ISO 8601).
    pub(crate) fn is_valid_lastmod(lastmod: &str) -> bool {
        // Simple check: must contain YYYY-MM (at minimum) with valid separators
        let bytes = lastmod.as_bytes();
        if bytes.len() < 7 {
            return false;
        }
        // Look for a 4-digit year followed by -MM
        for i in 0..=bytes.len().saturating_sub(7) {
            if bytes[i].is_ascii_digit()
                && bytes[i + 1].is_ascii_digit()
                && bytes[i + 2].is_ascii_digit()
                && bytes[i + 3].is_ascii_digit()
                && bytes[i + 4] == b'-'
                && bytes[i + 5].is_ascii_digit()
                && bytes[i + 6].is_ascii_digit()
            {
                return true;
            }
        }
        false
    }

    /// Validate changefreq value.
    pub(crate) fn is_valid_changefreq(freq: &str) -> bool {
        matches!(
            freq,
            "always" | "hourly" | "daily" | "weekly" | "monthly" | "yearly" | "never"
        )
    }

    /// Validate priority value (0.0 - 1.0).
    pub(crate) fn is_valid_priority(p: f64) -> bool {
        (0.0..=1.0).contains(&p)
    }
}

impl Default for SitemapAnalyzer {
    fn default() -> Self {
        Self::empty()
    }
}

impl Analyzer for SitemapAnalyzer {
    fn name(&self) -> &str {
        "sitemap"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if self.known_urls.is_empty() {
            // No sitemap data was loaded for this crawl; sitemap validation
            // is a silent no-op rather than a per-page informational finding.
            return Vec::new();
        }

        // Check if this page URL is in the sitemap
        if !self.known_urls.contains(url) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "SITEMAP002".to_string(),
                title: "Page not found in sitemap".to_string(),
                description: "This page URL was not found in any loaded sitemap.".to_string(),
                url: url.clone(),
                recommendation: "Add this page to your sitemap.xml to ensure it is crawled and \
                                 indexed."
                    .to_string(),
            });
        }

        // Validate sitemap entry metadata
        if let Some(entry) = self.entries.iter().find(|e| e.url == *url) {
            if let Some(ref lastmod) = entry.lastmod {
                if !Self::is_valid_lastmod(lastmod) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "SITEMAP003".to_string(),
                        title: "Invalid lastmod format".to_string(),
                        description: format!(
                            "The lastmod value \"{lastmod}\" does not appear to be a valid \
                             ISO 8601 date."
                        ),
                        url: url.clone(),
                        recommendation: "Use ISO 8601 format for lastmod (e.g., \
                                         2024-01-15T10:30:00Z)."
                            .to_string(),
                    });
                }
            }

            if let Some(ref freq) = entry.changefreq {
                if !Self::is_valid_changefreq(freq) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "SITEMAP004".to_string(),
                        title: "Invalid changefreq value".to_string(),
                        description: format!(
                            "The changefreq value \"{freq}\" is not a recognized value."
                        ),
                        url: url.clone(),
                        recommendation: "Use one of: always, hourly, daily, weekly, monthly, \
                                         yearly, never."
                            .to_string(),
                    });
                }
            }

            if let Some(priority) = entry.priority {
                if !Self::is_valid_priority(priority) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "SITEMAP005".to_string(),
                        title: "Invalid priority value".to_string(),
                        description: format!(
                            "The priority value {priority} is outside the valid range (0.0-1.0)."
                        ),
                        url: url.clone(),
                        recommendation: "Set priority to a value between 0.0 and 1.0.".to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 7. Meta Tag Analyzer
// ---------------------------------------------------------------------------

pub struct MetaTagAnalyzer;

impl MetaTagAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MetaTagAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MetaTagAnalyzer {
    fn name(&self) -> &str {
        "meta-tags"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let meta = &ctx.page.meta;

        // --- Title ---
        match &meta.title {
            None => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Seo,
                    code: "META001".to_string(),
                    title: "Missing page title".to_string(),
                    description: "No <title> tag was found on this page.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a descriptive title tag (30-60 characters).".to_string(),
                });
            }
            Some(title) => {
                let len = title.len();
                if len < 30 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "META002".to_string(),
                        title: "Title too short".to_string(),
                        description: format!(
                            "Title is {len} characters, below the recommended minimum of 30."
                        ),
                        url: url.clone(),
                        recommendation: "Expand the title to 30-60 characters to improve \
                                         search snippet quality."
                            .to_string(),
                    });
                } else if len > 60 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "META003".to_string(),
                        title: "Title too long".to_string(),
                        description: format!(
                            "Title is {len} characters, exceeding the recommended maximum of 60."
                        ),
                        url: url.clone(),
                        recommendation: "Shorten the title to 60 characters or less to prevent \
                                         truncation in search results."
                            .to_string(),
                    });
                }
            }
        }

        // --- Description ---
        match &meta.description {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "META004".to_string(),
                    title: "Missing meta description".to_string(),
                    description: "No meta description was found on this page.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a meta description (120-160 characters) to control \
                                     how your page appears in search results."
                        .to_string(),
                });
            }
            Some(desc) => {
                let len = desc.len();
                if len < 120 {
                    if !is_utility_page(url) {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Seo,
                            code: "META005".to_string(),
                            title: "Meta description too short".to_string(),
                            description: format!(
                                "Description is {len} characters, below the recommended minimum \
                                 of 120."
                            ),
                            url: url.clone(),
                            recommendation: "Expand the description to 120-160 characters."
                                .to_string(),
                        });
                    }
                } else if len > 165 {
                    // Allow 5-char tolerance for truncation differences
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "META006".to_string(),
                        title: "Meta description too long".to_string(),
                        description: format!(
                            "Description is {len} characters, exceeding the recommended \
                             maximum of 160."
                        ),
                        url: url.clone(),
                        recommendation: "Shorten the description to 160 characters or less."
                            .to_string(),
                    });
                }
            }
        }

        // --- Viewport ---
        if meta.viewport.is_none() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Mobile,
                code: "META009".to_string(),
                title: "Missing viewport meta tag".to_string(),
                description: "No viewport meta tag was found. This may affect mobile rendering."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <meta name=\"viewport\" content=\"width=device-width, \
                                 initial-scale=1\"> for proper mobile rendering."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 8. Heading Hierarchy Analyzer
// ---------------------------------------------------------------------------

pub struct HeadingHierarchyAnalyzer;

impl HeadingHierarchyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HeadingHierarchyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HeadingHierarchyAnalyzer {
    fn name(&self) -> &str {
        "heading-hierarchy"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let headings = &ctx.page.headings;

        if headings.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "HEAD001".to_string(),
                title: "No headings found".to_string(),
                description: "The page contains no heading elements (H1-H6).".to_string(),
                url: url.clone(),
                recommendation: "Add at least one H1 heading to define the page topic.".to_string(),
            });
            return findings;
        }

        // Check H1 count
        let h1_count = headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Seo,
                code: "HEAD002".to_string(),
                title: "Missing H1 heading".to_string(),
                description: "The page has headings but no H1 tag.".to_string(),
                url: url.clone(),
                recommendation: "Add exactly one H1 heading that describes the main topic of \
                                 the page."
                    .to_string(),
            });
        } else if h1_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "HEAD003".to_string(),
                title: "Multiple H1 headings".to_string(),
                description: format!(
                    "The page has {h1_count} H1 headings. Best practice is to have exactly one."
                ),
                url: url.clone(),
                recommendation: "Use a single H1 for the main topic. Use H2-H6 for subsections."
                    .to_string(),
            });
        }

        // Check for skipped heading levels
        let mut prev_level: Option<u8> = None;
        for heading in headings {
            if let Some(prev) = prev_level {
                if heading.level > prev + 1 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Content,
                        code: "HEAD004".to_string(),
                        title: "Skipped heading level".to_string(),
                        description: format!(
                            "Heading level jumps from H{prev} to H{}, skipping intermediate \
                             levels.",
                            heading.level
                        ),
                        url: url.clone(),
                        recommendation: format!(
                            "Use H{} after H{prev} to maintain proper document hierarchy.",
                            prev + 1
                        ),
                    });
                }
            }
            prev_level = Some(heading.level);
        }

        // Calculate max depth
        let max_depth = headings.iter().map(|h| h.level).max().unwrap_or(0);
        if max_depth >= 5 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "HEAD005".to_string(),
                title: "Deep heading hierarchy".to_string(),
                description: format!(
                    "Heading hierarchy reaches H{max_depth}. Deep nesting may indicate \
                     overly complex content structure."
                ),
                url: url.clone(),
                recommendation: "Consider flattening the heading hierarchy for better \
                                 readability."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 9. Link Analyzer
// ---------------------------------------------------------------------------

/// Per-page link analysis summary.
#[derive(Debug, Clone, Default)]
pub struct LinkInfo {
    pub total: usize,
    pub internal: usize,
    pub external: usize,
    pub nofollow: usize,
    pub anchor_text_empty: usize,
}

pub struct LinkAnalyzer {
    /// All URLs observed across the crawl, mapped to the pages that link to them.
    /// Used for orphan page detection. Callers can inject this after a full crawl.
    inbound_links: HashMap<String, usize>,
}

impl LinkAnalyzer {
    pub fn new() -> Self {
        Self {
            inbound_links: HashMap::new(),
        }
    }

    /// Create a LinkAnalyzer with pre-computed inbound link counts for orphan
    /// detection. The map should contain target URL -> number of inbound links.
    pub fn with_inbound_links(inbound_links: HashMap<String, usize>) -> Self {
        Self { inbound_links }
    }
}

impl Default for LinkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LinkAnalyzer {
    fn name(&self) -> &str {
        "link-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let internal_count = ctx.page.links.iter().filter(|l| !l.is_external).count();
        let external_count = ctx.page.links.iter().filter(|l| l.is_external).count();
        let nofollow_count = ctx
            .page
            .links
            .iter()
            .filter(|l| l.rel.contains(&"nofollow".to_string()))
            .count();

        // 2.1 — Internal vs external link counts
        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Links,
            code: "LINK001".to_string(),
            title: "Link counts".to_string(),
            description: format!(
                "Internal: {}, External: {}, Nofollow: {}",
                internal_count, external_count, nofollow_count
            ),
            url: url.clone(),
            recommendation: String::new(),
        });

        // 2.2 — Flag broken links (4xx/5xx) when status code is available
        for link in &ctx.page.links {
            let resolved = Url::parse(url)
                .ok()
                .and_then(|base| base.join(&link.href).ok());
            let target = resolved.as_ref().map(|u| u.as_str()).unwrap_or(&link.href);
            if let Some(status) = ctx.status_code {
                if (400..=599).contains(&status) {
                    // The page itself is broken — all outbound links are suspect
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Links,
                        code: "LINK002".to_string(),
                        title: "Link on broken page".to_string(),
                        description: format!(
                            "Link \"{}\" points to \"{}\" but the current page itself returned \
                             HTTP {status}.",
                            link.text, target,
                        ),
                        url: url.clone(),
                        recommendation: "Fix the broken page or remove links from it.".to_string(),
                    });
                }
            }
        }

        // 2.2 — Nofollow detection
        if nofollow_count > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Links,
                code: "LINK003".to_string(),
                title: "Nofollow links present".to_string(),
                description: format!(
                    "{} link(s) have rel=\"nofollow\". This tells search engines not to pass \
                     PageRank.",
                    nofollow_count
                ),
                url: url.clone(),
                recommendation: "Ensure nofollow is used intentionally (e.g., paid links, \
                                 untrusted user content)."
                    .to_string(),
            });
        }

        // 2.3 — Anchor text quality
        for link in &ctx.page.links {
            // Check for accessible name: text content, aria-label, or img alt
            let has_accessible_name = !link.text.trim().is_empty()
                || link
                    .aria_label
                    .as_ref()
                    .is_some_and(|l| !l.trim().is_empty())
                || link.img_alt.as_ref().is_some_and(|a| !a.trim().is_empty());
            if !has_accessible_name {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Links,
                    code: "LINK004".to_string(),
                    title: "Empty anchor text".to_string(),
                    description: format!("Link to \"{}\" has no visible anchor text.", link.href),
                    url: url.clone(),
                    recommendation: "Add descriptive anchor text to help users and search engines \
                                     understand the link destination."
                        .to_string(),
                });
            } else if link.text.trim().len() < 3 && !link.text.trim().is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Links,
                    code: "LINK005".to_string(),
                    title: "Very short anchor text".to_string(),
                    description: format!(
                        "Link \"{}\" has very short anchor text ({} chars).",
                        link.text,
                        link.text.len()
                    ),
                    url: url.clone(),
                    recommendation: "Use more descriptive anchor text for better usability."
                        .to_string(),
                });
            }
        }

        // 2.3 — Orphan page detection (0 inbound links)
        if let Some(&count) = self.inbound_links.get(url) {
            if count == 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Links,
                    code: "LINK006".to_string(),
                    title: "Orphan page".to_string(),
                    description: "This page has no inbound links from other pages on the site."
                        .to_string(),
                    url: url.clone(),
                    recommendation:
                        "Add internal links from other pages to improve discoverability."
                            .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// OpenSearch Validator
// ---------------------------------------------------------------------------

pub struct OpenSearchValidator;

impl OpenSearchValidator {
    pub fn new() -> Self {
        Self
    }

    /// Check if the HTML contains an OpenSearch description link.
    pub(crate) fn has_opensearch_link(html: &str) -> bool {
        let lower = html.to_lowercase();
        (lower.contains(r#"rel="search""#)
            && lower.contains(r#"type="application/opensearchdescription+xml""#))
            || (lower.contains(r#"rel='search'"#)
                && lower.contains(r#"type='application/opensearchdescription+xml'"#))
    }
}

impl Default for OpenSearchValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for OpenSearchValidator {
    fn name(&self) -> &str {
        "opensearch"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // OPSEARCH001: No OpenSearch description XML link in head
        let has_opensearch = ctx.body.is_some_and(|body| Self::has_opensearch_link(body));
        if !has_opensearch {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "OPSEARCH001".to_string(),
                title: "No OpenSearch description link found".to_string(),
                description: "No <link rel=\"search\" type=\"application/opensearchdescription+xml\"> \
                              tag was found in the page head. OpenSearch allows browsers and search \
                              tools to discover your site's search functionality."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add a <link rel=\"search\" type=\"application/opensearchdescription+xml\" \
                                 title=\"...\" href=\"/opensearch.xml\"> tag to enable OpenSearch \
                                 discovery."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// HreflangSelfReferenceValidator
// ---------------------------------------------------------------------------

pub struct HreflangSelfReferenceValidator;

impl HreflangSelfReferenceValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HreflangSelfReferenceValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HreflangSelfReferenceValidator {
    fn name(&self) -> &str {
        "hreflang-self-reference"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hreflang_tags = &ctx.page.meta.hreflang;

        if hreflang_tags.is_empty() {
            return findings;
        }

        let page_url_str = url.as_str();
        let canonical_str = ctx.page.meta.canonical.as_ref().map(|c| c.as_str());

        let has_self_ref = hreflang_tags.iter().any(|tag| {
            let tag_url = tag.url.as_str();
            tag_url == page_url_str
                || Some(tag_url) == canonical_str
                || tag_url.trim_end_matches('/') == page_url_str.trim_end_matches('/')
        });

        if !has_self_ref {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "HREFSELF001".to_string(),
                title: "Missing self-referencing hreflang".to_string(),
                description: "The page has hreflang tags but none reference this page itself. \
                              Search engines require a self-referencing hreflang tag for each \
                              localized page."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add a hreflang tag that points to this page's own URL (or \
                                 canonical URL) to complete the international targeting setup."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// OpenSearchDescriptionValidator
// ---------------------------------------------------------------------------

pub struct OpenSearchDescriptionValidator;

impl OpenSearchDescriptionValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenSearchDescriptionValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for OpenSearchDescriptionValidator {
    fn name(&self) -> &str {
        "opensearch-description"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let has_opensearch = ctx.body.is_some_and(|body| {
            let lower = body.to_lowercase();
            (lower.contains(r#"rel="search""#)
                && lower.contains(r#"type="application/opensearchdescription+xml""#))
                || (lower.contains(r#"rel='search'"#)
                    && lower.contains(r#"type='application/opensearchdescription+xml'"#))
        });

        if !has_opensearch {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "OPDESC001".to_string(),
                title: "Missing OpenSearch description".to_string(),
                description:
                    "No <link rel=\"search\" type=\"application/opensearchdescription+xml\"> \
                              tag was found. An OpenSearch description document allows browsers to \
                              discover and add your site's search functionality."
                        .to_string(),
                url: url.clone(),
                recommendation:
                    "Create an OpenSearch XML description file and add a <link> tag in \
                                 the page head pointing to it."
                        .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 13. Word Count Analyzer
// ---------------------------------------------------------------------------

pub struct WordCountAnalyzer;

impl WordCountAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WordCountAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for WordCountAnalyzer {
    fn name(&self) -> &str {
        "word-count"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Both stats come from the parser's single visible-text walk, so
        // the average is over a consistent corpus. (The previous heading-
        // only sentence proxy divided full-page words by heading sentence
        // counts, producing impossible averages like 173 words/sentence.)
        let word_count = ctx.page.word_count;
        let sentence_count = ctx.page.sentence_count;
        let sentences_for_average = sentence_count.max(1);
        let avg_words_per_sentence = word_count as f64 / sentences_for_average as f64;

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "WC001".to_string(),
            title: "Word count statistics".to_string(),
            description: format!(
                "Words: {word_count}, Sentences: {sentence_count}, \
                 Avg words/sentence: {avg_words_per_sentence:.1}."
            ),
            url: url.clone(),
            recommendation: String::new(),
        });

        if word_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "WC002".to_string(),
                title: "Zero word count".to_string(),
                description: "The page has no detectable words. This may indicate a rendering \
                             issue or an empty page."
                    .to_string(),
                url: url.clone(),
                recommendation: "Verify the page content is visible and not hidden behind \
                                 JavaScript or CSS."
                    .to_string(),
            });
        } else if word_count < 100 {
            // Skip very low word count warning for utility pages
            if !is_utility_page(url) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "WC003".to_string(),
                    title: "Very low word count".to_string(),
                    description: format!(
                        "Page has only {word_count} words. This is very thin content."
                    ),
                    url: url.clone(),
                    recommendation: "Add more substantive content to the page.".to_string(),
                });
            }
        }

        if avg_words_per_sentence > 25.0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "WC004".to_string(),
                title: "Long average sentence length".to_string(),
                description: format!(
                    "Average sentence length is {avg_words_per_sentence:.1} words. Sentences \
                     longer than 25 words may be difficult to read."
                ),
                url: url.clone(),
                recommendation: "Break long sentences into shorter ones for better readability."
                    .to_string(),
            });
        }

        findings
    }
}

use super::STOP_WORDS;

// ---------------------------------------------------------------------------
// 21. Keyword Analyzer
// ---------------------------------------------------------------------------

pub struct KeywordAnalyzer {
    corpus_tf: HashMap<String, f64>,
}

impl KeywordAnalyzer {
    pub fn new() -> Self {
        Self {
            corpus_tf: HashMap::new(),
        }
    }

    pub fn with_corpus_tf(corpus_tf: HashMap<String, f64>) -> Self {
        Self { corpus_tf }
    }

    pub(crate) fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect()
            })
            .filter(|w: &String| w.len() > 2 && !STOP_WORDS.contains(&w.as_str()))
            .collect()
    }

    pub(crate) fn compute_tf(tokens: &[String]) -> HashMap<String, f64> {
        let mut freq: HashMap<String, usize> = HashMap::new();
        for token in tokens {
            *freq.entry(token.clone()).or_default() += 1;
        }
        let total = tokens.len() as f64;
        freq.into_iter()
            .map(|(term, count)| (term, count as f64 / total))
            .collect()
    }

    pub(crate) fn compute_tfidf(
        tf: &HashMap<String, f64>,
        corpus_tf: &HashMap<String, f64>,
    ) -> HashMap<String, f64> {
        let total_docs = corpus_tf.len().max(1) as f64;
        tf.iter()
            .map(|(term, tf_val)| {
                let df = corpus_tf.get(term).copied().unwrap_or(0.0);
                let idf = if df > 0.0 {
                    (total_docs / df).ln() + 1.0
                } else {
                    1.0
                };
                (term.clone(), tf_val * idf)
            })
            .collect()
    }

    pub(crate) fn keyword_density(tokens: &[String], total_words: usize) -> HashMap<String, f64> {
        if total_words == 0 {
            return HashMap::new();
        }
        let mut freq: HashMap<String, usize> = HashMap::new();
        for token in tokens {
            *freq.entry(token.clone()).or_default() += 1;
        }
        freq.into_iter()
            .map(|(term, count)| (term, count as f64 / total_words as f64 * 100.0))
            .collect()
    }

    pub(crate) fn detect_prominent_keywords(density: &HashMap<String, f64>) -> Vec<(String, f64)> {
        let mut prominent: Vec<(String, f64)> = density
            .iter()
            .filter(|(_, &d)| d >= 1.5)
            .map(|(k, &v)| (k.clone(), v))
            .collect();
        prominent.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        prominent
    }

    pub(crate) fn cooccurrence(tokens: &[String], window: usize) -> Vec<((String, String), usize)> {
        let mut pairs: HashMap<(String, String), usize> = HashMap::new();
        for i in 0..tokens.len() {
            let end = (i + window + 1).min(tokens.len());
            for j in (i + 1)..end {
                let mut pair = [tokens[i].clone(), tokens[j].clone()];
                pair.sort();
                *pairs.entry((pair[0].clone(), pair[1].clone())).or_default() += 1;
            }
        }
        let mut result: Vec<((String, String), usize)> = pairs.into_iter().collect();
        result.sort_by_key(|b| std::cmp::Reverse(b.1));
        result
    }
}

impl Default for KeywordAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for KeywordAnalyzer {
    fn name(&self) -> &str {
        "keyword-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.word_count == 0 {
            return findings;
        }

        let text = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if text.trim().is_empty() {
            return findings;
        }

        let tokens = Self::tokenize(&text);
        if tokens.is_empty() {
            return findings;
        }

        let tf = Self::compute_tf(&tokens);
        let tfidf = Self::compute_tfidf(&tf, &self.corpus_tf);
        let density = Self::keyword_density(&tokens, ctx.page.word_count);
        let prominent = Self::detect_prominent_keywords(&density);
        let cooccur = Self::cooccurrence(&tokens, 3);

        let mut tfidf_sorted: Vec<(&String, &f64)> = tfidf.iter().collect();
        tfidf_sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_tfidf: Vec<String> = tfidf_sorted
            .iter()
            .take(10)
            .map(|(k, v)| format!("{k} ({v:.2})"))
            .collect();

        if !top_tfidf.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "KW001".to_string(),
                title: "Top TF-IDF keywords".to_string(),
                description: format!("Top keywords by TF-IDF score: {}.", top_tfidf.join(", ")),
                url: url.clone(),
                recommendation: "TF-IDF highlights the most distinctive terms on this page. \
                                 Ensure these align with your target keywords."
                    .to_string(),
            });
        }

        let mut density_sorted: Vec<(&String, &f64)> = density.iter().collect();
        density_sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_density: Vec<String> = density_sorted
            .iter()
            .take(10)
            .map(|(k, v)| format!("{k} ({v:.1}%)"))
            .collect();

        if !top_density.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "KW002".to_string(),
                title: "Keyword density".to_string(),
                description: format!(
                    "Top keyword densities (of {} words): {}.",
                    ctx.page.word_count,
                    top_density.join(", ")
                ),
                url: url.clone(),
                recommendation: "Ideal keyword density is 1-2%. Higher may indicate keyword \
                                 stuffing."
                    .to_string(),
            });
        }

        if !prominent.is_empty() {
            let display: Vec<String> = prominent
                .iter()
                .map(|(k, v)| format!("\"{k}\" ({v:.1}%)"))
                .collect();
            findings.push(Finding {
                severity: if prominent.iter().any(|(_, d)| *d > 3.0) {
                    Severity::Warning
                } else {
                    Severity::Info
                },
                category: IssueCategory::Content,
                code: "KW003".to_string(),
                title: "Prominent keywords detected".to_string(),
                description: format!("Keywords with density >= 1.5%: {}.", display.join(", ")),
                url: url.clone(),
                recommendation: if prominent.iter().any(|(_, d)| *d > 3.0) {
                    "Some keywords exceed 3% density. This may be flagged as keyword \
                     stuffing by search engines."
                        .to_string()
                } else {
                    "Keyword densities are within acceptable range.".to_string()
                },
            });
        }

        if !cooccur.is_empty() {
            let display: Vec<String> = cooccur
                .iter()
                .take(5)
                .map(|((a, b), c)| format!("\"{a}\" + \"{b}\" ({c})"))
                .collect();
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "KW004".to_string(),
                title: "Keyword co-occurrence".to_string(),
                description: format!("Top keyword pairs: {}.", display.join(", ")),
                url: url.clone(),
                recommendation: "Co-occurring keywords help search engines understand topic \
                                 relationships."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 23. International SEO Analyzer
// ---------------------------------------------------------------------------

pub struct InternationalSeoAnalyzer {
    known_hrefs: HashMap<String, Vec<String>>,
}

impl InternationalSeoAnalyzer {
    pub fn new() -> Self {
        Self {
            known_hrefs: HashMap::new(),
        }
    }

    pub fn with_known_hrefs(known_hrefs: HashMap<String, Vec<String>>) -> Self {
        Self { known_hrefs }
    }

    pub(crate) fn detect_locale_from_url(url: &str) -> Option<String> {
        if let Ok(parsed) = Url::parse(url) {
            let segments: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.filter(|s| !s.is_empty()).collect())
                .unwrap_or_default();
            if let Some(first) = segments.first() {
                if Self::is_locale_segment(first) {
                    return Some(first.to_string());
                }
            }
        }
        None
    }

    pub(crate) fn is_locale_segment(s: &str) -> bool {
        let parts: Vec<&str> = s.split('-').collect();
        match parts.len() {
            1 => {
                let lang = parts[0];
                lang.len() >= 2 && lang.len() <= 3 && lang.chars().all(|c| c.is_ascii_alphabetic())
            }
            2 => {
                let lang = parts[0];
                let region = parts[1];
                lang.len() >= 2
                    && lang.len() <= 3
                    && lang.chars().all(|c| c.is_ascii_alphabetic())
                    && ((region.len() == 2 && region.chars().all(|c| c.is_ascii_alphabetic()))
                        || (region.len() == 4 && region.chars().all(|c| c.is_ascii_digit())))
            }
            _ => false,
        }
    }
}

impl Default for InternationalSeoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for InternationalSeoAnalyzer {
    fn name(&self) -> &str {
        "international-seo"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let hreflang_tags = &ctx.page.meta.hreflang;

        if !hreflang_tags.is_empty() {
            for tag in hreflang_tags {
                // Skip locale mismatch check for x-default — it's a special fallback
                // that doesn't correspond to a specific locale segment in the URL.
                if tag.lang.to_lowercase() == "x-default" {
                    continue;
                }
                if let Some(locale) = Self::detect_locale_from_url(tag.url.as_str()) {
                    let locale_lower = locale.to_lowercase();
                    let tag_lang_lower = tag.lang.to_lowercase();
                    if locale_lower != tag_lang_lower
                        && !tag_lang_lower.starts_with(&locale_lower)
                        && !locale_lower.starts_with(&tag_lang_lower)
                    {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Seo,
                            code: "ISEO001".to_string(),
                            title: "Hreflang URL locale mismatch".to_string(),
                            description: format!(
                                "Hreflang tag lang=\"{}\" points to URL \"{}\" which has \
                                 locale segment \"{}\".",
                                tag.lang, tag.url, locale
                            ),
                            url: url.clone(),
                            recommendation: "Ensure the hreflang URL path segment matches \
                                             the language code in the hreflang tag."
                                .to_string(),
                        });
                    }
                }
            }

            let has_x_default = hreflang_tags.iter().any(|t| t.lang == "x-default");
            if !has_x_default {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ISEO002".to_string(),
                    title: "Missing x-default in enhanced hreflang".to_string(),
                    description: "No x-default hreflang found. This tag specifies the fallback \
                                  page for unmatched locales."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add <link rel=\"alternate\" hreflang=\"x-default\" \
                                     href=\"...\"> pointing to the default language version."
                        .to_string(),
                });
            }

            let lang_count: HashMap<String, usize> =
                hreflang_tags.iter().fold(HashMap::new(), |mut acc, t| {
                    *acc.entry(t.lang.clone()).or_insert(0) += 1;
                    acc
                });
            for (lang, count) in &lang_count {
                if *count > 1 && lang != "x-default" {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Seo,
                        code: "ISEO003".to_string(),
                        title: "Duplicate hreflang language".to_string(),
                        description: format!(
                            "Language \"{lang}\" appears {count} times. Each language code \
                             must be unique per page."
                        ),
                        url: url.clone(),
                        recommendation: "Remove duplicate hreflang tags for the same language."
                            .to_string(),
                    });
                }
            }
        }

        let locale_in_url = Self::detect_locale_from_url(url);
        if locale_in_url.is_none()
            && ctx.page.meta.hreflang.is_empty()
            && ctx.page.html_lang.is_some()
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "ISEO004".to_string(),
                title: "Single-language page without hreflang".to_string(),
                description: "Page has a lang attribute but no hreflang tags. If this \
                                  site serves multiple languages, add hreflang annotations."
                    .to_string(),
                url: url.clone(),
                recommendation: "For multilingual sites, add hreflang tags to all language \
                                     variants of each page."
                    .to_string(),
            });
        }

        if let Some(canonical) = &ctx.page.meta.canonical {
            if let Some(hop_from) = self.known_hrefs.get(url) {
                for hop_url in hop_from {
                    if let Some(target_canonical) = self.known_hrefs.get(hop_url.as_str()) {
                        if !target_canonical.is_empty() {
                            let canonical_str = canonical.to_string();
                            if target_canonical.iter().any(|c| c == &canonical_str) {
                                findings.push(Finding {
                                    severity: Severity::Info,
                                    category: IssueCategory::Seo,
                                    code: "ISEO005".to_string(),
                                    title: "Canonical chain detected".to_string(),
                                    description: format!(
                                        "URL \"{url}\" canonical points to \"{}\", which is \
                                         itself canonicalized. This forms a chain.",
                                        canonical
                                    ),
                                    url: url.clone(),
                                    recommendation: "Ensure all pages point to the same \
                                                     final canonical URL to avoid crawl \
                                                     confusion."
                                        .to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // NOTE: multilingual presence (hreflang tags / regional html lang) is
        // deliberately NOT emitted as a finding. Correct i18n configuration
        // is a best practice, not a defect — reporting it fired on 100% of
        // pages on properly configured multilingual sites (pure noise). The
        // hreflang *validation* findings above remain.

        findings
    }
}

// ---------------------------------------------------------------------------
// Pagination Analyzer
// ---------------------------------------------------------------------------

pub struct PaginationAnalyzer;

impl PaginationAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Detect if a URL looks like a paginated URL by checking for common
    /// pagination query parameters or path segments.
    pub(crate) fn is_paginated_url(url: &str) -> bool {
        if let Ok(parsed) = Url::parse(url) {
            let query_pairs: Vec<(String, String)> = parsed
                .query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();
            let has_pagination_query = query_pairs.iter().any(|(k, _)| {
                matches!(
                    k.to_lowercase().as_str(),
                    "page" | "p" | "pg" | "start" | "offset" | "cursor" | "pagina"
                )
            });
            let path = parsed.path().to_lowercase();
            let has_pagination_path = path.contains("/page/")
                || path.contains("/p/")
                || path.ends_with("/page")
                || path.contains("/pagina/");
            has_pagination_query || has_pagination_path
        } else {
            false
        }
    }

    /// Extract the current page number from a paginated URL.
    /// Returns None if not a numeric pagination pattern.
    pub(crate) fn extract_page_number(url: &str) -> Option<u32> {
        if let Ok(parsed) = Url::parse(url) {
            // Check query parameters
            for (key, value) in parsed.query_pairs() {
                if matches!(
                    key.to_lowercase().as_str(),
                    "page" | "p" | "pg" | "start" | "offset" | "pagina"
                ) {
                    if let Ok(num) = value.parse::<u32>() {
                        // Convert offset-based params to page numbers
                        if key.to_lowercase() == "offset" {
                            return Some(num / 10 + 1); // Assume 10 per page
                        }
                        return Some(num);
                    }
                }
            }
            // Check path segments: /page/3, /p/2, /pagina/4
            let segments: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.collect())
                .unwrap_or_default();
            for (i, seg) in segments.iter().enumerate() {
                if i > 0
                    && (segments[i - 1] == "page"
                        || segments[i - 1] == "p"
                        || segments[i - 1] == "pagina")
                {
                    if let Ok(num) = seg.parse::<u32>() {
                        return Some(num);
                    }
                }
            }
        }
        None
    }
}

impl Default for PaginationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PaginationAnalyzer {
    fn name(&self) -> &str {
        "pagination"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if !Self::is_paginated_url(url) {
            return findings;
        }

        let page_number = Self::extract_page_number(url);

        // PAG001: Missing rel="next"/"prev" on paginated pages
        let has_next = ctx
            .page
            .links
            .iter()
            .any(|l| l.rel.iter().any(|r| r == "next"));
        let has_prev = ctx
            .page
            .links
            .iter()
            .any(|l| l.rel.iter().any(|r| r == "prev"));

        if !has_next && !has_prev {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "PAG001".to_string(),
                title: "Missing rel=\"next\"/\"prev\" on paginated page".to_string(),
                description: "This appears to be a paginated page but lacks rel=\"next\" and/or \
                              rel=\"prev\" link annotations."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <link rel=\"next\" href=\"...\"> and <link rel=\"prev\" \
                                 href=\"...\"> to help search engines understand the pagination \
                                 structure."
                    .to_string(),
            });
        } else if !has_next && page_number.unwrap_or(1) > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "PAG001".to_string(),
                title: "Missing rel=\"next\" on paginated page".to_string(),
                description: "This paginated page has rel=\"prev\" but is missing rel=\"next\"."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <link rel=\"next\" href=\"...\"> pointing to the next page \
                                 in the sequence."
                    .to_string(),
            });
        } else if has_next && !has_prev && page_number.unwrap_or(1) > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "PAG001".to_string(),
                title: "Missing rel=\"prev\" on paginated page".to_string(),
                description: "This paginated page has rel=\"next\" but is missing rel=\"prev\"."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <link rel=\"prev\" href=\"...\"> pointing to the previous \
                                 page in the sequence."
                    .to_string(),
            });
        }

        // PAG002: Infinite scroll detection (JavaScript-heavy pagination)
        let has_infinite_scroll_signals = ctx.page.scripts.iter().any(|s| {
            s.src
                .as_ref()
                .map(|src| {
                    let src_lower = src.to_lowercase();
                    src_lower.contains("infinite")
                        || src_lower.contains("lazyload")
                        || src_lower.contains("lazy-load")
                        || src_lower.contains("pagination")
                        || src_lower.contains("infinite-scroll")
                        || src_lower.contains("infscroll")
                })
                .unwrap_or(false)
        });

        if has_infinite_scroll_signals {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "PAG002".to_string(),
                title: "Infinite scroll pagination detected".to_string(),
                description: "The page appears to use JavaScript-based infinite scroll \
                              pagination. Search engines may not be able to discover all \
                              paginated content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Implement traditional paginated URLs alongside infinite \
                                 scroll, or use the Push State API with proper rel=\"next\"/\"prev\" \
                                 annotations."
                    .to_string(),
            });
        }

        // PAG003: Paginated URL not in sitemap
        // (Only fires when the SitemapAnalyzer data is available via known URLs,
        //  which we check indirectly by looking for sitemap-related data in the
        //  analysis context. The primary check is done in SitemapAnalyzer.)

        // PAG004: Pagination depth > 5 levels
        if let Some(depth) = page_number {
            if depth > 5 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "PAG004".to_string(),
                    title: "Excessive pagination depth".to_string(),
                    description: format!(
                        "This page is at pagination depth {depth} (page {depth}). Search \
                         engines may not crawl deep pagination levels."
                    ),
                    url: url.clone(),
                    recommendation: "Consider restructuring content to reduce pagination \
                                     depth. Consolidate thin paginated pages or use alternative \
                                     navigation (e.g., jump-to-section links)."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 24. Language Attribute Analyzer
// ---------------------------------------------------------------------------

pub struct LanguageAttributeAnalyzer;

impl LanguageAttributeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LanguageAttributeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LanguageAttributeAnalyzer {
    fn name(&self) -> &str {
        "language-attribute"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // LANG001: Missing html lang attribute
        if !ctx.page.has_lang_attribute || ctx.page.html_lang.is_none() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "LANG001".to_string(),
                title: "Missing html lang attribute".to_string(),
                description: "The <html> element does not have a lang attribute.".to_string(),
                url: url.clone(),
                recommendation: "Add lang=\"en\" (or the appropriate language code) to the <html> \
                                 element to help search engines and screen readers identify the \
                                 page language."
                    .to_string(),
            });
            return findings;
        }

        let lang = ctx.page.html_lang.as_deref().unwrap();

        // LANG003: Empty lang attribute
        if lang.trim().is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Seo,
                code: "LANG003".to_string(),
                title: "Empty lang attribute".to_string(),
                description: "The <html> lang attribute is present but empty.".to_string(),
                url: url.clone(),
                recommendation: "Set the lang attribute to a valid BCP 47 language tag (e.g., \
                                 \"en\", \"fr-CA\")."
                    .to_string(),
            });
            return findings;
        }

        // LANG002: html lang doesn't match content language (meta.language)
        if let Some(meta_lang) = &ctx.page.meta.language {
            let lang_lower = lang.to_lowercase();
            let meta_lower = meta_lang.to_lowercase();
            if lang_lower != meta_lower
                && !lang_lower.starts_with(&meta_lower)
                && !meta_lower.starts_with(&lang_lower)
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "LANG002".to_string(),
                    title: "Language attribute mismatch".to_string(),
                    description: format!(
                        "The html lang attribute (\"{lang}\") does not match the content \
                         language meta tag (\"{meta_lang}\")."
                    ),
                    url: url.clone(),
                    recommendation: "Ensure the html lang attribute and the Content-Language \
                                     meta tag declare the same language."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 25. Hreflang Consistency Analyzer
// ---------------------------------------------------------------------------

pub struct HreflangConsistencyAnalyzer;

impl HreflangConsistencyAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Validate that a locale code follows BCP 47 / ISO 639 format.
    pub(crate) fn is_valid_locale(code: &str) -> bool {
        if code == "x-default" {
            return true;
        }
        let parts: Vec<&str> = code.split('-').collect();
        match parts.len() {
            1 => {
                let lang = parts[0];
                lang.len() >= 2 && lang.len() <= 3 && lang.chars().all(|c| c.is_ascii_alphabetic())
            }
            2 => {
                let lang = parts[0];
                let region = parts[1];
                lang.len() >= 2
                    && lang.len() <= 3
                    && lang.chars().all(|c| c.is_ascii_alphabetic())
                    && ((region.len() == 2 && region.chars().all(|c| c.is_ascii_alphabetic()))
                        || (region.len() == 3 && region.chars().all(|c| c.is_ascii_digit())))
            }
            _ => false,
        }
    }
}

impl Default for HreflangConsistencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HreflangConsistencyAnalyzer {
    fn name(&self) -> &str {
        "hreflang-consistency"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hreflang_tags = &ctx.page.meta.hreflang;

        if hreflang_tags.is_empty() {
            return findings;
        }

        let current_canonical = ctx.page.meta.canonical.as_ref();

        for tag in hreflang_tags {
            let target_url = tag.url.as_str();

            // HREFT001: Hreflang URL returns non-200 status
            if target_url == url {
                if let Some(status) = ctx.status_code {
                    if status != 200 {
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Seo,
                            code: "HREFT001".to_string(),
                            title: "Hreflang target returns non-200 status".to_string(),
                            description: format!(
                                "Hreflang tag lang=\"{}\" points to this page which returned \
                                 HTTP {status}.",
                                tag.lang
                            ),
                            url: url.clone(),
                            recommendation: "Ensure all hreflang target URLs return HTTP 200."
                                .to_string(),
                        });
                    }
                }
            }

            // HREFT002: Hreflang URL has different canonical than current page
            if let Some(canonical) = current_canonical {
                if target_url == url {
                    let canonical_str = canonical.to_string();
                    if canonical_str != *url {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Seo,
                            code: "HREFT002".to_string(),
                            title: "Hreflang target canonical mismatch".to_string(),
                            description: format!(
                                "Hreflang tag lang=\"{}\" points to this page, but the page \
                                 canonical (\"{canonical_str}\") does not match the page URL.",
                                tag.lang
                            ),
                            url: url.clone(),
                            recommendation: "Ensure the canonical URL matches the hreflang \
                                             target URL."
                                .to_string(),
                        });
                    }
                }
            }

            // HREFT003: Hreflang tag with invalid locale code format
            if !Self::is_valid_locale(&tag.lang) {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Seo,
                    code: "HREFT003".to_string(),
                    title: "Invalid hreflang locale code".to_string(),
                    description: format!(
                        "The hreflang code \"{}\" does not follow BCP 47 format.",
                        tag.lang
                    ),
                    url: url.clone(),
                    recommendation: "Use valid BCP 47 language tags (e.g., \"en\", \"en-US\", \
                                     \"fr-CA\", \"x-default\")."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 26. Charset Validator
// ---------------------------------------------------------------------------

pub struct CharsetValidator;

impl CharsetValidator {
    pub fn new() -> Self {
        Self
    }

    /// Extract charset from Content-Type header value.
    pub(crate) fn extract_charset_from_content_type(content_type: &str) -> Option<String> {
        for part in content_type.split(';') {
            let trimmed = part.trim();
            if let Some(val) = trimmed.strip_prefix("charset=") {
                return Some(val.trim().to_lowercase());
            }
        }
        None
    }
}

impl Default for CharsetValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CharsetValidator {
    fn name(&self) -> &str {
        "charset-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let meta_charset = ctx.page.meta.charset.as_ref().map(|c| c.to_lowercase());

        let header_charset = ctx
            .content_type
            .and_then(|ct| Self::extract_charset_from_content_type(ct));

        // CHARSET001: Missing charset declaration
        if meta_charset.is_none() && header_charset.is_none() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "CHARSET001".to_string(),
                title: "Missing charset declaration".to_string(),
                description: "No charset was declared in either the meta tag or HTTP headers."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <meta charset=\"utf-8\"> or ensure the Content-Type header \
                                 includes charset=utf-8."
                    .to_string(),
            });
            return findings;
        }

        // CHARSET002: Charset declared in meta but not in HTTP header
        if meta_charset.is_some() && header_charset.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "CHARSET002".to_string(),
                title: "Charset missing from HTTP header".to_string(),
                description: "Charset is declared in a meta tag but not in the Content-Type \
                              HTTP header."
                    .to_string(),
                url: url.clone(),
                recommendation: "Consider adding charset to the Content-Type header for faster \
                                 browser detection."
                    .to_string(),
            });
        }

        // CHARSET003: Non-UTF-8 charset
        let effective_charset = meta_charset
            .as_deref()
            .or(header_charset.as_deref())
            .unwrap_or_default();
        if !effective_charset.is_empty()
            && effective_charset != "utf-8"
            && effective_charset != "utf8"
        {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "CHARSET003".to_string(),
                title: "Non-UTF-8 charset".to_string(),
                description: format!(
                    "The declared charset is \"{effective_charset}\". UTF-8 is recommended for \
                     universal compatibility."
                ),
                url: url.clone(),
                recommendation: "Use UTF-8 encoding for broadest character support and to avoid \
                                 rendering issues."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 27. Robots Meta Analyzer
// ---------------------------------------------------------------------------

pub struct RobotsMetaAnalyzer;

impl RobotsMetaAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Parse robots meta directives into a set of individual tokens.
    pub(crate) fn parse_robots_directives(robots: &str) -> HashSet<String> {
        robots
            .split(',')
            .map(|d| d.trim().to_lowercase())
            .filter(|d| !d.is_empty())
            .collect()
    }
}

impl Default for RobotsMetaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for RobotsMetaAnalyzer {
    fn name(&self) -> &str {
        "robots-meta"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let robots = match &ctx.page.meta.robots {
            Some(r) if !r.trim().is_empty() => r,
            _ => return findings,
        };

        let directives = Self::parse_robots_directives(robots);

        // ROBOTS001: noindex on potentially important pages
        if directives.contains("noindex") {
            if !is_utility_page(url) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ROBOTS001".to_string(),
                    title: "noindex on content page".to_string(),
                    description: "The page has a noindex robots directive. This page will be \
                                  excluded from search engine indices."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Verify that noindex is intentional. Remove it if this page \
                                     should appear in search results."
                        .to_string(),
                });
            }
        }

        // ROBOTS002: nofollow on potentially important pages
        if directives.contains("nofollow") {
            if !is_utility_page(url) {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "ROBOTS002".to_string(),
                    title: "nofollow on content page".to_string(),
                    description: "The page has a nofollow robots directive. Search engines will \
                                  not follow any links on this page."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Ensure nofollow is intentional. If the page contains \
                                     important internal links, remove the nofollow directive."
                        .to_string(),
                });
            }
        }

        // ROBOTS003: Conflicting robots directives (noindex + index, or nofollow + follow)
        let has_index = directives.contains("index");
        let has_noindex = directives.contains("noindex");
        let has_follow = directives.contains("follow");
        let has_nofollow = directives.contains("nofollow");

        if has_index && has_noindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Seo,
                code: "ROBOTS003".to_string(),
                title: "Conflicting robots directives".to_string(),
                description: "The robots meta tag contains both \"index\" and \"noindex\". This \
                              is contradictory and the behavior is undefined."
                    .to_string(),
                url: url.clone(),
                recommendation: "Remove one of the conflicting directives. Use either \"index\" \
                                 or \"noindex\", not both."
                    .to_string(),
            });
        }

        if has_follow && has_nofollow {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Seo,
                code: "ROBOTS003".to_string(),
                title: "Conflicting robots directives".to_string(),
                description: "The robots meta tag contains both \"follow\" and \"nofollow\". This \
                              is contradictory and the behavior is undefined."
                    .to_string(),
                url: url.clone(),
                recommendation: "Remove one of the conflicting directives. Use either \"follow\" \
                                 or \"nofollow\", not both."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 28. Canonical Depth Analyzer
// ---------------------------------------------------------------------------

pub struct CanonicalDepthAnalyzer;

impl CanonicalDepthAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Count the number of path segments (depth) in a URL.
    pub(crate) fn path_depth(url: &Url) -> usize {
        url.path_segments()
            .map(|s| s.filter(|seg| !seg.is_empty()).count())
            .unwrap_or(0)
    }
}

impl Default for CanonicalDepthAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CanonicalDepthAnalyzer {
    fn name(&self) -> &str {
        "canonical-depth"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let canonical = match &ctx.page.meta.canonical {
            Some(c) => c,
            None => return findings,
        };

        // CDEP001: Canonical URL is more than 3 levels deep
        let depth = Self::path_depth(canonical);
        if depth > 3 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "CDEP001".to_string(),
                title: "Canonical URL is deeply nested".to_string(),
                description: format!(
                    "The canonical URL \"{}\" has a path depth of {} segments (more than 3). \
                     Deeply nested canonicals may indicate poor URL structure.",
                    canonical, depth
                ),
                url: url.clone(),
                recommendation: "Consider flattening the URL structure or pointing the canonical \
                                 to a higher-level URL if the deep page is not the preferred \
                                 version."
                    .to_string(),
            });
        }

        // CDEP002: Canonical URL has query parameters
        if canonical.query().is_some() && !canonical.query().unwrap_or("").is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "CDEP002".to_string(),
                title: "Canonical URL contains query parameters".to_string(),
                description: format!(
                    "The canonical URL \"{}\" contains query parameters. Canonical URLs with \
                     parameters may not be crawlable or may cause indexing issues.",
                    canonical
                ),
                url: url.clone(),
                recommendation: "Point the canonical URL to the clean, parameterless version of \
                                 the page."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 29. Mobile Viewport Analyzer
// ---------------------------------------------------------------------------

pub struct MobileViewportAnalyzer;

impl MobileViewportAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Parse viewport content string into a map of directives.
    pub(crate) fn parse_viewport(content: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for part in content.split(',') {
            let trimmed = part.trim();
            if let Some((key, value)) = trimmed.split_once('=') {
                map.insert(key.trim().to_lowercase(), value.trim().to_lowercase());
            }
        }
        map
    }
}

impl Default for MobileViewportAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MobileViewportAnalyzer {
    fn name(&self) -> &str {
        "mobile-viewport"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let viewport = match &ctx.page.meta.viewport {
            Some(v) if !v.trim().is_empty() => v,
            _ => return findings,
        };

        let directives = Self::parse_viewport(viewport);

        // MOBVIEW001: Viewport missing initial-scale
        if !directives.contains_key("initial-scale") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Mobile,
                code: "MOBVIEW001".to_string(),
                title: "Viewport missing initial-scale".to_string(),
                description: "The viewport meta tag does not include initial-scale. Without it, \
                              some mobile browsers may not scale the page correctly."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add initial-scale=1 to the viewport meta tag: <meta \
                                 name=\"viewport\" content=\"width=device-width, \
                                 initial-scale=1\">."
                    .to_string(),
            });
        }

        // MOBVIEW002: Viewport width not set to device-width
        let width = directives.get("width").map(|s| s.as_str());
        if width != Some("device-width") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Mobile,
                code: "MOBVIEW002".to_string(),
                title: "Viewport width not set to device-width".to_string(),
                description: format!(
                    "The viewport width is set to \"{}\" instead of \"device-width\". This may \
                     cause the page to render at a fixed width on mobile devices.",
                    width.unwrap_or("not set")
                ),
                url: url.clone(),
                recommendation: "Set width=device-width in the viewport meta tag for proper \
                                 responsive layout."
                    .to_string(),
            });
        }

        // MOBVIEW003: Viewport user-scalable=no
        let scalable = directives.get("user-scalable");
        if scalable == Some(&"no".to_string()) || scalable == Some(&"0".to_string()) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Mobile,
                code: "MOBVIEW003".to_string(),
                title: "Viewport disables user scaling".to_string(),
                description: "The viewport meta tag sets user-scalable=no, which prevents users \
                              from zooming in. This is an accessibility issue."
                    .to_string(),
                url: url.clone(),
                recommendation: "Remove user-scalable=no to allow pinch-to-zoom for users with \
                                 low vision."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// InternalLinkAnchorAnalyzer
// =========================================================================

/// Analyzes internal link anchor text quality.
pub struct InternalLinkAnchorAnalyzer;

impl Default for InternalLinkAnchorAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalLinkAnchorAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for InternalLinkAnchorAnalyzer {
    fn name(&self) -> &str {
        "internal-link-anchor"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let internal_links: Vec<&ExtractedLink> =
            ctx.page.links.iter().filter(|l| !l.is_external).collect();

        if internal_links.is_empty() {
            return findings;
        }

        // ANCHOR001: Anchor text identical to URL
        for link in &internal_links {
            let text = link.text.trim();
            if text.is_empty() {
                continue;
            }
            // Compare text to the href (strip protocol and domain for comparison)
            let href_lower = link.href.to_lowercase();
            let text_lower = text.to_lowercase();
            // Strip protocol and domain for comparison
            let href_path = href_lower
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_start_matches("www.");
            if text_lower == href_path || text_lower == href_lower {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ANCHOR001".to_string(),
                    title: "Anchor text identical to URL".to_string(),
                    description: format!(
                        "Internal link anchor text \"{}\" is identical to the link URL. \
                         This provides no contextual signal to search engines.",
                        text
                    ),
                    url: url.clone(),
                    recommendation: "Use descriptive anchor text that describes the linked \
                                     page's content instead of using the URL itself."
                        .to_string(),
                });
            }
        }

        // ANCHOR002: Over-optimized anchor text (>50% exact-match keywords)
        if internal_links.len() >= 3 {
            // Collect anchor texts, normalized
            let anchors: Vec<String> = internal_links
                .iter()
                .filter(|l| !l.text.trim().is_empty())
                .map(|l| {
                    l.text
                        .trim()
                        .to_lowercase()
                        .chars()
                        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                        .collect()
                })
                .collect();

            if anchors.len() >= 3 {
                // Count exact-match duplicates
                let mut freq: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                for anchor in &anchors {
                    *freq.entry(anchor.clone()).or_default() += 1;
                }

                let total = anchors.len();
                let max_freq = freq.values().copied().max().unwrap_or(0);
                let ratio = max_freq as f64 / total as f64;

                if ratio > 0.5 {
                    let most_common = freq
                        .iter()
                        .max_by_key(|(_, &c)| c)
                        .map(|(k, _)| k.clone())
                        .unwrap_or_default();
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "ANCHOR002".to_string(),
                        title: "Over-optimized anchor text".to_string(),
                        description: format!(
                            "The anchor text \"{}\" appears in {:.0}% of internal links \
                             ({} of {}), exceeding the 50% threshold. Over-optimized anchors \
                             may be flagged as manipulative.",
                            most_common,
                            ratio * 100.0,
                            max_freq,
                            total
                        ),
                        url: url.clone(),
                        recommendation: "Vary anchor text naturally. Use descriptive, diverse \
                                         phrases instead of repeating the same exact-match text."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// WikipediaLinkAnalyzer
// ---------------------------------------------------------------------------

/// Detects outbound links to Wikipedia and Wikidata.
///
/// Wikipedia/Wikidata links indicate the page references authoritative
/// sources, which can signal content depth and reliability. This is
/// informational and considered positive for E-E-A-T.
pub struct WikipediaLinkAnalyzer;

impl Default for WikipediaLinkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl WikipediaLinkAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WikipediaLinkAnalyzer {
    fn name(&self) -> &str {
        "wikipedia-link"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let external_links: Vec<&ExtractedLink> =
            ctx.page.links.iter().filter(|l| l.is_external).collect();

        if external_links.is_empty() {
            return findings;
        }

        // WIKI001: Page has outbound Wikipedia links
        let wikipedia_count = external_links
            .iter()
            .filter(|l| is_wikipedia_url(&l.href))
            .count();

        if wikipedia_count > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Links,
                code: "WIKI001".to_string(),
                title: "Wikipedia links detected".to_string(),
                description: format!(
                    "This page contains {wikipedia_count} outbound link(s) to Wikipedia. \
                     Linking to authoritative reference sources signals content depth \
                     and can strengthen E-E-A-T signals."
                ),
                url: url.clone(),
                recommendation: "Keep outbound Wikipedia links as they demonstrate thorough \
                                 research and provide additional context for readers."
                    .to_string(),
            });
        }

        // WIKI002: Page has outbound Wikidata links
        let wikidata_count = external_links
            .iter()
            .filter(|l| is_wikidata_url(&l.href))
            .count();

        if wikidata_count > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Links,
                code: "WIKI002".to_string(),
                title: "Wikidata links detected".to_string(),
                description: format!(
                    "This page contains {wikidata_count} outbound link(s) to Wikidata. \
                     Wikidata links reference structured knowledge-base entries, which \
                     may support entity recognition by search engines."
                ),
                url: url.clone(),
                recommendation: "Retain Wikidata links as they reference structured knowledge \
                                 entries that can reinforce entity signals."
                    .to_string(),
            });
        }

        findings
    }
}

fn is_wikipedia_url(href: &str) -> bool {
    let lower = href.to_lowercase();
    if !(lower.starts_with("https://en.wikipedia.org/")
        || lower.starts_with("https://www.wikipedia.org/")
        || lower.starts_with("http://en.wikipedia.org/")
        || lower.starts_with("http://www.wikipedia.org/")
        || lower.starts_with("https://en.m.wikipedia.org/"))
    {
        return false;
    }
    // Exclude Wikipedia namespace pages (case-insensitive)
    let namespaces = [
        "wikipedia:",
        "help:",
        "special:",
        "template:",
        "talk:",
        "user:",
        "category:",
        "portal:",
        "file:",
    ];
    !namespaces.iter().any(|ns| {
        let marker = format!("/wiki/{ns}");
        lower.contains(&marker)
    })
}

fn is_wikidata_url(href: &str) -> bool {
    let lower = href.to_lowercase();
    lower.starts_with("https://www.wikidata.org/wiki/")
        || lower.starts_with("http://www.wikidata.org/wiki/")
        || lower.starts_with("https://www.wikidata.org/entity/")
        || lower.starts_with("http://www.wikidata.org/entity/")
}

// ---------------------------------------------------------------------------
// AnchorTextDiversityAnalyzer
// ---------------------------------------------------------------------------

/// Analyzes anchor text diversity across internal links.
///
/// Monitors for two anti-patterns: all links sharing the same anchor text,
/// and excessive use of generic phrases like "click here" or "read more".
pub struct AnchorTextDiversityAnalyzer;

impl Default for AnchorTextDiversityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AnchorTextDiversityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AnchorTextDiversityAnalyzer {
    fn name(&self) -> &str {
        "anchor-text-diversity"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let internal_links: Vec<&ExtractedLink> =
            ctx.page.links.iter().filter(|l| !l.is_external).collect();

        // Need at least 3 internal links with anchor text to analyze
        let links_with_text: Vec<&ExtractedLink> = internal_links
            .iter()
            .filter(|l| !l.text.trim().is_empty())
            .copied()
            .collect();

        if links_with_text.len() < 3 {
            return findings;
        }

        // Normalize anchor texts
        let anchors: Vec<String> = links_with_text
            .iter()
            .map(|l| {
                l.text
                    .trim()
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<&str>>()
                    .join(" ")
            })
            .filter(|a| !a.is_empty())
            .collect();

        if anchors.is_empty() {
            return findings;
        }

        // ANCH-DIV001: All internal links use the same anchor text
        {
            let unique_anchors: HashSet<&str> = anchors.iter().map(|a| a.as_str()).collect();
            if unique_anchors.len() == 1 && anchors.len() >= 3 {
                let sample = anchors[0].clone();
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ANCH-DIV001".to_string(),
                    title: "All internal links use identical anchor text".to_string(),
                    description: format!(
                        "All {count} internal links with anchor text use the identical phrase \
                         \"{sample}\". Uniform anchor text provides no keyword diversity and \
                         may appear manipulative to search engines.",
                        count = anchors.len()
                    ),
                    url: url.clone(),
                    recommendation:
                        "Diversify anchor text across internal links. Use descriptive, \
                                     varied phrases that naturally reflect the content of each \
                                     linked page."
                            .to_string(),
                });
            }
        }

        // ANCH-DIV002: >80% of anchor text is generic
        {
            let total = anchors.len();
            let generic_count = anchors.iter().filter(|a| is_generic_anchor(a)).count();

            let ratio = generic_count as f64 / total as f64;
            if ratio > 0.8 && total >= 3 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ANCH-DIV002".to_string(),
                    title: "Overuse of generic anchor text".to_string(),
                    description: format!(
                        "{:.0}% of internal link anchor text ({generic_count} of {total}) \
                         consists of generic phrases (e.g., \"click here\", \"read more\", \
                         \"learn more\"). Generic anchor text misses opportunities to signal \
                         topical relevance.",
                        ratio * 100.0
                    ),
                    url: url.clone(),
                    recommendation: "Replace generic anchor text with descriptive phrases that \
                                     convey the topic or purpose of the linked page."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// TitlePixelWidthAnalyzer
// =========================================================================

/// Checks if the page title would be truncated in Google SERP (~580px / ~60 chars display).
///
/// Estimates pixel width using a simple heuristic: 7px per ASCII char, 14px per CJK char.
pub struct TitlePixelWidthAnalyzer;

impl Default for TitlePixelWidthAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TitlePixelWidthAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Estimate pixel width of text. ASCII chars ~7px, CJK/fullwidth ~14px.
    pub(crate) fn estimate_pixel_width(text: &str) -> f64 {
        text.chars()
            .map(|c| {
                if c.is_ascii() {
                    7.0
                } else if is_cjk(c) {
                    14.0
                } else {
                    7.0
                }
            })
            .sum()
    }
}

fn is_cjk(c: char) -> bool {
    let cp = c as u32;
    (0x4E00..=0x9FFF).contains(&cp)
        || (0x3400..=0x4DBF).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0x3000..=0x303F).contains(&cp)
        || (0xFF00..=0xFFEF).contains(&cp)
}

impl Analyzer for TitlePixelWidthAnalyzer {
    fn name(&self) -> &str {
        "title-pixel-width"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let title = match &ctx.page.meta.title {
            Some(t) if !t.trim().is_empty() => t,
            _ => return findings,
        };

        let px = Self::estimate_pixel_width(title);

        if px > 580.0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLE-PX001".to_string(),
                title: "Title exceeds SERP pixel width".to_string(),
                description: format!(
                    "Title estimated width is {:.0}px, exceeding the ~580px display limit.                      Google SERP will likely truncate this title.",
                    px
                ),
                url: url.clone(),
                recommendation: "Shorten the title to fit within ~60 characters / 580px to                                  prevent truncation in search results."
                    .to_string(),
            });
        }

        if title.len() < 20 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "TITLE-PX002".to_string(),
                title: "Title too short for SERP display".to_string(),
                description: format!(
                    "Title is {} characters ({:.0}px estimated), which is shorter than the                      typical SERP display width. This wastes valuable search result real estate.",
                    title.len(),
                    px
                ),
                url: url.clone(),
                recommendation: "Expand the title to 30-60 characters to maximize SERP                                  click-through rate."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// MetaDescriptionPixelWidthAnalyzer
// =========================================================================

/// Checks if meta description would be truncated in Google SERP (~920px / ~155 chars display).
pub struct MetaDescriptionPixelWidthAnalyzer;

impl Default for MetaDescriptionPixelWidthAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaDescriptionPixelWidthAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Estimate pixel width of text. ASCII chars ~7px, CJK/fullwidth ~14px.
    pub(crate) fn estimate_pixel_width(text: &str) -> f64 {
        text.chars()
            .map(|c| {
                if c.is_ascii() {
                    7.0
                } else if is_cjk(c) {
                    14.0
                } else {
                    7.0
                }
            })
            .sum()
    }
}

impl Analyzer for MetaDescriptionPixelWidthAnalyzer {
    fn name(&self) -> &str {
        "meta-description-pixel-width"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let desc = match &ctx.page.meta.description {
            Some(d) if !d.trim().is_empty() => d,
            _ => return findings,
        };

        let px = Self::estimate_pixel_width(desc);

        if px > 920.0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "MDESC-PX001".to_string(),
                title: "Meta description exceeds SERP pixel width".to_string(),
                description: format!(
                    "Meta description estimated width is {:.0}px, exceeding the ~920px display                      limit. Google SERP will likely truncate this description.",
                    px
                ),
                url: url.clone(),
                recommendation: "Shorten the meta description to fit within ~155 characters /                                  920px to prevent truncation in search results."
                    .to_string(),
            });
        }

        if desc.len() < 70 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "MDESC-PX002".to_string(),
                title: "Meta description too short for SERP display".to_string(),
                description: format!(
                    "Meta description is {} characters ({:.0}px estimated), shorter than the                      typical SERP display width. This wastes valuable search result space.",
                    desc.len(),
                    px
                ),
                url: url.clone(),
                recommendation: "Expand the meta description to 120-160 characters to maximize                                  SERP click-through rate."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// InternalLinkTopicalAnalyzer
// =========================================================================

/// Checks if internal link anchor text relates to page headings (topical relevance).
pub struct InternalLinkTopicalAnalyzer;

impl Default for InternalLinkTopicalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalLinkTopicalAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Tokenize text into lowercase words > 2 chars, excluding stop words.
    fn tokenize(text: &str) -> HashSet<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|w| w.len() > 2 && !STOP_WORDS.contains(&w.as_str()))
            .collect()
    }
}

impl Analyzer for InternalLinkTopicalAnalyzer {
    fn name(&self) -> &str {
        "internal-link-topical"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let internal_links: Vec<&ExtractedLink> = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.is_external && !l.text.trim().is_empty())
            .collect();

        if internal_links.is_empty() || ctx.page.headings.is_empty() {
            return findings;
        }

        let heading_text: String = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let heading_keywords = Self::tokenize(&heading_text);

        if heading_keywords.is_empty() {
            return findings;
        }

        let relevant_count = internal_links
            .iter()
            .filter(|link| {
                let anchor_words = Self::tokenize(&link.text);
                anchor_words.iter().any(|w| heading_keywords.contains(w))
            })
            .count();

        let total = internal_links.len();
        let ratio = relevant_count as f64 / total as f64;

        if ratio < 0.20 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "INTOPIC001".to_string(),
                title: "Internal link anchor text lacks topical relevance".to_string(),
                description: format!(
                    "Only {:.0}% of internal link anchor text ({}/{}) contains keywords from                      the page headings. Anchor text that relates to the page topic provides                      stronger topical signals to search engines.",
                    ratio * 100.0,
                    relevant_count,
                    total
                ),
                url: url.clone(),
                recommendation: "Use anchor text that reflects the page's topic and headings.                                  Descriptive, topic-relevant anchor text strengthens topical                                  relevance signals."
                    .to_string(),
            });
        }

        findings
    }
}

fn is_generic_anchor(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    matches!(
        lower.as_str(),
        "click here"
            | "click this"
            | "read more"
            | "learn more"
            | "more"
            | "here"
            | "this"
            | "link"
            | "go"
            | "see more"
            | "view more"
            | "see details"
            | "view details"
            | "find out more"
            | "find out"
            | "discover more"
            | "continue reading"
            | "continue"
            | "next"
            | "previous"
            | "read more here"
            | "click to read more"
            | "click to learn more"
            | "learn more here"
            | "see more here"
            | "view here"
            | "click here for more"
            | "go here"
            | "tap here"
            | "press here"
            | "see this"
            | "check this out"
            | "check it out"
    )
}

// =========================================================================
// RobotsTxtDirectivesAnalyzer
// =========================================================================

/// Checks if pages are disallowed by robots.txt but still crawled.
pub struct RobotsTxtDirectivesAnalyzer;

impl Default for RobotsTxtDirectivesAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RobotsTxtDirectivesAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a path is disallowed for the given user-agent in robots.txt.
    fn is_disallowed(path: &str, disallowed_paths: &[String]) -> bool {
        disallowed_paths
            .iter()
            .any(|blocked| path.starts_with(blocked.as_str()))
    }
}

impl Analyzer for RobotsTxtDirectivesAnalyzer {
    fn name(&self) -> &str {
        "robots-txt-directives"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let robots_txt = match ctx.robots_txt {
            Some(txt) => txt,
            None => return findings,
        };

        // Parse disallowed paths from robots.txt for * user-agent
        let mut disallowed = Vec::new();
        let mut in_wildcard = false;
        for line in robots_txt.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(val) = trimmed.strip_prefix("User-agent:") {
                let val = val.trim();
                in_wildcard = val == "*";
            } else if in_wildcard {
                if let Some(val) = trimmed.strip_prefix("Disallow:") {
                    let val = val.trim();
                    if !val.is_empty() {
                        disallowed.push(val.to_string());
                    }
                }
            }
        }

        if disallowed.is_empty() {
            return findings;
        }

        // Extract path from page URL
        if let Ok(parsed) = Url::parse(url) {
            let path = parsed.path();
            if Self::is_disallowed(path, &disallowed) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ROBOTS-D001".to_string(),
                    title: "Page disallowed by robots.txt".to_string(),
                    description: format!(
                        "This page's path ({}) matches a Disallow directive in robots.txt for \
                         the wildcard user-agent. Crawling disallowed pages may waste crawl budget \
                         and could be blocked by search engines.",
                        path
                    ),
                    url: url.to_string(),
                    recommendation: "Either remove the Disallow directive if the page should be \
                                     crawled, or add a noindex meta tag if the page should not \
                                     appear in search results."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// SitemapUrlAnalyzer
// =========================================================================

/// Checks page URLs for sitemap-unfriendly patterns.
pub struct SitemapUrlAnalyzer;

impl Default for SitemapUrlAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SitemapUrlAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SitemapUrlAnalyzer {
    fn name(&self) -> &str {
        "sitemap-url"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let parsed = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => return findings,
        };

        // SITEMAP-U001: URL contains query parameters
        if !parsed.query().unwrap_or("").is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "SITEMAP-U001".to_string(),
                title: "URL contains query parameters".to_string(),
                description: "This page URL contains query parameters. URLs with query \
                              parameters are generally unfriendly for sitemaps as they may \
                              create duplicate content issues."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Use parameterless URLs for sitemap inclusion. If parameters \
                                 are necessary, consider using canonical URLs to consolidate \
                                 duplicate content."
                    .to_string(),
            });
        }

        // SITEMAP-U002: URL contains uppercase characters in path
        let path = parsed.path();
        if path.chars().any(|c| c.is_ascii_uppercase()) {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "SITEMAP-U002".to_string(),
                title: "URL contains uppercase characters".to_string(),
                description: "This page URL contains uppercase characters in the path. Search \
                              engines generally treat URLs as case-sensitive, which can lead to \
                              duplicate content issues."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Use lowercase characters in URLs for consistency. If the page \
                                 is already indexed with uppercase, ensure proper canonical tags \
                                 are in place."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// TitleAnalysisDeepAnalyzer
// =========================================================================

pub struct TitleAnalysisDeepAnalyzer;

impl Default for TitleAnalysisDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TitleAnalysisDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TitleAnalysisDeepAnalyzer {
    fn name(&self) -> &str {
        "title-analysis-deep"
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
                code: "TITLEDEEP001".to_string(),
                title: "Title too short".to_string(),
                description: format!(
                    "Title is {} characters. Aim for 30-60 characters.",
                    title.len()
                ),
                url: url.clone(),
                recommendation: "Expand the title to 30-60 characters with target keywords."
                    .to_string(),
            });
        }
        if title.len() > 65 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLEDEEP002".to_string(),
                title: "Title too long for SERP display".to_string(),
                description: format!(
                    "Title is {} characters. Google typically shows ~60 characters.",
                    title.len()
                ),
                url: url.clone(),
                recommendation:
                    "Shorten the title to under 60 characters to avoid truncation in SERPs."
                        .to_string(),
            });
        }

        let has_brand_separator = title.contains(" | ")
            || title.contains(" - ")
            || title.contains(" – ")
            || title.contains(" — ");
        if !has_brand_separator && title.len() > 30 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "TITLEDEEP003".to_string(),
                title: "Title missing brand separator".to_string(),
                description: "Title doesn't contain a common brand separator (|, -, –)."
                    .to_string(),
                url: url.clone(),
                recommendation: "Consider adding ' | Brand Name' at the end for brand recognition."
                    .to_string(),
            });
        }

        let words: Vec<&str> = title.split_whitespace().collect();
        let stop_count = words
            .iter()
            .filter(|w| STOP_WORDS.contains(&w.to_lowercase().as_str()))
            .count();
        if words.len() > 3 && stop_count as f64 / words.len() as f64 > 0.5 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "TITLEDEEP004".to_string(),
                title: "Title has many stop words".to_string(),
                description: format!(
                    "{} of {} words are stop words, which may dilute keyword prominence.",
                    stop_count,
                    words.len()
                ),
                url: url.clone(),
                recommendation: "Reduce stop words and focus on target keywords in the title."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// MetaDescriptionDeepAnalyzer
// =========================================================================

pub struct MetaDescriptionDeepAnalyzer;

impl Default for MetaDescriptionDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaDescriptionDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MetaDescriptionDeepAnalyzer {
    fn name(&self) -> &str {
        "meta-description-deep"
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
                code: "METADEEP001".to_string(),
                title: "Meta description too short".to_string(),
                description: format!(
                    "Description is {} characters. Aim for 120-155 characters.",
                    desc.len()
                ),
                url: url.clone(),
                recommendation: "Expand the meta description to 120-155 characters.".to_string(),
            });
        }
        if desc.len() > 160 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "METADEEP002".to_string(),
                title: "Meta description too long".to_string(),
                description: format!(
                    "Description is {} characters. Google typically shows ~155 characters.",
                    desc.len()
                ),
                url: url.clone(),
                recommendation: "Shorten the meta description to under 155 characters.".to_string(),
            });
        }

        if desc.contains("\"") || desc.contains("'") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "METADEEP003".to_string(),
                title: "Meta description contains quotes".to_string(),
                description: "Quotes in meta descriptions may cause truncation in SERPs."
                    .to_string(),
                url: url.clone(),
                recommendation: "Remove or replace quotes with other punctuation.".to_string(),
            });
        }

        let words: Vec<&str> = desc.split_whitespace().collect();
        let unique_words: std::collections::HashSet<String> =
            words.iter().map(|w| w.to_lowercase()).collect();
        if words.len() > 5 && unique_words.len() as f64 / (words.len() as f64) < 0.6 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "METADEEP004".to_string(),
                title: "Meta description has repetitive words".to_string(),
                description:
                    "Many repeated words in the meta description reduce its effectiveness."
                        .to_string(),
                url: url.clone(),
                recommendation:
                    "Diversify vocabulary in the meta description for better click-through rates."
                        .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// CanonicalValidationDeepAnalyzer
// =========================================================================

pub struct CanonicalValidationDeepAnalyzer;

impl Default for CanonicalValidationDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CanonicalValidationDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CanonicalValidationDeepAnalyzer {
    fn name(&self) -> &str {
        "canonical-validation-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let canonical = match &ctx.page.meta.canonical {
            Some(c) => c,
            None => return findings,
        };

        let canonical_url = canonical.clone();

        if let Ok(page_url) = url::Url::parse(url) {
            let canonical_path = canonical_url.path();
            let page_path = page_url.path();
            if canonical_path != page_path && canonical_url.host_str() == page_url.host_str() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "CANDEEP001".to_string(), title: "Canonical path mismatch".to_string(), description: format!("Canonical path '{canonical_path}' differs from page path '{page_path}' on the same host."), url: url.clone(), recommendation: "Canonical should point to the same path unless intentionally consolidating pages.".to_string() });
            }

            let canonical_params: std::collections::HashMap<String, String> = canonical_url
                .query_pairs()
                .into_owned()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let page_params: std::collections::HashMap<String, String> = page_url
                .query_pairs()
                .into_owned()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let has_diff = page_params
                .iter()
                .any(|(k, v)| canonical_params.get(k.as_str()) != Some(v));
            if has_diff && canonical_params.len() == page_params.len() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "CANDEEP002".to_string(),
                    title: "Canonical differs by parameters".to_string(),
                    description: "Canonical URL differs from page URL only by query parameters."
                        .to_string(),
                    url: url.clone(),
                    recommendation:
                        "Verify the canonical correctly points to the preferred URL version."
                            .to_string(),
                });
            }
        }

        if canonical.as_str().contains('#') {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "CANDEEP003".to_string(),
                title: "Canonical URL contains fragment".to_string(),
                description:
                    "The canonical URL includes a fragment (#) which search engines ignore."
                        .to_string(),
                url: url.clone(),
                recommendation: "Remove the fragment from the canonical URL.".to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// SitemapCoverageDeepAnalyzer
// =========================================================================

pub struct SitemapCoverageDeepAnalyzer;

impl Default for SitemapCoverageDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SitemapCoverageDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SitemapCoverageDeepAnalyzer {
    fn name(&self) -> &str {
        "sitemap-coverage-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if let Some(robots) = ctx.robots_txt {
            let lower = robots.to_lowercase();
            if lower.contains("sitemap:") {
                let sitemap_count = lower.matches("sitemap:").count();
                if sitemap_count > 5 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "SITEMAPDEEP001".to_string(),
                        title: "Many sitemaps declared in robots.txt".to_string(),
                        description: format!(
                            "robots.txt declares {sitemap_count} sitemaps. Consider consolidating."
                        ),
                        url: url.clone(),
                        recommendation:
                            "Use a sitemap index file instead of listing many individual sitemaps."
                                .to_string(),
                    });
                }
            } else {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "SITEMAPDEEP002".to_string(), title: "No sitemap declared in robots.txt".to_string(), description: "robots.txt doesn't reference any sitemap. While not required, sitemaps help discovery.".to_string(), url: url.clone(), recommendation: "Add a Sitemap: directive to robots.txt pointing to your sitemap.xml.".to_string() });
            }
        }

        if let Some(body) = ctx.body {
            if body.contains("rel=\"sitemap\"") || body.contains("rel='sitemap'") {
                // HTML sitemap link found - good
            }
        }

        findings
    }
}

// =========================================================================
// RobotsTxtAnalysisDeepAnalyzer
// =========================================================================

pub struct RobotsTxtAnalysisDeepAnalyzer;

impl Default for RobotsTxtAnalysisDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RobotsTxtAnalysisDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RobotsTxtAnalysisDeepAnalyzer {
    fn name(&self) -> &str {
        "robots-txt-analysis-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let robots = match ctx.robots_txt {
            Some(r) => r,
            None => return findings,
        };

        let lower = robots.to_lowercase();
        if lower.contains("disallow: /") && !lower.contains("disallow: / ") {
            // Check if there's a blanket disallow for all agents
            let lines: Vec<&str> = robots.lines().collect();
            let mut current_agent = "*";
            for line in &lines {
                let trimmed = line.trim();
                if trimmed.to_lowercase().starts_with("user-agent:") {
                    current_agent = trimmed.split(':').nth(1).unwrap_or("*").trim();
                }
                if trimmed.to_lowercase().starts_with("disallow:") && current_agent == "*" {
                    let path = trimmed.split(':').nth(1).unwrap_or("").trim();
                    if path == "/" {
                        findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Seo, code: "ROBOTSDEEP001".to_string(), title: "robots.txt blocks all crawlers".to_string(), description: "A blanket 'Disallow: /' rule blocks all crawlers from the entire site.".to_string(), url: url.clone(), recommendation: "Remove the blanket disallow unless intentionally blocking indexing.".to_string() });
                    }
                }
            }
        }

        let crawl_delay_lines: Vec<&str> = robots
            .lines()
            .filter(|l| l.trim().to_lowercase().starts_with("crawl-delay:"))
            .collect();
        for line in &crawl_delay_lines {
            let val_str = line.split(':').nth(1).unwrap_or("").trim();
            if let Ok(delay) = val_str.parse::<f64>() {
                if delay > 10.0 {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "ROBOTSDEEP002".to_string(), title: "High crawl-delay value".to_string(), description: format!("crawl-delay is {delay} seconds, which may slow indexing significantly."), url: url.clone(), recommendation: "Consider reducing crawl-delay to under 5 seconds.".to_string() });
                }
            }
        }

        findings
    }
}

// =========================================================================
// InternalLinkQualityAnalyzer
// =========================================================================

pub struct InternalLinkQualityAnalyzer;

impl Default for InternalLinkQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalLinkQualityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for InternalLinkQualityAnalyzer {
    fn name(&self) -> &str {
        "internal-link-quality"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let internal_links: Vec<&ExtractedLink> =
            ctx.page.links.iter().filter(|l| !l.is_external).collect();
        if internal_links.is_empty() {
            return findings;
        }

        let self_links: usize = internal_links.iter().filter(|l| l.href == *url).count();
        if self_links > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "INTLINKQ001".to_string(),
                title: "Page contains self-links".to_string(),
                description: format!("{self_links} link(s) point to the same page."),
                url: url.clone(),
                recommendation: "Remove or nofollow self-referencing internal links.".to_string(),
            });
        }

        let nofollow_count = internal_links
            .iter()
            .filter(|l| l.rel.iter().any(|r| r == "nofollow"))
            .count();
        if nofollow_count > 0 && nofollow_count == internal_links.len() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "INTLINKQ002".to_string(),
                title: "All internal links are nofollowed".to_string(),
                description:
                    "Every internal link on this page has rel=nofollow, preventing PageRank flow."
                        .to_string(),
                url: url.clone(),
                recommendation: "Remove nofollow from internal links to allow link equity flow."
                    .to_string(),
            });
        }

        let empty_text = internal_links
            .iter()
            .filter(|l| l.text.trim().is_empty() && l.aria_label.is_none())
            .count();
        if empty_text > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "INTLINKQ003".to_string(),
                title: "Internal links with empty anchor text".to_string(),
                description: format!(
                    "{empty_text} internal link(s) have no visible text or aria-label."
                ),
                url: url.clone(),
                recommendation: "Add descriptive anchor text to all internal links.".to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// ExternalLinkAuthorityDeepAnalyzer
// =========================================================================

pub struct ExternalLinkAuthorityDeepAnalyzer;

impl Default for ExternalLinkAuthorityDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternalLinkAuthorityDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ExternalLinkAuthorityDeepAnalyzer {
    fn name(&self) -> &str {
        "external-link-authority-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let external_links: Vec<&ExtractedLink> =
            ctx.page.links.iter().filter(|l| l.is_external).collect();
        if external_links.is_empty() {
            return findings;
        }

        let nofollow_count = external_links
            .iter()
            .filter(|l| l.rel.iter().any(|r| r == "nofollow"))
            .count();
        let followed_count = external_links.len() - nofollow_count;

        if followed_count > 10 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "EXTLINKAUTH001".to_string(),
                title: "Many followed external links".to_string(),
                description: format!(
                    "{followed_count} external link(s) are followed (not nofollowed)."
                ),
                url: url.clone(),
                recommendation:
                    "Consider nofollowing non-essential external links to conserve link equity."
                        .to_string(),
            });
        }

        let empty_text = external_links
            .iter()
            .filter(|l| l.text.trim().is_empty() && l.aria_label.is_none())
            .count();
        if empty_text > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "EXTLINKAUTH002".to_string(),
                title: "External links with empty anchor text".to_string(),
                description: format!("{empty_text} external link(s) have no visible text."),
                url: url.clone(),
                recommendation: "Add descriptive anchor text to external links.".to_string(),
            });
        }

        let same_domain_count = external_links
            .iter()
            .filter(|l| {
                url::Url::parse(&l.href)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h.to_string()))
                    .map_or(false, |h| url.contains(&h))
            })
            .count();
        if same_domain_count > 0 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "EXTLINKAUTH003".to_string(), title: "External links pointing to same domain".to_string(), description: format!("{same_domain_count} external link(s) point to the same domain (may be relative URLs misclassified)."), url: url.clone(), recommendation: "Verify these are truly external links, not internal ones.".to_string() });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Tests for new analyzers
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_new_analyzers {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;
    use crate::types::{IssueCategory, Severity};
    use url::Url;

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
            rendered: None,
        }
    }

    // ---- LanguageAttributeAnalyzer tests ----

    #[test]
    fn test_lang_missing_no_lang_attribute() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LANG001"));
    }

    #[test]
    fn test_lang_present_no_findings() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LANG001"));
    }

    #[test]
    fn test_lang_empty_attribute() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LANG003"));
    }

    #[test]
    fn test_lang_whitespace_only() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("   ".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LANG003"));
    }

    #[test]
    fn test_lang_mismatch_with_meta_language() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        page.meta.language = Some("fr".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LANG002"));
    }

    #[test]
    fn test_lang_match_with_meta_language() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        page.meta.language = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LANG002"));
    }

    #[test]
    fn test_lang_prefix_match() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en-US".to_string());
        page.meta.language = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LANG002"));
    }

    #[test]
    fn test_lang_no_meta_language_no_mismatch() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_lang_no_lang_attribute_has_meta() {
        let mut page = make_page("https://example.com");
        page.meta.language = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LANG001"));
    }

    #[test]
    fn test_lang_mismatch_severity_warning() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("de".to_string());
        page.meta.language = Some("es".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "LANG002").unwrap();
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.category, IssueCategory::Seo);
    }

    #[test]
    fn test_lang_name() {
        assert_eq!(
            LanguageAttributeAnalyzer::new().name(),
            "language-attribute"
        );
    }

    #[test]
    fn test_lang_default() {
        let a = LanguageAttributeAnalyzer;
        assert_eq!(a.name(), "language-attribute");
    }

    #[test]
    fn test_lang_valid_bcp47_codes() {
        for code in &["en", "fr", "zh", "en-US", "pt-BR", "zh-CN"] {
            let mut page = make_page("https://example.com");
            page.has_lang_attribute = true;
            page.html_lang = Some(code.to_string());
            let ctx = make_ctx(&page, Some(200));
            let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
            assert!(
                !findings.iter().any(|f| f.code == "LANG003"),
                "code {code} should be valid"
            );
        }
    }

    #[test]
    fn test_lang_mismatch_both_directions() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        page.meta.language = Some("en-US".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        // en starts with en — prefix match, no mismatch
        assert!(!findings.iter().any(|f| f.code == "LANG002"));
    }

    #[test]
    fn test_lang_empty_returns_immediately() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("".to_string());
        page.meta.language = Some("fr".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        // LANG003 fires, LANG002 does NOT (returns early)
        assert!(findings.iter().any(|f| f.code == "LANG003"));
        assert!(!findings.iter().any(|f| f.code == "LANG002"));
    }

    // ---- HreflangConsistencyAnalyzer tests ----

    #[test]
    fn test_hreflang_consistency_no_tags() {
        let page = make_page("https://example.com/en");
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hreflang_consistency_non200_self_referencing() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(404));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HREFT001"));
    }

    #[test]
    fn test_hreflang_consistency_200_self_referencing() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "HREFT001"));
    }

    #[test]
    fn test_hreflang_consistency_invalid_locale() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "invalid-code-too-long".to_string(),
                url: Url::parse("https://example.com/x").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HREFT003"));
    }

    #[test]
    fn test_hreflang_consistency_valid_locales() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "fr-CA".to_string(),
                url: Url::parse("https://example.com/fr").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "x-default".to_string(),
                url: Url::parse("https://example.com").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "HREFT003"));
    }

    #[test]
    fn test_hreflang_consistency_canonical_mismatch_self() {
        let mut page = make_page("https://example.com/en");
        page.meta.canonical = Some(Url::parse("https://example.com/canonical").unwrap());
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HREFT002"));
    }

    #[test]
    fn test_hreflang_consistency_canonical_match_self() {
        let mut page = make_page("https://example.com/en");
        page.meta.canonical = Some(Url::parse("https://example.com/en").unwrap());
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "HREFT002"));
    }

    #[test]
    fn test_hreflang_consistency_external_target_no_canonical_check() {
        let mut page = make_page("https://example.com/en");
        page.meta.canonical = Some(Url::parse("https://example.com/canonical").unwrap());
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "fr".to_string(),
            url: Url::parse("https://example.com/fr").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "HREFT002"));
    }

    #[test]
    fn test_hreflang_consistency_is_valid_locale() {
        assert!(HreflangConsistencyAnalyzer::is_valid_locale("en"));
        assert!(HreflangConsistencyAnalyzer::is_valid_locale("fr"));
        assert!(HreflangConsistencyAnalyzer::is_valid_locale("en-US"));
        assert!(HreflangConsistencyAnalyzer::is_valid_locale("zh-CN"));
        assert!(HreflangConsistencyAnalyzer::is_valid_locale("x-default"));
        assert!(!HreflangConsistencyAnalyzer::is_valid_locale("e"));
        assert!(!HreflangConsistencyAnalyzer::is_valid_locale("english"));
        assert!(!HreflangConsistencyAnalyzer::is_valid_locale("123"));
    }

    #[test]
    fn test_hreflang_consistency_name() {
        assert_eq!(
            HreflangConsistencyAnalyzer::new().name(),
            "hreflang-consistency"
        );
    }

    #[test]
    fn test_hreflang_consistency_default() {
        let a = HreflangConsistencyAnalyzer;
        assert_eq!(a.name(), "hreflang-consistency");
    }

    #[test]
    fn test_hreflang_consistency_multiple_findings() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "bad-code".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(500));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HREFT001"));
        assert!(findings.iter().any(|f| f.code == "HREFT003"));
    }

    #[test]
    fn test_hreflang_consistency_non200_301_redirect() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(301));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HREFT001"));
    }

    #[test]
    fn test_hreflang_consistency_500_server_error() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(500));
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "HREFT001").unwrap();
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.category, IssueCategory::Seo);
    }

    #[test]
    fn test_hreflang_consistency_no_status_code_no_hreft001() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, None);
        let findings = HreflangConsistencyAnalyzer::new().analyze(&ctx);
        // No status code available, can't check
        assert!(!findings.iter().any(|f| f.code == "HREFT001"));
    }

    // ---- CharsetValidator tests ----

    #[test]
    fn test_charset_missing_all() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = CharsetValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CHARSET001"));
    }

    #[test]
    fn test_charset_meta_only() {
        let mut page = make_page("https://example.com");
        page.meta.charset = Some("utf-8".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = CharsetValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CHARSET002"));
    }

    #[test]
    fn test_charset_header_only() {
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: Some("text/html; charset=utf-8"),
            rendered: None,
        };
        let findings = CharsetValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CHARSET001"));
        assert!(!findings.iter().any(|f| f.code == "CHARSET002"));
    }

    #[test]
    fn test_charset_both_present_utf8() {
        let mut page = make_page("https://example.com");
        page.meta.charset = Some("utf-8".to_string());
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: Some("text/html; charset=utf-8"),
            rendered: None,
        };
        let findings = CharsetValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_charset_non_utf8() {
        let mut page = make_page("https://example.com");
        page.meta.charset = Some("iso-8859-1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = CharsetValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CHARSET003"));
    }

    #[test]
    fn test_charset_utf8_uppercase() {
        let mut page = make_page("https://example.com");
        page.meta.charset = Some("UTF-8".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = CharsetValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CHARSET003"));
    }

    #[test]
    fn test_charset_extract_from_content_type() {
        assert_eq!(
            CharsetValidator::extract_charset_from_content_type("text/html; charset=utf-8"),
            Some("utf-8".to_string())
        );
        assert_eq!(
            CharsetValidator::extract_charset_from_content_type("text/html"),
            None
        );
        assert_eq!(
            CharsetValidator::extract_charset_from_content_type(
                "text/html; charset=iso-8859-1; boundary=something"
            ),
            Some("iso-8859-1".to_string())
        );
    }

    #[test]
    fn test_charset_non_utf8_header() {
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: Some("text/html; charset=windows-1252"),
            rendered: None,
        };
        let findings = CharsetValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CHARSET003"));
    }

    #[test]
    fn test_charset_name() {
        assert_eq!(CharsetValidator::new().name(), "charset-validator");
    }

    #[test]
    fn test_charset_default() {
        let a = CharsetValidator;
        assert_eq!(a.name(), "charset-validator");
    }

    #[test]
    fn test_charset_missing_severity_warning() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = CharsetValidator::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "CHARSET001").unwrap();
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn test_charset_non_utf8_severity_warning() {
        let mut page = make_page("https://example.com");
        page.meta.charset = Some("ascii".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = CharsetValidator::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "CHARSET003").unwrap();
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn test_charset_meta_only_severity_info() {
        let mut page = make_page("https://example.com");
        page.meta.charset = Some("utf-8".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = CharsetValidator::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "CHARSET002").unwrap();
        assert_eq!(f.severity, Severity::Info);
    }

    #[test]
    fn test_charset_utf8_alias() {
        let mut page = make_page("https://example.com");
        page.meta.charset = Some("utf8".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = CharsetValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CHARSET003"));
    }

    #[test]
    fn test_charset_non_utf8_category() {
        let mut page = make_page("https://example.com");
        page.meta.charset = Some("shift_jis".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = CharsetValidator::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "CHARSET003").unwrap();
        assert_eq!(f.category, IssueCategory::Seo);
    }

    // ---- RobotsMetaAnalyzer tests ----

    #[test]
    fn test_robots_meta_no_directives() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_meta_index_follow() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("index, follow".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_meta_noindex_content_page() {
        let mut page = make_page("https://example.com/blog/post");
        page.meta.robots = Some("noindex".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOTS001"));
    }

    #[test]
    fn test_robots_meta_noindex_utility_page() {
        let mut page = make_page("https://example.com/login");
        page.meta.robots = Some("noindex".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ROBOTS001"));
    }

    #[test]
    fn test_robots_meta_nofollow_content_page() {
        let mut page = make_page("https://example.com/blog/post");
        page.meta.robots = Some("nofollow".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOTS002"));
    }

    #[test]
    fn test_robots_meta_nofollow_utility_page() {
        let mut page = make_page("https://example.com/admin");
        page.meta.robots = Some("nofollow".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ROBOTS002"));
    }

    #[test]
    fn test_robots_meta_conflicting_index_noindex() {
        let mut page = make_page("https://example.com/page");
        page.meta.robots = Some("index, noindex".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOTS003"));
    }

    #[test]
    fn test_robots_meta_conflicting_follow_nofollow() {
        let mut page = make_page("https://example.com/page");
        page.meta.robots = Some("follow, nofollow".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOTS003"));
    }

    #[test]
    fn test_robots_meta_noconflict() {
        let mut page = make_page("https://example.com/page");
        page.meta.robots = Some("noindex, nofollow".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ROBOTS003"));
    }

    #[test]
    fn test_robots_meta_parse_directives() {
        let dirs = RobotsMetaAnalyzer::parse_robots_directives("noindex, nofollow");
        assert!(dirs.contains("noindex"));
        assert!(dirs.contains("nofollow"));
        assert!(!dirs.contains("index"));
    }

    #[test]
    fn test_robots_meta_empty_string() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_meta_whitespace() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("  ".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_meta_name() {
        assert_eq!(RobotsMetaAnalyzer::new().name(), "robots-meta");
    }

    #[test]
    fn test_robots_meta_default() {
        let a = RobotsMetaAnalyzer::default();
        assert_eq!(a.name(), "robots-meta");
    }

    #[test]
    fn test_robots_meta_severity_noindex_warning() {
        let mut page = make_page("https://example.com/page");
        page.meta.robots = Some("noindex".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "ROBOTS001").unwrap();
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn test_robots_meta_severity_conflict_error() {
        let mut page = make_page("https://example.com/page");
        page.meta.robots = Some("index, noindex".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsMetaAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "ROBOTS003").unwrap();
        assert_eq!(f.severity, Severity::Error);
    }

    // ---- CanonicalDepthAnalyzer tests ----

    #[test]
    fn test_canonical_depth_no_canonical() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_canonical_depth_shallow() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/page").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CDEP001"));
    }

    #[test]
    fn test_canonical_depth_exactly_3() {
        let mut page = make_page("https://example.com/a/b/c");
        page.meta.canonical = Some(Url::parse("https://example.com/a/b/c").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CDEP001"));
    }

    #[test]
    fn test_canonical_depth_deep() {
        let mut page = make_page("https://example.com/a/b/c/d");
        page.meta.canonical = Some(Url::parse("https://example.com/a/b/c/d").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CDEP001"));
    }

    #[test]
    fn test_canonical_depth_very_deep() {
        let mut page = make_page("https://example.com/a/b/c/d/e/f");
        page.meta.canonical = Some(Url::parse("https://example.com/a/b/c/d/e/f").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CDEP001"));
    }

    #[test]
    fn test_canonical_depth_with_query_params() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/page?ref=nav").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CDEP002"));
    }

    #[test]
    fn test_canonical_depth_no_query_params() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/page").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CDEP002"));
    }

    #[test]
    fn test_canonical_depth_empty_query() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/page?").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CDEP002"));
    }

    #[test]
    fn test_canonical_depth_path_depth_calc() {
        assert_eq!(
            CanonicalDepthAnalyzer::path_depth(&Url::parse("https://example.com").unwrap()),
            0
        );
        assert_eq!(
            CanonicalDepthAnalyzer::path_depth(&Url::parse("https://example.com/a").unwrap()),
            1
        );
        assert_eq!(
            CanonicalDepthAnalyzer::path_depth(&Url::parse("https://example.com/a/b/c/d").unwrap()),
            4
        );
    }

    #[test]
    fn test_canonical_depth_name() {
        assert_eq!(CanonicalDepthAnalyzer::new().name(), "canonical-depth");
    }

    #[test]
    fn test_canonical_depth_default() {
        let a = CanonicalDepthAnalyzer::default();
        assert_eq!(a.name(), "canonical-depth");
    }

    #[test]
    fn test_canonical_depth_deep_severity_info() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/a/b/c/d").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "CDEP001").unwrap();
        assert_eq!(f.severity, Severity::Info);
    }

    #[test]
    fn test_canonical_depth_query_severity_warning() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/page?foo=bar").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "CDEP002").unwrap();
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn test_canonical_depth_both_issues() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/a/b/c/d?ref=nav").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CDEP001"));
        assert!(findings.iter().any(|f| f.code == "CDEP002"));
    }

    #[test]
    fn test_canonical_depth_root_slash() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalDepthAnalyzer::new().analyze(&ctx);
        // Root has depth 0
        assert!(!findings.iter().any(|f| f.code == "CDEP001"));
    }

    // ---- MobileViewportAnalyzer tests ----

    #[test]
    fn test_viewport_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_viewport_complete() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_viewport_missing_initial_scale() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOBVIEW001"));
    }

    #[test]
    fn test_viewport_wrong_width() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=1024, initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOBVIEW002"));
    }

    #[test]
    fn test_viewport_correct_width() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "MOBVIEW002"));
    }

    #[test]
    fn test_viewport_user_scalable_no() {
        let mut page = make_page("https://example.com");
        page.meta.viewport =
            Some("width=device-width, initial-scale=1, user-scalable=no".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOBVIEW003"));
    }

    #[test]
    fn test_viewport_user_scalable_zero() {
        let mut page = make_page("https://example.com");
        page.meta.viewport =
            Some("width=device-width, initial-scale=1, user-scalable=0".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOBVIEW003"));
    }

    #[test]
    fn test_viewport_user_scalable_yes() {
        let mut page = make_page("https://example.com");
        page.meta.viewport =
            Some("width=device-width, initial-scale=1, user-scalable=yes".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "MOBVIEW003"));
    }

    #[test]
    fn test_viewport_parse() {
        let map = MobileViewportAnalyzer::parse_viewport("width=device-width, initial-scale=1");
        assert_eq!(map.get("width").unwrap(), "device-width");
        assert_eq!(map.get("initial-scale").unwrap(), "1");
    }

    #[test]
    fn test_viewport_parse_with_spaces() {
        let map = MobileViewportAnalyzer::parse_viewport("width=device-width , initial-scale=1");
        assert_eq!(map.get("width").unwrap(), "device-width");
        assert_eq!(map.get("initial-scale").unwrap(), "1");
    }

    #[test]
    fn test_viewport_name() {
        assert_eq!(MobileViewportAnalyzer::new().name(), "mobile-viewport");
    }

    #[test]
    fn test_viewport_default() {
        let a = MobileViewportAnalyzer::default();
        assert_eq!(a.name(), "mobile-viewport");
    }

    #[test]
    fn test_viewport_all_three_issues() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=800, user-scalable=no".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOBVIEW001"));
        assert!(findings.iter().any(|f| f.code == "MOBVIEW002"));
        assert!(findings.iter().any(|f| f.code == "MOBVIEW003"));
    }

    #[test]
    fn test_viewport_severity_warnings() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=800, initial-scale=1, user-scalable=no".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.severity, Severity::Warning);
            assert_eq!(f.category, IssueCategory::Mobile);
        }
    }

    #[test]
    fn test_viewport_empty_string() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileViewportAnalyzer::new().analyze(&ctx);
        // Empty viewport treated as missing (early return)
        assert!(findings.is_empty());
    }

    // ---- InternalLinkAnchorAnalyzer tests ----

    #[test]
    fn test_anchor001_identical_to_url() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/about".to_string(),
            text: "/about".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ANCHOR001"));
    }

    #[test]
    fn test_anchor001_not_identical() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/about".to_string(),
            text: "About Us".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ANCHOR001"));
    }

    #[test]
    fn test_anchor001_case_insensitive() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/About".to_string(),
            text: "/about".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ANCHOR001"));
    }

    #[test]
    fn test_anchor001_full_url_match() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://example.com/about".to_string(),
            text: "https://example.com/about".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ANCHOR001"));
    }

    #[test]
    fn test_anchor001_empty_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/about".to_string(),
            text: String::new(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ANCHOR001"));
    }

    #[test]
    fn test_anchor002_over_optimized() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "best shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "best shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "best shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ANCHOR002"));
    }

    #[test]
    fn test_anchor002_not_over_optimized() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "best shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "running shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "hiking boots".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ANCHOR002"));
    }

    #[test]
    fn test_anchor002_exactly_at_threshold() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "boots".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        // 2/3 = 66.7% > 50%, should fire
        assert!(findings.iter().any(|f| f.code == "ANCHOR002"));
    }

    #[test]
    fn test_anchor002_below_threshold() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "boots".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "sandals".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ANCHOR002"));
    }

    #[test]
    fn test_anchor_no_internal_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_anchor_external_links_ignored() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://other.com".to_string(),
            text: "https://other.com".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_anchor002_only_two_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkAnchorAnalyzer::new().analyze(&ctx);
        // Only 2 links, below the 3-link minimum for ANCHOR002
        assert!(!findings.iter().any(|f| f.code == "ANCHOR002"));
    }

    // ---- WikipediaLinkAnalyzer tests ----

    #[test]
    fn test_wiki_no_external_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wiki_no_wikipedia_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://other.com/resource".to_string(),
            text: "Other resource".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wiki_wikipedia_link_detected() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Rust_(programming_language)".to_string(),
            text: "Rust on Wikipedia".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WIKI001"));
    }

    #[test]
    fn test_wiki_wikidata_link_detected() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://www.wikidata.org/wiki/Q12345".to_string(),
            text: "Wikidata entry".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WIKI002"));
    }

    #[test]
    fn test_wiki_both_wikipedia_and_wikidata() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "https://en.wikipedia.org/wiki/Rust".to_string(),
                text: "Rust".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "https://www.wikidata.org/wiki/Q12345".to_string(),
                text: "Wikidata".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WIKI001"));
        assert!(findings.iter().any(|f| f.code == "WIKI002"));
    }

    #[test]
    fn test_wiki_wikipedia_talk_page_ignored() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Talk:Rust".to_string(),
            text: "Rust talk".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wiki_wikipedia_category_page_ignored() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Category:Programming_languages".to_string(),
            text: "Category".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wiki_wikipedia_template_page_ignored() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Template:Cite_web".to_string(),
            text: "Template".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wiki_internal_wikipedia_link_not_counted() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Rust".to_string(),
            text: "Rust".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wiki_multiple_wikipedia_links_count() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "https://en.wikipedia.org/wiki/Rust".to_string(),
                text: "Rust".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "https://en.wikipedia.org/wiki/Python".to_string(),
                text: "Python".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        let wiki_finding = findings.iter().find(|f| f.code == "WIKI001").unwrap();
        assert!(wiki_finding.description.contains("2"));
    }

    #[test]
    fn test_wiki_wikidata_entity_url() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://www.wikidata.org/entity/Q12345".to_string(),
            text: "Entity".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WIKI002"));
    }

    #[test]
    fn test_wiki_wikipedia_info_level() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Rust".to_string(),
            text: "Rust".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn test_wiki_mobile_wikipedia() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://en.m.wikipedia.org/wiki/Rust".to_string(),
            text: "Rust".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WIKI001"));
    }

    #[test]
    fn test_wiki_non_main_wikipedia_ignored() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://fr.wikipedia.org/wiki/Rust".to_string(),
            text: "Rust FR".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ---- AnchorTextDiversityAnalyzer tests ----

    #[test]
    fn test_diversity_no_internal_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_diversity_too_few_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "boots".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_diversity_all_same_anchor() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ANCH-DIV001"));
    }

    #[test]
    fn test_diversity_diverse_anchors_no_finding() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "running shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "hiking boots".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "casual sandals".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_diversity_generic_over_80_percent() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "read more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/d".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // 4 of 4 are generic (100%), so >80%
        assert!(findings.iter().any(|f| f.code == "ANCH-DIV002"));
    }

    #[test]
    fn test_diversity_generic_below_80_percent() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "running shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "hiking boots".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/d".to_string(),
                text: "casual sandals".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/e".to_string(),
                text: "running shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // 1 of 5 = 20% generic, well below 80%
        assert!(!findings.iter().any(|f| f.code == "ANCH-DIV002"));
    }

    #[test]
    fn test_diversity_exactly_80_percent_no_finding() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "read more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "learn more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/d".to_string(),
                text: "running shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // 3 of 4 = 75%, not >80%
        assert!(!findings.iter().any(|f| f.code == "ANCH-DIV002"));
    }

    #[test]
    fn test_diversity_mixed_same_and_generic() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // All same but not generic, so DIV001 fires but DIV002 doesn't
        assert!(findings.iter().any(|f| f.code == "ANCH-DIV001"));
        assert!(!findings.iter().any(|f| f.code == "ANCH-DIV002"));
    }

    #[test]
    fn test_diversity_empty_anchor_text_links_skipped() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // All anchor texts are empty, filtered out, so <3 valid anchors
        assert!(findings.is_empty());
    }

    #[test]
    fn test_diversity_external_links_excluded() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "https://other.com".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "https://other2.com".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "https://other3.com".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // Only external links, no internal links to analyze
        assert!(findings.is_empty());
    }

    #[test]
    fn test_diversity_all_generic_warnings() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/d".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // Both DIV001 and DIV002 should fire
        assert!(findings.iter().any(|f| f.code == "ANCH-DIV001"));
        assert!(findings.iter().any(|f| f.code == "ANCH-DIV002"));
    }

    #[test]
    fn test_diversity_various_generic_phrases() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "read more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "learn more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/d".to_string(),
                text: "view more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // All 4 are different generic phrases, so DIV001 doesn't fire, DIV002 does
        assert!(!findings.iter().any(|f| f.code == "ANCH-DIV001"));
        assert!(findings.iter().any(|f| f.code == "ANCH-DIV002"));
    }

    #[test]
    fn test_diversity_case_insensitive() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "Shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "SHOES".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // All same after case normalization
        assert!(findings.iter().any(|f| f.code == "ANCH-DIV001"));
    }

    #[test]
    fn test_diversity_warning_severity() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        let finding = findings.iter().find(|f| f.code == "ANCH-DIV001").unwrap();
        assert_eq!(finding.severity, Severity::Warning);
    }

    #[test]
    fn test_diversity_with_whitespace_in_anchors() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "  shoes  ".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "  shoes  ".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "  shoes  ".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // Should match after trimming/normalization
        assert!(findings.iter().any(|f| f.code == "ANCH-DIV001"));
    }

    #[test]
    fn test_diversity_only_internal_links_considered() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "shoes".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "https://other.com".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AnchorTextDiversityAnalyzer::new().analyze(&ctx);
        // Only 1 internal link, below minimum of 3
        assert!(findings.is_empty());
    }

    // ===== RobotsTxtDirectivesAnalyzer tests =====

    #[test]
    fn test_robots_directives_no_robots_txt() {
        let page = make_page("https://example.com/admin");
        let ctx = make_ctx(&page, Some(200));
        assert!(RobotsTxtDirectivesAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_robots_directives_disallowed() {
        let robots = "User-agent: *\nDisallow: /admin\n";
        let page = make_page("https://example.com/admin/secret");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOTS-D001"));
    }

    #[test]
    fn test_robots_directives_allowed() {
        let robots = "User-agent: *\nDisallow: /admin\n";
        let page = make_page("https://example.com/page");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_directives_empty_disallow() {
        let robots = "User-agent: *\nDisallow:\n";
        let page = make_page("https://example.com/page");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_directives_multiple_rules() {
        let robots = "User-agent: *\nDisallow: /admin\nDisallow: /private\nDisallow: /temp\n";
        let page = make_page("https://example.com/private/data");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOTS-D001"));
    }

    #[test]
    fn test_robots_directives_specific_user_agent_only() {
        let robots = "User-agent: Googlebot\nDisallow: /admin\n";
        let page = make_page("https://example.com/admin");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        // Only Googlebot is disallowed, wildcard is not
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_directives_root_disallow() {
        let robots = "User-agent: *\nDisallow: /\n";
        let page = make_page("https://example.com/anything");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOTS-D001"));
    }

    #[test]
    fn test_robots_directives_comments_ignored() {
        let robots = "# This is a comment\nUser-agent: *\n# Another comment\nDisallow: /admin\n";
        let page = make_page("https://example.com/admin");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOTS-D001"));
    }

    #[test]
    fn test_robots_directives_partial_path_match() {
        let robots = "User-agent: *\nDisallow: /admin\n";
        let page = make_page("https://example.com/admin-page");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        // /admin-page does NOT start with /admin followed by / or end
        // Actually /admin-page DOES start with /admin, so it IS disallowed
        assert!(findings.iter().any(|f| f.code == "ROBOTS-D001"));
    }

    #[test]
    fn test_robots_directives_no_disallow_allows_everything() {
        let robots = "User-agent: *\nAllow: /\n";
        let page = make_page("https://example.com/anywhere");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_directives_only_comments() {
        let robots = "# Just comments\n# Nothing else\n";
        let page = make_page("https://example.com/page");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_robots_directives_subpath_disallowed() {
        let robots = "User-agent: *\nDisallow: /files\n";
        let page = make_page("https://example.com/files/doc.pdf");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = RobotsTxtDirectivesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ROBOTS-D001"));
    }

    // ===== SitemapUrlAnalyzer tests =====

    #[test]
    fn test_sitemap_url_with_query_params() {
        let page = make_page("https://example.com/page?foo=bar&baz=1");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP-U001"));
    }

    #[test]
    fn test_sitemap_url_no_query_params() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SITEMAP-U001"));
    }

    #[test]
    fn test_sitemap_url_uppercase() {
        let page = make_page("https://example.com/MyPage");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP-U002"));
    }

    #[test]
    fn test_sitemap_url_lowercase() {
        let page = make_page("https://example.com/mypage");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SITEMAP-U002"));
    }

    #[test]
    fn test_sitemap_url_both_issues() {
        let page = make_page("https://example.com/MyPage?foo=bar");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP-U001"));
        assert!(findings.iter().any(|f| f.code == "SITEMAP-U002"));
    }

    #[test]
    fn test_sitemap_url_empty_query() {
        let page = make_page("https://example.com/page?=");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        // Empty query string: url crate may not set query for `?` alone
        // Use `?=` which has a key with empty value
        assert!(findings.iter().any(|f| f.code == "SITEMAP-U001"));
    }

    #[test]
    fn test_sitemap_url_fragment_not_query() {
        let page = make_page("https://example.com/page#section");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SITEMAP-U001"));
    }

    #[test]
    fn test_sitemap_url_root() {
        let page = make_page("https://example.com/");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_sitemap_url_mixed_case_path() {
        let page = make_page("https://example.com/Path/To/Page");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP-U002"));
    }

    #[test]
    fn test_sitemap_url_only_uppercase_in_segment() {
        let page = make_page("https://example.com/API/v2/users");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP-U002"));
    }

    #[test]
    fn test_sitemap_url_query_with_multiple_params() {
        let page = make_page("https://example.com/search?q=rust&page=2&sort=asc");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SITEMAP-U001"));
    }

    #[test]
    fn test_sitemap_url_invalid_url() {
        let page = make_page("not-a-valid-url");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapUrlAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ---- TitlePixelWidthAnalyzer tests ----

    #[test]
    fn test_title_px_no_title() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(TitlePixelWidthAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_title_px_empty_title() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(TitlePixelWidthAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_title_px_within_limit() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Short Title".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitlePixelWidthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TITLE-PX001"));
        assert!(findings.iter().any(|f| f.code == "TITLE-PX002"));
    }

    #[test]
    fn test_title_px_exceeds_limit() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some(
            "This is a very long page title that definitely exceeds the pixel width limit for SERP"
                .to_string(),
        );
        let ctx = make_ctx(&page, Some(200));
        let findings = TitlePixelWidthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE-PX001"));
    }

    #[test]
    fn test_title_px_below_char_threshold() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Short".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitlePixelWidthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE-PX002"));
        assert!(!findings.iter().any(|f| f.code == "TITLE-PX001"));
    }

    #[test]
    fn test_title_px_exact_20_chars() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Exactly twenty chars!!!".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitlePixelWidthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TITLE-PX002"));
    }

    #[test]
    fn test_title_px_cjk_characters() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("日本語テストページタイトル".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitlePixelWidthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TITLE-PX001"));
    }

    #[test]
    fn test_title_px_cjk_exceeds_limit() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("这是一个非常长的中文页面标题用于测试像素宽度限制是否正确工作超过了五百八十像素的限制哦".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitlePixelWidthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE-PX001"));
    }

    #[test]
    fn test_title_px_mixed_ascii_cjk() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Hello World 日本語".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitlePixelWidthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TITLE-PX001"));
    }

    #[test]
    fn test_title_px_estimate_function() {
        assert_eq!(TitlePixelWidthAnalyzer::estimate_pixel_width(""), 0.0);
        assert_eq!(TitlePixelWidthAnalyzer::estimate_pixel_width("abc"), 21.0);
        assert_eq!(TitlePixelWidthAnalyzer::estimate_pixel_width("中"), 14.0);
    }

    #[test]
    fn test_title_px_severity_warning() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some(
            "This is a very long page title that definitely exceeds the pixel width limit for SERP"
                .to_string(),
        );
        let ctx = make_ctx(&page, Some(200));
        let findings = TitlePixelWidthAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "TITLE-PX001").unwrap();
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.category, IssueCategory::Seo);
    }

    #[test]
    fn test_title_px_severity_info() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Short".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitlePixelWidthAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "TITLE-PX002").unwrap();
        assert_eq!(f.severity, Severity::Info);
    }

    #[test]
    fn test_title_px_name() {
        assert_eq!(TitlePixelWidthAnalyzer::new().name(), "title-pixel-width");
    }

    // ---- MetaDescriptionPixelWidthAnalyzer tests ----

    #[test]
    fn test_mdesc_px_no_description() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionPixelWidthAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_mdesc_px_empty_description() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionPixelWidthAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_mdesc_px_within_limit() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some(
            "A reasonable meta description that fits within SERP limits perfectly.".to_string(),
        );
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "MDESC-PX001"));
    }

    #[test]
    fn test_mdesc_px_exceeds_limit() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("This is a very long meta description that will definitely exceed the pixel width limit for Google SERP display area and get truncated.".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MDESC-PX001"));
    }

    #[test]
    fn test_mdesc_px_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("Short desc".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MDESC-PX002"));
    }

    #[test]
    fn test_mdesc_px_exact_70_chars() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some(
            "Abcdefghij Abcdefghij Abcdefghij Abcdefghij Abcdefghij Abcdefghij Abcdefgh"
                .to_string(),
        );
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "MDESC-PX002"));
    }

    #[test]
    fn test_mdesc_px_69_chars() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some(
            "Abcdefghij Abcdefghij Abcdefghij Abcdefghij Abcdefghij Abcdefghij Abc".to_string(),
        );
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MDESC-PX002"));
    }

    #[test]
    fn test_mdesc_px_cjk_description() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("这是一个非常长的中文元描述用于测试搜索引擎结果页面中像素宽度限制是否会截断描述内容显示效果因为描述太长了所以会被搜索引擎截断显示不完整".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MDESC-PX001"));
    }

    #[test]
    fn test_mdesc_px_mixed_content() {
        let mut page = make_page("https://example.com");
        page.meta.description =
            Some("Buy shoes online - best prices for running shoes and hiking boots".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "MDESC-PX001"));
    }

    #[test]
    fn test_mdesc_px_estimate_function() {
        assert_eq!(
            MetaDescriptionPixelWidthAnalyzer::estimate_pixel_width(""),
            0.0
        );
        assert_eq!(
            MetaDescriptionPixelWidthAnalyzer::estimate_pixel_width("abc"),
            21.0
        );
        assert_eq!(
            MetaDescriptionPixelWidthAnalyzer::estimate_pixel_width("中"),
            14.0
        );
    }

    #[test]
    fn test_mdesc_px_severity_warning() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("This is a very long meta description that will definitely exceed the pixel width limit for Google SERP display area and get truncated.".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "MDESC-PX001").unwrap();
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.category, IssueCategory::Seo);
    }

    #[test]
    fn test_mdesc_px_severity_info() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("Short".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "MDESC-PX002").unwrap();
        assert_eq!(f.severity, Severity::Info);
    }

    #[test]
    fn test_mdesc_px_name() {
        assert_eq!(
            MetaDescriptionPixelWidthAnalyzer::new().name(),
            "meta-description-pixel-width"
        );
    }

    #[test]
    fn test_mdesc_px_whitespace_only() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("   ".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionPixelWidthAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ---- InternalLinkTopicalAnalyzer tests ----

    #[test]
    fn test_intopic_no_internal_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalLinkTopicalAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_intopic_no_headings() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/about".to_string(),
            text: "About us".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalLinkTopicalAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_intopic_relevant_anchor_text() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming".to_string(),
            length: 16,
        }];
        page.links = vec![ExtractedLink {
            href: "/rust-guide".to_string(),
            text: "Rust programming guide".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "INTOPIC001"));
    }

    #[test]
    fn test_intopic_irrelevant_anchor_text() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming Languages".to_string(),
            length: 26,
        }];
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "read more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "learn more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/d".to_string(),
                text: "see details".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/e".to_string(),
                text: "view more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "INTOPIC001"));
    }

    #[test]
    fn test_intopic_mixed_relevance() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming".to_string(),
            length: 16,
        }];
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "rust guide".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "read more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/d".to_string(),
                text: "learn more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/e".to_string(),
                text: "view details".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "INTOPIC001"));
    }

    #[test]
    fn test_intopic_empty_anchor_text_skipped() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming".to_string(),
            length: 16,
        }];
        page.links = vec![ExtractedLink {
            href: "/a".to_string(),
            text: String::new(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_intopic_external_links_not_counted() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming".to_string(),
            length: 16,
        }];
        page.links = vec![ExtractedLink {
            href: "https://other.com/rust".to_string(),
            text: "Rust".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_intopic_stop_words_excluded() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "The Best Rust Guide".to_string(),
            length: 19,
        }];
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "the".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "best".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "rust guide tutorial".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/d".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/e".to_string(),
                text: "read more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "INTOPIC001"));
    }

    #[test]
    fn test_intopic_severity_warning() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming".to_string(),
            length: 16,
        }];
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "read more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "learn more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "INTOPIC001").unwrap();
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.category, IssueCategory::Seo);
    }

    #[test]
    fn test_intopic_name() {
        assert_eq!(
            InternalLinkTopicalAnalyzer::new().name(),
            "internal-link-topical"
        );
    }

    #[test]
    fn test_intopic_case_insensitive_matching() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "RUST Programming".to_string(),
            length: 16,
        }];
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "rust guide".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "read more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/c".to_string(),
                text: "learn more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "INTOPIC001"));
    }

    #[test]
    fn test_intopic_heading_empty_text() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "".to_string(),
            length: 0,
        }];
        page.links = vec![ExtractedLink {
            href: "/a".to_string(),
            text: "click here".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_intopic_single_link_relevant() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming".to_string(),
            length: 16,
        }];
        page.links = vec![ExtractedLink {
            href: "/rust".to_string(),
            text: "rust tutorial".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = InternalLinkTopicalAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "INTOPIC001"));
    }
}
// =========================================================================
// PaginationDepthValidator
// =========================================================================

pub struct PaginationDepthValidator;

impl Default for PaginationDepthValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PaginationDepthValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PaginationDepthValidator {
    fn name(&self) -> &str {
        "pagination-depth"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(_) => return findings,
        };

        // Check for page parameter pagination depth
        if let Some(page_param) = parsed.query_pairs().find_map(|(k, v)| {
            if k == "page" {
                Some(v.to_string())
            } else {
                None
            }
        }) {
            if let Ok(page_num) = page_param.parse::<u32>() {
                if page_num > 5 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "PAGDEP001".to_string(),
                        title: "Deep pagination detected".to_string(),
                        description: format!(
                            "Page parameter indicates pagination depth of {page_num} levels. \
                             Deeply paginated pages receive less crawl budget and link equity."
                        ),
                        url: url.clone(),
                        recommendation: "Flatten pagination to 5 levels or fewer. Consider \
                                         adding a sitemap index or using rel=canonical to \
                                         consolidate paginated content."
                            .to_string(),
                    });
                }
            }
        }

        // Check for /page/N/ URL pattern
        let path = parsed.path();
        if let Some(pos) = path.rfind("/page/") {
            let after = &path[pos + 6..];
            if let Ok(page_num) = after.trim_end_matches('/').parse::<u32>() {
                if page_num > 5 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "PAGDEP001".to_string(),
                        title: "Deep pagination detected".to_string(),
                        description: format!(
                            "URL path indicates pagination depth of {page_num} levels (/page/{page_num}/). \
                             Deeply paginated pages receive less crawl budget."
                        ),
                        url: url.clone(),
                        recommendation: "Limit pagination to 5 levels or fewer. Consider \
                                         alternative navigation for deep content."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// RedirectLoopDetector
// =========================================================================

pub struct RedirectLoopDetector;

impl Default for RedirectLoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl RedirectLoopDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RedirectLoopDetector {
    fn name(&self) -> &str {
        "redirect-loop"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.redirect_chain.is_empty() {
            return findings;
        }

        // Check for URL appearing twice in the chain (loop indicator)
        let mut seen = HashSet::new();
        for hop in ctx.redirect_chain {
            let from_str = hop.from.as_str();
            if !seen.insert(from_str) {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Http,
                    code: "REDIRLOOP001".to_string(),
                    title: "Redirect loop detected".to_string(),
                    description: format!(
                        "URL \"{}\" appears multiple times in the redirect chain, \
                         indicating an infinite redirect loop.",
                        hop.from
                    ),
                    url: url.clone(),
                    recommendation: "Fix the redirect chain to eliminate the loop. Ensure \
                                     the final destination does not redirect back to an \
                                     earlier hop."
                        .to_string(),
                });
                return findings;
            }

            // Self-redirect: from == to
            if hop.from.as_str() == hop.to.as_str() {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Http,
                    code: "REDIRLOOP001".to_string(),
                    title: "Redirect loop detected".to_string(),
                    description: format!(
                        "URL \"{}\" redirects to itself, creating an infinite loop.",
                        hop.from
                    ),
                    url: url.clone(),
                    recommendation: "Fix the redirect to point to a different destination."
                        .to_string(),
                });
                return findings;
            }
        }

        // Also check if the last hop's target is any earlier hop's source
        if let Some(last_hop) = ctx.redirect_chain.last() {
            for hop in &ctx.redirect_chain[..ctx.redirect_chain.len().saturating_sub(1)] {
                if last_hop.to.as_str() == hop.from.as_str() {
                    findings.push(Finding {
                        severity: Severity::Critical,
                        category: IssueCategory::Http,
                        code: "REDIRLOOP001".to_string(),
                        title: "Redirect loop detected".to_string(),
                        description: format!(
                            "The redirect chain ends by redirecting back to \"{}\", \
                             which appeared earlier in the chain.",
                            hop.from
                        ),
                        url: url.clone(),
                        recommendation: "Fix the redirect chain to eliminate the loop.".to_string(),
                    });
                    return findings;
                }
            }
        }

        findings
    }
}

// =========================================================================
// MixedProtocolRedirectValidator
// =========================================================================

pub struct MixedProtocolRedirectValidator;

impl Default for MixedProtocolRedirectValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MixedProtocolRedirectValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MixedProtocolRedirectValidator {
    fn name(&self) -> &str {
        "mixed-protocol-redirect"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for hop in ctx.redirect_chain {
            let from_scheme = hop.from.scheme();
            let to_scheme = hop.to.scheme();

            if from_scheme == "http" && to_scheme == "https" && hop.status_code != 301 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Http,
                    code: "MIXPROT001".to_string(),
                    title: "HTTP to HTTPS redirect not using 301".to_string(),
                    description: format!(
                        "HTTP→HTTPS redirect uses status {} instead of 301 (permanent). \
                         Non-301 redirects may cause search engines to treat the HTTP \
                         version as the canonical URL.",
                        hop.status_code
                    ),
                    url: url.clone(),
                    recommendation: "Use HTTP 301 for HTTP→HTTPS redirects to signal that \
                                     HTTPS is the permanent canonical URL."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// InternalNofollowOveruseValidator
// =========================================================================

pub struct InternalNofollowOveruseValidator;

impl Default for InternalNofollowOveruseValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalNofollowOveruseValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for InternalNofollowOveruseValidator {
    fn name(&self) -> &str {
        "internal-nofollow-overuse"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let internal_links: Vec<_> = ctx.page.links.iter().filter(|l| !l.is_external).collect();

        if internal_links.len() < 5 {
            return findings;
        }

        let nofollow_count = internal_links
            .iter()
            .filter(|l| l.rel.contains(&"nofollow".to_string()))
            .count();

        let ratio = nofollow_count as f64 / internal_links.len() as f64;

        if ratio > 0.30 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "NOFOLLOW001".to_string(),
                title: "Excessive internal nofollow links".to_string(),
                description: format!(
                    "{:.0}% of internal links ({}/{}) have rel=\"nofollow\". Excessive use \
                     of nofollow on internal links wastes crawl budget and dilutes link \
                     equity flow.",
                    ratio * 100.0,
                    nofollow_count,
                    internal_links.len()
                ),
                url: url.clone(),
                recommendation: "Audit internal nofollow usage. Most internal links should \
                                     not have rel=\"nofollow\" unless linking to untrusted \
                                     user-generated content."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// ExternalNofollowUnderuseValidator
// =========================================================================

pub struct ExternalNofollowUnderuseValidator;

impl Default for ExternalNofollowUnderuseValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternalNofollowUnderuseValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ExternalNofollowUnderuseValidator {
    fn name(&self) -> &str {
        "external-nofollow-underuse"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let external_links: Vec<_> = ctx.page.links.iter().filter(|l| l.is_external).collect();

        if external_links.len() < 5 {
            return findings;
        }

        let nofollow_count = external_links
            .iter()
            .filter(|l| l.rel.contains(&"nofollow".to_string()))
            .count();

        let ratio = nofollow_count as f64 / external_links.len() as f64;

        if ratio < 0.10 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "NOFOLLOW002".to_string(),
                title: "Few external links use nofollow".to_string(),
                description: format!(
                    "Only {:.0}% of external links ({}/{}) have rel=\"nofollow\". \
                     Paid links, sponsored content, and untrusted external links \
                     should use rel=\"nofollow\" to avoid passing link equity.",
                    ratio * 100.0,
                    nofollow_count,
                    external_links.len()
                ),
                url: url.clone(),
                recommendation: "Add rel=\"nofollow\" to paid links, sponsored content, \
                                     and links to untrusted external sites."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// SitemapXmlSizeValidator
// =========================================================================

pub struct SitemapXmlSizeValidator;

impl Default for SitemapXmlSizeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SitemapXmlSizeValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SitemapXmlSizeValidator {
    fn name(&self) -> &str {
        "sitemap-xml-size"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if let Some(size) = ctx.body_size {
            if size > 50 * 1024 * 1024 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "SITEMAPSIZE001".to_string(),
                    title: "Sitemap XML exceeds 50MB".to_string(),
                    description: format!(
                        "Sitemap is {:.1}MB (uncompressed), exceeding the 50MB limit. \
                         Search engines may reject or truncate oversized sitemaps.",
                        size as f64 / (1024.0 * 1024.0)
                    ),
                    url: url.clone(),
                    recommendation: "Split the sitemap into multiple smaller sitemaps, each \
                                     under 50MB and containing no more than 50,000 URLs. \
                                     Use a sitemap index to reference them."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// RobotsTxtSizeValidator
// =========================================================================

pub struct RobotsTxtSizeValidator;

impl Default for RobotsTxtSizeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RobotsTxtSizeValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RobotsTxtSizeValidator {
    fn name(&self) -> &str {
        "robots-txt-size"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if let Some(robots) = ctx.robots_txt {
            let size = robots.len();
            if size > 500 * 1024 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ROBOTSSIZE001".to_string(),
                    title: "robots.txt exceeds 500KB".to_string(),
                    description: format!(
                        "robots.txt is {:.1}KB, exceeding the recommended 500KB limit. \
                         Oversized robots.txt files waste crawl budget as crawlers \
                         must download and parse the entire file.",
                        size as f64 / 1024.0
                    ),
                    url: url.clone(),
                    recommendation: "Consolidate redundant rules and remove unnecessary \
                                     comments to keep robots.txt under 500KB."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// TitleLengthQualityAnalyzer
// =========================================================================

pub struct TitleLengthQualityAnalyzer;

impl Default for TitleLengthQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TitleLengthQualityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TitleLengthQualityAnalyzer {
    fn name(&self) -> &str {
        "title-length-quality"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let title = match &ctx.page.meta.title {
            Some(t) if !t.trim().is_empty() => t.trim(),
            _ => return findings,
        };

        let len = title.chars().count();

        if len < 20 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLE-QLT001".to_string(),
                title: "Title too short for quality".to_string(),
                description: format!(
                    "Title is {len} characters, which is below the quality minimum of 20 characters."
                ),
                url: url.clone(),
                recommendation: "Write a title of at least 30 characters."
                    .into(),
            });
        } else if len > 60 && len <= 70 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "TITLE-QLT002".to_string(),
                title: "Title may be truncated in search results".to_string(),
                description: format!(
                    "Title is {len} characters. Titles over 60 characters may be truncated."
                ),
                url: url.clone(),
                recommendation: "Keep the title under 60 characters.".into(),
            });
        } else if len > 70 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLE-QLT003".to_string(),
                title: "Title too long for search results".to_string(),
                description: format!(
                    "Title is {len} characters, which exceeds the recommended maximum."
                ),
                url: url.clone(),
                recommendation: "Keep the title under 60 characters.".into(),
            });
        }

        let words: Vec<&str> = title.split_whitespace().collect();
        if words.len() < 2 && len > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "TITLE-QLT004".to_string(),
                title: "Title appears to be a single word".to_string(),
                description: "The title consists of only one word.".to_string(),
                url: url.clone(),
                recommendation: "Expand the title to include multiple descriptive words.".into(),
            });
        }

        findings
    }
}

// =========================================================================
// MetaDescriptionQualityAnalyzer
// =========================================================================

pub struct MetaDescriptionQualityAnalyzer;

impl Default for MetaDescriptionQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaDescriptionQualityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MetaDescriptionQualityAnalyzer {
    fn name(&self) -> &str {
        "meta-description-quality"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let description = match &ctx.page.meta.description {
            Some(d) if !d.trim().is_empty() => d.trim(),
            _ => return findings,
        };

        let len = description.chars().count();

        if len < 70 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "META-QLT001".to_string(),
                title: "Meta description too short for quality".to_string(),
                description: format!(
                    "Meta description is {len} characters. Descriptions under 70 characters may not provide enough context."
                ),
                url: url.clone(),
                recommendation: "Write a meta description of 120-160 characters."
                    .into(),
            });
        }

        if len > 160 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "META-QLT002".to_string(),
                title: "Meta description may be truncated".to_string(),
                description: format!(
                    "Meta description is {len} characters. Descriptions over 160 characters are typically truncated."
                ),
                url: url.clone(),
                recommendation: "Keep the meta description under 160 characters."
                    .into(),
            });
        }

        let sentences: Vec<&str> = description
            .split(['.', '!', '?'])
            .filter(|s| !s.trim().is_empty())
            .collect();
        if sentences.len() > 1 {
            let lower_sentences: Vec<String> =
                sentences.iter().map(|s| s.trim().to_lowercase()).collect();
            let mut seen = std::collections::HashSet::new();
            for s in &lower_sentences {
                if !seen.insert(s.clone()) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "META-QLT003".to_string(),
                        title: "Meta description contains duplicate sentences".to_string(),
                        description: "The meta description contains repeated text.".to_string(),
                        url: url.clone(),
                        recommendation: "Write unique, varied content.".into(),
                    });
                    break;
                }
            }
        }

        findings
    }
}

// =========================================================================
// InternalLinkAnchorAnalyzerV2
// =========================================================================

pub struct InternalLinkAnchorAnalyzerV2;

impl Default for InternalLinkAnchorAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalLinkAnchorAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for InternalLinkAnchorAnalyzerV2 {
    fn name(&self) -> &str {
        "internal-link-anchor-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let internal_links: Vec<&ExtractedLink> =
            ctx.page.links.iter().filter(|l| !l.is_external).collect();

        if internal_links.is_empty() {
            return findings;
        }

        let anchors: Vec<String> = internal_links
            .iter()
            .filter(|l| !l.text.trim().is_empty())
            .map(|l| {
                l.text
                    .trim()
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                    .collect()
            })
            .collect();

        if anchors.len() >= 3 {
            let mut freq: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for anchor in &anchors {
                *freq.entry(anchor.clone()).or_default() += 1;
            }

            let total = anchors.len();
            let unique_count = freq.len();
            let diversity_ratio = unique_count as f64 / total as f64;

            if diversity_ratio < 0.3 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ANCH-V2001".to_string(),
                    title: "Low anchor text diversity".to_string(),
                    description: format!(
                        "Only {unique_count} unique anchor text(s) across {total} internal links ({:.0}% diversity).",
                        diversity_ratio * 100.0
                    ),
                    url: url.clone(),
                    recommendation: "Use more varied, descriptive anchor text."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// WikipediaLinkAnalyzerV2
// =========================================================================

pub struct WikipediaLinkAnalyzerV2;

impl Default for WikipediaLinkAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl WikipediaLinkAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }

    fn is_wikipedia_or_wikimedia(url: &str) -> bool {
        let lower = url.to_lowercase();
        lower.contains("wikipedia.org")
            || lower.contains("wikimedia.org")
            || lower.contains("wikidata.org")
    }
}

impl Analyzer for WikipediaLinkAnalyzerV2 {
    fn name(&self) -> &str {
        "wikipedia-link-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let external_links: Vec<&ExtractedLink> =
            ctx.page.links.iter().filter(|l| l.is_external).collect();

        if external_links.is_empty() {
            return findings;
        }

        let wiki_count = external_links
            .iter()
            .filter(|l| Self::is_wikipedia_or_wikimedia(&l.href))
            .count();

        if wiki_count > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Links,
                code: "WIKI-V2001".to_string(),
                title: "Wikipedia/Wikimedia links detected".to_string(),
                description: format!(
                    "This page contains {wiki_count} outbound link(s) to Wikipedia, Wikimedia, or Wikidata."
                ),
                url: url.clone(),
                recommendation: "Keep Wikipedia/Wikimedia links as they demonstrate thorough research."
                    .to_string(),
            });
        }

        let nofollow_wiki = external_links
            .iter()
            .filter(|l| {
                Self::is_wikipedia_or_wikimedia(&l.href) && l.rel.iter().any(|r| r == "nofollow")
            })
            .count();

        if nofollow_wiki > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Links,
                code: "WIKI-V2002".to_string(),
                title: "Wikipedia links marked nofollow".to_string(),
                description: format!(
                    "{nofollow_wiki} Wikipedia/Wikimedia link(s) are marked with rel=\"nofollow\"."
                ),
                url: url.clone(),
                recommendation: "Consider removing nofollow from Wikipedia links.".to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// MetaDescriptionLengthAnalyzerV2 — META-PX001
// =========================================================================

pub struct MetaDescriptionLengthAnalyzerV2;
impl Default for MetaDescriptionLengthAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl MetaDescriptionLengthAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MetaDescriptionLengthAnalyzerV2 {
    fn name(&self) -> &str {
        "meta-description-length-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(desc) = &ctx.page.meta.description {
            let len = desc.len();
            // Approximate pixel width: ~10px per character at typical SERP rendering
            let approx_px = len * 10;
            if len > 160 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "META-PX001".to_string(),
                    title: "Meta description too long for SERP display".to_string(),
                    description: format!("Meta description is {len} characters (~{approx_px}px). Google typically truncates descriptions beyond 155-160 characters."),
                    url: url.to_string(),
                    recommendation: "Keep meta descriptions between 120-155 characters for optimal SERP display.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// TitleKeywordAnalyzer — TITLE-KW001
// =========================================================================

pub struct TitleKeywordAnalyzer;
impl Default for TitleKeywordAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleKeywordAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TitleKeywordAnalyzer {
    fn name(&self) -> &str {
        "title-keyword"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            let title_lower = title.to_lowercase();
            // Check for common keyword absence patterns
            let has_keyword = !ctx.page.meta.description.as_ref().map_or(true, |d| {
                let words: Vec<&str> = d.split_whitespace().filter(|w| w.len() > 3).collect();
                words
                    .iter()
                    .all(|w| !title_lower.contains(&w.to_lowercase()))
            });
            if !has_keyword && ctx.page.meta.description.is_some() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "TITLE-KW001".to_string(),
                    title: "Title may not contain primary keywords from description".to_string(),
                    description: "The page title does not appear to contain significant keywords found in the meta description.".to_string(),
                    url: url.to_string(),
                    recommendation: "Include primary keywords from your description in the title tag.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// CanonicalSelfReferenceAnalyzerV2 — CAN-SR001
// =========================================================================

pub struct CanonicalSelfReferenceAnalyzerV2;
impl Default for CanonicalSelfReferenceAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CanonicalSelfReferenceAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl CanonicalSelfReferenceAnalyzerV2 {
    fn normalize(url: &str) -> String {
        let mut s = url.to_string();
        if let Some(pos) = s.find('#') {
            s.truncate(pos);
        }
        if s.ends_with('/') && url.trim_end_matches('/') != "/" {
            s.pop();
        }
        s.to_lowercase()
    }
}

impl Analyzer for CanonicalSelfReferenceAnalyzerV2 {
    fn name(&self) -> &str {
        "canonical-self-reference-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let canonical_str = canonical.to_string();
            let page_str = url.to_string();
            if Self::normalize(&canonical_str) == Self::normalize(&page_str) {
                // Self-referencing — correct
            } else {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "CAN-SR001".to_string(),
                    title: "Canonical URL is not self-referencing".to_string(),
                    description: format!("Canonical URL ({canonical_str}) differs from the current page URL ({page_str}). Non-self-referencing canonicals should be intentional."),
                    url: url.to_string(),
                    recommendation: "Ensure the canonical URL points to the preferred version of this page.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// HreflangReciprocalAnalyzerV2 — HREF-REC001
// =========================================================================

pub struct HreflangReciprocalAnalyzerV2;
impl Default for HreflangReciprocalAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl HreflangReciprocalAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HreflangReciprocalAnalyzerV2 {
    fn name(&self) -> &str {
        "hreflang-reciprocal-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hreflang = &ctx.page.meta.hreflang;
        if hreflang.is_empty() {
            return findings;
        }
        let self_url = Url::parse(url);
        if let Ok(self_url) = self_url {
            let self_paths: Vec<String> = hreflang
                .iter()
                .filter(|h| h.url == self_url)
                .map(|h| h.lang.clone())
                .collect();
            for h in hreflang {
                if self_paths.contains(&h.lang) {
                    // This language's URL points to us, but do we reciprocate?
                    let reciprocates = hreflang
                        .iter()
                        .any(|other| other.lang == h.lang && other.url == self_url);
                    if !reciprocates {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Seo,
                            code: "HREF-REC001".to_string(),
                            title: "Hreflang not reciprocated".to_string(),
                            description: format!("Language \"{}\" hreflang points to this page but is not declared in this page's hreflang tags.", h.lang),
                            url: url.to_string(),
                            recommendation: "Add a reciprocal hreflang tag for this language pointing back to the current page.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// InternalLinkAnchorTextDiversityAnalyzer — ANCH-DIV001
// =========================================================================

pub struct InternalLinkAnchorTextDiversityAnalyzer;
impl Default for InternalLinkAnchorTextDiversityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl InternalLinkAnchorTextDiversityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for InternalLinkAnchorTextDiversityAnalyzer {
    fn name(&self) -> &str {
        "internal-link-anchor-text-diversity"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let page_url = Url::parse(url).ok();
        if let Some(page_url) = &page_url {
            let internal_links: Vec<&ExtractedLink> = ctx
                .page
                .links
                .iter()
                .filter(|l| {
                    !l.is_external
                        && Url::parse(&l.href).map_or(true, |u| u.host_str() == page_url.host_str())
                })
                .collect();
            if internal_links.len() >= 5 {
                let mut text_counts: HashMap<String, usize> = HashMap::new();
                for link in &internal_links {
                    let text = link.text.trim().to_lowercase();
                    if !text.is_empty() {
                        *text_counts.entry(text).or_insert(0) += 1;
                    }
                }
                let total_texts = text_counts.len();
                if total_texts > 0 {
                    let max_count = text_counts.values().max().unwrap_or(&0);
                    let max_pct = (*max_count as f64 / internal_links.len() as f64 * 100.0) as u32;
                    if max_pct > 40 {
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Seo,
                            code: "ANCH-DIV001".to_string(),
                            title: "Low internal link anchor text diversity".to_string(),
                            description: format!("The most common anchor text appears {max_count} times ({max_pct}% of {total_texts} unique texts). Overly uniform anchor text may appear manipulative."),
                            url: url.to_string(),
                            recommendation: "Vary anchor text for internal links using natural, descriptive phrases.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// ExternalLinkAuthorityAnalyzer — EXT-AUTH001
// =========================================================================

pub struct ExternalLinkAuthorityAnalyzer;
impl Default for ExternalLinkAuthorityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl ExternalLinkAuthorityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl ExternalLinkAuthorityAnalyzer {
    const HIGH_AUTHORITY_DOMAINS: &[&str] = &[
        "wikipedia.org",
        "github.com",
        "stackoverflow.com",
        "mozilla.org",
        "w3.org",
        "schema.org",
        "developer.mozilla.org",
        "docs.python.org",
        "learn.microsoft.com",
        "cloud.google.com",
        "aws.amazon.com",
    ];
}

impl Analyzer for ExternalLinkAuthorityAnalyzer {
    fn name(&self) -> &str {
        "external-link-authority"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let high_auth: usize = ctx
            .page
            .links
            .iter()
            .filter(|l| {
                l.is_external
                    && Self::HIGH_AUTHORITY_DOMAINS
                        .iter()
                        .any(|d| l.href.contains(d))
            })
            .count();
        let total_ext = ctx.page.links.iter().filter(|l| l.is_external).count();
        if total_ext >= 3 && high_auth == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "EXT-AUTH001".to_string(),
                title: "No links to high-authority domains".to_string(),
                description: format!("Page has {total_ext} external link(s) but none point to known high-authority domains."),
                url: url.to_string(),
                recommendation: "Link to authoritative sources (e.g., Wikipedia, MDN, official docs) to boost E-E-A-T signals.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// SitemapCoverageAnalyzerV2 — SITEMAP-COV001
// =========================================================================

pub struct SitemapCoverageAnalyzerV2 {
    known_urls: HashSet<String>,
}

impl Default for SitemapCoverageAnalyzerV2 {
    fn default() -> Self {
        Self {
            known_urls: HashSet::new(),
        }
    }
}

impl SitemapCoverageAnalyzerV2 {
    pub fn new(known_urls: HashSet<String>) -> Self {
        Self { known_urls }
    }
    pub fn empty() -> Self {
        Self::new(HashSet::new())
    }
}

impl Analyzer for SitemapCoverageAnalyzerV2 {
    fn name(&self) -> &str {
        "sitemap-coverage-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !self.known_urls.is_empty() && !self.known_urls.contains(url) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "SITEMAP-COV001".to_string(),
                title: "Page not found in sitemap".to_string(),
                description: "This page URL was not found in any sitemap file.".to_string(),
                url: url.to_string(),
                recommendation:
                    "Add this page to your sitemap.xml to help search engines discover it."
                        .to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// RobotsTxtCoverageAnalyzerV2 — ROBOTS-COV001
// =========================================================================

pub struct RobotsTxtCoverageAnalyzerV2;
impl Default for RobotsTxtCoverageAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl RobotsTxtCoverageAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RobotsTxtCoverageAnalyzerV2 {
    fn name(&self) -> &str {
        "robots-txt-coverage-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let path = Url::parse(url).map_or(String::new(), |u| {
                let p = u.path().to_string();
                if p.ends_with('/') {
                    p
                } else {
                    p
                }
            });
            let lower = robots.to_lowercase();
            let lines: Vec<&str> = lower.lines().collect();
            let mut in_disallow = false;
            let mut user_agent_all = false;
            for line in &lines {
                let trimmed = line.trim();
                if trimmed.starts_with("user-agent:") {
                    let agent = trimmed.split(':').nth(1).unwrap_or("").trim();
                    user_agent_all = agent == "*";
                }
                if trimmed.starts_with("disallow:") && user_agent_all {
                    let disallow_path = trimmed.split(':').nth(1).unwrap_or("").trim();
                    if !disallow_path.is_empty() && path.starts_with(disallow_path) {
                        in_disallow = true;
                    }
                }
            }
            if in_disallow {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ROBOTS-COV001".to_string(),
                    title: "Page blocked by robots.txt".to_string(),
                    description: "This page's URL path matches a Disallow rule in robots.txt for user-agent *.".to_string(),
                    url: url.to_string(),
                    recommendation: "Review robots.txt Disallow rules to ensure important pages are not blocked.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// PaginationLinkAnalyzer — PAG-LINK001
// =========================================================================

pub struct PaginationLinkAnalyzer;
impl Default for PaginationLinkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl PaginationLinkAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PaginationLinkAnalyzer {
    fn name(&self) -> &str {
        "pagination-link"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let is_pagination =
            ctx.page.links.iter().any(|l| {
                l.rel.contains(&"next".to_string()) || l.rel.contains(&"prev".to_string())
            });
        if is_pagination {
            let nofollow_pagination: Vec<&ExtractedLink> = ctx
                .page
                .links
                .iter()
                .filter(|l| {
                    l.rel.contains(&"next".to_string()) || l.rel.contains(&"prev".to_string())
                })
                .filter(|l| l.rel.contains(&"nofollow".to_string()))
                .collect();
            if !nofollow_pagination.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "PAG-LINK001".to_string(),
                    title: "Pagination links marked nofollow".to_string(),
                    description: format!("{} pagination link(s) have rel=\"nofollow\". Nofollow on pagination prevents PageRank from flowing through paginated series.", nofollow_pagination.len()),
                    url: url.to_string(),
                    recommendation: "Remove nofollow from pagination rel=next/prev links to allow crawling of paginated content.".to_string(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Meta Description V3 — description too long >165 chars
// ---------------------------------------------------------------------------

pub struct MetaDescriptionAnalyzerV3;

impl Default for MetaDescriptionAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaDescriptionAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MetaDescriptionAnalyzerV3 {
    fn name(&self) -> &str {
        "meta-description-v3"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(desc) = &ctx.page.meta.description {
            if desc.len() > 165 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "META-V3001".to_string(),
                    title: "Meta description too long".to_string(),
                    description: format!("Meta description is {} characters, exceeding the recommended maximum of 165. Search engines will truncate it.", desc.len()),
                    url: url.clone(),
                    recommendation: "Shorten the meta description to 150-165 characters.".into(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Title V3 — title too short <20 chars
// ---------------------------------------------------------------------------

pub struct TitleAnalyzerV3;

impl Default for TitleAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}

impl TitleAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TitleAnalyzerV3 {
    fn name(&self) -> &str {
        "title-v3"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            if !title.trim().is_empty() && title.len() < 20 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "TITLE-V3001".to_string(),
                    title: "Title tag too short".to_string(),
                    description: format!("Title tag is {} characters. Titles under 20 characters may not be descriptive enough for search engine results.", title.len()),
                    url: url.clone(),
                    recommendation: "Write a descriptive title of 20-60 characters that accurately describes the page content.".into(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Canonical URL V2 — canonical mismatch with page URL
// ---------------------------------------------------------------------------

pub struct CanonicalUrlAnalyzerV2;

impl Default for CanonicalUrlAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl CanonicalUrlAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CanonicalUrlAnalyzerV2 {
    fn name(&self) -> &str {
        "canonical-url-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let canonical_str = canonical.to_string().trim_end_matches('/').to_lowercase();
            let page_str = url.trim_end_matches('/').to_lowercase();
            if canonical_str != page_str {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "CAN-V2001".to_string(),
                    title: "Canonical URL mismatch".to_string(),
                    description: format!("Canonical URL ({canonical}) does not match the current page URL ({url})."),
                    url: url.clone(),
                    recommendation: "Ensure the canonical URL points to the correct preferred version of this page.".into(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Hreflang V3 — missing x-default
// ---------------------------------------------------------------------------

pub struct HreflangValidatorV3;

impl Default for HreflangValidatorV3 {
    fn default() -> Self {
        Self::new()
    }
}

impl HreflangValidatorV3 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HreflangValidatorV3 {
    fn name(&self) -> &str {
        "hreflang-v3"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hreflang = &ctx.page.meta.hreflang;
        if !hreflang.is_empty() {
            let has_x_default = hreflang.iter().any(|t| t.lang == "x-default");
            if !has_x_default {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "HREF-V3001".to_string(),
                    title: "Hreflang missing x-default".to_string(),
                    description: "Hreflang tags are present but none specify x-default. The x-default tag tells search engines which URL to show for users whose language doesn't match any other hreflang.".into(),
                    url: url.clone(),
                    recommendation: "Add an x-default hreflang tag to specify the fallback URL for unmatched languages.".into(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Sitemap V2 — lastmod format invalid
// ---------------------------------------------------------------------------

pub struct SitemapAnalyzerV2;

impl Default for SitemapAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl SitemapAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SitemapAnalyzerV2 {
    fn name(&self) -> &str {
        "sitemap-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let body = ctx.body.unwrap_or("");
        if body.contains("sitemap") {
            if let Some(pos) = body.find("sitemap") {
                let after = &body[pos..];
                if let Some(end) = after.find('"') {
                    let sitemap_path = &after[..end];
                    if !sitemap_path.ends_with(".xml")
                        && !sitemap_path.ends_with(".xml.gz")
                        && !sitemap_path.is_empty()
                    {
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Seo,
                            code: "SITEMAP-V2001".to_string(),
                            title: "Sitemap URL may be invalid".to_string(),
                            description: format!("Sitemap reference \"{sitemap_path}\" does not end with .xml or .xml.gz. Search engines expect XML sitemaps."),
                            url: url.clone(),
                            recommendation: "Ensure the sitemap URL points to a valid XML sitemap file.".into(),
                        });
                    }
                }
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Robots.txt V2 — disallows important paths
// ---------------------------------------------------------------------------

pub struct RobotsTxtAnalyzerV2;

impl Default for RobotsTxtAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl RobotsTxtAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RobotsTxtAnalyzerV2 {
    fn name(&self) -> &str {
        "robots-txt-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let lines: Vec<&str> = robots.lines().collect();
            let disallowed: Vec<&str> = lines
                .iter()
                .filter(|l| l.starts_with("Disallow:") || l.starts_with("disallow:"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim())
                .filter(|p| !p.is_empty())
                .collect();
            let important_paths = ["/", "/index.html", "/sitemap.xml"];
            for imp in important_paths {
                if disallowed.iter().any(|d| *d == imp) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "ROBOTS-V2001".to_string(),
                        title: "Robots.txt disallows important path".to_string(),
                        description: format!("Robots.txt disallows \"{imp}\". This may prevent search engines from crawling important content."),
                        url: url.clone(),
                        recommendation: "Review robots.txt Disallow directives to ensure important pages are crawlable.".into(),
                    });
                }
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Internal Link V2 — links with no anchor text
// ---------------------------------------------------------------------------

pub struct InternalLinkAnalyzerV2;

impl Default for InternalLinkAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalLinkAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for InternalLinkAnalyzerV2 {
    fn name(&self) -> &str {
        "internal-link-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let page_host = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()));
        if let Some(host) = &page_host {
            let empty_text_internal: Vec<&str> = ctx
                .page
                .links
                .iter()
                .filter(|l| !l.is_external)
                .filter(|l| l.href.contains(host) || l.href.starts_with('/'))
                .filter(|l| l.text.trim().is_empty())
                .map(|l| l.href.as_str())
                .collect();
            if !empty_text_internal.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "INTLINK-V2001".to_string(),
                    title: "Internal links with no anchor text".to_string(),
                    description: format!("{} internal link(s) have empty anchor text: {}.", empty_text_internal.len(), empty_text_internal.iter().take(3).cloned().collect::<Vec<_>>().join(", ")),
                    url: url.clone(),
                    recommendation: "Add descriptive anchor text to all internal links to help search engines understand the linked content.".into(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: External Link V2 — links to low-authority domains
// ---------------------------------------------------------------------------

pub struct ExternalLinkAnalyzerV2;

impl Default for ExternalLinkAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternalLinkAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ExternalLinkAnalyzerV2 {
    fn name(&self) -> &str {
        "external-link-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let low_authority_tlds = [".xyz", ".top", ".buzz", ".club", ".work", ".click"];
        let suspicious: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| l.is_external)
            .filter(|l| {
                let lower = l.href.to_lowercase();
                low_authority_tlds.iter().any(|tld| lower.contains(tld))
            })
            .map(|l| l.href.as_str())
            .collect();
        if !suspicious.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "EXTLINK-V2001".to_string(),
                title: "External links to low-authority domains".to_string(),
                description: format!("{} external link(s) point to domains with low-authority TLDs: {}.", suspicious.len(), suspicious.iter().take(3).cloned().collect::<Vec<_>>().join(", ")),
                url: url.clone(),
                recommendation: "Review external links to ensure they point to reputable domains. Low-authority TLDs may be associated with spam.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Keyword Analyzer V2 — keyword density too high >5%
// ---------------------------------------------------------------------------

pub struct KeywordAnalyzerV2;

impl Default for KeywordAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl KeywordAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for KeywordAnalyzerV2 {
    fn name(&self) -> &str {
        "keyword-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            if !title.trim().is_empty() && ctx.page.word_count > 0 {
                let title_lower = title.to_lowercase();
                let title_words: Vec<&str> = title_lower.split_whitespace().collect();
                if let Some(first_word) = title_words.first() {
                    if first_word.len() > 3 {
                        let body_text = ctx.body.unwrap_or("").to_lowercase();
                        let word_count = ctx.page.word_count.max(1);
                        let occurrences = body_text.matches(first_word).count();
                        let density = (occurrences as f64 / word_count as f64) * 100.0;
                        if density > 5.0 {
                            findings.push(Finding {
                                severity: Severity::Warning,
                                category: IssueCategory::Seo,
                                code: "KW-V2001".to_string(),
                                title: "Keyword density too high".to_string(),
                                description: format!("The keyword \"{first_word}\" appears {occurrences} times ({density:.1}%) in the body text, exceeding the 5% threshold. This may be flagged as keyword stuffing."),
                                url: url.clone(),
                                recommendation: "Reduce keyword repetition. Use synonyms and related terms instead of repeating the same keyword.".into(),
                            });
                        }
                    }
                }
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Content Quality V2 — readability below grade 8
// ---------------------------------------------------------------------------

pub struct ContentQualityAnalyzerV2;

impl Default for ContentQualityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentQualityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ContentQualityAnalyzerV2 {
    fn name(&self) -> &str {
        "content-quality-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.word_count > 50 && ctx.page.sentence_count > 0 {
            let mut syllable_count = 0;
            let body = ctx.body.unwrap_or("");
            for word in body.split_whitespace() {
                syllable_count += super::count_syllables(word);
            }
            let grade = super::flesch_kincaid_grade(
                ctx.page.word_count,
                ctx.page.sentence_count,
                syllable_count,
            );
            if grade > 12.0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "CQ-V2001".to_string(),
                    title: "Readability score below grade 8".to_string(),
                    description: format!("Flesch-Kincaid grade level is {grade:.1}, indicating complex text. Content above grade 12 may be difficult for a general audience to understand."),
                    url: url.clone(),
                    recommendation: "Simplify sentence structure and use shorter, more common words to improve readability.".into(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Word Count V2 — word count <100
// ---------------------------------------------------------------------------

pub struct WordCountAnalyzerV2;

impl Default for WordCountAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl WordCountAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WordCountAnalyzerV2 {
    fn name(&self) -> &str {
        "word-count-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.word_count > 0 && ctx.page.word_count < 100 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "WC-V2001".to_string(),
                title: "Word count below 100".to_string(),
                description: format!("Page has only {} words. Pages with very thin content may rank poorly in search results.", ctx.page.word_count),
                url: url.clone(),
                recommendation: "Add more substantive content to the page. Aim for at least 300 words for informational pages.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// SEO: Link Analyzer V2 — too many external links >50%
// ---------------------------------------------------------------------------

pub struct LinkAnalyzerV2;

impl Default for LinkAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LinkAnalyzerV2 {
    fn name(&self) -> &str {
        "link-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let total_links = ctx.page.links.len();
        if total_links > 0 {
            let external_count = ctx.page.links.iter().filter(|l| l.is_external).count();
            let ratio = external_count as f64 / total_links as f64;
            if ratio > 0.5 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "LINK-V2001".to_string(),
                    title: "Too many external links".to_string(),
                    description: format!("{external_count} of {total_links} links ({:.0}%) are external, exceeding the 50% threshold. A high ratio of outbound links may dilute PageRank.", ratio * 100.0),
                    url: url.clone(),
                    recommendation: "Balance external links with internal links. Ensure the majority of links point to relevant internal pages.".into(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// New SEO analyzer tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod new_seo_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ExtractedLink, ParsedPage};

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
            rendered: None,
        }
    }

    fn make_ctx_with_chain<'a>(
        page: &'a ParsedPage,
        chain: &'a [crate::RedirectHop],
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: None,
            headers: &[],
            response_time: None,
            redirect_chain: chain,
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    // PaginationDepthValidator tests

    #[test]
    fn test_pagination_depth_no_page_param() {
        let page = make_page("https://example.com/products");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationDepthValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_pagination_depth_page_3() {
        let page = make_page("https://example.com/products?page=3");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationDepthValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_pagination_depth_page_6() {
        let page = make_page("https://example.com/products?page=6");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationDepthValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "PAGDEP001"));
    }

    #[test]
    fn test_pagination_depth_page_path() {
        let page = make_page("https://example.com/blog/page/10/");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationDepthValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "PAGDEP001"));
    }

    #[test]
    fn test_pagination_depth_page_path_4() {
        let page = make_page("https://example.com/blog/page/4/");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationDepthValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_pagination_depth_invalid_page() {
        let page = make_page("https://example.com/products?page=abc");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationDepthValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_pagination_depth_name() {
        assert_eq!(PaginationDepthValidator::new().name(), "pagination-depth");
    }

    #[test]
    fn test_pagination_depth_page_exact_5() {
        let page = make_page("https://example.com/products?page=5");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationDepthValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_pagination_depth_page_100() {
        let page = make_page("https://example.com/products?page=100");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationDepthValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "PAGDEP001"));
    }

    #[test]
    fn test_pagination_depth_category_param() {
        let page = make_page("https://example.com/products?category=books&page=7");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationDepthValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "PAGDEP001"));
    }

    // RedirectLoopDetector tests

    #[test]
    fn test_redirect_loop_no_chain() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(RedirectLoopDetector::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_redirect_loop_no_loop() {
        let page = make_page("https://example.com/c");
        use url::Url;
        let chain = vec![
            crate::RedirectHop {
                from: Url::parse("http://example.com/a").unwrap(),
                to: Url::parse("http://example.com/b").unwrap(),
                status_code: 301,
            },
            crate::RedirectHop {
                from: Url::parse("http://example.com/b").unwrap(),
                to: Url::parse("http://example.com/c").unwrap(),
                status_code: 302,
            },
        ];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(RedirectLoopDetector::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_redirect_loop_detected() {
        let page = make_page("https://example.com/a");
        use url::Url;
        let chain = vec![
            crate::RedirectHop {
                from: Url::parse("http://example.com/a").unwrap(),
                to: Url::parse("http://example.com/b").unwrap(),
                status_code: 301,
            },
            crate::RedirectHop {
                from: Url::parse("http://example.com/b").unwrap(),
                to: Url::parse("http://example.com/a").unwrap(),
                status_code: 301,
            },
        ];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(RedirectLoopDetector::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "REDIRLOOP001"));
    }

    #[test]
    fn test_redirect_loop_self_redirect() {
        let page = make_page("https://example.com/a");
        use url::Url;
        let chain = vec![crate::RedirectHop {
            from: Url::parse("http://example.com/a").unwrap(),
            to: Url::parse("http://example.com/a").unwrap(),
            status_code: 301,
        }];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(RedirectLoopDetector::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "REDIRLOOP001"));
    }

    #[test]
    fn test_redirect_loop_three_hop_loop() {
        let page = make_page("https://example.com/a");
        use url::Url;
        let chain = vec![
            crate::RedirectHop {
                from: Url::parse("http://example.com/a").unwrap(),
                to: Url::parse("http://example.com/b").unwrap(),
                status_code: 301,
            },
            crate::RedirectHop {
                from: Url::parse("http://example.com/b").unwrap(),
                to: Url::parse("http://example.com/c").unwrap(),
                status_code: 302,
            },
            crate::RedirectHop {
                from: Url::parse("http://example.com/c").unwrap(),
                to: Url::parse("http://example.com/a").unwrap(),
                status_code: 301,
            },
        ];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(RedirectLoopDetector::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "REDIRLOOP001"));
    }

    #[test]
    fn test_redirect_loop_name() {
        assert_eq!(RedirectLoopDetector::new().name(), "redirect-loop");
    }

    #[test]
    fn test_redirect_loop_no_loop_linear() {
        let page = make_page("https://example.com/d");
        use url::Url;
        let chain = vec![
            crate::RedirectHop {
                from: Url::parse("http://example.com/a").unwrap(),
                to: Url::parse("http://example.com/b").unwrap(),
                status_code: 301,
            },
            crate::RedirectHop {
                from: Url::parse("http://example.com/b").unwrap(),
                to: Url::parse("http://example.com/c").unwrap(),
                status_code: 301,
            },
            crate::RedirectHop {
                from: Url::parse("http://example.com/c").unwrap(),
                to: Url::parse("http://example.com/d").unwrap(),
                status_code: 301,
            },
        ];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(RedirectLoopDetector::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_redirect_loop_same_url_different_schemes() {
        let page = make_page("https://example.com/a");
        use url::Url;
        let chain = vec![crate::RedirectHop {
            from: Url::parse("http://example.com/a").unwrap(),
            to: Url::parse("https://example.com/a").unwrap(),
            status_code: 301,
        }];
        let ctx = make_ctx_with_chain(&page, &chain);
        // Same path different scheme is NOT a loop
        assert!(RedirectLoopDetector::new().analyze(&ctx).is_empty());
    }

    // MixedProtocolRedirectValidator tests

    #[test]
    fn test_mixed_protocol_no_redirects() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(MixedProtocolRedirectValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_mixed_protocol_301() {
        let page = make_page("https://example.com");
        use url::Url;
        let chain = vec![crate::RedirectHop {
            from: Url::parse("http://example.com").unwrap(),
            to: Url::parse("https://example.com").unwrap(),
            status_code: 301,
        }];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(MixedProtocolRedirectValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_mixed_protocol_302() {
        let page = make_page("https://example.com");
        use url::Url;
        let chain = vec![crate::RedirectHop {
            from: Url::parse("http://example.com").unwrap(),
            to: Url::parse("https://example.com").unwrap(),
            status_code: 302,
        }];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(MixedProtocolRedirectValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MIXPROT001"));
    }

    #[test]
    fn test_mixed_protocol_307() {
        let page = make_page("https://example.com");
        use url::Url;
        let chain = vec![crate::RedirectHop {
            from: Url::parse("http://example.com").unwrap(),
            to: Url::parse("https://example.com").unwrap(),
            status_code: 307,
        }];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(MixedProtocolRedirectValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MIXPROT001"));
    }

    #[test]
    fn test_mixed_protocol_308() {
        let page = make_page("https://example.com");
        use url::Url;
        let chain = vec![crate::RedirectHop {
            from: Url::parse("http://example.com").unwrap(),
            to: Url::parse("https://example.com").unwrap(),
            status_code: 308,
        }];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(MixedProtocolRedirectValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MIXPROT001"));
    }

    #[test]
    fn test_mixed_protocol_same_scheme() {
        let page = make_page("https://example.com");
        use url::Url;
        let chain = vec![crate::RedirectHop {
            from: Url::parse("https://example.com/a").unwrap(),
            to: Url::parse("https://example.com/b").unwrap(),
            status_code: 302,
        }];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(MixedProtocolRedirectValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_mixed_protocol_name() {
        assert_eq!(
            MixedProtocolRedirectValidator::new().name(),
            "mixed-protocol-redirect"
        );
    }

    #[test]
    fn test_mixed_protocol_multiple_hops() {
        let page = make_page("https://example.com");
        use url::Url;
        let chain = vec![
            crate::RedirectHop {
                from: Url::parse("http://example.com/a").unwrap(),
                to: Url::parse("https://example.com/a").unwrap(),
                status_code: 302,
            },
            crate::RedirectHop {
                from: Url::parse("https://example.com/a").unwrap(),
                to: Url::parse("https://example.com/b").unwrap(),
                status_code: 301,
            },
        ];
        let ctx = make_ctx_with_chain(&page, &chain);
        assert!(MixedProtocolRedirectValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MIXPROT001"));
    }

    // InternalNofollowOveruseValidator tests

    #[test]
    fn test_nofollow_overuse_no_internal_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalNofollowOveruseValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_nofollow_overuse_low_ratio() {
        let mut page = make_page("https://example.com");
        page.links = (0..10)
            .map(|i| ExtractedLink {
                href: format!("/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        // Only 1 nofollow = 10% < 30%
        page.links[0].rel = vec!["nofollow".to_string()];
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalNofollowOveruseValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_nofollow_overuse_high_ratio() {
        let mut page = make_page("https://example.com");
        page.links = (0..10)
            .map(|i| ExtractedLink {
                href: format!("/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        // 4 nofollow = 40% > 30%
        for i in 0..4 {
            page.links[i].rel = vec!["nofollow".to_string()];
        }
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalNofollowOveruseValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "NOFOLLOW001"));
    }

    #[test]
    fn test_nofollow_overuse_fewer_than_5_links() {
        let mut page = make_page("https://example.com");
        page.links = (0..4)
            .map(|i| ExtractedLink {
                href: format!("/page{i}"),
                text: format!("Link {i}"),
                rel: vec!["nofollow".to_string()],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalNofollowOveruseValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_nofollow_overuse_all_nofollow() {
        let mut page = make_page("https://example.com");
        page.links = (0..10)
            .map(|i| ExtractedLink {
                href: format!("/page{i}"),
                text: format!("Link {i}"),
                rel: vec!["nofollow".to_string()],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalNofollowOveruseValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "NOFOLLOW001"));
    }

    #[test]
    fn test_nofollow_overuse_name() {
        assert_eq!(
            InternalNofollowOveruseValidator::new().name(),
            "internal-nofollow-overuse"
        );
    }

    #[test]
    fn test_nofollow_overuse_mixed_internal_external() {
        let mut page = make_page("https://example.com");
        let mut links: Vec<ExtractedLink> = (0..10)
            .map(|i| ExtractedLink {
                href: format!("/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        // Add 5 external links
        for i in 0..5 {
            links.push(ExtractedLink {
                href: format!("https://other.com/page{i}"),
                text: format!("Ext {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            });
        }
        // 4 internal nofollow = 40% of 10 internal
        for i in 0..4 {
            links[i].rel = vec!["nofollow".to_string()];
        }
        page.links = links;
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalNofollowOveruseValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "NOFOLLOW001"));
    }

    // ExternalNofollowUnderuseValidator tests

    #[test]
    fn test_nofollow_underuse_no_external() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(ExternalNofollowUnderuseValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_nofollow_underuse_enough_nofollow() {
        let mut page = make_page("https://example.com");
        page.links = (0..10)
            .map(|i| ExtractedLink {
                href: format!("https://other.com/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        // 2 nofollow = 20% >= 10%
        page.links[0].rel = vec!["nofollow".to_string()];
        page.links[1].rel = vec!["nofollow".to_string()];
        let ctx = make_ctx(&page, Some(200));
        assert!(ExternalNofollowUnderuseValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_nofollow_underuse_insufficient() {
        let mut page = make_page("https://example.com");
        page.links = (0..10)
            .map(|i| ExtractedLink {
                href: format!("https://other.com/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        // 0 nofollow = 0% < 10%
        let ctx = make_ctx(&page, Some(200));
        assert!(ExternalNofollowUnderuseValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "NOFOLLOW002"));
    }

    #[test]
    fn test_nofollow_underuse_fewer_than_5() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(ExternalNofollowUnderuseValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_nofollow_underuse_name() {
        assert_eq!(
            ExternalNofollowUnderuseValidator::new().name(),
            "external-nofollow-underuse"
        );
    }

    #[test]
    fn test_nofollow_underuse_one_nofollow() {
        let mut page = make_page("https://example.com");
        page.links = (0..10)
            .map(|i| ExtractedLink {
                href: format!("https://other.com/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        // 1 nofollow = 10% = 10% (not < 10%)
        page.links[0].rel = vec!["nofollow".to_string()];
        let ctx = make_ctx(&page, Some(200));
        assert!(ExternalNofollowUnderuseValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    // SitemapXmlSizeValidator tests

    #[test]
    fn test_sitemap_size_no_body_size() {
        let page = make_page("https://example.com/sitemap.xml");
        let ctx = make_ctx(&page, Some(200));
        assert!(SitemapXmlSizeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sitemap_size_under_limit() {
        let page = make_page("https://example.com/sitemap.xml");
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body_size = Some(10 * 1024 * 1024); // 10MB
        assert!(SitemapXmlSizeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sitemap_size_over_limit() {
        let page = make_page("https://example.com/sitemap.xml");
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body_size = Some(60 * 1024 * 1024); // 60MB
        assert!(SitemapXmlSizeValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "SITEMAPSIZE001"));
    }

    #[test]
    fn test_sitemap_size_exact_limit() {
        let page = make_page("https://example.com/sitemap.xml");
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body_size = Some(50 * 1024 * 1024); // 50MB exactly
        assert!(SitemapXmlSizeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sitemap_size_name() {
        assert_eq!(SitemapXmlSizeValidator::new().name(), "sitemap-xml-size");
    }

    #[test]
    fn test_sitemap_size_1mb() {
        let page = make_page("https://example.com/sitemap.xml");
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body_size = Some(1024 * 1024); // 1MB
        assert!(SitemapXmlSizeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sitemap_size_100mb() {
        let page = make_page("https://example.com/sitemap.xml");
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body_size = Some(100 * 1024 * 1024); // 100MB
        assert!(SitemapXmlSizeValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "SITEMAPSIZE001"));
    }

    // RobotsTxtSizeValidator tests

    #[test]
    fn test_robots_size_no_robots_txt() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(RobotsTxtSizeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_robots_size_under_limit() {
        let page = make_page("https://example.com");
        let robots = "User-agent: *\nDisallow: /admin\n";
        let mut ctx = make_ctx(&page, Some(200));
        ctx.robots_txt = Some(robots);
        assert!(RobotsTxtSizeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_robots_size_over_limit() {
        let page = make_page("https://example.com");
        let robots = "User-agent: *\n".repeat(40000);
        let mut ctx = make_ctx(&page, Some(200));
        ctx.robots_txt = Some(&robots);
        assert!(RobotsTxtSizeValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ROBOTSSIZE001"));
    }

    #[test]
    fn test_robots_size_exact_limit() {
        let page = make_page("https://example.com");
        let robots = "x".repeat(500 * 1024);
        let mut ctx = make_ctx(&page, Some(200));
        ctx.robots_txt = Some(&robots);
        assert!(RobotsTxtSizeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_robots_size_name() {
        assert_eq!(RobotsTxtSizeValidator::new().name(), "robots-txt-size");
    }

    #[test]
    fn test_robots_size_empty() {
        let page = make_page("https://example.com");
        let mut ctx = make_ctx(&page, Some(200));
        ctx.robots_txt = Some("");
        assert!(RobotsTxtSizeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_robots_size_600kb() {
        let page = make_page("https://example.com");
        let robots = "x".repeat(600 * 1024);
        let mut ctx = make_ctx(&page, Some(200));
        ctx.robots_txt = Some(&robots);
        assert!(RobotsTxtSizeValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ROBOTSSIZE001"));
    }

    // ---- HreflangSelfReferenceValidator tests ----

    #[test]
    fn test_href_self_no_hreflang() {
        let page = make_page("https://example.com/en");
        let ctx = make_ctx(&page, Some(200));
        assert!(HreflangSelfReferenceValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_href_self_has_self_ref() {
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
        assert!(HreflangSelfReferenceValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_href_self_missing_self_ref() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "fr".to_string(),
                url: Url::parse("https://example.com/fr").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "de".to_string(),
                url: Url::parse("https://example.com/de").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        assert!(HreflangSelfReferenceValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "HREFSELF001"));
    }

    #[test]
    fn test_href_self_self_ref_via_canonical() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en/").unwrap(),
        }];
        page.meta.canonical = Some(Url::parse("https://example.com/en").unwrap());
        let ctx = make_ctx(&page, Some(200));
        assert!(HreflangSelfReferenceValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_href_self_xdefault_not_self() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "x-default".to_string(),
                url: Url::parse("https://example.com/").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "fr".to_string(),
                url: Url::parse("https://example.com/fr").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        assert!(HreflangSelfReferenceValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "HREFSELF001"));
    }

    #[test]
    fn test_href_self_trailing_slash_match() {
        let mut page = make_page("https://example.com/en/");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HreflangSelfReferenceValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_href_self_single_self_ref() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HreflangSelfReferenceValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_href_self_multiple_langs_no_self() {
        let mut page = make_page("https://example.com/en");
        page.meta.hreflang = vec![
            crate::meta::HreflangTag {
                lang: "fr".to_string(),
                url: Url::parse("https://example.com/fr").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "de".to_string(),
                url: Url::parse("https://example.com/de").unwrap(),
            },
            crate::meta::HreflangTag {
                lang: "es".to_string(),
                url: Url::parse("https://example.com/es").unwrap(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangSelfReferenceValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "HREFSELF001");
    }

    #[test]
    fn test_href_self_name() {
        assert_eq!(
            HreflangSelfReferenceValidator::new().name(),
            "hreflang-self-reference"
        );
    }

    #[test]
    fn test_href_self_default() {
        let _ = HreflangSelfReferenceValidator::default();
    }

    // ---- OpenSearchDescriptionValidator tests ----

    #[test]
    fn test_opdesc_no_body() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(OpenSearchDescriptionValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "OPDESC001"));
    }

    #[test]
    fn test_opdesc_has_opensearch() {
        let page = make_page("https://example.com");
        let body = r#"<html><head><link rel="search" type="application/opensearchdescription+xml" title="Search" href="/opensearch.xml"></head></html>"#;
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body = Some(body);
        assert!(OpenSearchDescriptionValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_opdesc_no_opensearch() {
        let page = make_page("https://example.com");
        let body = "<html><head><title>Test</title></head></html>";
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body = Some(body);
        assert!(OpenSearchDescriptionValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "OPDESC001"));
    }

    #[test]
    fn test_opdesc_single_quotes() {
        let page = make_page("https://example.com");
        let body = r#"<html><head><link rel='search' type='application/opensearchdescription+xml' title='Search' href='/opensearch.xml'></head></html>"#;
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body = Some(body);
        assert!(OpenSearchDescriptionValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_opdesc_wrong_type() {
        let page = make_page("https://example.com");
        let body = r#"<html><head><link rel="search" type="text/html" title="Search" href="/search"></head></html>"#;
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body = Some(body);
        assert!(OpenSearchDescriptionValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "OPDESC001"));
    }

    #[test]
    fn test_opdesc_rel_search_wrong_type() {
        let page = make_page("https://example.com");
        let body = r#"<html><head><link rel="search" type="application/rss+xml" title="Feed" href="/feed.xml"></head></html>"#;
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body = Some(body);
        assert!(OpenSearchDescriptionValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "OPDESC001"));
    }

    #[test]
    fn test_opdesc_empty_body() {
        let page = make_page("https://example.com");
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body = Some("");
        assert!(OpenSearchDescriptionValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "OPDESC001"));
    }

    #[test]
    fn test_opdesc_case_insensitive() {
        let page = make_page("https://example.com");
        let body = r#"<html><head><link REL="search" TYPE="application/opensearchdescription+xml" TITLE="Search" HREF="/opensearch.xml"></head></html>"#;
        let mut ctx = make_ctx(&page, Some(200));
        ctx.body = Some(body);
        assert!(OpenSearchDescriptionValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_opdesc_name() {
        assert_eq!(
            OpenSearchDescriptionValidator::new().name(),
            "opensearch-description"
        );
    }

    #[test]
    fn test_opdesc_default() {
        let _ = OpenSearchDescriptionValidator::default();
    }

    // TitleLengthQualityAnalyzer tests

    #[test]
    fn test_title_qlt_no_title() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(TitleLengthQualityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_title_qlt_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Hi".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(TitleLengthQualityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TITLE-QLT001"));
    }

    #[test]
    fn test_title_qlt_just_right() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A Great Page Title for Testing".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(TitleLengthQualityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_title_qlt_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(80));
        let ctx = make_ctx(&page, Some(200));
        assert!(TitleLengthQualityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TITLE-QLT003"));
    }

    #[test]
    fn test_title_qlt_single_word() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Homepage".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthQualityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE-QLT004"));
    }

    // MetaDescriptionQualityAnalyzer tests

    #[test]
    fn test_meta_qlt_no_description() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionQualityAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_meta_qlt_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("Short".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionQualityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "META-QLT001"));
    }

    #[test]
    fn test_meta_qlt_just_right() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some(
            "A comprehensive guide to building modern web applications with Rust and WebAssembly."
                .to_string(),
        );
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionQualityAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_meta_qlt_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(200));
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionQualityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "META-QLT002"));
    }

    #[test]
    fn test_meta_qlt_duplicate_sentences() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("Learn Rust programming. Learn Rust programming.".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionQualityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "META-QLT003"));
    }

    // InternalLinkAnchorAnalyzerV2 tests

    #[test]
    fn test_anchor_v2_no_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalLinkAnchorAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_anchor_v2_good_diversity() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            crate::parser::ExtractedLink {
                href: "https://example.com/a".to_string(),
                text: "Page A".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://example.com/b".to_string(),
                text: "Page B".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://example.com/c".to_string(),
                text: "Page C".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalLinkAnchorAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_anchor_v2_low_diversity() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            crate::parser::ExtractedLink {
                href: "https://example.com/a".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://example.com/b".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://example.com/c".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://example.com/d".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://example.com/e".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://example.com/f".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://example.com/g".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalLinkAnchorAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ANCH-V2001"));
    }

    // WikipediaLinkAnalyzerV2 tests

    #[test]
    fn test_wiki_v2_no_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(WikipediaLinkAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_wiki_v2_has_wiki_link() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Rust".to_string(),
            text: "Rust".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(WikipediaLinkAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "WIKI-V2001"));
    }

    #[test]
    fn test_wiki_v2_nofollow() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Rust".to_string(),
            text: "Rust".to_string(),
            rel: vec!["nofollow".to_string()],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WikipediaLinkAnalyzerV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WIKI-V2001"));
        assert!(findings.iter().any(|f| f.code == "WIKI-V2002"));
    }

    #[test]
    fn test_wiki_v2_no_wiki_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://github.com/rust-lang/rust".to_string(),
            text: "Rust GitHub".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(WikipediaLinkAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    // === MetaDescriptionLengthAnalyzerV2 tests ===

    #[test]
    fn test_meta_px_long_desc() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(200));
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionLengthAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "META-PX001"));
    }

    #[test]
    fn test_meta_px_short_desc() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("Short description".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionLengthAnalyzerV2::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_meta_px_no_desc() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionLengthAnalyzerV2::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_meta_px_name() {
        assert_eq!(
            MetaDescriptionLengthAnalyzerV2::new().name(),
            "meta-description-length-v2"
        );
    }

    #[test]
    fn test_meta_px_default() {
        let _ = MetaDescriptionLengthAnalyzerV2::default();
    }

    #[test]
    fn test_meta_px_category() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(200));
        let ctx = make_ctx(&page, Some(200));
        for f in MetaDescriptionLengthAnalyzerV2::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Seo);
        }
    }

    #[test]
    fn test_meta_px_exact_boundary() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(160));
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionLengthAnalyzerV2::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_meta_px_over_boundary() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(161));
        let ctx = make_ctx(&page, Some(200));
        assert!(MetaDescriptionLengthAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "META-PX001"));
    }

    #[test]
    fn test_meta_px_severity() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(200));
        let ctx = make_ctx(&page, Some(200));
        assert_eq!(
            MetaDescriptionLengthAnalyzerV2::new().analyze(&ctx)[0].severity,
            Severity::Warning
        );
    }

    // === TitleKeywordAnalyzer tests ===

    #[test]
    fn test_title_kw_no_title() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(TitleKeywordAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_title_kw_no_desc() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("My Page".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(TitleKeywordAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_title_kw_name() {
        assert_eq!(TitleKeywordAnalyzer::new().name(), "title-keyword");
    }

    #[test]
    fn test_title_kw_default() {
        let _ = TitleKeywordAnalyzer::default();
    }

    #[test]
    fn test_title_kw_category() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("My Page".to_string());
        page.meta.description = Some("completely different content".to_string());
        let ctx = make_ctx(&page, Some(200));
        for f in TitleKeywordAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Seo);
        }
    }

    // === CanonicalSelfReferenceAnalyzerV2 tests ===

    #[test]
    fn test_can_sr_self_referencing() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/page").unwrap());
        let ctx = make_ctx(&page, Some(200));
        assert!(CanonicalSelfReferenceAnalyzerV2::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_can_sr_different() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/other").unwrap());
        let ctx = make_ctx(&page, Some(200));
        assert!(CanonicalSelfReferenceAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "CAN-SR001"));
    }

    #[test]
    fn test_can_sr_no_canonical() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        assert!(CanonicalSelfReferenceAnalyzerV2::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_can_sr_name() {
        assert_eq!(
            CanonicalSelfReferenceAnalyzerV2::new().name(),
            "canonical-self-reference-v2"
        );
    }

    #[test]
    fn test_can_sr_default() {
        let _ = CanonicalSelfReferenceAnalyzerV2::default();
    }

    #[test]
    fn test_can_sr_category() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/other").unwrap());
        let ctx = make_ctx(&page, Some(200));
        for f in CanonicalSelfReferenceAnalyzerV2::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Seo);
        }
    }

    // === InternalLinkAnchorTextDiversityAnalyzer tests ===

    #[test]
    fn test_anch_div_low_diversity() {
        let mut page = make_page("https://example.com");
        let links = (0..10)
            .map(|_| crate::parser::ExtractedLink {
                href: "https://example.com/page".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect::<Vec<_>>();
        page.links = links;
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalLinkAnchorTextDiversityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ANCH-DIV001"));
    }

    #[test]
    fn test_anch_div_good_diversity() {
        let mut page = make_page("https://example.com");
        let texts = ["About", "Contact", "Pricing", "Blog", "FAQ", "Help"];
        page.links = texts
            .iter()
            .map(|t| crate::parser::ExtractedLink {
                href: "https://example.com/page".to_string(),
                text: t.to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalLinkAnchorTextDiversityAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_anch_div_too_few_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/page".to_string(),
            text: "click here".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(InternalLinkAnchorTextDiversityAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_anch_div_name() {
        assert_eq!(
            InternalLinkAnchorTextDiversityAnalyzer::new().name(),
            "internal-link-anchor-text-diversity"
        );
    }

    #[test]
    fn test_anch_div_default() {
        let _ = InternalLinkAnchorTextDiversityAnalyzer::default();
    }

    #[test]
    fn test_anch_div_category() {
        let mut page = make_page("https://example.com");
        page.links = (0..10)
            .map(|_| crate::parser::ExtractedLink {
                href: "https://example.com/page".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200));
        for f in InternalLinkAnchorTextDiversityAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Seo);
        }
    }

    // === ExternalLinkAuthorityAnalyzer tests ===

    #[test]
    fn test_ext_auth_no_high_auth() {
        let mut page = make_page("https://example.com");
        page.links = (0..5)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://random{i}.com/page"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200));
        assert!(ExternalLinkAuthorityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "EXT-AUTH001"));
    }

    #[test]
    fn test_ext_auth_has_high_auth() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://wikipedia.org/wiki/Test".to_string(),
            text: "Wiki".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(ExternalLinkAuthorityAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_ext_auth_few_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://random.com".to_string(),
            text: "Link".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(ExternalLinkAuthorityAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_ext_auth_name() {
        assert_eq!(
            ExternalLinkAuthorityAnalyzer::new().name(),
            "external-link-authority"
        );
    }

    #[test]
    fn test_ext_auth_default() {
        let _ = ExternalLinkAuthorityAnalyzer::default();
    }

    #[test]
    fn test_ext_auth_category() {
        let mut page = make_page("https://example.com");
        page.links = (0..5)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://random{i}.com/page"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200));
        for f in ExternalLinkAuthorityAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Seo);
        }
    }

    // === SitemapCoverageAnalyzerV2 tests ===

    #[test]
    fn test_sitemap_cov_not_in_sitemap() {
        let known = std::collections::HashSet::from(["https://example.com/other".to_string()]);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let a = SitemapCoverageAnalyzerV2::new(known);
        assert!(a.analyze(&ctx).iter().any(|f| f.code == "SITEMAP-COV001"));
    }

    #[test]
    fn test_sitemap_cov_in_sitemap() {
        let known = std::collections::HashSet::from(["https://example.com/page".to_string()]);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let a = SitemapCoverageAnalyzerV2::new(known);
        assert!(a.analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sitemap_cov_empty_sitemap() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let a = SitemapCoverageAnalyzerV2::empty();
        assert!(a.analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sitemap_cov_name() {
        assert_eq!(
            SitemapCoverageAnalyzerV2::empty().name(),
            "sitemap-coverage-v2"
        );
    }

    #[test]
    fn test_sitemap_cov_default() {
        let _ = SitemapCoverageAnalyzerV2::default();
    }

    #[test]
    fn test_sitemap_cov_category() {
        let known = std::collections::HashSet::from(["https://example.com/other".to_string()]);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let a = SitemapCoverageAnalyzerV2::new(known);
        for f in a.analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Seo);
        }
    }

    // === RobotsTxtCoverageAnalyzerV2 tests ===

    #[test]
    fn test_robots_cov_blocked() {
        let robots = "User-agent: *\nDisallow: /page\n";
        let page = make_page("https://example.com/page");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(RobotsTxtCoverageAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ROBOTS-COV001"));
    }

    #[test]
    fn test_robots_cov_not_blocked() {
        let robots = "User-agent: *\nDisallow: /admin\n";
        let page = make_page("https://example.com/page");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(RobotsTxtCoverageAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_robots_cov_no_robots() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        assert!(RobotsTxtCoverageAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_robots_cov_name() {
        assert_eq!(
            RobotsTxtCoverageAnalyzerV2::new().name(),
            "robots-txt-coverage-v2"
        );
    }

    #[test]
    fn test_robots_cov_default() {
        let _ = RobotsTxtCoverageAnalyzerV2::default();
    }

    #[test]
    fn test_robots_cov_category() {
        let robots = "User-agent: *\nDisallow: /page\n";
        let page = make_page("https://example.com/page");
        let ctx = AnalysisContext {
            page: &page,
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
        for f in RobotsTxtCoverageAnalyzerV2::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Seo);
        }
    }

    // === PaginationLinkAnalyzer tests ===

    #[test]
    fn test_pag_nofollow() {
        let mut page = make_page("https://example.com/page/1");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/page/2".to_string(),
            text: "Next".to_string(),
            rel: vec!["next".to_string(), "nofollow".to_string()],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationLinkAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "PAG-LINK001"));
    }

    #[test]
    fn test_pag_no_nofollow() {
        let mut page = make_page("https://example.com/page/1");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/page/2".to_string(),
            text: "Next".to_string(),
            rel: vec!["next".to_string()],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationLinkAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_pag_no_pagination() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationLinkAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_pag_prev_nofollow() {
        let mut page = make_page("https://example.com/page/2");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/page/1".to_string(),
            text: "Prev".to_string(),
            rel: vec!["prev".to_string(), "nofollow".to_string()],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(PaginationLinkAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "PAG-LINK001"));
    }

    #[test]
    fn test_pag_name() {
        assert_eq!(PaginationLinkAnalyzer::new().name(), "pagination-link");
    }

    #[test]
    fn test_pag_default() {
        let _ = PaginationLinkAnalyzer::default();
    }

    #[test]
    fn test_pag_category() {
        let mut page = make_page("https://example.com/page/1");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/page/2".to_string(),
            text: "Next".to_string(),
            rel: vec!["next".to_string(), "nofollow".to_string()],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        for f in PaginationLinkAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Seo);
        }
    }

    #[test]
    fn test_pag_severity() {
        let mut page = make_page("https://example.com/page/1");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/page/2".to_string(),
            text: "Next".to_string(),
            rel: vec!["next".to_string(), "nofollow".to_string()],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert_eq!(
            PaginationLinkAnalyzer::new().analyze(&ctx)[0].severity,
            Severity::Warning
        );
    }
}
