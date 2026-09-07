//! crawlkit plugin: heading-structure.
//!
//! Flags broken heading hierarchies: multiple `<h1>` elements (warning),
//! skipped levels such as an `<h3>` directly under an `<h1>` (warning),
//! and pages with no headings at all (info). Built for
//! `wasm32-unknown-unknown` and published in the first-party plugin index
//! (`plugins/index/`).

use crawlkit_plugin_sdk::{AnalysisContext, Analyzer, Finding, Severity};

pub struct HeadingStructure;

impl HeadingStructure {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HeadingStructure {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HeadingStructure {
    fn name(&self) -> &str {
        "heading-structure"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let levels = heading_levels(&ctx.html.to_lowercase());
        let mut findings = Vec::new();

        let h1_count = levels.iter().filter(|level| **level == 1).count();
        if h1_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: "seo".into(),
                code: "HEAD001".into(),
                title: "Multiple H1 headings".into(),
                description: format!(
                    "Page has {h1_count} <h1> elements; \
                     search engines expect a single top-level heading."
                ),
                url: ctx.url.clone(),
                recommendation: "Keep one <h1> and demote the rest to \
                                 <h2>-<h6> subsections."
                    .into(),
            });
        }

        if let Some((from, to)) = first_skipped_level(&levels) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: "seo".into(),
                code: "HEAD002".into(),
                title: "Heading level skipped".into(),
                description: format!(
                    "An <h{to}> follows an <h{from}> without an intermediate \
                     level; assistive technologies and search engines use \
                     consecutive heading levels to build the page outline."
                ),
                url: ctx.url.clone(),
                recommendation: "Insert the missing intermediate heading or \
                                 renumber the outline so levels are consecutive."
                    .into(),
            });
        }

        if levels.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: "seo".into(),
                code: "HEAD003".into(),
                title: "No headings found".into(),
                description: "Page contains no <h1>-<h6> headings; \
                              the document has no machine-readable outline."
                    .into(),
                url: ctx.url.clone(),
                recommendation: "Structure the content with headings, \
                                 starting with a single <h1>."
                    .into(),
            });
        }

        findings
    }
}

/// Heading levels (1-6) in document order, from a lowercased document.
fn heading_levels(lower_html: &str) -> Vec<u8> {
    let bytes = lower_html.as_bytes();
    let mut levels = Vec::new();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i] == b'<'
            && bytes[i + 1] == b'h'
            && (b'1'..=b'6').contains(&bytes[i + 2])
            && (bytes[i + 3] == b'>'
                || bytes[i + 3] == b'/'
                || bytes[i + 3].is_ascii_whitespace())
        {
            levels.push(bytes[i + 2] - b'0');
        }
        i += 1;
    }
    levels
}

/// First place the outline jumps down by more than one level
/// (e.g. H1 straight to H3), as `(outer, inner)` levels.
fn first_skipped_level(levels: &[u8]) -> Option<(u8, u8)> {
    levels.windows(2).find_map(|window| {
        let (outer, inner) = (window[0], window[1]);
        (inner > outer + 1).then_some((outer, inner))
    })
}

crawlkit_plugin_sdk::export_analyzer!(HeadingStructure);

#[cfg(test)]
mod tests {
    use super::*;
    use crawlkit_plugin_sdk::AnalysisContext;

    fn analyze_html(html: &str) -> Vec<Finding> {
        HeadingStructure.analyze(&AnalysisContext {
            url: "https://example.com".into(),
            html: html.into(),
            status_code: Some(200),
            headers: Vec::new(),
            response_time_ms: None,
        })
    }

    #[test]
    fn consecutive_levels_are_clean() {
        let html = "<html><body><h1>A</h1><h2>B</h2><h3>C</h3></body></html>";
        assert!(analyze_html(html).is_empty());
    }

    #[test]
    fn level_can_jump_back_up() {
        let html = "<html><body><h1>A</h1><h3>B</h3><h2>C</h2></body></html>";
        let findings = analyze_html(html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "HEAD002");
    }

    #[test]
    fn skipped_level_warns() {
        let html = "<html><body><h1>A</h1><h3>B</h3></body></html>";
        let findings = analyze_html(html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "HEAD002");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn multiple_h1_warns() {
        let html = "<html><body><h1>A</h1><h2>B</h2><h1>C</h1></body></html>";
        let findings = analyze_html(html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "HEAD001");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn no_headings_is_info() {
        let html = "<html><body><p>Plain text</p></body></html>";
        let findings = analyze_html(html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "HEAD003");
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn all_findings_fire_together() {
        let html = "<html><body><h1>A</h1><h1>B</h1><h3>C</h3></body></html>";
        let findings = analyze_html(html);
        let mut codes: Vec<_> = findings.iter().map(|f| f.code.as_str()).collect();
        codes.sort_unstable();
        assert_eq!(codes, vec!["HEAD001", "HEAD002"]);
    }

    #[test]
    fn heading_attributes_and_uppercase_tags_are_recognized() {
        let html = "<html><body><H1 class=\"title\">A</H1><h2\n id=\"x\">B</h2></body></html>";
        assert!(analyze_html(html).is_empty());
    }

    #[test]
    fn non_heading_h_tags_are_not_headings() {
        let html = "<html><body><header><h1>A</h1></header><head>x</head></body></html>";
        assert!(analyze_html(html).is_empty());
    }

    #[test]
    fn h7_and_h0_are_not_headings() {
        let html = "<html><body><h0>A</h0><h7>B</h7></body></html>";
        let findings = analyze_html(html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "HEAD003");
    }

    #[test]
    fn closing_tags_alone_do_not_count() {
        let html = "<html><body></h1></h2></body></html>";
        let findings = analyze_html(html);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "HEAD003");
    }
}
