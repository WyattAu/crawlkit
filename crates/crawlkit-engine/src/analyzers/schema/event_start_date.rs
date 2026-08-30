#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct EventStartDateValidator;

impl Default for EventStartDateValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStartDateValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for EventStartDateValidator {
    fn name(&self) -> &str {
        "event-start-date"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") {
                continue;
            }
            let data = &sd.data;

            let start_date = match data.get("startDate").and_then(|v| v.as_str()) {
                Some(d) => d,
                None => continue,
            };

            // Try to parse the date (ISO 8601 prefix match)
            if let Some(year) = start_date.get(0..4) {
                if let Ok(year_i) = year.parse::<i32>() {
                    if year_i < 2024 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "EVENTPAST001".to_string(),
                            title: "Event has a past startDate".to_string(),
                            description: format!(
                                "Event startDate \"{start_date}\" appears to be in the past \
                                 (year {year_i}). Past events may not display correctly in \
                                 search results."
                            ),
                            url: url.clone(),
                            recommendation: "Update the startDate to a future date or remove \
                                             past events from structured data."
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
    fn test_event_past_date() {
        let mut page = make_page("https://example.com/event");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert", "startDate": "2020-06-15T19:00"}),
        }];
        assert!(EventStartDateValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "EVENTPAST001"));
    }

    #[test]
    fn test_event_future_date() {
        let mut page = make_page("https://example.com/event");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert", "startDate": "2030-06-15T19:00"}),
        }];
        assert!(EventStartDateValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_event_no_start_date() {
        let mut page = make_page("https://example.com/event");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert"}),
        }];
        assert!(EventStartDateValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_non_event_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "startDate": "2020-01-01"}),
        }];
        assert!(EventStartDateValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_event_empty_date() {
        let mut page = make_page("https://example.com/event");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert", "startDate": ""}),
        }];
        // Empty string - no year to parse
        assert!(EventStartDateValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_event_year_1999() {
        let mut page = make_page("https://example.com/event");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert", "startDate": "1999-12-31T23:59"}),
        }];
        assert!(EventStartDateValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "EVENTPAST001"));
    }

    #[test]
    fn test_name() {
        assert_eq!(EventStartDateValidator::new().name(), "event-start-date");
    }

    #[test]
    fn test_event_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(EventStartDateValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_event_non_numeric_year() {
        let mut page = make_page("https://example.com/event");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert", "startDate": "abcd-06-15"}),
        }];
        assert!(EventStartDateValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_default() {
        let _ = EventStartDateValidator::default();
    }
}
