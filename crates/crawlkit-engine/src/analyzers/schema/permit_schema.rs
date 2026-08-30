#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct PermitSchemaValidator;

impl Default for PermitSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PermitSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PermitSchemaValidator {
    fn name(&self) -> &str {
        "permit-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Permit") {
                continue;
            }
            let data = &sd.data;

            if data
                .get("permitNumber")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PERMIT001".to_string(),
                    title: "Permit schema missing permitNumber".to_string(),
                    description: "A Permit structured data block is missing the \"permitNumber\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"permitNumber\" with the permit identification number."
                        .to_string(),
                });
            }

            if data.get("issuedBy").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PERMIT002".to_string(),
                    title: "Permit schema missing issuedBy".to_string(),
                    description: "A Permit structured data block is missing the \"issuedBy\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"issuedBy\" with the issuing authority.".to_string(),
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
    fn test_permit_missing_permitnumber() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERMIT001"));
    }

    #[test]
    fn test_permit_missing_issuedby() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit",
                "permitNumber": "P-12345"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERMIT002"));
    }

    #[test]
    fn test_permit_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit",
                "permitNumber": "P-12345",
                "issuedBy": {"@type": "GovernmentOrganization", "name": "City Hall"}
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_permit_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_permit_non_permit_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_permit_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_permit_number_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit",
                "permitNumber": ""
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERMIT001"));
    }

    #[test]
    fn test_permit_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Permit".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Permit",
                    "permitNumber": "P-1",
                    "issuedBy": {"@type": "GovernmentOrganization"}
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Permit".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Permit"
                }),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERMIT001"));
        assert!(findings.iter().any(|f| f.code == "PERMIT002"));
    }
}
