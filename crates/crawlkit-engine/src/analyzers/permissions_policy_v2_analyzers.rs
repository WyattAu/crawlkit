//! Permissions-Policy V2 security analyzer.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. The public analyzer name and behavior are preserved by
//! re-exports from `analyzers::mod` and `security_analyzers`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// PermissionsPolicyAnalyzerV2
// =========================================================================

pub struct PermissionsPolicyAnalyzerV2;

impl Default for PermissionsPolicyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionsPolicyAnalyzerV2 {
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

impl Analyzer for PermissionsPolicyAnalyzerV2 {
    fn name(&self) -> &str {
        "permissions-policy-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Permissions-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "PERM-V2001".to_string(),
                    title: "Permissions-Policy header missing".to_string(),
                    description: "No Permissions-Policy header was found.".to_string(),
                    url: url.to_string(),
                    recommendation: "Set a Permissions-Policy header.".into(),
                });
            }
            Some(policy) => {
                let lower = policy.to_lowercase();
                for feature in &["camera", "microphone", "geolocation", "payment"] {
                    if !lower.contains(&format!("{feature}=()"))
                        && !lower.contains(&format!("{feature}=(self)"))
                    {
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Security,
                            code: "PERM-V2002".to_string(),
                            title: format!("Permissions-Policy does not restrict {feature}"),
                            description: format!(
                                "The Permissions-Policy header does not explicitly restrict \
                                 {feature} access."
                            ),
                            url: url.to_string(),
                            recommendation: format!("Add {feature}=() to Permissions-Policy."),
                        });
                    }
                }
            }
        }

        findings
    }
}
