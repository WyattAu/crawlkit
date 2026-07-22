use std::collections::{HashMap, HashSet};
use std::time::Duration;
use url::Url;

use crate::parser::ParsedPage;
use crate::storage::{IssueCategory, Severity};
use crate::{CrawlConfig, RedirectHop};

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Context for analyzing a page, bundling parsed HTML with HTTP metadata.
pub struct AnalysisContext<'a> {
    /// The parsed page content.
    pub page: &'a ParsedPage,
    /// HTTP status code (if fetched).
    pub status_code: Option<u16>,
    /// Response headers.
    pub headers: &'a [(String, String)],
    /// Response time (if measured).
    pub response_time: Option<Duration>,
    /// Redirect chain hops (if any).
    pub redirect_chain: &'a [RedirectHop],
}

/// A finding/issue detected by an analyzer.
#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub category: IssueCategory,
    pub code: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub recommendation: String,
}

/// Trait for page analyzers.
pub trait Analyzer: Send + Sync {
    /// Returns the human-readable name of this analyzer.
    fn name(&self) -> &str;

    /// Analyze a page and return any findings/issues.
    fn analyze(&self, ctx: &AnalysisContext, config: &CrawlConfig) -> Vec<Finding>;
}

// ---------------------------------------------------------------------------
// 1. HTTP Status Analyzer
// ---------------------------------------------------------------------------

pub struct HttpStatusAnalyzer;

impl HttpStatusAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a 200 response looks like an error page (soft 404).
    #[allow(dead_code)]
    pub(crate) fn is_soft_404(body: &str) -> bool {
        let lower = body.to_lowercase();
        let indicators = [
            "page not found",
            "404 not found",
            "the page you requested",
            "does not exist",
            "has been removed",
            "no longer available",
            "error 404",
            "not found",
            "sorry, we couldn't find",
            "this page is no longer available",
            "the requested url was not found",
        ];
        indicators.iter().any(|ind| lower.contains(ind))
    }

    fn status_category(code: u16) -> &'static str {
        match code {
            200..=299 => "success",
            300..=399 => "redirection",
            400..=499 => "client_error",
            500..=599 => "server_error",
            _ => "unknown",
        }
    }
}

impl Default for HttpStatusAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HttpStatusAnalyzer {
    fn name(&self) -> &str {
        "http-status"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let status = match ctx.status_code {
            Some(s) => s,
            None => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Http,
                    code: "HTTP001".to_string(),
                    title: "Missing status code".to_string(),
                    description: "No HTTP status code was recorded for this page.".to_string(),
                    url: url.clone(),
                    recommendation: "Ensure the page is fetched and the status code is recorded."
                        .to_string(),
                });
                return findings;
            }
        };

        // Record response time if present
        if let Some(rt) = ctx.response_time {
            if rt > Duration::from_secs(5) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Performance,
                    code: "HTTP002".to_string(),
                    title: "Slow response time".to_string(),
                    description: format!("Response took {rt:?}, which exceeds 5 seconds."),
                    url: url.clone(),
                    recommendation: "Optimize server response time. Consider caching, CDN, or \
                                     reducing server-side processing."
                        .to_string(),
                });
            }
        }

        match status {
            200 => {
                // Check for soft 404
                let body_lower = ctx.page.word_count;
                // Soft 404 heuristic: very low word count on a 200 page can indicate error page
                if body_lower == 0 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Http,
                        code: "HTTP003".to_string(),
                        title: "Possible soft 404 — empty body".to_string(),
                        description: "Page returned 200 but has no content. This may indicate \
                                     a soft 404."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Verify the page renders correctly. Fix server-side \
                                         rendering issues."
                            .to_string(),
                    });
                }
            }
            301 | 302 | 307 | 308 => {
                // Redirects are handled by RedirectChainAnalyzer
            }
            404 => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Http,
                    code: "HTTP004".to_string(),
                    title: "Page not found (404)".to_string(),
                    description: "The page returned a 404 status code.".to_string(),
                    url: url.clone(),
                    recommendation: "Remove links to this page or redirect to a valid URL."
                        .to_string(),
                });
            }
            500..=599 => {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Http,
                    code: "HTTP005".to_string(),
                    title: format!("Server error ({status})"),
                    description: format!(
                        "The page returned a {status} status code, indicating a server error."
                    ),
                    url: url.clone(),
                    recommendation: "Fix server-side errors. Check application logs for details."
                        .to_string(),
                });
            }
            _ => {
                // Info-level for other codes
            }
        }

        // Check for status category
        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Http,
            code: "HTTP006".to_string(),
            title: format!(
                "Status category: {}",
                Self::status_category(status)
            ),
            description: format!(
                "HTTP {status} — categorized as {}.",
                Self::status_category(status)
            ),
            url: url.clone(),
            recommendation: String::new(),
        });

        findings
    }
}

// ---------------------------------------------------------------------------
// 2. Redirect Chain Tracker
// ---------------------------------------------------------------------------

pub struct RedirectChainAnalyzer;

impl RedirectChainAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn is_mixed_protocol(from: &Url, to: &Url) -> bool {
        from.scheme() != to.scheme()
    }
}

impl Default for RedirectChainAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for RedirectChainAnalyzer {
    fn name(&self) -> &str {
        "redirect-chain"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hops = ctx.redirect_chain;

        if hops.is_empty() {
            return findings;
        }

        // Flag chains longer than 5 hops
        if hops.len() > 5 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Http,
                code: "REDIR001".to_string(),
                title: "Long redirect chain".to_string(),
                description: format!(
                    "Redirect chain has {} hops, exceeding the recommended maximum of 5.",
                    hops.len()
                ),
                url: url.clone(),
                recommendation: "Reduce redirect hops. Update links to point directly to the \
                                 final URL."
                    .to_string(),
            });
        }

        // Detect loops
        let mut seen_urls: HashSet<String> = HashSet::new();
        seen_urls.insert(hops[0].from.to_string());
        let mut has_loop = false;
        for hop in hops {
            if !seen_urls.insert(hop.to.to_string()) {
                has_loop = true;
                break;
            }
        }
        if has_loop {
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Http,
                code: "REDIR002".to_string(),
                title: "Redirect loop detected".to_string(),
                description: "The redirect chain contains a loop, which will prevent the page \
                             from loading."
                    .to_string(),
                url: url.clone(),
                recommendation: "Fix the redirect chain to eliminate the loop. Ensure each \
                                 redirect points to a unique destination."
                    .to_string(),
            });
        }

        // Detect mixed-protocol redirects
        for hop in hops {
            if Self::is_mixed_protocol(&hop.from, &hop.to) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "REDIR003".to_string(),
                    title: "Mixed-protocol redirect".to_string(),
                    description: format!(
                        "Redirect from {} ({}) to {} ({}) changes protocol.",
                        hop.from,
                        hop.from.scheme(),
                        hop.to,
                        hop.to.scheme()
                    ),
                    url: url.clone(),
                    recommendation: "Use consistent protocol (prefer HTTPS) for all redirects."
                        .to_string(),
                });
                break;
            }
        }

        // Check for unnecessary redirect (direct URL works)
        if hops.len() == 1 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Http,
                code: "REDIR004".to_string(),
                title: "Single redirect detected".to_string(),
                description: format!(
                    "URL redirects from {} to {}.",
                    hops[0].from, hops[0].to
                ),
                url: url.clone(),
                recommendation: "Consider updating inbound links to point directly to the \
                                 final URL to eliminate the redirect."
                    .to_string(),
            });
        }

        findings
    }
}

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
// 4. Hreflang Validator
// ---------------------------------------------------------------------------

pub struct HreflangValidator;

impl HreflangValidator {
    pub fn new() -> Self {
        Self
    }

    /// Check if a locale code looks valid (language[-region] format).
    fn is_valid_locale(code: &str) -> bool {
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
        Self { known_urls, entries }
    }

    pub fn empty() -> Self {
        Self {
            known_urls: HashSet::new(),
            entries: Vec::new(),
        }
    }

    /// Validate a lastmod date format (ISO 8601).
    fn is_valid_lastmod(lastmod: &str) -> bool {
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
    fn is_valid_changefreq(freq: &str) -> bool {
        matches!(
            freq,
            "always" | "hourly" | "daily" | "weekly" | "monthly" | "yearly" | "never"
        )
    }

    /// Validate priority value (0.0 - 1.0).
    fn is_valid_priority(p: f64) -> bool {
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
                recommendation: "Provide sitemap data to enable sitemap validation."
                    .to_string(),
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
                        recommendation: "Set priority to a value between 0.0 and 1.0."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 6. Robots.txt Analyzer
// ---------------------------------------------------------------------------

/// A parsed robots.txt rule.
#[derive(Debug, Clone)]
pub struct RobotsRule {
    pub user_agent: String,
    pub disallowed_paths: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub crawl_delay: Option<f64>,
    pub sitemaps: Vec<String>,
}

pub struct RobotsTxtAnalyzer {
    rules: Vec<RobotsRule>,
    sitemap_urls: Vec<String>,
}

impl RobotsTxtAnalyzer {
    pub fn new(rules: Vec<RobotsRule>, sitemap_urls: Vec<String>) -> Self {
        Self {
            rules,
            sitemap_urls,
        }
    }

    pub fn empty() -> Self {
        Self {
            rules: Vec::new(),
            sitemap_urls: Vec::new(),
        }
    }

    /// Check if a path is disallowed for a user-agent.
    fn is_disallowed(path: &str, disallowed: &[String]) -> bool {
        for pattern in disallowed {
            if path.starts_with(pattern.as_str()) {
                return true;
            }
        }
        false
    }

    /// Check if a path is explicitly allowed for a user-agent.
    fn is_allowed(path: &str, allowed: &[String]) -> bool {
        for pattern in allowed {
            if path.starts_with(pattern.as_str()) {
                return true;
            }
        }
        false
    }
}

impl Default for RobotsTxtAnalyzer {
    fn default() -> Self {
        Self::empty()
    }
}

impl Analyzer for RobotsTxtAnalyzer {
    fn name(&self) -> &str {
        "robots-txt"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if self.rules.is_empty() {
            return findings;
        }

        let page_url = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => return findings,
        };

        let path = page_url.path();

        // Check rules for the default user-agent and the configured user-agent
        for rule in &self.rules {
            let user_agent = &rule.user_agent;
            let is_match = user_agent == "*" || user_agent.contains("crawlkit");

            if !is_match {
                continue;
            }

            // Check disallow
            if Self::is_disallowed(path, &rule.disallowed_paths) {
                // Check if an allow rule overrides it
                if Self::is_allowed(path, &rule.allowed_paths) {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "ROBOT001".to_string(),
                        title: "Path allowed by robots.txt override".to_string(),
                        description: format!(
                            "Path \"{path}\" was disallowed but is explicitly allowed by a \
                             more specific rule for user-agent \"{user_agent}\"."
                        ),
                        url: url.clone(),
                        recommendation: "This is informational. The path is crawlable due to an \
                                         allow rule override."
                            .to_string(),
                    });
                } else {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Seo,
                        code: "ROBOT002".to_string(),
                        title: "Path disallowed by robots.txt".to_string(),
                        description: format!(
                            "Path \"{path}\" is disallowed for user-agent \"{user_agent}\"."
                        ),
                        url: url.clone(),
                        recommendation: "Remove the disallow rule if this page should be \
                                         crawled, or update internal links to avoid disallowed \
                                         paths."
                            .to_string(),
                    });
                }
            }

            // Check crawl-delay
            if let Some(delay) = rule.crawl_delay {
                if delay > 10.0 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Performance,
                        code: "ROBOT003".to_string(),
                        title: "High crawl-delay value".to_string(),
                        description: format!(
                            "Crawl-delay of {delay}s for user-agent \"{user_agent}\" may \
                             significantly slow crawling."
                        ),
                        url: url.clone(),
                        recommendation: "Consider reducing crawl-delay if faster crawling is \
                                         needed."
                            .to_string(),
                    });
                }
            }
        }

        // Validate sitemap references in robots.txt
        for sitemap_url in &self.sitemap_urls {
            if Url::parse(sitemap_url).is_err() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ROBOT004".to_string(),
                    title: "Invalid sitemap URL in robots.txt".to_string(),
                    description: format!(
                        "The sitemap URL \"{sitemap_url}\" in robots.txt is not a valid URL."
                    ),
                    url: url.clone(),
                    recommendation: "Fix the sitemap URL in robots.txt to be a valid absolute URL."
                        .to_string(),
                });
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
                    recommendation: "Add a descriptive title tag (30-60 characters)."
                        .to_string(),
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
                } else if len > 160 {
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

        // --- Open Graph completeness ---
        let og_required = [
            ("og:title", &meta.og.title),
            ("og:image", &meta.og.image),
            ("og:url", &meta.og.url),
            ("og:type", &meta.og.r#type),
        ];

        for (tag_name, value) in &og_required {
            if value.is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Social,
                    code: "META007".to_string(),
                    title: format!("Missing {tag_name} tag"),
                    description: format!(
                        "The Open Graph tag {tag_name} is missing. Social media previews \
                         may be incomplete."
                    ),
                    url: url.clone(),
                    recommendation: format!(
                        "Add <meta property=\"{tag_name}\" content=\"...\"> to improve social \
                         sharing."
                    ),
                });
            }
        }

        // --- Twitter Card completeness ---
        let twitter_required = [
            ("twitter:card", &meta.twitter.card),
            ("twitter:title", &meta.twitter.title),
            ("twitter:image", &meta.twitter.image),
        ];

        for (tag_name, value) in &twitter_required {
            if value.is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Social,
                    code: "META008".to_string(),
                    title: format!("Missing {tag_name} tag"),
                    description: format!(
                        "The Twitter Card tag {tag_name} is missing. Twitter/X previews \
                         may be incomplete."
                    ),
                    url: url.clone(),
                    recommendation: format!(
                        "Add <meta name=\"{tag_name}\" content=\"...\"> to improve Twitter/X \
                         sharing."
                    ),
                });
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
                recommendation: "Add at least one H1 heading to define the page topic."
                    .to_string(),
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
// Analyzer Registry
// ---------------------------------------------------------------------------

pub struct AnalyzerRegistry {
    analyzers: Vec<Box<dyn Analyzer>>,
}

impl AnalyzerRegistry {
    /// Create a registry with all default analyzers.
    pub fn new(_config: &CrawlConfig) -> Self {
        Self {
            analyzers: vec![
                Box::new(HttpStatusAnalyzer::new()),
                Box::new(RedirectChainAnalyzer::new()),
                Box::new(CanonicalUrlValidator::new()),
                Box::new(HreflangValidator::new()),
                Box::new(SitemapAnalyzer::empty()),
                Box::new(RobotsTxtAnalyzer::empty()),
                Box::new(MetaTagAnalyzer::new()),
                Box::new(HeadingHierarchyAnalyzer::new()),
            ],
        }
    }

    /// Create a registry with custom analyzers.
    pub fn with_analyzers(analyzers: Vec<Box<dyn Analyzer>>) -> Self {
        Self { analyzers }
    }

    /// Add an analyzer to the registry.
    pub fn register(&mut self, analyzer: Box<dyn Analyzer>) {
        self.analyzers.push(analyzer);
    }

    /// Run all analyzers on a page and collect findings.
    pub fn analyze(&self, ctx: &AnalysisContext, config: &CrawlConfig) -> Vec<Finding> {
        self.analyzers
            .iter()
            .flat_map(|a| a.analyze(ctx, config))
            .collect()
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::Heading;
    use crate::storage::{IssueCategory, Severity};

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
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
        }
    }

    // ---- HttpStatusAnalyzer ----

    #[test]
    fn test_http_status_200() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = HttpStatusAnalyzer::new().analyze(&ctx, &default_config());
        // Should have info about status category
        assert!(findings.iter().any(|f| f.code == "HTTP006"));
    }

    #[test]
    fn test_http_status_404() {
        let page = make_page("https://example.com/missing");
        let ctx = make_ctx(&page, Some(404));
        let findings = HttpStatusAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HTTP004"));
    }

    #[test]
    fn test_http_status_500() {
        let page = make_page("https://example.com/error");
        let ctx = make_ctx(&page, Some(500));
        let findings = HttpStatusAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HTTP005"));
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Critical));
    }

    #[test]
    fn test_http_status_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = HttpStatusAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HTTP001"));
    }

    #[test]
    fn test_http_status_soft_404_empty_body() {
        let mut page = make_page("https://example.com/soft404");
        page.word_count = 0;
        let ctx = make_ctx(&page, Some(200));
        let findings = HttpStatusAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HTTP003"));
    }

    #[test]
    fn test_http_status_slow_response() {
        let page = make_page("https://example.com/slow");
        let ctx = AnalysisContext {
            page: &page,
            status_code: Some(200),
            headers: &[],
            response_time: Some(Duration::from_secs(10)),
            redirect_chain: &[],
        };
        let findings = HttpStatusAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HTTP002"));
    }

    // ---- RedirectChainAnalyzer ----

    #[test]
    fn test_redirect_no_hops() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = RedirectChainAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &hops,
        };
        let findings = RedirectChainAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &hops,
        };
        let findings = RedirectChainAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "REDIR002"));
    }

    #[test]
    fn test_redirect_mixed_protocol() {
        let hops = vec![
            RedirectHop {
                from: Url::parse("http://example.com/page").unwrap(),
                to: Url::parse("https://example.com/page").unwrap(),
                status_code: 301,
            },
        ];
        let page = make_page("http://example.com/page");
        let ctx = AnalysisContext {
            page: &page,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &hops,
        };
        let findings = RedirectChainAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &hops,
        };
        let findings = RedirectChainAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "REDIR004"));
    }

    // ---- CanonicalUrlValidator ----

    #[test]
    fn test_canonical_missing() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalUrlValidator::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "CANON001"));
    }

    #[test]
    fn test_canonical_self_referencing() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/page").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalUrlValidator::new().analyze(&ctx, &default_config());
        // Self-referencing is fine — no mismatch finding
        assert!(!findings.iter().any(|f| f.code == "CANON003"));
    }

    #[test]
    fn test_canonical_mismatch() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some(Url::parse("https://example.com/other").unwrap());
        let ctx = make_ctx(&page, Some(200));
        let findings = CanonicalUrlValidator::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "CANON003"));
    }

    // ---- HreflangValidator ----

    #[test]
    fn test_hreflang_no_tags() {
        let page = make_page("https://example.com/en");
        let ctx = make_ctx(&page, Some(200));
        let findings = HreflangValidator::new().analyze(&ctx, &default_config());
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
        let findings = HreflangValidator::new().analyze(&ctx, &default_config());
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
        let findings = HreflangValidator::new().analyze(&ctx, &default_config());
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
        let findings = HreflangValidator::new().analyze(&ctx, &default_config());
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
        let findings = HreflangValidator::new().analyze(&ctx, &default_config());
        // No errors for valid setup
        assert!(!findings.iter().any(|f| f.severity == Severity::Error));
    }

    // ---- SitemapAnalyzer ----

    #[test]
    fn test_sitemap_no_data() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = SitemapAnalyzer::empty().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "SITEMAP001"));
    }

    #[test]
    fn test_sitemap_url_not_found() {
        let mut known = HashSet::new();
        known.insert("https://example.com/other".to_string());
        let analyzer = SitemapAnalyzer::new(known, Vec::new());
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "SITEMAP002"));
    }

    #[test]
    fn test_sitemap_url_found() {
        let mut known = HashSet::new();
        known.insert("https://example.com/page".to_string());
        let analyzer = SitemapAnalyzer::new(known, Vec::new());
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx, &default_config());
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
        let findings = analyzer.analyze(&ctx, &default_config());
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
        let findings = analyzer.analyze(&ctx, &default_config());
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
        let findings = analyzer.analyze(&ctx, &default_config());
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
        let findings = analyzer.analyze(&ctx, &default_config());
        // No errors for valid metadata
        assert!(!findings.iter().any(|f| f.severity == Severity::Error));
    }

    // ---- RobotsTxtAnalyzer ----

    #[test]
    fn test_robots_empty() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = RobotsTxtAnalyzer::empty().analyze(&ctx, &default_config());
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
        let findings = analyzer.analyze(&ctx, &default_config());
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
        let findings = analyzer.analyze(&ctx, &default_config());
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
        let findings = analyzer.analyze(&ctx, &default_config());
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
        let findings = analyzer.analyze(&ctx, &default_config());
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
        let findings = analyzer.analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "ROBOT004"));
    }

    // ---- MetaTagAnalyzer ----

    #[test]
    fn test_meta_missing_title() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "META001"));
    }

    #[test]
    fn test_meta_title_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Hi".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "META002"));
    }

    #[test]
    fn test_meta_title_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(80));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "META003"));
    }

    #[test]
    fn test_meta_title_just_right() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(45));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "META002"));
        assert!(!findings.iter().any(|f| f.code == "META003"));
    }

    #[test]
    fn test_meta_missing_description() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "META004"));
    }

    #[test]
    fn test_meta_description_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("Short".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "META005"));
    }

    #[test]
    fn test_meta_description_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(200));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "META006"));
    }

    #[test]
    fn test_meta_missing_og_tags() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
        // Should flag og:title, og:image, og:url, og:type
        let og_codes: Vec<&str> = findings
            .iter()
            .filter(|f| f.code == "META007")
            .map(|f| f.title.as_str())
            .collect();
        assert!(og_codes.len() >= 4);
    }

    #[test]
    fn test_meta_missing_twitter_tags() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
        // Should flag twitter:card, twitter:title, twitter:image
        let tw_count = findings.iter().filter(|f| f.code == "META008").count();
        assert!(tw_count >= 3);
    }

    #[test]
    fn test_meta_missing_viewport() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = MetaTagAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HEAD001"));
    }

    #[test]
    fn test_heading_missing_h1() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading { level: 2, text: "Section".to_string(), length: 7 },
            Heading { level: 3, text: "Sub".to_string(), length: 3 },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HEAD002"));
    }

    #[test]
    fn test_heading_multiple_h1() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading { level: 1, text: "First".to_string(), length: 5 },
            Heading { level: 1, text: "Second".to_string(), length: 6 },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HEAD003"));
    }

    #[test]
    fn test_heading_skipped_level() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading { level: 1, text: "Title".to_string(), length: 5 },
            Heading { level: 3, text: "Skipped H2".to_string(), length: 10 },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HEAD004"));
    }

    #[test]
    fn test_heading_valid_hierarchy() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading { level: 1, text: "Main".to_string(), length: 4 },
            Heading { level: 2, text: "Section".to_string(), length: 7 },
            Heading { level: 2, text: "Section 2".to_string(), length: 9 },
            Heading { level: 3, text: "Sub".to_string(), length: 3 },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "HEAD004"));
    }

    #[test]
    fn test_heading_deep_hierarchy() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading { level: 1, text: "H1".to_string(), length: 2 },
            Heading { level: 2, text: "H2".to_string(), length: 2 },
            Heading { level: 3, text: "H3".to_string(), length: 2 },
            Heading { level: 4, text: "H4".to_string(), length: 2 },
            Heading { level: 5, text: "H5".to_string(), length: 2 },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HEAD005"));
    }

    // ---- AnalyzerRegistry ----

    #[test]
    fn test_registry_default() {
        let config = default_config();
        let registry = AnalyzerRegistry::new(&config);
        assert_eq!(registry.len(), 8);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_analyze() {
        let config = default_config();
        let registry = AnalyzerRegistry::new(&config);
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Good Title Here for SEO".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = registry.analyze(&ctx, &config);
        // Should produce findings from multiple analyzers
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_registry_custom() {
        struct DummyAnalyzer;
        impl Analyzer for DummyAnalyzer {
            fn name(&self) -> &str { "dummy" }
            fn analyze(&self, _ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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
        let findings = registry.analyze(&ctx, &default_config());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "DUMMY001");
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
        let findings = registry.analyze(&ctx, &config);

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
            Heading { level: 1, text: "Main Topic".to_string(), length: 10 },
            Heading { level: 2, text: "Section".to_string(), length: 7 },
        ];

        let ctx = make_ctx(&page, Some(200));
        let findings = registry.analyze(&ctx, &config);

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
}
