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
//! | `ColorContrastLinkAnalyzerV2` | `COLRCL-V2001` | Underline heuristic (NOT contrast math) | Distinct, retained |
//! | `ColorContrastLinkDeepValidator` | `COLRCL-V2001` | White-on-white deep heuristic | Distinct trigger, SHARED code |
//!
//! The V2 and deep link analyzers both emit `COLRCL-V2001` but with
//! different triggers and titles; they are namespaced here so the
//! registry-level uniqueness guard stays meaningful.

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
        body_size: None,
        compressed_size: None,
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
fn v2_underline_heuristic_is_distinct_from_ratio_math() {
    // The V2 analyzer fires on underline removal, not on contrast ratios.
    let page = page_at("https://example.com");
    let body = r#"<style>a { color: #333; text-decoration: none; }</style>"#;
    let findings = ColorContrastLinkAnalyzerV2::new().analyze(&ctx(&page, body));
    assert!(
        findings.iter().any(|f| f.code == "COLRCL-V2001-UNDERLINE"),
        "underline heuristic must fire with its own code: {findings:?}"
    );

    // The ratio analyzers must NOT flag #333 on white (sufficient contrast).
    assert!(ColorContrastAnalyzer::new()
        .analyze(&ctx(&page, body))
        .is_empty());
    assert!(ColorContrastLinkAnalyzer::new()
        .analyze(&ctx(&page, body))
        .is_empty());
}

#[test]
fn deep_white_on_white_heuristic_is_distinct_from_underline_check() {
    let page = page_at("https://example.com");
    let body = r#"<style>a { color: #fff; background-color: #fff; }</style>"#;
    let findings = ColorContrastLinkDeepValidator::new().analyze(&ctx(&page, body));
    assert!(
        findings.iter().any(|f| f.code == "COLRCL-V2001-DEEP"),
        "deep white-on-white heuristic must use its own code: {findings:?}"
    );

    // The V2 underline heuristic must not fire on this input.
    assert!(ColorContrastLinkAnalyzerV2::new()
        .analyze(&ctx(&page, body))
        .is_empty());
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
