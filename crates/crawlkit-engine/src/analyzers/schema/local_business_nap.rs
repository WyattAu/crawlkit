#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct LocalBusinessNapAnalyzer;

impl Default for LocalBusinessNapAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalBusinessNapAnalyzer {
    pub fn new() -> Self {
        Self
    }

    const LOCAL_BUSINESS_TYPES: &[&str] = &[
        "LocalBusiness", "Store", "Restaurant", "MedicalBusiness",
        "FinancialService", "TravelAgency", "AutoBodyShop", "AutoDealer",
        "AutoPartsStore", "AutoRental", "AutoRepair", "Bakery", "BarOrPub",
        "BeautySalon", "Brewery", "CafeOrCoffeeShop", "Cemetery",
        "ChildCare", "Dentist", "EmploymentAgency", "EntertainmentBusiness",
        "FoodEstablishment", "GardenStore",
        "GovernmentOffice", "HealthAndBeautyBusiness", "HomeAndConstructionBusiness",
        "InsuranceAgency", "InternetCafe", "LegalService", "Library",
        "LodgingBusiness", "MovingCompany",
        "MusicStore", "OfficeEquipmentStore", "OutletStore", "PawnShop",
        "PetStore", "Physician", "Plumber", "RealEstateAgent",
        "RecyclingCenter", "SelfStorage", "ShoeStore", "ShoppingCenter",
        "SportingGoodsStore", "TattooParlor", "TelevisionStation",
        "ToyStore", "WholesaleStore",
    ];
}

impl Analyzer for LocalBusinessNapAnalyzer {
    fn name(&self) -> &str {
        "local-business-nap"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if !Self::LOCAL_BUSINESS_TYPES.contains(&schema_type) {
                continue;
            }

            // NAP001: Missing telephone
            if sd.data.get("telephone").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "NAP001".to_string(),
                    title: "LocalBusiness schema missing telephone".to_string(),
                    description: format!(
                        "A {} schema is missing the \"telephone\" property. Phone numbers are \
                         essential for NAP consistency and local SEO."
                    ,
                        schema_type
                    ),
                    url: url.clone(),
                    recommendation: "Add \"telephone\" with the business phone number in \
                                     international format (e.g., \"+1-555-555-5555\")."
                        .to_string(),
                });
            }

            // NAP002: Missing openingHours
            if sd.data.get("openingHours").is_none()
                && sd.data.get("openingHoursSpecification").is_none()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "NAP002".to_string(),
                    title: "LocalBusiness schema missing openingHours".to_string(),
                    description: format!(
                        "A {} schema is missing \"openingHours\" or \
                         \"openingHoursSpecification\". Business hours help customers know when \
                         to visit."
                    ,
                        schema_type
                    ),
                    url: url.clone(),
                    recommendation: "Add \"openingHours\" with ISO 8601 time ranges or \
                                     \"openingHoursSpecification\" with OpeningHoursSpecification \
                                     objects."
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
fn test_nap_missing_telephone() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "address": {"@type": "PostalAddress"}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "NAP001"));
}


    #[test]
fn test_nap_missing_opening_hours() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "telephone": "+1-555-555-5555"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "NAP002"));
}


    #[test]
fn test_nap_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "telephone": "+1-555-555-5555", "openingHours": "Mo-Fr 09:00-17:00"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_nap_missing_all() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({"@type": "LocalBusiness"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "NAP001"));
    assert!(findings.iter().any(|f| f.code == "NAP002"));
}


    #[test]
fn test_nap_non_local_business_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({"@type": "Product", "name": "Widget"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_nap_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_nap_restaurant_subtype() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Restaurant".to_string()),
        data: serde_json::json!({"@type": "Restaurant"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "NAP001"));
    assert!(findings.iter().any(|f| f.code == "NAP002"));
}


    #[test]
fn test_nap_opening_hours_specification_present() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "telephone": "+1-555-555-5555", "openingHoursSpecification": [{"@type": "OpeningHoursSpecification", "dayOfWeek": "Monday", "opens": "09:00", "closes": "17:00"}]}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(!LocalBusinessNapAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "NAP002"));
}


    #[test]
fn test_nap_telephone_present_no_opening_hours() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "telephone": "+1-555-555-5555"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "NAP001"));
    assert!(findings.iter().any(|f| f.code == "NAP002"));
}


    #[test]
fn test_nap_multiple_businesses() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness"}),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness"}),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
    assert_eq!(findings.iter().filter(|f| f.code == "NAP001").count(), 2);
    assert_eq!(findings.iter().filter(|f| f.code == "NAP002").count(), 2);
}


    #[test]
fn test_nap_store_subtype() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Store".to_string()),
        data: serde_json::json!({"@type": "Store"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "NAP001"));
    assert!(findings.iter().any(|f| f.code == "NAP002"));
}


    #[test]
fn test_nap_restaurant_with_both_present() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Restaurant".to_string()),
        data: serde_json::json!({
            "@type": "Restaurant",
            "name": "Pizza Place",
            "telephone": "+1-555-123-4567",
            "openingHours": "Mo-Su 11:00-22:00"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
