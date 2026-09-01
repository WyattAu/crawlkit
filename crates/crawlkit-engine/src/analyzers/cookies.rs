//! Cookie analyzers: Set-Cookie flag checks.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 architecture
//! simplification step (SRP: one concern per module). No public API change —
//! the types are re-exported from `analyzers::mod` identically.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------
// Cookie Analyzer
// ---------------------------------------------------------------------------

pub struct CookieAnalyzer;

impl CookieAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CookieAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CookieAnalyzer {
    fn name(&self) -> &str {
        "cookies"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Only check HTTPS pages
        if !url.starts_with("https://") {
            return findings;
        }

        let set_cookie_headers: Vec<&str> = ctx
            .headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("Set-Cookie"))
            .map(|(_, v)| v.as_str())
            .collect();

        if set_cookie_headers.is_empty() {
            return findings;
        }

        for cookie_header in &set_cookie_headers {
            let lower = cookie_header.to_lowercase();
            let cookie_name = cookie_header.split('=').next().unwrap_or("unknown").trim();

            // COOKIE001: Missing Secure flag
            if !lower.contains("secure") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIE001".to_string(),
                    title: "Cookie missing Secure flag".to_string(),
                    description: format!(
                        "Cookie \"{cookie_name}\" does not have the Secure flag. Without it, \
                         the cookie will be sent over unencrypted HTTP connections."
                    ),
                    url: url.clone(),
                    recommendation: "Add the Secure flag to the Set-Cookie header to ensure \
                                     the cookie is only sent over HTTPS."
                        .to_string(),
                });
            }

            // COOKIE002: Missing HttpOnly flag
            if !lower.contains("httponly") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIE002".to_string(),
                    title: "Cookie missing HttpOnly flag".to_string(),
                    description: format!(
                        "Cookie \"{cookie_name}\" does not have the HttpOnly flag. Without \
                         it, the cookie is accessible to JavaScript, increasing the risk \
                         of XSS attacks."
                    ),
                    url: url.clone(),
                    recommendation: "Add the HttpOnly flag to the Set-Cookie header to \
                                     prevent JavaScript access to the cookie."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// CookieSecurityFlagAnalyzer
// =========================================================================

pub struct CookieSecurityFlagAnalyzer;

impl Default for CookieSecurityFlagAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CookieSecurityFlagAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CookieSecurityFlagAnalyzer {
    fn name(&self) -> &str {
        "cookie-security-flag-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let cookie_name = v.split('=').next().unwrap_or("cookie").to_string();

            if !lower.contains("secure") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIESEC001".to_string(), title: format!("Cookie '{cookie_name}' missing Secure flag"), description: "A Set-Cookie header lacks the Secure flag, allowing transmission over HTTP.".to_string(), url: url.clone(), recommendation: "Add the Secure flag to all cookies.".to_string() });
            }
            if !lower.contains("httponly") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIESEC002".to_string(), title: format!("Cookie '{cookie_name}' missing HttpOnly flag"), description: "A Set-Cookie header lacks the HttpOnly flag, making it accessible to JavaScript.".to_string(), url: url.clone(), recommendation: "Add the HttpOnly flag to prevent XSS cookie theft.".to_string() });
            }
            if !lower.contains("samesite") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "COOKIESEC003".to_string(),
                    title: format!("Cookie '{cookie_name}' missing SameSite attribute"),
                    description: "A Set-Cookie header lacks the SameSite attribute.".to_string(),
                    url: url.clone(),
                    recommendation: "Add SameSite=Strict or SameSite=Lax.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// MixedContentDetectionAnalyzer
// =========================================================================

// =========================================================================
// CookieSecureFlagValidator — COOKIESEC001
// =========================================================================

pub struct CookieSecureFlagValidator;
impl Default for CookieSecureFlagValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSecureFlagValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CookieSecureFlagValidator {
    fn name(&self) -> &str {
        "cookie-secure-flag"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        for (k, v) in ctx.headers {
            if k.eq_ignore_ascii_case("Set-Cookie") && !v.to_lowercase().contains("secure") {
                let name = v.split('=').next().unwrap_or("unknown").trim();
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIESEC001-VALIDATOR".to_string(),
                    title: "Cookie missing Secure flag".to_string(),
                    description: format!("Cookie \"{name}\" does not have the Secure flag. It will be sent over unencrypted HTTP connections."),
                    url: url.to_string(),
                    recommendation: "Add the Secure flag to the Set-Cookie header.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// CookieHttpOnlyFlagValidator — COOKIEHTTP001
// =========================================================================

pub struct CookieHttpOnlyFlagValidator;
impl Default for CookieHttpOnlyFlagValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieHttpOnlyFlagValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CookieHttpOnlyFlagValidator {
    fn name(&self) -> &str {
        "cookie-httponly-flag"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if k.eq_ignore_ascii_case("Set-Cookie") && !v.to_lowercase().contains("httponly") {
                let name = v.split('=').next().unwrap_or("unknown").trim();
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIEHTTP001".to_string(),
                    title: "Cookie missing HttpOnly flag".to_string(),
                    description: format!("Cookie \"{name}\" does not have the HttpOnly flag. It is accessible to JavaScript, increasing XSS risk."),
                    url: url.to_string(),
                    recommendation: "Add the HttpOnly flag to the Set-Cookie header.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// MixedContentFormValidator — MIXFRM001
