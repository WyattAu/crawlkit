#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct BroadcastEventSchemaValidator;

impl Default for BroadcastEventSchemaValidator {
    fn default() -> Self { Self::new() }
}

impl BroadcastEventSchemaValidator {
    pub fn new() -> Self { Self }
}

impl Analyzer for BroadcastEventSchemaValidator {
    fn name(&self) -> &str { "broadcast-event-schema" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("BroadcastEvent") { continue; }
            let data = &sd.data;
            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "BROADCAST001".to_string(), title: "BroadcastEvent schema missing name".to_string(), description: "A BroadcastEvent structured data block is missing the \"name\" property.".to_string(), url: url.clone(), recommendation: "Add \"name\" with the broadcast event name.".to_string() });
            }
            if data.get("startDate").is_none() && data.get("endDate").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "BROADCAST002".to_string(), title: "BroadcastEvent schema missing dates".to_string(), description: "A BroadcastEvent structured data block has neither \"startDate\" nor \"endDate\".".to_string(), url: url.clone(), recommendation: "Add \"startDate\" with an ISO 8601 date.".to_string() });
            }
            if data.get("broadcastOfEvent").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "BROADCAST003".to_string(), title: "BroadcastEvent schema missing broadcastOfEvent".to_string(), description: "A BroadcastEvent structured data block is missing the \"broadcastOfEvent\" property.".to_string(), url: url.clone(), recommendation: "Add \"broadcastOfEvent\" referencing the Event being broadcast.".to_string() });
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

    #[test] fn test_broadcast_missing_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BroadcastEvent".into()), data: serde_json::json!({"@type":"BroadcastEvent"}) }]; assert!(BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "BROADCAST001")); }
    #[test] fn test_broadcast_missing_dates() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BroadcastEvent".into()), data: serde_json::json!({"@type":"BroadcastEvent","name":"Live Show"}) }]; assert!(BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "BROADCAST002")); }
    #[test] fn test_broadcast_missing_event() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BroadcastEvent".into()), data: serde_json::json!({"@type":"BroadcastEvent","name":"Live Show","startDate":"2024-06-01T20:00:00Z"}) }]; assert!(BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "BROADCAST003")); }
    #[test] fn test_broadcast_valid() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BroadcastEvent".into()), data: serde_json::json!({"@type":"BroadcastEvent","name":"Live Show","startDate":"2024-06-01T20:00:00Z","broadcastOfEvent":{"@type":"Event","name":"Concert"}}) }]; assert!(BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_broadcast_non_broadcast_ignored() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({"@type":"Product"}) }]; assert!(BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_broadcast_no_data() { let p = make_page("https://example.com"); assert!(BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_broadcast_all_issues() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BroadcastEvent".into()), data: serde_json::json!({"@type":"BroadcastEvent"}) }]; assert_eq!(BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)).len(), 3); }
    #[test] fn test_broadcast_empty_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BroadcastEvent".into()), data: serde_json::json!({"@type":"BroadcastEvent","name":""}) }]; assert!(BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "BROADCAST001")); }
    #[test] fn test_broadcast_with_end_date() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BroadcastEvent".into()), data: serde_json::json!({"@type":"BroadcastEvent","name":"Show","endDate":"2024-06-01T22:00:00Z","broadcastOfEvent":{"@type":"Event"}}) }]; let f = BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)); assert!(!f.iter().any(|x| x.code == "BROADCAST002")); }
    #[test] fn test_broadcast_name_only_two_issues() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BroadcastEvent".into()), data: serde_json::json!({"@type":"BroadcastEvent","name":"Show"}) }]; let f = BroadcastEventSchemaValidator::new().analyze(&make_ctx(&p)); assert!(f.iter().any(|f| f.code == "BROADCAST002")); assert!(f.iter().any(|f| f.code == "BROADCAST003")); }
}
