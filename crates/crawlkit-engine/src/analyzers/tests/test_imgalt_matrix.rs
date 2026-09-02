//! Behavior matrix for the image-alt analyzer family.
//!
//! Registered analyzers addressing image alternative text:
//!
//! | Analyzer | Codes | Semantics | Status |
//! |---|---|---|---|
//! | `ImageAltTextAnalyzer` (basic) | `IMGALT001` | Basic missing-alt | Canonical (base) |
//! | `ImageAltTextDeepAnalyzer` (a11y) | `IMGALTDEEP001/002` | Deep accessibility view | Distinct, retained |
//! | `ImageAltMissingDeepValidator` | `IMGALTMISS-V6116` | V6 missing-alt | Unique code, retained |
//! | `ImageAltEmptyDeepValidator` | `IMGALTEMPTY-V6117` | V6 empty-alt | Unique code, retained |
//! | `ImageAltTextDeepAnalyzerV2` | `IMGALT-V2001`, `IMGALT-V2003` | Missing + generic alt | Distinct, retained |
//! | `ImageAltTextDeepDeepValidator` | `IMGALT-V2001-DEEP-DEEP`, `IMGALT-V2002-DEEP-DEEP` | Missing + empty alt | Namespaced generation |
//! | `ImageAltTextDeepDeepDeepValidator` | `IMGALT-V2001-DEEP-DEEP-DEEP` | Subset of deep-deep | **Unregistered** (kept exported) |
//!
//! The deep-deep-deep validator performed only the missing-alt check of
//! the deep-deep validator, so its default registration was removed. The
//! deep-deep generation previously collided with V2 on the plain
//! `IMGALT-V2001` code; both have distinct trigger semantics (V2 treats
//! empty alt as missing, deep-deep separates them), so both remain with
//! namespaced codes.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::{ExtractedImage, ParsedPage};

fn page_with_images(images: Vec<ExtractedImage>) -> ParsedPage {
    ParsedPage {
        url: "https://example.com".to_string(),
        meta: MetaTags::default(),
        headings: Vec::new(),
        links: Vec::new(),
        images,
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

fn img(has_alt: bool, alt: &str) -> ExtractedImage {
    ExtractedImage {
        src: "https://example.com/i.png".to_string(),
        alt: alt.to_string(),
        has_alt,
        width: None,
        height: None,
        is_lazy_loaded: false,
        aria_hidden: false,
    }
}

#[test]
fn v2_owns_plain_v2001_for_missing_or_empty_alt() {
    let page = page_with_images(vec![img(false, "")]);
    let findings = ImageAltTextDeepAnalyzerV2::new().analyze(&ctx(&page));
    assert!(
        findings.iter().any(|f| f.code == "IMGALT-V2001"),
        "V2 must own plain IMGALT-V2001: {findings:?}"
    );
}

#[test]
fn v2_flags_generic_alt_with_v2003() {
    let page = page_with_images(vec![img(true, "image")]);
    let findings = ImageAltTextDeepAnalyzerV2::new().analyze(&ctx(&page));
    assert!(findings.iter().any(|f| f.code == "IMGALT-V2003"));
    assert!(!findings.iter().any(|f| f.code == "IMGALT-V2001"));
}

#[test]
fn deep_deep_codes_are_namespaced() {
    // Missing alt -> V2001-DEEP-DEEP; empty alt -> V2002-DEEP-DEEP.
    let page = page_with_images(vec![img(false, ""), img(true, "")]);
    let findings = ImageAltTextDeepDeepValidator::new().analyze(&ctx(&page));
    assert!(findings.iter().any(|f| f.code == "IMGALT-V2001-DEEP-DEEP"));
    assert!(findings.iter().any(|f| f.code == "IMGALT-V2002-DEEP-DEEP"));
    assert!(
        !findings
            .iter()
            .any(|f| f.code.starts_with("IMGALT-V2") && !f.code.contains("DEEP")),
        "deep-deep must emit only namespaced codes: {findings:?}"
    );
}

#[test]
fn deep_deep_silent_when_all_images_have_descriptive_alt() {
    let page = page_with_images(vec![img(true, "A chart of quarterly revenue")]);
    let findings = ImageAltTextDeepDeepValidator::new().analyze(&ctx(&page));
    assert!(findings.is_empty(), "expected silence, got {findings:?}");
}

#[test]
fn deep_deep_deep_generation_is_fully_namespaced() {
    let page = page_with_images(vec![img(false, "")]);
    let findings = ImageAltTextDeepDeepDeepValidator::new().analyze(&ctx(&page));
    assert!(findings
        .iter()
        .any(|f| f.code == "IMGALT-V2001-DEEP-DEEP-DEEP"));
    assert!(!findings.iter().any(|f| f.code == "IMGALT-V2001"));
}

/// The deep-deep-deep registration was removed from the default registry
/// because its only check (missing alt) is contained within deep-deep:
/// whenever deep-deep-deep fires, deep-deep also fires. The converse does
/// not hold (deep-deep additionally detects empty alt), which is what
/// makes deep-deep-deep strictly redundant.
#[test]
fn deep_deep_deep_firing_implies_deep_deep_fires() {
    let scenarios: Vec<Vec<ExtractedImage>> = vec![
        vec![],
        vec![img(false, "")],                    // missing alt
        vec![img(true, "")],                     // empty alt
        vec![img(true, "Descriptive alt text")], // healthy
    ];
    for images in scenarios {
        let page = page_with_images(images.clone());
        let subset = ImageAltTextDeepDeepDeepValidator::new().analyze(&ctx(&page));
        let superset = ImageAltTextDeepDeepValidator::new().analyze(&ctx(&page));
        assert!(
            subset.is_empty() || !superset.is_empty(),
            "deep-deep-deep fired but deep-deep stayed silent for {images:?}"
        );
    }
}

/// Full-registry regression: on a page with a missing-alt image the
/// registry must not emit the same IMGALT code twice.
#[test]
fn full_registry_has_no_duplicate_imgalt_codes_on_missing_alt_page() {
    let registry = AnalyzerRegistry::new(&crate::CrawlConfig::default());
    let page = page_with_images(vec![img(false, "")]);
    let findings = registry.analyze(&ctx(&page));
    let mut codes: Vec<&str> = findings
        .iter()
        .map(|f| f.code.as_str())
        .filter(|c| {
            c.starts_with("IMGALT") || c.starts_with("IMGALTMISS") || c.starts_with("IMGALTEMPTY")
        })
        .collect();
    codes.sort();
    let before = codes.len();
    codes.dedup();
    assert_eq!(
        before,
        codes.len(),
        "duplicate image-alt finding codes: {codes:?}"
    );
}
