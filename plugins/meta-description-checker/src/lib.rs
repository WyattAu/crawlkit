//! crawlkit plugin: meta-description-checker.
//!
//! Flags missing, short (<70 chars), and overlong (>160 chars)
//! `<meta name="description">` content. Built for `wasm32-unknown-unknown`
//! and published in the first-party plugin index (`plugins/index/`).

use crawlkit_plugin_sdk::{AnalysisContext, Analyzer, Finding, Severity};

/// SEO guidance bounds for the meta description, in characters.
const MIN_LEN: usize = 70;
const MAX_LEN: usize = 160;

pub struct MetaDescriptionChecker;

impl MetaDescriptionChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MetaDescriptionChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MetaDescriptionChecker {
    fn name(&self) -> &str {
        "meta-description-checker"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        match extract_description(&ctx.html.to_lowercase()) {
            None => vec![Finding {
                severity: Severity::Error,
                category: "seo".into(),
                code: "META001".into(),
                title: "Missing meta description".into(),
                description: "No <meta name=\"description\"> tag with content was found \
                              in the document; search engines will synthesize a snippet."
                    .into(),
                url: ctx.url.clone(),
                recommendation: "Add <meta name=\"description\" content=\"...\"> with a \
                                 70-160 character summary of the page."
                    .into(),
            }],
            Some(description) => {
                let len = description.trim().chars().count();
                if len < MIN_LEN {
                    vec![Finding {
                        severity: Severity::Warning,
                        category: "seo".into(),
                        code: "META002".into(),
                        title: "Meta description too short".into(),
                        description: format!(
                            "Meta description is {len} characters; \
                             descriptions under {MIN_LEN} are often replaced \
                             by search-engine-generated snippets."
                        ),
                        url: ctx.url.clone(),
                        recommendation: format!(
                            "Expand the description to at least {MIN_LEN} characters."
                        ),
                    }]
                } else if len > MAX_LEN {
                    vec![Finding {
                        severity: Severity::Warning,
                        category: "seo".into(),
                        code: "META003".into(),
                        title: "Meta description too long".into(),
                        description: format!(
                            "Meta description is {len} characters; \
                             results pages truncate descriptions over {MAX_LEN}."
                        ),
                        url: ctx.url.clone(),
                        recommendation: format!(
                            "Trim the description to at most {MAX_LEN} characters."
                        ),
                    }]
                } else {
                    Vec::new()
                }
            }
        }
    }
}

/// Extract the `content` of the first `<meta name="description">` tag
/// from a lowercased document. A tag with empty/whitespace content
/// counts as missing.
fn extract_description(lower_html: &str) -> Option<String> {
    let mut cursor = 0;
    while let Some(offset) = lower_html[cursor..].find("<meta") {
        let tag_start = cursor + offset;
        let rest = &lower_html[tag_start + "<meta".len()..];
        let tag_end = rest.find('>')?;
        let tag = &rest[..tag_end];
        if attr_value(tag, "name").is_some_and(|v| v == "description") {
            return attr_value(tag, "content").filter(|v| !v.trim().is_empty());
        }
        cursor = tag_start + "<meta".len() + tag_end + 1;
    }
    None
}

/// Value of attribute `attr` in a lowercased tag body, supporting
/// single-quoted, double-quoted, and unquoted values.
fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let name_start = i;
        while i < bytes.len()
            && bytes[i] != b'='
            && bytes[i] != b'/'
            && !bytes[i].is_ascii_whitespace()
        {
            i += 1;
        }
        let name = &tag[name_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if name.is_empty() || i >= bytes.len() || bytes[i] != b'=' {
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let value = if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
            let quote = bytes[i];
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            let v = &tag[start..i.min(bytes.len())];
            i += 1;
            v
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b'/' && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            &tag[start..i]
        };
        if name == attr {
            return Some(value.to_string());
        }
    }
    None
}

crawlkit_plugin_sdk::export_analyzer!(MetaDescriptionChecker);

#[cfg(test)]
mod tests {
    use super::*;
    use crawlkit_plugin_sdk::AnalysisContext;

    fn analyze_html(html: &str) -> Vec<Finding> {
        MetaDescriptionChecker.analyze(&AnalysisContext {
            url: "https://example.com".into(),
            html: html.into(),
            status_code: Some(200),
            headers: Vec::new(),
            response_time_ms: None,
        })
    }

    fn page_with(content: &str) -> String {
        format!("<html><head><meta name=\"description\" content=\"{content}\"></head></html>")
    }

    #[test]
    fn missing_description_is_an_error() {
        let findings = analyze_html("<html><head><title>x</title></head></html>");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "META001");
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn empty_content_counts_as_missing() {
        let findings = analyze_html(&page_with("   "));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "META001");
    }

    #[test]
    fn short_description_warns() {
        let findings = analyze_html(&page_with(&"a".repeat(MIN_LEN - 1)));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "META002");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn long_description_warns() {
        let findings = analyze_html(&page_with(&"a".repeat(MAX_LEN + 1)));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "META003");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn in_range_description_is_clean() {
        let findings = analyze_html(&page_with(&"a".repeat(100)));
        assert!(findings.is_empty());
    }

    #[test]
    fn boundaries_are_inclusive() {
        assert!(analyze_html(&page_with(&"a".repeat(MIN_LEN))).is_empty());
        assert!(analyze_html(&page_with(&"a".repeat(MAX_LEN))).is_empty());
    }

    #[test]
    fn attribute_matching_is_case_insensitive_and_quote_agnostic() {
        let html = "<html><head><META NAME=\"description\" CONTENT='x'></head></html>";
        let findings = analyze_html(html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "META002");
    }

    #[test]
    fn attribute_order_does_not_matter() {
        let html = "<html><head><meta content=\"x\" name=description></head></html>";
        let findings = analyze_html(html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "META002");
    }

    #[test]
    fn unrelated_meta_tags_are_ignored() {
        let html = "<html><head>\
                    <meta name=\"viewport\" content=\"width=device-width\">\
                    <meta property=\"og:description\" content=\"social\">\
                    </head></html>";
        let findings = analyze_html(html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "META001");
    }

    #[test]
    fn first_description_tag_wins() {
        let html = format!(
            "<html><head>\
             <meta name=\"description\" content=\"x\">\
             <meta name=\"description\" content=\"{}\">\
             </head></html>",
            "a".repeat(100)
        );
        let findings = analyze_html(&html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "META002");
    }

    #[test]
    fn unicode_content_is_counted_in_chars() {
        let content = "é".repeat(MIN_LEN);
        let findings = analyze_html(&page_with(&content));
        assert!(findings.is_empty());
    }
}
