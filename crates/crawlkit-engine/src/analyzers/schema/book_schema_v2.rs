#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct BookSchemaValidatorV2;

impl Default for BookSchemaValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl BookSchemaValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for BookSchemaValidatorV2 {
    fn name(&self) -> &str {
        "book-schema-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Book") {
                continue;
            }
            let data = &sd.data;

            if data
                .get("isbn")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "BOOK001".to_string(),
                    title: "Book schema missing isbn".to_string(),
                    description: "A Book structured data block is missing the \"isbn\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"isbn\" with the book's ISBN-10 or ISBN-13.".to_string(),
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
    fn test_book_missing_isbn() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Book".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Book",
                "name": "Rust in Action"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BOOK001"));
    }

    #[test]
    fn test_book_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Book".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Book",
                "name": "Rust in Action",
                "isbn": "978-1617294558"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_book_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_book_non_book_type_ignored() {
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
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_book_isbn_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Book".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Book",
                "name": "Rust in Action",
                "isbn": ""
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BOOK001"));
    }

    #[test]
    fn test_book_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Book".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Book",
                    "name": "Good Book",
                    "isbn": "978-1234567890"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Book".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Book",
                    "name": "Bad Book"
                }),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BOOK001"));
    }

    #[test]
    fn test_book_name_only_no_isbn() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Book".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Book",
                "name": "My Book"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BOOK001"));
    }

    #[test]
    fn test_book_with_author_but_no_isbn() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Book".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Book",
                "name": "My Book",
                "author": "Author Name"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BOOK001"));
    }

    #[test]
    fn test_book_isbn_present_not_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Book".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Book",
                "name": "My Book",
                "isbn": "978-0-13-468599-1"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_book_isbn_missing_field() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Book".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Book",
                "name": "My Book"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "BOOK001");
    }

    #[test]
    fn test_book_severity_is_warning() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Book".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Book",
                "name": "My Book"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = BookSchemaValidatorV2::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].category, IssueCategory::Schema);
    }
}
