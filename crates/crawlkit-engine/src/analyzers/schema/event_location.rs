use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct EventLocationValidator;

impl EventLocationValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EventLocationValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for EventLocationValidator {
    fn name(&self) -> &str {
        "event-location"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") {
                continue;
            }
            let data = &sd.data;

            if data.get("location").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ELOC001".to_string(),
                    title: "Event missing location".to_string(),
                    description: "An Event structured data block is missing the \"location\" \
                                  property. Location is important for event rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"location\" with a Place, VirtualLocation, or PostalAddress object."
                        .to_string(),
                });
                continue;
            }

            if let Some(location) = data.get("location") {
                let has_name = location.get("name").is_some()
                    || location.get("url").is_some()
                    || location.get("address").is_some();
                if !has_name {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "ELOC002".to_string(),
                        title: "Event location missing name".to_string(),
                        description: "The \"location\" property in Event schema does not contain \
                                      a \"name\", \"url\", or \"address\" sub-property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"name\" to the location object to identify the \
                                         venue or place."
                            .to_string(),
                    });
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
fn test_eloc_missing_location() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Event".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Event",
            "name": "Conference",
            "startDate": "2024-06-15"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = EventLocationValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ELOC001"));
}


    #[test]
fn test_eloc_location_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Event".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Event",
            "name": "Conference",
            "startDate": "2024-06-15",
            "location": {"@type": "Place"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = EventLocationValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ELOC002"));
}


    #[test]
fn test_eloc_valid_location() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Event".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Event",
            "name": "Conference",
            "startDate": "2024-06-15",
            "location": {"@type": "Place", "name": "Convention Center"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = EventLocationValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_eloc_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = EventLocationValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_eloc_non_event_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = EventLocationValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_eloc_virtual_location_no_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Event".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Event",
            "name": "Webinar",
            "startDate": "2024-06-15",
            "location": {"@type": "VirtualLocation"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = EventLocationValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ELOC002"));
}


    #[test]
fn test_eloc_location_with_url() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Event".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Event",
            "name": "Webinar",
            "startDate": "2024-06-15",
            "location": {"@type": "VirtualLocation", "url": "https://zoom.us/123"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = EventLocationValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_eloc_location_with_address() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Event".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Event",
            "name": "Concert",
            "startDate": "2024-06-15",
            "location": {
                "@type": "Place",
                "address": {"@type": "PostalAddress", "streetAddress": "123 Main St"}
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = EventLocationValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
