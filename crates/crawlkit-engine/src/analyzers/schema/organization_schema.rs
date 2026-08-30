#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct OrganizationSchemaValidator;

impl Default for OrganizationSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganizationSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for OrganizationSchemaValidator {
    fn name(&self) -> &str {
        "organization-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Organization" {
                continue;
            }
            let data = &sd.data;

            // ORG001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "ORG001".to_string(),
                    title: "Organization schema missing name".to_string(),
                    description: "An Organization structured data block is missing the required \
                                  \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the organization's official name."
                        .to_string(),
                });
            }

            // ORG002: Missing url
            if data.get("url").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ORG002".to_string(),
                    title: "Organization schema missing url".to_string(),
                    description: "An Organization structured data block is missing the \"url\" \
                                  property. This helps search engines verify the organization."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"url\" with the organization's official website URL."
                        .to_string(),
                });
            }

            // ORG003: Missing logo
            if data.get("logo").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ORG003".to_string(),
                    title: "Organization schema missing logo".to_string(),
                    description: "An Organization structured data block is missing the \"logo\" \
                                  property. Logos are used in Knowledge Graph results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"logo\" with a URL to the organization's logo image."
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
    fn test_org_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "url": "https://example.com"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ORG001"));
    }

    #[test]
    fn test_org_missing_url() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ORG002"));
    }

    #[test]
    fn test_org_missing_logo() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp",
                "url": "https://example.com"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ORG003"));
    }

    #[test]
    fn test_org_all_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp",
                "url": "https://example.com",
                "logo": "https://example.com/logo.png"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_org_missing_all() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ORG001"));
        assert!(findings.iter().any(|f| f.code == "ORG002"));
        assert!(findings.iter().any(|f| f.code == "ORG003"));
    }

    #[test]
    fn test_org_non_org_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_org_no_schema_no_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}
