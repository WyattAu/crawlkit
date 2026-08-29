use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ArticleSchemaValidator;

impl Default for ArticleSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ArticleSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    const ARTICLE_TYPES: &[&str] = &[
        "Article",
        "NewsArticle",
        "BlogPosting",
        "ScholarlyArticle",
        "TechArticle",
        "Report",
    ];
}

impl Analyzer for ArticleSchemaValidator {
    fn name(&self) -> &str {
        "article-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if !Self::ARTICLE_TYPES.contains(&schema_type) {
                continue;
            }
            let data = &sd.data;

            // ART001: Missing headline
            if data.get("headline").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "ART001".to_string(),
                    title: "Article schema missing headline".to_string(),
                    description: format!(
                        "A {schema_type} structured data block is missing the required \
                         \"headline\" property."
                    ),
                    url: url.clone(),
                    recommendation: "Add \"headline\" with the article title.".to_string(),
                });
            }

            // ART002: Missing datePublished
            if data.get("datePublished").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "ART002".to_string(),
                    title: "Article schema missing datePublished".to_string(),
                    description: format!(
                        "A {schema_type} structured data block is missing the required \
                         \"datePublished\" property."
                    ),
                    url: url.clone(),
                    recommendation: "Add \"datePublished\" with an ISO 8601 date value."
                        .to_string(),
                });
            }

            // ART003: Missing author
            if data.get("author").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "ART003".to_string(),
                    title: "Article schema missing author".to_string(),
                    description: format!(
                        "A {schema_type} structured data block is missing the required \
                         \"author\" property."
                    ),
                    url: url.clone(),
                    recommendation: "Add \"author\" with a Person or Organization object."
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
fn test_article_missing_headline() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article",
            "author": "John"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ArticleSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ART001"));
}


    #[test]
fn test_article_missing_date_published() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": "Test",
            "author": "John"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ArticleSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ART002"));
}


    #[test]
fn test_article_missing_author() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": "Test",
            "datePublished": "2024-01-01"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ArticleSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ART003"));
}


    #[test]
fn test_article_all_present() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": "Test",
            "datePublished": "2024-01-01",
            "author": "John"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ArticleSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_article_missing_all_three() {
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
    let findings = ArticleSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ART001"));
    assert!(findings.iter().any(|f| f.code == "ART002"));
    assert!(findings.iter().any(|f| f.code == "ART003"));
}


    #[test]
fn test_article_news_article_type() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("NewsArticle".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "NewsArticle"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ArticleSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ART001"));
}


    #[test]
fn test_article_blog_posting_type() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("BlogPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "BlogPosting"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = ArticleSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ART001"));
    assert!(findings.iter().any(|f| f.code == "ART003"));
}


    #[test]
fn test_article_no_schema_no_findings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = ArticleSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_article_non_article_type_ignored() {
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
    let findings = ArticleSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
