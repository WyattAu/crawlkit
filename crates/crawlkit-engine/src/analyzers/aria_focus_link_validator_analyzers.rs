//! Link, ARIA, and focus accessibility validators.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. Public names and behavior are preserved through re-exports.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// AnchorTextGenericAnalyzer — ANCHGEN001
// =========================================================================

pub struct AnchorTextGenericAnalyzer;
impl Default for AnchorTextGenericAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl AnchorTextGenericAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AnchorTextGenericAnalyzer {
    fn name(&self) -> &str {
        "anchor-text-generic"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic = [
            "click here",
            "read more",
            "learn more",
            "here",
            "more",
            "link",
            "this",
            "continue",
            "go",
        ];
        for link in &ctx.page.links {
            let text = link.text.trim().to_lowercase();
            if !text.is_empty() && generic.contains(&text.as_str()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "ANCHGEN001".to_string(),
                    title: "Link with generic anchor text".to_string(),
                    description: format!(
                        "Link text \"{}\" is generic and does not describe the destination.",
                        link.text.trim()
                    ),
                    url: url.to_string(),
                    recommendation: "Use descriptive text that explains the link purpose."
                        .to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// AriaRequiredAttributesAnalyzer — ARIAREQ001
// =========================================================================

pub struct AriaRequiredAttributesAnalyzer;
impl Default for AriaRequiredAttributesAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl AriaRequiredAttributesAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AriaRequiredAttributesAnalyzer {
    fn name(&self) -> &str {
        "aria-required-attributes"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIAREQ001".to_string(),
                title: "ARIA roles missing required accessible name attributes".to_string(),
                description: format!("{} ARIA role(s) found without aria-label or aria-labelledby. Roles require accessible names for screen readers.", ctx.page.aria_role_count),
                url: url.to_string(),
                recommendation: "Add aria-label or aria-labelledby to all elements with ARIA roles.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// FocusOrderPositiveTabindexAnalyzer — TABPOS001
// =========================================================================

pub struct FocusOrderPositiveTabindexAnalyzer;
impl Default for FocusOrderPositiveTabindexAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl FocusOrderPositiveTabindexAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FocusOrderPositiveTabindexAnalyzer {
    fn name(&self) -> &str {
        "focus-order-positive-tabindex"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "TABPOS001".to_string(),
                title: "Positive tabindex values disrupt focus order".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, making keyboard navigation unpredictable.".to_string(),
                url: ctx.page.url.to_string(),
                recommendation: "Use tabindex=\"0\" for natural order or tabindex=\"-1\" for programmatic focus.".to_string(),
            });
        }
        findings
    }
}
