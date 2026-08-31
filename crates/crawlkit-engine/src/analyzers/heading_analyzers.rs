//! Heading-order accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks heading hierarchy for skipped levels and inconsistent ordering.
pub struct HeadingOrderAnalyzer;

impl HeadingOrderAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HeadingOrderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HeadingOrderAnalyzer {
    fn name(&self) -> &str {
        "heading-order"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.headings.len() < 2 {
            return findings;
        }

        let mut prev_level: Option<u8> = None;
        let mut found_descent = false;

        for heading in &ctx.page.headings {
            if let Some(prev) = prev_level {
                if heading.level > prev + 1 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "HORDER001".to_string(),
                        title: "Heading level skip detected".to_string(),
                        description: format!(
                            "Heading jumps from H{prev} to H{}, skipping intermediate levels. \
                             Screen readers and outline tools rely on sequential heading levels.",
                            heading.level
                        ),
                        url: url.to_string(),
                        recommendation: format!(
                            "Use H{} after H{prev} to maintain a proper document outline.",
                            prev + 1
                        ),
                    });
                }
                if heading.level < prev && !found_descent {
                    found_descent = true;
                }
                if found_descent && heading.level > prev {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "HORDER002".to_string(),
                        title: "Non-sequential heading order".to_string(),
                        description: format!(
                            "Heading level decreased from H{prev} to H{} and then increased again. \
                             Heading levels should follow a strictly non-increasing pattern within sections.",
                            heading.level
                        ),
                        url: url.to_string(),
                        recommendation: "Ensure heading levels descend sequentially (H1 > H2 > H3) and do not increase within a section."
                            .to_string(),
                    });
                    break;
                }
            }
            prev_level = Some(heading.level);
        }

        findings
    }
}
