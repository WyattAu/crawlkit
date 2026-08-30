//! Analyzer registry contract harness.
//!
//! Defines invariants that the `AnalyzerRegistry` must satisfy:
//! non-empty by default, deterministic ordering, and feature-flag
//! toggling changes the analyzer count predictably.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use crawlkit_engine::analyzers::AnalyzerRegistry;
use crawlkit_engine::parser::HtmlParser;
use crawlkit_engine::{AnalysisContext, CrawlConfig, FeatureFlags, FLAG_AI_ANALYZERS};
use url::Url;

fn make_ctx() -> AnalysisContext<'static> {
    let url = Url::parse("https://example.com/").unwrap();
    let html = "<!DOCTYPE html><html lang=\"en\"><head><title>Test</title>\
                <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
                </head><body><main><h1>Hello</h1></main></body></html>";
    let page = Box::leak(Box::new(HtmlParser::parse(html, &url)));
    let headers: &'static [(String, String)] = Box::leak(Box::new([]));
    AnalysisContext {
        page,
        body: Some(html),
        status_code: Some(200),
        headers,
        response_time: Some(Duration::from_millis(100)),
        redirect_chain: &[],
        robots_txt: None,
        body_size: Some(html.len()),
        compressed_size: Some(html.len()),
        server: None,
        content_type: Some("text/html; charset=utf-8"),
        rendered: None,
    }
}

#[test]
fn default_registry_is_non_empty() {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);
    assert!(
        !registry.is_empty(),
        "default registry must contain analyzers"
    );
    assert!(
        registry.len() > 20,
        "default registry should have a broad analyzer set"
    );
}

#[test]
fn analyze_produces_canonically_ordered_findings() {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);
    let ctx = make_ctx();
    let findings = registry.analyze(&ctx);

    // Findings must be sorted by (code, url).
    let mut sorted = findings.clone();
    sorted.sort_by(|a, b| a.code.cmp(&b.code).then_with(|| a.url.cmp(&b.url)));
    assert_eq!(findings.len(), sorted.len(), "clone must preserve length");
    for (i, (a, b)) in findings.iter().zip(sorted.iter()).enumerate() {
        assert_eq!(
            a.code, b.code,
            "finding {i} code must match sorted position"
        );
        assert_eq!(a.url, b.url, "finding {i} url must match sorted position");
    }
}

#[test]
fn analyze_is_deterministic_across_calls() {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);
    let ctx = make_ctx();
    let first = registry.analyze(&ctx);
    let second = registry.analyze(&ctx);

    // Compare by (code, url) since Finding does not derive PartialEq.
    assert_eq!(first.len(), second.len(), "finding count must be stable");
    for (i, (a, b)) in first.iter().zip(second.iter()).enumerate() {
        assert_eq!(a.code, b.code, "finding {i} code must match across calls");
        assert_eq!(a.url, b.url, "finding {i} url must match across calls");
    }
}

#[cfg(feature = "full")]
#[test]
fn feature_flags_toggle_ai_analyzer_group() {
    let mut flags_with = FeatureFlags::default();
    flags_with.set(FLAG_AI_ANALYZERS, true);
    let with_ai = AnalyzerRegistry::with_feature_flags(&flags_with);

    let mut flags_without = FeatureFlags::default();
    flags_without.set(FLAG_AI_ANALYZERS, false);
    let without_ai = AnalyzerRegistry::with_feature_flags(&flags_without);

    assert!(
        with_ai.len() > without_ai.len(),
        "disabling AI analyzers must reduce the registry count"
    );
}
