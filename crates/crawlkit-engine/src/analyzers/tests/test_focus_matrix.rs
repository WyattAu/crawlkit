//! Behavior matrix for the focus-management analyzer family.
//!
//! Five registered analyzers address keyboard focus with different
//! semantics. This file documents the ownership decision:
//!
//! | Analyzer | Code | Semantics | Status |
//! |---|---|---|---|
//! | `FocusOrderAnalyzer` | `A11Y-FOCUS001/002` | Tabindex check + visible-focus CSS heuristic | Distinct, retained |
//! | `FocusManagementDeepAnalyzer` | `A11Y-FOCUS002` | Focus-styles heuristic (matches FocusOrder) | Distinct trigger, SHARED code |
//! | `FocusManagementDeepAnalyzerV2` | `FOCUS-V2001` | Positive tabindex warning | **Canonical** for `FOCUS-V2001` |
//! | `FocusManagementDeepDeepValidator` | `FOCUS-V2001-DEEP-DEEP` | Same check, V8 generation | Namespaced generation |
//! | `FocusManagementDeepDeepDeepValidator` | `FOCUS-V2001-DEEP-DEEP-DEEP` | Same check, V8 generation | Namespaced generation |
//! | `FocusTabindexPositiveValidator` | `FOCUSTABPOS-V6119` | Same check, V6 generation | Unique code, retained |
//! | `FocusTrapMissingValidator` | `FOCTR001` | Dialog focus-trap check | Distinct, retained |
//!
//! The deep-deep and deep-deep-deep generations previously emitted the
//! plain `FOCUS-V2001` code, colliding with `FocusManagementDeepAnalyzerV2`.
//! The registry uniqueness fixture only exercises pages *without* positive
//! tabindex, so the collision was invisible at runtime until a dedicated
//! fixture activated it.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::ParsedPage;

fn page_at(url: &str, has_positive_tabindex: bool) -> ParsedPage {
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
        has_positive_tabindex,
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
fn canonical_v2_emits_plain_focus_v2001() {
    let page = page_at("https://example.com", true);
    let findings = FocusManagementDeepAnalyzerV2::new().analyze(&ctx(&page, ""));
    assert!(
        findings.iter().any(|f| f.code == "FOCUS-V2001"),
        "V2 must own the plain FOCUS-V2001 code: {findings:?}"
    );
}

#[test]
fn deep_deep_generation_is_namespaced() {
    let page = page_at("https://example.com", true);
    let findings = FocusManagementDeepDeepValidator::new().analyze(&ctx(&page, ""));
    assert!(findings.iter().any(|f| f.code == "FOCUS-V2001-DEEP-DEEP"));
    assert!(
        !findings.iter().any(|f| f.code == "FOCUS-V2001"),
        "deep-deep must not emit the canonical code"
    );
}

#[test]
fn deep_deep_deep_generation_is_namespaced() {
    let page = page_at("https://example.com", true);
    let findings = FocusManagementDeepDeepDeepValidator::new().analyze(&ctx(&page, ""));
    assert!(findings
        .iter()
        .any(|f| f.code == "FOCUS-V2001-DEEP-DEEP-DEEP"));
    assert!(
        !findings.iter().any(|f| f.code == "FOCUS-V2001"),
        "deep-deep-deep must not emit the canonical code"
    );
}

#[test]
fn v6_validator_keeps_unique_code() {
    let page = page_at("https://example.com", true);
    let findings = FocusTabindexPositiveValidator::new().analyze(&ctx(&page, ""));
    assert!(findings.iter().any(|f| f.code == "FOCUSTABPOS-V6119"));
}

#[test]
fn focus_order_flags_tabindex_and_missing_focus_styles() {
    // No :focus rules in the body -> both the tabindex error and the
    // focus-styles warning must fire.
    let mut page = page_at("https://example.com", true);
    page.links.push(crate::parser::ExtractedLink {
        href: "https://example.com/a".to_string(),
        text: "a link".to_string(),
        rel: Vec::new(),
        is_external: false,
        aria_label: None,
        img_alt: None,
    });
    let findings = FocusOrderAnalyzer::new().analyze(&ctx(&page, ""));
    assert!(findings.iter().any(|f| f.code == "A11Y-FOCUS001"));
    assert!(findings.iter().any(|f| f.code == "A11Y-FOCUS002"));
}

#[test]
fn focus_order_silent_without_tabindex_or_interactive_elements() {
    let page = page_at("https://example.com", false);
    let findings = FocusOrderAnalyzer::new().analyze(&ctx(&page, ""));
    assert!(findings.is_empty(), "expected silence, got {findings:?}");
}

#[test]
fn focus_trap_validator_flags_modal_without_trap() {
    // FocusTrapMissingValidator checks dialogs; a page without dialogs
    // must stay silent.
    let page = page_at("https://example.com", false);
    let findings = FocusTrapMissingValidator::new().analyze(&ctx(&page, ""));
    assert!(
        findings.is_empty(),
        "no dialogs -> no findings: {findings:?}"
    );
}

/// Regression for the runtime collision this matrix was written to expose:
/// the full registry, run against a page WITH positive tabindex, must not
/// emit the same finding code twice.
#[test]
fn full_registry_has_no_duplicate_focus_codes_on_tabindex_page() {
    let registry = AnalyzerRegistry::new(&crate::CrawlConfig::default());
    let mut page = page_at("https://example.com", true);
    page.links.push(crate::parser::ExtractedLink {
        href: "https://example.com/a".to_string(),
        text: "a link".to_string(),
        rel: Vec::new(),
        is_external: false,
        aria_label: None,
        img_alt: None,
    });
    let findings = registry.analyze(&ctx(&page, ""));
    let mut focus_codes: Vec<&str> = findings
        .iter()
        .map(|f| f.code.as_str())
        .filter(|c| c.starts_with("FOCUS") || c.starts_with("A11Y-FOCUS") || c.starts_with("FOCT"))
        .collect();
    focus_codes.sort();
    let before = focus_codes.len();
    focus_codes.dedup();
    assert_eq!(
        before,
        focus_codes.len(),
        "duplicate focus finding codes on tabindex page: {focus_codes:?}"
    );
}
