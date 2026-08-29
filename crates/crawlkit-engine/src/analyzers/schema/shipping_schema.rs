use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ShippingSchemaValidator;

impl ShippingSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    fn extract_product_schemas<'a>(ctx: &'a AnalysisContext<'a>) -> Vec<&'a serde_json::Value> {
        ctx.page
            .structured_data
            .iter()
            .filter(|sd| sd.r#type.as_deref() == Some("Product"))
            .map(|sd| &sd.data)
            .collect()
    }

    fn has_offers(data: &serde_json::Value) -> bool {
        match data.get("offers") {
            None => false,
            Some(v) => {
                if let Some(arr) = v.as_array() {
                    !arr.is_empty()
                } else if let Some(obj) = v.as_object() {
                    obj.get("@type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == "Offer")
                        .unwrap_or(false)
                        || obj.get("price").is_some()
                } else {
                    false
                }
            }
        }
    }

    fn has_shipping_details(data: &serde_json::Value) -> bool {
        if data.get("hasShippingDetails").is_some() {
            return true;
        }
        if let Some(offers) = data.get("offers").and_then(|v| v.as_array()) {
            return offers
                .iter()
                .any(|o| o.get("hasShippingDetails").is_some());
        }
        if let Some(offers) = data.get("offers") {
            if let Some(obj) = offers.as_object() {
                return obj.get("hasShippingDetails").is_some();
            }
        }
        false
    }
}

impl Default for ShippingSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ShippingSchemaValidator {
    fn name(&self) -> &str {
        "shipping-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for data in Self::extract_product_schemas(ctx) {
            if Self::has_offers(data) && !Self::has_shipping_details(data) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SHIP001".to_string(),
                    title: "Product has offers but no ShippingDetails".to_string(),
                    description: "A Product schema has offers but no hasShippingDetails property. \
                                  Shipping details help search engines display delivery information \
                                  in product search results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a \"hasShippingDetails\" property to the Product or Offer \
                                     schema with shipping cost, delivery time, and destination."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Offer Availability Analyzer
// ---------------------------------------------------------------------------


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
fn test_shipping_no_product_schema() {
    let page = make_page("https://example.com");
    assert!(ShippingSchemaValidator::new().analyze(&make_ctx(&page, Some(200))).is_empty());
}


    #[test]
fn test_shipping_product_with_offers_no_shipping() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "priceCurrency": "USD"}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(ShippingSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SHIP001"));
}


    #[test]
fn test_shipping_product_with_shipping_details() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "hasShippingDetails": {"@type": "ShippingDetails"}}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(!ShippingSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SHIP001"));
}


    #[test]
fn test_shipping_product_no_offers() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(ShippingSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_shipping_product_empty_offers() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": []}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(ShippingSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_shipping_offers_array_with_shipping() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": [{"@type": "Offer", "price": "9.99", "hasShippingDetails": {"@type": "ShippingDetails"}}]}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(!ShippingSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SHIP001"));
}


    #[test]
fn test_shipping_top_level_shipping() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99"}, "hasShippingDetails": {"@type": "ShippingDetails"}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(!ShippingSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SHIP001"));
}


    #[test]
fn test_shipping_non_product_schema() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({"@type": "Article", "headline": "News"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(ShippingSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_shipping_product_url_reference() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "priceCurrency": "USD"}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = ShippingSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SHIP001"));
}


}
