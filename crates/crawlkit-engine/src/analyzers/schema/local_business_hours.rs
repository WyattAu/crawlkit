#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct LocalBusinessHoursValidator;

impl LocalBusinessHoursValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalBusinessHoursValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LocalBusinessHoursValidator {
    fn name(&self) -> &str {
        "local-business-hours"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.r#type.as_deref();
            let is_local = matches!(
                schema_type,
                Some("LocalBusiness")
                    | Some("Store")
                    | Some("Restaurant")
                    | Some("Hotel")
                    | Some("HealthClub")
                    | Some("AutomotiveBusiness")
                    | Some("EntertainmentBusiness")
                    | Some("FinancialService")
                    | Some("FoodEstablishment")
                    | Some("GovernmentOffice")
                    | Some("HealthAndBeautyBusiness")
                    | Some("HomeAndConstructionBusiness")
                    | Some("InternetCafe")
                    | Some("LegalService")
                    | Some("Library")
                    | Some("LodgingBusiness")
                    | Some("ProfessionalService")
                    | Some("RadioStation")
                    | Some("SelfStorage")
                    | Some("ShoppingCenter")
                    | Some("SportsActivityLocation")
                    | Some("TelevisionStation")
                    | Some("TouristInformationCenter")
                    | Some("TravelAgency")
            );
            if !is_local {
                continue;
            }

            let data = &sd.data;

            if data.get("openingHours").is_none() && data.get("openingHoursSpecification").is_none()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "LBH001".to_string(),
                    title: "LocalBusiness missing openingHours".to_string(),
                    description: "A LocalBusiness structured data block is missing the \
                                  \"openingHours\" or \"openingHoursSpecification\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"openingHours\" with ISO 8601 time ranges (e.g., \
                                     \"Mo-Fr 09:00-17:00\") or \"openingHoursSpecification\" for \
                                     detailed hours."
                        .to_string(),
                });
                continue;
            }

            if let Some(hours) = data.get("openingHours") {
                if let Some(s) = hours.as_str() {
                    let valid_format = s.split(',').all(|entry| {
                        let entry = entry.trim();
                        if entry.is_empty() {
                            return true;
                        }
                        let days = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
                        let has_day = days.iter().any(|d| entry.contains(d));
                        let has_dash_range = entry.contains('-') && entry.matches('-').count() <= 2;
                        let has_time = entry.contains(':');
                        has_day || has_dash_range || has_time
                    });
                    if !valid_format {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "LBH002".to_string(),
                            title: "LocalBusiness openingHours in invalid format".to_string(),
                            description: format!(
                                "The openingHours value \"{s}\" does not appear to follow ISO \
                                 8601 format."
                            ),
                            url: url.clone(),
                            recommendation: "Use ISO 8601 format for openingHours, e.g., \
                                             \"Mo-Fr 09:00-17:00, Sa 10:00-14:00\"."
                                .to_string(),
                        });
                    }
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
    fn test_lbh_missing_opening_hours() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "My Shop"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LBH001"));
    }

    #[test]
    fn test_lbh_with_opening_hours_no_findings() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "My Shop",
                "openingHours": "Mo-Fr 09:00-17:00"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_lbh_invalid_format() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "My Shop",
                "openingHours": "open all day!!!"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LBH002"));
    }

    #[test]
    fn test_lbh_store_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Store".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Store",
                "name": "My Store"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LBH001"));
    }

    #[test]
    fn test_lbh_restaurant_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Restaurant".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Restaurant",
                "name": "My Restaurant"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LBH001"));
    }

    #[test]
    fn test_lbh_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_lbh_non_local_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": "Test"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_lbh_opening_hours_specification_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "My Shop",
                "openingHoursSpecification": {
                    "@type": "OpeningHoursSpecification",
                    "dayOfWeek": "Monday",
                    "opens": "09:00",
                    "closes": "17:00"
                }
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}
