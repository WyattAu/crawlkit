//! Mixed-content and HSTS preload-readiness analyzers.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. Public
//! analyzer names and behavior are preserved through `analyzers::mod`
//! re-exports.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// MixedContentDetectionAnalyzer
// =========================================================================

pub struct MixedContentDetectionAnalyzer;

impl Default for MixedContentDetectionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl MixedContentDetectionAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MixedContentDetectionAnalyzer {
    fn name(&self) -> &str {
        "mixed-content-detection"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_script = lower.contains("src=\"http://") && lower.contains("<script");
            let http_img = lower.contains("src=\"http://")
                && (lower.contains("<img") || lower.contains("background-image"));
            let http_link = lower.contains("href=\"http://") && lower.contains("<link");

            if http_script {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Security,
                    code: "MIXCONT001".to_string(),
                    title: "Mixed content: active script".to_string(),
                    description:
                        "HTTPS page loads scripts over HTTP, which can be intercepted and modified."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Change all script src attributes to use HTTPS.".to_string(),
                });
            }
            if http_img {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXCONT002".to_string(),
                    title: "Mixed content: passive image".to_string(),
                    description: "HTTPS page loads images over HTTP.".to_string(),
                    url: url.clone(),
                    recommendation: "Change image sources to use HTTPS.".to_string(),
                });
            }
            if http_link {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXCONT003".to_string(),
                    title: "Mixed content: stylesheet/resource".to_string(),
                    description: "HTTPS page loads stylesheets or other resources over HTTP."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Change resource URLs to use HTTPS.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// HstsPreloadReadinessAnalyzer
// =========================================================================

pub struct HstsPreloadReadinessAnalyzer;

impl Default for HstsPreloadReadinessAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl HstsPreloadReadinessAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HstsPreloadReadinessAnalyzer {
    fn name(&self) -> &str {
        "hsts-preload-readiness"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hsts = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str());
        let hsts = match hsts {
            Some(v) => v,
            None => return findings,
        };

        let lower = hsts.to_lowercase();
        if !lower.contains("includesubdomains") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "HSTSPR001".to_string(),
                title: "HSTS missing includeSubDomains for preload".to_string(),
                description:
                    "HSTS header lacks includeSubDomains, required for preload list submission."
                        .to_string(),
                url: url.clone(),
                recommendation: "Add includeSubDomains to the HSTS header.".to_string(),
            });
        }
        if !lower.contains("preload") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "HSTSPR002".to_string(),
                title: "HSTS missing preload directive".to_string(),
                description:
                    "HSTS header lacks the preload directive for browser preload list inclusion."
                        .to_string(),
                url: url.clone(),
                recommendation: "Add preload to the HSTS header.".to_string(),
            });
        }
        if let Some(ma_pos) = lower.find("max-age=") {
            let after = &lower[ma_pos + 8..];
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(age) = num_str.parse::<u64>() {
                if age < 31536000 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "HSTSPR003".to_string(),
                        title: "HSTS max-age too low for preload".to_string(),
                        description: format!(
                            "max-age is {age}, preload requires at least 31536000 (1 year)."
                        ),
                        url: url.clone(),
                        recommendation: "Set max-age to at least 31536000.".to_string(),
                    });
                }
            }
        }
        findings
    }
}
