use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct OrganizationLogoValidator;

impl OrganizationLogoValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OrganizationLogoValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for OrganizationLogoValidator {
    fn name(&self) -> &str {
        "organization-logo"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.r#type.as_deref();
            if schema_type != Some("Organization") && schema_type != Some("LocalBusiness") {
                continue;
            }
            let data = &sd.data;

            if data.get("logo").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "OLOGO001".to_string(),
                    title: "Organization missing logo".to_string(),
                    description: "An Organization structured data block is missing the \"logo\" \
                                  property. The logo is used for knowledge panel display."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"logo\" with a URL string or ImageObject pointing to \
                                     the organization's logo."
                        .to_string(),
                });
                continue;
            }

            if let Some(logo) = data.get("logo") {
                let logo_str = logo.as_str().unwrap_or("");
                if !logo_str.is_empty()
                    && !logo_str.starts_with("http://")
                    && !logo_str.starts_with("https://")
                    && logo.get("@type").is_none()
                {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "OLOGO002".to_string(),
                        title: "Organization logo URL invalid format".to_string(),
                        description: format!(
                            "The logo value \"{logo_str}\" is not a valid absolute URL."
                        ),
                        url: url.clone(),
                        recommendation: "Use an absolute URL (https://...) for the logo property."
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
fn test_ologo_missing_logo() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Organization".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Organization",
            "name": "Acme Corp"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OrganizationLogoValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OLOGO001"));
}


    #[test]
fn test_ologo_valid_logo() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Organization".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Organization",
            "name": "Acme Corp",
            "logo": "https://example.com/logo.png"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OrganizationLogoValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_ologo_invalid_url() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Organization".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Organization",
            "name": "Acme Corp",
            "logo": "/images/logo.png"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OrganizationLogoValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OLOGO002"));
}


    #[test]
fn test_ologo_local_business_type() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("LocalBusiness".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "LocalBusiness",
            "name": "My Shop"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OrganizationLogoValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "OLOGO001"));
}


    #[test]
fn test_ologo_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = OrganizationLogoValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_ologo_non_org_ignored() {
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
    let findings = OrganizationLogoValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_ologo_logo_object() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Organization".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Organization",
            "name": "Acme Corp",
            "logo": {
                "@type": "ImageObject",
                "url": "https://example.com/logo.png"
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OrganizationLogoValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_ologo_empty_logo_string() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Organization".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Organization",
            "name": "Acme Corp",
            "logo": ""
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = OrganizationLogoValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
