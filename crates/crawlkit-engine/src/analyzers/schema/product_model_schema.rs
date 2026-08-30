#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ProductModelSchemaValidator;

impl Default for ProductModelSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductModelSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ProductModelSchemaValidator {
    fn name(&self) -> &str {
        "productmodel-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("ProductModel") {
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
                    code: "PMODEL001".to_string(),
                    title: "ProductModel schema missing name".to_string(),
                    description: "A ProductModel structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the product model name.".to_string(),
                });
            }

            if data.get("brand").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PMODEL002".to_string(),
                    title: "ProductModel schema missing brand".to_string(),
                    description: "A ProductModel structured data block is missing the \"brand\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"brand\" with the product brand.".to_string(),
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
    fn test_productmodel_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PMODEL001"));
    }

    #[test]
    fn test_productmodel_missing_brand() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel",
                "name": "XPS 15"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PMODEL002"));
    }

    #[test]
    fn test_productmodel_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel",
                "name": "XPS 15",
                "brand": "Dell"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_productmodel_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_productmodel_non_productmodel_type_ignored() {
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
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_productmodel_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_productmodel_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PMODEL001"));
    }

    #[test]
    fn test_productmodel_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ProductModel".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ProductModel",
                    "name": "Good Model",
                    "brand": "Acme"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ProductModel".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ProductModel"
                }),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PMODEL001"));
        assert!(findings.iter().any(|f| f.code == "PMODEL002"));
    }
}
