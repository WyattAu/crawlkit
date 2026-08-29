#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ProductOfferValidator;

impl Default for ProductOfferValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductOfferValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ProductOfferValidator {
    fn name(&self) -> &str {
        "product-offer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Product") {
                continue;
            }
            let data = &sd.data;

            let offers = match data.get("offers") {
                None => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "PROFFER001".to_string(),
                        title: "Product missing offers".to_string(),
                        description: "A Product structured data block is missing the \"offers\" \
                                      property. Offers help search engines display price and \
                                      availability in rich results."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"offers\" with an Offer or array of Offer objects."
                            .to_string(),
                    });
                    continue;
                }
                Some(val) => val,
            };

            let offer_list: Vec<&serde_json::Value> = match offers {
                serde_json::Value::Array(arr) => arr.iter().collect(),
                other => vec![other],
            };

            for offer in offer_list {
                match offer.get("priceCurrency") {
                    None => {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "PROFFER002".to_string(),
                            title: "Product offer missing priceCurrency".to_string(),
                            description: "A Product Offer is missing the \"priceCurrency\" \
                                          property. Search engines require priceCurrency for \
                                          product rich results."
                                .to_string(),
                            url: url.clone(),
                            recommendation: "Add \"priceCurrency\" with an ISO 4217 currency \
                                             code (e.g., \"USD\", \"EUR\")."
                                .to_string(),
                        });
                    }
                    Some(cur) => {
                        if let Some(cur_str) = cur.as_str() {
                            if cur_str.len() != 3 || !cur_str.chars().all(|c| c.is_ascii_uppercase()) {
                                findings.push(Finding {
                                    severity: Severity::Warning,
                                    category: IssueCategory::Schema,
                                    code: "PROFFER003".to_string(),
                                    title: "Invalid priceCurrency value".to_string(),
                                    description: format!(
                                        "priceCurrency \"{cur_str}\" is not a valid ISO 4217 \
                                         code. It should be a 3-letter uppercase code."
                                    ),
                                    url: url.clone(),
                                    recommendation: "Use a valid ISO 4217 currency code (e.g., \
                                                     \"USD\", \"EUR\", \"GBP\")."
                                        .to_string(),
                                });
                            }
                        }
                    }
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
    fn test_missing_offers() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let findings = ProductOfferValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "PROFFER001"));
    }

    #[test]
    fn test_offer_missing_price_currency() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99"}}),
        }];
        let findings = ProductOfferValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "PROFFER002"));
    }

    #[test]
    fn test_valid_offer_with_currency() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "priceCurrency": "USD"}}),
        }];
        let findings = ProductOfferValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_invalid_price_currency_lowercase() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "priceCurrency": "usd"}}),
        }];
        let findings = ProductOfferValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "PROFFER003"));
    }

    #[test]
    fn test_invalid_price_currency_wrong_length() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "priceCurrency": "US"}}),
        }];
        let findings = ProductOfferValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "PROFFER003"));
    }

    #[test]
    fn test_array_of_offers_one_missing_currency() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": [
                {"@type": "Offer", "price": "9.99", "priceCurrency": "USD"},
                {"@type": "Offer", "price": "19.99"}
            ]}),
        }];
        let findings = ProductOfferValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "PROFFER002"));
    }

    #[test]
    fn test_non_product_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "Test"}),
        }];
        let findings = ProductOfferValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        let findings = ProductOfferValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_valid_eur_currency() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "priceCurrency": "EUR"}}),
        }];
        let findings = ProductOfferValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(ProductOfferValidator::new().name(), "product-offer");
    }
}
