#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct TVSeriesEpisodeSchemaValidator;

impl Default for TVSeriesEpisodeSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TVSeriesEpisodeSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TVSeriesEpisodeSchemaValidator {
    fn name(&self) -> &str {
        "tvseries-episode-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("TVEpisode") {
                continue;
            }
            let data = &sd.data;

            if data.get("episodeNumber").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "TVEP001".to_string(),
                    title: "TVEpisode schema missing episodeNumber".to_string(),
                    description: "A TVEpisode structured data block is missing the \
                                  \"episodeNumber\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"episodeNumber\" with the episode number.".to_string(),
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
    fn test_tvep_missing_episode_number() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVEpisode".to_string()),
            data: serde_json::json!({"@type": "TVEpisode", "name": "Pilot"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = TVSeriesEpisodeSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TVEP001"));
    }

    #[test]
    fn test_tvep_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVEpisode".to_string()),
            data: serde_json::json!({"@type": "TVEpisode", "name": "Pilot", "episodeNumber": 1}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(TVSeriesEpisodeSchemaValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_tvep_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(TVSeriesEpisodeSchemaValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_tvep_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVSeries".to_string()),
            data: serde_json::json!({"@type": "TVSeries"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(TVSeriesEpisodeSchemaValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_tvep_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("TVEpisode".to_string()),
                data: serde_json::json!({"@type": "TVEpisode", "name": "A", "episodeNumber": 1}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("TVEpisode".to_string()),
                data: serde_json::json!({"@type": "TVEpisode", "name": "B"}),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = TVSeriesEpisodeSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TVEP001"));
    }

    #[test]
    fn test_tvep_episode_number_zero() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVEpisode".to_string()),
            data: serde_json::json!({"@type": "TVEpisode", "episodeNumber": 0}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(TVSeriesEpisodeSchemaValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_tvep_episode_number_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVEpisode".to_string()),
            data: serde_json::json!({"@type": "TVEpisode", "episodeNumber": "S01E01"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(TVSeriesEpisodeSchemaValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_tvep_severity_warning() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVEpisode".to_string()),
            data: serde_json::json!({"@type": "TVEpisode"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = TVSeriesEpisodeSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].category, IssueCategory::Schema);
    }

    #[test]
    fn test_tvep_one_finding() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVEpisode".to_string()),
            data: serde_json::json!({"@type": "TVEpisode"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = TVSeriesEpisodeSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_tvep_name_only_no_episode_number() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVEpisode".to_string()),
            data: serde_json::json!({"@type": "TVEpisode", "name": "The Finale"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = TVSeriesEpisodeSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TVEP001"));
    }
}
