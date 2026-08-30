#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct PlaybookSchemaValidator;

impl Default for PlaybookSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybookSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PlaybookSchemaValidator {
    fn name(&self) -> &str {
        "playbook-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Playbook") {
                continue;
            }
            let data = &sd.data;

            if data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PLAYBOOK001".to_string(),
                    title: "Playbook schema missing name".to_string(),
                    description: "A Playbook structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the playbook title.".to_string(),
                });
            }

            match data.get("step") {
                None => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "PLAYBOOK002".to_string(),
                        title: "Playbook schema missing step".to_string(),
                        description: "A Playbook structured data block is missing the required \
                                      \"step\" property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"step\" with an array of HowToStep objects."
                            .to_string(),
                    });
                }
                Some(val) if val.as_array().is_none_or(|a| a.is_empty()) => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "PLAYBOOK002".to_string(),
                        title: "Playbook schema step is empty".to_string(),
                        description:
                            "A Playbook structured data block has an empty \"step\" array."
                                .to_string(),
                        url: url.clone(),
                        recommendation: "Populate the step array with HowToStep objects."
                            .to_string(),
                    });
                }
                _ => {}
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
    fn test_playbook_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK001"));
    }

    #[test]
    fn test_playbook_missing_step() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Quick Start Guide"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Quick Start Guide",
                "step": [
                    {"@type": "HowToStep", "text": "Step 1"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_non_playbook_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "HowTo"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK001"));
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK001"));
    }

    #[test]
    fn test_playbook_step_empty_array() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Guide",
                "step": []
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_step_null() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Guide",
                "step": null
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Playbook".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Playbook",
                    "name": "Good Guide",
                    "step": [{"@type": "HowToStep", "text": "Do this"}]
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Playbook".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Playbook"
                }),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK001"));
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_name_only_no_step() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Deployment Playbook"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PLAYBOOK001"));
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }
}
