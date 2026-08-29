use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ServiceSchemaValidator;

impl Default for ServiceSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ServiceSchemaValidator {
    fn name(&self) -> &str {
        "service-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Service") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SVC001".to_string(),
                    title: "Service schema missing name".to_string(),
                    description: "A Service structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the service name to the Service schema."
                        .to_string(),
                });
            }

            if data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SVC002".to_string(),
                    title: "Service schema missing provider".to_string(),
                    description: "A Service structured data block is missing the \"provider\" \
                                  property. The provider identifies the organization offering the \
                                  service."
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
fn test_service_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Service".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Service"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SVC001"));
}


    #[test]
fn test_service_missing_provider() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Service".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Service",
            "name": "Web Hosting"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SVC002"));
}


    #[test]
fn test_service_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Service".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Service",
            "name": "Web Hosting",
            "provider": {"@type": "Organization", "name": "Acme Corp"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SVC001"));
    assert!(!findings.iter().any(|f| f.code == "SVC002"));
}


    #[test]
fn test_service_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_service_non_service_type_ignored() {
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
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_service_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Service".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Service"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 2);
    assert!(findings.iter().any(|f| f.code == "SVC001"));
    assert!(findings.iter().any(|f| f.code == "SVC002"));
}


    #[test]
fn test_service_name_empty_string() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Service".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Service",
            "name": ""
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SVC001"));
}


    #[test]
fn test_service_provider_is_string() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Service".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Service",
            "name": "Cloud Storage",
            "provider": "Acme Corp"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SVC001"));
    assert!(!findings.iter().any(|f| f.code == "SVC002"));
}


    #[test]
fn test_service_multiple_services() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Service".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Service",
                "name": "Valid Service",
                "provider": {"@type": "Organization", "name": "Corp"}
            }),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Service".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Service"
            }),
        },
    ];
    let ctx = make_ctx(&page, None);
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SVC001"));
    assert!(findings.iter().any(|f| f.code == "SVC002"));
}


    #[test]
fn test_service_name_only_no_provider() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Service".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Service",
            "name": "SEO Audit"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ServiceSchemaValidator::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SVC001"));
    assert!(findings.iter().any(|f| f.code == "SVC002"));
}


}
