//! Structured data that is present but broken.
//!
//! # The defect this exists to fix
//!
//! The parser extracts `<script type="application/ld+json">` blocks and parses
//! each with `serde_json::from_str`. On a parse error it *skips the block*:
//!
//! ```ignore
//! let value: serde_json::Value = match serde_json::from_str(raw) {
//!     Ok(v) => v,
//!     Err(_) => return None,   // the block stops existing here
//! };
//! ```
//!
//! A page with broken schema therefore looks identical to a page with none, and
//! every analyzer reading `structured_data` sees nothing to complain about. A
//! trailing comma — the single commonest JSON-LD mistake, because authors copy
//! JSON that was valid inside a larger object — silently removes a page's rich
//! result eligibility with no diagnostic anywhere.
//!
//! Found by the testbed: a fixture with exactly that trailing comma produced no
//! finding at all.
//!
//! # Why the analyzer re-reads the document
//!
//! Recording parse failures on `ParsedPage` would be the tidier shape, but
//! `ParsedPage` has 335 construction sites and adding a field to it is not worth
//! the churn for one check. The blocks that matter are precisely the ones the
//! parser throws away, so this analyzer re-reads the served document through
//! [`html_text::ldjson_script_bodies`] and parses them itself.
//!
//! # What is and is not reported
//!
//! Only blocks that fail to parse are reported, and the parser's own error
//! message is included: "expected `,` or `}` at line 5 column 3" is actionable
//! and "structured data is broken" is not. A block that parses is not validated
//! against schema.org's vocabulary here — the existing schema validators own
//! that, and they only see blocks that parsed, which is the same gap from the
//! other side.

use crate::analyzers::html_text::ldjson_script_bodies;
use crate::analyzers::Analyzer;
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};

/// One or more JSON-LD blocks failed to parse.
pub const SCHEMA_PARSE: &str = "SCHEMA-PARSE-ERROR";

pub struct JsonLdValidityAnalyzer;

impl Default for JsonLdValidityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonLdValidityAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for JsonLdValidityAnalyzer {
    fn name(&self) -> &str {
        "jsonld-validity"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        // `ctx.body` is the rendered DOM when the page was rendered, which is
        // the document a visitor's browser actually executed. Schema that only
        // exists after hydration is still schema the crawler-dependent
        // ecosystem sees, so this is the right document to judge. The served
        // copy is a different question and belongs to the parity analyzer.
        let Some(body) = ctx.body else {
            return Vec::new();
        };
        let bodies = ldjson_script_bodies(body);
        if bodies.is_empty() {
            return Vec::new();
        }

        let failures: Vec<String> = bodies
            .iter()
            .enumerate()
            .filter_map(|(i, raw)| {
                serde_json::from_str::<serde_json::Value>(raw)
                    .err()
                    .map(|e| {
                        format!(
                            "block {}: {} (starts {:?})",
                            i + 1,
                            e,
                            &raw[..raw.len().min(60)]
                        )
                    })
            })
            .collect();

        if failures.is_empty() {
            return Vec::new();
        }

        let url = ctx.page.url.to_string();
        vec![Finding {
            severity: Severity::Error,
            category: IssueCategory::Schema,
            code: SCHEMA_PARSE.to_string(),
            title: format!(
                "{} JSON-LD block(s) on this page are not valid JSON",
                failures.len()
            ),
            description: format!(
                "{} of {} JSON-LD block(s) could not be parsed, so search engines \
                 receive no structured data from them. A page with no parseable \
                 structured data is not eligible for rich results. {}",
                failures.len(),
                bodies.len(),
                failures.join("; ")
            ),
            url,
            recommendation: "Fix the JSON syntax. The usual causes are a trailing \
                             comma before a closing brace, single quotes instead of \
                             double quotes, and unescaped newlines inside strings. \
                             Validate with a JSON linter before publishing."
                .to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    fn page(url: &str) -> &'static ParsedPage {
        Box::leak(Box::new(ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: vec![],
            links: vec![],
            images: vec![],
            forms: vec![],
            scripts: vec![],
            styles: vec![],
            structured_data: vec![],
            word_count: 0,
            sentence_count: 0,
            landmarks: vec![],
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: true,
            html_lang: Some("en".to_string()),
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }))
    }

    fn ctx_for(body: &str) -> AnalysisContext<'_> {
        AnalysisContext {
            page: page("https://shop.example/"),
            body: Some(body),
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            user_agent: None,
            body_size: None,
            compressed_size: None,
            content_encoding: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    fn codes(body: &str) -> Vec<String> {
        JsonLdValidityAnalyzer::new()
            .analyze(&ctx_for(body))
            .into_iter()
            .map(|f| f.code)
            .collect()
    }

    /// The regression: a trailing comma is the commonest JSON-LD mistake and it
    /// produced no finding at all, because the parser dropped the block before
    /// any analyzer could see it.
    #[test]
    fn trailing_comma_is_reported() {
        let body = r#"<script type="application/ld+json">
            { "a": 1, }
        </script>"#;
        let found = codes(body);
        assert!(
            found.contains(&SCHEMA_PARSE.to_string()),
            "a trailing comma must be reported, got {found:?}"
        );
    }

    #[test]
    fn valid_jsonld_is_not_reported() {
        let body = r#"<script type="application/ld+json">{"@type":"Product"}</script>"#;
        assert_eq!(codes(body), Vec::<String>::new());
    }

    #[test]
    fn one_bad_block_does_not_hide_a_good_one() {
        let body = r#"<script type="application/ld+json">{"ok":true}</script>
            <script type="application/ld+json">{bad}</script>"#;
        let findings = JsonLdValidityAnalyzer::new().analyze(&ctx_for(body));
        assert_eq!(findings.len(), 1);
        // "1 of 2" so the operator knows a good block survived.
        assert!(
            findings[0].description.contains("1 of 2"),
            "{}",
            findings[0].description
        );
    }

    #[test]
    fn the_parser_error_message_is_included() {
        let body = r#"<script type="application/ld+json">{bad}</script>"#;
        let findings = JsonLdValidityAnalyzer::new().analyze(&ctx_for(body));
        assert!(
            findings[0].description.contains("block 1:"),
            "the serde error must be quoted, got {}",
            findings[0].description
        );
    }

    /// Minified unquoted `type=application/ld+json` is how Astro and friends
    /// emit it. Missing it would make the check silent on build-optimised sites.
    #[test]
    fn minified_unquoted_type_is_detected() {
        let body = r#"<script type=application/ld+json>{bad}</script>"#;
        assert!(codes(body).contains(&SCHEMA_PARSE.to_string()));
    }

    #[test]
    fn no_jsonld_means_no_finding() {
        assert_eq!(codes("<p>nothing here</p>"), Vec::<String>::new());
    }
}
