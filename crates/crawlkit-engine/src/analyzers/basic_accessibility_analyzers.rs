//! Basic accessibility analyzers for form inputs, links, images, and ARIA roles.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. Public names and behavior are preserved by re-exports from
//! `analyzers::mod` and `security_analyzers`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// FormInputLabelAnalyzer
// =========================================================================

pub struct FormInputLabelAnalyzer;

impl Default for FormInputLabelAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl FormInputLabelAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FormInputLabelAnalyzer {
    fn name(&self) -> &str {
        "form-input-label"
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
                            code: "FILABEL001".to_string(),
                            title: "Form input missing associated label".to_string(),
                            description: format!(
                                "{desc} has no associated <label> element, aria-label, or \
                                 aria-labelledby attribute."
                            ),
                            url: url.to_string(),
                            recommendation: "Associate a <label> element with the input."
                                .to_string(),
                        });
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// LinkTextAnalyzer
// =========================================================================

pub struct LinkTextAnalyzer;

impl Default for LinkTextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkTextAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LinkTextAnalyzer {
    fn name(&self) -> &str {
        "link-text"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for link in &ctx.page.links {
            let text = link.text.trim();
            if text.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "LINKTEXT001".to_string(),
                    title: "Link with empty text".to_string(),
                    description: format!(
                        "A link to \"{}\" has no visible text content.",
                        link.href
                    ),
                    url: url.to_string(),
                    recommendation: "Add descriptive text content inside the <a> tag.".to_string(),
                });
                continue;
            }

            let lower = text.to_lowercase();
            let generic_texts = [
                "click here",
                "read more",
                "learn more",
                "here",
                "link",
                "more",
                "this",
                "continue",
            ];
            for generic in &generic_texts {
                if lower == *generic {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "LINKTEXT002".to_string(),
                        title: "Link with generic text".to_string(),
                        description: format!(
                            "Link text \"{text}\" is generic and does not describe the destination."
                        ),
                        url: url.to_string(),
                        recommendation: "Replace generic text with descriptive text.".to_string(),
                    });
                    break;
                }
            }
        }

        findings
    }
}

// =========================================================================
// ImageAltTextAnalyzer
// =========================================================================

pub struct ImageAltTextAnalyzer;

impl Default for ImageAltTextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageAltTextAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ImageAltTextAnalyzer {
    fn name(&self) -> &str {
        "image-alt-text"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for image in &ctx.page.images {
            if !image.has_alt || image.alt.trim().is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "IMGALT001".to_string(),
                    title: "Image missing alt text".to_string(),
                    description: format!(
                        "Image \"{}\" is missing an alt attribute or has empty alt text.",
                        image.src
                    ),
                    url: url.to_string(),
                    recommendation: "Add a descriptive alt attribute to the image.".to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// AriaRoleAnalyzer
// =========================================================================

pub struct AriaRoleAnalyzer;

impl Default for AriaRoleAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AriaRoleAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AriaRoleAnalyzer {
    fn name(&self) -> &str {
        "aria-role"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIAROLE001".to_string(),
                title: "ARIA roles without accessible names".to_string(),
                description: format!(
                    "{} ARIA role(s) found but no aria-label or aria-labelledby attributes.",
                    ctx.page.aria_role_count
                ),
                url: url.to_string(),
                recommendation: "Add aria-label or aria-labelledby to all elements with ARIA roles."
                    .to_string(),
            });
        }

        findings
    }
}
