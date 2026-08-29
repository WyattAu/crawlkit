#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct LocalBusinessNpiValidator;

impl Default for LocalBusinessNpiValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalBusinessNpiValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LocalBusinessNpiValidator {
    fn name(&self) -> &str {
        "local-business-npi"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            let is_local_business = schema_type == "LocalBusiness"
                || schema_type == "Physician"
                || schema_type == "Dentist"
                || schema_type == "Hospital"
                || schema_type == "Pharmacy"
                || schema_type == "MedicalClinic";

            if !is_local_business {
                continue;
            }

            // Check for medical specialty
            let has_medical = sd.data.get("medicalSpecialty").is_some()
                || sd.data.get("availableService").is_some()
                || sd.data.get("areaServed").is_some();

            if !has_medical {
                continue;
            }

            // Check for NPI
            let has_npi = sd.data.get("npi").is_some()
                || sd.data.get("identifier").is_some_and(|id| {
                    if let Some(s) = id.as_str() {
                        s.len() == 10 && s.chars().all(|c| c.is_ascii_digit())
                    } else if let Some(obj) = id.as_object() {
                        obj.values()
                            .any(|v| v.as_str().is_some_and(|s| s.len() == 10 && s.chars().all(|c| c.is_ascii_digit())))
                    } else {
                        false
                    }
                });

            if !has_npi {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "NPI001".to_string(),
                    title: "Medical LocalBusiness missing NPI".to_string(),
                    description: format!(
                        "A LocalBusiness with medical specialty (type \"{schema_type}\") is \
                         missing an NPI (National Provider Identifier) identifier. NPI is \
                         required for healthcare providers in the US."
                    ),
                    url: url.clone(),
                    recommendation: "Add an NPI identifier to the LocalBusiness schema for \
                                     healthcare providers."
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
    fn test_physician_with_medical_no_npi() {
        let mut page = make_page("https://example.com/doctor");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Physician".to_string()),
            data: serde_json::json!({"@type": "Physician", "name": "Dr. Smith", "medicalSpecialty": "Cardiology"}),
        }];
        assert!(LocalBusinessNpiValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "NPI001"));
    }

    #[test]
    fn test_physician_with_medical_with_npi() {
        let mut page = make_page("https://example.com/doctor");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Physician".to_string()),
            data: serde_json::json!({"@type": "Physician", "name": "Dr. Smith", "medicalSpecialty": "Cardiology", "npi": "1234567890"}),
        }];
        assert!(LocalBusinessNpiValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_local_business_no_medical() {
        let mut page = make_page("https://example.com/store");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "Store"}),
        }];
        assert!(LocalBusinessNpiValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_local_business_with_available_service() {
        let mut page = make_page("https://example.com/clinic");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "Clinic", "availableService": "Checkup"}),
        }];
        assert!(LocalBusinessNpiValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "NPI001"));
    }

    #[test]
    fn test_non_medical_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Restaurant".to_string()),
            data: serde_json::json!({"@type": "Restaurant", "name": "Cafe"}),
        }];
        assert!(LocalBusinessNpiValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_dentist_no_npi() {
        let mut page = make_page("https://example.com/dentist");
        page.structured_data = vec![StructuredData {
            context: Some("https:://schema.org".to_string()),
            r#type: Some("Dentist".to_string()),
            data: serde_json::json!({"@type": "Dentist", "name": "Dr. Jones", "medicalSpecialty": "Dentistry"}),
        }];
        assert!(LocalBusinessNpiValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "NPI001"));
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(LocalBusinessNpiValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(LocalBusinessNpiValidator::new().name(), "local-business-npi");
    }

    #[test]
    fn test_local_business_with_identifier_object() {
        let mut page = make_page("https://example.com/clinic");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "Clinic", "medicalSpecialty": "General", "identifier": {"npi": "9876543210"}}),
        }];
        assert!(LocalBusinessNpiValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_default() {
        let _ = LocalBusinessNpiValidator::default();
    }
}
