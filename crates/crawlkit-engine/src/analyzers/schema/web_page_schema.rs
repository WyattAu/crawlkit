#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct WebPageSchemaValidator;

impl Default for WebPageSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl WebPageSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WebPageSchemaValidator {
    fn name(&self) -> &str {
        "webpage-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("WebPage") {
                continue;
            }
            let data = &sd.data;

            if data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WEBPG001".to_string(),
                    title: "WebPage schema missing name".to_string(),
                    description:
                        "A WebPage structured data block is missing the \"name\" property. \
                                  Search engines use the name to understand the page topic."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with a descriptive page title to the WebPage \
                                     schema."
                        .to_string(),
                });
            }

            if data.get("datePublished").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WEBPG002".to_string(),
                    title: "WebPage schema missing datePublished".to_string(),
                    description:
                        "A WebPage structured data block is missing the \"datePublished\" \
                                  property. This helps search engines assess content freshness."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"datePublished\" with an ISO 8601 date value."
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
    fn test_webpage_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
    }

    #[test]
    fn test_webpage_missing_date_published() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "name": "My Page"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "name": "My Page",
                "datePublished": "2024-01-01"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(!findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webpage_non_webpage_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webpage_multiple_webpages() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebPage".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebPage",
                    "name": "Page 1",
                    "datePublished": "2024-01-01"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebPage".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebPage"
                }),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_name_empty_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
    }

    #[test]
    fn test_webpage_name_only_no_date() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "name": "About Us"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_date_only_no_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "datePublished": "2024-06-15"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(!findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(findings.iter().any(|f| f.code == "WEBPG002"));
    }
}
