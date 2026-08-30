#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct InvoiceSchemaValidator;

impl Default for InvoiceSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl InvoiceSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for InvoiceSchemaValidator {
    fn name(&self) -> &str {
        "invoice-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Invoice") {
                continue;
            }
            let data = &sd.data;

            if data
                .get("accountId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "INV001".to_string(),
                    title: "Invoice schema missing accountId".to_string(),
                    description: "An Invoice structured data block is missing the \"accountId\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"accountId\" with the account identifier.".to_string(),
                });
            }

            if data
                .get("dueDate")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "INV002".to_string(),
                    title: "Invoice schema missing dueDate".to_string(),
                    description: "An Invoice structured data block is missing the \"dueDate\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"dueDate\" with the invoice due date in ISO 8601 \
                                     format."
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
    fn test_invoice_missing_accountid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice",
                "dueDate": "2024-01-15"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "INV001"));
    }

    #[test]
    fn test_invoice_missing_duedate() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice",
                "accountId": "INV-001"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "INV002"));
    }

    #[test]
    fn test_invoice_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice",
                "accountId": "INV-001",
                "dueDate": "2024-01-15"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_invoice_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_invoice_non_invoice_type_ignored() {
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
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_invoice_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_invoice_accountid_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice",
                "accountId": ""
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "INV001"));
    }

    #[test]
    fn test_invoice_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Invoice".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Invoice",
                    "accountId": "INV-001",
                    "dueDate": "2024-01-15"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Invoice".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Invoice"
                }),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "INV001"));
        assert!(findings.iter().any(|f| f.code == "INV002"));
    }
}
