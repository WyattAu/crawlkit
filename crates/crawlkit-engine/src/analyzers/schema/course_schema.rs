#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct CourseSchemaValidator;

impl Default for CourseSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CourseSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CourseSchemaValidator {
    fn name(&self) -> &str {
        "course-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Course" {
                continue;
            }
            let data = &sd.data;

            // COURSE001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "COURSE001".to_string(),
                    title: "Course schema missing name".to_string(),
                    description: "A Course structured data block is missing the required \
                                  \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the course title.".to_string(),
                });
            }

            // COURSE002: Missing provider
            if data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "COURSE002".to_string(),
                    title: "Course schema missing provider".to_string(),
                    description: "A Course structured data block is missing the \"provider\" \
                                  property. The provider identifies the organization offering \
                                  the course."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"provider\" with an Organization or Person object."
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
fn test_course_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COURSE001"));
}


    #[test]
fn test_course_missing_provider() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course",
            "name": "Rust 101"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COURSE002"));
}


    #[test]
fn test_course_all_present() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course",
            "name": "Rust 101",
            "provider": {"@type": "Organization", "name": "Acme U"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_course_missing_both() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COURSE001"));
    assert!(findings.iter().any(|f| f.code == "COURSE002"));
}


    #[test]
fn test_course_no_schema_no_findings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = CourseSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
