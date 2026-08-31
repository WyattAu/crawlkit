//! Language-attribute accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks the HTML language attribute and its relationship to hreflang tags.
pub struct LanguageAttributeAnalyzer;

impl LanguageAttributeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LanguageAttributeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LanguageAttributeAnalyzer {
    fn name(&self) -> &str {
        "language-attribute"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if !ctx.page.has_lang_attribute {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LANGACC001".to_string(),
                title: "Missing html lang attribute".to_string(),
                description: "The <html> element has no lang attribute. Screen readers use this \
                              attribute to select the correct pronunciation engine and hyphenation rules."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add lang=\"en\" (or the appropriate language code) to the <html> element."
                    .to_string(),
            });
        }

        if let Some(lang) = &ctx.page.html_lang {
            if lang.len() < 2 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "LANGACC002".to_string(),
                    title: "Lang attribute value too short".to_string(),
                    description: format!(
                        "The html lang attribute is set to \"{}\", which is shorter than the minimum \
                         2-character language code. Valid examples: \"en\", \"fr\", \"de\", \"zh-CN\".",
                        lang
                    ),
                    url: url.to_string(),
                    recommendation: "Use a valid BCP 47 language tag (e.g., \"en\", \"fr-CA\", \"zh-CN\")."
                        .to_string(),
                });
            }

            let has_hreflang = ctx.page.meta.hreflang.iter().any(|h| h.lang == *lang);
            let has_content = ctx.page.word_count > 0;
            if has_content && !has_hreflang && !ctx.page.meta.hreflang.is_empty() {
                let hreflang_langs: Vec<&str> = ctx
                    .page
                    .meta
                    .hreflang
                    .iter()
                    .map(|h| h.lang.as_str())
                    .collect();
                if !hreflang_langs.contains(&lang.as_str()) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "LANGACC002".to_string(),
                        title: "Lang attribute doesn't match hreflang declarations".to_string(),
                        description: format!(
                            "The html lang=\"{}\" but hreflang tags declare: {}. The page language \
                             should match one of the declared hreflang values.",
                            lang,
                            hreflang_langs.join(", ")
                        ),
                        url: url.to_string(),
                        recommendation: "Ensure the html lang attribute matches the content language declared in hreflang tags."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}
