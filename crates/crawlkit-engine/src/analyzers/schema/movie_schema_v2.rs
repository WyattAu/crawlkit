#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct MovieSchemaValidatorV2;

impl Default for MovieSchemaValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl MovieSchemaValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MovieSchemaValidatorV2 {
    fn name(&self) -> &str {
        "movie-schema-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Movie") {
                continue;
            }
            let data = &sd.data;

            if data.get("director").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "MOVIE-V2001".to_string(),
                    title: "Movie schema missing director".to_string(),
                    description: "A Movie structured data block is missing the \"director\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"director\" with the movie director.".to_string(),
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
    fn test_movie_v2_missing_director() {
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
        let findings = MovieSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOVIE-V2001"));
    }

    #[test]
    fn test_movie_v2_valid() {
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
        let findings = MovieSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_movie_v2_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(MovieSchemaValidatorV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_movie_v2_non_movie_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(MovieSchemaValidatorV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_movie_v2_multiple_movies() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Movie".to_string()),
                data: serde_json::json!({"@type": "Movie", "name": "Good", "director": "X"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Movie".to_string()),
                data: serde_json::json!({"@type": "Movie", "name": "Bad"}),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = MovieSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOVIE-V2001"));
    }

    #[test]
    fn test_movie_v2_director_is_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({"@type": "Movie", "name": "X", "director": "Nolan"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(MovieSchemaValidatorV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_movie_v2_director_is_object() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({"@type": "Movie", "name": "X", "director": {"@type": "Person", "name": "Nolan"}}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(MovieSchemaValidatorV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_movie_v2_director_empty_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({"@type": "Movie", "name": "X", "director": ""}),
        }];
        let ctx = make_ctx(&page, None);
        // Empty string is still "present" (is_some), so no finding
        assert!(MovieSchemaValidatorV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_movie_v2_severity_warning() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({"@type": "Movie", "name": "X"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = MovieSchemaValidatorV2::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].category, IssueCategory::Schema);
    }

    #[test]
    fn test_movie_v2_name_present_director_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({"@type": "Movie", "name": "Inception"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = MovieSchemaValidatorV2::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "MOVIE-V2001");
    }
}
