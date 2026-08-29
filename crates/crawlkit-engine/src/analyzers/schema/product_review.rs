use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ProductReviewValidator;

impl ProductReviewValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProductReviewValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ProductReviewValidator {
    fn name(&self) -> &str {
        "product-review"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Product") {
                continue;
            }
            let data = &sd.data;

            if let Some(reviews) = data.get("review") {
                let review_iter: Vec<&serde_json::Value> = if let Some(arr) = reviews.as_array() {
                    arr.iter().collect()
                } else {
                    vec![reviews]
                };

                for (i, review) in review_iter.iter().enumerate() {
                    if review.get("reviewRating").is_none() {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "PREV001".to_string(),
                            title: "Product review missing reviewRating".to_string(),
                            description: format!(
                                "Review #{i} in Product schema is missing the \"reviewRating\" \
                                 property."
                            ),
                            url: url.clone(),
                            recommendation: "Add \"reviewRating\" with a Rating object to each \
                                             review."
                                .to_string(),
                        });
                    }

                    if review.get("author").is_none() {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "PREV002".to_string(),
                            title: "Product review missing author".to_string(),
                            description: format!(
                                "Review #{i} in Product schema is missing the \"author\" property."
                            ),
                            url: url.clone(),
                            recommendation: "Add \"author\" with a Person or Organization object \
                                             to each review."
                                .to_string(),
                        });
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
fn test_prev_missing_review_rating() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget",
            "review": {
                "@type": "Review",
                "author": {"@type": "Person", "name": "John"}
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ProductReviewValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PREV001"));
}


    #[test]
fn test_prev_missing_author() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget",
            "review": {
                "@type": "Review",
                "reviewRating": {"@type": "Rating", "ratingValue": 5}
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ProductReviewValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PREV002"));
}


    #[test]
fn test_prev_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget",
            "review": {
                "@type": "Review"
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ProductReviewValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PREV001"));
    assert!(findings.iter().any(|f| f.code == "PREV002"));
}


    #[test]
fn test_prev_valid_review() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget",
            "review": {
                "@type": "Review",
                "author": {"@type": "Person", "name": "John"},
                "reviewRating": {"@type": "Rating", "ratingValue": 5}
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ProductReviewValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_prev_no_reviews() {
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
    let findings = ProductReviewValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_prev_multiple_reviews() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget",
            "review": [
                {"@type": "Review"},
                {"@type": "Review"}
            ]
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ProductReviewValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 4);
}


    #[test]
fn test_prev_non_product_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": "Test"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ProductReviewValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_prev_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = ProductReviewValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
