//! ARIA roles accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks ARIA roles for accessible names.
pub struct AriaRolesAnalyzer;

impl AriaRolesAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AriaRolesAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AriaRolesAnalyzer {
    fn name(&self) -> &str {
        "aria-roles"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIA001".to_string(),
                title: "ARIA roles without accessible names".to_string(),
                description: format!(
                    "{} ARIA role(s) found but no aria-label or aria-labelledby attributes. \
                     Custom ARIA roles require accessible names so screen readers can announce \
                     the element purpose.",
                    ctx.page.aria_role_count
                ),
                url: url.to_string(),
                recommendation:
                    "Add aria-label or aria-labelledby to all elements with custom ARIA roles."
                        .to_string(),
            });
        }

        if ctx.page.aria_role_count > 0
            && ctx.page.aria_label_count > 0
            && ctx.page.aria_role_count > ctx.page.aria_label_count
        {
            let unlabeled = ctx
                .page
                .aria_role_count
                .saturating_sub(ctx.page.aria_label_count);
            if unlabeled > 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "ARIA002".to_string(),
                    title: "ARIA roles may need accessible names on non-semantic elements".to_string(),
                    description: format!(
                        "{} ARIA role(s) are used but not all have associated accessible names. \
                         When adding ARIA roles to non-semantic elements like <div> or <span>, \
                         ensure each has an aria-label or aria-labelledby.",
                        unlabeled
                    ),
                    url: url.to_string(),
                    recommendation: "Every element with a role attribute should have an accessible name via aria-label, aria-labelledby, or visible text content."
                        .to_string(),
                });
            }
        }

        findings
    }
}
