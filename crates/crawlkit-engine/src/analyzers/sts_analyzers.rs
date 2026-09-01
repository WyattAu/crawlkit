//! Transport and isolation policy analyzers: Strict-Transport-Security,
//! XSS-Protection, Content-Type sniffing, Permissions-Policy, COEP, COOP,
//! Feature-Policy, Expect-CT, and Certificate-Transparency checks.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 architecture
//! simplification step (SRP: one concern per module). No public API change —
//! the types are re-exported from `analyzers::mod` identically.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// StrictTransportSecurityAnalyzer
// =========================================================================

/// Analyzes Strict-Transport-Security header for proper HSTS configuration.
pub struct StrictTransportSecurityAnalyzer;

impl Default for StrictTransportSecurityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl StrictTransportSecurityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Parse max-age from an HSTS header value.
    fn parse_max_age(value: &str) -> Option<u64> {
        for part in value.split(';') {
            let trimmed = part.trim();
            if let Some(val) = trimmed.strip_prefix("max-age=") {
                if let Ok(n) = val.trim().parse::<u64>() {
                    return Some(n);
                }
            }
        }
        None
    }
}

impl Analyzer for StrictTransportSecurityAnalyzer {
    fn name(&self) -> &str {
        "strict-transport-security"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Strict-Transport-Security") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "STRICT001".to_string(),
                    title: "Missing Strict-Transport-Security header".to_string(),
                    description: "No Strict-Transport-Security header was found. HSTS tells \
                                  browsers to only use HTTPS for this domain."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set Strict-Transport-Security: max-age=31536000; \
                                     includeSubDomains; preload."
                        .to_string(),
                });
            }
            Some(value) => {
                if let Some(max_age) = Self::parse_max_age(value) {
                    if max_age < 31536000 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Security,
                            code: "STRICT002".to_string(),
                            title: "HSTS max-age is too short".to_string(),
                            description: format!(
                                "Strict-Transport-Security max-age is {max_age} seconds, \
                                 which is less than the recommended minimum of 31536000 \
                                 (1 year)."
                            ),
                            url: url.to_string(),
                            recommendation: "Set max-age to at least 31536000 (1 year)."
                                .to_string(),
                        });
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// XSSProtectionAnalyzer
// =========================================================================

/// Analyzes X-XSS-Protection header configuration.
pub struct XSSProtectionAnalyzer;

impl Default for XSSProtectionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl XSSProtectionAnalyzer {
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

impl Analyzer for XSSProtectionAnalyzer {
    fn name(&self) -> &str {
        "xss-protection"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "X-XSS-Protection") {
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "XSS001".to_string(),
                    title: "Missing X-XSS-Protection header".to_string(),
                    description: "No X-XSS-Protection header was found. While modern browsers \
                                  rely on CSP, this header provides legacy XSS protection."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set X-XSS-Protection: 1; mode=block for legacy browser \
                                     support."
                        .to_string(),
                });
            }
            Some(value) => {
                if value.trim().contains("mode=block") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "XSS002".to_string(),
                        title: "X-XSS-Protection set to mode=block".to_string(),
                        description: "X-XSS-Protection is set to mode=block. While this enables \
                                      the XSS auditor in mode=block, the header is deprecated \
                                      and Content-Security-Policy is preferred."
                            .to_string(),
                        url: url.to_string(),
                        recommendation: "Consider removing X-XSS-Protection and relying on \
                                         Content-Security-Policy instead."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// ContentTypeSniffingAnalyzer
// =========================================================================

/// Analyzes X-Content-Type-Options for MIME type sniffing protection.
pub struct ContentTypeSniffingAnalyzer;

impl Default for ContentTypeSniffingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentTypeSniffingAnalyzer {
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

impl Analyzer for ContentTypeSniffingAnalyzer {
    fn name(&self) -> &str {
        "content-type-sniffing"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "X-Content-Type-Options") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "CTSNIFF001".to_string(),
                    title: "Missing X-Content-Type-Options header".to_string(),
                    description: "No X-Content-Type-Options header was found. This header \
                                  prevents browsers from MIME-sniffing a response away from \
                                  the declared content type."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set X-Content-Type-Options: nosniff.".to_string(),
                });
            }
            Some(value) => {
                if !value.trim().eq_ignore_ascii_case("nosniff") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CTSNIFF002".to_string(),
                        title: "X-Content-Type-Options not set to nosniff".to_string(),
                        description: format!(
                            "X-Content-Type-Options is \"{value}\" but should be \"nosniff\". \
                             Only the nosniff value is recognized by browsers."
                        ),
                        url: url.to_string(),
                        recommendation: "Set X-Content-Type-Options: nosniff.".to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Permissions Policy Analyzer (PPERM codes)
// ---------------------------------------------------------------------------

pub struct PermissionsPolicyAnalyzerNew;

impl PermissionsPolicyAnalyzerNew {
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

impl Default for PermissionsPolicyAnalyzerNew {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PermissionsPolicyAnalyzerNew {
    fn name(&self) -> &str {
        "permissions-policy-check"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Permissions-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "PPERM001".to_string(),
                    title: "Permissions-Policy header missing".to_string(),
                    description: "No Permissions-Policy header was found. This header controls \
                                  which browser features and APIs can be used by the page. \
                                  Without it, all features are available by default."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set a Permissions-Policy header to restrict access to \
                                     sensitive features like camera, microphone, and geolocation."
                        .into(),
                });
            }
            Some(policy) => {
                let lower = policy.to_lowercase();
                if lower.contains("camera")
                    && !lower.contains("camera=()")
                    && !lower.contains("camera=(self)")
                {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "PPERM002".to_string(),
                        title: "Permissions-Policy allows camera by default".to_string(),
                        description: "The Permissions-Policy header does not explicitly restrict \
                                      camera access. Allowing camera access increases the attack \
                                      surface for camera-based attacks."
                            .to_string(),
                        url: url.to_string(),
                        recommendation: "Add camera=() to Permissions-Policy to disable camera \
                                         access, or camera=(self) to restrict to same-origin."
                            .into(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Cross-Origin Embedder Policy Analyzer (COEP codes)
// ---------------------------------------------------------------------------

pub struct CrossOriginEmbedderPolicyAnalyzer;

impl CrossOriginEmbedderPolicyAnalyzer {
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

impl Default for CrossOriginEmbedderPolicyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CrossOriginEmbedderPolicyAnalyzer {
    fn name(&self) -> &str {
        "cross-origin-embedder-policy"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Cross-Origin-Embedder-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "COEP001-POLICY".to_string(),
                    title: "Cross-Origin-Embedder-Policy header missing".to_string(),
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
            Some(value) => {
                if value.trim() != "require-corp" {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "COEP002".to_string(),
                        title: "Cross-Origin-Embedder-Policy not set to require-corp".to_string(),
                        description: format!(
                            "Cross-Origin-Embedder-Policy is \"{value}\" instead of \
                             \"require-corp\". The require-corp value is needed for full \
                             cross-origin isolation."
                        ),
                        url: url.to_string(),
                        recommendation: "Set Cross-Origin-Embedder-Policy: require-corp for \
                                         strictest cross-origin isolation."
                            .into(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Cross-Origin Opener Policy Analyzer (COOP codes)
// ---------------------------------------------------------------------------

pub struct CrossOriginOpenerPolicyAnalyzer;

impl CrossOriginOpenerPolicyAnalyzer {
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

impl Default for CrossOriginOpenerPolicyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CrossOriginOpenerPolicyAnalyzer {
    fn name(&self) -> &str {
        "cross-origin-opener-policy"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Cross-Origin-Opener-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "COOP001".to_string(),
                    title: "Cross-Origin-Opener-Policy header missing".to_string(),
                    description: "No Cross-Origin-Opener-Policy (COOP) header was found. COOP \
                                  isolates your browsing context from cross-origin popups, \
                                  preventing cross-origin window references that could be \
                                  exploited."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set Cross-Origin-Opener-Policy: same-origin to isolate your \
                                     browsing context."
                        .into(),
                });
            }
            Some(value) => {
                if value.trim() != "same-origin" {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "COOP002-POLICY".to_string(),
                        title: "Cross-Origin-Opener-Policy not set to same-origin".to_string(),
                        description: format!(
                            "Cross-Origin-Opener-Policy is \"{value}\" instead of \"same-origin\". \
                             The same-origin value provides the strictest isolation."
                        ),
                        url: url.to_string(),
                        recommendation:
                            "Set Cross-Origin-Opener-Policy: same-origin for strictest \
                                         cross-origin isolation."
                                .into(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================

pub struct FeaturePolicyAnalyzer;
impl Default for FeaturePolicyAnalyzer {
    fn default() -> Self {
        Self
    }
}
impl FeaturePolicyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FeaturePolicyAnalyzer {
    fn name(&self) -> &str {
        "feature-policy"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_feature_policy = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("feature-policy"));
        let has_permissions_policy = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("permissions-policy"));
        if !has_feature_policy && !has_permissions_policy {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "FP001".to_string(),
                title: "No Feature-Policy or Permissions-Policy header".to_string(),
                description: "Neither Feature-Policy nor Permissions-Policy header is set."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add a Permissions-Policy header to control browser features."
                    .to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// ExpectCTAnalyzer
// =========================================================================

pub struct ExpectCTAnalyzer;
impl Default for ExpectCTAnalyzer {
    fn default() -> Self {
        Self
    }
}
impl ExpectCTAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ExpectCTAnalyzer {
    fn name(&self) -> &str {
        "expect-ct"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_expect_ct = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("expect-ct"));
        if !has_expect_ct && ctx.status_code == Some(200) {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "ECT001".to_string(), title: "No Expect-CT header".to_string(), description: "Expect-CT header is not set. Consider adding for Certificate Transparency enforcement.".to_string(), url: url.clone(), recommendation: "Add Expect-CT header with enforce and max-age directives.".to_string() });
        }
        findings
    }
}

// =========================================================================
// CertificateTransparencyAnalyzer
// =========================================================================

pub struct CertificateTransparencyAnalyzer;
impl Default for CertificateTransparencyAnalyzer {
    fn default() -> Self {
        Self
    }
}
impl CertificateTransparencyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CertificateTransparencyAnalyzer {
    fn name(&self) -> &str {
        "certificate-transparency"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_sct = ctx
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("expect-ct") && v.contains("enforce"));
        if !has_sct && ctx.status_code == Some(200) {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "CT001".to_string(),
                title: "No Certificate Transparency enforcement".to_string(),
                description: "Expect-CT header with enforce directive is not set.".to_string(),
                url: url.clone(),
                recommendation: "Add Expect-CT: enforce, max-age=31536000 for CT compliance."
                    .to_string(),
            });
        }
        findings
    }
}

// =========================================================================

pub struct CspDirectiveAnalyzer;

impl Default for CspDirectiveAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CspDirectiveAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CspDirectiveAnalyzer {
    fn name(&self) -> &str {
        "csp-directive-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str());
        let csp = match csp {
            Some(v) if !v.is_empty() => v,
            _ => return findings,
        };

        let directives: Vec<&str> = csp
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let directive_names: Vec<&str> = directives
            .iter()
            .map(|d| d.split_whitespace().next().unwrap_or(""))
            .collect();

        if !directive_names.contains(&"default-src") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPDIR001".to_string(),
                title: "CSP missing default-src".to_string(),
                description: "Content-Security-Policy lacks a default-src directive as fallback."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add default-src 'self' as a baseline CSP directive.".to_string(),
            });
        }

        for &dir in &["script-src", "style-src", "img-src"] {
            if let Some(directive) = directives.iter().find(|d| d.starts_with(dir)) {
                if directive.contains("'unsafe-inline'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPDIR002".to_string(),
                        title: format!("CSP {dir} allows unsafe-inline"),
                        description: format!(
                            "{dir} includes 'unsafe-inline', weakening XSS protection."
                        ),
                        url: url.clone(),
                        recommendation: format!(
                            "Remove 'unsafe-inline' from {dir} and use nonces or hashes."
                        ),
                    });
                }
                if directive.contains("'unsafe-eval'") {
                    findings.push(Finding {
                        severity: Severity::Critical,
                        category: IssueCategory::Security,
                        code: "CSPDIR003".to_string(),
                        title: format!("CSP {dir} allows unsafe-eval"),
                        description: format!(
                            "{dir} includes 'unsafe-eval', allowing arbitrary code execution."
                        ),
                        url: url.clone(),
                        recommendation: format!("Remove 'unsafe-eval' from {dir}."),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// CorsPolicyAnalyzer
// =========================================================================

pub struct CorsPolicyAnalyzer;

impl Default for CorsPolicyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CorsPolicyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CorsPolicyAnalyzer {
    fn name(&self) -> &str {
        "cors-policy-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let acao = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin"))
            .map(|(_, v)| v.as_str());
        let acac = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Credentials"))
            .map(|(_, v)| v.as_str());

        if let Some(origin) = acao {
            if origin == "*"
                && acac
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false)
            {
                findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Security, code: "CORS001".to_string(), title: "CORS wildcard with credentials".to_string(), description: "Access-Control-Allow-Origin is '*' with Allow-Credentials: true, which browsers reject but indicates misconfiguration.".to_string(), url: url.clone(), recommendation: "Use a specific origin instead of '*' when credentials are allowed.".to_string() });
            }
            if origin == "*" {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "CORS002".to_string(),
                    title: "CORS allows all origins".to_string(),
                    description:
                        "Access-Control-Allow-Origin is '*', allowing any website to make requests."
                            .to_string(),
                    url: url.clone(),
                    recommendation:
                        "Restrict to specific trusted origins if the resource is sensitive."
                            .to_string(),
                });
            }
        }
        findings
    }
}
