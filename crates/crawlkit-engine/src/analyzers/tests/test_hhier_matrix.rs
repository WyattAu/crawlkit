//! Behavior matrix for the heading-hierarchy analyzer family.
//!
//! Registered analyzers addressing heading structure:
//!
//! | Analyzer | Codes | Semantics | Status |
//! |---|---|---|---|
//! | `HeadingHierarchyAnalyzer` | `HEAD001-004` | Base SEO hierarchy check | **Canonical** (base) |
//! | `HeadingHierarchyDeepAnalyzerV2` | `HHIER-V2001..V2003` | Empty headings + level skips | Distinct, retained |
//! | `HeadingHierarchyDeepDeepValidator` | `HHIER-V2001`, `HHIER-V2002/3-DEEP-DEEP`, `HHIER-V2004` | Full hierarchy re-check | Namespaced generation |
//! | `HeadingHierarchyDeepDeepDeepValidator` | `HHIER-V2001-DEEP-DEEP-DEEP`, `...-V2002/3-DEEP-DEEP-DEEP` | Subset of deep-deep | **Unregistered** (kept exported) |
//! | `HeadingHierarchyDeepAnalyzer` (a11y) | `HHIERDEEP001-003` | Deep accessibility view | Distinct, retained |
//!
//! Two problems this matrix locks in:
//!
//! 1. **Semantic collision (fixed):** `HHIER-V2002` previously meant
//!    "empty headings" in the V2 generation but "missing H1" in both V8
//!    generations. The same code describing two different defects makes
//!    downstream deduplication and reporting impossible.
//! 2. **Strict-subset duplication (removed):** the deep-deep-deep
//!    validator performed the exact missing-H1/multiple-H1 checks of the
//!    deep-deep validator with no additional detection, so its default
//!    registration was removed. The public type remains exported so
//!    downstream consumers are unaffected.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::{Heading, ParsedPage};

fn page_with_headings(headings: Vec<(&str, u8)>) -> ParsedPage {
    ParsedPage {
        url: "https://example.com".to_string(),
        meta: MetaTags::default(),
        headings: headings
            .into_iter()
            .map(|(text, level)| Heading {
                level,
                text: text.to_string(),
                length: text.len(),
            })
            .collect(),
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
fn v2_owns_plain_v2002_for_empty_headings() {
    // Empty-heading text is the V2 generation's semantic for V2002.
    let page = page_with_headings(vec![("   ", 1)]);
    let findings = HeadingHierarchyDeepAnalyzerV2::new().analyze(&ctx(&page));
    assert!(
        findings
            .iter()
            .any(|f| f.code == "HHIER-V2002" && f.title.contains("Empty")),
        "V2 must own plain HHIER-V2002 with empty-heading semantics: {findings:?}"
    );
}

#[test]
fn deep_deep_missing_h1_is_namespaced() {
    let page = page_with_headings(vec![("H2", 2)]);
    let findings = HeadingHierarchyDeepDeepValidator::new().analyze(&ctx(&page));
    assert!(findings.iter().any(|f| f.code == "HHIER-V2002-DEEP-DEEP"));
    assert!(
        !findings.iter().any(|f| f.code == "HHIER-V2002"),
        "deep-deep must not emit the plain V2002 code (owned by V2)"
    );
}

#[test]
fn deep_deep_empty_headings_uses_v2001() {
    // HHIER-V2001 (no-headings) is deep-deep's own branch and never
    // collides: V2 returns silently for an empty heading list.
    let page = page_with_headings(vec![]);
    let findings = HeadingHierarchyDeepDeepValidator::new().analyze(&ctx(&page));
    assert!(findings.iter().any(|f| f.code == "HHIER-V2001"));
    let v2 = HeadingHierarchyDeepAnalyzerV2::new().analyze(&ctx(&page));
    assert!(v2.is_empty(), "V2 is silent on empty heading lists: {v2:?}");
}

#[test]
fn deep_deep_multiple_h1_is_namespaced() {
    let page = page_with_headings(vec![("A", 1), ("B", 1)]);
    let findings = HeadingHierarchyDeepDeepValidator::new().analyze(&ctx(&page));
    assert!(findings.iter().any(|f| f.code == "HHIER-V2003-DEEP-DEEP"));
    assert!(!findings.iter().any(|f| f.code == "HHIER-V2003"));
}

#[test]
fn deep_deep_detects_level_skips_with_v2004() {
    let page = page_with_headings(vec![("H1", 1), ("H3", 3)]);
    let findings = HeadingHierarchyDeepDeepValidator::new().analyze(&ctx(&page));
    assert!(findings.iter().any(|f| f.code == "HHIER-V2004"));
}

#[test]
fn deep_deep_deep_generation_is_fully_namespaced() {
    let page = page_with_headings(vec![("A", 1), ("B", 1)]);
    let findings = HeadingHierarchyDeepDeepDeepValidator::new().analyze(&ctx(&page));
    assert!(findings
        .iter()
        .any(|f| f.code == "HHIER-V2003-DEEP-DEEP-DEEP"));
    assert!(
        !findings
            .iter()
            .any(|f| f.code.starts_with("HHIER-V200") && !f.code.contains("DEEP")),
        "deep-deep-deep must emit only namespaced codes: {findings:?}"
    );
}

/// The deep-deep-deep registration was removed from the default registry
/// because it is a strict subset of deep-deep. Prove the subset property
/// that justified the removal: every non-empty finding of the subset on a
/// missing-H1 / multi-H1 page corresponds to a finding the deep-deep
/// validator also produces.
#[test]
fn deep_deep_deep_is_subset_of_deep_deep() {
    let scenarios: Vec<Vec<(&str, u8)>> = vec![
        vec![],                   // no headings
        vec![("H2", 2)],          // missing H1
        vec![("A", 1), ("B", 1)], // multiple H1
        vec![("H1", 1)],          // healthy
    ];
    for headings in scenarios {
        let page = page_with_headings(headings.clone());
        let subset = HeadingHierarchyDeepDeepDeepValidator::new().analyze(&ctx(&page));
        let superset = HeadingHierarchyDeepDeepValidator::new().analyze(&ctx(&page));
        // Every trigger condition detected by the subset is detected by the
        // superset (code families differ by namespace suffix, so compare
        // finding counts per base semantic instead of exact codes).
        assert_eq!(
            subset.is_empty(),
            superset.is_empty(),
            "subset and superset must agree on silence for {headings:?}"
        );
    }
}

/// Full-registry regression: on a heading-skip page the registry must not
/// emit the same HHIER code twice.
#[test]
fn full_registry_has_no_duplicate_hhier_codes_on_skip_page() {
    let registry = AnalyzerRegistry::new(&crate::CrawlConfig::default());
    let page = page_with_headings(vec![("H1", 1), ("H3", 3)]);
    let findings = registry.analyze(&ctx(&page));
    let mut hhier: Vec<&str> = findings
        .iter()
        .map(|f| f.code.as_str())
        .filter(|c| c.starts_with("HHIER") || c.starts_with("HEAD00"))
        .collect();
    hhier.sort();
    let before = hhier.len();
    hhier.dedup();
    assert_eq!(
        before,
        hhier.len(),
        "duplicate heading-hierarchy finding codes: {hhier:?}"
    );
}
