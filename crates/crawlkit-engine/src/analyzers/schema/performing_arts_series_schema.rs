#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct PerformingArtsSeriesSchemaValidator;

impl Default for PerformingArtsSeriesSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformingArtsSeriesSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PerformingArtsSeriesSchemaValidator {
    fn name(&self) -> &str {
        "performing-arts-series-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("PerformingArtsSeries") {
                continue;
            }
            let data = &sd.data;
            if data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "PERFORMARTS001".to_string(), title: "PerformingArtsSeries schema missing name".to_string(), description: "A PerformingArtsSeries structured data block is missing the \"name\" property.".to_string(), url: url.clone(), recommendation: "Add \"name\" with the series name.".to_string() });
            }
            if data.get("organizer").is_none() && data.get("performer").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PERFORMARTS002".to_string(), title: "PerformingArtsSeries schema missing organizer/performer".to_string(), description: "A PerformingArtsSeries structured data block has neither \"organizer\" nor \"performer\".".to_string(), url: url.clone(), recommendation: "Add \"organizer\" or \"performer\" to identify who runs the series.".to_string() });
            }
            if data.get("event").is_none() && data.get("subEvent").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PERFORMARTS003".to_string(), title: "PerformingArtsSeries schema missing events".to_string(), description: "A PerformingArtsSeries structured data block has neither \"event\" nor \"subEvent\".".to_string(), url: url.clone(), recommendation: "Add \"event\" or \"subEvent\" with Event objects.".to_string() });
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
    fn test_perform_missing_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("PerformingArtsSeries".into()),
            data: serde_json::json!({"@type":"PerformingArtsSeries"}),
        }];
        assert!(PerformingArtsSeriesSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "PERFORMARTS001"));
    }
    #[test]
    fn test_perform_missing_org() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("PerformingArtsSeries".into()),
            data: serde_json::json!({"@type":"PerformingArtsSeries","name":"Concert Series"}),
        }];
        assert!(PerformingArtsSeriesSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "PERFORMARTS002"));
    }
    #[test]
    fn test_perform_missing_events() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("PerformingArtsSeries".into()),
            data: serde_json::json!({"@type":"PerformingArtsSeries","name":"Concert Series","organizer":{"@type":"Organization","name":"Org"}}),
        }];
        assert!(PerformingArtsSeriesSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "PERFORMARTS003"));
    }
    #[test]
    fn test_perform_valid() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("PerformingArtsSeries".into()),
            data: serde_json::json!({"@type":"PerformingArtsSeries","name":"Concert Series","organizer":{"@type":"Organization"},"event":{"@type":"Event"}}),
        }];
        assert!(PerformingArtsSeriesSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .is_empty());
    }
    #[test]
    fn test_perform_non_perform_ignored() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({"@type":"Product"}),
        }];
        assert!(PerformingArtsSeriesSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .is_empty());
    }
    #[test]
    fn test_perform_no_data() {
        let p = make_page("https://example.com");
        assert!(PerformingArtsSeriesSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .is_empty());
    }
    #[test]
    fn test_perform_all_issues() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("PerformingArtsSeries".into()),
            data: serde_json::json!({"@type":"PerformingArtsSeries"}),
        }];
        assert_eq!(
            PerformingArtsSeriesSchemaValidator::new()
                .analyze(&make_ctx(&p))
                .len(),
            3
        );
    }
    #[test]
    fn test_perform_empty_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("PerformingArtsSeries".into()),
            data: serde_json::json!({"@type":"PerformingArtsSeries","name":""}),
        }];
        assert!(PerformingArtsSeriesSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .iter()
            .any(|f| f.code == "PERFORMARTS001"));
    }
    #[test]
    fn test_perform_with_performer() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("PerformingArtsSeries".into()),
            data: serde_json::json!({"@type":"PerformingArtsSeries","name":"Series","performer":{"@type":"Person","name":"Artist"},"subEvent":{"@type":"Event"}}),
        }];
        assert!(PerformingArtsSeriesSchemaValidator::new()
            .analyze(&make_ctx(&p))
            .is_empty());
    }
    #[test]
    fn test_perform_name_only_two_issues() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("PerformingArtsSeries".into()),
            data: serde_json::json!({"@type":"PerformingArtsSeries","name":"Series"}),
        }];
        let f = PerformingArtsSeriesSchemaValidator::new().analyze(&make_ctx(&p));
        assert!(f.iter().any(|f| f.code == "PERFORMARTS002"));
        assert!(f.iter().any(|f| f.code == "PERFORMARTS003"));
    }
}
