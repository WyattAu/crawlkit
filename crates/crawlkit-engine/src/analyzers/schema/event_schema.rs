#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct EventSchemaValidator;

impl Default for EventSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for EventSchemaValidator {
    fn name(&self) -> &str {
        "event-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") {
                continue;
            }
            let data = &sd.data;

            // EVENT001: Missing startDate
            if data.get("startDate").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "EVENT001".to_string(),
                    title: "Event schema missing startDate".to_string(),
                    description: "An Event structured data block is missing the required \
                                 \"startDate\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"startDate\" with an ISO 8601 date/time value."
                        .to_string(),
                });
            }

            // EVENT002: Missing location
            if data.get("location").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "EVENT002".to_string(),
                    title: "Event schema missing location".to_string(),
                    description: "An Event structured data block is missing the \"location\" \
                                 property. This may reduce eligibility for rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"location\" with a Place or VirtualLocation object."
                        .to_string(),
                });
            }

            // EVENT003: Missing organizer
            if data.get("organizer").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "EVENT003".to_string(),
                    title: "Event schema missing organizer".to_string(),
                    description: "An Event structured data block is missing the \"organizer\" \
                                 property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"organizer\" with a Person or Organization object."
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
    fn test_event_missing_start_date() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(EventSchemaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "EVENT001"));
    }

    #[test]
    fn test_event_missing_location() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert", "startDate": "2025-06-01T19:00:00Z"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(EventSchemaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "EVENT002"));
    }

    #[test]
    fn test_event_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert", "startDate": "2025-06-01T19:00:00Z", "location": {"@type": "Place", "name": "Venue"}, "organizer": {"@type": "Organization", "name": "Org"}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EventSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}
