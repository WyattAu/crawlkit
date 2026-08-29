use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct TripSchemaValidator;

impl Default for TripSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TripSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TripSchemaValidator {
    fn name(&self) -> &str {
        "trip-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Trip") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "TRIP001".to_string(),
                    title: "Trip schema missing name".to_string(),
                    description: "A Trip structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the trip name or title."
                        .to_string(),
                });
            }

            if data.get("itinerary").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "TRIP002".to_string(),
                    title: "Trip schema missing itinerary".to_string(),
                    description: "A Trip structured data block is missing the \"itinerary\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"itinerary\" with the trip itinerary details."
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

    fn make_ctx_with_body<'a>(
        page: &'a crate::parser::ParsedPage,
        status: Option<u16>,
        body: &'a str,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: Some(body),
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
fn test_trip_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Trip".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Trip"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TripSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TRIP001"));
}


    #[test]
fn test_trip_missing_itinerary() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Trip".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Trip",
            "name": "Italy Vacation"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TripSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TRIP002"));
}


    #[test]
fn test_trip_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Trip".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Trip",
            "name": "Italy Vacation",
            "itinerary": {"@type": "ItemList", "numberOfItems": 5}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TripSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_trip_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = TripSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_trip_non_trip_type_ignored() {
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
    let findings = TripSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_trip_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Trip".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Trip"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TripSchemaValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 2);
}


    #[test]
fn test_trip_name_empty() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Trip".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Trip",
            "name": ""
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TripSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TRIP001"));
}


    #[test]
fn test_trip_multiple() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Trip".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Trip",
                "name": "Good Trip",
                "itinerary": {"@type": "ItemList"}
            }),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Trip".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Trip"
            }),
        },
    ];
    let ctx = make_ctx(&page, None);
    let findings = TripSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TRIP001"));
    assert!(findings.iter().any(|f| f.code == "TRIP002"));
}


}
