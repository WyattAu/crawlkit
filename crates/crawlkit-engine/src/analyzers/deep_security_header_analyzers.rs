//! Deep security-header analyzers for X-Content-Type-Options, Referrer-Policy,
//! X-Frame-Options, Permissions-Policy, and cross-origin isolation.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. Public
//! analyzer names and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// XContentTypeOptionsDeepAnalyzer
// =========================================================================

pub struct XContentTypeOptionsDeepAnalyzer;

impl Default for XContentTypeOptionsDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl XContentTypeOptionsDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for XContentTypeOptionsDeepAnalyzer {
    fn name(&self) -> &str {
        "x-content-type-options-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let xcto = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-Content-Type-Options"))
            .map(|(_, v)| v.as_str());
        match xcto {
            None => {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XCTODEEP001".to_string(), title: "Missing X-Content-Type-Options header".to_string(), description: "No X-Content-Type-Options header found. Without nosniff, browsers may MIME-sniff responses.".to_string(), url: url.clone(), recommendation: "Add X-Content-Type-Options: nosniff.".to_string() });
            }
            Some(val) if !val.eq_ignore_ascii_case("nosniff") => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XCTODEEP002".to_string(),
                    title: "Invalid X-Content-Type-Options value".to_string(),
                    description: format!(
                        "X-Content-Type-Options is \"{val}\" but should be \"nosniff\"."
                    ),
                    url: url.clone(),
                    recommendation: "Set X-Content-Type-Options to nosniff.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

// =========================================================================
// ReferrerPolicyDeepAnalyzer
// =========================================================================

pub struct ReferrerPolicyDeepAnalyzer;

impl Default for ReferrerPolicyDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferrerPolicyDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ReferrerPolicyDeepAnalyzer {
    fn name(&self) -> &str {
        "referrer-policy-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let rp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy"))
            .map(|(_, v)| v.as_str());
        match rp {
            None => {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "RPDEEP001".to_string(), title: "Missing Referrer-Policy header".to_string(), description: "No Referrer-Policy header found. Browser default may leak referrer information.".to_string(), url: url.clone(), recommendation: "Add Referrer-Policy: strict-origin-when-cross-origin.".to_string() });
            }
            Some(val) if val.eq_ignore_ascii_case("unsafe-url") => {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "RPDEEP002".to_string(), title: "Referrer-Policy set to unsafe-url".to_string(), description: "Referrer-Policy is 'unsafe-url', leaking full URL including path and query on cross-origin requests.".to_string(), url: url.clone(), recommendation: "Use strict-origin-when-cross-origin instead of unsafe-url.".to_string() });
            }
            _ => {}
        }
        findings
    }
}

// =========================================================================
// XFrameOptionsDeepAnalyzer
// =========================================================================

pub struct XFrameOptionsDeepAnalyzer;

impl Default for XFrameOptionsDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl XFrameOptionsDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for XFrameOptionsDeepAnalyzer {
    fn name(&self) -> &str {
        "x-frame-options-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let xfo = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"))
            .map(|(_, v)| v.as_str());
        let csp_frame = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
            .contains("frame-ancestors");

        match xfo {
            None => {
                if !csp_frame {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "XFODEEP001".to_string(),
                        title: "No clickjacking protection".to_string(),
                        description: "Neither X-Frame-Options nor CSP frame-ancestors is set."
                            .to_string(),
                        url: url.clone(),
                        recommendation:
                            "Add X-Frame-Options: DENY or CSP frame-ancestors directive."
                                .to_string(),
                    });
                }
            }
            Some(val) => {
                let upper = val.to_uppercase().trim().to_string();
                if upper != "DENY" && upper != "SAMEORIGIN" {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "XFODEEP002".to_string(),
                        title: "Invalid X-Frame-Options value".to_string(),
                        description: format!(
                            "X-Frame-Options is \"{val}\", must be DENY or SAMEORIGIN."
                        ),
                        url: url.clone(),
                        recommendation: "Set X-Frame-Options to DENY or SAMEORIGIN.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

// =========================================================================
// PermissionsPolicyDeepAnalyzer
// =========================================================================

pub struct PermissionsPolicyDeepAnalyzer;

impl Default for PermissionsPolicyDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionsPolicyDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PermissionsPolicyDeepAnalyzer {
    fn name(&self) -> &str {
        "permissions-policy-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        match pp {
            None => {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PERMPDEEP001".to_string(), title: "Missing Permissions-Policy header".to_string(), description: "No Permissions-Policy header found. Browsers may allow access to sensitive APIs.".to_string(), url: url.clone(), recommendation: "Add Permissions-Policy to restrict sensitive browser features.".to_string() });
            }
            Some(val) => {
                let lower = val.to_lowercase();
                for feature in &["camera", "microphone", "geolocation", "payment"] {
                    if !lower.contains(feature) {
                        findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PERMPDEEP002".to_string(), title: format!("Permissions-Policy missing {feature}"), description: format!("The {feature} feature is not explicitly restricted in Permissions-Policy."), url: url.clone(), recommendation: format!("Add {feature}=() to restrict {feature} access.") });
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// CrossOriginIsolationDeepAnalyzer
// =========================================================================

pub struct CrossOriginIsolationDeepAnalyzer;

impl Default for CrossOriginIsolationDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossOriginIsolationDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CrossOriginIsolationDeepAnalyzer {
    fn name(&self) -> &str {
        "cross-origin-isolation-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let coep = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy"))
            .map(|(_, v)| v.as_str());
        let coop = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy"))
            .map(|(_, v)| v.as_str());

        if coep.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COISODEEP001".to_string(),
                title: "Missing Cross-Origin-Embedder-Policy".to_string(),
                description: "COEP header is not set, preventing cross-origin isolation."
                    .to_string(),
                url: url.clone(),
                recommendation:
                    "Add Cross-Origin-Embedder-Policy: require-corp for cross-origin isolation."
                        .to_string(),
            });
        }
        if coop.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COISODEEP002".to_string(),
                title: "Missing Cross-Origin-Opener-Policy".to_string(),
                description: "COOP header is not set.".to_string(),
                url: url.clone(),
                recommendation:
                    "Add Cross-Origin-Opener-Policy: same-origin for cross-origin isolation."
                        .to_string(),
            });
        }

        if let (Some(coep_val), Some(coop_val)) = (coep, coop) {
            if coep_val.eq_ignore_ascii_case("require-corp")
                && coop_val.eq_ignore_ascii_case("same-origin")
            {
                // Full cross-origin isolation achieved
            } else {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "COISODEEP003".to_string(), title: "Partial cross-origin isolation".to_string(), description: format!("COEP={coep_val}, COOP={coop_val}. Full isolation requires COEP=require-corp and COOP=same-origin."), url: url.clone(), recommendation: "Set COEP=require-corp and COOP=same-origin for full cross-origin isolation.".to_string() });
            }
        }

        findings
    }
}
