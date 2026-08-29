use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct SpecialAnnouncementSchemaValidator;

impl Default for SpecialAnnouncementSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecialAnnouncementSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SpecialAnnouncementSchemaValidator {
    fn name(&self) -> &str {
        "special-announcement-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("SpecialAnnouncement") {
                continue;
            }
            let data = &sd.data;

            // SPEC001: Missing datePosted
            if data.get("datePosted").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "SPEC001".to_string(),
                    title: "SpecialAnnouncement missing datePosted".to_string(),
                    description: "A SpecialAnnouncement structured data block is missing the \
                                  required \"datePosted\" property. The date indicates when the \
                                  announcement was published."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"datePosted\" with an ISO 8601 date value."
                        .to_string(),
                });
            }

            // SPEC002: Missing category
            if data.get("category").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SPEC002".to_string(),
                    title: "SpecialAnnouncement missing category".to_string(),
                    description: "A SpecialAnnouncement structured data block is missing the \
                                  \"category\" property. The category classifies the type of \
                                  announcement."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"category\" with a URL from the Schema.org vocabulary \
                                     (e.g., https://schema.org/EmergencyAlert)."
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
fn test_special_announcement_missing_date_posted() {
    let mut page = make_page("https://example.com/announce");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SpecialAnnouncement".to_string()),
        data: serde_json::json!({
            "@type": "SpecialAnnouncement",
            "category": "https://schema.org/EmergencyAlert"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SPEC001"));
}


    #[test]
fn test_special_announcement_missing_category() {
    let mut page = make_page("https://example.com/announce");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SpecialAnnouncement".to_string()),
        data: serde_json::json!({
            "@type": "SpecialAnnouncement",
            "datePosted": "2025-01-15"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SPEC002"));
}


    #[test]
fn test_special_announcement_valid() {
    let mut page = make_page("https://example.com/announce");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SpecialAnnouncement".to_string()),
        data: serde_json::json!({
            "@type": "SpecialAnnouncement",
            "datePosted": "2025-01-15",
            "category": "https://schema.org/EmergencyAlert"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_special_announcement_non_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Event".to_string()),
        data: serde_json::json!({"@type": "Event", "name": "Concert"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_special_announcement_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_special_announcement_missing_all_fields() {
    let mut page = make_page("https://example.com/announce");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("SpecialAnnouncement".to_string()),
        data: serde_json::json!({"@type": "SpecialAnnouncement"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SpecialAnnouncementSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SPEC001"));
    assert!(findings.iter().any(|f| f.code == "SPEC002"));
}


    #[test]
fn test_special_announcement_multiple_announcements() {
    let mut page = make_page("https://example.com/announce");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SpecialAnnouncement".to_string()),
            data: serde_json::json!({"@type": "SpecialAnnouncement"}),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SpecialAnnouncement".to_string()),
            data: serde_json::json!({"@type": "SpecialAnnouncement"}),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = SpecialAnnouncementSchemaValidator::new().analyze(&ctx);
    let spec001_count = findings.iter().filter(|f| f.code == "SPEC001").count();
    let spec002_count = findings.iter().filter(|f| f.code == "SPEC002").count();
    assert_eq!(spec001_count, 2);
    assert_eq!(spec002_count, 2);
}


}
