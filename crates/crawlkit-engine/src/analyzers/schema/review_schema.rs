use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ReviewSchemaValidator;

impl Default for ReviewSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ReviewSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ReviewSchemaValidator {
    fn name(&self) -> &str {
        "review-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let data = &sd.data;
            let schema_type = data.get("@type").and_then(|t| t.as_str());

            // Check AggregateRating
            if schema_type == Some("AggregateRating") || schema_type == Some("Product") {
                if let Some(rating) = data.get("aggregateRating") {
                    // REV001: Missing reviewCount or ratingCount
                    if rating.get("reviewCount").is_none() && rating.get("ratingCount").is_none() {
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Schema,
                            code: "REV001".to_string(),
                            title: "AggregateRating missing reviewCount".to_string(),
                            description: "AggregateRating schema is missing both \"reviewCount\" \
                                         and \"ratingCount\" properties."
                                .to_string(),
                            url: url.clone(),
                            recommendation: "Add \"reviewCount\" or \"ratingCount\" to the \
                                             AggregateRating schema."
                                .to_string(),
                        });
                    }

                    // REV002: ratingValue out of range
                    if let Some(value) = rating.get("ratingValue").and_then(|v| v.as_f64()) {
                        let best = rating
                            .get("bestRating")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(5.0);
                        if value > best || value < 0.0 {
                            findings.push(Finding {
                                severity: Severity::Error,
                                category: IssueCategory::Schema,
                                code: "REV002".to_string(),
                                title: "AggregateRating ratingValue out of range".to_string(),
                                description: format!(
                                    "ratingValue ({}) is outside the valid range (0 to {}).",
                                    value, best
                                ),
                                url: url.clone(),
                                recommendation: "Ensure ratingValue is between 0 and bestRating."
                                    .to_string(),
                            });
                        }
                    }
                }
            }

            // Check Review
            if schema_type == Some("Review") {
                // REV003: Missing author
                if data.get("author").is_none() {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "REV003".to_string(),
                        title: "Review schema missing author".to_string(),
                        description: "A Review structured data block is missing the \"author\" \
                                     property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"author\" with a Person or Organization object."
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
fn test_review_missing_review_count() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "aggregateRating": {"@type": "AggregateRating", "ratingValue": 4.5}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(ReviewSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "REV001"));
}


    #[test]
fn test_review_rating_out_of_range() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "aggregateRating": {"@type": "AggregateRating", "ratingValue": 6.0, "bestRating": 5, "reviewCount": 100}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(ReviewSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "REV002"));
}


    #[test]
fn test_review_missing_author() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Review".to_string()),
        data: serde_json::json!({"@type": "Review", "reviewBody": "Great product!"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(ReviewSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "REV003"));
}


    #[test]
fn test_review_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("AggregateRating".to_string()),
        data: serde_json::json!({"@type": "AggregateRating", "ratingValue": 4.5, "bestRating": 5, "reviewCount": 100}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = ReviewSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
