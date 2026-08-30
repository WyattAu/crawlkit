#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct LandformSchemaValidator;

impl Default for LandformSchemaValidator {
    fn default() -> Self { Self::new() }
}

impl LandformSchemaValidator {
    pub fn new() -> Self { Self }
}

impl Analyzer for LandformSchemaValidator {
    fn name(&self) -> &str { "landform-schema" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Landform") { continue; }
            let data = &sd.data;
            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "LANDFORM001".to_string(), title: "Landform schema missing name".to_string(), description: "A Landform structured data block is missing the \"name\" property.".to_string(), url: url.clone(), recommendation: "Add \"name\" with the landform name.".to_string() });
            }
            if data.get("geo").is_none() && data.get("address").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "LANDFORM002".to_string(), title: "Landform schema missing geographic data".to_string(), description: "A Landform structured data block has neither \"geo\" nor \"address\".".to_string(), url: url.clone(), recommendation: "Add \"geo\" (GeoCoordinates) to describe the location.".to_string() });
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

    #[test] fn test_landform_missing_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Landform".into()), data: serde_json::json!({"@type":"Landform"}) }]; assert!(LandformSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "LANDFORM001")); }
    #[test] fn test_landform_missing_geo() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Landform".into()), data: serde_json::json!({"@type":"Landform","name":"Mountain"}) }]; assert!(LandformSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "LANDFORM002")); }
    #[test] fn test_landform_valid() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Landform".into()), data: serde_json::json!({"@type":"Landform","name":"Mountain","geo":{"@type":"GeoCoordinates"}}) }]; assert!(LandformSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_landform_non_landform_ignored() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({"@type":"Product"}) }]; assert!(LandformSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_landform_no_data() { let p = make_page("https://example.com"); assert!(LandformSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_landform_empty_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Landform".into()), data: serde_json::json!({"@type":"Landform","name":""}) }]; assert!(LandformSchemaValidator::new().analyze(&make_ctx(&p)).iter().any(|f| f.code == "LANDFORM001")); }
    #[test] fn test_landform_all_issues() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Landform".into()), data: serde_json::json!({"@type":"Landform"}) }]; assert_eq!(LandformSchemaValidator::new().analyze(&make_ctx(&p)).len(), 2); }
    #[test] fn test_landform_name_only() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Landform".into()), data: serde_json::json!({"@type":"Landform","name":"Mountain"}) }]; let f = LandformSchemaValidator::new().analyze(&make_ctx(&p)); assert!(f.iter().any(|f| f.code == "LANDFORM002")); assert!(!f.iter().any(|f| f.code == "LANDFORM001")); }
    #[test] fn test_landform_with_address() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Landform".into()), data: serde_json::json!({"@type":"Landform","name":"Mountain","address":{"@type":"PostalAddress"}}) }]; assert!(LandformSchemaValidator::new().analyze(&make_ctx(&p)).is_empty()); }
    #[test] fn test_landform_name_and_geo() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Landform".into()), data: serde_json::json!({"@type":"Landform","name":"Mountain","geo":{"@type":"GeoCoordinates"}}) }]; let f = LandformSchemaValidator::new().analyze(&make_ctx(&p)); assert!(!f.iter().any(|x| x.code == "LANDFORM001")); assert!(!f.iter().any(|x| x.code == "LANDFORM002")); }
}
