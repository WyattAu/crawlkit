//! Registry uniqueness and coverage tests (Phase 2 roadmap item).
//!
//! Guards the single registration site (`AnalyzerRegistry::build_registry`)
//! against duplicate analyzer names and empty registrations.

use crate::analyzers::AnalyzerRegistry;
use crate::CrawlConfig;
use std::collections::HashSet;

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
