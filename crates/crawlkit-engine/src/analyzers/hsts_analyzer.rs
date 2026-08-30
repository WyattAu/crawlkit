//! HSTS preload readiness analyzer.
//!
//! Extracted from `security_analyzers.rs` as the first Phase 2 architecture
//! simplification step (SRP: one analyzer per file/module where the file has
//! grown past a maintainable size). No public API change — the type is
//! re-exported from `analyzers::mod` identically.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------

pub struct HstsPreloadAnalyzer;

impl HstsPreloadAnalyzer {
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

impl Default for HstsPreloadAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HstsPreloadAnalyzer {
    fn name(&self) -> &str {
        "hsts-preload"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let hsts_value = match Self::get_header(ctx.headers, "Strict-Transport-Security") {
            Some(v) => v,
            None => return findings,
        };

        let lower = hsts_value.to_lowercase();

        // Check max-age >= 31536000
        if let Some(ma_pos) = lower.find("max-age=") {
            let after = &lower[ma_pos + 8..];
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(age) = num_str.parse::<u64>() {
                if age < 31536000 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "HSTS003".to_string(),
                        title: "HSTS max-age below recommended minimum".to_string(),
                        description: format!(
                            "HSTS max-age is {age}, which is below the recommended minimum of \
                             31536000 (1 year). Short max-age values provide less protection \
                             against protocol downgrade attacks."
                        ),
                        url: url.to_string(),
                        recommendation: "Set max-age to at least 31536000 (1 year). Ideally use \
                                         63072000 (2 years) for preload list eligibility."
                            .into(),
                    });
                }
            }
        }

        // Check includeSubDomains
        if !lower.contains("includesubdomains") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "HSTS001".to_string(),
                title: "HSTS missing includeSubDomains directive".to_string(),
                description: "The Strict-Transport-Security header does not include the \
                              includeSubDomains directive. Without it, subdomains are not \
                              protected by HSTS and may be vulnerable to downgrade attacks."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add includeSubDomains to the HSTS header to extend protection \
                                 to all subdomains."
                    .into(),
            });
        }

        // Check preload
        if !lower.contains("preload") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "HSTS002".to_string(),
                title: "HSTS missing preload directive".to_string(),
                description: "The Strict-Transport-Security header does not include the preload \
                              directive. The preload directive signals intent to be included in \
                              browser HSTS preload lists."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add preload to the HSTS header and submit your domain to the \
                                 HSTS preload list at hstspreload.org."
                    .into(),
            });
        }

        findings
    }
}
