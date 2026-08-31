//! Skip-navigation link accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. The public analyzer name and behavior are preserved by
//! re-exports from `analyzers::mod` and `security_analyzers`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// SkipLinkAnalyzer
// =========================================================================

pub struct SkipLinkAnalyzer;

impl Default for SkipLinkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SkipLinkAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SkipLinkAnalyzer {
    fn name(&self) -> &str {
        "skip-link"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.has_nav_landmark && !ctx.page.has_skip_link {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "SKIPLINK001".to_string(),
                title: "No skip navigation link".to_string(),
                description: "The page has a navigation landmark but no skip-to-content \
                              link. Keyboard users must tab through all navigation links \
                              to reach main content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add a skip link as the first focusable element pointing \
                                 to the main content area."
                    .to_string(),
            });
        }

        findings
    }
}
