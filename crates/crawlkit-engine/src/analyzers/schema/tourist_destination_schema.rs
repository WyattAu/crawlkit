#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct TouristDestinationSchemaValidator;

impl Default for TouristDestinationSchemaValidator {
    fn default() -> Self { Self::new() }
}

impl TouristDestinationSchemaValidator {
    pub fn new() -> Self { Self }
}

impl Analyzer for TouristDestinationSchemaValidator {
    fn name(&self) -> &str { "tourist-destination-schema" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("TouristDestination") { continue; }
            let data = &sd.data;
            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "TOURISTDEST001".to_string(), title: "TouristDestination schema missing name".to_string(), description: "A TouristDestination structured data block is missing the \"name\" property.".to_string(), url: url.clone(), recommendation: "Add \"name\" with the destination name.".to_string() });
            }
            if data.get("address").is_none() && data.get("geo").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "TOURISTDEST002".to_string(), title: "TouristDestination schema missing location".to_string(), description: "A TouristDestination structured data block has neither \"address\" nor \"geo\".".to_string(), url: url.clone(), recommendation: "Add \"address\" or \"geo\" to describe the location.".to_string() });
            }
            if data.get("touristType").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "TOURISTDEST003".to_string(), title: "TouristDestination schema missing touristType".to_string(), description: "A TouristDestination structured data block is missing the \"touristType\" property.".to_string(), url: url.clone(), recommendation: "Add \"touristType\" to describe the audience.".to_string() });
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

    #[test] fn test_tdest_missing_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristDestination".into()), data: serde_json::json!({"@type":"TouristDestination"}) }]; assert!(TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "TOURISTDEST001")); }
    #[test] fn test_tdest_missing_location() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristDestination".into()), data: serde_json::json!({"@type":"TouristDestination","name":"Paris"}) }]; assert!(TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "TOURISTDEST002")); }
    #[test] fn test_tdest_missing_type() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristDestination".into()), data: serde_json::json!({"@type":"TouristDestination","name":"Paris","address":{"@type":"PostalAddress"}}) }]; assert!(TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "TOURISTDEST003")); }
    #[test] fn test_tdest_valid() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristDestination".into()), data: serde_json::json!({"@type":"TouristDestination","name":"Paris","address":{"@type":"PostalAddress"},"touristType":"Couples"}) }]; assert!(TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_tdest_non_dest_ignored() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({"@type":"Product"}) }]; assert!(TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_tdest_no_data() { let p = make_page("https://example.com"); assert!(TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_tdest_all_issues() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristDestination".into()), data: serde_json::json!({"@type":"TouristDestination"}) }]; assert_eq!(TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)).len(), 3); }
    #[test] fn test_tdest_empty_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristDestination".into()), data: serde_json::json!({"@type":"TouristDestination","name":""}) }]; assert!(TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "TOURISTDEST001")); }
    #[test] fn test_tdest_with_geo() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristDestination".into()), data: serde_json::json!({"@type":"TouristDestination","name":"Paris","geo":{"@type":"GeoCoordinates"},"touristType":"Families"}) }]; assert!(TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_tdest_name_only_two_issues() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristDestination".into()), data: serde_json::json!({"@type":"TouristDestination","name":"Paris"}) }]; let f = TouristDestinationSchemaValidator::new().analyze(&make_ctx(&p)); assert!(f.iter().any(|f| f.code == "TOURISTDEST002")); assert!(f.iter().any(|f| f.code == "TOURISTDEST003")); }
}
