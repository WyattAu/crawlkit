#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct VideoObjectEmbedUrlValidator;

impl Default for VideoObjectEmbedUrlValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoObjectEmbedUrlValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for VideoObjectEmbedUrlValidator {
    fn name(&self) -> &str {
        "video-object-embed-url"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("VideoObject") {
                continue;
            }
            let data = &sd.data;

            if data.get("embedUrl").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "VIDEMB001".to_string(),
                    title: "VideoObject missing embedUrl".to_string(),
                    description: "A VideoObject structured data block is missing the \
                                  \"embedUrl\" property. embedUrl helps search engines \
                                  understand how to embed the video on a page."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"embedUrl\" with the URL to the embedded player \
                                     (e.g., YouTube embed URL)."
                        .to_string(),
                });
            }

            if data.get("contentUrl").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "VIDEMB002".to_string(),
                    title: "VideoObject missing contentUrl".to_string(),
                    description: "A VideoObject structured data block is missing the \
                                  \"contentUrl\" property. contentUrl helps search engines \
                                  find the direct video file URL."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"contentUrl\" with the direct URL to the video file."
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
    fn test_missing_embed_url() {
        let mut page = make_page("https://example.com/video/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test Video"}),
        }];
        let findings = VideoObjectEmbedUrlValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "VIDEMB001"));
    }

    #[test]
    fn test_missing_content_url() {
        let mut page = make_page("https://example.com/video/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test Video", "embedUrl": "https://youtube.com/embed/123"}),
        }];
        let findings = VideoObjectEmbedUrlValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "VIDEMB002"));
    }

    #[test]
    fn test_valid_with_both_urls() {
        let mut page = make_page("https://example.com/video/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Test Video", "embedUrl": "https://youtube.com/embed/123", "contentUrl": "https://youtube.com/watch?v=123"}),
        }];
        let findings = VideoObjectEmbedUrlValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_non_video_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "Test"}),
        }];
        let findings = VideoObjectEmbedUrlValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        let findings = VideoObjectEmbedUrlValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_multiple_videos() {
        let mut page = make_page("https://example.com/videos");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("VideoObject".to_string()),
                data: serde_json::json!({"@type": "VideoObject", "name": "Video 1"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("VideoObject".to_string()),
                data: serde_json::json!({"@type": "VideoObject", "name": "Video 2"}),
            },
        ];
        let findings = VideoObjectEmbedUrlValidator::new().analyze(&make_ctx(&page));
        assert_eq!(findings.len(), 4);
    }

    #[test]
    fn test_one_valid_one_invalid() {
        let mut page = make_page("https://example.com/videos");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("VideoObject".to_string()),
                data: serde_json::json!({"@type": "VideoObject", "name": "Video 1", "embedUrl": "https://youtube.com/embed/1", "contentUrl": "https://youtube.com/watch?v=1"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("VideoObject".to_string()),
                data: serde_json::json!({"@type": "VideoObject", "name": "Video 2"}),
            },
        ];
        let findings = VideoObjectEmbedUrlValidator::new().analyze(&make_ctx(&page));
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_name() {
        assert_eq!(VideoObjectEmbedUrlValidator::new().name(), "video-object-embed-url");
    }
}
