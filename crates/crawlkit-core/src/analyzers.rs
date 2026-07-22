use std::collections::{HashMap, HashSet};
use std::time::Duration;
use url::Url;

use crate::parser::ParsedPage;
use crate::storage::{IssueCategory, Severity};
use crate::{CrawlConfig, RedirectHop};

// ---------------------------------------------------------------------------
// Security Header Analyzer types
// ---------------------------------------------------------------------------

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
            title: format!("Status category: {}", Self::status_category(status)),
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
                description: format!("URL redirects from {} to {}.", hops[0].from, hops[0].to),
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
                        recommendation: "Expand the description to 120-160 characters.".to_string(),
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
            if link.text.trim().is_empty() {
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
            } else if link.text.len() < 3 {
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
// 10. Image Analyzer
// ---------------------------------------------------------------------------

/// Detailed image information for analysis.
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub src: String,
    pub alt: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
    pub file_size: Option<u64>,
    pub has_alt: bool,
    pub is_lazy_loaded: bool,
}

pub struct ImageAnalyzer;

impl ImageAnalyzer {
    pub fn new() -> Self {
        Self
    }

    #[allow(dead_code)]
    pub(crate) fn detect_format(src: &str) -> Option<String> {
        let path = src.split('?').next()?;
        let ext = path.rsplit('.').next()?;
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => Some("jpeg".to_string()),
            "png" => Some("png".to_string()),
            "gif" => Some("gif".to_string()),
            "webp" => Some("webp".to_string()),
            "avif" => Some("avif".to_string()),
            "svg" => Some("svg".to_string()),
            "bmp" => Some("bmp".to_string()),
            "ico" => Some("ico".to_string()),
            "tiff" | "tif" => Some("tiff".to_string()),
            _ => None,
        }
    }
}

impl Default for ImageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ImageAnalyzer {
    fn name(&self) -> &str {
        "image-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.images.is_empty() {
            return findings;
        }

        let mut total_lazy = 0u32;
        let mut total_with_dimensions = 0u32;

        for img in &ctx.page.images {
            // 2.4 — Missing alt text
            if !img.has_alt {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "IMG001".to_string(),
                    title: "Image missing alt text".to_string(),
                    description: format!("Image \"{}\" has no alt attribute.", img.src),
                    url: url.clone(),
                    recommendation: "Add descriptive alt text to improve accessibility and SEO."
                        .to_string(),
                });
            }

            // Oversized images (> 500 KB) — flagged as info since file_size is not
            // available from HTML parsing. This check is a placeholder for when
            // file_size is available from HTTP response headers.

            if img.is_lazy_loaded {
                total_lazy += 1;
            }

            if img.width.is_some() && img.height.is_some() {
                total_with_dimensions += 1;
            }
        }

        // Lazy loading summary
        if total_lazy > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "IMG003".to_string(),
                title: "Lazy-loaded images".to_string(),
                description: format!(
                    "{} of {} images use lazy loading.",
                    total_lazy,
                    ctx.page.images.len()
                ),
                url: url.clone(),
                recommendation: "Verify lazy loading is only applied to below-the-fold images."
                    .to_string(),
            });
        }

        // Dimension summary
        let missing_dimensions = ctx.page.images.len() as u32 - total_with_dimensions;
        if missing_dimensions > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "IMG004".to_string(),
                title: "Images missing dimensions".to_string(),
                description: format!(
                    "{} of {} images are missing width/height attributes.",
                    missing_dimensions,
                    ctx.page.images.len()
                ),
                url: url.clone(),
                recommendation: "Specify width and height to prevent layout shifts (CLS)."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 11. Structured Data Validator
// ---------------------------------------------------------------------------

/// Required properties per Schema.org type (subset).
const REQUIRED_PROPERTIES: &[(&str, &[&str])] = &[
    ("Article", &["headline", "author"]),
    ("NewsArticle", &["headline", "author"]),
    ("BlogPosting", &["headline", "author"]),
    ("Product", &["name"]),
    ("Organization", &["name"]),
    ("LocalBusiness", &["name", "address"]),
    ("WebPage", &["name"]),
    ("BreadcrumbList", &["itemListElement"]),
    ("FAQPage", &["mainEntity"]),
    ("HowTo", &["name"]),
    ("Event", &["name", "startDate"]),
    ("Recipe", &["name"]),
    ("VideoObject", &["name", "embedUrl"]),
    ("SoftwareApplication", &["name"]),
    ("Book", &["name"]),
    ("MusicAlbum", &["name"]),
    ("Movie", &["name"]),
];

/// Recognized Schema.org types for validation.
const RECOGNIZED_TYPES: &[&str] = &[
    "Article",
    "NewsArticle",
    "BlogPosting",
    "Product",
    "Organization",
    "LocalBusiness",
    "WebSite",
    "WebPage",
    "BreadcrumbList",
    "FAQPage",
    "HowTo",
    "Event",
    "Recipe",
    "VideoObject",
    "SoftwareApplication",
    "Book",
    "MusicAlbum",
    "Movie",
    "Person",
    "Place",
    "ItemList",
    "AggregateRating",
    "Review",
    "Offer",
    "Brand",
];

pub struct StructuredDataValidator;

impl StructuredDataValidator {
    pub fn new() -> Self {
        Self
    }

    fn required_properties(schema_type: &str) -> &'static [&'static str] {
        REQUIRED_PROPERTIES
            .iter()
            .find(|(t, _)| *t == schema_type)
            .map(|(_, props)| *props)
            .unwrap_or(&[])
    }
}

impl Default for StructuredDataValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for StructuredDataValidator {
    fn name(&self) -> &str {
        "structured-data"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.structured_data.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "SD001".to_string(),
                title: "No structured data found".to_string(),
                description: "No JSON-LD structured data was found on this page.".to_string(),
                url: url.clone(),
                recommendation: "Add relevant Schema.org JSON-LD markup to enhance search \
                                 results."
                    .to_string(),
            });
            return findings;
        }

        for sd in &ctx.page.structured_data {
            // 2.5 — Validate @context
            match &sd.context {
                None => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "SD002".to_string(),
                        title: "Missing @context".to_string(),
                        description: "JSON-LD block is missing the @context property.".to_string(),
                        url: url.clone(),
                        recommendation: "Add \"@context\": \"https://schema.org\" to all JSON-LD \
                                         blocks."
                            .to_string(),
                    });
                }
                Some(ctx_val) => {
                    if ctx_val != "https://schema.org" && ctx_val != "schema.org" {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "SD003".to_string(),
                            title: "Non-standard @context".to_string(),
                            description: format!(
                                "JSON-LD @context is \"{ctx_val}\" instead of \
                                 \"https://schema.org\"."
                            ),
                            url: url.clone(),
                            recommendation: "Use \"https://schema.org\" as the @context."
                                .to_string(),
                        });
                    }
                }
            }

            // Validate @type
            match &sd.r#type {
                None => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "SD004".to_string(),
                        title: "Missing @type".to_string(),
                        description: "JSON-LD block is missing the @type property.".to_string(),
                        url: url.clone(),
                        recommendation: "Add an appropriate @type to describe the content."
                            .to_string(),
                    });
                }
                Some(type_val) => {
                    if !RECOGNIZED_TYPES.contains(&type_val.as_str()) {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "SD005".to_string(),
                            title: "Unknown @type".to_string(),
                            description: format!(
                                "JSON-LD @type \"{type_val}\" is not a recognized Schema.org \
                                 type."
                            ),
                            url: url.clone(),
                            recommendation: "Verify this is a valid Schema.org type or use a \
                                             recognized type."
                                .to_string(),
                        });
                    }

                    // 2.6 — Check required properties
                    let required = Self::required_properties(type_val);
                    if !required.is_empty() {
                        let mut missing = Vec::new();
                        for prop in required {
                            if sd.data.get(*prop).is_none() {
                                missing.push(*prop);
                            }
                        }
                        if !missing.is_empty() {
                            findings.push(Finding {
                                severity: Severity::Error,
                                category: IssueCategory::Schema,
                                code: "SD006".to_string(),
                                title: "Missing required properties".to_string(),
                                description: format!(
                                    "Schema type \"{type_val}\" is missing required properties: \
                                     {}.",
                                    missing.join(", ")
                                ),
                                url: url.clone(),
                                recommendation: format!(
                                    "Add the missing properties to the \"{type_val}\" schema."
                                ),
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
// 12. Content Quality Analyzer
// ---------------------------------------------------------------------------

/// Stop words for keyword density analysis (common English words).
const STOP_WORDS: &[&str] = &[
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

pub struct ContentQualityAnalyzer;

impl ContentQualityAnalyzer {
    pub fn new() -> Self {
        Self
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

        // Adjust for silent 'e'
        if chars.last() == Some(&'e') && count > 1 {
            count -= 1;
        }

        count.max(1)
    }

    /// Count sentences in text (by punctuation).
    fn count_sentences(text: &str) -> usize {
        if text.trim().is_empty() {
            return 0;
        }
        let count = text
            .chars()
            .filter(|&c| c == '.' || c == '!' || c == '?')
            .count();
        count.max(1)
    }

    /// Flesch-Kincaid readability score (0-100, higher = easier).
    fn flesch_kincaid(words: usize, sentences: usize, syllables: usize) -> f64 {
        if words == 0 || sentences == 0 {
            return 0.0;
        }
        let score = 206.835
            - 1.015 * (words as f64 / sentences as f64)
            - 84.6 * (syllables as f64 / words as f64);
        score.clamp(0.0, 100.0)
    }
}

impl Default for ContentQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ContentQualityAnalyzer {
    fn name(&self) -> &str {
        "content-quality"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // 2.7 — Flesch-Kincaid readability
        let word_count = ctx.page.word_count;
        if word_count > 0 {
            let text = &ctx
                .page
                .headings
                .iter()
                .map(|h| h.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let sentences = Self::count_sentences(text);
            let syllables: usize = text.split_whitespace().map(Self::count_syllables).sum();
            let score = Self::flesch_kincaid(word_count, sentences.max(1), syllables);

            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "CQ001".to_string(),
                title: "Flesch-Kincaid readability score".to_string(),
                description: format!(
                    "Readability score: {score:.1}/100 (words: {word_count}, sentences: \
                     {sentences}, syllables: {syllables})."
                ),
                url: url.clone(),
                recommendation: if score < 30.0 {
                    "Content is very difficult to read. Consider simplifying language and \
                     shortening sentences."
                        .to_string()
                } else if score < 50.0 {
                    "Content is fairly difficult to read. Aim for a score of 60+ for general \
                     audiences."
                        .to_string()
                } else if score < 70.0 {
                    "Content has moderate readability. Suitable for most audiences.".to_string()
                } else {
                    "Content is easy to read. Good for general audiences.".to_string()
                },
            });
        }

        // Keyword density (top 10 terms from headings as proxy)
        let headings_text: String = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if !headings_text.trim().is_empty() {
            let mut freq: HashMap<String, usize> = HashMap::new();
            for word in headings_text.split_whitespace() {
                let lower = word
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>();
                if lower.len() > 2 && !STOP_WORDS.contains(&lower.as_str()) {
                    *freq.entry(lower).or_default() += 1;
                }
            }

            let mut terms: Vec<(String, usize)> = freq.into_iter().collect();
            terms.sort_by(|a, b| b.1.cmp(&a.1));
            terms.truncate(10);

            if !terms.is_empty() {
                let display: String = terms
                    .iter()
                    .map(|(word, count)| format!("\"{}\" ({})", word, count))
                    .collect::<Vec<_>>()
                    .join(", ");
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Content,
                    code: "CQ002".to_string(),
                    title: "Top keywords".to_string(),
                    description: format!("Top 10 keyword occurrences in headings: {display}."),
                    url: url.clone(),
                    recommendation: "Ensure target keywords appear in headings and body content \
                                     naturally."
                        .to_string(),
                });
            }
        }

        // Content-to-markup ratio (word count as proxy for content volume)
        if word_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CQ003".to_string(),
                title: "No content detected".to_string(),
                description: "The page has zero word count, which may indicate missing or \
                             hidden content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Ensure the page has meaningful visible text content.".to_string(),
            });
        } else if word_count < 300 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CQ004".to_string(),
                title: "Thin content".to_string(),
                description: format!(
                    "Page has only {word_count} words. Pages with fewer than 300 words may be \
                     considered thin content."
                ),
                url: url.clone(),
                recommendation: "Expand the content to at least 300 words for better search \
                                 visibility."
                    .to_string(),
            });
        } else if word_count > 3000 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "CQ005".to_string(),
                title: "Long-form content".to_string(),
                description: format!(
                    "Page has {word_count} words. Consider whether all content is necessary or \
                     if it could be split into multiple pages."
                ),
                url: url.clone(),
                recommendation: "Long-form content is good for SEO but ensure it remains \
                                 scannable with proper headings."
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

// ---------------------------------------------------------------------------
// 14. Security Header Analyzer
// ---------------------------------------------------------------------------

pub struct SecurityHeaderAnalyzer;

impl SecurityHeaderAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Look up a header value by name (case-insensitive).
    fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Validate a CSP directive string (basic syntax check).
    fn is_valid_csp(value: &str) -> bool {
        if value.trim().is_empty() {
            return false;
        }
        // CSP must contain at least one directive (e.g. "default-src 'self'")
        let directives = [
            "default-src",
            "script-src",
            "style-src",
            "img-src",
            "font-src",
            "connect-src",
            "frame-src",
            "object-src",
            "media-src",
            "child-src",
            "worker-src",
            "manifest-src",
            "form-action",
            "frame-ancestors",
            "base-uri",
            "upgrade-insecure-requests",
            "block-all-mixed-content",
        ];
        value.split(';').any(|part| {
            let trimmed = part.trim();
            directives.iter().any(|d| trimmed.starts_with(d))
        })
    }

    /// Validate HSTS value (max-age, includeSubDomains, preload).
    fn validate_hsts(value: &str) -> Vec<String> {
        let mut issues = Vec::new();
        let lower = value.to_lowercase();

        // Must contain max-age
        if !lower.contains("max-age=") {
            issues.push("missing max-age directive".to_string());
        } else if let Some(ma_pos) = lower.find("max-age=") {
            let after = &value[ma_pos + 8..];
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            match num_str.parse::<u64>() {
                Ok(age) if age < 31536000 => {
                    issues.push(format!(
                        "max-age ({age}) is below recommended minimum of 31536000 (1 year)"
                    ));
                }
                Ok(_) => {} // acceptable
                Err(_) => {
                    issues.push("max-age is not a valid integer".to_string());
                }
            }
        }

        issues
    }

    /// Compute a security posture score (0-100) from the findings.
    fn compute_score(findings: &[Finding]) -> u32 {
        let mut score: i32 = 100;
        for f in findings {
            if f.code == "SEC012" {
                continue; // Don't count the score finding itself
            }
            match f.severity {
                Severity::Critical => score -= 20,
                Severity::Error => score -= 10,
                Severity::Warning => score -= 5,
                Severity::Info => {}
            }
        }
        score.max(0) as u32
    }
}

impl Default for SecurityHeaderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SecurityHeaderAnalyzer {
    fn name(&self) -> &str {
        "security-headers"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let headers = ctx.headers;

        // --- Content-Security-Policy ---
        match Self::get_header(headers, "Content-Security-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "SEC001".to_string(),
                    title: "Missing Content-Security-Policy header".to_string(),
                    description: "No Content-Security-Policy header was found. CSP helps prevent \
                                  XSS, clickjacking, and other code injection attacks."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Implement a Content-Security-Policy header. Start with \
                                     \"default-src 'self'\" and refine as needed."
                        .to_string(),
                });
            }
            Some(csp) => {
                if !Self::is_valid_csp(csp) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "SEC013".to_string(),
                        title: "Invalid Content-Security-Policy syntax".to_string(),
                        description: "The CSP header value does not appear to contain valid \
                                      directive syntax."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Ensure CSP contains at least one valid directive \
                                         (e.g. default-src, script-src)."
                            .to_string(),
                    });
                }
            }
        }

        // --- Strict-Transport-Security (HSTS) ---
        match Self::get_header(headers, "Strict-Transport-Security") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "SEC002".to_string(),
                    title: "Missing Strict-Transport-Security header".to_string(),
                    description: "No Strict-Transport-Security (HSTS) header was found. HSTS \
                                  forces browsers to use HTTPS."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"Strict-Transport-Security: max-age=31536000; \
                                     includeSubDomains; preload\"."
                        .to_string(),
                });
            }
            Some(hsts) => {
                let hsts_issues = Self::validate_hsts(hsts);
                for issue in &hsts_issues {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "SEC014".to_string(),
                        title: "HSTS configuration issue".to_string(),
                        description: format!("HSTS header: {issue}."),
                        url: url.clone(),
                        recommendation: "Set max-age to at least 31536000 (1 year). Add \
                                         includeSubDomains and preload."
                            .to_string(),
                    });
                }
                if !hsts.to_lowercase().contains("includesubdomains") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "SEC015".to_string(),
                        title: "HSTS missing includeSubDomains".to_string(),
                        description: "The HSTS header does not include the includeSubDomains \
                                      directive."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add includeSubDomains to protect all subdomains."
                            .to_string(),
                    });
                }
                if !hsts.to_lowercase().contains("preload") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "SEC016".to_string(),
                        title: "HSTS missing preload".to_string(),
                        description: "The HSTS header does not include the preload directive."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Consider adding preload for browser HSTS preload \
                                         list inclusion."
                            .to_string(),
                    });
                }
            }
        }

        // --- X-Frame-Options ---
        match Self::get_header(headers, "X-Frame-Options") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "SEC003".to_string(),
                    title: "Missing X-Frame-Options header".to_string(),
                    description: "No X-Frame-Options header was found. This header prevents \
                                  clickjacking by controlling frame embedding."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Set X-Frame-Options to DENY or SAMEORIGIN.".to_string(),
                });
            }
            Some(value) => {
                let upper = value.to_uppercase().trim().to_string();
                if upper != "DENY" && upper != "SAMEORIGIN" {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "SEC004".to_string(),
                        title: "Invalid X-Frame-Options value".to_string(),
                        description: format!(
                            "X-Frame-Options is \"{value}\" but must be DENY or SAMEORIGIN."
                        ),
                        url: url.clone(),
                        recommendation: "Set X-Frame-Options to DENY (preferred) or SAMEORIGIN."
                            .to_string(),
                    });
                }
            }
        }

        // --- X-Content-Type-Options ---
        match Self::get_header(headers, "X-Content-Type-Options") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "SEC005".to_string(),
                    title: "Missing X-Content-Type-Options header".to_string(),
                    description: "No X-Content-Type-Options header was found. This header \
                                  prevents MIME-type sniffing."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Set X-Content-Type-Options to nosniff.".to_string(),
                });
            }
            Some(value) => {
                if value.trim().to_lowercase() != "nosniff" {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "SEC006".to_string(),
                        title: "Invalid X-Content-Type-Options value".to_string(),
                        description: format!(
                            "X-Content-Type-Options is \"{value}\" but must be nosniff."
                        ),
                        url: url.clone(),
                        recommendation: "Set X-Content-Type-Options to nosniff.".to_string(),
                    });
                }
            }
        }

        // --- Referrer-Policy ---
        const RECOMMENDED_REFERRER: &[&str] = &[
            "no-referrer",
            "no-referrer-when-downgrade",
            "origin",
            "origin-when-cross-origin",
            "same-origin",
            "strict-origin",
            "strict-origin-when-cross-origin",
            "unsafe-url",
        ];
        match Self::get_header(headers, "Referrer-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "SEC007".to_string(),
                    title: "Missing Referrer-Policy header".to_string(),
                    description: "No Referrer-Policy header was found. This header controls how \
                                  much referrer information is sent with requests."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Set Referrer-Policy to strict-origin-when-cross-origin \
                                     or no-referrer for maximum privacy."
                        .to_string(),
                });
            }
            Some(value) => {
                if !RECOMMENDED_REFERRER.contains(&value.trim()) {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "SEC017".to_string(),
                        title: "Uncommon Referrer-Policy value".to_string(),
                        description: format!(
                            "Referrer-Policy \"{value}\" is not in the list of commonly used \
                             policies."
                        ),
                        url: url.clone(),
                        recommendation: "Consider using strict-origin-when-cross-origin or \
                                         no-referrer."
                            .to_string(),
                    });
                }
            }
        }

        // --- Permissions-Policy ---
        if Self::get_header(headers, "Permissions-Policy").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "SEC008".to_string(),
                title: "Missing Permissions-Policy header".to_string(),
                description: "No Permissions-Policy header was found. This header controls \
                              which browser features APIs can be used."
                    .to_string(),
                url: url.clone(),
                recommendation: "Consider setting Permissions-Policy to disable unused features \
                                 like camera, microphone, geolocation."
                    .to_string(),
            });
        } else if let Some(pp) = Self::get_header(headers, "Permissions-Policy") {
            // Check that dangerous features are restricted
            let dangerous = ["camera", "microphone", "geolocation"];
            for feature in &dangerous {
                if pp.to_lowercase().contains(feature)
                    && pp.to_lowercase().contains(&format!("{feature}=()"))
                {
                    // Feature is restricted (empty allowlist) — good
                } else if pp.to_lowercase().contains(feature) {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "SEC018".to_string(),
                        title: format!("Permissions-Policy: {feature} not restricted"),
                        description: format!(
                            "The {feature} feature in Permissions-Policy is not explicitly \
                             restricted."
                        ),
                        url: url.clone(),
                        recommendation: format!(
                            "Add {feature}=() to Permissions-Policy to disable it if not \
                             needed."
                        ),
                    });
                }
            }
        }

        // --- Cross-Origin-Embedder-Policy (COEP) ---
        if Self::get_header(headers, "Cross-Origin-Embedder-Policy").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "SEC009".to_string(),
                title: "Missing Cross-Origin-Embedder-Policy header".to_string(),
                description: "No COEP header was found. COEP prevents resources from loading \
                              cross-origin without explicit permission."
                    .to_string(),
                url: url.clone(),
                recommendation: "Set COEP to require-corp for stricter cross-origin isolation."
                    .to_string(),
            });
        }

        // --- Cross-Origin-Opener-Policy (COOP) ---
        if Self::get_header(headers, "Cross-Origin-Opener-Policy").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "SEC010".to_string(),
                title: "Missing Cross-Origin-Opener-Policy header".to_string(),
                description: "No COOP header was found. COOP isolates your browsing context \
                              from cross-origin popups."
                    .to_string(),
                url: url.clone(),
                recommendation: "Set COOP to same-origin for stricter isolation.".to_string(),
            });
        }

        // --- Cross-Origin-Resource-Policy (CORP) ---
        if Self::get_header(headers, "Cross-Origin-Resource-Policy").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "SEC011".to_string(),
                title: "Missing Cross-Origin-Resource-Policy header".to_string(),
                description: "No CORP header was found. CORP prevents cross-origin reads of \
                              embedded resources."
                    .to_string(),
                url: url.clone(),
                recommendation: "Set CORP to same-origin if the resource should only be used \
                                 by the same origin."
                    .to_string(),
            });
        }

        // --- Security posture score ---
        let score = Self::compute_score(&findings);
        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Security,
            code: "SEC012".to_string(),
            title: "Security posture score".to_string(),
            description: format!("Security header score: {score}/100."),
            url: url.clone(),
            recommendation: if score < 50 {
                "Security posture is weak. Prioritize adding CSP, HSTS, and frame-protecting \
                 headers."
                    .to_string()
            } else if score < 80 {
                "Security posture is moderate. Address remaining missing headers.".to_string()
            } else {
                "Security posture is strong. Minor improvements possible.".to_string()
            },
        });

        findings
    }
}

// ---------------------------------------------------------------------------
// 15. SSL Certificate Validator
// ---------------------------------------------------------------------------

pub struct SslCertificateValidator {
    cert_info: Option<SslCertificateInfo>,
}

impl SslCertificateValidator {
    /// Create a validator with pre-fetched certificate information.
    pub fn new(cert_info: Option<SslCertificateInfo>) -> Self {
        Self { cert_info }
    }

    pub fn empty() -> Self {
        Self { cert_info: None }
    }

    /// Parse an ISO 8601 date string into seconds since epoch for comparison.
    fn parse_epoch(s: &str) -> Option<i64> {
        chrono::DateTime::parse_from_rfc3339(s)
            .or_else(|_| chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ"))
            .ok()
            .map(|dt| dt.timestamp())
    }

    /// Check if an algorithm is considered weak.
    fn is_weak_algorithm(algo: &str) -> bool {
        let lower = algo.to_lowercase();
        lower.contains("md5")
            || lower.contains("sha1")
            || lower.contains("with rsa encryption")
                && !lower.contains("sha256")
                && !lower.contains("sha384")
                && !lower.contains("sha512")
    }
}

impl Default for SslCertificateValidator {
    fn default() -> Self {
        Self::empty()
    }
}

impl Analyzer for SslCertificateValidator {
    fn name(&self) -> &str {
        "ssl-certificate"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let info = match &self.cert_info {
            Some(i) => i,
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "SSL007".to_string(),
                    title: "No SSL certificate data available".to_string(),
                    description: "No TLS certificate information was provided for validation."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Provide certificate data from the TLS connection to enable \
                                     SSL validation."
                        .to_string(),
                });
                return findings;
            }
        };

        // --- Expired certificate ---
        if let Some(ref not_after) = info.not_after {
            if let Some(expiry_epoch) = Self::parse_epoch(not_after) {
                let now = chrono::Utc::now().timestamp();
                if now > expiry_epoch {
                    findings.push(Finding {
                        severity: Severity::Critical,
                        category: IssueCategory::Security,
                        code: "SSL001".to_string(),
                        title: "SSL certificate has expired".to_string(),
                        description: format!(
                            "Certificate expired on {not_after} ({} days ago).",
                            (now - expiry_epoch) / 86400
                        ),
                        url: url.clone(),
                        recommendation: "Renew the SSL certificate immediately. Expired \
                                         certificates cause browser security warnings."
                            .to_string(),
                    });
                } else {
                    let days_left = (expiry_epoch - now) / 86400;
                    if days_left < 30 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Security,
                            code: "SSL002".to_string(),
                            title: "SSL certificate expiring soon".to_string(),
                            description: format!(
                                "Certificate expires on {not_after} ({days_left} days remaining)."
                            ),
                            url: url.clone(),
                            recommendation: "Renew the certificate before it expires. Set up \
                                             auto-renewal (e.g. Let's Encrypt certbot)."
                                .to_string(),
                        });
                    }
                }
            }
        }

        // --- Invalid certificate chain ---
        if !info.is_valid_chain {
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Security,
                code: "SSL003".to_string(),
                title: "Invalid certificate chain".to_string(),
                description: "The TLS certificate chain did not validate successfully. Browsers \
                              will show a security warning."
                    .to_string(),
                url: url.clone(),
                recommendation: "Ensure the full certificate chain (including intermediate \
                                 certificates) is properly installed."
                    .to_string(),
            });
        }

        // --- Self-signed certificate ---
        if info.is_self_signed {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Security,
                code: "SSL006".to_string(),
                title: "Self-signed certificate detected".to_string(),
                description: "The certificate is self-signed and will not be trusted by browsers."
                    .to_string(),
                url: url.clone(),
                recommendation: "Use a certificate signed by a trusted Certificate Authority. \
                                 Consider Let's Encrypt for free trusted certificates."
                    .to_string(),
            });
        }

        // --- Subject/SAN mismatch ---
        let page_host = Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(String::from));
        if let Some(ref host) = page_host {
            let mut matched = false;
            if let Some(ref subject) = info.subject {
                if subject.eq_ignore_ascii_case(host) {
                    matched = true;
                }
                // Check wildcard match
                if subject.starts_with("*.") {
                    let wildcard_domain = &subject[2..];
                    if let Some(stripped_host) = host.strip_prefix('*') {
                        if stripped_host == wildcard_domain {
                            matched = true;
                        }
                    }
                    // Also handle bare wildcard: *.example.com matches sub.example.com
                    let parts: Vec<&str> = host.split('.').collect();
                    if parts.len() > 1 {
                        let root = parts[1..].join(".");
                        if wildcard_domain == root {
                            matched = true;
                        }
                    }
                }
            }
            for san in &info.san_entries {
                if san.eq_ignore_ascii_case(host) {
                    matched = true;
                    break;
                }
                // Wildcard SAN
                if san.starts_with("*.") {
                    let wildcard_domain = &san[2..];
                    let parts: Vec<&str> = host.split('.').collect();
                    if parts.len() > 1 {
                        let root = parts[1..].join(".");
                        if wildcard_domain == root {
                            matched = true;
                            break;
                        }
                    }
                }
            }
            if !matched && !info.san_entries.is_empty() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Security,
                    code: "SSL004".to_string(),
                    title: "Subject/SAN does not match hostname".to_string(),
                    description: format!(
                        "Certificate subject {:?} and SANs {:?} do not match hostname \"{host}\".",
                        info.subject, info.san_entries,
                    ),
                    url: url.clone(),
                    recommendation: "Issue a certificate that includes the correct hostname in \
                                     the Subject CN or Subject Alternative Names."
                        .to_string(),
                });
            }
        }

        // --- Weak signature algorithm ---
        if let Some(ref algo) = info.signature_algorithm {
            if Self::is_weak_algorithm(algo) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "SSL005".to_string(),
                    title: "Weak signature algorithm".to_string(),
                    description: format!(
                        "Certificate uses signature algorithm \"{algo}\", which is considered \
                         weak."
                    ),
                    url: url.clone(),
                    recommendation: "Reissue the certificate with SHA-256 or stronger signature \
                                     algorithm."
                        .to_string(),
                });
            }
        }

        // --- Certificate info summary ---
        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Security,
            code: "SSL008".to_string(),
            title: "SSL certificate details".to_string(),
            description: format!(
                "Subject: {}, Issuer: {}, SANs: {}, Chain valid: {}, Self-signed: {}",
                info.subject.as_deref().unwrap_or("N/A"),
                info.issuer.as_deref().unwrap_or("N/A"),
                info.san_entries.len(),
                info.is_valid_chain,
                info.is_self_signed,
            ),
            url: url.clone(),
            recommendation: String::new(),
        });

        findings
    }
}

// ---------------------------------------------------------------------------
// 16. Mobile-Friendliness Checker
// ---------------------------------------------------------------------------

pub struct MobileFriendlinessChecker;

impl MobileFriendlinessChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse viewport meta content into a map of directives.
    fn parse_viewport(viewport: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for part in viewport.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                map.insert(key.trim().to_lowercase(), value.trim().to_lowercase());
            }
        }
        map
    }
}

impl Default for MobileFriendlinessChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MobileFriendlinessChecker {
    fn name(&self) -> &str {
        "mobile-friendliness"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // --- Viewport meta tag presence ---
        let viewport = match &ctx.page.meta.viewport {
            Some(v) => v,
            None => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Mobile,
                    code: "MOB001".to_string(),
                    title: "Missing viewport meta tag".to_string(),
                    description: "No viewport meta tag was found. Without it, the page will not \
                                  scale properly on mobile devices."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add <meta name=\"viewport\" content=\"width=device-width, \
                                     initial-scale=1\"> to the <head>."
                        .to_string(),
                });
                return findings;
            }
        };

        let directives = Self::parse_viewport(viewport);

        // --- width=device-width ---
        match directives.get("width") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Mobile,
                    code: "MOB002".to_string(),
                    title: "Viewport missing width directive".to_string(),
                    description: "The viewport meta tag does not specify width=device-width."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Set width=device-width in the viewport meta tag.".to_string(),
                });
            }
            Some(w) if w != "device-width" => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Mobile,
                    code: "MOB003".to_string(),
                    title: "Viewport width is not device-width".to_string(),
                    description: format!(
                        "Viewport width is set to \"{w}\" instead of device-width. This forces \
                         a fixed layout."
                    ),
                    url: url.clone(),
                    recommendation: "Change viewport width to device-width for responsive layout."
                        .to_string(),
                });
            }
            _ => {} // device-width — correct
        }

        // --- user-scalable=no or maximum-scale=1.0 ---
        if directives.get("user-scalable") == Some(&"no".to_string()) {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Mobile,
                code: "MOB004".to_string(),
                title: "Zooming is disabled (user-scalable=no)".to_string(),
                description: "The viewport meta tag disables user zooming with \
                              user-scalable=no. This is a critical accessibility issue."
                    .to_string(),
                url: url.clone(),
                recommendation: "Remove user-scalable=no to allow pinch-to-zoom. This is \
                                 required for WCAG 2.1 compliance."
                    .to_string(),
            });
        }

        if let Some(max_scale) = directives.get("maximum-scale") {
            if max_scale == "1" || max_scale == "1.0" {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Mobile,
                    code: "MOB005".to_string(),
                    title: "Maximum scale restricted".to_string(),
                    description: format!(
                        "maximum-scale={max_scale} limits zooming. While not as severe as \
                         user-scalable=no, it can still hinder accessibility."
                    ),
                    url: url.clone(),
                    recommendation: "Remove the maximum-scale constraint or set it to at least \
                                     5.0."
                        .to_string(),
                });
            }
        }

        // --- initial-scale ---
        if let Some(scale) = directives.get("initial-scale") {
            if scale != "1" && scale != "1.0" {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Mobile,
                    code: "MOB009".to_string(),
                    title: "Non-standard initial scale".to_string(),
                    description: format!(
                        "initial-scale is set to \"{scale}\" instead of the standard 1.0. This \
                         may cause unexpected zoom behavior on page load."
                    ),
                    url: url.clone(),
                    recommendation: "Set initial-scale=1 for a consistent mobile experience."
                        .to_string(),
                });
            }
        }

        // --- Touch target size heuristic ---
        // We can't compute actual CSS sizes, but we can flag if there's a strong
        // indication of tiny touch targets based on known anti-patterns.
        // This is a heuristic since we don't have computed styles.
        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Mobile,
            code: "MOB006".to_string(),
            title: "Touch target sizes".to_string(),
            description: "Touch target size analysis requires CSS layout computation. Review \
                          interactive elements to ensure they are at least 44x44 CSS pixels \
                          (WCAG 2.5.8)."
                .to_string(),
            url: url.clone(),
            recommendation: "Ensure all clickable/tappable elements have a minimum touch target \
                             size of 44x44 CSS pixels."
                .to_string(),
        });

        // --- Font size heuristic ---
        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Mobile,
            code: "MOB007".to_string(),
            title: "Font size analysis".to_string(),
            description: "Base font size analysis requires CSS parsing. Ensure body text is at \
                          least 16px for comfortable mobile reading."
                .to_string(),
            url: url.clone(),
            recommendation: "Set html { font-size: 16px } or larger. Avoid viewport-relative \
                             units (vw) for body text without fallbacks."
                .to_string(),
        });

        // --- Horizontal scrolling heuristic ---
        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Mobile,
            code: "MOB008".to_string(),
            title: "Horizontal scrolling check".to_string(),
            description: "Horizontal scrolling detection requires runtime layout testing. With \
                          width=device-width set, this is typically avoided."
                .to_string(),
            url: url.clone(),
            recommendation: "Test on multiple viewport widths. Avoid fixed-width containers \
                             wider than 100vw."
                .to_string(),
        });

        findings
    }
}

// ---------------------------------------------------------------------------
// 17. Accessibility Analyzer (WCAG 2.1 AA)
// ---------------------------------------------------------------------------

pub struct AccessibilityAnalyzer;

impl AccessibilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Generic link text patterns that are not descriptive.
    const VAGUE_LINK_TEXTS: &[&str] = &[
        "click here",
        "here",
        "read more",
        "more",
        "learn more",
        "click",
        "link",
        "this",
        "go",
        "continue",
    ];
}

impl Default for AccessibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AccessibilityAnalyzer {
    fn name(&self) -> &str {
        "accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // --- WCAG 1.1.1: Image alt text ---
        let images_without_alt: Vec<&str> = ctx
            .page
            .images
            .iter()
            .filter(|img| !img.has_alt || img.alt.trim().is_empty())
            .map(|img| img.src.as_str())
            .collect();
        if !images_without_alt.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "A11Y001".to_string(),
                title: "Images missing alt text".to_string(),
                description: format!(
                    "{} image(s) missing alt attribute or have empty alt text: {}.",
                    images_without_alt.len(),
                    images_without_alt.join(", ")
                ),
                url: url.clone(),
                recommendation: "Add descriptive alt text to all images. Use alt=\"\" for \
                                 decorative images."
                    .to_string(),
            });
        }

        // --- WCAG 1.3.1: Heading hierarchy ---
        if ctx.page.headings.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y002".to_string(),
                title: "No headings found".to_string(),
                description: "The page has no heading elements. Headings provide structure \
                              for screen reader users."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add heading elements (H1-H6) to provide page structure."
                    .to_string(),
            });
        } else {
            let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
            if h1_count == 0 {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Accessibility,
                    code: "A11Y003".to_string(),
                    title: "Missing H1 heading".to_string(),
                    description: "No H1 heading found. Screen readers use H1 to identify \
                                  the main page topic."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add exactly one H1 heading per page.".to_string(),
                });
            } else if h1_count > 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "A11Y004".to_string(),
                    title: "Multiple H1 headings".to_string(),
                    description: format!(
                        "Page has {h1_count} H1 headings. Use a single H1 for the main topic."
                    ),
                    url: url.clone(),
                    recommendation: "Use one H1 for the page title and H2+ for sections."
                        .to_string(),
                });
            }

            // Skipped heading levels
            let mut prev_level: Option<u8> = None;
            for heading in &ctx.page.headings {
                if let Some(prev) = prev_level {
                    if heading.level > prev + 1 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Accessibility,
                            code: "A11Y005".to_string(),
                            title: "Skipped heading level".to_string(),
                            description: format!(
                                "Heading jumps from H{prev} to H{}, skipping intermediate \
                                 levels.",
                                heading.level
                            ),
                            url: url.clone(),
                            recommendation: format!(
                                "Use H{} after H{prev} to maintain document outline.",
                                prev + 1
                            ),
                        });
                        break; // Report first skip only
                    }
                }
                prev_level = Some(heading.level);
            }
        }

        // --- WCAG 1.3.1: Landmark roles ---
        if !ctx.page.has_main_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y006".to_string(),
                title: "Missing main landmark".to_string(),
                description: "No <main> element or role=\"main\" found. Screen readers use \
                              landmarks for page navigation."
                    .to_string(),
                url: url.clone(),
                recommendation: "Wrap primary content in a <main> element.".to_string(),
            });
        }

        if !ctx.page.has_nav_landmark {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "A11Y007".to_string(),
                title: "No navigation landmark".to_string(),
                description: "No <nav> element or role=\"navigation\" found.".to_string(),
                url: url.clone(),
                recommendation: "Wrap navigation links in a <nav> element.".to_string(),
            });
        }

        // --- WCAG 2.4.1: Skip navigation ---
        if !ctx.page.has_skip_link && ctx.page.has_nav_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y008".to_string(),
                title: "Missing skip navigation link".to_string(),
                description: "No skip-to-content link found. Keyboard users must tab through \
                              all navigation links to reach main content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add a skip link as the first focusable element: \
                                 <a href=\"#main\" class=\"skip-link\">Skip to content</a>."
                    .to_string(),
            });
        }

        // --- WCAG 2.4.4: Link text quality ---
        for link in &ctx.page.links {
            let text_lower = link.text.trim().to_lowercase();
            if text_lower.is_empty() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Accessibility,
                    code: "A11Y009".to_string(),
                    title: "Empty link text".to_string(),
                    description: format!(
                        "Link to \"{}\" has no text. Screen readers announce the URL, \
                         which is not descriptive.",
                        link.href
                    ),
                    url: url.clone(),
                    recommendation: "Add descriptive text or an aria-label to the link."
                        .to_string(),
                });
            } else if Self::VAGUE_LINK_TEXTS.contains(&text_lower.as_str()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "A11Y010".to_string(),
                    title: "Non-descriptive link text".to_string(),
                    description: format!(
                        "Link text \"{}\" is vague and does not describe the destination.",
                        link.text
                    ),
                    url: url.clone(),
                    recommendation: "Use descriptive text that explains the link purpose \
                                     (e.g., \"View pricing details\" instead of \"click here\")."
                        .to_string(),
                });
            }
        }

        // --- WCAG 1.3.1: Form label association ---
        for form in &ctx.page.forms {
            for input in &form.inputs {
                if !input.has_label {
                    let desc = match (&input.name, &input.input_type) {
                        (Some(n), Some(t)) => format!("input (name=\"{n}\", type=\"{t}\")"),
                        (Some(n), None) => format!("input (name=\"{n}\")"),
                        (None, Some(t)) => format!("input (type=\"{t}\")"),
                        (None, None) => "input".to_string(),
                    };
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Accessibility,
                        code: "A11Y011".to_string(),
                        title: "Form input missing label".to_string(),
                        description: format!(
                            "{desc} has no associated <label>, aria-label, or aria-labelledby."
                        ),
                        url: url.clone(),
                        recommendation: "Add a <label for=\"id\"> element or an aria-label \
                                         attribute to the input."
                            .to_string(),
                    });
                }
            }
        }

        // --- WCAG 2.1.1: Keyboard navigation (tabindex) ---
        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "A11Y012".to_string(),
                title: "Positive tabindex values detected".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, \
                              making keyboard navigation unpredictable."
                    .to_string(),
                url: url.clone(),
                recommendation: "Use tabindex=\"0\" to add elements to the natural tab order \
                                 or tabindex=\"-1\" for programmatic focus only."
                    .to_string(),
            });
        }

        // --- WCAG 4.1.2: ARIA usage ---
        if ctx.page.aria_role_count == 0 && !ctx.page.landmarks.is_empty() {
            // No ARIA roles used but landmarks exist via HTML — that's fine
        } else if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y013".to_string(),
                title: "ARIA roles without labels".to_string(),
                description: format!(
                    "{} ARIA role(s) found but no aria-label or aria-labelledby attributes. \
                     Custom roles require accessible names.",
                    ctx.page.aria_role_count
                ),
                url: url.clone(),
                recommendation: "Add aria-label or aria-labelledby to elements with custom \
                                 ARIA roles."
                    .to_string(),
            });
        }

        // --- WCAG 1.3.1: Table accessibility ---
        if ctx.page.tables_total > 0 {
            let tables_without_headers = ctx.page.tables_total - ctx.page.tables_with_headers;
            if tables_without_headers > 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "A11Y014".to_string(),
                    title: "Table missing header cells".to_string(),
                    description: format!(
                        "{} of {} table(s) have no <th> header cells.",
                        tables_without_headers, ctx.page.tables_total
                    ),
                    url: url.clone(),
                    recommendation: "Use <th> elements for header cells and add scope \
                                     attributes for complex tables."
                        .to_string(),
                });
            }

            let tables_without_captions = ctx.page.tables_total - ctx.page.tables_with_captions;
            if tables_without_captions > 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "A11Y015".to_string(),
                    title: "Table missing caption".to_string(),
                    description: format!(
                        "{} of {} table(s) have no <caption> element.",
                        tables_without_captions, ctx.page.tables_total
                    ),
                    url: url.clone(),
                    recommendation: "Add a <caption> to describe the table purpose.".to_string(),
                });
            }
        }

        // --- WCAG 1.4.4: Language attribute ---
        if !ctx.page.has_lang_attribute {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "A11Y016".to_string(),
                title: "Missing html lang attribute".to_string(),
                description: "The <html> element has no lang attribute. Screen readers use \
                              this to select the correct pronunciation engine."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add lang=\"en\" (or the appropriate language code) to the \
                                 <html> element."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 18. Social Media Analyzer
// ---------------------------------------------------------------------------

pub struct SocialMediaAnalyzer;

impl SocialMediaAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Minimum recommended OG image width (pixels).
    const OG_MIN_WIDTH: u32 = 1200;
    /// Minimum recommended OG image height (pixels).
    const OG_MIN_HEIGHT: u32 = 630;
}

impl Default for SocialMediaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SocialMediaAnalyzer {
    fn name(&self) -> &str {
        "social-media"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // --- OG image dimensions ---
        if ctx.page.og_image_width.is_none() && ctx.page.og_image_height.is_none() {
            // No OG image dimensions at all — check if there's an OG image
            if ctx.page.meta.og.image.is_some() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Social,
                    code: "SOCIAL001".to_string(),
                    title: "OG image missing dimensions".to_string(),
                    description: "og:image is set but og:image:width and og:image:height \
                                  are missing. Social platforms may crop or scale the image \
                                  incorrectly."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add <meta property=\"og:image:width\" content=\"1200\"> \
                                     and <meta property=\"og:image:height\" content=\"630\">."
                        .to_string(),
                });
            }
        } else {
            // Check dimensions meet minimum requirements
            if let Some(width) = ctx.page.og_image_width {
                if width < Self::OG_MIN_WIDTH {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "SOCIAL002".to_string(),
                        title: "OG image too narrow".to_string(),
                        description: format!(
                            "og:image:width is {width}px, below the recommended minimum of \
                             {}px.",
                            Self::OG_MIN_WIDTH
                        ),
                        url: url.clone(),
                        recommendation: format!(
                            "Use an image at least {}x{} pixels for optimal social previews.",
                            Self::OG_MIN_WIDTH,
                            Self::OG_MIN_HEIGHT
                        ),
                    });
                }
            }
            if let Some(height) = ctx.page.og_image_height {
                if height < Self::OG_MIN_HEIGHT {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "SOCIAL003".to_string(),
                        title: "OG image too short".to_string(),
                        description: format!(
                            "og:image:height is {height}px, below the recommended minimum of \
                             {}px.",
                            Self::OG_MIN_HEIGHT
                        ),
                        url: url.clone(),
                        recommendation: format!(
                            "Use an image at least {}x{} pixels for optimal social previews.",
                            Self::OG_MIN_WIDTH,
                            Self::OG_MIN_HEIGHT
                        ),
                    });
                }
            }
        }

        // --- Twitter Card type ---
        match &ctx.page.meta.twitter.card {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Social,
                    code: "SOCIAL004".to_string(),
                    title: "Missing Twitter Card type".to_string(),
                    description: "No twitter:card meta tag found. Twitter/X will not render \
                                  a rich preview without it."
                        .to_string(),
                    url: url.clone(),
                    recommendation:
                        "Add <meta name=\"twitter:card\" content=\"summary_large_image\">."
                            .to_string(),
                });
            }
            Some(card_type) => {
                let valid_types = ["summary", "summary_large_image", "app", "player"];
                if !valid_types.contains(&card_type.as_str()) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "SOCIAL005".to_string(),
                        title: "Invalid Twitter Card type".to_string(),
                        description: format!(
                            "twitter:card value \"{card_type}\" is not a recognized card type."
                        ),
                        url: url.clone(),
                        recommendation: "Use one of: summary, summary_large_image, app, player."
                            .to_string(),
                    });
                }
            }
        }

        // --- Social preview completeness ---
        let og_required = [
            ("og:title", ctx.page.meta.og.title.is_some()),
            ("og:description", ctx.page.meta.og.description.is_some()),
            ("og:image", ctx.page.meta.og.image.is_some()),
            ("og:url", ctx.page.meta.og.url.is_some()),
            ("og:type", ctx.page.meta.og.r#type.is_some()),
        ];

        let missing_og: Vec<&str> = og_required
            .iter()
            .filter(|(_, present)| !present)
            .map(|(name, _)| *name)
            .collect();

        if !missing_og.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Social,
                code: "SOCIAL006".to_string(),
                title: "Incomplete Open Graph tags".to_string(),
                description: format!(
                    "Missing OG tags: {}. Social previews may be incomplete.",
                    missing_og.join(", ")
                ),
                url: url.clone(),
                recommendation: "Add all required OG tags for complete social media previews."
                    .to_string(),
            });
        }

        let twitter_required = [
            ("twitter:title", ctx.page.meta.twitter.title.is_some()),
            (
                "twitter:description",
                ctx.page.meta.twitter.description.is_some(),
            ),
            ("twitter:image", ctx.page.meta.twitter.image.is_some()),
        ];

        let missing_twitter: Vec<&str> = twitter_required
            .iter()
            .filter(|(_, present)| !present)
            .map(|(name, _)| *name)
            .collect();

        if !missing_twitter.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Social,
                code: "SOCIAL007".to_string(),
                title: "Incomplete Twitter Card tags".to_string(),
                description: format!(
                    "Missing Twitter tags: {}. Twitter/X previews may fall back to OG tags.",
                    missing_twitter.join(", ")
                ),
                url: url.clone(),
                recommendation: "Add Twitter-specific tags for optimal X/Twitter previews."
                    .to_string(),
            });
        }

        // --- Social preview summary ---
        let og_score = og_required.iter().filter(|(_, p)| *p).count();
        let twitter_score = twitter_required.iter().filter(|(_, p)| *p).count();
        let total = og_required.len() + twitter_required.len();
        let score = og_score + twitter_score;

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Social,
            code: "SOCIAL008".to_string(),
            title: "Social preview completeness score".to_string(),
            description: format!(
                "Social metadata score: {score}/{total} (OG: {og_score}/{og_len}, Twitter: \
                 {twitter_score}/{tw_len}).",
                og_len = og_required.len(),
                tw_len = twitter_required.len(),
            ),
            url: url.clone(),
            recommendation: if score < total {
                "Add missing social meta tags to improve how your page appears when shared."
                    .to_string()
            } else {
                "Social metadata is complete.".to_string()
            },
        });

        findings
    }
}

// ---------------------------------------------------------------------------
// 19. Entity Analyzer
// ---------------------------------------------------------------------------

const PERSON_INDICATORS: &[&str] = &[
    "mr.", "mrs.", "ms.", "dr.", "prof.", "sir", "lord", "president", "ceo", "cto",
    "founder", "author", "by", "written by", "edited by", "interview with",
];

const ORG_INDICATORS: &[&str] = &[
    "inc.", "llc", "ltd.", "corp.", "corporation", "company", "organization",
    "university", "institute", "foundation", "association", "group", "partners",
];

const LOCATION_INDICATORS: &[&str] = &[
    "city", "state", "country", "province", "district", "region", "street",
    "avenue", "boulevard", "road", "lane", "square", "park", "mountain",
    "river", "lake", "island", "bay", "coast", "valley",
];

pub struct EntityAnalyzer;

impl EntityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn detect_people(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let lower = text.to_lowercase();
        for indicator in PERSON_INDICATORS {
            if lower.contains(indicator) {
                let words: Vec<&str> = text.split_whitespace().collect();
                let indicator_word = indicator.trim_end_matches('.');
                for (i, word) in words.iter().enumerate() {
                    let clean: String = word
                        .chars()
                        .filter(|c| c.is_alphanumeric())
                        .collect();
                    if clean.to_lowercase() == indicator_word && i + 1 < words.len() {
                        let mut name_parts = Vec::new();
                        for w in words.iter().skip(i + 1) {
                            let w_clean: String =
                                w.chars().filter(|c| c.is_alphanumeric() || *c == '-').collect();
                            let w_lower = w_clean.to_lowercase();
                            if w_clean
                                .chars()
                                .next()
                                .map(|c| c.is_uppercase())
                                .unwrap_or(false)
                                || w_lower == "de"
                                || w_lower == "van"
                                || w_lower == "von"
                                || w_lower == "la"
                                || w_lower == "le"
                            {
                                name_parts.push(w_clean);
                            } else {
                                break;
                            }
                        }
                        let name = name_parts.join(" ");
                        if !name.is_empty() {
                            found.push(name);
                        }
                    }
                }
            }
        }
        found.sort();
        found.dedup();
        found
    }

    fn detect_organizations(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let lower = text.to_lowercase();
        for indicator in ORG_INDICATORS {
            if lower.contains(indicator) {
                for sentence in text.split(|c| c == '.' || c == '!' || c == '?') {
                    let words: Vec<&str> = sentence.split_whitespace().collect();
                    for (i, word) in words.iter().enumerate() {
                        if word.to_lowercase().contains(indicator) {
                            let start = i.saturating_sub(2);
                            let org: String = words[start..=i.min(words.len() - 1)]
                                .iter()
                                .map(|w| w.to_string())
                                .collect::<Vec<_>>()
                                .join(" ");
                            if org.len() > 3 {
                                found.push(org);
                            }
                        }
                    }
                }
            }
        }
        found.sort();
        found.dedup();
        found
    }

    fn detect_locations(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let lower = text.to_lowercase();
        for indicator in LOCATION_INDICATORS {
            if lower.contains(indicator) {
                for sentence in text.split(|c| c == '.' || c == '!' || c == '?') {
                    let words: Vec<&str> = sentence.split_whitespace().collect();
                    for (i, word) in words.iter().enumerate() {
                        if word.to_lowercase().contains(indicator) {
                            let start = i.saturating_sub(2);
                            let loc: String = words[start..=i.min(words.len() - 1)]
                                .iter()
                                .map(|w| w.to_string())
                                .collect::<Vec<_>>()
                                .join(" ");
                            if loc.len() > 3 {
                                found.push(loc);
                            }
                        }
                    }
                }
            }
        }
        found.sort();
        found.dedup();
        found
    }

    fn detect_topics(headings: &[crate::parser::Heading], word_count: usize) -> Vec<String> {
        if word_count == 0 {
            return Vec::new();
        }
        let mut freq: HashMap<String, usize> = HashMap::new();
        for heading in headings {
            for word in heading.text.split_whitespace() {
                let lower = word
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>();
                if lower.len() > 3 && !STOP_WORDS.contains(&lower.as_str()) {
                    *freq.entry(lower).or_default() += 1;
                }
            }
        }
        let mut terms: Vec<(String, usize)> = freq.into_iter().collect();
        terms.sort_by(|a, b| b.1.cmp(&a.1));
        terms.into_iter().take(5).map(|(w, _)| w).collect()
    }

    fn analyze_sentiment(text: &str) -> (f64, &'static str) {
        let positive_words = [
            "good", "great", "excellent", "amazing", "wonderful", "best", "love",
            "happy", "fantastic", "superb", "outstanding", "perfect", "beautiful",
            "brilliant", "awesome", "nice", "pleasant", "delightful", "impressive",
            "remarkable", "magnificent", "splendid", "fabulous", "terrific",
        ];
        let negative_words = [
            "bad", "terrible", "horrible", "awful", "worst", "hate", "sad",
            "ugly", "poor", "disappointing", "boring", "annoying", "frustrating",
            "difficult", "broken", "failed", "error", "wrong", "problem", "issue",
            "bug", "fail", "crash", "dead",
        ];
        let words: Vec<String> = text
            .split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect()
            })
            .collect();
        if words.is_empty() {
            return (0.0, "neutral");
        }
        let pos_count = words
            .iter()
            .filter(|w| positive_words.contains(&w.as_str()))
            .count();
        let neg_count = words
            .iter()
            .filter(|w| negative_words.contains(&w.as_str()))
            .count();
        let total = words.len() as f64;
        let score = ((pos_count as f64 - neg_count as f64) / total * 100.0).round() / 100.0;
        let label = if score > 0.05 {
            "positive"
        } else if score < -0.05 {
            "negative"
        } else {
            "neutral"
        };
        (score, label)
    }
}

impl Default for EntityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for EntityAnalyzer {
    fn name(&self) -> &str {
        "entity-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let headings_text: String = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let text = &headings_text;

        let people = Self::detect_people(text);
        let organizations = Self::detect_organizations(text);
        let locations = Self::detect_locations(text);
        let topics = Self::detect_topics(&ctx.page.headings, ctx.page.word_count);
        let (sentiment_score, sentiment_label) = Self::analyze_sentiment(text);

        if !people.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "ENTITY001".to_string(),
                title: "People entities detected".to_string(),
                description: format!(
                    "Found {} people entity(ies): {}.",
                    people.len(),
                    people.join(", ")
                ),
                url: url.clone(),
                recommendation: "People entities can boost E-E-A-T signals. Link to author \
                                 profiles when applicable."
                    .to_string(),
            });
        }

        if !organizations.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "ENTITY002".to_string(),
                title: "Organization entities detected".to_string(),
                description: format!(
                    "Found {} organization entity(ies): {}.",
                    organizations.len(),
                    organizations.join(", ")
                ),
                url: url.clone(),
                recommendation: "Organization entities help establish topical authority. \
                                 Consider adding Organization schema markup."
                    .to_string(),
            });
        }

        if !locations.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "ENTITY003".to_string(),
                title: "Location entities detected".to_string(),
                description: format!(
                    "Found {} location entity(ies): {}.",
                    locations.len(),
                    locations.join(", ")
                ),
                url: url.clone(),
                recommendation: "Location entities are important for local SEO. Ensure \
                                 NAP consistency across the site."
                    .to_string(),
            });
        }

        if !topics.is_empty() {
            let topic_display = topics.join(", ");
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "ENTITY004".to_string(),
                title: "Detected topics and themes".to_string(),
                description: format!("Primary topics: {topic_display}."),
                url: url.clone(),
                recommendation: "Ensure these topics align with the target keywords and \
                                 page intent."
                    .to_string(),
            });
        }

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "ENTITY005".to_string(),
            title: "Content sentiment analysis".to_string(),
            description: format!(
                "Sentiment score: {sentiment_score} ({sentiment_label})."
            ),
            url: url.clone(),
            recommendation: if sentiment_label == "negative" {
                "Content has a negative sentiment tone. Consider revising for a more \
                 neutral or positive tone."
                    .to_string()
            } else if sentiment_label == "positive" {
                "Positive sentiment detected. This can improve user engagement.".to_string()
            } else {
                "Neutral sentiment detected.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "ENTITY006".to_string(),
            title: "Entity counts per page".to_string(),
            description: format!(
                "People: {}, Organizations: {}, Locations: {}, Topics: {}.",
                people.len(),
                organizations.len(),
                locations.len(),
                topics.len()
            ),
            url: url.clone(),
            recommendation: String::new(),
        });

        findings
    }
}

// ---------------------------------------------------------------------------
// 20. Enhanced Readability Analyzer
// ---------------------------------------------------------------------------

pub struct EnhancedReadabilityAnalyzer;

impl EnhancedReadabilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn count_letters(text: &str) -> usize {
        text.chars().filter(|c| c.is_alphabetic()).count()
    }

    fn count_sentences(text: &str) -> usize {
        if text.trim().is_empty() {
            return 1;
        }
        text.chars()
            .filter(|&c| c == '.' || c == '!' || c == '?')
            .count()
            .max(1)
    }

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

    fn flesch_kincaid_grade(words: usize, sentences: usize, syllables: usize) -> f64 {
        if words == 0 || sentences == 0 {
            return 0.0;
        }
        0.39 * (words as f64 / sentences as f64)
            + 11.8 * (syllables as f64 / words as f64)
            - 15.59
    }

    fn coleman_liau_index(letters: usize, words: usize, sentences: usize) -> f64 {
        if words == 0 {
            return 0.0;
        }
        let l = letters as f64 / words as f64 * 100.0;
        let s = sentences as f64 / words as f64 * 100.0;
        0.0588 * l - 0.296 * s - 15.8
    }

    fn automated_readability_index(
        characters: usize,
        words: usize,
        sentences: usize,
    ) -> f64 {
        if words == 0 || sentences == 0 {
            return 0.0;
        }
        4.71 * (characters as f64 / words as f64)
            + 0.5 * (words as f64 / sentences as f64)
            - 21.43
    }

    fn gunning_fog_index(words: usize, sentences: usize, complex_words: usize) -> f64 {
        if words == 0 || sentences == 0 {
            return 0.0;
        }
        0.4 * (words as f64 / sentences as f64 + 100.0 * complex_words as f64 / words as f64)
    }

    fn flesch_reading_ease(words: usize, sentences: usize, syllables: usize) -> f64 {
        if words == 0 || sentences == 0 {
            return 0.0;
        }
        let score = 206.835
            - 1.015 * (words as f64 / sentences as f64)
            - 84.6 * (syllables as f64 / words as f64);
        score.clamp(0.0, 100.0)
    }

    fn reading_ease_label(score: f64) -> &'static str {
        if score >= 90.0 {
            "very easy"
        } else if score >= 80.0 {
            "easy"
        } else if score >= 70.0 {
            "fairly easy"
        } else if score >= 60.0 {
            "standard"
        } else if score >= 50.0 {
            "fairly difficult"
        } else if score >= 30.0 {
            "difficult"
        } else {
            "very difficult"
        }
    }

    fn grade_label(grade: f64) -> &'static str {
        if grade < 1.0 {
            "kindergarten"
        } else if grade < 6.0 {
            "elementary school"
        } else if grade < 9.0 {
            "middle school"
        } else if grade < 13.0 {
            "high school"
        } else if grade < 16.0 {
            "college"
        } else {
            "postgraduate"
        }
    }
}

impl Default for EnhancedReadabilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for EnhancedReadabilityAnalyzer {
    fn name(&self) -> &str {
        "enhanced-readability"
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

        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count = words.len();
        let sentence_count = Self::count_sentences(&text);
        let letter_count = Self::count_letters(&text);
        let syllable_count: usize = words.iter().map(|w| Self::count_syllables(w)).sum();
        let complex_words = Self::complex_words_count(&words);

        let fk_grade = Self::flesch_kincaid_grade(word_count, sentence_count, syllable_count);
        let cl_index = Self::coleman_liau_index(letter_count, word_count, sentence_count);
        let ari = Self::automated_readability_index(letter_count, word_count, sentence_count);
        let fog = Self::gunning_fog_index(word_count, sentence_count, complex_words);
        let fre = Self::flesch_reading_ease(word_count, sentence_count, syllable_count);

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ001".to_string(),
            title: "Flesch-Kincaid Grade Level".to_string(),
            description: format!(
                "Grade level: {fk_grade:.1} ({})",
                Self::grade_label(fk_grade)
            ),
            url: url.clone(),
            recommendation: if fk_grade > 12.0 {
                "Content is at a college reading level. Consider simplifying for broader \
                 audiences."
                    .to_string()
            } else if fk_grade > 8.0 {
                "Content is at a high school reading level. Suitable for most web audiences."
                    .to_string()
            } else {
                "Content is easy to read for most audiences.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ002".to_string(),
            title: "Coleman-Liau Index".to_string(),
            description: format!("Index: {cl_index:.1}"),
            url: url.clone(),
            recommendation: if cl_index > 12.0 {
                "High Coleman-Liau index. Consider reducing sentence complexity."
                    .to_string()
            } else {
                "Readability is within acceptable range.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ003".to_string(),
            title: "Automated Readability Index".to_string(),
            description: format!("ARI: {ari:.1}"),
            url: url.clone(),
            recommendation: if ari > 12.0 {
                "High ARI score. Content may be difficult for general audiences."
                    .to_string()
            } else {
                "Readability is within acceptable range.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ004".to_string(),
            title: "Gunning Fog Index".to_string(),
            description: format!("Fog index: {fog:.1}"),
            url: url.clone(),
            recommendation: if fog > 17.0 {
                "Very high Fog index. Content is extremely complex. Simplify vocabulary \
                 and shorten sentences."
                    .to_string()
            } else if fog > 12.0 {
                "High Fog index. Consider simplifying for a broader audience.".to_string()
            } else {
                "Fog index is within acceptable range.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ005".to_string(),
            title: "Flesch Reading Ease score".to_string(),
            description: format!(
                "Score: {fre:.1}/100 ({})",
                Self::reading_ease_label(fre)
            ),
            url: url.clone(),
            recommendation: if fre < 30.0 {
                "Very difficult to read. Aim for a score of 60+ for general audiences."
                    .to_string()
            } else if fre < 50.0 {
                "Fairly difficult. Consider simplifying language.".to_string()
            } else if fre < 70.0 {
                "Standard readability. Suitable for most web content.".to_string()
            } else {
                "Easy to read. Good for broad audiences.".to_string()
            },
        });

        findings
    }
}

impl EnhancedReadabilityAnalyzer {
    fn complex_words_count(words: &[&str]) -> usize {
        words
            .iter()
            .filter(|w| Self::count_syllables(w) >= 3)
            .count()
    }
}

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

    fn tokenize(text: &str) -> Vec<String> {
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

    fn compute_tf(tokens: &[String]) -> HashMap<String, f64> {
        let mut freq: HashMap<String, usize> = HashMap::new();
        for token in tokens {
            *freq.entry(token.clone()).or_default() += 1;
        }
        let total = tokens.len() as f64;
        freq.into_iter()
            .map(|(term, count)| (term, count as f64 / total))
            .collect()
    }

    fn compute_tfidf(tf: &HashMap<String, f64>, corpus_tf: &HashMap<String, f64>) -> HashMap<String, f64> {
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

    fn keyword_density(tokens: &[String], total_words: usize) -> HashMap<String, f64> {
        if total_words == 0 {
            return HashMap::new();
        }
        let mut freq: HashMap<String, usize> = HashMap::new();
        for token in tokens {
            *freq.entry(token.clone()).or_default() += 1;
        }
        freq.into_iter()
            .map(|(term, count)| {
                (term, count as f64 / total_words as f64 * 100.0)
            })
            .collect()
    }

    fn detect_prominent_keywords(density: &HashMap<String, f64>) -> Vec<(String, f64)> {
        let mut prominent: Vec<(String, f64)> = density
            .iter()
            .filter(|(_, &d)| d >= 1.5)
            .map(|(k, &v)| (k.clone(), v))
            .collect();
        prominent.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        prominent
    }

    fn cooccurrence(tokens: &[String], window: usize) -> Vec<((String, String), usize)> {
        let mut pairs: HashMap<(String, String), usize> = HashMap::new();
        for i in 0..tokens.len() {
            let end = (i + window + 1).min(tokens.len());
            for j in (i + 1)..end {
                let mut pair = [tokens[i].clone(), tokens[j].clone()];
                pair.sort();
                *pairs.entry((pair[0].clone(), pair[1].clone()))
                    .or_default() += 1;
            }
        }
        let mut result: Vec<((String, String), usize)> = pairs.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
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
                description: format!(
                    "Keywords with density >= 1.5%: {}.",
                    display.join(", ")
                ),
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
// 22. E-commerce Signals Analyzer
// ---------------------------------------------------------------------------

pub struct EcommerceSignalsAnalyzer;

impl EcommerceSignalsAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn detect_product_schema(sd: &crate::parser::StructuredData) -> bool {
        sd.r#type
            .as_deref()
            .map(|t| t == "Product" || t == "IndividualProduct" || t == "AggregateOffer")
            .unwrap_or(false)
    }

    fn extract_price(data: &serde_json::Value) -> Option<String> {
        let direct = data
            .get("price")
            .or_else(|| data.get("lowPrice"))
            .or_else(|| data.get("highPrice"));
        if let Some(v) = direct {
            return Self::value_to_price(v);
        }
        let offers = data.get("offers")?;
        let offer = if offers.is_array() {
            offers.get(0)?
        } else {
            offers
        };
        let price_val = offer
            .get("price")
            .or_else(|| offer.get("lowPrice"))
            .or_else(|| offer.get("highPrice"))?;
        Self::value_to_price(price_val)
    }

    fn value_to_price(v: &serde_json::Value) -> Option<String> {
        if let Some(s) = v.as_str() {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
        v.as_f64().map(|p| format!("{p}"))
    }

    fn extract_availability(data: &serde_json::Value) -> Option<String> {
        data.get("availability")
            .and_then(|v| v.as_str().map(String::from))
            .or_else(|| {
                data.get("offers")
                    .and_then(|o| o.get("availability"))
                    .and_then(|v| v.as_str().map(String::from))
            })
    }

    fn detect_reviews(sd: &crate::parser::StructuredData) -> Vec<String> {
        let mut reviews = Vec::new();
        if let Some(obj) = sd.data.as_object() {
            if let Some(rating) = obj.get("aggregateRating") {
                if let Some(score) = rating.get("ratingValue").and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                }) {
                    reviews.push(format!("rating: {score}"));
                }
            }
            if let Some(r) = obj.get("reviewCount").or_else(|| obj.get("ratingCount")) {
                if let Some(count) = r
                    .as_f64()
                    .or_else(|| r.as_str().and_then(|s| s.parse::<f64>().ok()))
                {
                    reviews.push(format!("reviews: {count}"));
                }
            }
        }
        reviews
    }

    fn detect_offers(sd: &crate::parser::StructuredData) -> bool {
        sd.data
            .get("offers")
            .or_else(|| sd.data.get("hasOffersCatalog"))
            .map(|v| !v.is_null())
            .unwrap_or(false)
    }
}

impl Default for EcommerceSignalsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for EcommerceSignalsAnalyzer {
    fn name(&self) -> &str {
        "ecommerce-signals"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.structured_data.is_empty() {
            return findings;
        }

        let mut has_product = false;
        let mut prices_found = Vec::new();
        let mut availability_found = Vec::new();
        let mut reviews_found = Vec::new();
        let mut offers_found = false;

        for sd in &ctx.page.structured_data {
            if Self::detect_product_schema(sd) {
                has_product = true;

                if let Some(price) = Self::extract_price(&sd.data) {
                    prices_found.push(price);
                }
                if let Some(avail) = Self::extract_availability(&sd.data) {
                    availability_found.push(avail);
                }
                reviews_found.extend(Self::detect_reviews(sd));
                if Self::detect_offers(sd) {
                    offers_found = true;
                }
            }

            if sd.r#type.as_deref() == Some("Offer") || sd.r#type.as_deref() == Some("AggregateOffer") {
                if let Some(price) = Self::extract_price(&sd.data) {
                    prices_found.push(price);
                }
            }

            if sd.r#type.as_deref() == Some("Review") || sd.r#type.as_deref() == Some("AggregateRating") {
                reviews_found.extend(Self::detect_reviews(sd));
            }
        }

        if has_product {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM001".to_string(),
                title: "Product schema detected".to_string(),
                description: "Product structured data found. This enables rich product \
                              results in search."
                    .to_string(),
                url: url.clone(),
                recommendation: "Ensure all required Product properties are present (name, \
                                 image, description, offers)."
                    .to_string(),
            });
        }

        if !prices_found.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM002".to_string(),
                title: "Price information detected".to_string(),
                description: format!(
                    "Prices found in structured data: {}.",
                    prices_found.join(", ")
                ),
                url: url.clone(),
                recommendation: "Verify prices match the visible page content.".to_string(),
            });
        }

        if !availability_found.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM003".to_string(),
                title: "Availability information detected".to_string(),
                description: format!(
                    "Availability: {}.",
                    availability_found.join(", ")
                ),
                url: url.clone(),
                recommendation: "Availability status should match the actual product state."
                    .to_string(),
            });
        }

        if !reviews_found.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM004".to_string(),
                title: "Review/rating information detected".to_string(),
                description: format!("Review data: {}.", reviews_found.join(", ")),
                url: url.clone(),
                recommendation: "Ratings and reviews enhance search result CTR. Keep them \
                                 updated."
                    .to_string(),
            });
        }

        if offers_found {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM005".to_string(),
                title: "Offer schema detected".to_string(),
                description: "Offer structured data found in product schema.".to_string(),
                url: url.clone(),
                recommendation: "Ensure offer includes price, priceCurrency, and availability."
                    .to_string(),
            });
        }

        if !has_product && !prices_found.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Schema,
                code: "ECOM006".to_string(),
                title: "Price data without Product schema".to_string(),
                description: "Price information found but no Product schema type detected. \
                              Search engines may not interpret this as product data."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add Product schema to wrap price and availability data."
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

    fn detect_locale_from_url(url: &str) -> Option<String> {
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

    fn is_locale_segment(s: &str) -> bool {
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

    fn detect_multilang_content(
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
        if locale_in_url.is_none() && ctx.page.meta.hreflang.is_empty() {
            if ctx.page.html_lang.is_some() {
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
        }

        if let Some(canonical) = &ctx.page.meta.canonical {
            if let Some(&ref hop_from) = self.known_hrefs.get(url) {
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

        let is_multilang =
            Self::detect_multilang_content(hreflang_tags, &ctx.page.html_lang);
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
    use crate::parser::{ExtractedImage, ExtractedLink, Heading, StructuredData};
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
        assert!(findings.iter().any(|f| f.severity == Severity::Critical));
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
        let hops = vec![RedirectHop {
            from: Url::parse("http://example.com/page").unwrap(),
            to: Url::parse("https://example.com/page").unwrap(),
            status_code: 301,
        }];
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
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "HEAD005"));
    }

    // ---- AnalyzerRegistry ----

    #[test]
    fn test_registry_default() {
        let config = default_config();
        let registry = AnalyzerRegistry::new(&config);
        assert_eq!(registry.len(), 23);
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
            fn name(&self) -> &str {
                "dummy"
            }
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

    // =========================================================================
    // LinkAnalyzer tests
    // =========================================================================

    #[test]
    fn test_link_analyzer_no_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx, &default_config());
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
            },
            ExtractedLink {
                href: "/contact".to_string(),
                text: "Contact".to_string(),
                rel: vec![],
                is_external: false,
            },
            ExtractedLink {
                href: "https://external.com".to_string(),
                text: "External".to_string(),
                rel: vec![],
                is_external: true,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(404));
        let findings = LinkAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "LINK002"));
    }

    #[test]
    fn test_link_analyzer_orphan_page() {
        let mut inbound = HashMap::new();
        inbound.insert("https://example.com/orphan".to_string(), 0);
        let analyzer = LinkAnalyzer::with_inbound_links(inbound);
        let page = make_page("https://example.com/orphan");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "LINK006"));
    }

    #[test]
    fn test_link_analyzer_not_orphan() {
        let mut inbound = HashMap::new();
        inbound.insert("https://example.com/page".to_string(), 3);
        let analyzer = LinkAnalyzer::with_inbound_links(inbound);
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = analyzer.analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "LINK006"));
    }

    #[test]
    fn test_link_analyzer_no_nofollow_when_absent() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "Normal link".to_string(),
            rel: vec![],
            is_external: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LinkAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "LINK003"));
    }

    // =========================================================================
    // ImageAnalyzer tests
    // =========================================================================

    #[test]
    fn test_image_analyzer_no_images() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx, &default_config());
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
            },
            ExtractedImage {
                src: "/b.png".to_string(),
                alt: "B".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: true,
            },
            ExtractedImage {
                src: "/c.png".to_string(),
                alt: "C".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = ImageAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = StructuredDataValidator::new().analyze(&ctx, &default_config());
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
        let findings = StructuredDataValidator::new().analyze(&ctx, &default_config());
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
        let findings = StructuredDataValidator::new().analyze(&ctx, &default_config());
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
        let findings = StructuredDataValidator::new().analyze(&ctx, &default_config());
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
        let findings = StructuredDataValidator::new().analyze(&ctx, &default_config());
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
        let findings = StructuredDataValidator::new().analyze(&ctx, &default_config());
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
        let findings = StructuredDataValidator::new().analyze(&ctx, &default_config());
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
        let findings = StructuredDataValidator::new().analyze(&ctx, &default_config());
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
        let findings = StructuredDataValidator::new().analyze(&ctx, &default_config());
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
        let findings = ContentQualityAnalyzer::new().analyze(&ctx, &default_config());
        // Should flag zero content
        assert!(findings.iter().any(|f| f.code == "CQ003"));
    }

    #[test]
    fn test_content_quality_thin_content() {
        let mut page = make_page("https://example.com");
        page.word_count = 150;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentQualityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "CQ004"));
    }

    #[test]
    fn test_content_quality_long_form() {
        let mut page = make_page("https://example.com");
        page.word_count = 5000;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentQualityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = ContentQualityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = ContentQualityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "CQ002"));
        let cq002 = findings.iter().find(|f| f.code == "CQ002").unwrap();
        assert!(cq002.description.contains("rust"));
    }

    #[test]
    fn test_content_quality_syllable_counting() {
        assert_eq!(ContentQualityAnalyzer::count_syllables("cat"), 1);
        assert_eq!(ContentQualityAnalyzer::count_syllables("hello"), 2);
        assert_eq!(ContentQualityAnalyzer::count_syllables("beautiful"), 3);
        assert_eq!(ContentQualityAnalyzer::count_syllables("a"), 1);
        assert_eq!(ContentQualityAnalyzer::count_syllables(""), 0);
    }

    #[test]
    fn test_content_quality_sentence_counting() {
        assert_eq!(ContentQualityAnalyzer::count_sentences("Hello world."), 1);
        assert_eq!(
            ContentQualityAnalyzer::count_sentences("First sentence. Second sentence! Third?"),
            3
        );
        assert_eq!(ContentQualityAnalyzer::count_sentences(""), 0);
        assert_eq!(ContentQualityAnalyzer::count_sentences("   "), 0);
    }

    #[test]
    fn test_content_quality_flesch_kincaid() {
        // Simple text should score higher
        let score = ContentQualityAnalyzer::flesch_kincaid(100, 5, 150);
        assert!(score > 50.0);

        // Complex text should score lower
        let score = ContentQualityAnalyzer::flesch_kincaid(100, 20, 300);
        assert!(score < 50.0);

        // Zero words should return 0
        assert_eq!(ContentQualityAnalyzer::flesch_kincaid(0, 0, 0), 0.0);
    }

    // =========================================================================
    // WordCountAnalyzer tests
    // =========================================================================

    #[test]
    fn test_word_count_analyzer_empty() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = WordCountAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = WordCountAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = WordCountAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "WC003"));
    }

    #[test]
    fn test_word_count_analyzer_long_sentences() {
        let mut page = make_page("https://example.com");
        page.word_count = 100;
        page.headings = vec![Heading {
            level: 1,
            text: "This is a very long sentence that contains many words and should trigger \
                   the long sentence warning because it has more than twenty five words on average"
                .to_string(),
            length: 150,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = WordCountAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "WC004"));
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
        let findings = WordCountAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "SEC004"));
    }

    #[test]
    fn test_security_headers_invalid_xcto() {
        let headers = vec![("X-Content-Type-Options".to_string(), "sniff".to_string())];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "SEC006"));
    }

    #[test]
    fn test_security_headers_xfo_sameorigin() {
        let headers = vec![("X-Frame-Options".to_string(), "SAMEORIGIN".to_string())];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
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
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
        };
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = SecurityHeaderAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = SslCertificateValidator::empty().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "SSL007"));
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = validator.analyze(&ctx, &default_config());
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
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "MOB001"));
    }

    #[test]
    fn test_mobile_optimal_viewport() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
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
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "MOB004"));
    }

    #[test]
    fn test_mobile_maximum_scale_restricted() {
        let mut page = make_page("https://example.com");
        page.meta.viewport =
            Some("width=device-width, initial-scale=1, maximum-scale=1.0".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "MOB005"));
    }

    #[test]
    fn test_mobile_fixed_width() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=980".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "MOB003"));
    }

    #[test]
    fn test_mobile_missing_width_directive() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "MOB002"));
    }

    #[test]
    fn test_mobile_non_standard_initial_scale() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=2.0".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "MOB009"));
    }

    #[test]
    fn test_mobile_touch_target_info() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
        // Should always have the touch target heuristic finding
        assert!(findings.iter().any(|f| f.code == "MOB006"));
    }

    #[test]
    fn test_mobile_font_size_info() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "MOB007"));
    }

    #[test]
    fn test_mobile_horizontal_scroll_info() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=1".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "MOB008"));
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
        let findings = MobileFriendlinessChecker::new().analyze(&ctx, &default_config());
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
            },
            ExtractedImage {
                src: "/b.jpg".to_string(),
                alt: "Good alt".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "A11Y001"));
    }

    #[test]
    fn test_a11y_no_headings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "A11Y005"));
    }

    #[test]
    fn test_a11y_missing_main_landmark() {
        let mut page = make_page("https://example.com");
        page.has_main_landmark = false;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "A11Y006"));
    }

    #[test]
    fn test_a11y_has_main_landmark() {
        let mut page = make_page("https://example.com");
        page.has_main_landmark = true;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "A11Y011"));
    }

    #[test]
    fn test_a11y_positive_tabindex() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "A11Y012"));
    }

    #[test]
    fn test_a11y_no_positive_tabindex() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = false;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "A11Y012"));
    }

    #[test]
    fn test_a11y_table_missing_headers() {
        let mut page = make_page("https://example.com");
        page.tables_total = 2;
        page.tables_with_headers = 1;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "A11Y014"));
    }

    #[test]
    fn test_a11y_missing_lang_attribute() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = false;
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "A11Y016"));
    }

    #[test]
    fn test_a11y_has_lang_attribute() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "A11Y016"));
    }

    #[test]
    fn test_a11y_no_images_no_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
            href: "/about".to_string(),
            text: "About us".to_string(),
            rel: vec![],
            is_external: false,
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = AccessibilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "SOCIAL001"));
    }

    #[test]
    fn test_social_og_image_adequate_dimensions() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = Some("https://example.com/og.png".to_string());
        page.og_image_width = Some(1200);
        page.og_image_height = Some(630);
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "SOCIAL002"));
        assert!(!findings.iter().any(|f| f.code == "SOCIAL003"));
    }

    #[test]
    fn test_social_missing_twitter_card() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "SOCIAL004"));
    }

    #[test]
    fn test_social_valid_twitter_card() {
        let mut page = make_page("https://example.com");
        page.meta.twitter.card = Some("summary_large_image".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "SOCIAL004"));
        assert!(!findings.iter().any(|f| f.code == "SOCIAL005"));
    }

    #[test]
    fn test_social_invalid_twitter_card() {
        let mut page = make_page("https://example.com");
        page.meta.twitter.card = Some("invalid_type".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "SOCIAL005"));
    }

    #[test]
    fn test_social_incomplete_og_tags() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
        let score = findings.iter().find(|f| f.code == "SOCIAL008").unwrap();
        assert!(score.description.contains("8/8"));
    }

    #[test]
    fn test_social_no_og_image_no_dimension_warning() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = None;
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
        // No OG image = no dimension warning needed
        assert!(!findings.iter().any(|f| f.code == "SOCIAL001"));
    }

    #[test]
    fn test_social_all_twitter_cards_valid() {
        for card_type in &["summary", "summary_large_image", "app", "player"] {
            let mut page = make_page("https://example.com");
            page.meta.twitter.card = Some(card_type.to_string());
            let ctx = make_ctx(&page, Some(200));
            let findings = SocialMediaAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = EntityAnalyzer::new().analyze(&ctx, &default_config());
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
        assert!(topics.iter().any(|t| t.contains("rust") || t.contains("programming")));
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
        let (score, label) = EntityAnalyzer::analyze_sentiment(
            "The page contains information about the topic.",
        );
        assert_eq!(label, "neutral");
        assert!(score >= -0.05 && score <= 0.05);
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
        let findings = EntityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "ENTITY001"));
    }

    // =========================================================================
    // EnhancedReadabilityAnalyzer tests
    // =========================================================================

    #[test]
    fn test_readability_empty_page() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "READ001"));
        let fk = findings.iter().find(|f| f.code == "READ001").unwrap();
        assert!(fk.description.contains("Grade level:"));
    }

    #[test]
    fn test_readability_syllable_counting() {
        assert_eq!(EnhancedReadabilityAnalyzer::count_syllables("cat"), 1);
        assert_eq!(EnhancedReadabilityAnalyzer::count_syllables("hello"), 2);
        assert_eq!(EnhancedReadabilityAnalyzer::count_syllables("beautiful"), 3);
        assert_eq!(EnhancedReadabilityAnalyzer::count_syllables("a"), 1);
        assert_eq!(EnhancedReadabilityAnalyzer::count_syllables(""), 0);
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
        let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "READ001"));
        assert!(findings.iter().any(|f| f.code == "READ002"));
        assert!(findings.iter().any(|f| f.code == "READ003"));
        assert!(findings.iter().any(|f| f.code == "READ004"));
        assert!(findings.iter().any(|f| f.code == "READ005"));
    }

    #[test]
    fn test_readability_grade_label() {
        assert_eq!(EnhancedReadabilityAnalyzer::grade_label(0.5), "kindergarten");
        assert_eq!(EnhancedReadabilityAnalyzer::grade_label(4.0), "elementary school");
        assert_eq!(EnhancedReadabilityAnalyzer::grade_label(7.0), "middle school");
        assert_eq!(EnhancedReadabilityAnalyzer::grade_label(10.0), "high school");
        assert_eq!(EnhancedReadabilityAnalyzer::grade_label(14.0), "college");
        assert_eq!(EnhancedReadabilityAnalyzer::grade_label(18.0), "postgraduate");
    }

    #[test]
    fn test_readability_ease_label() {
        assert_eq!(EnhancedReadabilityAnalyzer::reading_ease_label(95.0), "very easy");
        assert_eq!(EnhancedReadabilityAnalyzer::reading_ease_label(85.0), "easy");
        assert_eq!(EnhancedReadabilityAnalyzer::reading_ease_label(75.0), "fairly easy");
        assert_eq!(EnhancedReadabilityAnalyzer::reading_ease_label(65.0), "standard");
        assert_eq!(
            EnhancedReadabilityAnalyzer::reading_ease_label(55.0),
            "fairly difficult"
        );
        assert_eq!(EnhancedReadabilityAnalyzer::reading_ease_label(35.0), "difficult");
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
        let findings = KeywordAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = KeywordAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = KeywordAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "ISEO003"));
    }

    #[test]
    fn test_iseo_multilang_content() {
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
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "ISEO006"));
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
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx, &default_config());
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
        let findings = InternationalSeoAnalyzer::new().analyze(&ctx, &default_config());
        assert!(!findings.iter().any(|f| f.code == "ISEO004"));
    }

    #[test]
    fn test_iseo_multilang_detect() {
        assert!(InternationalSeoAnalyzer::detect_multilang_content(
            &[crate::meta::HreflangTag {
                lang: "en".to_string(),
                url: Url::parse("https://example.com/en").unwrap(),
            }],
            &None,
        ));
        assert!(InternationalSeoAnalyzer::detect_multilang_content(
            &[],
            &Some("en-US".to_string()),
        ));
        assert!(!InternationalSeoAnalyzer::detect_multilang_content(
            &[],
            &Some("en".to_string()),
        ));
    }
}
