//! Mixed-content form, script, and image validators.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. Public names and behavior are preserved through re-exports.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// MixedContentFormValidator
// =========================================================================

pub struct MixedContentFormValidator;
impl Default for MixedContentFormValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentFormValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MixedContentFormValidator {
    fn name(&self) -> &str {
        "mixed-content-form"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let prefix_dq = "action=\"http://";
            let prefix_sq = "action='http://";
            for (prefix, quote) in [(prefix_dq, '"'), (prefix_sq, '\'')] {
                let mut remaining = body;
                while let Some(pos) = remaining.find(prefix) {
                    let start = pos + prefix.len();
                    if let Some(end) = remaining[start..].find(quote) {
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Security,
                            code: "MIXFRM001".to_string(),
                            title: "HTTP form action on HTTPS page".to_string(),
                            description: "A form has an action attribute using HTTP on an HTTPS page. User data will be transmitted in plaintext.".to_string(),
                            url: url.to_string(),
                            recommendation: "Change the form action URL to HTTPS.".to_string(),
                        });
                        remaining = &remaining[start + end + 1..];
                    } else {
                        break;
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// MixedContentScriptValidator
// =========================================================================

pub struct MixedContentScriptValidator;
impl Default for MixedContentScriptValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentScriptValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MixedContentScriptValidator {
    fn name(&self) -> &str {
        "mixed-content-script"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let mut count = 0;
            for (prefix, quote) in [("src=\"http://", '"'), ("src='http://", '\'')] {
                let mut remaining = body;
                while let Some(pos) = remaining.find(prefix) {
                    let start = pos + prefix.len();
                    if let Some(end) = remaining[start..].find(quote) {
                        let res = remaining[start..start + end].to_lowercase();
                        if res.ends_with(".js") || res.contains("/script") {
                            count += 1;
                        }
                        remaining = &remaining[start + end + 1..];
                    } else {
                        break;
                    }
                }
            }
            if count > 0 {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Security,
                    code: "MIXSCR001".to_string(),
                    title: "HTTP script sources on HTTPS page".to_string(),
                    description: format!("{count} script(s) loaded over HTTP on an HTTPS page. Browsers may block mixed active content."),
                    url: url.to_string(),
                    recommendation: "Update script URLs to use HTTPS.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// MixedContentImageValidator
// =========================================================================

pub struct MixedContentImageValidator;
impl Default for MixedContentImageValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentImageValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MixedContentImageValidator {
    fn name(&self) -> &str {
        "mixed-content-image"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let img_exts = [
                ".jpg", ".jpeg", ".png", ".gif", ".svg", ".webp", ".ico", ".bmp", ".tiff",
            ];
            let mut count = 0;
            for (prefix, quote) in [("src=\"http://", '"'), ("src='http://", '\'')] {
                let mut remaining = body;
                while let Some(pos) = remaining.find(prefix) {
                    let start = pos + prefix.len();
                    if let Some(end) = remaining[start..].find(quote) {
                        let res = remaining[start..start + end].to_lowercase();
                        if img_exts.iter().any(|ext| res.ends_with(ext)) {
                            count += 1;
                        }
                        remaining = &remaining[start + end + 1..];
                    } else {
                        break;
                    }
                }
            }
            if count > 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXIMG001".to_string(),
                    title: "HTTP image sources on HTTPS page".to_string(),
                    description: format!("{count} image(s) loaded over HTTP on an HTTPS page. Mixed passive content degrades HTTPS security."),
                    url: url.to_string(),
                    recommendation: "Update image URLs to use HTTPS.".to_string(),
                });
            }
        }
        findings
    }
}
