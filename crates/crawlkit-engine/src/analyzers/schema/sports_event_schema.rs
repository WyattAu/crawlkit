#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct SportsEventSchemaValidator;

impl Default for SportsEventSchemaValidator {
    fn default() -> Self { Self::new() }
}

impl SportsEventSchemaValidator {
    pub fn new() -> Self { Self }
}

impl Analyzer for SportsEventSchemaValidator {
    fn name(&self) -> &str { "sports-event-schema" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("SportsEvent") { continue; }
            let data = &sd.data;
            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "SPORTSEVT001".to_string(), title: "SportsEvent schema missing name".to_string(), description: "A SportsEvent structured data block is missing the \"name\" property.".to_string(), url: url.clone(), recommendation: "Add \"name\" with the event name.".to_string() });
            }
            if data.get("startDate").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "SPORTSEVT002".to_string(), title: "SportsEvent schema missing startDate".to_string(), description: "A SportsEvent structured data block is missing the \"startDate\" property.".to_string(), url: url.clone(), recommendation: "Add \"startDate\" with an ISO 8601 date.".to_string() });
            }
            if data.get("location").is_none() && data.get("locationCreated").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "SPORTSEVT003".to_string(), title: "SportsEvent schema missing location".to_string(), description: "A SportsEvent structured data block has neither \"location\" nor \"locationCreated\".".to_string(), url: url.clone(), recommendation: "Add \"location\" with a Place or PostalAddress.".to_string() });
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
    fn make_page(url: &str) -> crate::parser::ParsedPage { crate::parser::ParsedPage { url: url.to_string(), meta: MetaTags::default(), headings: Vec::new(), links: Vec::new(), images: Vec::new(), forms: Vec::new(), scripts: Vec::new(), styles: Vec::new(), structured_data: Vec::new(), word_count: 0, sentence_count: 0, landmarks: Vec::new(), has_skip_link: false, has_main_landmark: false, has_nav_landmark: false, has_positive_tabindex: false, tabindex_negative_count: 0, aria_role_count: 0, aria_label_count: 0, has_lang_attribute: false, html_lang: None, has_aria_hidden: false, tables_with_headers: 0, tables_total: 0, tables_with_captions: 0, og_image_width: None, og_image_height: None } }
    fn make_ctx<'a>(page: &'a crate::parser::ParsedPage) -> AnalysisContext<'a> { AnalysisContext { page, body: None, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None } }

    #[test] fn test_sportsevt_missing_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({"@type":"SportsEvent"}) }]; assert!(SportsEventSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "SPORTSEVT001")); }
    #[test] fn test_sportsevt_missing_date() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({"@type":"SportsEvent","name":"Game"}) }]; assert!(SportsEventSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "SPORTSEVT002")); }
    #[test] fn test_sportsevt_missing_location() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({"@type":"SportsEvent","name":"Game","startDate":"2024-06-01"}) }]; assert!(SportsEventSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "SPORTSEVT003")); }
    #[test] fn test_sportsevt_valid() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({"@type":"SportsEvent","name":"Game","startDate":"2024-06-01","location":{"@type":"Place"}}) }]; assert!(SportsEventSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_sportsevt_non_sport_ignored() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({"@type":"Product"}) }]; assert!(SportsEventSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_sportsevt_no_data() { let p = make_page("https://example.com"); assert!(SportsEventSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_sportsevt_all_issues() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({"@type":"SportsEvent"}) }]; assert_eq!(SportsEventSchemaValidator::new().analyze(&make_ctx(&p)).len(), 3); }
    #[test] fn test_sportsevt_empty_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({"@type":"SportsEvent","name":""}) }]; assert!(SportsEventSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "SPORTSEVT001")); }
    #[test] fn test_sportsevt_with_location_created() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({"@type":"SportsEvent","name":"Game","startDate":"2024-06-01","locationCreated":{"@type":"Place"}}) }]; assert!(SportsEventSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_sportsevt_name_only_two_issues() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({"@type":"SportsEvent","name":"Game"}) }]; let f = SportsEventSchemaValidator::new().analyze(&make_ctx(&p)); assert!(f.iter().any(|f| f.code == "SPORTSEVT002")); assert!(f.iter().any(|f| f.code == "SPORTSEVT003")); }
}
