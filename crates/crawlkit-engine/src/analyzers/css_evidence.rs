//! Whether a page's CSS was available to analyze, and what follows from that.
//!
//! # The defect this exists to prevent
//!
//! crawlkit does **not** fetch external stylesheets. Every site that ships its
//! styling as an external bundle — which is essentially every modern site —
//! therefore hands the analyzers a document with no CSS in it.
//!
//! Two rules responded by asserting *absence*: if the HTML contains no `:focus`,
//! report "No visible focus indicators found". Verified against
//! `wyattsnotes.wyattau.com`, a Starlight site whose stylesheet
//! `/_astro/Layout.QTI7V5vg.css` (47 KB) contains five focus rules including
//! `:focus-visible { outline: 2px solid … }`. The finding fired on 60 of 60 pages
//! with the description *"no `:focus` or `:focus-visible` CSS rules were
//! detected"* — stated as fact about the page, when the truth was "I never looked
//! at the stylesheet".
//!
//! That is the failure mode worth naming: **"not detected" is not "not present".**
//! A rule that cannot see the evidence must not claim the evidence is absent.
//!
//! # Why the fix is here rather than in each analyzer
//!
//! Only two rules assert absence today (`A11Y-FOCUS002`, `FOCUS002`). The other
//! CSS-derived rules — `FSIZE001`, `FSIZE002`, `COLRCT-V2001`, `COLRCL-V2001-DEEP`,
//! `COLRCT-V2003` — only ever fire on values they actually observed, so they lose
//! *coverage* rather than inventing defects. Coverage loss is recoverable; a
//! confident false claim is not.
//!
//! Centralising the predicate means the day crawlkit starts fetching stylesheets,
//! [`is_complete`] starts returning true and the absence assertions become
//! legitimate again without touching the analyzers.
//!
//! [`AnalysisContext`]: crate::AnalysisContext

use crate::AnalysisContext;

/// Codes reporting that a CSS-derived rule could not run.
pub const CSS_COVERAGE_PARTIAL: &str = "CSS-COVERAGE-PARTIAL";

/// External stylesheets referenced by the page.
///
/// Inline `<style>` blocks and `style` attributes are already inside
/// `ctx.body`, so they are excluded: they are analyzed.
#[must_use]
pub fn external_stylesheet_count(ctx: &AnalysisContext) -> usize {
    ctx.page
        .styles
        .iter()
        .filter(|s| s.href.is_some() && !s.is_inline)
        .count()
}

/// True when everything the page references for styling is present in
/// [`AnalysisContext::body`].
///
/// When this is false, a CSS-derived rule may still report what it *found*, but
/// must not report something as missing.
#[must_use]
pub fn is_complete(ctx: &AnalysisContext) -> bool {
    external_stylesheet_count(ctx) == 0
}

/// True when a rule that asserts the *absence* of something CSS-shaped may fire.
///
/// Absence is only provable from a complete document.
#[must_use]
pub fn may_assert_absence(ctx: &AnalysisContext) -> bool {
    is_complete(ctx)
}

/// A coverage measurement for pages whose CSS was not analyzed.
///
/// This is deliberately a **measurement**, not a defect. "crawlkit did not fetch
/// your stylesheet" is a fact about the crawl, not a fault in the page, and
/// putting it on the issue channel would replace 60 false findings with 60 true
/// but useless ones. [`CSS_COVERAGE_PARTIAL`] is registered in
/// [`crate::analyzers::metric_codes`] so it lands in `page-metrics.json` rather
/// than in issue totals.
#[must_use]
pub fn coverage_finding(ctx: &AnalysisContext) -> Option<crate::Finding> {
    let count = external_stylesheet_count(ctx);
    if count == 0 {
        return None;
    }
    Some(crate::Finding {
        severity: crate::Severity::Info,
        category: crate::IssueCategory::Performance,
        code: CSS_COVERAGE_PARTIAL.to_string(),
        title: "External stylesheets were not analyzed".to_string(),
        description: format!(
            "{count} external stylesheet(s) are referenced but were not fetched, so rules that \
             depend on CSS (focus indicators, font sizes, line height, colour contrast) could \
             only inspect inline styles and cannot report anything as missing."
        ),
        url: ctx.page.url.to_string(),
        recommendation: "Fetch and analyze external stylesheets to enable the full CSS rule \
                         set."
            .to_string(),
    })
}

/// Emits [`CSS_COVERAGE_PARTIAL`] once per page whose CSS was not analyzed.
///
/// A dedicated analyzer rather than a side effect of the rules that consult CSS.
/// Two of those rules are gated on [`is_complete`], and both wanted to report the
/// coverage gap -- which produced two identical rows per page, because
/// `collapse_duplicates` never merges two findings sharing a code. Verified on
/// wyattsnotes.wyattau.com: 120 findings for 60 pages.
///
/// The rule this replaces asserted absence from the document; this one reports
/// the gap once and stays silent otherwise, so the page's issue count drops by
/// exactly the number of false positives that were removed.
#[derive(Debug, Clone, Default)]
pub struct CssCoverageAnalyzer;

impl crate::analyzers::Analyzer for CssCoverageAnalyzer {
    fn name(&self) -> &str {
        "css-coverage"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<crate::Finding> {
        coverage_finding(ctx).into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ParsedPage, StyleInfo};

    fn ctx_with(styles: Vec<StyleInfo>) -> AnalysisContext<'static> {
        let page = Box::leak(Box::new(ParsedPage {
            url: "https://example.com/".to_string(),
            meta: MetaTags::default(),
            headings: vec![],
            links: vec![],
            images: vec![],
            forms: vec![],
            scripts: vec![],
            styles,
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
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }));
        AnalysisContext {
            page,
            body: None,
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

    fn external(href: &str) -> StyleInfo {
        StyleInfo {
            href: Some(href.to_string()),
            media: None,
            is_inline: false,
            has_integrity: false,
        }
    }

    fn inline() -> StyleInfo {
        StyleInfo {
            href: None,
            media: None,
            is_inline: true,
            has_integrity: false,
        }
    }

    /// The regression: a page whose only stylesheet is external has no CSS in the
    /// body, and absence is not provable from it.
    #[test]
    fn external_stylesheet_makes_css_incomplete() {
        let ctx = ctx_with(vec![external("/_astro/Layout.css")]);
        assert_eq!(external_stylesheet_count(&ctx), 1);
        assert!(!is_complete(&ctx));
        assert!(!may_assert_absence(&ctx));
    }

    /// No stylesheets at all means the body holds everything there is.
    #[test]
    fn no_stylesheets_means_css_is_complete() {
        let ctx = ctx_with(vec![]);
        assert!(is_complete(&ctx));
        assert!(may_assert_absence(&ctx));
        assert!(coverage_finding(&ctx).is_none());
    }

    /// Inline CSS is already in the body, so it does not make the document
    /// incomplete. Counting it would suppress absence assertions on pages that
    /// genuinely are fully analyzed.
    #[test]
    fn inline_styles_do_not_make_css_incomplete() {
        let ctx = ctx_with(vec![inline(), inline()]);
        assert_eq!(external_stylesheet_count(&ctx), 0);
        assert!(is_complete(&ctx));
    }

    /// A preload of a font is not a stylesheet.
    #[test]
    fn preload_links_are_not_stylesheets() {
        let ctx = ctx_with(vec![external("/fonts/x.woff2")]);
        // `styles` only holds real stylesheet links, so whatever is here counts.
        assert_eq!(external_stylesheet_count(&ctx), 1);
    }

    /// The coverage note is a measurement, and it must not appear on a page whose
    /// CSS was fully analyzed.
    #[test]
    fn coverage_finding_only_appears_when_css_is_missing() {
        assert!(coverage_finding(&ctx_with(vec![external("/a.css")])).is_some());
        assert!(coverage_finding(&ctx_with(vec![])).is_none());
        let f = coverage_finding(&ctx_with(vec![external("/a.css"), external("/b.css")]))
            .expect("two stylesheets");
        assert!(f.description.contains("2 external stylesheet"));
        assert_eq!(f.severity, crate::Severity::Info);
    }
}
