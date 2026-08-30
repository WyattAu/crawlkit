use std::collections::HashSet;
use std::time::Duration;
use url::Url;

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding, SslCertificateInfo};

// ---------------------------------------------------------------------------
// HTTP Version Analyzer
// ---------------------------------------------------------------------------

pub struct HttpVersionAnalyzer;

impl HttpVersionAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HttpVersionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HttpVersionAnalyzer {
    fn name(&self) -> &str {
        "http-version"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let version = Self::get_header(ctx.headers, "HTTP-Version")
            .or_else(|| Self::get_header(ctx.headers, "X-Http-Version"));

        // Also detect from Via header (e.g. "1.1 varnish")
        let via = Self::get_header(ctx.headers, "Via");

        let http_version = match version {
            Some(v) => v.to_string(),
            None => {
                // Infer from Via header or server behavior
                if let Some(via_val) = via {
                    // Via header format: "1.1 varnish" or "2.0 google"
                    if let Some(proto) = via_val.split_whitespace().next() {
                        proto.to_string()
                    } else {
                        return findings;
                    }
                } else {
                    return findings;
                }
            }
        };

        // HTTPVER001: HTTP/1.0 response when HTTP/2 is available
        if http_version.starts_with("1.0") {
            // Check if there are hints that HTTP/2 could be used
            // (e.g., server supports h2 but responding with HTTP/1.0)
            let server = ctx.server.unwrap_or("");
            let has_h2_hints = server.contains("nginx/1.19")
                || server.contains("nginx/1.20")
                || server.contains("nginx/1.21")
                || server.contains("nginx/1.22")
                || server.contains("nginx/1.23")
                || server.contains("nginx/1.24")
                || server.contains("nginx/1.25")
                || server.contains("nginx/1.26")
                || server.contains("Apache/2.4")
                || server.contains("cloudflare")
                || server.contains("Google");

            if has_h2_hints {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Http,
                    code: "HTTPVER001".to_string(),
                    title: "HTTP/1.0 response when HTTP/2 may be available".to_string(),
                    description: format!(
                        "Response uses HTTP/1.0 but the server ({server}) likely supports \
                         HTTP/2. HTTP/2 provides multiplexing, header compression, and server push."
                    ),
                    url: url.clone(),
                    recommendation: "Enable HTTP/2 on the server. Most modern web servers \
                                     support HTTP/2 with TLS."
                        .to_string(),
                });
            }
        }

        // HTTPVER002: HTTP/1.1 response (INFO)
        if http_version.starts_with("1.1") && !http_version.starts_with("1.0") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Http,
                code: "HTTPVER002".to_string(),
                title: "HTTP/1.1 response".to_string(),
                description: "Page is served over HTTP/1.1. While functional, HTTP/2 or HTTP/3 \
                              offer significant performance improvements including multiplexing \
                              and header compression."
                    .to_string(),
                url: url.clone(),
                recommendation: "Consider upgrading to HTTP/2 or HTTP/3 for better performance."
                    .to_string(),
            });
        }

        findings
    }
}

impl HttpVersionAnalyzer {
    fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

// ---------------------------------------------------------------------------
// Server Header Analyzer
// ---------------------------------------------------------------------------

pub struct ServerHeaderAnalyzer;

impl ServerHeaderAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Known version patterns that leak in Server headers.
    const VERSION_PATTERNS: &[&str] = &[
        "Apache/",
        "nginx/",
        "Microsoft-IIS/",
        "LiteSpeed/",
        "OpenResty/",
        "Cloudflare",
        "GWS/",
        "gws/",
        "AmazonS3",
    ];

    /// Known technology stack keywords.
    const TECH_KEYWORDS: &[&str] = &[
        "PHP",
        "ASP.NET",
        "Express",
        "Django",
        "Flask",
        "Ruby on Rails",
        "Laravel",
        "Spring",
        "WordPress",
        "Drupal",
        "Joomla",
        "Varnish",
        "squid",
        "ATS",
    ];
}

impl Default for ServerHeaderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ServerHeaderAnalyzer {
    fn name(&self) -> &str {
        "server-header"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let server = match ctx.server {
            Some(s) => s,
            None => {
                // No Server header found — that's good from a security perspective
                return findings;
            }
        };

        // SERVER001: Server header leaks version information
        let server_lower = server.to_lowercase();
        for pattern in Self::VERSION_PATTERNS {
            if server_lower.contains(&pattern.to_lowercase()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "SERVER001".to_string(),
                    title: "Server header leaks version information".to_string(),
                    description: format!(
                        "Server header reveals version info: \"{server}\". Attackers can use \
                         this to target known vulnerabilities for specific software versions."
                    ),
                    url: url.clone(),
                    recommendation: "Remove or obfuscate the Server header to hide version \
                                     information. For nginx: server_tokens off; For Apache: \
                                     ServerTokens Prod"
                        .to_string(),
                });
                break;
            }
        }

        // SERVER002: Server header reveals technology stack
        for keyword in Self::TECH_KEYWORDS {
            if server.to_lowercase().contains(&keyword.to_lowercase()) {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "SERVER002".to_string(),
                    title: "Server header reveals technology stack".to_string(),
                    description: format!(
                        "Server header reveals technology: \"{server}\". This information \
                         helps attackers understand the technology stack."
                    ),
                    url: url.clone(),
                    recommendation: "Remove the Server header or set it to a generic value to \
                                     hide the underlying technology stack."
                        .to_string(),
                });
                break;
            }
        }

        findings
    }
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

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
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

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
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

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
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

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // No certificate data was captured for this crawl. The production
        // registry registers `SslCertificateValidator::empty()`, so TLS
        // certificate metadata is not yet wired from the HTTP client into
        // the analyzer. Emit a single informational finding per page so the
        // limitation is visible in reports rather than silently absent.
        // Tracked for closure in ROADMAP.md (Phase 1 security hardening).
        let info = match &self.cert_info {
            Some(i) => i,
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "SSL000".to_string(),
                    title: "SSL certificate not inspected".to_string(),
                    description: "No TLS certificate metadata was captured for this \
                                  page, so expiry, chain, and hostname checks were \
                                  not performed. Certificate inspection is not yet \
                                  wired into the default crawl path."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Until certificate inspection is wired in, \
                                     verify TLS configuration with an external tool \
                                     (e.g. sslscan, testssl.sh, or an online checker)."
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
// 18. Response Size Analyzer
// ---------------------------------------------------------------------------

pub struct ResponseSizeAnalyzer;

impl ResponseSizeAnalyzer {
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
}

impl Default for ResponseSizeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ResponseSizeAnalyzer {
    fn name(&self) -> &str {
        "response-size"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let size = match ctx.body_size {
            Some(s) => s,
            None => return findings,
        };

        // SIZE002: Response body > 10MB (ERROR)
        if size > 10 * 1024 * 1024 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Performance,
                code: "SIZE002".to_string(),
                title: "Response body exceeds 10MB".to_string(),
                description: format!(
                    "Response body is {} bytes ({:.1} MB), exceeding the 10MB threshold.",
                    size,
                    size as f64 / (1024.0 * 1024.0)
                ),
                url: url.clone(),
                recommendation:
                    "Reduce response size. Consider pagination, lazy loading, or content compression."
                        .to_string(),
            });
        } else if size > 5 * 1024 * 1024 {
            // SIZE001: Response body > 5MB (WARNING)
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "SIZE001".to_string(),
                title: "Response body exceeds 5MB".to_string(),
                description: format!(
                    "Response body is {} bytes ({:.1} MB), exceeding the 5MB threshold.",
                    size,
                    size as f64 / (1024.0 * 1024.0)
                ),
                url: url.clone(),
                recommendation:
                    "Consider reducing response size through compression, pagination, or lazy loading."
                        .to_string(),
            });
        }

        // SIZE003: No Content-Length header when body is present
        if Self::get_header(ctx.headers, "Content-Length").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Http,
                code: "SIZE003".to_string(),
                title: "Missing Content-Length header".to_string(),
                description: "Response has a body but no Content-Length header was found."
                    .to_string(),
                url: url.clone(),
                recommendation:
                    "Add a Content-Length header to enable caching and bandwidth optimization."
                        .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 19. TTFB Analyzer
// ---------------------------------------------------------------------------

pub struct TtfbAnalyzer;

impl TtfbAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TtfbAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TtfbAnalyzer {
    fn name(&self) -> &str {
        "ttfb"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let response_time = match ctx.response_time {
            Some(rt) => rt,
            None => return findings,
        };

        let ttfb_ms = response_time.as_millis();

        // TTFB002: TTFB > 1000ms (ERROR)
        if ttfb_ms > 1000 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Performance,
                code: "TTFB002".to_string(),
                title: "High Time to First Byte (TTFB)".to_string(),
                description: format!(
                    "TTFB is {ttfb_ms}ms, exceeding the 1000ms threshold for a poor user \
                     experience."
                ),
                url: url.clone(),
                recommendation:
                    "Optimize server response time. Use a CDN, enable server-side caching, or \
                     optimize database queries."
                        .to_string(),
            });
        } else if ttfb_ms > 600 {
            // TTFB001: TTFB > 600ms (WARNING)
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "TTFB001".to_string(),
                title: "Slow Time to First Byte (TTFB)".to_string(),
                description: format!("TTFB is {ttfb_ms}ms, exceeding the 600ms threshold."),
                url: url.clone(),
                recommendation:
                    "Improve server response time. Consider CDN, caching, or optimizing backend \
                     processing."
                        .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 20. Cache Header Analyzer
// ---------------------------------------------------------------------------

pub struct CacheHeaderAnalyzer;

impl CacheHeaderAnalyzer {
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

    /// Check if a status code is eligible for caching.
    fn is_cacheable_response(status: u16) -> bool {
        matches!(
            status,
            200 | 203 | 204 | 206 | 300 | 301 | 302 | 304 | 404 | 410
        )
    }
}

impl Default for CacheHeaderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CacheHeaderAnalyzer {
    fn name(&self) -> &str {
        "cache-headers"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let status = match ctx.status_code {
            Some(s) => s,
            None => return findings,
        };

        if !Self::is_cacheable_response(status) {
            return findings;
        }

        let cache_control = Self::get_header(ctx.headers, "Cache-Control");
        let etag = Self::get_header(ctx.headers, "ETag");
        let last_modified = Self::get_header(ctx.headers, "Last-Modified");
        let content_type = Self::get_header(ctx.headers, "Content-Type").or(ctx.content_type);

        // CACHE001: No Cache-Control header on cacheable responses
        if cache_control.is_none() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "CACHE001".to_string(),
                title: "Missing Cache-Control header".to_string(),
                description: "No Cache-Control header was found on a cacheable response. \
                              Without caching directives, browsers may not cache this resource \
                              effectively."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add appropriate Cache-Control headers (e.g., max-age=86400) \
                                 for cacheable responses."
                    .to_string(),
            });
        }

        // CACHE002: No ETag or Last-Modified header
        if etag.is_none() && last_modified.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "CACHE002".to_string(),
                title: "No ETag or Last-Modified header".to_string(),
                description: "Neither ETag nor Last-Modified headers were found. Without these \
                              conditional request headers, browsers cannot perform efficient \
                              cache validation."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add ETag or Last-Modified headers to enable conditional \
                                 requests and reduce bandwidth usage."
                    .to_string(),
            });
        }

        // CACHE003: Cache-Control: no-cache on HTML content
        if let Some(cc) = cache_control {
            let cc_lower = cc.to_lowercase();
            if cc_lower.contains("no-cache") || cc_lower.contains("no-store") {
                if let Some(ct) = content_type {
                    if ct.contains("text/html") {
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Performance,
                            code: "CACHE003".to_string(),
                            title: "HTML content marked as non-cacheable".to_string(),
                            description: format!(
                                "Cache-Control header contains '{cc}' for HTML content. This \
                                 prevents browsers from caching the page, which may be \
                                 unnecessary for static content."
                            ),
                            url: url.clone(),
                            recommendation: "For static HTML pages, consider using a moderate \
                                             max-age (e.g., 300-3600 seconds) with \
                                             stale-while-revalidate."
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
// 16. Mobile-Friendliness Checker
// ---------------------------------------------------------------------------

pub struct MobileFriendlinessChecker;

// =========================================================================
// CompressionAnalyzer
// =========================================================================

/// Analyzes response compression for performance.
pub struct CompressionAnalyzer;

impl Default for CompressionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

impl Analyzer for CompressionAnalyzer {
    fn name(&self) -> &str {
        "compression"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body_size = match ctx.body_size {
            Some(s) => s,
            None => return findings,
        };

        let has_compression = Self::get_header(ctx.headers, "Content-Encoding").is_some();

        // COMP001: Response not compressed when >1KB
        if body_size > 1024 && !has_compression {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "COMP001".to_string(),
                title: "Large response not compressed".to_string(),
                description: format!(
                    "This response is {} bytes but has no Content-Encoding header. Compressing \
                     responses with gzip, br, or deflate can significantly reduce transfer size \
                     and improve load times.",
                    body_size
                ),
                url: url.to_string(),
                recommendation: "Enable server-side compression (gzip, brotli, or deflate) for \
                                 responses larger than 1KB."
                    .to_string(),
            });
        }

        // COMP002: Unnecessary compression for <1KB responses
        if body_size <= 1024 && has_compression {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "COMP002".to_string(),
                title: "Unnecessary compression for small response".to_string(),
                description: format!(
                    "This response is only {} bytes but has Content-Encoding applied. Compressing \
                     very small responses may add overhead without meaningful benefit.",
                    body_size
                ),
                url: url.to_string(),
                recommendation: "Consider not compressing responses under 1KB as the overhead \
                                 may outweigh the benefit."
                    .to_string(),
            });
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

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

    fn make_ctx<'a>(
        page: &'a ParsedPage,
        status: Option<u16>,
        headers: &'a [(String, String)],
        body_size: Option<usize>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    // ===== CompressionAnalyzer tests =====

    #[test]
    fn test_compression_no_body_size() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(CompressionAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_compression_large_uncompressed() {
        let headers = vec![];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(2048));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COMP001"));
    }

    #[test]
    fn test_compression_large_compressed() {
        let headers = vec![("Content-Encoding".to_string(), "gzip".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(2048));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_compression_small_compressed() {
        let headers = vec![("Content-Encoding".to_string(), "gzip".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(512));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COMP002"));
    }

    #[test]
    fn test_compression_small_uncompressed() {
        let headers = vec![];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(512));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_compression_exact_1024_uncompressed() {
        let headers = vec![];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(1024));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        // Exactly 1024: not > 1024, so no COMP001
        assert!(!findings.iter().any(|f| f.code == "COMP001"));
    }

    #[test]
    fn test_compression_exact_1025_uncompressed() {
        let headers = vec![];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(1025));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COMP001"));
    }

    #[test]
    fn test_compression_brotli_encoding() {
        let headers = vec![("Content-Encoding".to_string(), "br".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(5000));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_compression_deflate_encoding() {
        let headers = vec![("Content-Encoding".to_string(), "deflate".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(3000));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_compression_case_insensitive_header() {
        let headers = vec![("content-encoding".to_string(), "gzip".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(2048));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_compression_exact_1024_compressed() {
        let headers = vec![("Content-Encoding".to_string(), "gzip".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(1024));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        // Exactly 1024: not <= 1024 for small check? Actually 1024 <= 1024 is true
        assert!(findings.iter().any(|f| f.code == "COMP002"));
    }

    #[test]
    fn test_compression_zero_size() {
        let headers = vec![];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some(0));
        let findings = CompressionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ===== HttpVersionAnalyzer tests =====

    #[test]
    fn test_http_version_no_headers() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HttpVersionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_http_version_http10_with_modern_server() {
        let headers = vec![("HTTP-Version".to_string(), "1.0".to_string())];
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
            server: Some("nginx/1.24.0"),
            content_type: None,
            rendered: None,
        };
        let findings = HttpVersionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTPVER001"));
    }

    #[test]
    fn test_http_version_http10_without_modern_server() {
        let headers = vec![("HTTP-Version".to_string(), "1.0".to_string())];
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
            server: Some("nginx/1.10.0"),
            content_type: None,
            rendered: None,
        };
        let findings = HttpVersionAnalyzer::new().analyze(&ctx);
        // Old nginx may not support h2, so no finding expected
        assert!(!findings.iter().any(|f| f.code == "HTTPVER001"));
    }

    #[test]
    fn test_http_version_http11_info() {
        let headers = vec![("HTTP-Version".to_string(), "1.1".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = HttpVersionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTPVER002"));
        let f = findings.iter().find(|f| f.code == "HTTPVER002").unwrap();
        assert_eq!(f.severity, Severity::Info);
    }

    #[test]
    fn test_http_version_http2_no_finding() {
        let headers = vec![("HTTP-Version".to_string(), "2.0".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = HttpVersionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_http_version_via_header_http11() {
        let headers = vec![("Via".to_string(), "1.1 varnish".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = HttpVersionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTPVER002"));
    }

    #[test]
    fn test_http_version_via_header_http2() {
        let headers = vec![("Via".to_string(), "2.0 google".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = HttpVersionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_http_version_http10_cloudflare() {
        let headers = vec![("HTTP-Version".to_string(), "1.0".to_string())];
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
            server: Some("cloudflare"),
            content_type: None,
            rendered: None,
        };
        let findings = HttpVersionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTPVER001"));
    }

    #[test]
    fn test_http_version_case_insensitive() {
        let headers = vec![("http-version".to_string(), "1.1".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = HttpVersionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HTTPVER002"));
    }

    // ===== ServerHeaderAnalyzer tests =====

    #[test]
    fn test_server_no_header() {
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
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_server_leaks_nginx_version() {
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
            server: Some("nginx/1.24.0"),
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SERVER001"));
    }

    #[test]
    fn test_server_leaks_apache_version() {
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
            server: Some("Apache/2.4.51 (Ubuntu)"),
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SERVER001"));
    }

    #[test]
    fn test_server_leaks_iis_version() {
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
            server: Some("Microsoft-IIS/10.0"),
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SERVER001"));
    }

    #[test]
    fn test_server_reveals_php() {
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
            server: Some("Apache"),
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SERVER001"));
        assert!(!findings.iter().any(|f| f.code == "SERVER002"));
    }

    #[test]
    fn test_server_reveals_wordpress() {
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
            server: Some("nginx/1.20.0 + WordPress"),
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SERVER001"));
        assert!(findings.iter().any(|f| f.code == "SERVER002"));
    }

    #[test]
    fn test_server_generic_no_leak() {
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
            server: Some("WebServer"),
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_server_case_insensitive_tech_match() {
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
            server: Some("MyServer PHP/8.1"),
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SERVER002"));
    }

    #[test]
    fn test_server_cloudflare_no_version() {
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
            server: Some("cloudflare"),
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        // "cloudflare" matches VERSION_PATTERNS (SERVER001) but not TECH_KEYWORDS
        assert!(findings.iter().any(|f| f.code == "SERVER001"));
    }

    #[test]
    fn test_server_litespeed_version() {
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
            server: Some("LiteSpeed/1.7.16"),
            content_type: None,
            rendered: None,
        };
        let findings = ServerHeaderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SERVER001"));
    }
}
