#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct SportsActivityLocationSchemaValidator;

impl Default for SportsActivityLocationSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SportsActivityLocationSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SportsActivityLocationSchemaValidator {
    fn name(&self) -> &str {
        "sports-activity-location-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("SportsActivityLocation") {
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
                    code: "SPORTS001".to_string(),
                    title: "SportsActivityLocation schema missing name".to_string(),
                    description: "A SportsActivityLocation structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the location name.".to_string(),
                });
            }

            if data.get("address").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SPORTS002".to_string(),
                    title: "SportsActivityLocation schema missing address".to_string(),
                    description: "A SportsActivityLocation structured data block is missing the \"address\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"address\" with a PostalAddress object.".to_string(),
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
    fn test_sports_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SportsActivityLocation".to_string()),
            data: serde_json::json!({"@type": "SportsActivityLocation"}),
        }];
        assert!(SportsActivityLocationSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "SPORTS001"));
    }

    #[test]
    fn test_sports_missing_address() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SportsActivityLocation".to_string()),
            data: serde_json::json!({"@type": "SportsActivityLocation", "name": "Gym"}),
        }];
        assert!(SportsActivityLocationSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "SPORTS002"));
    }

    #[test]
    fn test_sports_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SportsActivityLocation".to_string()),
            data: serde_json::json!({"@type": "SportsActivityLocation", "name": "Gym", "address": {"@type": "PostalAddress"}}),
        }];
        assert!(SportsActivityLocationSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_sports_non_sports_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        assert!(SportsActivityLocationSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_sports_no_data() {
        let page = make_page("https://example.com");
        assert!(SportsActivityLocationSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_sports_all_issues() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SportsActivityLocation".to_string()),
            data: serde_json::json!({"@type": "SportsActivityLocation"}),
        }];
        assert_eq!(
            SportsActivityLocationSchemaValidator::new()
                .analyze(&make_ctx(&page))
                .len(),
            2
        );
    }

    #[test]
    fn test_sports_empty_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SportsActivityLocation".to_string()),
            data: serde_json::json!({"@type": "SportsActivityLocation", "name": ""}),
        }];
        assert!(SportsActivityLocationSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "SPORTS001"));
    }

    #[test]
    fn test_sports_name_only() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SportsActivityLocation".to_string()),
            data: serde_json::json!({"@type": "SportsActivityLocation", "name": "Gym"}),
        }];
        let findings = SportsActivityLocationSchemaValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "SPORTS002"));
        assert!(!findings.iter().any(|f| f.code == "SPORTS001"));
    }

    #[test]
    fn test_sports_multiple_locations() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("SportsActivityLocation".to_string()),
                data: serde_json::json!({"@type": "SportsActivityLocation"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("SportsActivityLocation".to_string()),
                data: serde_json::json!({"@type": "SportsActivityLocation", "name": "Pool"}),
            },
        ];
        let findings = SportsActivityLocationSchemaValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().filter(|f| f.code == "SPORTS001").count() >= 1);
    }

    #[test]
    fn test_sports_valid_with_extra_fields() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SportsActivityLocation".to_string()),
            data: serde_json::json!({"@type": "SportsActivityLocation", "name": "Gym", "address": {"@type": "PostalAddress"}, "telephone": "+1-555-0100", "priceRange": "$$"}),
        }];
        assert!(SportsActivityLocationSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }
}
