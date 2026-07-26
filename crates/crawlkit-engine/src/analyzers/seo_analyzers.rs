use std::collections::{HashMap, HashSet};
use url::Url;

use crate::storage::{IssueCategory, Severity};
use crate::CrawlConfig;

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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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
                    && (region.len() == 2 && region.chars().all(|c| c.is_ascii_alphabetic())
                        || region.len() == 4 && region.chars().all(|c| c.is_ascii_digit()))
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if self.known_urls.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "SITEMAP001".to_string(),
                title: "No sitemap data available".to_string(),
                description: "No sitemap URLs were loaded for validation.".to_string(),
                url: url.clone(),
                recommendation: "Provide sitemap data to enable sitemap validation.".to_string(),
            });
            return findings;
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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
// 13. Word Count Analyzer
// ---------------------------------------------------------------------------

pub struct WordCountAnalyzer;

impl WordCountAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Extract visible text from headings (proxy for page text).
    fn visible_text(ctx: &AnalysisContext) -> String {
        ctx.page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let text = Self::visible_text(ctx);
        let word_count = ctx.page.word_count;
        let char_count = text.chars().count();

        // Count sentences
        let sentence_count = if text.trim().is_empty() {
            0
        } else {
            text.chars()
                .filter(|&c| c == '.' || c == '!' || c == '?')
                .count()
                .max(1)
        };

        let avg_words_per_sentence = if sentence_count > 0 {
            word_count as f64 / sentence_count as f64
        } else {
            0.0
        };

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "WC001".to_string(),
            title: "Word count statistics".to_string(),
            description: format!(
                "Words: {word_count}, Characters: {char_count}, Sentences: {sentence_count}, \
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

    fn compute_tfidf(
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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

    pub(crate) fn detect_multilang_content(
        hreflang: &[crate::meta::HreflangTag],
        html_lang: &Option<String>,
    ) -> bool {
        if !hreflang.is_empty() {
            return true;
        }
        if let Some(lang) = html_lang {
            let parts: Vec<&str> = lang.split('-').collect();
            if parts.len() >= 2 {
                return true;
            }
        }
        false
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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

        let is_multilang = Self::detect_multilang_content(hreflang_tags, &ctx.page.html_lang);
        if is_multilang {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "ISEO006".to_string(),
                title: "Multi-language content detected".to_string(),
                description: "Page appears to be part of a multilingual setup. Ensure all \
                              language variants cross-reference each other via hreflang."
                    .to_string(),
                url: url.clone(),
                recommendation: "Each language version should have reciprocal hreflang tags \
                                 pointing to all other versions."
                    .to_string(),
            });
        }

        findings
    }
}
