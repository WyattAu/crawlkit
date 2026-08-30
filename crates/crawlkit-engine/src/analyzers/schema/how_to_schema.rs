#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct HowToSchemaValidator;

impl Default for HowToSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl HowToSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HowToSchemaValidator {
    fn name(&self) -> &str {
        "howto-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("HowTo") {
                continue;
            }
            let data = &sd.data;

            // HOWTO001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "HOWTO001".to_string(),
                    title: "HowTo schema missing name".to_string(),
                    description: "A HowTo structured data block is missing the required \
                                  \"name\" property. The name describes the overall procedure."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with a descriptive title for the how-to guide."
                        .to_string(),
                });
            }

            // HOWTO002: Missing step
            let Some(steps) = data.get("step") else {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "HOWTO002".to_string(),
                    title: "HowTo schema missing step".to_string(),
                    description: "A HowTo structured data block is missing the required \
                                  \"step\" property. Steps define the individual actions in the \
                                  how-to procedure."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"step\" with an array of HowToStep objects.".to_string(),
                });
                continue;
            };
            let steps_arr = steps.as_array();

            // HOWTO003: Steps missing name or text
            if let Some(arr) = steps_arr {
                for (i, step) in arr.iter().enumerate() {
                    let has_name = step.get("name").is_some();
                    let has_text = step.get("text").is_some();
                    if !has_name || !has_text {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "HOWTO003".to_string(),
                            title: "HowTo step missing name or text".to_string(),
                            description: format!(
                                "Step at position {} is missing {}.",
                                i + 1,
                                if !has_name && !has_text {
                                    "both \"name\" and \"text\""
                                } else if !has_name {
                                    "the \"name\" property"
                                } else {
                                    "the \"text\" property"
                                }
                            ),
                            url: url.clone(),
                            recommendation: "Add both \"name\" and \"text\" properties to each \
                                             HowToStep."
                                .to_string(),
                        });
                    }
                }
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
    use crate::parser::StructuredData;

    fn make_page(url: &str) -> crate::parser::ParsedPage {
        crate::parser::ParsedPage {
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

    fn make_ctx<'a>(
        page: &'a crate::parser::ParsedPage,
        status: Option<u16>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
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
    fn test_howto_missing_name() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "step": [{"@type": "HowToStep", "name": "Step 1", "text": "Do this"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "HOWTO001"));
    }

    #[test]
    fn test_howto_missing_step() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake a cake"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "HOWTO002"));
    }

    #[test]
    fn test_howto_step_missing_name_and_text() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake",
                "step": [{"@type": "HowToStep"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "HOWTO003"));
    }

    #[test]
    fn test_howto_step_missing_name() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake",
                "step": [{"@type": "HowToStep", "text": "Do this"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HowToSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HOWTO003"));
        assert!(!findings.iter().any(|f| f.code == "HOWTO001"));
    }

    #[test]
    fn test_howto_step_missing_text() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake",
                "step": [{"@type": "HowToStep", "name": "Step 1"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HowToSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HOWTO003"));
        assert!(!findings.iter().any(|f| f.code == "HOWTO001"));
    }

    #[test]
    fn test_howto_valid() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake a cake",
                "step": [
                    {"@type": "HowToStep", "name": "Prep", "text": "Preheat oven"},
                    {"@type": "HowToStep", "name": "Bake", "text": "Put in oven"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_howto_non_howto_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_howto_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_howto_multiple_steps_missing_properties() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake",
                "step": [
                    {"@type": "HowToStep", "text": "Step 1"},
                    {"@type": "HowToStep", "name": "Step 2"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HowToSchemaValidator::new().analyze(&ctx);
        let howto003_count = findings.iter().filter(|f| f.code == "HOWTO003").count();
        assert_eq!(howto003_count, 2);
    }

    #[test]
    fn test_howto_missing_all_fields() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({"@type": "HowTo"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HowToSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HOWTO001"));
        assert!(findings.iter().any(|f| f.code == "HOWTO002"));
    }
}
