#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct CreativeWorkSchemaValidator;

impl Default for CreativeWorkSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CreativeWorkSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CreativeWorkSchemaValidator {
    fn name(&self) -> &str {
        "creative-work-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("CreativeWork") {
                continue;
            }
            let data = &sd.data;

            if data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CREATIVE001".to_string(),
                    title: "CreativeWork schema missing name".to_string(),
                    description: "A CreativeWork structured data block is missing the \"name\" \
                                  property. Search engines use this to understand the work title."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the title of the creative work.".to_string(),
                });
            }

            if data.get("author").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "CREATIVE002".to_string(),
                    title: "CreativeWork schema missing author".to_string(),
                    description: "A CreativeWork structured data block is missing the \"author\" \
                                  property. Author attribution helps establish E-E-A-T signals."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"author\" with a Person or Organization object."
                        .to_string(),
                });
            }

            if data.get("datePublished").is_none() && data.get("dateCreated").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "CREATIVE003".to_string(),
                    title: "CreativeWork schema missing date".to_string(),
                    description: "A CreativeWork structured data block has neither \"datePublished\" \
                                  nor \"dateCreated\". Dates help search engines understand content freshness."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"datePublished\" with an ISO 8601 date."
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

    fn make_ctx<'a>(page: &'a crate::parser::ParsedPage) -> AnalysisContext<'a> {
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
    fn test_creative_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("CreativeWork".to_string()),
            data: serde_json::json!({"@type": "CreativeWork"}),
        }];
        assert!(CreativeWorkSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "CREATIVE001"));
    }

    #[test]
    fn test_creative_missing_author() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("CreativeWork".to_string()),
            data: serde_json::json!({"@type": "CreativeWork", "name": "Title"}),
        }];
        assert!(CreativeWorkSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "CREATIVE002"));
    }

    #[test]
    fn test_creative_missing_dates() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("CreativeWork".to_string()),
            data: serde_json::json!({"@type": "CreativeWork", "name": "Title"}),
        }];
        assert!(CreativeWorkSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "CREATIVE003"));
    }

    #[test]
    fn test_creative_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("CreativeWork".to_string()),
            data: serde_json::json!({"@type": "CreativeWork", "name": "Title", "author": {"@type": "Person", "name": "Author"}, "datePublished": "2024-01-01"}),
        }];
        assert!(CreativeWorkSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_creative_non_creative_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        assert!(CreativeWorkSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_creative_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(CreativeWorkSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_creative_multiple_issues() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("CreativeWork".to_string()),
            data: serde_json::json!({"@type": "CreativeWork"}),
        }];
        let findings = CreativeWorkSchemaValidator::new().analyze(&make_ctx(&page));
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn test_creative_with_date_created() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("CreativeWork".to_string()),
            data: serde_json::json!({"@type": "CreativeWork", "name": "Title", "dateCreated": "2024-01-01"}),
        }];
        assert!(!CreativeWorkSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "CREATIVE003"));
    }

    #[test]
    fn test_creative_name_only() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("CreativeWork".to_string()),
            data: serde_json::json!({"@type": "CreativeWork", "name": "Title"}),
        }];
        let findings = CreativeWorkSchemaValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "CREATIVE002"));
        assert!(findings.iter().any(|f| f.code == "CREATIVE003"));
        assert!(!findings.iter().any(|f| f.code == "CREATIVE001"));
    }

    #[test]
    fn test_creative_empty_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("CreativeWork".to_string()),
            data: serde_json::json!({"@type": "CreativeWork", "name": ""}),
        }];
        assert!(CreativeWorkSchemaValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "CREATIVE001"));
    }
}
