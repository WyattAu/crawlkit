use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct OfferSchemaValidator;

impl Default for OfferSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OfferSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for OfferSchemaValidator {
    fn name(&self) -> &str {
        "offer-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Offer") {
                continue;
            }
            let data = &sd.data;

            if data.get("price").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "OFFER001".to_string(),
                    title: "Offer schema missing price".to_string(),
                    description: "An Offer structured data block is missing the \"price\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"price\" with the item price."
                        .to_string(),
                });
            }

            if data.get("priceCurrency").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "OFFER002".to_string(),
                    title: "Offer schema missing priceCurrency".to_string(),
                    description: "An Offer structured data block is missing the \"priceCurrency\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"priceCurrency\" with an ISO 4217 currency code (e.g., \
                                     \"USD\")."
                        .to_string(),
                });
            }

            if data.get("availability").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "OFFER003".to_string(),
                    title: "Offer schema missing availability".to_string(),
                    description: "An Offer structured data block is missing the \"availability\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"availability\" with a schema.org Availability value \
                                     (e.g., \"https://schema.org/InStock\")."
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
fn test_offer_missing_price() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer",
            "priceCurrency": "USD",
            "availability": "https://schema.org/InStock"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OFFER001"));
}


    #[test]
fn test_offer_missing_price_currency() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer",
            "price": 29.99,
            "availability": "https://schema.org/InStock"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OFFER002"));
}


    #[test]
fn test_offer_missing_availability() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer",
            "price": 29.99,
            "priceCurrency": "USD"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OFFER003"));
}


    #[test]
fn test_offer_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer",
            "price": 29.99,
            "priceCurrency": "USD",
            "availability": "https://schema.org/InStock"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_offer_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_offer_non_offer_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_offer_all_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 3);
    assert!(findings.iter().any(|f| f.code == "OFFER001"));
    assert!(findings.iter().any(|f| f.code == "OFFER002"));
    assert!(findings.iter().any(|f| f.code == "OFFER003"));
}


    #[test]
fn test_offer_price_zero() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer",
            "price": 0,
            "priceCurrency": "USD",
            "availability": "https://schema.org/InStock"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_offer_price_string() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer",
            "price": "29.99",
            "priceCurrency": "USD",
            "availability": "https://schema.org/InStock"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_offer_price_only_no_currency_no_availability() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer",
            "price": 19.99
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OfferSchemaValidator::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "OFFER001"));
    assert!(findings.iter().any(|f| f.code == "OFFER002"));
    assert!(findings.iter().any(|f| f.code == "OFFER003"));
}


}
