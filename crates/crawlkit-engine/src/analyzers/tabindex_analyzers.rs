//! Tabindex accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. The public analyzer name and behavior are preserved by
//! re-exports from `analyzers::mod` and `security_analyzers`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// TabindexAnalyzer
// =========================================================================

pub struct TabindexAnalyzer;

impl Default for TabindexAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TabindexAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TabindexAnalyzer {
    fn name(&self) -> &str {
        "tabindex"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Only positive tabindex is an accessibility issue. tabindex=-1 is a
        // valid, intentional pattern (removes from tab order while keeping
        // the element programmatically focusable), so negative counts must
        // not produce findings.
        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "TABINDEX001".to_string(),
                title: "Positive tabindex values detected".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, \
                              making keyboard navigation unpredictable. Users expect a \
                              sequential tab flow matching the visual layout."
                    .to_string(),
                url: url.clone(),
                recommendation: "Use tabindex=\"0\" to add elements to the natural tab \
                                 order or tabindex=\"-1\" for programmatic focus only."
                    .to_string(),
            });
        }

        findings
    }
}
