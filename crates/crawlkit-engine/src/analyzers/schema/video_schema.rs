use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct VideoSchemaValidator;

impl Default for VideoSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for VideoSchemaValidator {
    fn name(&self) -> &str {
        "video-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("VideoObject") {
                continue;
            }
            let data = &sd.data;

            // VID001: Missing embedUrl
            if data.get("embedUrl").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "VID001".to_string(),
                    title: "VideoObject missing embedUrl".to_string(),
                    description: "A VideoObject structured data block is missing the required \
                                 \"embedUrl\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"embedUrl\" with the URL of the embedded video player."
                        .to_string(),
                });
            }

            // VID002: Missing thumbnailUrl
            if data.get("thumbnailUrl").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "VID002".to_string(),
                    title: "VideoObject missing thumbnailUrl".to_string(),
                    description: "A VideoObject structured data block is missing the \
                                 \"thumbnailUrl\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"thumbnailUrl\" with a URL to the video thumbnail image."
                        .to_string(),
                });
            }

            // VID003: Missing duration
            if data.get("duration").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "VID003".to_string(),
                    title: "VideoObject missing duration".to_string(),
                    description: "A VideoObject structured data block is missing the \
                                 \"duration\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"duration\" with an ISO 8601 duration value (e.g., PT1H30M)."
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
fn test_video_missing_embed_url() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("VideoObject".to_string()),
        data: serde_json::json!({"@type": "VideoObject", "name": "Demo"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(VideoSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "VID001"));
}


    #[test]
fn test_video_missing_thumbnail() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("VideoObject".to_string()),
        data: serde_json::json!({"@type": "VideoObject", "name": "Demo", "embedUrl": "https://youtube.com/embed/123"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(VideoSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "VID002"));
}


    #[test]
fn test_video_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("VideoObject".to_string()),
        data: serde_json::json!({"@type": "VideoObject", "name": "Demo", "embedUrl": "https://youtube.com/embed/123", "thumbnailUrl": "https://example.com/thumb.jpg", "duration": "PT10M"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = VideoSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
