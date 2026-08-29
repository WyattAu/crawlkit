use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct OccupationSchemaValidator;

impl Default for OccupationSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OccupationSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for OccupationSchemaValidator {
    fn name(&self) -> &str {
        "occupation-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Occupation") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "OCCUP001".to_string(),
                    title: "Occupation schema missing name".to_string(),
                    description: "An Occupation structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the occupation title."
                        .to_string(),
                });
            }

            let has_category = match data.get("occupationalCategory") {
                None | Some(serde_json::Value::Null) => false,
                Some(serde_json::Value::String(s)) => !s.is_empty(),
                Some(serde_json::Value::Object(_)) => true,
                Some(serde_json::Value::Array(a)) => !a.is_empty(),
                Some(_) => true, // numbers, booleans
            };
            if !has_category {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "OCCUP002".to_string(),
                    title: "Occupation schema missing occupationalCategory".to_string(),
                    description: "An Occupation structured data block is missing the \
                                  \"occupationalCategory\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"occupationalCategory\" with a category code or text."
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
fn test_occupation_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Occupation".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Occupation"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OCCUP001"));
}


    #[test]
fn test_occupation_missing_occupational_category() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Occupation".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Occupation",
            "name": "Software Engineer"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OCCUP002"));
}


    #[test]
fn test_occupation_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Occupation".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Occupation",
            "name": "Software Engineer",
            "occupationalCategory": "15-1252.00"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_occupation_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_occupation_non_occupation_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Person".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Person",
            "name": "Jane"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_occupation_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Occupation".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Occupation"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 2);
    assert!(findings.iter().any(|f| f.code == "OCCUP001"));
    assert!(findings.iter().any(|f| f.code == "OCCUP002"));
}


    #[test]
fn test_occupation_name_empty() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Occupation".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Occupation",
            "name": ""
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OCCUP001"));
}


    #[test]
fn test_occupation_category_empty() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Occupation".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Occupation",
            "name": "Doctor",
            "occupationalCategory": ""
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OCCUP002"));
}


    #[test]
fn test_occupation_multiple() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Occupation",
                "name": "Engineer",
                "occupationalCategory": "17-2000"
            }),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Occupation"
            }),
        },
    ];
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OCCUP001"));
    assert!(findings.iter().any(|f| f.code == "OCCUP002"));
}


    #[test]
fn test_occupation_category_as_object() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Occupation".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Occupation",
            "name": "Nurse",
            "occupationalCategory": {"@type": "CategoryCode", "codeValue": "29-1141"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OccupationSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
