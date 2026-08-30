#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct CarSchemaValidatorV2;

impl Default for CarSchemaValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl CarSchemaValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CarSchemaValidatorV2 {
    fn name(&self) -> &str {
        "car-schema-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Car") {
                continue;
            }
            let data = &sd.data;

            if data
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CAR-V2001".to_string(),
                    title: "Car schema missing model".to_string(),
                    description: "A Car structured data block is missing the \"model\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"model\" with the car model name.".to_string(),
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
    fn test_car_v2_missing_model() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({"@type": "Car", "name": "Sedan"}),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAR-V2001"));
    }

    #[test]
    fn test_car_v2_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({"@type": "Car", "name": "Sedan", "model": "Model 3"}),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_car_v2_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_car_v2_non_car_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_car_v2_model_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({"@type": "Car", "model": ""}),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAR-V2001"));
    }

    #[test]
    fn test_car_v2_model_none() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({"@type": "Car"}),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAR-V2001"));
    }

    #[test]
    fn test_car_v2_multiple_cars() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Car".to_string()),
                data: serde_json::json!({"@type": "Car", "model": "Civic"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Car".to_string()),
                data: serde_json::json!({"@type": "Car"}),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAR-V2001"));
    }

    #[test]
    fn test_car_v2_with_manufacturer_no_model() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({"@type": "Car", "name": "Sedan", "manufacturer": "Toyota"}),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAR-V2001"));
    }

    #[test]
    fn test_car_v2_model_valid_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({"@type": "Car", "model": "Model S"}),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_car_v2_all_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({"@type": "Car"}),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert!(findings.iter().any(|f| f.code == "CAR-V2001"));
    }

    #[test]
    fn test_car_v2_name_missing_but_model_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({"@type": "Car", "model": "Civic"}),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}
