//! Remaining versioned security-header analyzers.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. Public names and behavior are preserved through re-exports.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------
// Security: Referrer-Policy V2 — missing header
// ---------------------------------------------------------------------------

pub struct ReferrerPolicyAnalyzerV2;
impl Default for ReferrerPolicyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ReferrerPolicyAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ReferrerPolicyAnalyzerV2 {
    fn name(&self) -> &str {
        "referrer-policy-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy"));
        if !has {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "REF-V2001".to_string(),
                title: "Missing Referrer-Policy header".to_string(),
                description: "No Referrer-Policy header was found. This header controls how much referrer information is sent with requests.".into(),
                url: url.clone(),
                recommendation: "Add Referrer-Policy: strict-origin-when-cross-origin or no-referrer.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: X-Frame-Options V2 — missing header
// ---------------------------------------------------------------------------

pub struct XFrameOptionsAnalyzerV2;
impl Default for XFrameOptionsAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl XFrameOptionsAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for XFrameOptionsAnalyzerV2 {
    fn name(&self) -> &str {
        "x-frame-options-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"));
        if !has {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "XFO-V2001".to_string(),
                title: "Missing X-Frame-Options header".to_string(),
                description: "No X-Frame-Options header was found. This header prevents clickjacking by controlling frame embedding.".into(),
                url: url.clone(),
                recommendation: "Set X-Frame-Options to DENY or SAMEORIGIN.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: CSP V2 — missing script-src
// ---------------------------------------------------------------------------

pub struct ContentSecurityPolicyAnalyzerV2;
impl Default for ContentSecurityPolicyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ContentSecurityPolicyAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ContentSecurityPolicyAnalyzerV2 {
    fn name(&self) -> &str {
        "content-security-policy-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str());
        match csp {
            None => findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSP-V2001".to_string(),
                title: "CSP missing script-src directive".to_string(),
                description: "No Content-Security-Policy header with script-src was found. CSP script-src helps prevent XSS attacks.".into(),
                url: url.clone(),
                recommendation: "Add Content-Security-Policy with a script-src directive (e.g., script-src 'self').".into(),
            }),
            Some(val) if !val.contains("script-src") => findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSP-V2001".to_string(),
                title: "CSP missing script-src directive".to_string(),
                description: "Content-Security-Policy header is present but does not include a script-src directive.".into(),
                url: url.clone(),
                recommendation: "Add script-src directive to the Content-Security-Policy header.".into(),
            }),
            Some(_) => {}
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: HSTS V3 — missing includeSubDomains
// ---------------------------------------------------------------------------

pub struct StrictTransportSecurityAnalyzerV3;
impl Default for StrictTransportSecurityAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}
impl StrictTransportSecurityAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for StrictTransportSecurityAnalyzerV3 {
    fn name(&self) -> &str {
        "strict-transport-security-v3"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hsts = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str());
        match hsts {
            None => findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "HSTS-V3001".to_string(),
                title: "HSTS missing includeSubDomains".to_string(),
                description: "No Strict-Transport-Security header with includeSubDomains was found.".into(),
                url: url.clone(),
                recommendation: "Add Strict-Transport-Security: max-age=31536000; includeSubDomains; preload.".into(),
            }),
            Some(val) if !val.to_lowercase().contains("includesubdomains") => findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "HSTS-V3001".to_string(),
                title: "HSTS missing includeSubDomains".to_string(),
                description: "The Strict-Transport-Security header does not include the includeSubDomains directive.".into(),
                url: url.clone(),
                recommendation: "Add includeSubDomains to protect all subdomains.".into(),
            }),
            Some(_) => {}
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: X-Content-Type-Options V2 — missing header
// ---------------------------------------------------------------------------

pub struct XContentTypeOptionsAnalyzerV2;
impl Default for XContentTypeOptionsAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl XContentTypeOptionsAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for XContentTypeOptionsAnalyzerV2 {
    fn name(&self) -> &str {
        "x-content-type-options-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("X-Content-Type-Options"));
        if !has {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "XCTO-V2001".to_string(),
                title: "Missing X-Content-Type-Options header".to_string(),
                description: "No X-Content-Type-Options header was found. This header prevents MIME-type sniffing.".into(),
                url: url.clone(),
                recommendation: "Set X-Content-Type-Options to nosniff.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: Permissions-Policy V3 — missing camera
// ---------------------------------------------------------------------------

pub struct PermissionsPolicyAnalyzerV3;
impl Default for PermissionsPolicyAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyAnalyzerV3 {
    fn name(&self) -> &str {
        "permissions-policy-v3"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy"))
            .map(|(_, v)| v.as_str());
        match pp {
            None => findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "PERM-V3001".to_string(),
                title: "Permissions-Policy missing camera restriction".to_string(),
                description: "No Permissions-Policy header was found. Without it, the camera API may be accessible by default.".into(),
                url: url.clone(),
                recommendation: "Add Permissions-Policy header with camera=() to disable camera access if not needed.".into(),
            }),
            Some(val) if !val.to_lowercase().contains("camera=()") => findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "PERM-V3001".to_string(),
                title: "Permissions-Policy missing camera restriction".to_string(),
                description: "The Permissions-Policy header does not explicitly restrict camera access.".into(),
                url: url.clone(),
                recommendation: "Add camera=() to Permissions-Policy to disable camera access if not needed.".into(),
            }),
            Some(_) => {}
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: COEP V2 — missing header
// ---------------------------------------------------------------------------

pub struct CrossOriginIsolationAnalyzerV2;
impl Default for CrossOriginIsolationAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CrossOriginIsolationAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CrossOriginIsolationAnalyzerV2 {
    fn name(&self) -> &str {
        "cross-origin-isolation-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy"));
        if !has {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COEP-V2001".to_string(),
                title: "Missing Cross-Origin-Embedder-Policy header".to_string(),
                description: "No Cross-Origin-Embedder-Policy header was found. COEP prevents resources from loading cross-origin without explicit permission.".into(),
                url: url.clone(),
                recommendation: "Set Cross-Origin-Embedder-Policy to require-corp for stricter cross-origin isolation.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: COOP V2 — missing header
// ---------------------------------------------------------------------------

pub struct CrossOriginOpenerPolicyAnalyzerV2;
impl Default for CrossOriginOpenerPolicyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CrossOriginOpenerPolicyAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CrossOriginOpenerPolicyAnalyzerV2 {
    fn name(&self) -> &str {
        "cross-origin-opener-policy-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy"));
        if !has {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COOP-V2001".to_string(),
                title: "Missing Cross-Origin-Opener-Policy header".to_string(),
                description: "No Cross-Origin-Opener-Policy header was found. COOP isolates your browsing context from cross-origin popups.".into(),
                url: url.clone(),
                recommendation: "Set Cross-Origin-Opener-Policy to same-origin for stricter isolation.".into(),
            });
        }
        findings
    }
}
