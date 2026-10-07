//! Links that hand a new browsing context to an untrusted page.
//!
//! # The defect, and how much of one is left
//!
//! `<a href="…" target="_blank">` without `rel="noopener"` hands the new window
//! a `window.opener` handle, so the page that was opened can navigate the
//! originator — the classic reverse-tabnabbing attack, where a linked page
//! swaps a trusted tab for a phishing copy.
//!
//! Reporting it as though modern browsers were exposed would overstate the risk,
//! so this analyzer does not do that. Chrome 88, Firefox 79 and Safari 12.1 all
//! imply `noopener` for `target="_blank"`, so the attack is largely closed on
//! current browsers. What remains is real but narrower:
//!
//! - embedded webviews and older mobile browsers, which have no such default
//! - explicit `window.open` chains that re-introduce an opener
//! - defence in depth at zero cost: the attribute is three characters
//!
//! That is a Warning with a description that says what is and is not at risk,
//! not an Error claiming every such link is exploitable.
//!
//! # Why this analyzer re-reads the document
//!
//! `ExtractedLink` carries `rel` and `is_external` but not `target`, and adding
//! a field to it means touching 260 construction sites for one check. The
//! anchors that matter are still in the served document, so
//! [`html_text::start_tags`] reads them directly.
//!
//! # One finding per page
//!
//! A page with forty such links gets one finding listing them, not forty
//! findings. The fix is identical for every link and the page is the unit the
//! operator edits, so forty rows is noise forty times over.

use crate::analyzers::html_text::{attr_value, start_tags};
use crate::analyzers::Analyzer;
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};

/// `target="_blank"` without `rel="noopener"`.
pub const LINK_TABNAB: &str = "LINK-TABNAB-OPENER";

pub struct TabnabbingAnalyzer;

impl Default for TabnabbingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TabnabbingAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Whether `rel` already neutralises the opener.
///
/// `noreferrer` implies `noopener` per the HTML spec, so either is sufficient.
fn has_opener_protection(rel: Option<&str>) -> bool {
    let Some(rel) = rel else { return false };
    rel.split(|c: char| c.is_whitespace() || c == ',')
        .any(|t| t.eq_ignore_ascii_case("noopener") || t.eq_ignore_ascii_case("noreferrer"))
}

impl Analyzer for TabnabbingAnalyzer {
    fn name(&self) -> &str {
        "tabnabbing"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let Some(body) = ctx.body else {
            return Vec::new();
        };
        let page_url = match url::Url::parse(&ctx.page.url) {
            Ok(u) => u,
            Err(_) => return Vec::new(),
        };

        let mut affected: Vec<String> = Vec::new();
        for tag in start_tags(body, "a") {
            // Only links that open a new browsing context are candidates. This
            // was inverted on the first pass, which skipped every link the
            // analyzer exists to check and reported none.
            if !attr_value(&tag, "target")
                .map(|t| t.trim().eq_ignore_ascii_case("_blank"))
                .unwrap_or(false)
            {
                continue;
            }
            if has_opener_protection(attr_value(&tag, "rel").as_deref()) {
                continue;
            }
            let Some(href) = attr_value(&tag, "href") else {
                continue;
            };
            let Ok(resolved) = page_url.join(href.trim()) else {
                continue;
            };
            // Same-origin links that open a new tab hand the opener to a page
            // the operator already controls, which is not a cross-origin risk.
            if resolved.host_str() == page_url.host_str() && resolved.scheme() == page_url.scheme()
            {
                continue;
            }
            let display = if href.len() > 60 {
                format!("{}…", &href[..60])
            } else {
                href.clone()
            };
            affected.push(display);
        }

        if affected.is_empty() {
            return Vec::new();
        }

        let url = ctx.page.url.to_string();
        vec![Finding {
            severity: Severity::Warning,
            category: IssueCategory::Security,
            code: LINK_TABNAB.to_string(),
            title: format!(
                "{} external link(s) open a new tab without rel=noopener",
                affected.len()
            ),
            description: format!(
                "{} link(s) use target=\"_blank\" with no rel=\"noopener\" or \
                 rel=\"noreferrer\", so the opened page receives a window.opener \
                 handle and can navigate this page. Modern browsers -- Chrome 88+, \
                 Firefox 79+, Safari 12.1+ -- imply noopener for target=\"_blank\", \
                 so the exposure is mainly older browsers, embedded webviews and \
                 defence in depth. Affected: {}.",
                affected.len(),
                affected.join(", ")
            ),
            url,
            recommendation: "Add rel=\"noopener\" (or rel=\"noreferrer\", which also \
                             withholds the referrer) to these links. It costs nothing \
                             and covers the browsers that do not imply it."
                .to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    fn ctx(body: &str) -> AnalysisContext<'_> {
        let page: &'static ParsedPage = Box::leak(Box::new(ParsedPage {
            url: "https://shop.example/".to_string(),
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
        }));
        AnalysisContext {
            page,
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
        TabnabbingAnalyzer::new()
            .analyze(&ctx(body))
            .into_iter()
            .map(|f| f.code)
            .collect()
    }

    #[test]
    fn external_blank_without_rel_is_reported() {
        let body = r#"<a href="https://other.example" target="_blank">partner</a>"#;
        assert!(codes(body).contains(&LINK_TABNAB.to_string()));
    }

    #[test]
    fn noopener_and_noreferrer_both_satisfy() {
        let a = r#"<a href="https://other.example" target="_blank" rel="noopener">x</a>"#;
        let b = r#"<a href="https://other.example" target="_blank" rel="noreferrer">x</a>"#;
        let c = r#"<a href="https://other.example" target="_blank" rel="nofollow noopener">x</a>"#;
        for body in [a, b, c] {
            assert!(
                !codes(body).contains(&LINK_TABNAB.to_string()),
                "protected link reported: {body}"
            );
        }
    }

    #[test]
    fn same_origin_blank_is_not_reported() {
        let body = r#"<a href="/elsewhere" target="_blank">internal</a>"#;
        assert_eq!(codes(body), Vec::<String>::new());
    }

    /// No `target` means no new browsing context, so there is no opener to hand
    /// over however the rel attribute is set.
    #[test]
    fn links_without_target_are_not_reported() {
        let body = r#"<a href="https://other.example">plain external</a>"#;
        assert_eq!(codes(body), Vec::<String>::new());
    }

    /// Minified markup drops attribute quotes. Missing that shape would make the
    /// check silent on build-optimised sites, which is where most links live.
    #[test]
    fn minified_markup_is_handled() {
        let body = r#"<a href=https://other.example target=_blank>minified</a>"#;
        assert!(codes(body).contains(&LINK_TABNAB.to_string()));
    }

    /// One finding for the page, listing the links -- forty rows for forty links
    /// that share one fix is noise forty times over.
    #[test]
    fn one_finding_lists_all_affected_links() {
        let body = r#"<a href="https://a.example" target="_blank">a</a>
            <a href="https://b.example" target="_blank">b</a>
            <a href="https://c.example" target="_blank">c</a>"#;
        let findings = TabnabbingAnalyzer::new().analyze(&ctx(body));
        assert_eq!(findings.len(), 1, "one finding per page, not per link");
        assert!(findings[0].title.contains('3'), "{}", findings[0].title);
        for host in ["a.example", "b.example", "c.example"] {
            assert!(
                findings[0].description.contains(host),
                "{host} missing from the list"
            );
        }
    }

    /// The description must not overstate the risk. Modern browsers imply
    /// noopener, and a Warning that claims every such link is exploitable
    /// trains people to ignore it.
    #[test]
    fn description_states_the_modern_browser_default() {
        let body = r#"<a href="https://other.example" target="_blank">x</a>"#;
        let findings = TabnabbingAnalyzer::new().analyze(&ctx(body));
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(
            findings[0].description.contains("imply noopener"),
            "the description must say modern browsers imply it"
        );
    }
}
