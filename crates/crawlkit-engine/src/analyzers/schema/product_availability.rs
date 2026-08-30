#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ProductAvailabilityValidator;

impl Default for ProductAvailabilityValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductAvailabilityValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ProductAvailabilityValidator {
    fn name(&self) -> &str {
        "product-availability"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Product" {
                continue;
            }
            let data = &sd.data;

            let has_offers = data.get("offers").is_some();
            if !has_offers {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PRODAVAIL001".to_string(),
                    title: "Product schema missing offers".to_string(),
                    description: "A Product structured data block has no \"offers\" property. \
                                  Without offers, search engines cannot display price and \
                                  availability in rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add an \"offers\" property with price and availability \
                                     information."
                        .to_string(),
                });
                continue;
            }

            if let Some(offers) = data.get("offers") {
                let availability_missing = match offers {
                    serde_json::Value::Array(arr) => {
                        arr.iter().any(|o| o.get("availability").is_none())
                    }
                    serde_json::Value::Object(_) => offers.get("availability").is_none(),
                    _ => true,
                };
                if availability_missing {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "PRODAVAIL001".to_string(),
                        title: "Product offers missing availability".to_string(),
                        description: "A Product structured data block has offers without \
                                      the \"availability\" property. Availability helps \
                                      search engines show stock status in results."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"availability\" with a Schema.org availability \
                                         value (e.g., InStock, OutOfStock)."
                            .to_string(),
                    });
                }
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
    use crate::parser::{ParsedPage, StructuredData};

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
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

    fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
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
    fn test_product_missing_offers() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        assert!(ProductAvailabilityValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "PRODAVAIL001"));
    }

    #[test]
    fn test_product_offer_missing_availability() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "10"}}),
        }];
        assert!(ProductAvailabilityValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "PRODAVAIL001"));
    }

    #[test]
    fn test_product_valid_availability() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "InStock"}}),
        }];
        assert!(ProductAvailabilityValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_non_product_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        assert!(ProductAvailabilityValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_product_array_offers_missing_availability() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": [
                {"@type": "Offer", "price": "10"},
                {"@type": "Offer", "availability": "InStock"}
            ]}),
        }];
        assert!(ProductAvailabilityValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "PRODAVAIL001"));
    }

    #[test]
    fn test_product_array_offers_all_valid() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": [
                {"@type": "Offer", "availability": "InStock"},
                {"@type": "Offer", "availability": "OutOfStock"}
            ]}),
        }];
        assert!(ProductAvailabilityValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(ProductAvailabilityValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(
            ProductAvailabilityValidator::new().name(),
            "product-availability"
        );
    }

    #[test]
    fn test_product_with_string_offers() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": "not an object"}),
        }];
        assert!(ProductAvailabilityValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "PRODAVAIL001"));
    }

    #[test]
    fn test_default() {
        let _ = ProductAvailabilityValidator::default();
    }
}
