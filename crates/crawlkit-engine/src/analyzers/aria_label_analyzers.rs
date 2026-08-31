//! ARIA label accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. The public analyzer name and behavior are preserved by
//! re-exports from `analyzers::mod` and `security_analyzers`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// AriaLabelAnalyzer
// =========================================================================

pub struct AriaLabelAnalyzer;

impl Default for AriaLabelAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AriaLabelAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AriaLabelAnalyzer {
    fn name(&self) -> &str {
        "aria-label"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Check if there are ARIA roles but no labels
        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIALABEL001".to_string(),
                title: "ARIA roles without labels".to_string(),
                description: format!(
                    "{} ARIA role(s) found but no aria-label or aria-labelledby \
                     attributes. Interactive elements need accessible names.",
                    ctx.page.aria_role_count
                ),
                url: url.clone(),
                recommendation: "Add aria-label or aria-labelledby to elements with \
                                 ARIA roles."
                    .to_string(),
            });
        }

        findings
    }
}
