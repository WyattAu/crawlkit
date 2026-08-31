//! Table caption accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. The public analyzer name and behavior are preserved by
//! re-exports from `analyzers::mod` and `security_analyzers`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// TableCaptionAnalyzer
// =========================================================================

pub struct TableCaptionAnalyzer;

impl Default for TableCaptionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TableCaptionAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TableCaptionAnalyzer {
    fn name(&self) -> &str {
        "table-caption"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.tables_total == 0 {
            return findings;
        }

        let without_captions = ctx.page.tables_total - ctx.page.tables_with_captions;
        if without_captions > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TABLECAP001".to_string(),
                title: "Table missing caption element".to_string(),
                description: format!(
                    "{without_captions} of {} table(s) have no <caption> element. \
                     Captions help screen reader users understand table purpose.",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add a <caption> element to each table to describe its \
                                 purpose."
                    .to_string(),
            });
        }

        findings
    }
}
