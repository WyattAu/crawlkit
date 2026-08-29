#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct OfferAvailabilityAnalyzer;

impl OfferAvailabilityAnalyzer {
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

    fn get_schema_availability(data: &serde_json::Value) -> Option<String> {
        if let Some(offers) = data.get("offers") {
            if let Some(arr) = offers.as_array() {
                if let Some(first) = arr.first() {
                    return first
                        .get("availability")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            } else if let Some(obj) = offers.as_object() {
                return obj
                    .get("availability")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        None
    }

    fn page_says_out_of_stock(body: &str) -> bool {
        let lower = body.to_lowercase();
        [
            "out of stock",
            "out-of-stock",
            "sold out",
            "currently unavailable",
            "not available",
            "no longer available",
            "temporarily out of stock",
        ]
        .iter()
        .any(|&ind| lower.contains(ind))
    }

    fn page_says_in_stock(body: &str) -> bool {
        let lower = body.to_lowercase();
        [
            "add to cart",
            "add to bag",
            "buy now",
            "in stock",
            "ships in",
            "delivery",
            "available",
        ]
        .iter()
        .any(|&ind| lower.contains(ind))
    }
}

impl Default for OfferAvailabilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for OfferAvailabilityAnalyzer {
    fn name(&self) -> &str {
        "offer-availability"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body = match ctx.body {
            Some(b) => b,
            None => return findings,
        };

        for data in Self::extract_product_schemas(ctx) {
            if let Some(availability) = Self::get_schema_availability(data) {
                let lower_avail = availability.to_lowercase();

                if lower_avail.contains("instock") && Self::page_says_out_of_stock(body) {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "AVAIL001".to_string(),
                        title: "Schema says InStock but page says out of stock".to_string(),
                        description: format!(
                            "Product schema availability is \"{availability}\" but the page \
                             text contains out-of-stock indicators."
                        ),
                        url: url.clone(),
                        recommendation: "Update the schema availability to match the actual \
                                         page content. Mismatched availability confuses search \
                                         engines and users."
                            .to_string(),
                    });
                }

                if lower_avail.contains("outofstock") && Self::page_says_in_stock(body) {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "AVAIL002".to_string(),
                        title: "Schema says OutOfStock but page says in stock".to_string(),
                        description: format!(
                            "Product schema availability is \"{availability}\" but the page \
                             text contains in-stock indicators."
                        ),
                        url: url.clone(),
                        recommendation: "Update the schema availability to match the actual \
                                         page content. Mismatched availability confuses search \
                                         engines and users."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Coupon Schema Validator
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
fn test_availability_no_product() {
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Hello</body></html>");
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_availability_in_stock_schema_out_of_stock_page() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
    }];
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>This product is out of stock</body></html>");
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL001"));
}


    #[test]
fn test_availability_out_of_stock_schema_in_stock_page() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/OutOfStock"}}),
    }];
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Add to cart now!</body></html>");
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL002"));
}


    #[test]
fn test_availability_consistent_in_stock() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
    }];
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>In stock, add to cart</body></html>");
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_availability_consistent_out_of_stock() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/OutOfStock"}}),
    }];
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Sorry, this is out of stock</body></html>");
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_availability_no_availability_in_schema() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99"}}),
    }];
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>This product is out of stock</body></html>");
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_availability_sold_out_indicator() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
    }];
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Sold out! Check back later.</body></html>");
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL001"));
}


    #[test]
fn test_availability_buy_now_indicator() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/OutOfStock"}}),
    }];
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Buy now! Free shipping.</body></html>");
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL002"));
}


    #[test]
fn test_availability_offers_array_first_item() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": [{"@type": "Offer", "availability": "https://schema.org/InStock"}]}),
    }];
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>This product is out of stock</body></html>");
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL001"));
}


    #[test]
fn test_availability_no_body() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_availability_multiple_products() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "A", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "B", "offers": {"@type": "Offer", "availability": "https://schema.org/OutOfStock"}}),
        },
    ];
    let ctx = make_ctx_with_body(&page, Some(200), "<html><body>This product is out of stock but also buy now</body></html>");
    let f = OfferAvailabilityAnalyzer::new().analyze(&ctx);
    assert!(f.iter().any(|f| f.code == "AVAIL001"));
    assert!(f.iter().any(|f| f.code == "AVAIL002"));
}


}
