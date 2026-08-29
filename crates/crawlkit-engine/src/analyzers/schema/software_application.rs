#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct SoftwareApplicationValidator;

impl Default for SoftwareApplicationValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareApplicationValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SoftwareApplicationValidator {
    fn name(&self) -> &str {
        "software-application-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("SoftwareApplication") {
                continue;
            }
            let data = &sd.data;

            // SOFT001: Missing operatingSystem
            if data.get("operatingSystem").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SOFT001".to_string(),
                    title: "SoftwareApplication missing operatingSystem".to_string(),
                    description: "A SoftwareApplication structured data block is missing the \
                                  \"operatingSystem\" property. This helps search engines display \
                                  platform compatibility."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"operatingSystem\" with the supported platforms (e.g., \
                                     \"Windows\", \"macOS\", \"iOS\", \"Android\")."
                        .to_string(),
                });
            }

            // SOFT002: Missing offers
            if data.get("offers").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SOFT002".to_string(),
                    title: "SoftwareApplication missing offers".to_string(),
                    description: "A SoftwareApplication structured data block is missing the \
                                  \"offers\" property. Offers provide pricing and availability \
                                  information that helps search engines display cost details in \
                                  app search results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"offers\" with an Offer object containing \"price\" and \
                                     \"priceCurrency\"."
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
fn test_software_missing_operating_system() {
    let mut page = make_page("https://example.com/app");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SoftwareApplication".to_string()),
        data: serde_json::json!({
            "@type": "SoftwareApplication",
            "name": "My App",
            "applicationCategory": "https://schema.org/GameApplication",
            "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"}
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SoftwareApplicationValidator::new().analyze(&ctx).iter().any(|f| f.code == "SOFT001"));
}


    #[test]
fn test_software_missing_offers() {
    let mut page = make_page("https://example.com/app");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SoftwareApplication".to_string()),
        data: serde_json::json!({
            "@type": "SoftwareApplication",
            "name": "My App",
            "operatingSystem": "Windows"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SoftwareApplicationValidator::new().analyze(&ctx).iter().any(|f| f.code == "SOFT002"));
}


    #[test]
fn test_software_valid() {
    let mut page = make_page("https://example.com/app");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SoftwareApplication".to_string()),
        data: serde_json::json!({
            "@type": "SoftwareApplication",
            "name": "My App",
            "operatingSystem": "Windows",
            "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"}
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SoftwareApplicationValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_software_non_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SoftwareApplicationValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_software_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    assert!(SoftwareApplicationValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_software_offers_array_with_price() {
    let mut page = make_page("https://example.com/app");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SoftwareApplication".to_string()),
        data: serde_json::json!({
            "@type": "SoftwareApplication",
            "name": "My App",
            "operatingSystem": "iOS",
            "applicationCategory": "https://schema.org/GameApplication",
            "offers": [{"@type": "Offer", "price": "2.99", "priceCurrency": "USD"}]
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(!SoftwareApplicationValidator::new().analyze(&ctx).iter().any(|f| f.code == "SOFT003"));
}


    #[test]
fn test_software_offers_array_without_price() {
    let mut page = make_page("https://example.com/app");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SoftwareApplication".to_string()),
        data: serde_json::json!({
            "@type": "SoftwareApplication",
            "name": "My App",
            "operatingSystem": "Android",
            "offers": [{"@type": "Offer", "availability": "https://schema.org/InStock"}]
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(!SoftwareApplicationValidator::new().analyze(&ctx).iter().any(|f| f.code == "SOFT002"));
}


    #[test]
fn test_software_missing_all_fields() {
    let mut page = make_page("https://example.com/app");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SoftwareApplication".to_string()),
        data: serde_json::json!({"@type": "SoftwareApplication"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SoftwareApplicationValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SOFT001"));
    assert!(findings.iter().any(|f| f.code == "SOFT002"));
}


}
