#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct NGOSchemaValidator;

impl Default for NGOSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl NGOSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for NGOSchemaValidator {
    fn name(&self) -> &str {
        "ngo-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("NGO") {
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
                    code: "NGO001".to_string(),
                    title: "NGO schema missing name".to_string(),
                    description: "An NGO structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the NGO name.".to_string(),
                });
            }
            if data.get("address").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "NGO002".to_string(),
                    title: "NGO schema missing address".to_string(),
                    description:
                        "An NGO structured data block is missing the \"address\" property."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"address\" with a PostalAddress object.".to_string(),
                });
            }
            if data.get("url").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "NGO003".to_string(),
                    title: "NGO schema missing url".to_string(),
                    description: "An NGO structured data block is missing the \"url\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"url\" with the official website URL.".to_string(),
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
    fn test_ngo_missing_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("NGO".into()),
            data: serde_json::json!({"@type":"NGO"}),
        }];
        assert!(NGOSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "NGO001"));
    }
    #[test]
    fn test_ngo_missing_address() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("NGO".into()),
            data: serde_json::json!({"@type":"NGO","name":"Red Cross"}),
        }];
        assert!(NGOSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "NGO002"));
    }
    #[test]
    fn test_ngo_missing_url() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("NGO".into()),
            data: serde_json::json!({"@type":"NGO","name":"Red Cross","address":{"@type":"PostalAddress"}}),
        }];
        assert!(NGOSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "NGO003"));
    }
    #[test]
    fn test_ngo_valid() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("NGO".into()),
            data: serde_json::json!({"@type":"NGO","name":"Red Cross","address":{"@type":"PostalAddress"},"url":"https://redcross.org"}),
        }];
        assert!(NGOSchemaValidator::new().analyze(&make_ctx(&p)).is_empty());
    }
    #[test]
    fn test_ngo_non_ngo_ignored() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({"@type":"Product"}),
        }];
        assert!(NGOSchemaValidator::new().analyze(&make_ctx(&p)).is_empty());
    }
    #[test]
    fn test_ngo_no_data() {
        let p = make_page("https://example.com");
        assert!(NGOSchemaValidator::new().analyze(&make_ctx(&p)).is_empty());
    }
    #[test]
    fn test_ngo_all_issues() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("NGO".into()),
            data: serde_json::json!({"@type":"NGO"}),
        }];
        assert_eq!(NGOSchemaValidator::new().analyze(&make_ctx(&p)).len(), 3);
    }
    #[test]
    fn test_ngo_empty_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("NGO".into()),
            data: serde_json::json!({"@type":"NGO","name":""}),
        }];
        assert!(NGOSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "NGO001"));
    }
    #[test]
    fn test_ngo_name_only_two_issues() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("NGO".into()),
            data: serde_json::json!({"@type":"NGO","name":"Red Cross"}),
        }];
        let f = NGOSchemaValidator::new().analyze(&make_ctx(&p));
        assert!(f.iter().any(|f| f.code == "NGO002"));
        assert!(f.iter().any(|f| f.code == "NGO003"));
    }
    #[test]
    fn test_ngo_name_and_address() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("NGO".into()),
            data: serde_json::json!({"@type":"NGO","name":"Red Cross","address":{"@type":"PostalAddress"}}),
        }];
        let f = NGOSchemaValidator::new().analyze(&make_ctx(&p));
        assert!(f.iter().any(|f| f.code == "NGO003"));
        assert!(!f.iter().any(|f| f.code == "NGO001"));
    }
}
