//! Behavior matrix for the generic-anchor-text analyzer family.
//!
//! Registered analyzers addressing generic anchor text:
//!
//! | Analyzer | Code | Trigger list | Severity / Category | Status |
//! |---|---|---|---|---|
//! | `AnchorTextGenericAnalyzer` | `ANCHGEN001` | Base heuristic | Base | **Canonical** (base) |
//! | `AnchorTextGenericAnalyzerV2` | `ANCHGEN-V2001` | click here, here, read more, more info, learn more, link | Warning / SEO | Distinct, retained |
//! | `AnchorTextGenericDeepValidator` | `ANCHGEN-V2001-DEEP` | click here, read more, learn more, more, here, link, this page | Info / Accessibility | Namespaced generation |
//!
//! The two generation analyzers were previously colliding on the plain
//! `ANCHGEN-V2001` code. Their trigger lists overlap but neither contains
//! the other ("more info" only in V2; "more" and "this page" only in the
//! deep validator), and they differ in severity and category — so both
//! remain registered and the deep generation is namespaced. Consumers can
//! now distinguish the SEO warning from the accessibility info signal.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::{ExtractedLink, ParsedPage};

fn page_with_links(texts: &[&str]) -> ParsedPage {
    ParsedPage {
        url: "https://example.com".to_string(),
        meta: MetaTags::default(),
        headings: Vec::new(),
        links: texts
            .iter()
            .map(|t| ExtractedLink {
                href: "https://example.com/target".to_string(),
                text: (*t).to_string(),
                rel: Vec::new(),
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect(),
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

fn ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
    AnalysisContext {
        page,
        body: None,
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
fn v2_owns_plain_v2001() {
    let page = page_with_links(&["click here"]);
    let findings = AnchorTextGenericAnalyzerV2::new().analyze(&ctx(&page));
    assert!(
        findings.iter().any(|f| f.code == "ANCHGEN-V2001"),
        "V2 must own plain ANCHGEN-V2001: {findings:?}"
    );
}

#[test]
fn deep_generation_is_namespaced() {
    let page = page_with_links(&["click here"]);
    let findings = AnchorTextGenericDeepValidator::new().analyze(&ctx(&page));
    assert!(findings.iter().any(|f| f.code == "ANCHGEN-V2001-DEEP"));
    assert!(
        !findings.iter().any(|f| f.code == "ANCHGEN-V2001"),
        "deep generation must not emit the canonical code"
    );
}

#[test]
fn v2_detects_more_info_which_deep_does_not() {
    // "more info" is only in the V2 trigger list.
    let page = page_with_links(&["More Info"]);
    let v2 = AnchorTextGenericAnalyzerV2::new().analyze(&ctx(&page));
    assert!(v2.iter().any(|f| f.code == "ANCHGEN-V2001"));
    let deep = AnchorTextGenericDeepValidator::new().analyze(&ctx(&page));
    assert!(
        deep.is_empty(),
        "deep must stay silent for 'more info': {deep:?}"
    );
}

#[test]
fn deep_detects_this_page_which_v2_does_not() {
    // "this page" is only in the deep trigger list.
    let page = page_with_links(&["This Page"]);
    let deep = AnchorTextGenericDeepValidator::new().analyze(&ctx(&page));
    assert!(deep.iter().any(|f| f.code == "ANCHGEN-V2001-DEEP"));
    let v2 = AnchorTextGenericAnalyzerV2::new().analyze(&ctx(&page));
    assert!(v2.is_empty(), "V2 must stay silent for 'this page': {v2:?}");
}

#[test]
fn both_stay_silent_on_descriptive_anchor_text() {
    let page = page_with_links(&["Kingston peptides research overview"]);
    let v2 = AnchorTextGenericAnalyzerV2::new().analyze(&ctx(&page));
    let deep = AnchorTextGenericDeepValidator::new().analyze(&ctx(&page));
    assert!(v2.is_empty() && deep.is_empty());
}

/// Full-registry regression: the aggregate generation analyzers must
/// emit at most one finding each (a second finding would indicate a
/// collision or double registration). The base analyzer legitimately
/// emits one finding per generic link, so its multiplicity is checked
/// against the link count instead.
#[test]
fn full_registry_has_no_duplicate_anchorgen_codes() {
    let registry = AnalyzerRegistry::new(&crate::CrawlConfig::default());
    let page = page_with_links(&["click here", "learn more"]);
    let findings = registry.analyze(&ctx(&page));
    let count = |prefix: &str| findings.iter().filter(|f| f.code == prefix).count();
    // Aggregate analyzers: exactly one finding each on this page.
    assert_eq!(count("ANCHGEN-V2001"), 1, "V2 must aggregate, not per-link");
    assert_eq!(
        count("ANCHGEN-V2001-DEEP"),
        1,
        "deep must aggregate, not per-link"
    );
    // Per-link base analyzer: one finding per generic link.
    assert_eq!(
        count("ANCHGEN001"),
        2,
        "base analyzer emits one finding per generic link"
    );
}
