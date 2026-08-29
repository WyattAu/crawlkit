#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct TVSeriesSchemaValidator;

impl Default for TVSeriesSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TVSeriesSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TVSeriesSchemaValidator {
    fn name(&self) -> &str {
        "tvseries-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("TVSeries") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "TV001".to_string(),
                    title: "TVSeries schema missing name".to_string(),
                    description: "A TVSeries structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the TV series title."
                        .to_string(),
                });
            }

            if data.get("numberOfEpisodes").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "TV002".to_string(),
                    title: "TVSeries schema missing numberOfEpisodes".to_string(),
                    description: "A TVSeries structured data block is missing the \
                                  \"numberOfEpisodes\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"numberOfEpisodes\" with the total episode count."
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
fn test_tvseries_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("TVSeries".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "TVSeries",
            "numberOfEpisodes": 10
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TV001"));
}


    #[test]
fn test_tvseries_missing_numberofepisodes() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("TVSeries".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "TVSeries",
            "name": "Breaking Bad"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TV002"));
}


    #[test]
fn test_tvseries_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("TVSeries".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "TVSeries",
            "name": "Breaking Bad",
            "numberOfEpisodes": 62
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_tvseries_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_tvseries_non_tvseries_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Movie".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Movie"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_tvseries_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("TVSeries".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "TVSeries"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 2);
}


    #[test]
fn test_tvseries_name_empty() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("TVSeries".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "TVSeries",
            "name": ""
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TV001"));
}


    #[test]
fn test_tvseries_multiple() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVSeries".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "TVSeries",
                "name": "Good Series"
            }),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVSeries".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "TVSeries"
            }),
        },
    ];
    let ctx = make_ctx(&page, None);
    let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TV001"));
    assert!(findings.iter().any(|f| f.code == "TV002"));
}


}
