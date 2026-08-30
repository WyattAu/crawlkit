#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct PersonJobTitleValidator;

impl PersonJobTitleValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PersonJobTitleValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PersonJobTitleValidator {
    fn name(&self) -> &str {
        "person-job-title"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Person") {
                continue;
            }
            let data = &sd.data;

            if data.get("jobTitle").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PJOB001".to_string(),
                    title: "Person missing jobTitle".to_string(),
                    description: "A Person structured data block is missing the \"jobTitle\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"jobTitle\" with the person's professional title."
                        .to_string(),
                });
            }

            if data.get("worksFor").is_none() && data.get("memberOf").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PJOB002".to_string(),
                    title: "Person missing worksFor".to_string(),
                    description: "A Person structured data block is missing the \"worksFor\" or \
                                  \"memberOf\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"worksFor\" with an Organization object to indicate the \
                                     person's employer."
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
    fn test_pjob_missing_job_title() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PJOB001"));
    }

    #[test]
    fn test_pjob_missing_works_for() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe",
                "jobTitle": "Engineer"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PJOB002"));
    }

    #[test]
    fn test_pjob_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe",
                "jobTitle": "Engineer",
                "worksFor": {"@type": "Organization", "name": "Acme Corp"}
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pjob_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pjob_non_person_ignored() {
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
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pjob_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_pjob_with_member_of() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe",
                "jobTitle": "Engineer",
                "memberOf": {"@type": "Organization", "name": "Acme Corp"}
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pjob_with_job_title_no_works_for() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe",
                "jobTitle": "Engineer"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PJOB001"));
        assert!(findings.iter().any(|f| f.code == "PJOB002"));
    }
}
