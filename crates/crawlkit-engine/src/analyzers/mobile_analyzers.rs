//! Mobile-friendliness analyzer for viewport configuration and zoom accessibility.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use std::collections::HashMap;

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks viewport configuration and zoom accessibility on mobile devices.
pub struct MobileFriendlinessChecker;

impl MobileFriendlinessChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse viewport meta content into a map of directives.
    pub(crate) fn parse_viewport(viewport: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for part in viewport.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                map.insert(key.trim().to_lowercase(), value.trim().to_lowercase());
            }
        }
        map
    }
}

impl Default for MobileFriendlinessChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MobileFriendlinessChecker {
    fn name(&self) -> &str {
        "mobile-friendliness"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // --- Viewport meta tag presence ---
        let viewport = match &ctx.page.meta.viewport {
            Some(v) => v,
            None => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Mobile,
                    code: "MOB001".to_string(),
                    title: "Missing viewport meta tag".to_string(),
                    description: "No viewport meta tag was found. Without it, the page will not \
                                  scale properly on mobile devices."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add <meta name=\"viewport\" content=\"width=device-width, \
                                     initial-scale=1\"> to the <head>."
                        .to_string(),
                });
                return findings;
            }
        };

        let directives = Self::parse_viewport(viewport);

        // --- width=device-width ---
        match directives.get("width") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Mobile,
                    code: "MOB002".to_string(),
                    title: "Viewport missing width directive".to_string(),
                    description: "The viewport meta tag does not specify width=device-width."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Set width=device-width in the viewport meta tag.".to_string(),
                });
            }
            Some(w) if w != "device-width" => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Mobile,
                    code: "MOB003".to_string(),
                    title: "Viewport width is not device-width".to_string(),
                    description: format!(
                        "Viewport width is set to \"{w}\" instead of device-width. This forces \
                         a fixed layout."
                    ),
                    url: url.clone(),
                    recommendation: "Change viewport width to device-width for responsive layout."
                        .to_string(),
                });
            }
            _ => {}
        }

        // --- user-scalable=no or maximum-scale=1.0 ---
        if directives.get("user-scalable") == Some(&"no".to_string()) {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Mobile,
                code: "MOB004".to_string(),
                title: "Zooming is disabled (user-scalable=no)".to_string(),
                description: "The viewport meta tag disables user zooming with \
                              user-scalable=no. This is a critical accessibility issue."
                    .to_string(),
                url: url.clone(),
                recommendation: "Remove user-scalable=no to allow pinch-to-zoom. This is \
                                 required for WCAG 2.1 compliance."
                    .to_string(),
            });
        }

        if let Some(max_scale) = directives.get("maximum-scale") {
            if max_scale == "1" || max_scale == "1.0" {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Mobile,
                    code: "MOB005".to_string(),
                    title: "Maximum scale restricted".to_string(),
                    description: format!(
                        "maximum-scale={max_scale} limits zooming. While not as severe as \
                         user-scalable=no, it can still hinder accessibility."
                    ),
                    url: url.clone(),
                    recommendation: "Remove the maximum-scale constraint or set it to at least \
                                     5.0."
                        .to_string(),
                });
            }
        }

        // --- initial-scale ---
        if let Some(scale) = directives.get("initial-scale") {
            if scale != "1" && scale != "1.0" {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Mobile,
                    code: "MOB009".to_string(),
                    title: "Non-standard initial scale".to_string(),
                    description: format!(
                        "initial-scale is set to \"{scale}\" instead of the standard 1.0. This \
                         may cause unexpected zoom behavior on page load."
                    ),
                    url: url.clone(),
                    recommendation: "Set initial-scale=1 for a consistent mobile experience."
                        .to_string(),
                });
            }
        }

        // MOB006-008 remain intentionally absent: their checks require CSS
        // layout computation or runtime testing that is not implemented.

        findings
    }
}
