//! Form and table accessibility validators.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. Public names and behavior are preserved through re-exports.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// FormLabelAssociationAnalyzer — FORMLAB001
// =========================================================================

pub struct FormLabelAssociationAnalyzer;
impl Default for FormLabelAssociationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl FormLabelAssociationAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FormLabelAssociationAnalyzer {
    fn name(&self) -> &str {
        "form-label-association"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let mut unlabeled = 0;
        for form in &ctx.page.forms {
            for input in &form.inputs {
                if !input.has_label
                    && input
                        .aria_label
                        .as_ref()
                        .is_none_or(|l| l.trim().is_empty())
                    && input
                        .aria_labelledby
                        .as_ref()
                        .is_none_or(|l| l.trim().is_empty())
                {
                    unlabeled += 1;
                }
            }
        }
        if unlabeled > 0 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "FORMLAB001".to_string(),
                title: "Form inputs missing label associations".to_string(),
                description: format!("{unlabeled} form input(s) have no associated <label>, aria-label, or aria-labelledby."),
                url: url.to_string(),
                recommendation: "Associate a <label> element with each input using for/id attributes, or add aria-label.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// TableHeaderScopeAnalyzer — TBLSCOP001
// =========================================================================

pub struct TableHeaderScopeAnalyzer;
impl Default for TableHeaderScopeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl TableHeaderScopeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TableHeaderScopeAnalyzer {
    fn name(&self) -> &str {
        "table-header-scope"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 {
            return findings;
        }
        let without = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_headers);
        if without > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TBLSCOP001".to_string(),
                title: "Tables missing header cells with scope".to_string(),
                description: format!("{without} of {} table(s) have no <th> header cells. Header cells with scope attributes clarify data relationships.", ctx.page.tables_total),
                url: url.to_string(),
                recommendation: "Use <th scope=\"col\"> for column headers and <th scope=\"row\"> for row headers.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// TableCaptionPresenceAnalyzer — TBLCAP001
// =========================================================================

pub struct TableCaptionPresenceAnalyzer;
impl Default for TableCaptionPresenceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl TableCaptionPresenceAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TableCaptionPresenceAnalyzer {
    fn name(&self) -> &str {
        "table-caption-presence"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 {
            return findings;
        }
        let without = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_captions);
        if without > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TBLCAP001".to_string(),
                title: "Tables missing caption element".to_string(),
                description: format!("{without} of {} table(s) have no <caption>. Captions describe table purpose for screen readers.", ctx.page.tables_total),
                url: url.to_string(),
                recommendation: "Add a <caption> element to each data table.".to_string(),
            });
        }
        findings
    }
}
