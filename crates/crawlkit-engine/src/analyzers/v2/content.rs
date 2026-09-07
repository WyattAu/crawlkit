#![allow(
    clippy::unwrap_used,
    clippy::manual_range_contains,
    clippy::redundant_closure,
    clippy::collapsible_if,
    clippy::unnecessary_map_or,
    clippy::default_constructed_unit_structs,
    clippy::needless_return,
    clippy::needless_range_loop,
    clippy::useless_format,
    clippy::if_same_then_else,
    clippy::derivable_impls,
    clippy::manual_pattern_char_comparison,
    clippy::manual_contains,
    clippy::collapsible_match,
    clippy::redundant_clone,
    clippy::useless_conversion
)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct DuplicateContentDetectorV2;
impl Default for DuplicateContentDetectorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl DuplicateContentDetectorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for DuplicateContentDetectorV2 {
    fn name(&self) -> &str {
        "duplicate-content-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let trimmed = body.trim();
            if trimmed.len() < 200 {
                return findings;
            }
            let normalized: String = trimmed
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .to_lowercase();
            let chunk_size = 200;
            let mut seen: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            // Build char-boundary byte offsets to avoid slicing multi-byte UTF-8 chars.
            let char_offsets: Vec<usize> = std::iter::once(0)
                .chain(normalized.char_indices().map(|(i, _)| i))
                .chain(std::iter::once(normalized.len()))
                .collect();
            let total_chars = char_offsets.len() - 1;
            for i in (0..total_chars).step_by(chunk_size / 2) {
                let byte_start = char_offsets[i];
                let byte_end_idx = (i + chunk_size).min(total_chars);
                let byte_end = char_offsets[byte_end_idx];
                let chunk = &normalized[byte_start..byte_end];
                if chunk.len() >= 50 {
                    *seen.entry(chunk.to_string()).or_insert(0) += 1;
                }
            }
            let dup_count: usize = seen.values().filter(|&&c| c > 2).count();
            if dup_count > 3 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "DUP-V2001".to_string(),
                    title: "Repeated text chunks detected".to_string(),
                    description: format!(
                        "{dup_count} text chunks appear 3+ times, suggesting boilerplate content."
                    ),
                    url: url.clone(),
                    recommendation:
                        "Reduce repetitive boilerplate and ensure unique value on each page."
                            .to_string(),
                });
            }
        }
        findings
    }
}

pub struct ArticleWordCountAnalyzer;
impl Default for ArticleWordCountAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl ArticleWordCountAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ArticleWordCountAnalyzer {
    fn name(&self) -> &str {
        "article-word-count-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let is_article = ctx.page.structured_data.iter().any(|sd| {
            sd.r#type.as_deref() == Some("Article") || sd.r#type.as_deref() == Some("BlogPosting")
        });
        if !is_article {
            return findings;
        }
        if ctx.page.word_count < 300 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "ARTWC-V5001".to_string(),
                title: "Article too short".to_string(),
                description: format!("{} words, recommended 300+.", ctx.page.word_count),
                url: url.clone(),
                recommendation: "Expand article to 300+ words.".to_string(),
            });
        }
        findings
    }
}

pub struct ArticleAuthorUrlValidator;
impl Default for ArticleAuthorUrlValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ArticleAuthorUrlValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ArticleAuthorUrlValidator {
    fn name(&self) -> &str {
        "article-author-url-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Article" && t != "BlogPosting" && t != "NewsArticle" {
                continue;
            }
            if let Some(author) = sd.data.get("author") {
                if author.is_string() {
                    continue;
                }
                if let Some(obj) = author.as_object() {
                    if !obj.contains_key("url") && !obj.contains_key("@id") {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Content,
                            code: "ARTAUTH-V5001".to_string(),
                            title: "Author missing URL".to_string(),
                            description: "Author object has no url or @id.".to_string(),
                            url: url.clone(),
                            recommendation: "Add url to author object.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

pub struct ArticleDateModifiedValidator;
impl Default for ArticleDateModifiedValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ArticleDateModifiedValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ArticleDateModifiedValidator {
    fn name(&self) -> &str {
        "article-date-modified-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Article" && t != "BlogPosting" && t != "NewsArticle" {
                continue;
            }
            if sd
                .data
                .get("dateModified")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Content,
                    code: "ARTMOD-V5001".to_string(),
                    title: "Article missing dateModified".to_string(),
                    description: "No dateModified in structured data.".to_string(),
                    url: url.clone(),
                    recommendation: "Add dateModified to indicate freshness.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ArticleMissingHeadlineValidatorV2;
impl Default for ArticleMissingHeadlineValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ArticleMissingHeadlineValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ArticleMissingHeadlineValidatorV2 {
    fn name(&self) -> &str {
        "article-missing-headline-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Article" {
                continue;
            }
            if sd
                .data
                .get("headline")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ARTHL-V2001".to_string(),
                    title: "Article missing headline".to_string(),
                    description: "Article schema is missing or has empty headline.".to_string(),
                    url: url.clone(),
                    recommendation: "Add headline to Article structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ArticleMissingDatePublishedValidatorV2;
impl Default for ArticleMissingDatePublishedValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ArticleMissingDatePublishedValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ArticleMissingDatePublishedValidatorV2 {
    fn name(&self) -> &str {
        "article-missing-date-published-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Article" {
                continue;
            }
            if sd.data.get("datePublished").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ARTDT-V2001".to_string(),
                    title: "Article missing datePublished".to_string(),
                    description: "Article schema is missing the datePublished property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add datePublished to Article structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ArticleMissingAuthorValidatorV2;
impl Default for ArticleMissingAuthorValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ArticleMissingAuthorValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ArticleMissingAuthorValidatorV2 {
    fn name(&self) -> &str {
        "article-missing-author-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Article" {
                continue;
            }
            if sd.data.get("author").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ARTAUTH-V2001".to_string(),
                    title: "Article missing author".to_string(),
                    description: "Article schema is missing the author property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add author to Article structured data.".to_string(),
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
        body: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body,
            status_code: Some(200),
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
    fn test_duplicate_content_v2_empty() {
        assert!(DuplicateContentDetectorV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_duplicate_content_v2_short() {
        assert!(DuplicateContentDetectorV2::new()
            .analyze(&make_ctx(
                &make_page("https://example.com"),
                Some("short body")
            ))
            .is_empty());
    }
    #[test]
    fn test_article_word_count_no_sd() {
        let p = make_page("https://example.com");
        assert!(ArticleWordCountAnalyzer::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_article_word_count_short() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({}),
        }];
        p.word_count = 100;
        let f = ArticleWordCountAnalyzer::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "ARTWC-V5001");
    }
    #[test]
    fn test_article_word_count_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({}),
        }];
        p.word_count = 500;
        assert!(ArticleWordCountAnalyzer::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_article_author_url_no_url() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"author": {"@type": "Person", "name": "John"}}),
        }];
        let f = ArticleAuthorUrlValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_article_author_url_string() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"author": "John"}),
        }];
        assert!(ArticleAuthorUrlValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_article_date_modified_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("BlogPosting".into()),
            data: serde_json::json!({}),
        }];
        let f = ArticleDateModifiedValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_article_date_modified_present() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"dateModified": "2025-01-01"}),
        }];
        assert!(ArticleDateModifiedValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_article_headline_v8_missing() {
        let mut p = make_page("https://example.com/art");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let f = ArticleMissingHeadlineValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "ARTHL-V2001");
    }
    #[test]
    fn test_article_headline_v8_present() {
        let mut p = make_page("https://example.com/art");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"@type": "Article", "headline": "News"}),
        }];
        assert!(ArticleMissingHeadlineValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_article_date_v8_missing() {
        let mut p = make_page("https://example.com/art");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let f = ArticleMissingDatePublishedValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "ARTDT-V2001");
    }
    #[test]
    fn test_article_date_v8_present() {
        let mut p = make_page("https://example.com/art");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"@type": "Article", "datePublished": "2023-01-01"}),
        }];
        assert!(ArticleMissingDatePublishedValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_article_author_v8_missing() {
        let mut p = make_page("https://example.com/art");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let f = ArticleMissingAuthorValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "ARTAUTH-V2001");
    }
    #[test]
    fn test_article_author_v8_present() {
        let mut p = make_page("https://example.com/art");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"@type": "Article", "author": "Me"}),
        }];
        assert!(ArticleMissingAuthorValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
}
