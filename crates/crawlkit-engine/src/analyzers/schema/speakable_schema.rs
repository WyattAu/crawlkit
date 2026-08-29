use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct SpeakableSchemaValidator;

impl Default for SpeakableSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeakableSchemaValidator {
    pub fn new() -> Self {
    #[allow(clippy::unwrap_used)]
        Self
    }
}

impl Analyzer for SpeakableSchemaValidator {
    fn name(&self) -> &str {
        "speakable-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let speakable = sd.data.get("speakable");
            if speakable.is_none() {
                continue;
            }
            let speakable = speakable.unwrap();

            // Handle both object and array forms
            let speakables: Vec<&serde_json::Value> = if let Some(arr) = speakable.as_array() {
                arr.iter().collect()
            } else {
                vec![speakable]
            };

            for s in &speakables {
                let has_xpath = s.get("xpath").is_some();
                let has_css_selector = s.get("cssSelector").is_some();

                // SPEAK001: Speakable present but missing xpath
                if !has_xpath {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "SPEAK001".to_string(),
                        title: "Speakable schema missing xpath".to_string(),
                        description: "A Speakable structured data property is present but does \
                                      not specify an \"xpath\" selector. XPath helps voice \
                                      assistants identify which content to read aloud."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"xpath\" with an XPath expression pointing to the \
                                         speakable content."
                            .to_string(),
                    });
                }

                // SPEAK002: Speakable present but missing cssSelector
                if !has_css_selector {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "SPEAK002".to_string(),
                        title: "Speakable schema missing cssSelector".to_string(),
                        description: "A Speakable structured data property is present but does \
                                      not specify a \"cssSelector\". CSS selectors provide an \
                                      alternative way to identify speakable content."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"cssSelector\" with a CSS selector pointing to the \
                                         speakable content."
                            .to_string(),
                    });
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
fn test_speakable_missing_xpath() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("WebPage".to_string()),
        data: serde_json::json!({
            "@type": "WebPage",
            "speakable": {"cssSelector": ".intro"}
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SpeakableSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SPEAK001"));
}


    #[test]
fn test_speakable_missing_css_selector() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("WebPage".to_string()),
        data: serde_json::json!({
            "@type": "WebPage",
            "speakable": {"xpath": ["/html/body/h1"]}
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SpeakableSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SPEAK002"));
}


    #[test]
fn test_speakable_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("WebPage".to_string()),
        data: serde_json::json!({
            "@type": "WebPage",
            "speakable": {"xpath": ["/html/body/h1"], "cssSelector": ".intro"}
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SpeakableSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_speakable_no_speakable_property() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("WebPage".to_string()),
        data: serde_json::json!({"@type": "WebPage", "name": "Home"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SpeakableSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_speakable_array_form() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("WebPage".to_string()),
        data: serde_json::json!({
            "@type": "WebPage",
            "speakable": [
                {"xpath": ["/html/body/h1"]},
                {"cssSelector": ".intro"}
            ]
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SpeakableSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SPEAK002"));
}


    #[test]
fn test_speakable_array_form_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("WebPage".to_string()),
        data: serde_json::json!({
            "@type": "WebPage",
            "speakable": [{"@type": "SpeakableSpecification"}]
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SpeakableSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SPEAK001"));
    assert!(findings.iter().any(|f| f.code == "SPEAK002"));
}


    #[test]
fn test_speakable_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    assert!(SpeakableSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_speakable_array_form_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("WebPage".to_string()),
        data: serde_json::json!({
            "@type": "WebPage",
            "speakable": [
                {"xpath": ["/html/body/h1"], "cssSelector": ".intro"},
                {"xpath": ["/html/body/p"], "cssSelector": "main"}
            ]
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(SpeakableSchemaValidator::new().analyze(&ctx).is_empty());
}


}
