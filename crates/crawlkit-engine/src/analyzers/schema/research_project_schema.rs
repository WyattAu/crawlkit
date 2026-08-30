#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ResearchProjectSchemaValidator;

impl Default for ResearchProjectSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResearchProjectSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ResearchProjectSchemaValidator {
    fn name(&self) -> &str {
        "researchproject-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("ResearchProject") {
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
                    code: "RPROJ001".to_string(),
                    title: "ResearchProject schema missing name".to_string(),
                    description: "A ResearchProject structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the research project name.".to_string(),
                });
            }

            if data.get("about").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RPROJ002".to_string(),
                    title: "ResearchProject schema missing about".to_string(),
                    description:
                        "A ResearchProject structured data block is missing the \"about\" \
                                  property."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"about\" with the topic or subject of the project."
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
    fn test_researchproject_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RPROJ001"));
    }

    #[test]
    fn test_researchproject_missing_about() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject",
                "name": "Climate Study"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RPROJ002"));
    }

    #[test]
    fn test_researchproject_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject",
                "name": "Climate Study",
                "about": "Climate Change"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_researchproject_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_researchproject_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_researchproject_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_researchproject_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RPROJ001"));
    }

    #[test]
    fn test_researchproject_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ResearchProject".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ResearchProject",
                    "name": "Good Project",
                    "about": "Topic"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ResearchProject".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ResearchProject"
                }),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RPROJ001"));
        assert!(findings.iter().any(|f| f.code == "RPROJ002"));
    }
}
