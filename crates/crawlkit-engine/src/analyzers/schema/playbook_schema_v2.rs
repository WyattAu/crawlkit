#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct PlaybookSchemaValidatorV2;

impl Default for PlaybookSchemaValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybookSchemaValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PlaybookSchemaValidatorV2 {
    fn name(&self) -> &str {
        "playbook-schema-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Playbook") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PLAYBOOK-V2001".to_string(),
                    title: "Playbook schema missing name".to_string(),
                    description: "A Playbook structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the playbook title.".to_string(),
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

    fn make_ctx<'a>(page: &'a crate::parser::ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: None,
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
    fn test_playbook_v2_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({"@type": "Playbook", "step": [{"@type": "HowToStep", "text": "Step 1"}]}),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK-V2001"));
    }

    #[test]
    fn test_playbook_v2_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({"@type": "Playbook", "name": "Quick Start", "step": [{"@type": "HowToStep", "text": "Step 1"}]}),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_v2_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_v2_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({"@type": "HowTo"}),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_v2_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({"@type": "Playbook", "name": ""}),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK-V2001"));
    }

    #[test]
    fn test_playbook_v2_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Playbook".to_string()),
                data: serde_json::json!({"@type": "Playbook", "name": "Good"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Playbook".to_string()),
                data: serde_json::json!({"@type": "Playbook"}),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK-V2001"));
    }

    #[test]
    fn test_playbook_v2_name_only() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({"@type": "Playbook", "name": "Deployment Playbook"}),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PLAYBOOK-V2001"));
    }

    #[test]
    fn test_playbook_v2_both_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({"@type": "Playbook", "name": "Incident Response", "step": [{"@type": "HowToStep", "text": "Step 1"}]}),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_v2_name_missing_no_step() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({"@type": "Playbook"}),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK-V2001"));
    }

    #[test]
    fn test_playbook_v2_name_valid_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({"@type": "Playbook", "name": "Runbook"}),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}
