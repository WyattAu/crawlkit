//! Security header analyzers extracted from `security_analyzers.rs`.
//!
//! Phase 2 SRP step: each analyzer group moves to its own module so the
//! 13.5k-line monolith shrinks incrementally. No public API change —
//! types are re-exported from `analyzers::mod` identically.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------
// Subresource Integrity (SRI) Analyzer
// ---------------------------------------------------------------------------

pub struct SriAnalyzer;

impl SriAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SriAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SriAnalyzer {
    fn name(&self) -> &str {
        "sri"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Check external scripts without integrity
        let external_scripts_without_integrity: Vec<&str> = ctx
            .page
            .scripts
            .iter()
            .filter(|s| {
                s.src.as_ref().is_some_and(|src| {
                    let is_external = src.starts_with("http://")
                        || src.starts_with("https://")
                        || src.starts_with("//");
                    is_external && !s.has_integrity
                })
            })
            .filter_map(|s| s.src.as_deref())
            .collect();

        if !external_scripts_without_integrity.is_empty() {
            let examples: Vec<&str> = external_scripts_without_integrity
                .iter()
                .take(5)
                .copied()
                .collect();
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "SRI001".to_string(),
                title: "External scripts missing integrity attribute".to_string(),
                description: format!(
                    "{} external script(s) lack the integrity attribute: {}. Without SRI, \
                     compromised CDNs or man-in-the-middle attacks could inject malicious code.",
                    external_scripts_without_integrity.len(),
                    examples.join(", ")
                ),
                url: url.to_string(),
                recommendation: "Add an integrity attribute with the expected hash to all \
                                 external scripts. Use a tool like SRI Hash Generator to \
                                 compute the hash."
                    .into(),
            });
        }

        // Check external stylesheets without integrity
        let external_styles_without_integrity: Vec<&str> = ctx
            .page
            .styles
            .iter()
            .filter(|s| {
                s.href.as_ref().is_some_and(|href| {
                    let is_external = href.starts_with("http://")
                        || href.starts_with("https://")
                        || href.starts_with("//");
                    is_external && !s.has_integrity
                })
            })
            .filter_map(|s| s.href.as_deref())
            .collect();

        if !external_styles_without_integrity.is_empty() {
            let examples: Vec<&str> = external_styles_without_integrity
                .iter()
                .take(5)
                .copied()
                .collect();
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "SRI002".to_string(),
                title: "External stylesheets missing integrity attribute".to_string(),
                description: format!(
                    "{} external stylesheet(s) lack the integrity attribute: {}. Without SRI, \
                     a compromised stylesheet could alter page layout or inject CSS-based attacks.",
                    external_styles_without_integrity.len(),
                    examples.join(", ")
                ),
                url: url.to_string(),
                recommendation: "Add an integrity attribute with the expected hash to all \
                                 external stylesheets."
                    .into(),
            });
        }

        findings
    }
}

// =========================================================================
// ContentSecurityPolicyAnalyzer
// =========================================================================

/// Analyzes Content-Security-Policy for insecure directives.
pub struct ContentSecurityPolicyAnalyzer;

impl Default for ContentSecurityPolicyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentSecurityPolicyAnalyzer {
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

impl Analyzer for ContentSecurityPolicyAnalyzer {
    fn name(&self) -> &str {
        "content-security-policy"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let csp = match Self::get_header(ctx.headers, "Content-Security-Policy") {
            Some(v) => v,
            None => return findings,
        };

        // CSP001: script-src allows unsafe-inline
        let lower = csp.to_lowercase();
        if let Some(script_src_pos) = lower.find("script-src") {
            let after = &csp[script_src_pos..];
            if after
                .split(';')
                .next()
                .unwrap_or("")
                .contains("'unsafe-inline'")
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "CSP001".to_string(),
                    title: "CSP script-src allows unsafe-inline".to_string(),
                    description: "The Content-Security-Policy script-src directive includes \
                                  'unsafe-inline', which allows inline JavaScript execution. \
                                  This weakens XSS protection."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Remove 'unsafe-inline' from script-src and use nonces or \
                                     hashes for inline scripts."
                        .to_string(),
                });
            }
        }

        // CSP002: Missing frame-ancestors
        if !lower.contains("frame-ancestors") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSP002".to_string(),
                title: "CSP missing frame-ancestors directive".to_string(),
                description: "The Content-Security-Policy header does not include a \
                              'frame-ancestors' directive. Without it, the page may be embedded \
                              in iframes from any origin, enabling clickjacking."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add frame-ancestors 'self' (or frame-ancestors 'none') to \
                                 the Content-Security-Policy header."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// ReferrerPolicyAnalyzer
// =========================================================================

/// Analyzes Referrer-Policy header configuration.
pub struct ReferrerPolicyAnalyzer;

impl Default for ReferrerPolicyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferrerPolicyAnalyzer {
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

impl Analyzer for ReferrerPolicyAnalyzer {
    fn name(&self) -> &str {
        "referrer-policy"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Referrer-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "REF001".to_string(),
                    title: "Missing Referrer-Policy header".to_string(),
                    description: "No Referrer-Policy header was found. Without this header, \
                                  browsers may send full URLs as referrers, leaking sensitive \
                                  information."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Add Referrer-Policy: strict-origin-when-cross-origin to \
                                     control referrer information leakage."
                        .to_string(),
                });
            }
            Some(value) => {
                if value.trim().eq_ignore_ascii_case("unsafe-url") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "REF002".to_string(),
                        title: "Referrer-Policy set to unsafe-url".to_string(),
                        description: "The Referrer-Policy header is set to 'unsafe-url', which \
                                      sends the full URL (including path and query string) as \
                                      referrer for all requests, including cross-origin."
                            .to_string(),
                        url: url.to_string(),
                        recommendation: "Use 'strict-origin-when-cross-origin' or \
                                         'no-referrer-when-downgrade' instead of 'unsafe-url'."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// XFrameOptionsAnalyzer
// =========================================================================

/// Analyzes X-Frame-Options header for clickjacking protection.
pub struct XFrameOptionsAnalyzer;

impl Default for XFrameOptionsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl XFrameOptionsAnalyzer {
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

impl Analyzer for XFrameOptionsAnalyzer {
    fn name(&self) -> &str {
        "x-frame-options"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Only check HTML pages
        if let Some(ct) = ctx.content_type {
            if !ct.contains("text/html") {
                return findings;
            }
        }

        match Self::get_header(ctx.headers, "X-Frame-Options") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XFO001".to_string(),
                    title: "Missing X-Frame-Options header on HTML page".to_string(),
                    description: "No X-Frame-Options header was found on this HTML page. This \
                                  header prevents the page from being embedded in iframes, which \
                                  protects against clickjacking attacks."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set X-Frame-Options to DENY (preferred) or SAMEORIGIN."
                        .to_string(),
                });
            }
            Some(value) => {
                if value.trim().eq_ignore_ascii_case("ALLOWALL") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "XFO002".to_string(),
                        title: "X-Frame-Options set to ALLOWALL".to_string(),
                        description: "The X-Frame-Options header is set to 'ALLOWALL', which \
                                      permits the page to be embedded in iframes from any origin. \
                                      This provides no clickjacking protection."
                            .to_string(),
                        url: url.to_string(),
                        recommendation: "Set X-Frame-Options to DENY or SAMEORIGIN instead of \
                                         ALLOWALL."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Mixed Content Analyzer
// ---------------------------------------------------------------------------

pub struct MixedContentAnalyzer;

impl MixedContentAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MixedContentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MixedContentAnalyzer {
    fn name(&self) -> &str {
        "mixed-content"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Only check HTTPS pages
        if !url.starts_with("https://") {
            return findings;
        }

        let body = ctx.body.unwrap_or("");

        // MIXED001: HTTP resources on HTTPS page
        let http_resources = Self::find_http_resources(body);
        if !http_resources.is_empty() {
            let examples = if http_resources.len() > 5 {
                format!(
                    "{}, ...",
                    http_resources
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                http_resources.join(", ")
            };
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "MIXED001".to_string(),
                title: "HTTP resources on HTTPS page".to_string(),
                description: format!(
                    "Page loads {} resource(s) over HTTP: {}. Mixed content prevents full \
                     HTTPS security and may be blocked by browsers.",
                    http_resources.len(),
                    examples
                ),
                url: url.clone(),
                recommendation: "Update all resource URLs to use HTTPS. This includes images, \
                                 scripts, stylesheets, and iframes."
                    .to_string(),
            });
        }

        // MIXED002: Mixed content forms
        let http_forms = Self::find_http_forms(body);
        if !http_forms.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Security,
                code: "MIXED002".to_string(),
                title: "Form submissions over HTTP".to_string(),
                description: format!(
                    "Found {} form(s) with action URLs using HTTP. User data submitted \
                     through these forms will be transmitted in plaintext.",
                    http_forms.len()
                ),
                url: url.clone(),
                recommendation: "Change form action URLs to HTTPS to protect user data in \
                                 transit."
                    .to_string(),
            });
        }

        findings
    }
}

impl MixedContentAnalyzer {
    /// Find HTTP resource references in HTML body.
    fn find_http_resources(body: &str) -> Vec<String> {
        let mut resources = Vec::new();
        let patterns = [
            "src=\"http://",
            "src='http://",
            "href=\"http://",
            "href='http://",
            "action=\"http://",
            "action='http://",
        ];
        for pattern in &patterns {
            let mut remaining = body;
            while let Some(pos) = remaining.find(pattern) {
                let start = pos + pattern.len();
                let Some(quote_char) = pattern.chars().last() else {
                    break;
                };
                if let Some(end) = remaining[start..].find(quote_char) {
                    let url = &remaining[start..start + end];
                    // Skip data: and javascript: URIs
                    if !url.starts_with("data:") && !url.starts_with("javascript:") {
                        resources.push(format!("http://{url}"));
                    }
                    remaining = &remaining[start + end + 1..];
                } else {
                    break;
                }
            }
        }
        resources
    }

    /// Find HTTP form actions in HTML body.
    fn find_http_forms(body: &str) -> Vec<String> {
        let mut forms = Vec::new();
        let patterns = ["action=\"http://", "action='http://"];
        for pattern in &patterns {
            let mut remaining = body;
            while let Some(pos) = remaining.find(pattern) {
                let start = pos + pattern.len();
                let Some(quote_char) = pattern.chars().last() else {
                    break;
                };
                if let Some(end) = remaining[start..].find(quote_char) {
                    let url = &remaining[start..start + end];
                    forms.push(format!("http://{url}"));
                    remaining = &remaining[start + end + 1..];
                } else {
                    break;
                }
            }
        }
        forms
    }
}

// ---------------------------------------------------------------------------
// Permission Policy Analyzer
// ---------------------------------------------------------------------------

pub struct PermissionPolicyAnalyzer;

impl PermissionPolicyAnalyzer {
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

impl Default for PermissionPolicyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PermissionPolicyAnalyzer {
    fn name(&self) -> &str {
        "permission-policy"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Permissions-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "PERM001".to_string(),
                    title: "Missing Permissions-Policy header".to_string(),
                    description: "No Permissions-Policy header was found. This header controls \
                                  which browser features and APIs can be used. Without it, all \
                                  features are available by default."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set a Permissions-Policy header to restrict access to \
                                     sensitive features like camera, microphone, and geolocation."
                        .into(),
                });
            }
            Some(policy) => {
                let lower = policy.to_lowercase();
                for feature in &["camera", "microphone"] {
                    // If the feature is mentioned but not restricted to ()
                    if lower.contains(feature)
                        && !lower.contains(&format!("{feature}=()"))
                        && !lower.contains(&format!("{feature}=(self)"))
                    {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Security,
                            code: "PERM002".to_string(),
                            title: format!("Permissions-Policy: {feature} not restricted"),
                            description: format!(
                                "The {feature} feature in Permissions-Policy is not explicitly \
                                 restricted. Allowing {feature} access increases the attack \
                                 surface for microphone/camera-based attacks."
                            ),
                            url: url.to_string(),
                            recommendation: format!(
                                "Add {feature}=() to Permissions-Policy to disable it if not \
                                 needed, or {feature}=(self) to restrict to same-origin."
                            ),
                        });
                    }
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Cross-Origin Isolation Analyzer
// ---------------------------------------------------------------------------

pub struct CrossOriginIsolationAnalyzer;

impl CrossOriginIsolationAnalyzer {
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

impl Default for CrossOriginIsolationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CrossOriginIsolationAnalyzer {
    fn name(&self) -> &str {
        "cross-origin-isolation"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if Self::get_header(ctx.headers, "Cross-Origin-Embedder-Policy").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COEP001".to_string(),
                title: "Missing Cross-Origin-Embedder-Policy header".to_string(),
                description: "No Cross-Origin-Embedder-Policy (COEP) header was found. COEP \
                              prevents loading cross-origin resources without explicit opt-in, \
                              enabling cross-origin isolation for high-precision timing \
                              mitigation (Spectre)."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Set Cross-Origin-Embedder-Policy: require-corp to enable \
                                 cross-origin isolation."
                    .into(),
            });
        }

        if Self::get_header(ctx.headers, "Cross-Origin-Opener-Policy").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COOP002".to_string(),
                title: "Missing Cross-Origin-Opener-Policy header".to_string(),
                description: "No Cross-Origin-Opener-Policy (COOP) header was found. COOP \
                              isolates your browsing context from cross-origin popups, preventing \
                              cross-origin window references that could be exploited."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Set Cross-Origin-Opener-Policy: same-origin to isolate your \
                                 browsing context."
                    .into(),
            });
        }

        findings
    }
}
