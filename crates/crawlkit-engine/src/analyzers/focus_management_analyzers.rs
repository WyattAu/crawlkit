//! Focus-management accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks keyboard focus order and visible focus indicators.
pub struct FocusManagementAnalyzer;

impl FocusManagementAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FocusManagementAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FocusManagementAnalyzer {
    fn name(&self) -> &str {
        "focus-management"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "FOCUS001".to_string(),
                title: "Positive tabindex values disrupt focus order".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, causing \
                              keyboard navigation to skip elements or follow an unpredictable sequence. \
                              This violates WCAG 2.4.3 Focus Order."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Remove positive tabindex values. Use tabindex=\"0\" to add elements to the natural tab order or tabindex=\"-1\" for programmatic focus only."
                    .to_string(),
            });
        }

        let body = ctx.body.unwrap_or("");
        let has_focus_style = body.contains(":focus")
            || body.contains(":focus-visible")
            || body.contains(":focus-within");
        let interactive_count = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.text.trim().is_empty())
            .count()
            + ctx.page.forms.len();

        if interactive_count > 0 && !has_focus_style {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FOCUS002".to_string(),
                title: "No visible focus indicators found".to_string(),
                description: format!(
                    "Page has {} interactive element(s) but no :focus or :focus-visible CSS rules \
                     were detected. Keyboard users rely on visible focus indicators to know which \
                     element is active.",
                    interactive_count
                ),
                url: url.to_string(),
                recommendation: "Add :focus and/or :focus-visible CSS rules with a visible outline or background change. Ensure the indicator has sufficient contrast (3:1 minimum)."
                    .to_string(),
            });
        }

        findings
    }
}
