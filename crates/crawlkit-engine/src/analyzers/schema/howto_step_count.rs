#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct HowToStepCountValidator;

impl Default for HowToStepCountValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl HowToStepCountValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HowToStepCountValidator {
    fn name(&self) -> &str {
        "howto-step-count"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("HowTo") {
                continue;
            }
            let data = &sd.data;

            let step = match data.get("step") {
                Some(s) => s,
                None => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "HOWTOSTEP001".to_string(),
                        title: "HowTo schema missing step property".to_string(),
                        description: "A HowTo structured data block is missing the required \
                                      \"step\" property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add a \"step\" array with HowToStep objects.".to_string(),
                    });
                    continue;
                }
            };

            let step_count = match step.as_array() {
                Some(arr) => arr.len(),
                None => {
                    // step is a single object, count as 1
                    if step.is_object() {
                        1
                    } else {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "HOWTOSTEP001".to_string(),
                            title: "HowTo step property has unexpected format".to_string(),
                            description: "The step property is neither an array nor an object."
                                .to_string(),
                            url: url.clone(),
                            recommendation: "Set step to an array of HowToStep objects."
                                .to_string(),
                        });
                        continue;
                    }
                }
            };

            if step_count < 2 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "HOWTOSTEP001".to_string(),
                    title: "HowTo schema has fewer than 2 steps".to_string(),
                    description: format!(
                        "HowTo step property contains only {step_count} step(s). A HowTo \
                         guide should have at least 2 steps to be meaningful."
                    ),
                    url: url.clone(),
                    recommendation: "Add at least 2 HowToStep objects to the step array."
                        .to_string(),
                });
            }
        }

        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ParsedPage, StructuredData};

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    #[test]
    fn test_howto_missing_step() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({"@type": "HowTo", "name": "How to cook"}),
        }];
        assert!(HowToStepCountValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "HOWTOSTEP001"));
    }

    #[test]
    fn test_howto_one_step() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({"@type": "HowTo", "name": "How to cook", "step": [
                {"@type": "HowToStep", "text": "Step 1"}
            ]}),
        }];
        assert!(HowToStepCountValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "HOWTOSTEP001"));
    }

    #[test]
    fn test_howto_two_steps() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({"@type": "HowTo", "name": "How to cook", "step": [
                {"@type": "HowToStep", "text": "Step 1"},
                {"@type": "HowToStep", "text": "Step 2"}
            ]}),
        }];
        assert!(HowToStepCountValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_howto_single_step_object() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({"@type": "HowTo", "name": "How to cook", "step": {"@type": "HowToStep", "text": "Step 1"}}),
        }];
        assert!(HowToStepCountValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "HOWTOSTEP001"));
    }

    #[test]
    fn test_non_howto_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        assert!(HowToStepCountValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(HowToStepCountValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(HowToStepCountValidator::new().name(), "howto-step-count");
    }

    #[test]
    fn test_howto_empty_step_array() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({"@type": "HowTo", "name": "How to cook", "step": []}),
        }];
        assert!(HowToStepCountValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "HOWTOSTEP001"));
    }

    #[test]
    fn test_howto_step_string() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({"@type": "HowTo", "name": "How to cook", "step": "just a string"}),
        }];
        assert!(HowToStepCountValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "HOWTOSTEP001"));
    }

    #[test]
    fn test_default() {
        let _ = HowToStepCountValidator::default();
    }
}
