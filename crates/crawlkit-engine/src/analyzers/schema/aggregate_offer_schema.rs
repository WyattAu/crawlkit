use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct AggregateOfferSchemaValidator;

impl Default for AggregateOfferSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregateOfferSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AggregateOfferSchemaValidator {
    fn name(&self) -> &str {
        "aggregate-offer-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("AggregateOffer") {
                continue;
            }
            let data = &sd.data;

            if data.get("lowPrice").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "AGGOFFER001".to_string(),
                    title: "AggregateOffer schema missing lowPrice".to_string(),
                    description: "An AggregateOffer structured data block is missing the required \
                                  \"lowPrice\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"lowPrice\" with the lowest price in the range."
                        .to_string(),
                });
            }

            if data.get("priceCurrency").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "AGGOFFER002".to_string(),
                    title: "AggregateOffer schema missing priceCurrency".to_string(),
                    description: "An AggregateOffer structured data block is missing the \
                                  \"priceCurrency\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"priceCurrency\" with an ISO 4217 currency code."
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
fn test_aggregate_offer_missing_low_price() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("AggregateOffer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "AggregateOffer",
            "priceCurrency": "USD"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "AGGOFFER001"));
}


    #[test]
fn test_aggregate_offer_missing_price_currency() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("AggregateOffer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "AggregateOffer",
            "lowPrice": 9.99
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "AGGOFFER002"));
}


    #[test]
fn test_aggregate_offer_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("AggregateOffer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "AggregateOffer",
            "lowPrice": 9.99,
            "priceCurrency": "USD"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_aggregate_offer_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_aggregate_offer_non_aggregate_offer_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer",
            "price": 10
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_aggregate_offer_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("AggregateOffer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "AggregateOffer"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 2);
    assert!(findings.iter().any(|f| f.code == "AGGOFFER001"));
    assert!(findings.iter().any(|f| f.code == "AGGOFFER002"));
}


    #[test]
fn test_aggregate_offer_low_price_zero() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("AggregateOffer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "AggregateOffer",
            "lowPrice": 0,
            "priceCurrency": "USD"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_aggregate_offer_with_high_price() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("AggregateOffer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "AggregateOffer",
            "lowPrice": 9.99,
            "highPrice": 99.99,
            "priceCurrency": "EUR"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_aggregate_offer_multiple_aggregate_offers() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateOffer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "AggregateOffer",
                "lowPrice": 5,
                "priceCurrency": "USD"
            }),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateOffer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "AggregateOffer"
            }),
        },
    ];
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "AGGOFFER001"));
    assert!(findings.iter().any(|f| f.code == "AGGOFFER002"));
}


    #[test]
fn test_aggregate_offer_string_low_price() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("AggregateOffer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "AggregateOffer",
            "lowPrice": "9.99",
            "priceCurrency": "USD"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
