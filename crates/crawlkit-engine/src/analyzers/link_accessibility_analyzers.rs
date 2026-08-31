//! Link accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks links for empty, generic, or otherwise non-descriptive text.
pub struct LinkAccessibilityAnalyzer;

impl LinkAccessibilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    const GENERIC_TEXTS: &[&str] = &[
        "click here",
        "read more",
        "more",
        "learn more",
        "click",
        "go",
        "continue",
    ];

    const NON_DESCRIPTIVE_TEXTS: &[&str] = &["link", "here"];
}

impl Default for LinkAccessibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LinkAccessibilityAnalyzer {
    fn name(&self) -> &str {
        "link-accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for link in &ctx.page.links {
            let text_lower = link.text.trim().to_lowercase();
            let has_accessible_name = !text_lower.is_empty()
                || link
                    .aria_label
                    .as_ref()
                    .is_some_and(|l| !l.trim().is_empty())
                || link.img_alt.as_ref().is_some_and(|a| !a.trim().is_empty());

            if !has_accessible_name {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Accessibility,
                    code: "LNKACC001".to_string(),
                    title: "Link with empty text content".to_string(),
                    description: format!(
                        "Link to \"{}\" has no accessible text. Screen readers announce the raw URL, \
                         which is not descriptive for users.",
                        link.href
                    ),
                    url: url.to_string(),
                    recommendation: "Add descriptive text content, an aria-label, or an image with alt text inside the link."
                        .to_string(),
                });
            } else if Self::GENERIC_TEXTS.contains(&text_lower.as_str()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "LNKACC002".to_string(),
                    title: "Link with generic text".to_string(),
                    description: format!(
                        "Link text \"{}\" is generic and does not describe the destination. \
                         Screen reader users navigating by links hear a list of identical labels.",
                        link.text.trim()
                    ),
                    url: url.to_string(),
                    recommendation: "Use descriptive link text that explains the purpose or destination of the link."
                        .to_string(),
                });
            } else if Self::NON_DESCRIPTIVE_TEXTS.contains(&text_lower.as_str()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "LNKACC003".to_string(),
                    title: "Link with non-descriptive text".to_string(),
                    description: format!(
                        "Link text \"{}\" is too short to convey meaning. Users navigating by links \
                         cannot determine the destination.",
                        link.text.trim()
                    ),
                    url: url.to_string(),
                    recommendation: "Replace the link text with a phrase that describes the link destination."
                        .to_string(),
                });
            }
        }

        findings
    }
}
