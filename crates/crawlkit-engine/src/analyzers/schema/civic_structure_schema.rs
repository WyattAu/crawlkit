#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct CivicStructureSchemaValidator;

impl Default for CivicStructureSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CivicStructureSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CivicStructureSchemaValidator {
    fn name(&self) -> &str {
        "civic-structure-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("CivicStructure") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CIVIC001".to_string(),
                    title: "CivicStructure schema missing name".to_string(),
                    description: "A CivicStructure structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the civic structure name.".to_string(),
                });
            }

            if data.get("address").is_none() && data.get("geo").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CIVIC002".to_string(),
                    title: "CivicStructure schema missing location".to_string(),
                    description: "A CivicStructure structured data block has neither \"address\" nor \"geo\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"address\" or \"geo\" to describe the location.".to_string(),
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
        crate::parser::ParsedPage { url: url.to_string(), meta: MetaTags::default(), headings: Vec::new(), links: Vec::new(), images: Vec::new(), forms: Vec::new(), scripts: Vec::new(), styles: Vec::new(), structured_data: Vec::new(), word_count: 0, sentence_count: 0, landmarks: Vec::new(), has_skip_link: false, has_main_landmark: false, has_nav_landmark: false, has_positive_tabindex: false, tabindex_negative_count: 0, aria_role_count: 0, aria_label_count: 0, has_lang_attribute: false, html_lang: None, has_aria_hidden: false, tables_with_headers: 0, tables_total: 0, tables_with_captions: 0, og_image_width: None, og_image_height: None }
    }

    fn make_ctx<'a>(page: &'a crate::parser::ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext { page, body: None, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }
    }

    #[test]
    fn test_civic_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("CivicStructure".to_string()), data: serde_json::json!({"@type": "CivicStructure"}) }];
        assert!(CivicStructureSchemaValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "CIVIC001"));
    }

    #[test]
    fn test_civic_missing_location() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("CivicStructure".to_string()), data: serde_json::json!({"@type": "CivicStructure", "name": "Library"}) }];
        assert!(CivicStructureSchemaValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "CIVIC002"));
    }

    #[test]
    fn test_civic_valid_with_address() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("CivicStructure".to_string()), data: serde_json::json!({"@type": "CivicStructure", "name": "Library", "address": {"@type": "PostalAddress"}}) }];
        assert!(CivicStructureSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_civic_valid_with_geo() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("CivicStructure".to_string()), data: serde_json::json!({"@type": "CivicStructure", "name": "Library", "geo": {"@type": "GeoCoordinates", "latitude": 40.7128, "longitude": -74.0060}}) }];
        assert!(CivicStructureSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_civic_non_civic_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("Product".to_string()), data: serde_json::json!({"@type": "Product"}) }];
        assert!(CivicStructureSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_civic_no_data() {
        let page = make_page("https://example.com");
        assert!(CivicStructureSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_civic_all_issues() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("CivicStructure".to_string()), data: serde_json::json!({"@type": "CivicStructure"}) }];
        assert_eq!(CivicStructureSchemaValidator::new().analyze(&make_ctx(&page)).len(), 2);
    }

    #[test]
    fn test_civic_empty_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("CivicStructure".to_string()), data: serde_json::json!({"@type": "CivicStructure", "name": ""}) }];
        assert!(CivicStructureSchemaValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "CIVIC001"));
    }

    #[test]
    fn test_civic_name_only() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("CivicStructure".to_string()), data: serde_json::json!({"@type": "CivicStructure", "name": "Library"}) }];
        let findings = CivicStructureSchemaValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "CIVIC002"));
        assert!(!findings.iter().any(|f| f.code == "CIVIC001"));
    }

    #[test]
    fn test_civic_both_address_and_geo() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("CivicStructure".to_string()), data: serde_json::json!({"@type": "CivicStructure", "name": "Library", "address": {"@type": "PostalAddress"}, "geo": {"@type": "GeoCoordinates"}}) }];
        assert!(CivicStructureSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }
}
