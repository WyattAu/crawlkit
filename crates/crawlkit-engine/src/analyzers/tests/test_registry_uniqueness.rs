//! Registry uniqueness and coverage tests (Phase 2 roadmap item).
//!
//! Guards the single registration site (`AnalyzerRegistry::build_registry`)
//! against duplicate analyzer names and empty registrations.

use crate::analyzers::{AnalysisContext, AnalyzerRegistry};
use crate::parser::ParsedPage;
use crate::CrawlConfig;
use std::collections::{HashMap, HashSet};

fn fixture_page() -> ParsedPage {
    ParsedPage {
        url: "https://example.com".to_string(),
        meta: Default::default(),
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

fn fixture_context(page: &ParsedPage) -> AnalysisContext<'_> {
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

/// Core is intentionally smaller than the complete registry and remains duplicate-free.
#[test]
fn test_core_profile_is_small_and_unique() {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::core(&config);
    assert!(registry.len() < AnalyzerRegistry::new(&config).len());
    let names: HashSet<&str> = registry.iter().map(|analyzer| analyzer.name()).collect();
    assert_eq!(names.len(), registry.len());
}

/// Standard is a deliberately reduced, canonical profile; full coverage remains in `new`.
#[test]
fn test_standard_profile_is_reduced_and_unique() {
    let config = CrawlConfig::default();
    let standard = AnalyzerRegistry::standard(&config);
    let default = AnalyzerRegistry::new(&config);
    let names: HashSet<&str> = standard.iter().map(|analyzer| analyzer.name()).collect();
    assert_eq!(standard.len(), 17);
    assert!(standard.len() < default.len());
    assert_eq!(names.len(), standard.len());
}

/// Deep is a focused profile of advanced analyzers with unique names and codes.
#[test]
fn test_deep_profile_is_focused_and_unique() {
    let config = CrawlConfig::default();
    let deep = AnalyzerRegistry::deep(&config);
    let standard = AnalyzerRegistry::standard(&config);
    let names: HashSet<&str> = deep.iter().map(|analyzer| analyzer.name()).collect();
    assert_eq!(deep.len(), 20);
    assert_eq!(names.len(), deep.len());
    assert!(deep.len() > standard.len());
}

/// Every registered analyzer must have a unique name. A duplicate name means
/// two registrations race for the same finding identity and downstream
/// consumers cannot attribute findings.
#[test]
fn test_registry_analyzer_names_unique() {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);
    let names: Vec<&str> = registry.iter().map(|a| a.name()).collect();
    let unique: HashSet<&str> = names.iter().copied().collect();
    assert_eq!(
        names.len(),
        unique.len(),
        "duplicate analyzer names in registry: {:?}",
        names
            .iter()
            .filter(|n| names.iter().filter(|m| m == n).count() > 1)
            .collect::<Vec<_>>()
    );
}

/// Each analyzer must return unique finding codes on a representative page.
/// This catches registry-level duplicate output while preserving the ability
/// for one analyzer to emit multiple different issue codes.
#[test]
fn test_registry_finding_codes_unique_on_fixture() {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);
    let page = fixture_page();
    let ctx = fixture_context(&page);
    let mut owners: HashMap<String, String> = HashMap::new();
    for analyzer in registry.iter() {
        for finding in analyzer.analyze(&ctx) {
            if let Some(previous) = owners.get(&finding.code) {
                panic!(
                    "duplicate finding code {} emitted by {} and {}",
                    finding.code,
                    previous,
                    analyzer.name()
                );
            }
            owners.insert(finding.code.clone(), analyzer.name().to_string());
        }
    }
}

/// The default registry must be non-empty and at least the documented
/// minimum (the `new` doc comment says 39 built-ins; the registry has grown
/// far beyond that, but the documented floor must hold).
#[test]
fn test_registry_non_empty_documented_floor() {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);
    assert!(
        registry.len() >= 39,
        "registry below documented floor: {}",
        registry.len()
    );
}

/// Profile baselines make registry consolidation measurable and reviewable.
#[test]
fn test_profile_baseline_counts() {
    let config = CrawlConfig::default();
    let page = fixture_page();
    let ctx = fixture_context(&page);
    let core = AnalyzerRegistry::core(&config);
    let standard = AnalyzerRegistry::standard(&config);
    let deep = AnalyzerRegistry::deep(&config);
    let default = AnalyzerRegistry::new(&config);

    let count_findings = |registry: &AnalyzerRegistry| {
        registry
            .iter()
            .map(|analyzer| analyzer.analyze(&ctx).len())
            .sum::<usize>()
    };

    assert_eq!(core.len(), 9);
    assert_eq!(standard.len(), 17);
    assert_eq!(deep.len(), 20);
    assert!(standard.len() < deep.len());
    assert!(deep.len() < default.len());
    assert_eq!(count_findings(&core), 9);
    assert!(count_findings(&standard) <= count_findings(&default));
}

/// Every registered analyzer must be constructible with a non-empty name.
#[test]
fn test_registry_analyzers_have_names() {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);
    for a in registry.iter() {
        assert!(
            !a.name().trim().is_empty(),
            "registered analyzer with empty name"
        );
    }
}
