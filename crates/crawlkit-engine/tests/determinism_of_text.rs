//! Regression tests for run-to-run reproducibility of finding text.
//!
//! Several analyzers summarize a frequency table by taking the top N entries.
//! When the table comes from a `HashMap` and the sort compares only the count,
//! entries with equal counts are emitted in `HashMap` iteration order — which
//! Rust randomizes per process. The finding *codes* stayed stable, so a
//! code-only determinism check passed while the report text changed on every
//! run. Two audits of an unchanged page could not be diffed.
//!
//! Each test runs the analyzer many times within one process. Rust seeds
//! `RandomState` per `HashMap` instance, so repeated calls exercise the
//! nondeterminism even inside a single test.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use crawlkit_engine::analyzers::{AnalyzerRegistry, AnalysisContext, Analyzer};
use crawlkit_engine::parser::{Heading, HtmlParser};
use url::Url;

/// The HTML is chosen so that every candidate term occurs exactly once, forcing
/// a full tie and exposing any dependence on hash ordering.
const TIED_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<title>Tied Scores</title>
<meta name="description" content="A page constructed so every heading term ties.">
</head>
<body>
<h1>Tied Scores</h1>
<h2>alpha bravo charlie</h2>
<h2>delta echo foxtrot</h2>
<h2>golf hotel india</h2>
<h2>juliet kilo lima</h2>
<h2>mike november</h2>
<p>alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november.</p>
<p>alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november.</p>
</body>
</html>"#;

/// Run `f` against a freshly built context for the tied fixture.
///
/// The context borrows its backing data, so it is constructed inside the closure
/// and torn down with it.
fn with_ctx<R>(url: &str, f: impl FnOnce(&AnalysisContext<'_>) -> R) -> R {
    let parsed = HtmlParser::parse(TIED_HTML, &Url::parse(url).expect("valid url"));
    let headers: Vec<(String, String)> = Vec::new();
    let redirects: Vec<crawlkit_engine::RedirectHop> = Vec::new();
    let ctx = AnalysisContext {
        page: &parsed,
        body: Some(TIED_HTML),
        status_code: Some(200),
        headers: &headers,
        response_time: Some(Duration::from_millis(10)),
        redirect_chain: &redirects,
        robots_txt: None,
        user_agent: None,
        body_size: Some(TIED_HTML.len()),
        compressed_size: None,
        content_encoding: None,
        server: None,
        content_type: Some("text/html; charset=utf-8"),
        rendered: None,
    };
    f(&ctx)
}

/// Collect the text of a code across repeated analyses of the same page.
fn repeated_texts(code: &str) -> Vec<String> {
    let analyzer = crawlkit_engine::analyzers::content_analyzers::ContentQualityAnalyzer::new();
    (0..64)
        .map(|_| {
            with_ctx("https://example.com/tied", |ctx| {
                analyzer
                    .analyze(ctx)
                    .into_iter()
                    .find(|f| f.code == code)
                    .map(|f| f.description)
                    .unwrap_or_default()
            })
        })
        .collect()
}

#[test]
fn heading_keyword_summary_is_stable_across_runs() {
    let texts = repeated_texts("CQ002");
    assert!(
        !texts[0].is_empty(),
        "expected a CQ002 finding for the fixture, got {:?}",
        texts[0]
    );
    let first = &texts[0];
    let divergent = texts.iter().filter(|t| *t != first).count();
    assert_eq!(
        divergent, 0,
        "CQ002 text changed across {} of 64 runs:\n  {first}\n  {}",
        divergent,
        texts.iter().find(|t| *t != first).unwrap()
    );
}

#[test]
fn detected_topics_are_stable_across_runs() {
    let analyzer = crawlkit_engine::analyzers::content_analyzers::EntityAnalyzer::new();
    let texts: Vec<String> = (0..64)
        .map(|_| {
            with_ctx("https://example.com/tied", |ctx| {
                analyzer
                    .analyze(ctx)
                    .into_iter()
                    .find(|f| f.code == "ENTITY004")
                    .map(|f| f.description)
                    .unwrap_or_default()
            })
        })
        .collect();
    let first = &texts[0];
    assert!(
        !first.is_empty(),
        "expected an ENTITY004 finding for the fixture"
    );
    let divergent = texts.iter().filter(|t| *t != first).count();
    assert_eq!(
        divergent, 0,
        "ENTITY004 topics changed across {} of 64 runs:\n  {first}\n  {}",
        divergent,
        texts.iter().find(|t| *t != first).unwrap()
    );
}

#[test]
fn keyword_analyzer_summaries_are_stable_across_runs() {
    let analyzer = crawlkit_engine::analyzers::seo_analyzers::KeywordAnalyzer::new();
    for code in ["KW001", "KW002", "KW003", "KW004"] {
        let texts: Vec<String> = (0..64)
            .map(|_| {
                with_ctx("https://example.com/tied", |ctx| {
                    analyzer
                        .analyze(ctx)
                        .into_iter()
                        .find(|f| f.code == code)
                        .map(|f| f.description)
                        .unwrap_or_default()
                })
            })
            .collect();
        let first = &texts[0];
        if first.is_empty() {
            continue; // analyzer had nothing to say for this input
        }
        let divergent = texts.iter().filter(|t| *t != first).count();
        assert_eq!(
            divergent, 0,
            "{code} text changed across {} of 64 runs:\n  {first}\n  {}",
            divergent,
            texts.iter().find(|t| *t != first).unwrap()
        );
    }
}

/// Whole-registry check: the full finding list, and every finding's text, must
/// be byte-identical across repeated analyses of the same page.
#[test]
fn full_registry_output_is_reproducible() {
    let registry = AnalyzerRegistry::new(&Default::default());
    let snapshot = || {
        with_ctx("https://example.com/tied", |ctx| {
            let mut rows: Vec<String> = registry
                .analyze(ctx)
                .into_iter()
                .map(|f| format!("{}|{:?}|{}|{}", f.code, f.severity, f.title, f.description))
                .collect();
            rows.sort();
            rows
        })
    };

    let first = snapshot();
    for round in 1..8 {
        let again = snapshot();
        assert_eq!(
            first.len(),
            again.len(),
            "finding count changed on round {round}: {} vs {}",
            first.len(),
            again.len()
        );
        let differences: Vec<&str> = first
            .iter()
            .zip(again.iter())
            .filter(|(a, b)| a != b)
            .map(|(a, b)| Box::leak(format!("{a}\n    vs {b}").into_boxed_str()) as &str)
            .collect();
        assert!(
            differences.is_empty(),
            "round {round} differed in {} finding(s):\n    {}",
            differences.len(),
            differences.join("\n    ")
        );
    }
}

/// `log-analyze` output must be byte-identical between runs.
///
/// The two breakdown maps were `HashMap`, and serde serializes map keys in
/// iteration order, so the same access log produced differently ordered JSON on
/// every process. Serde's own advice is to use `BTreeMap` when output order
/// matters.
#[test]
fn log_analysis_serializes_deterministically() {
    use crawlkit_engine::log_analyzer::LogAnalysis;

    let build = || {
        let mut crawler = std::collections::BTreeMap::new();
        let mut status = std::collections::BTreeMap::new();
        for (bot, n) in [
            ("Googlebot", 5usize),
            ("bingbot", 3),
            ("Human", 9),
            ("Crawlkit", 1),
            ("YandexBot", 2),
        ] {
            *crawler.entry(bot.to_string()).or_default() += n;
        }
        for (code, n) in [(200u16, 12usize), (404, 3), (500, 5)] {
            *status.entry(code).or_default() += n;
        }
        LogAnalysis {
            total_requests: 20,
            crawler_breakdown: crawler,
            status_breakdown: status,
            top_urls: vec![("/a".into(), 12), ("/b".into(), 8)],
            error_urls: vec![("/c".into(), 500)],
        }
    };

    let first = serde_json::to_string(&build()).expect("serializes");
    for round in 1..16 {
        assert_eq!(
            first,
            serde_json::to_string(&build()).expect("serializes"),
            "log analysis JSON differed on round {round}"
        );
    }
    // Keys must come out ordered, not merely coincidentally equal.
    let human = first.find("Human").expect("Human key present");
    let google = first.find("Googlebot").expect("Googlebot key present");
    assert!(
        google < human,
        "breakdown keys should be sorted, got: {first}"
    );
}

/// Guard the fixture itself: the test is only meaningful while every heading
/// term occurs exactly once.
#[test]
fn fixture_headings_are_fully_tied() {
    let parsed = HtmlParser::parse(TIED_HTML, &Url::parse("https://example.com/").unwrap());
    let counts: std::collections::HashMap<&str, usize> = parsed
        .headings
        .iter()
        .flat_map(|h: &Heading| h.text.split_whitespace())
        .map(|w| w.to_lowercase())
        .fold(std::collections::HashMap::new(), |mut acc, w| {
            *acc.entry(Box::leak(w.into_boxed_str()) as &str).or_default() += 1;
            acc
        });
    assert!(
        counts.values().all(|c| *c == 1),
        "fixture must have all-tied heading terms, got {counts:?}"
    );
}