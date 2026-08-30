#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct VideoObjectDurationValidator;

impl Default for VideoObjectDurationValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoObjectDurationValidator {
    pub fn new() -> Self {
        Self
    }

    fn is_valid_iso8601_duration(dur: &str) -> bool {
        if dur.is_empty() {
            return false;
        }
        let trimmed = dur.trim();
        if !trimmed.starts_with('P') {
            return false;
        }
        let has_t = trimmed.contains('T');
        let has_time = trimmed.contains('H') || trimmed.contains('M') || trimmed.contains('S');
        let has_date = trimmed.contains('Y') || trimmed.contains('M') || trimmed.contains('D');
        // Must have at least one time component after T, or date components
        if has_t && has_time {
            return true;
        }
        if has_date && !has_t {
            return true;
        }
        false
    }
}

impl Analyzer for VideoObjectDurationValidator {
    fn name(&self) -> &str {
        "video-object-duration"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("VideoObject") {
                continue;
            }
            let data = &sd.data;

            match data.get("duration") {
                None => {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "VIDDUR001".to_string(),
                        title: "VideoObject missing duration".to_string(),
                        description: "A VideoObject structured data block is missing the \
                                      \"duration\" property. Duration helps search engines \
                                      understand video length for rich results."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"duration\" with an ISO 8601 duration value \
                                         (e.g., PT1H30M)."
                            .to_string(),
                    });
                }
                Some(dur_val) => {
                    if let Some(dur_str) = dur_val.as_str() {
                        if !Self::is_valid_iso8601_duration(dur_str) {
                            findings.push(Finding {
                                severity: Severity::Warning,
                                category: IssueCategory::Schema,
                                code: "VIDDUR002".to_string(),
                                title: "VideoObject duration is not valid ISO 8601".to_string(),
                                description: format!(
                                    "Duration \"{dur_str}\" is not a valid ISO 8601 duration. \
                                     Search engines require PT format (e.g., PT1H30M)."
                                ),
                                url: url.clone(),
                                recommendation:
                                    "Use ISO 8601 duration format: P[n]Y[n]M[n]DT[n]H[n]M[n]S."
                                        .to_string(),
                            });
                        }
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
    fn test_missing_duration() {
        let mut page = make_page("https://example.com/video");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test"}),
        }];
        assert!(VideoObjectDurationValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "VIDDUR001"));
    }

    #[test]
    fn test_valid_duration() {
        let mut page = make_page("https://example.com/video");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test", "duration": "PT1H30M"}),
        }];
        assert!(VideoObjectDurationValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_invalid_duration_format() {
        let mut page = make_page("https://example.com/video");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test", "duration": "90 minutes"}),
        }];
        assert!(VideoObjectDurationValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "VIDDUR002"));
    }

    #[test]
    fn test_empty_duration_string() {
        let mut page = make_page("https://example.com/video");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test", "duration": ""}),
        }];
        assert!(VideoObjectDurationValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "VIDDUR002"));
    }

    #[test]
    fn test_non_video_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "Test"}),
        }];
        assert!(VideoObjectDurationValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(VideoObjectDurationValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_duration_hours_only() {
        let mut page = make_page("https://example.com/video");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test", "duration": "PT2H"}),
        }];
        assert!(VideoObjectDurationValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_duration_seconds_only() {
        let mut page = make_page("https://example.com/video");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test", "duration": "PT30S"}),
        }];
        assert!(VideoObjectDurationValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_duration_date_only() {
        let mut page = make_page("https://example.com/video");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test", "duration": "P1D"}),
        }];
        assert!(VideoObjectDurationValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(
            VideoObjectDurationValidator::new().name(),
            "video-object-duration"
        );
    }
}
