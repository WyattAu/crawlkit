#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct MovieSchemaValidator;

impl Default for MovieSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MovieSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MovieSchemaValidator {
    fn name(&self) -> &str {
        "movie-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Movie") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "MOVIE001".to_string(),
                    title: "Movie schema missing name".to_string(),
                    description: "A Movie structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the movie title."
                        .to_string(),
                });
            }

            if data.get("director").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "MOVIE002".to_string(),
                    title: "Movie schema missing director".to_string(),
                    description: "A Movie structured data block is missing the \"director\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"director\" with the movie director."
                        .to_string(),
                });
            }

            if data.get("dateCreated").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MOVIE003".to_string(),
                    title: "Movie schema missing dateCreated".to_string(),
                    description: "A Movie structured data block is missing the \"dateCreated\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"dateCreated\" with the movie release date."
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
fn test_movie_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Movie".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Movie",
            "director": "Spielberg"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = MovieSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOVIE001"));
}


    #[test]
fn test_movie_missing_director() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Movie".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Movie",
            "name": "E.T."
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = MovieSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOVIE002"));
}


    #[test]
fn test_movie_missing_datecreated() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Movie".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Movie",
            "name": "E.T.",
            "director": "Spielberg"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = MovieSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOVIE003"));
}


    #[test]
fn test_movie_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Movie".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Movie",
            "name": "E.T.",
            "director": "Spielberg",
            "dateCreated": "1982-06-11"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = MovieSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_movie_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = MovieSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_movie_non_movie_type_ignored() {
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
    let findings = MovieSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_movie_all_missing() {
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
    let findings = MovieSchemaValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 3);
}


    #[test]
fn test_movie_name_empty() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Movie".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Movie",
            "name": ""
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = MovieSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOVIE001"));
}


}
