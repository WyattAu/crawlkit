#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct CouponSchemaValidator;

impl CouponSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    fn extract_coupon_schemas<'a>(ctx: &'a AnalysisContext<'a>) -> Vec<&'a serde_json::Value> {
        ctx.page
            .structured_data
            .iter()
            .filter(|sd| sd.r#type.as_deref() == Some("Coupon"))
            .map(|sd| &sd.data)
            .collect()
    }
}

impl Default for CouponSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CouponSchemaValidator {
    fn name(&self) -> &str {
        "coupon-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for data in Self::extract_coupon_schemas(ctx) {
            if data.get("validFrom").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "COUP001".to_string(),
                    title: "Coupon schema missing validFrom".to_string(),
                    description: "A Coupon schema was found but has no validFrom property. \
                                  The validFrom date tells search engines when the coupon \
                                  becomes active."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a \"validFrom\" property with an ISO 8601 date to \
                                     the Coupon schema."
                        .to_string(),
                });
            }

            let has_discount_percentage = data.get("discountPercentage").is_some();
            let has_discount_amount = data.get("discount").is_some();
            if !has_discount_percentage && !has_discount_amount {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "COUP002".to_string(),
                    title: "Coupon schema missing discount information".to_string(),
                    description: "A Coupon schema was found but has neither discountPercentage \
                                  nor discount. Search engines need at least one discount \
                                  value to display coupon information in search results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a \"discountPercentage\" or \"discount\" property to \
                                     the Coupon schema."
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
fn test_coupon_no_coupon_schema() {
    let page = make_page("https://example.com");
    assert!(CouponSchemaValidator::new().analyze(&make_ctx(&page, Some(200))).is_empty());
}


    #[test]
fn test_coupon_missing_valid_from() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Coupon".to_string()),
        data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "discountPercentage": "10%"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(CouponSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "COUP001"));
}


    #[test]
fn test_coupon_missing_discount() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Coupon".to_string()),
        data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "validFrom": "2025-06-01"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(CouponSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "COUP002"));
}


    #[test]
fn test_coupon_valid_coupon() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Coupon".to_string()),
        data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "validFrom": "2025-06-01", "discountPercentage": "10%"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(CouponSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_coupon_with_discount_amount() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Coupon".to_string()),
        data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "validFrom": "2025-06-01", "discount": "$5 off"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(!CouponSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "COUP002"));
}


    #[test]
fn test_coupon_missing_both() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Coupon".to_string()),
        data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let f = CouponSchemaValidator::new().analyze(&ctx);
    assert!(f.iter().any(|f| f.code == "COUP001"));
    assert!(f.iter().any(|f| f.code == "COUP002"));
}


    #[test]
fn test_coupon_non_coupon_schema() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(CouponSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_coupon_empty_coupon_data() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Coupon".to_string()),
        data: serde_json::json!({"@type": "Coupon"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let f = CouponSchemaValidator::new().analyze(&ctx);
    assert!(f.iter().any(|f| f.code == "COUP001"));
    assert!(f.iter().any(|f| f.code == "COUP002"));
}


    #[test]
fn test_coupon_multiple_coupons() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Coupon".to_string()),
            data: serde_json::json!({"@type": "Coupon", "name": "Sale 1", "validFrom": "2025-06-01", "discountPercentage": "10%"}),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Coupon".to_string()),
            data: serde_json::json!({"@type": "Coupon", "name": "Sale 2"}),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let f = CouponSchemaValidator::new().analyze(&ctx);
    assert!(f.iter().any(|f| f.code == "COUP001"));
    assert!(f.iter().any(|f| f.code == "COUP002"));
}


    #[test]
fn test_coupon_with_both_discount_types() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Coupon".to_string()),
        data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "validFrom": "2025-06-01", "discountPercentage": "10%", "discount": "$5 off"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(CouponSchemaValidator::new().analyze(&ctx).is_empty());
}


}
