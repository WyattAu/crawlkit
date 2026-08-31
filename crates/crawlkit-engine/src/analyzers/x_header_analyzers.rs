//! X-header analyzers: X-Content-Type-Options, X-Permitted-Cross-Domain-Policies,
//! and Cross-Origin-Resource-Policy checks.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 architecture
//! simplification step (SRP: one concern per module). No public API change —
//! the types are re-exported from `analyzers::mod` identically.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// XContentTypeOptionsAnalyzer
// =========================================================================

/// Analyzes X-Content-Type-Options header for MIME sniffing protection.
pub struct XContentTypeOptionsAnalyzer;

impl Default for XContentTypeOptionsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl XContentTypeOptionsAnalyzer {
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

impl Analyzer for XContentTypeOptionsAnalyzer {
    fn name(&self) -> &str {
        "x-content-type-options"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "X-Content-Type-Options") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XCTO001".to_string(),
                    title: "Missing X-Content-Type-Options header".to_string(),
                    description: "No X-Content-Type-Options header was found. This header \
                                  prevents browsers from MIME-sniffing a response away from the \
                                  declared content type."
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
                        code: "XCTO002".to_string(),
                        title: "X-Content-Type-Options not set to nosniff".to_string(),
                        description: format!(
                            "X-Content-Type-Options is \"{value}\" but should be \"nosniff\". \
                             Other values are not recognized by browsers."
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

// =========================================================================
// XPermittedCrossDomainPoliciesAnalyzer
// =========================================================================

/// Analyzes X-Permitted-Cross-Domain-Policies header.
pub struct XPermittedCrossDomainPoliciesAnalyzer;

impl Default for XPermittedCrossDomainPoliciesAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl XPermittedCrossDomainPoliciesAnalyzer {
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

impl Analyzer for XPermittedCrossDomainPoliciesAnalyzer {
    fn name(&self) -> &str {
        "x-permitted-cross-domain-policies"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "X-Permitted-Cross-Domain-Policies") {
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "XPCDP001".to_string(),
                    title: "Missing X-Permitted-Cross-Domain-Policies header".to_string(),
                    description: "No X-Permitted-Cross-Domain-Policies header was found. This \
                                  header controls cross-domain policy files for Flash, PDF, and \
                                  other plugins."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set X-Permitted-Cross-Domain-Policies: none to prevent \
                                     cross-domain policy loading."
                        .to_string(),
                });
            }
            Some(value) => {
                if value.trim().eq_ignore_ascii_case("all") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "XPCDP002".to_string(),
                        title: "X-Permitted-Cross-Domain-Policies set to all".to_string(),
                        description: "The X-Permitted-Cross-Domain-Policies header is set to \
                                      \"all\", which allows any cross-domain policy file. This \
                                      weakens security by permitting cross-domain data access."
                            .to_string(),
                        url: url.to_string(),
                        recommendation: "Set X-Permitted-Cross-Domain-Policies: none to block \
                                         all cross-domain policy files."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// CrossOriginResourcePolicyAnalyzer
// =========================================================================

/// Analyzes Cross-Origin-Resource-Policy header.
pub struct CrossOriginResourcePolicyAnalyzer;

impl Default for CrossOriginResourcePolicyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossOriginResourcePolicyAnalyzer {
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

impl Analyzer for CrossOriginResourcePolicyAnalyzer {
    fn name(&self) -> &str {
        "cross-origin-resource-policy"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if Self::get_header(ctx.headers, "Cross-Origin-Resource-Policy").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "CORP001".to_string(),
                title: "Missing Cross-Origin-Resource-Policy header".to_string(),
                description: "No Cross-Origin-Resource-Policy header was found. CORP prevents \
                              cross-origin reads of embedded resources, providing protection \
                              against Spectre-like side-channel attacks."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Set Cross-Origin-Resource-Policy: same-origin if the resource \
                                 should only be used by the same origin, or cross-origin for \
                                 resources that need cross-origin access."
                    .to_string(),
            });
        }

        findings
    }
}
