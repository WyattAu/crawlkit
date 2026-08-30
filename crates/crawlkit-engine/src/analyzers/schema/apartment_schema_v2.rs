#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ApartmentSchemaValidatorV2;

impl Default for ApartmentSchemaValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ApartmentSchemaValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ApartmentSchemaValidatorV2 {
    fn name(&self) -> &str {
        "apartment-schema-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Apartment") {
                continue;
            }
            let data = &sd.data;

            let has_rooms = match data.get("numberOfRooms") {
                None | Some(serde_json::Value::Null) => false,
                Some(serde_json::Value::String(s)) => !s.is_empty(),
                Some(serde_json::Value::Number(_)) => true,
                Some(_) => true,
            };
            if !has_rooms {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "APT-V2001".to_string(),
                    title: "Apartment schema missing numberOfRooms".to_string(),
                    description: "An Apartment structured data block is missing the \
                                  \"numberOfRooms\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"numberOfRooms\" with the number of rooms."
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
    fn test_apt_v2_missing_numberofrooms() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "Sunny Flat"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "APT-V2001"));
    }

    #[test]
    fn test_apt_v2_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "Sunny Flat",
                "numberOfRooms": 3
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_apt_v2_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_apt_v2_non_apt_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_apt_v2_numberofrooms_zero() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "Studio",
                "numberOfRooms": 0
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_apt_v2_numberofrooms_null() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "Flat",
                "numberOfRooms": null
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "APT-V2001"));
    }

    #[test]
    fn test_apt_v2_numberofrooms_empty_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "Flat",
                "numberOfRooms": ""
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "APT-V2001"));
    }

    #[test]
    fn test_apt_v2_numberofrooms_string_number() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "Flat",
                "numberOfRooms": "3"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_apt_v2_multiple_apartments() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Apartment".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Apartment",
                    "name": "Good",
                    "numberOfRooms": 2
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Apartment".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Apartment",
                    "name": "Bad"
                }),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "APT-V2001"));
    }

    #[test]
    fn test_apt_v2_both_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "Luxury Suite",
                "numberOfRooms": 5
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_apt_v2_name_missing_still_checks_rooms() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ApartmentSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "APT-V2001"));
    }
}
