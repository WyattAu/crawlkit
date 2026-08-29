use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct LocalBusinessSchemaValidator;

impl Default for LocalBusinessSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalBusinessSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    const LOCAL_BUSINESS_TYPES: &[&str] = &[
        "LocalBusiness", "Store", "Restaurant", "MedicalBusiness",
        "FinancialService", "TravelAgency", "AutoBodyShop", "AutoDealer",
        "AutoPartsStore", "AutoRental", "AutoRepair", "Bakery", "BarOrPub",
        "BeautySalon", "Brewery", "CafeOrCoffeeShop", "Cemetery",
        "ChildCare", "Dentist", "EmploymentAgency", "EntertainmentBusiness",
        "FinancialService", "FoodEstablishment", "GardenStore",
        "GovernmentOffice", "HealthAndBeautyBusiness", "HomeAndConstructionBusiness",
        "InsuranceAgency", "InternetCafe", "LegalService", "Library",
        "LodgingBusiness", "ManisBusiness", "MovieRentalStore", "MovingCompany",
        "MusicStore", "OfficeEquipmentStore", "OutletStore", "PawnShop",
        "PetStore", "Physician", "Plumber", "RealEstateAgent",
        "RecyclingCenter", "SelfStorage", "ShoeStore", "ShoppingCenter",
        "SportingGoodsStore", "TattooParlor", "TelevisionStation",
        "ToyStore", "TravelAgency", "WholesaleStore",
    ];
}

impl Analyzer for LocalBusinessSchemaValidator {
    fn name(&self) -> &str {
        "local-business-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if !Self::LOCAL_BUSINESS_TYPES.contains(&schema_type) {
                continue;
            }

            // LBIZ001: Missing name
            if sd.data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "LBIZ001".to_string(),
                    title: "LocalBusiness schema missing name".to_string(),
                    description: format!(
                        "A {} schema is missing the required \"name\" property.",
                        schema_type
                    ),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the business name."
                        .to_string(),
                });
            }

            // LBIZ002: Missing address
            if sd.data.get("address").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "LBIZ002".to_string(),
                    title: "LocalBusiness schema missing address".to_string(),
                    description: format!(
                        "A {} schema is missing the \"address\" property.",
                        schema_type
                    ),
                    url: url.clone(),
                    recommendation: "Add \"address\" with a PostalAddress object."
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

    fn make_ctx_with_body<'a>(
        page: &'a crate::parser::ParsedPage,
        status: Option<u16>,
        body: &'a str,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: Some(body),
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
fn test_local_business_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({"@type": "LocalBusiness", "address": {"@type": "PostalAddress"}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(LocalBusinessSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "LBIZ001"));
}


    #[test]
fn test_local_business_missing_address() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(LocalBusinessSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "LBIZ002"));
}


    #[test]
fn test_local_business_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "address": {"@type": "PostalAddress", "streetAddress": "123 Main St"}}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LocalBusinessSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_local_business_subtypes_checked() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Restaurant".to_string()),
        data: serde_json::json!({"@type": "Restaurant"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LocalBusinessSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LBIZ001"));
}


}
