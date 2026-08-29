use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ActionSchemaValidator;

impl Default for ActionSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ActionSchemaValidator {
    fn name(&self) -> &str {
        "action-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Action") {
                continue;
            }
            let data = &sd.data;

            if data.get("actionType").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ACTION001".to_string(),
                    title: "Action schema missing actionType".to_string(),
                    description: "An Action structured data block is missing the \"actionType\" \
                                  property. Search engines use this to understand the action kind."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"actionType\" with a specific action type (e.g., \
                                     \"BuyAction\", \"ViewAction\")."
                        .to_string(),
                });
            }

            let has_target = match data.get("target") {
                None | Some(serde_json::Value::Null) => false,
                Some(serde_json::Value::String(s)) => !s.is_empty(),
                Some(serde_json::Value::Object(_)) => true,
                Some(serde_json::Value::Array(a)) => !a.is_empty(),
                Some(_) => true,
            };
            if !has_target {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ACTION002".to_string(),
                    title: "Action schema missing target".to_string(),
                    description: "An Action structured data block is missing the \"target\" \
                                  property. The target defines where the action leads."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"target\" with an EntryPoint or URL string."
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
fn test_action_missing_action_type() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Action".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Action"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ACTION001"));
}


    #[test]
fn test_action_missing_target() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Action".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Action",
            "actionType": "BuyAction"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ACTION002"));
}


    #[test]
fn test_action_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Action".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Action",
            "actionType": "BuyAction",
            "target": "https://example.com/buy"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_action_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_action_non_action_type_ignored() {
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
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_action_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Action".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Action"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 2);
    assert!(findings.iter().any(|f| f.code == "ACTION001"));
    assert!(findings.iter().any(|f| f.code == "ACTION002"));
}


    #[test]
fn test_action_action_type_empty() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Action".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Action",
            "actionType": ""
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ACTION001"));
}


    #[test]
fn test_action_target_empty() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Action".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Action",
            "actionType": "ViewAction",
            "target": ""
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ACTION002"));
}


    #[test]
fn test_action_target_as_entry_point() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Action".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Action",
            "actionType": "BuyAction",
            "target": {
                "@type": "EntryPoint",
                "urlTemplate": "https://example.com/buy"
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_action_multiple_actions() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Action".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Action",
                "actionType": "BuyAction",
                "target": "https://example.com/buy"
            }),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Action".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Action"
            }),
        },
    ];
    let ctx = make_ctx(&page, None);
    let findings = ActionSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ACTION001"));
    assert!(findings.iter().any(|f| f.code == "ACTION002"));
}


}
