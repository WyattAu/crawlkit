#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct OrganizationSameAsValidator;

impl Default for OrganizationSameAsValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganizationSameAsValidator {
    pub fn new() -> Self {
        Self
    }

    fn is_valid_url(url: &str) -> bool {
        url::Url::parse(url).is_ok()
    }
}

impl Analyzer for OrganizationSameAsValidator {
    fn name(&self) -> &str {
        "organization-sameas"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Organization" && schema_type != "LocalBusiness" {
                continue;
            }

            let same_as = match sd.data.get("sameAs") {
                Some(serde_json::Value::String(s)) => {
                    vec![s.as_str()]
                }
                Some(serde_json::Value::Array(arr)) => {
                    arr.iter().filter_map(|v| v.as_str()).collect()
                }
                _ => continue,
            };

            for same_url in &same_as {
                if !Self::is_valid_url(same_url) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "ORSAMEAS001".to_string(),
                        title: "Organization sameAs URL is invalid".to_string(),
                        description: format!(
                            "The sameAs URL \"{same_url}\" is not a valid URL. Invalid \
                             sameAs URLs prevent search engines from associating the \
                             organization with its social profiles."
                        ),
                        url: url.clone(),
                        recommendation: "Ensure all sameAs URLs are valid, accessible URLs."
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
    use crate::parser::{ParsedPage, StructuredData};

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
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

    fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
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
    fn test_valid_same_as() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": "Org", "sameAs": "https://twitter.com/org"}),
        }];
        assert!(OrganizationSameAsValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_invalid_same_as() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": "Org", "sameAs": "not a url"}),
        }];
        assert!(OrganizationSameAsValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "ORSAMEAS001"));
    }

    #[test]
    fn test_valid_same_as_array() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": "Org", "sameAs": [
                "https://twitter.com/org",
                "https://facebook.com/org"
            ]}),
        }];
        assert!(OrganizationSameAsValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_mixed_valid_invalid_same_as() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": "Org", "sameAs": [
                "https://twitter.com/org",
                "not valid"
            ]}),
        }];
        assert!(OrganizationSameAsValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "ORSAMEAS001"));
    }

    #[test]
    fn test_local_business_same_as() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "Store", "sameAs": "bad url"}),
        }];
        assert!(OrganizationSameAsValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "ORSAMEAS001"));
    }

    #[test]
    fn test_no_same_as() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": "Org"}),
        }];
        assert!(OrganizationSameAsValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_non_organization_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "sameAs": "bad"}),
        }];
        assert!(OrganizationSameAsValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(OrganizationSameAsValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(
            OrganizationSameAsValidator::new().name(),
            "organization-sameas"
        );
    }

    #[test]
    fn test_same_as_not_string_or_array() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": "Org", "sameAs": 12345}),
        }];
        assert!(OrganizationSameAsValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_default() {
        let _ = OrganizationSameAsValidator::default();
    }
}
