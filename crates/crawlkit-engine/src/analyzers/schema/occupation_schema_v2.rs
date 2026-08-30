#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct OccupationSchemaValidatorV2;

impl Default for OccupationSchemaValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl OccupationSchemaValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for OccupationSchemaValidatorV2 {
    fn name(&self) -> &str {
        "occupation-schema-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Occupation") {
                continue;
            }
            let data = &sd.data;

            let has_category = match data.get("occupationalCategory") {
                None | Some(serde_json::Value::Null) => false,
                Some(serde_json::Value::String(s)) => !s.is_empty(),
                Some(serde_json::Value::Object(_)) => true,
                Some(serde_json::Value::Array(a)) => !a.is_empty(),
                Some(_) => true,
            };
            if !has_category {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "OCCUP-V2001".to_string(),
                    title: "Occupation schema missing occupationalCategory".to_string(),
                    description: "An Occupation structured data block is missing the \
                                  \"occupationalCategory\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"occupationalCategory\" with a category code or text."
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
    fn test_occup_v2_missing_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({"@type": "Occupation", "name": "Engineer"}),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP-V2001"));
    }

    #[test]
    fn test_occup_v2_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({"@type": "Occupation", "name": "Engineer", "occupationalCategory": "15-1252"}),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_occup_v2_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_occup_v2_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person"}),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_occup_v2_category_null() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({"@type": "Occupation", "occupationalCategory": null}),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP-V2001"));
    }

    #[test]
    fn test_occup_v2_category_empty_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({"@type": "Occupation", "occupationalCategory": ""}),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP-V2001"));
    }

    #[test]
    fn test_occup_v2_category_object() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({"@type": "Occupation", "occupationalCategory": {"@type": "CategoryCode", "codeValue": "29-1141"}}),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_occup_v2_category_empty_array() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({"@type": "Occupation", "occupationalCategory": []}),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP-V2001"));
    }

    #[test]
    fn test_occup_v2_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Occupation".to_string()),
                data: serde_json::json!({"@type": "Occupation", "occupationalCategory": "17-2000"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Occupation".to_string()),
                data: serde_json::json!({"@type": "Occupation"}),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP-V2001"));
    }

    #[test]
    fn test_occup_v2_both_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({"@type": "Occupation", "name": "Nurse", "occupationalCategory": "29-1141"}),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}
