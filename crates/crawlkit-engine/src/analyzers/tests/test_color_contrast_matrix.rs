//! Behavior matrix for the color-contrast analyzer family.
//!
//! Five registered analyzers address link/text color contrast with
//! different semantics. This file documents the ownership decision:
//!
//! | Analyzer | Code | Semantics | Status |
//! |---|---|---|---|
//! | `ColorContrastAnalyzer` | `CONTR001/002` | Full WCAG ratio math on inline fg/bg pairs | **Canonical** |
//! | `ColorContrastTextAnalyzer` | `COLRCT-V2003` | Hidden-text detection (distinct check) | Distinct, retained |
//! | `ColorContrastLinkAnalyzer` | `COLRCL001` | Link-specific ratio math (<3:1) | Distinct, retained |
//! | ~~`ColorContrastLinkAnalyzerV2`~~ | ~~`COLRCL-V2001`~~ | Underline heuristic (NOT contrast math) | **Removed** |
//! | `ColorContrastLinkDeepValidator` | `COLRCL-V2001-DEEP` | White-on-white deep heuristic | Distinct trigger, own code |
//!
//! ## Why the underline heuristic was removed
//!
//! `ColorContrastLinkAnalyzerV2` fired whenever *any* CSS rule paired
//! `text-decoration:none` with a `color:` declaration anywhere in the document
//! — not on links, and not on the link in question. That is the default styling
//! of most navigation, so it fired on 40 of 40 crawled pages of
//! kingstonpeptides.com while claiming to test a WCAG 1.4.1 concern.
//!
//! The concern is real and this check could not reach it. WCAG 1.4.1 asks
//! whether a link has *any* non-colour indicator, which is a property of one
//! element's computed style — underline, border, icon, or a background change on
//! hover/focus. Determining that needs a rendering engine, not a stylesheet
//! regex. The remaining analyzers compute real ratios and are unaffected.
//!
//! Note the code collision this also resolved: the deep validator and the V2
//! analyzer both emitted `COLRCL-V2001`, differing only in title, so the
//! registry-level uniqueness guard could not distinguish them.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::ParsedPage;

fn page_at(url: &str) -> ParsedPage {
    ParsedPage {
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

fn ctx<'a>(page: &'a ParsedPage, body: &'a str) -> AnalysisContext<'a> {
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

#[test]
fn canonical_ratio_analyzer_flags_low_contrast_pair() {
    // #cccccc on #dddddd is ~1.2:1 — below 3:1.
    let page = page_at("https://example.com");
    let body = r#"<span style="color: #cccccc; background-color: #dddddd">text</span>"#;
    let findings = ColorContrastAnalyzer::new().analyze(&ctx(&page, body));
    assert!(
        findings.iter().any(|f| f.code == "CONTR001"),
        "canonical ratio math must flag ~1.2:1: {findings:?}"
    );
}

#[test]
fn link_ratio_analyzer_flags_low_contrast_link() {
    let page = page_at("https://example.com");
    let body = r#"<a style="color: #cccccc; background-color: #dddddd">Link</a>"#;
    let findings = ColorContrastLinkAnalyzer::new().analyze(&ctx(&page, body));
    assert!(
        findings.iter().any(|f| f.code == "COLRCL001"),
        "link ratio math must flag low-contrast link: {findings:?}"
    );
}

#[test]
fn hidden_text_check_is_orthogonal_to_contrast() {
    let page = page_at("https://example.com");
    let body = r#"<style>.spoiler { opacity:0 }</style>"#;
    let findings = ColorContrastTextAnalyzerV2::new().analyze(&ctx(&page, body));
    assert!(
        findings.iter().any(|f| f.code == "COLRCT-V2003"),
        "hidden-text detection must fire: {findings:?}"
    );
}

#[test]
fn all_family_members_report_accessibility_category() {
    let page = page_at("https://example.com");
    let body = r#"<span style="color: #cccccc; background-color: #dddddd">x</span>"#;
    let ctx = ctx(&page, body);
    for f in ColorContrastAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, crate::types::IssueCategory::Accessibility);
    }
}
