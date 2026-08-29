#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct FaqPageEntityValidator;

impl Default for FaqPageEntityValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl FaqPageEntityValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FaqPageEntityValidator {
    fn name(&self) -> &str {
        "faq-page-entity"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("FAQPage") {
                continue;
            }
            let data = &sd.data;

            let main_entity = match data.get("mainEntity") {
                Some(e) => e,
                None => continue,
            };

            let count = match main_entity.as_array() {
                Some(arr) => arr.len(),
                None => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "FAQPAGE001".to_string(),
                        title: "FAQPage mainEntity is not an array".to_string(),
                        description: "The mainEntity property is not a JSON array. FAQPage \
                                      requires mainEntity to be an array of Question objects."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Set mainEntity to an array of Question objects."
                            .to_string(),
                    });
                    continue;
                }
            };

            if count < 2 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "FAQPAGE001".to_string(),
                    title: "FAQPage has fewer than 2 mainEntity items".to_string(),
                    description: format!(
                        "FAQPage mainEntity contains only {count} item(s). Google typically \
                         requires at least 2 question-answer pairs for FAQ rich results."
                    ),
                    url: url.clone(),
                    recommendation: "Add at least 2 Question objects to the mainEntity array."
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
    fn test_faq_missing_main_entity() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({"@type": "FAQPage"}),
        }];
        // No mainEntity at all - this analyzer skips when mainEntity is absent
        assert!(FaqPageEntityValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_faq_one_question() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({"@type": "FAQPage", "mainEntity": [
                {"@type": "Question", "name": "Q1", "acceptedAnswer": {"@type": "Answer", "text": "A1"}}
            ]}),
        }];
        assert!(FaqPageEntityValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "FAQPAGE001"));
    }

    #[test]
    fn test_faq_two_questions() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({"@type": "FAQPage", "mainEntity": [
                {"@type": "Question", "name": "Q1"},
                {"@type": "Question", "name": "Q2"}
            ]}),
        }];
        assert!(FaqPageEntityValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_faq_empty_array() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({"@type": "FAQPage", "mainEntity": []}),
        }];
        assert!(FaqPageEntityValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "FAQPAGE001"));
    }

    #[test]
    fn test_faq_non_array_main_entity() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({"@type": "FAQPage", "mainEntity": "not an array"}),
        }];
        assert!(FaqPageEntityValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "FAQPAGE001"));
    }

    #[test]
    fn test_non_faq_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        assert!(FaqPageEntityValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(FaqPageEntityValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(FaqPageEntityValidator::new().name(), "faq-page-entity");
    }

    #[test]
    fn test_faq_three_questions() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({"@type": "FAQPage", "mainEntity": [
                {"@type": "Question", "name": "Q1"},
                {"@type": "Question", "name": "Q2"},
                {"@type": "Question", "name": "Q3"}
            ]}),
        }];
        assert!(FaqPageEntityValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_default() {
        let _ = FaqPageEntityValidator::default();
    }
}
