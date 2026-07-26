use std::collections::HashSet;
use std::time::Duration;
use url::Url;

use crate::storage::{IssueCategory, Severity};
use crate::CrawlConfig;

use super::{AnalysisContext, Analyzer, Finding, SslCertificateInfo};

// ---------------------------------------------------------------------------
// 1. HTTP Status Analyzer
// ---------------------------------------------------------------------------

pub struct HttpStatusAnalyzer;

impl HttpStatusAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a 200 response looks like an error page (soft 404).
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
                // Check for soft 404 using body content
                if let Some(body) = ctx.body {
                    if Self::is_soft_404(body) {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Http,
                            code: "HTTP007".to_string(),
                            title: "Possible soft 404 — error page content detected".to_string(),
                            description: "Page returned 200 but contains text typically found \
                                         on error pages (e.g., \"page not found\"). This may \
                                         indicate a soft 404."
                                .to_string(),
                            url: url.clone(),
                            recommendation: "Verify the page renders correctly. Fix server-side \
                                             rendering issues or return a proper 404 status code."
                                .to_string(),
                        });
                    }
                }
                // Also check for empty body
                let body_lower = ctx.page.word_count;
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
    pub(crate) fn is_disallowed(path: &str, disallowed: &[String]) -> bool {
        for pattern in disallowed {
            if path.starts_with(pattern.as_str()) {
                return true;
            }
        }
        false
    }

    /// Check if a path is explicitly allowed for a user-agent.
    pub(crate) fn is_allowed(path: &str, allowed: &[String]) -> bool {
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
    ///
    /// Returns true for MD5, SHA-1, or RSA without SHA-256+.
    /// Explicit parentheses to clarify operator precedence:
    /// `(md5 || sha1) || (rsa && !sha256 && !sha384 && !sha512)`
    fn is_weak_algorithm(algo: &str) -> bool {
        let lower = algo.to_lowercase();
        (lower.contains("md5") || lower.contains("sha1"))
            || (lower.contains("with rsa encryption")
                && !lower.contains("sha256")
                && !lower.contains("sha384")
                && !lower.contains("sha512"))
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
                if let Some(wildcard_domain) = subject.strip_prefix("*.") {
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
                if let Some(wildcard_domain) = san.strip_prefix("*.") {
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
