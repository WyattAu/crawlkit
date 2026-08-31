//! Form-label accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks form controls for associated labels and accessible names.
pub struct FormLabelAnalyzer;

impl FormLabelAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FormLabelAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FormLabelAnalyzer {
    fn name(&self) -> &str {
        "form-labels"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for form in &ctx.page.forms {
            for input in &form.inputs {
                if !input.has_label {
                    let aria_has_name = input
                        .aria_label
                        .as_ref()
                        .is_some_and(|l| !l.trim().is_empty())
                        || input
                            .aria_labelledby
                            .as_ref()
                            .is_some_and(|l| !l.trim().is_empty());
                    if !aria_has_name {
                        let desc = match (&input.name, &input.input_type) {
                            (Some(n), Some(t)) => format!("input (name=\"{n}\", type=\"{t}\")"),
                            (Some(n), None) => format!("input (name=\"{n}\")"),
                            (None, Some(t)) => format!("input (type=\"{t}\")"),
                            (None, None) => "input".to_string(),
                        };
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Accessibility,
                            code: "FLABEL001".to_string(),
                            title: "Form input missing associated label".to_string(),
                            description: format!(
                                "{desc} has no associated <label> element, aria-label, or aria-labelledby attribute. \
                                 Screen readers cannot announce the purpose of unlabeled inputs."
                            ),
                            url: url.to_string(),
                            recommendation: "Associate a <label> element with the input using the for/id attributes, or add an aria-label attribute."
                                .to_string(),
                        });
                    }
                } else if let Some(placeholder) = &input.placeholder {
                    if !placeholder.trim().is_empty() && input.aria_label.is_none() {
                        let label_text = input.name.as_deref().unwrap_or("input");
                        if label_text.trim().is_empty() {
                            findings.push(Finding {
                                severity: Severity::Info,
                                category: IssueCategory::Accessibility,
                                code: "FLABEL002".to_string(),
                                title: "Form input with empty label text".to_string(),
                                description: format!(
                                    "input (name=\"{}\") has a <label> element but the label text may be empty. \
                                     Placeholder text is not a substitute for a proper label.",
                                    input.name.as_deref().unwrap_or("")
                                ),
                                url: url.to_string(),
                                recommendation: "Ensure the <label> element contains descriptive text explaining the input purpose."
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }

        findings
    }
}
