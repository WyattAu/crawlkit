#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct FoodEstablishmentSchemaValidator;

impl Default for FoodEstablishmentSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl FoodEstablishmentSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FoodEstablishmentSchemaValidator {
    fn name(&self) -> &str {
        "food-establishment-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("FoodEstablishment") {
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
                    code: "FOOD001".to_string(),
                    title: "FoodEstablishment schema missing name".to_string(),
                    description: "A FoodEstablishment structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the establishment name.".to_string(),
                });
            }

            if data.get("address").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "FOOD002".to_string(),
                    title: "FoodEstablishment schema missing address".to_string(),
                    description: "A FoodEstablishment structured data block is missing the \"address\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"address\" with a PostalAddress object.".to_string(),
                });
            }

            if data.get("servesCuisine").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "FOOD003".to_string(),
                    title: "FoodEstablishment schema missing servesCuisine".to_string(),
                    description: "A FoodEstablishment structured data block is missing the \"servesCuisine\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"servesCuisine\" with the cuisine type.".to_string(),
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
    fn test_food_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FoodEstablishment".to_string()),
            data: serde_json::json!({"@type": "FoodEstablishment"}),
        }];
        assert!(FoodEstablishmentSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "FOOD001"));
    }

    #[test]
    fn test_food_missing_address() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FoodEstablishment".to_string()),
            data: serde_json::json!({"@type": "FoodEstablishment", "name": "Cafe"}),
        }];
        assert!(FoodEstablishmentSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "FOOD002"));
    }

    #[test]
    fn test_food_missing_cuisine() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FoodEstablishment".to_string()),
            data: serde_json::json!({"@type": "FoodEstablishment", "name": "Cafe", "address": {"@type": "PostalAddress"}}),
        }];
        assert!(FoodEstablishmentSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "FOOD003"));
    }

    #[test]
    fn test_food_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FoodEstablishment".to_string()),
            data: serde_json::json!({"@type": "FoodEstablishment", "name": "Cafe", "address": {"@type": "PostalAddress"}, "servesCuisine": "Italian"}),
        }];
        assert!(FoodEstablishmentSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_food_non_food_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        assert!(FoodEstablishmentSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_food_no_data() {
        let page = make_page("https://example.com");
        assert!(FoodEstablishmentSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_food_all_issues() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FoodEstablishment".to_string()),
            data: serde_json::json!({"@type": "FoodEstablishment"}),
        }];
        let findings = FoodEstablishmentSchemaValidator::new().analyze(&make_ctx(&page));
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn test_food_name_only_missing_two() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FoodEstablishment".to_string()),
            data: serde_json::json!({"@type": "FoodEstablishment", "name": "Cafe"}),
        }];
        let findings = FoodEstablishmentSchemaValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "FOOD002"));
        assert!(findings.iter().any(|f| f.code == "FOOD003"));
        assert!(!findings.iter().any(|f| f.code == "FOOD001"));
    }

    #[test]
    fn test_food_empty_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FoodEstablishment".to_string()),
            data: serde_json::json!({"@type": "FoodEstablishment", "name": ""}),
        }];
        assert!(FoodEstablishmentSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "FOOD001"));
    }

    #[test]
    fn test_food_multiple_establishments() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("FoodEstablishment".to_string()),
                data: serde_json::json!({"@type": "FoodEstablishment"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("FoodEstablishment".to_string()),
                data: serde_json::json!({"@type": "FoodEstablishment", "name": "Cafe"}),
            },
        ];
        let findings = FoodEstablishmentSchemaValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().filter(|f| f.code == "FOOD001").count() >= 1);
    }
}
