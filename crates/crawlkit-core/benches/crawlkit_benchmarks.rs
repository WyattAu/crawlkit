//! Benchmarks for crawlkit-core
//!
//! Measures performance of critical components

use crawlkit_core::analyzers::AnalyzerRegistry;
use crawlkit_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crawlkit_core::feature_flags::FeatureFlags;
use crawlkit_core::link_graph::LinkGraph;
use crawlkit_core::meta::MetaTags;
use crawlkit_core::parser::{Heading, HtmlParser, ParsedPage, ScriptInfo};
use crawlkit_core::CrawlConfig;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn make_bench_page() -> ParsedPage {
    ParsedPage {
        url: "https://example.com/benchmark".to_string(),
        meta: MetaTags {
            title: Some("Benchmark Page".to_string()),
            description: Some("A page for benchmarking".to_string()),
            canonical: Some(url::Url::parse("https://example.com/benchmark").unwrap()),
            ..Default::default()
        },
        headings: vec![
            Heading {
                level: 1,
                text: "Main".to_string(),
                length: 4,
            },
            Heading {
                level: 2,
                text: "Section".to_string(),
                length: 7,
            },
        ],
        links: Vec::new(),
        images: Vec::new(),
        forms: Vec::new(),
        scripts: vec![ScriptInfo {
            src: Some("app.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
        }],
        styles: Vec::new(),
        structured_data: Vec::new(),
        word_count: 500,
        landmarks: Vec::new(),
        has_skip_link: false,
        has_main_landmark: true,
        has_nav_landmark: true,
        has_positive_tabindex: false,
        tabindex_negative_count: 0,
        aria_role_count: 5,
        aria_label_count: 3,
        has_lang_attribute: true,
        html_lang: Some("en".to_string()),
        has_aria_hidden: false,
        tables_with_headers: 0,
        tables_total: 0,
        tables_with_captions: 0,
        og_image_width: None,
        og_image_height: None,
    }
}

fn bench_analyzer_registry(c: &mut Criterion) {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);
    let page = make_bench_page();

    c.bench_function("analyzer_registry_analyze", |b| {
        b.iter(|| {
            let ctx = crawlkit_core::AnalysisContext {
                page: black_box(&page),
                status_code: Some(200),
                headers: &[],
                response_time: Some(Duration::from_millis(100)),
                redirect_chain: &[],
                robots_txt: None,
            };
            registry.analyze(&ctx, &config)
        })
    });
}

fn bench_html_parser(c: &mut Criterion) {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <title>Test Page</title>
    <meta name="description" content="Test">
    <link rel="canonical" href="https://example.com">
</head>
<body>
    <h1>Main Title</h1>
    <h2>Section 1</h2>
    <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
    <a href="/about">About</a>
    <a href="https://external.com">External</a>
    <img src="/image.jpg" alt="Image" width="100" height="200">
    <script type="application/ld+json">{"@type": "WebPage"}</script>
</body>
</html>"#;

    let url = url::Url::parse("https://example.com/test").unwrap();

    c.bench_function("html_parser_parse", |b| {
        b.iter(|| HtmlParser::parse(black_box(html), &url))
    });
}

fn bench_circuit_breaker(c: &mut Criterion) {
    let config = CircuitBreakerConfig {
        failure_threshold: 5,
        success_threshold: 3,
        cooldown: Duration::from_secs(60),
    };

    c.bench_function("circuit_breaker_state_check", |b| {
        let cb = CircuitBreaker::new(config.clone());
        b.iter(|| {
            black_box(cb.is_allowed());
        })
    });
}

fn bench_link_graph_pagerank(c: &mut Criterion) {
    let mut graph = LinkGraph::new();

    // Create a graph with 100 nodes
    for i in 0_usize..100 {
        let source = format!("https://example.com/page{}", i);
        let target = format!("https://example.com/page{}", (i + 1) % 100);
        graph.add_link(&source, &target);
        if i % 10 == 0 {
            let backlink = format!("https://example.com/page{}", (i + 50) % 100);
            graph.add_link(&backlink, &source);
        }
    }

    c.bench_function("link_graph_pagerank_100_nodes", |b| {
        b.iter(|| {
            let mut g = graph.clone();
            g.compute_pagerank(black_box(0.85), 20);
        })
    });
}

fn bench_feature_flags(c: &mut Criterion) {
    let flags = FeatureFlags::default();

    c.bench_function("feature_flags_get", |b| {
        b.iter(|| {
            black_box(flags.get("ai_analyzers"));
        })
    });
}

fn bench_playwright_detector(c: &mut Criterion) {
    c.bench_function("playwright_detector_detect", |b| {
        b.iter(|| {
            black_box(crawlkit_core::PlaywrightDetector::detect());
        })
    });
}

criterion_group!(
    benches,
    bench_analyzer_registry,
    bench_html_parser,
    bench_circuit_breaker,
    bench_link_graph_pagerank,
    bench_feature_flags,
    bench_playwright_detector,
);

criterion_main!(benches);
