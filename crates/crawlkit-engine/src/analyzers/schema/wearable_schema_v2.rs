#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct WearableSchemaValidatorV2;

impl Default for WearableSchemaValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl WearableSchemaValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WearableSchemaValidatorV2 {
    fn name(&self) -> &str {
        "wearable-schema-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("WearableDevice") {
                continue;
            }
            let data = &sd.data;

            if data.get("deviceType").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WEAR-V2001".to_string(),
                    title: "WearableDevice schema missing deviceType".to_string(),
                    description: "A WearableDevice structured data block is missing the \
                                  \"deviceType\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"deviceType\" with the type of wearable device."
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
    fn test_wear_v2_missing_device_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WearableDevice".to_string()),
            data: serde_json::json!({"@type": "WearableDevice", "name": "Smartwatch"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WearableSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEAR-V2001"));
    }

    #[test]
    fn test_wear_v2_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WearableDevice".to_string()),
            data: serde_json::json!({"@type": "WearableDevice", "deviceType": "Smartwatch"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(WearableSchemaValidatorV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_wear_v2_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(WearableSchemaValidatorV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_wear_v2_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(WearableSchemaValidatorV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_wear_v2_device_type_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WearableDevice".to_string()),
            data: serde_json::json!({"@type": "WearableDevice", "deviceType": ""}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WearableSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEAR-V2001"));
    }

    #[test]
    fn test_wear_v2_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WearableDevice".to_string()),
                data: serde_json::json!({"@type": "WearableDevice", "deviceType": "Watch"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WearableDevice".to_string()),
                data: serde_json::json!({"@type": "WearableDevice"}),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = WearableSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEAR-V2001"));
    }

    #[test]
    fn test_wear_v2_severity_warning() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WearableDevice".to_string()),
            data: serde_json::json!({"@type": "WearableDevice"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WearableSchemaValidatorV2::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].category, IssueCategory::Schema);
    }

    #[test]
    fn test_wear_v2_one_finding() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WearableDevice".to_string()),
            data: serde_json::json!({"@type": "WearableDevice"}),
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(WearableSchemaValidatorV2::new().analyze(&ctx).len(), 1);
    }

    #[test]
    fn test_wear_v2_name_only() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WearableDevice".to_string()),
            data: serde_json::json!({"@type": "WearableDevice", "name": "Fitness Tracker"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WearableSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEAR-V2001"));
    }

    #[test]
    fn test_wear_v2_device_type_smartwatch() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WearableDevice".to_string()),
            data: serde_json::json!({"@type": "WearableDevice", "deviceType": "Smartwatch", "name": "Watch"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(WearableSchemaValidatorV2::new().analyze(&ctx).is_empty());
    }
}
