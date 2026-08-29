#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct FaqSchemaValidator;

impl Default for FaqSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl FaqSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FaqSchemaValidator {
    fn name(&self) -> &str {
        "faq-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("FAQPage") {
                continue;
            }
            let data = &sd.data;

            // FAQ001: Missing mainEntity
            let Some(main_entity) = data.get("mainEntity") else {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "FAQ001".to_string(),
                    title: "FAQPage schema missing mainEntity".to_string(),
                    description: "An FAQPage structured data block is missing the required \
                                  \"mainEntity\" property. Without mainEntity, search engines \
                                  cannot extract question-answer pairs."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"mainEntity\" with an array of Question objects."
                        .to_string(),
                });
                continue;
            };

            // FAQ002: mainEntity has fewer than 2 questions
            let questions = main_entity.as_array();
            let question_count = questions.map_or(0, |arr| arr.len());
            if question_count < 2 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "FAQ002".to_string(),
                    title: "FAQPage schema has fewer than 2 questions".to_string(),
                    description: format!(
                        "FAQPage mainEntity contains only {} question(s). FAQ rich results \
                         typically require at least 2 question-answer pairs.",
                        question_count
                    ),
                    url: url.clone(),
                    recommendation: "Add at least 2 Question objects to the mainEntity array."
                        .to_string(),
                });
            }

            // FAQ003: Questions missing acceptedAnswer
            if let Some(arr) = questions {
                for (i, q) in arr.iter().enumerate() {
                    if q.get("acceptedAnswer").is_none() {
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Schema,
                            code: "FAQ003".to_string(),
                            title: "FAQPage question missing acceptedAnswer".to_string(),
                            description: format!(
                                "Question at position {} in FAQPage mainEntity is missing the \
                                 required \"acceptedAnswer\" property.",
                                i + 1
                            ),
                            url: url.clone(),
                            recommendation: "Add \"acceptedAnswer\" with an Answer object to each \
                                             Question in the FAQPage schema."
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
fn test_faq_missing_main_entity() {
    let mut page = make_page("https://example.com/faq");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("FAQPage".to_string()),
        data: serde_json::json!({"@type": "FAQPage"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(FaqSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "FAQ001"));
}


    #[test]
fn test_faq_too_few_questions() {
    let mut page = make_page("https://example.com/faq");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("FAQPage".to_string()),
        data: serde_json::json!({
            "@type": "FAQPage",
            "mainEntity": [{"@type": "Question", "name": "Q1", "acceptedAnswer": {"@type": "Answer", "text": "A1"}}]
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(FaqSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "FAQ002"));
}


    #[test]
fn test_faq_question_missing_accepted_answer() {
    let mut page = make_page("https://example.com/faq");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("FAQPage".to_string()),
        data: serde_json::json!({
            "@type": "FAQPage",
            "mainEntity": [
                {"@type": "Question", "name": "Q1", "acceptedAnswer": {"@type": "Answer", "text": "A1"}},
                {"@type": "Question", "name": "Q2"}
            ]
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(FaqSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "FAQ003"));
}


    #[test]
fn test_faq_valid() {
    let mut page = make_page("https://example.com/faq");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("FAQPage".to_string()),
        data: serde_json::json!({
            "@type": "FAQPage",
            "mainEntity": [
                {"@type": "Question", "name": "Q1", "acceptedAnswer": {"@type": "Answer", "text": "A1"}},
                {"@type": "Question", "name": "Q2", "acceptedAnswer": {"@type": "Answer", "text": "A2"}}
            ]
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(FaqSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_faq_non_faq_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({"@type": "Article", "headline": "News"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(FaqSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_faq_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    assert!(FaqSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_faq_main_entity_not_array() {
    let mut page = make_page("https://example.com/faq");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("FAQPage".to_string()),
        data: serde_json::json!({
            "@type": "FAQPage",
            "mainEntity": "not an array"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = FaqSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "FAQ002"));
}


    #[test]
fn test_faq_empty_main_entity_array() {
    let mut page = make_page("https://example.com/faq");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("FAQPage".to_string()),
        data: serde_json::json!({
            "@type": "FAQPage",
            "mainEntity": []
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(FaqSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "FAQ002"));
}


    #[test]
fn test_faq_multiple_questions_missing_answers() {
    let mut page = make_page("https://example.com/faq");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("FAQPage".to_string()),
        data: serde_json::json!({
            "@type": "FAQPage",
            "mainEntity": [
                {"@type": "Question", "name": "Q1"},
                {"@type": "Question", "name": "Q2"}
            ]
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = FaqSchemaValidator::new().analyze(&ctx);
    let faq003_count = findings.iter().filter(|f| f.code == "FAQ003").count();
    assert_eq!(faq003_count, 2);
}


}
