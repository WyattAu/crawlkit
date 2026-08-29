use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct PersonSchemaValidator;

impl Default for PersonSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PersonSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PersonSchemaValidator {
    fn name(&self) -> &str {
        "person-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Person" {
                continue;
            }
            let data = &sd.data;

            // PERS001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "PERS001".to_string(),
                    title: "Person schema missing name".to_string(),
                    description: "A Person structured data block is missing the required \
                                  \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the person's full name.".to_string(),
                });
            }

            // PERS002: Missing sameAs
            if data.get("sameAs").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PERS002".to_string(),
                    title: "Person schema missing sameAs".to_string(),
                    description: "A Person structured data block is missing the \"sameAs\" \
                                  property. sameAs links to social profiles and helps build \
                                  the Knowledge Graph."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"sameAs\" with an array of URLs to social profiles \
                                     (e.g., LinkedIn, Twitter)."
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
fn test_person_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Person".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Person"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = PersonSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PERS001"));
}


    #[test]
fn test_person_missing_same_as() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Person".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Person",
            "name": "John Doe"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = PersonSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PERS002"));
}


    #[test]
fn test_person_all_present() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Person".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Person",
            "name": "John Doe",
            "sameAs": ["https://twitter.com/johndoe"]
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = PersonSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_person_missing_both() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Person".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Person"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = PersonSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PERS001"));
    assert!(findings.iter().any(|f| f.code == "PERS002"));
}


    #[test]
fn test_person_non_person_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = PersonSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_person_no_schema_no_findings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = PersonSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
