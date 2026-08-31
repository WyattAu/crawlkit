//! Table accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks data tables for headers, captions, and scope metadata.
pub struct TableAccessibilityAnalyzer;

impl TableAccessibilityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TableAccessibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TableAccessibilityAnalyzer {
    fn name(&self) -> &str {
        "table-accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.tables_total == 0 {
            return findings;
        }

        let without_headers = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_headers);
        if without_headers > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TACC001".to_string(),
                title: "Table missing header cells".to_string(),
                description: format!(
                    "{} of {} table(s) have no <th> header cells. Header cells help screen reader \
                     users understand the structure and relationships of tabular data.",
                    without_headers, ctx.page.tables_total
                ),
                url: url.to_string(),
                recommendation: "Use <th> elements for header cells and add scope=\"col\" or scope=\"row\" attributes for complex tables."
                    .to_string(),
            });
        }

        let without_captions = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_captions);
        if without_captions > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TACC002".to_string(),
                title: "Table missing caption".to_string(),
                description: format!(
                    "{} of {} table(s) have no <caption> element. Captions provide a summary \
                     of the table purpose for screen reader users.",
                    without_captions, ctx.page.tables_total
                ),
                url: url.to_string(),
                recommendation: "Add a <caption> element to each data table describing its content."
                    .to_string(),
            });
        }

        let tables_needing_scope = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_headers);
        if tables_needing_scope > 10 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TACC003".to_string(),
                title: "Large number of tables missing scope attributes".to_string(),
                description: format!(
                    "{} table(s) with more than 10 rows are missing scope attributes on header cells. \
                     The scope attribute clarifies whether a header applies to a row or column.",
                    tables_needing_scope
                ),
                url: url.to_string(),
                recommendation: "Add scope=\"col\" to column headers and scope=\"row\" to row headers."
                    .to_string(),
            });
        }

        findings
    }
}
