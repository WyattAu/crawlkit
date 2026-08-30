#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct LandmarksOrHistoricalBuildingsSchemaValidator;

impl Default for LandmarksOrHistoricalBuildingsSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl LandmarksOrHistoricalBuildingsSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LandmarksOrHistoricalBuildingsSchemaValidator {
    fn name(&self) -> &str {
        "landmarks-or-historical-buildings-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("LandmarksOrHistoricalBuildings") {
                continue;
            }
            let data = &sd.data;
            if data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "LANDMARK001".to_string(), title: "LandmarksOrHistoricalBuildings schema missing name".to_string(), description: "A LandmarksOrHistoricalBuildings structured data block is missing the \"name\" property.".to_string(), url: url.clone(), recommendation: "Add \"name\" with the landmark name.".to_string() });
            }
            if data.get("address").is_none() && data.get("geo").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "LANDMARK002".to_string(), title: "LandmarksOrHistoricalBuildings schema missing location".to_string(), description: "A LandmarksOrHistoricalBuildings structured data block has neither \"address\" nor \"geo\".".to_string(), url: url.clone(), recommendation: "Add \"address\" or \"geo\" to describe the location.".to_string() });
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
    fn make_ctx<'a>(page: &'a crate::parser::ParsedPage) -> AnalysisContext<'a> {
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
    fn test_landmark_missing_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LandmarksOrHistoricalBuildings".into()),
            data: serde_json::json!({"@type":"LandmarksOrHistoricalBuildings"}),
        }];
        assert!(LandmarksOrHistoricalBuildingsSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "LANDMARK001"));
    }
    #[test]
    fn test_landmark_missing_location() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LandmarksOrHistoricalBuildings".into()),
            data: serde_json::json!({"@type":"LandmarksOrHistoricalBuildings","name":"Eiffel Tower"}),
        }];
        assert!(LandmarksOrHistoricalBuildingsSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "LANDMARK002"));
    }
    #[test]
    fn test_landmark_valid() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LandmarksOrHistoricalBuildings".into()),
            data: serde_json::json!({"@type":"LandmarksOrHistoricalBuildings","name":"Eiffel Tower","address":{"@type":"PostalAddress"}}),
        }];
        assert!(LandmarksOrHistoricalBuildingsSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .is_empty());
    }
    #[test]
    fn test_landmark_non_landmark_ignored() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({"@type":"Product"}),
        }];
        assert!(LandmarksOrHistoricalBuildingsSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .is_empty());
    }
    #[test]
    fn test_landmark_no_data() {
        let p = make_page("https://example.com");
        assert!(LandmarksOrHistoricalBuildingsSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .is_empty());
    }
    #[test]
    fn test_landmark_empty_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LandmarksOrHistoricalBuildings".into()),
            data: serde_json::json!({"@type":"LandmarksOrHistoricalBuildings","name":""}),
        }];
        assert!(LandmarksOrHistoricalBuildingsSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "LANDMARK001"));
    }
    #[test]
    fn test_landmark_all_issues() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LandmarksOrHistoricalBuildings".into()),
            data: serde_json::json!({"@type":"LandmarksOrHistoricalBuildings"}),
        }];
        assert_eq!(
            LandmarksOrHistoricalBuildingsSchemaValidator::new()
                .analyze(&make_ctx(&p))
                .len(),
            2
        );
    }
    #[test]
    fn test_landmark_with_geo() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LandmarksOrHistoricalBuildings".into()),
            data: serde_json::json!({"@type":"LandmarksOrHistoricalBuildings","name":"Tower","geo":{"@type":"GeoCoordinates"}}),
        }];
        assert!(LandmarksOrHistoricalBuildingsSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .is_empty());
    }
    #[test]
    fn test_landmark_name_only() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LandmarksOrHistoricalBuildings".into()),
            data: serde_json::json!({"@type":"LandmarksOrHistoricalBuildings","name":"Tower"}),
        }];
        let f = LandmarksOrHistoricalBuildingsSchemaValidator::new().analyze(&make_ctx(&p));
        assert!(f.iter().any(|f| f.code == "LANDMARK002"));
        assert!(!f.iter().any(|f| f.code == "LANDMARK001"));
    }
    #[test]
    fn test_landmark_both_location_types() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LandmarksOrHistoricalBuildings".into()),
            data: serde_json::json!({"@type":"LandmarksOrHistoricalBuildings","name":"Tower","address":{"@type":"PostalAddress"},"geo":{"@type":"GeoCoordinates"}}),
        }];
        assert!(LandmarksOrHistoricalBuildingsSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .is_empty());
    }
}
