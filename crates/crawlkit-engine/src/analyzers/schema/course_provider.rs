use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct CourseProviderValidator;

impl CourseProviderValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CourseProviderValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CourseProviderValidator {
    fn name(&self) -> &str {
        "course-provider"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Course") {
                continue;
            }
            let data = &sd.data;

            if let Some(provider) = data.get("provider") {
                if provider.get("name").is_none() {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "CPROV001".to_string(),
                        title: "Course provider missing name".to_string(),
                        description: "The \"provider\" object in Course schema is missing \
                                      \"name\"."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"name\" to the provider object to identify the \
                                         course provider."
                            .to_string(),
                    });
                }

                if provider.get("url").is_none() {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "CPROV002".to_string(),
                        title: "Course provider missing URL".to_string(),
                        description: "The \"provider\" object in Course schema is missing \"url\"."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"url\" to the provider object linking to the \
                                         provider's website."
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
fn test_cprov_missing_provider_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course",
            "name": "Rust Programming",
            "provider": {"@type": "Organization"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseProviderValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CPROV001"));
}


    #[test]
fn test_cprov_missing_provider_url() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course",
            "name": "Rust Programming",
            "provider": {"@type": "Organization", "name": "Udemy"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseProviderValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CPROV002"));
}


    #[test]
fn test_cprov_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course",
            "name": "Rust Programming",
            "provider": {
                "@type": "Organization",
                "name": "Udemy",
                "url": "https://udemy.com"
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseProviderValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_cprov_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = CourseProviderValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_cprov_non_course_ignored() {
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
    let findings = CourseProviderValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_cprov_no_provider() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course",
            "name": "Rust Programming"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseProviderValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_cprov_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course",
            "name": "Rust Programming",
            "provider": {"@type": "Organization"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseProviderValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 2);
}


    #[test]
fn test_cprov_provider_with_url_only() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Course".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Course",
            "name": "Rust Programming",
            "provider": {
                "@type": "Organization",
                "url": "https://udemy.com"
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = CourseProviderValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CPROV001"));
    assert!(!findings.iter().any(|f| f.code == "CPROV002"));
}


}
