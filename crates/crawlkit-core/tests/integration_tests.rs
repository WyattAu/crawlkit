//! Integration tests for crawlkit-core
//!
//! Tests the full pipeline: fetch -> parse -> analyze -> store -> export

use std::time::Duration;

use chrono::Utc;
use url::Url;

use crawlkit_core::ai_analyzers::AiCrawlerAccessibilityAnalyzer;
use crawlkit_core::analyzers::{AnalysisContext, Analyzer, AnalyzerRegistry};
use crawlkit_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use crawlkit_core::feature_flags::{FeatureFlags, FLAG_AI_ANALYZERS};
use crawlkit_core::link_graph::LinkGraph;
use crawlkit_core::meta::MetaTags;
use crawlkit_core::parser::{Heading, ParsedPage, ScriptInfo};
use crawlkit_core::playwright::{PlaywrightConfig, PlaywrightRenderer};
use crawlkit_core::ratelimit::RateLimiter;
use crawlkit_core::storage::{Issue, IssueCategory, PageData, Severity, Storage};
use crawlkit_core::wasm_analyzers::WasmPatternAnalyzer;
use crawlkit_core::CrawlConfig;

// ---------------------------------------------------------------------------
// Test Helpers
// ---------------------------------------------------------------------------

fn make_test_page(url: &str) -> ParsedPage {
    ParsedPage {
        url: url.to_string(),
        meta: MetaTags {
            title: Some("Test Page".to_string()),
            description: Some("A test page for integration testing".to_string()),
            canonical: Some(Url::parse("https://example.com/test").unwrap()),
            ..Default::default()
        },
        headings: vec![
            Heading {
                level: 1,
                text: "Main Title".to_string(),
                length: 10,
            },
            Heading {
                level: 2,
                text: "Section 1".to_string(),
                length: 9,
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

fn make_page_data(url: &str) -> PageData {
    PageData {
        id: uuid::Uuid::new_v4().to_string(),
        url: Url::parse(url).unwrap(),
        final_url: Url::parse(url).unwrap(),
        status_code: 200,
        title: Some("Test Page".to_string()),
        description: Some("Test description".to_string()),
        canonical_url: Some(Url::parse(url).unwrap()),
        word_count: Some(500),
        load_time_ms: Some(150),
        body_size: Some(1024),
        fetched_at: Utc::now(),
        links: Vec::new(),
    }
}

fn make_test_config() -> CrawlConfig {
    CrawlConfig::default()
}

// ---------------------------------------------------------------------------
// Integration Tests
// ---------------------------------------------------------------------------

#[test]
fn test_analyzer_registry_full_pipeline() {
    let config = make_test_config();
    let registry = AnalyzerRegistry::new(&config);
    let page = make_test_page("https://example.com/test");

    let ctx = AnalysisContext {
        page: &page,
        status_code: Some(200),
        headers: &[],
        response_time: Some(Duration::from_millis(150)),
        redirect_chain: &[],
    };

    let findings = registry.analyze(&ctx, &config);

    // Should produce findings from multiple analyzers
    assert!(!findings.is_empty());

    // Should have findings from different categories
    let categories: Vec<_> = findings.iter().map(|f| &f.category).collect();
    assert!(categories.len() > 1);
}

#[test]
fn test_circuit_breaker_integration() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        cooldown: Duration::from_millis(100),
    };
    let cb = CircuitBreaker::new(config);

    // Initially closed
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.is_allowed());

    // Record failures
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);

    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.is_allowed());
}

#[test]
fn test_feature_flags_integration() {
    let mut flags = FeatureFlags::default();
    flags.set(FLAG_AI_ANALYZERS, true);

    assert!(flags.get(FLAG_AI_ANALYZERS));
    assert!(!flags.get("js_rendering"));

    // Test merge
    let mut override_flags = FeatureFlags::new();
    override_flags.set("js_rendering", true);

    let merged = flags.merge(override_flags);
    assert!(merged.get(FLAG_AI_ANALYZERS));
    assert!(merged.get("js_rendering"));
}

#[test]
fn test_link_graph_integration() {
    let mut graph = LinkGraph::new();

    // Build a link graph
    graph.add_link("https://example.com/", "https://example.com/about");
    graph.add_link("https://example.com/", "https://example.com/contact");
    graph.add_link("https://example.com/about", "https://example.com/");
    graph.add_link("https://example.com/contact", "https://example.com/");
    graph.add_link("https://example.com/blog", "https://example.com/");

    // Compute PageRank
    graph.compute_pagerank(0.85, 20);

    // All pages should have scores
    assert!(!graph.pagerank.is_empty());

    // Homepage should have highest PageRank (most inbound links)
    let homepage_pr = graph.pagerank.get("https://example.com/").unwrap();
    let about_pr = graph.pagerank.get("https://example.com/about").unwrap();
    assert!(homepage_pr > about_pr);
}

#[test]
fn test_link_graph_export() {
    let mut graph = LinkGraph::new();
    graph.add_link("A", "B");
    graph.add_link("B", "C");
    graph.compute_pagerank(0.85, 10);

    // Test DOT export
    let dot = graph.to_dot();
    assert!(dot.contains("digraph"));
    assert!(dot.contains("\"A\" -> \"B\""));
    assert!(dot.contains("\"B\" -> \"C\""));

    // Test CSV export
    let csv = graph.to_csv();
    assert!(csv.contains("source,target"));
    assert!(csv.contains("A,B"));
    assert!(csv.contains("B,C"));
}

#[test]
fn test_observability_metrics() {
    let metrics = crawlkit_core::Metrics::new();

    // Record some activity
    metrics.record_page_success(1024, 1000, 500, 100, 3); // 1ms fetch, 0.5ms analysis
    metrics.record_page_success(2048, 2000, 1000, 200, 5); // 2ms fetch, 1ms analysis
    metrics.record_page_failure();

    // Verify counts
    assert_eq!(
        metrics
            .pages_crawled
            .load(std::sync::atomic::Ordering::Relaxed),
        2
    );
    assert_eq!(
        metrics
            .pages_failed
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(
        metrics
            .bytes_fetched
            .load(std::sync::atomic::Ordering::Relaxed),
        3072
    );
    assert_eq!(
        metrics
            .findings_generated
            .load(std::sync::atomic::Ordering::Relaxed),
        8
    );

    // Verify averages (1.5ms fetch, 0.75ms analysis)
    assert!((metrics.avg_fetch_time_ms() - 1.5).abs() < 0.1);
    assert!((metrics.avg_analysis_time_ms() - 0.75).abs() < 0.1);
}

#[test]
fn test_audit_trail_integration() {
    let trail = crawlkit_core::AuditTrail::new();

    // Record events
    let e1 = trail.record(
        crawlkit_core::AuditEventType::CrawlStarted,
        "test",
        "Crawl started",
    );
    let e2 = trail.record(
        crawlkit_core::AuditEventType::PageFetched,
        "test",
        "Page fetched",
    );
    let e3 = trail.record(
        crawlkit_core::AuditEventType::CrawlCompleted,
        "test",
        "Crawl completed",
    );

    // Verify chain integrity
    assert!(trail.verify_integrity());
    assert_eq!(trail.len(), 3);

    // Verify chain links
    assert_eq!(e1.previous_hash, "genesis");
    assert_eq!(e2.previous_hash, e1.hash);
    assert_eq!(e3.previous_hash, e2.hash);
}

#[test]
fn test_playwright_config() {
    let config = PlaywrightConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.max_concurrent, 5);
    assert!(config.headless);
    assert_eq!(config.max_memory_per_context, 512 * 1024 * 1024);

    let renderer = PlaywrightRenderer::new(config);
    assert!(!renderer.is_available());
    assert_eq!(renderer.active_contexts(), 0);
}

#[test]
fn test_rate_limiter_integration() {
    let limiter = RateLimiter::new(10.0, 10.0); // 10 tokens/sec, burst 10

    // Should be able to acquire tokens
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let permit = limiter.acquire("example.com").await;
        assert!(permit.is_ok());
    });
}

#[test]
fn test_wasm_pattern_analyzer_integration() {
    let analyzer = WasmPatternAnalyzer::new();
    let config = make_test_config();

    let page = ParsedPage {
        url: "https://example.com".to_string(),
        meta: MetaTags::default(),
        headings: Vec::new(),
        links: Vec::new(),
        images: Vec::new(),
        forms: Vec::new(),
        scripts: vec![ScriptInfo {
            src: Some("module.wasm".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
        }],
        styles: Vec::new(),
        structured_data: Vec::new(),
        word_count: 0,
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
    };

    let ctx = AnalysisContext {
        page: &page,
        status_code: Some(200),
        headers: &[],
        response_time: Some(Duration::from_millis(100)),
        redirect_chain: &[],
    };

    let findings = analyzer.analyze(&ctx, &config);
    // Should detect WASM patterns
    assert!(findings.iter().any(|f| f.code.starts_with("WASM")));
}

#[test]
fn test_ai_analyzer_integration() {
    let analyzer = AiCrawlerAccessibilityAnalyzer::new();
    let config = make_test_config();

    let page = make_test_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        status_code: Some(200),
        headers: &[],
        response_time: Some(Duration::from_millis(100)),
        redirect_chain: &[],
    };

    let findings = analyzer.analyze(&ctx, &config);
    // Should find "no robots.txt" since we don't have one
    assert!(findings.iter().any(|f| f.code == "AI-ACC009"));
}

#[test]
fn test_storage_integration() {
    let storage = Storage::new_in_memory().unwrap();

    // Start a crawl
    let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
    assert!(!crawl_id.is_empty());

    // Insert a page
    let page = make_page_data("https://example.com/test");
    storage.insert_page(&crawl_id, &page).unwrap();

    // Insert an issue
    let issue = Issue {
        id: uuid::Uuid::new_v4().to_string(),
        page_id: page.id.clone(),
        category: IssueCategory::Seo,
        severity: Severity::Warning,
        code: "SEO001".to_string(),
        title: "Missing meta description".to_string(),
        description: "Page lacks meta description".to_string(),
        element: None,
        recommendation: "Add meta description".to_string(),
    };
    storage.insert_issue(&issue).unwrap();
}

// ---------------------------------------------------------------------------
// End-to-End Tests
// ---------------------------------------------------------------------------

#[test]
fn test_full_crawl_pipeline() {
    // This test simulates a complete crawl pipeline
    let config = make_test_config();
    let registry = AnalyzerRegistry::new(&config);
    let storage = Storage::new_in_memory().unwrap();
    let metrics = crawlkit_core::Metrics::new();

    // Start crawl
    let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

    // Simulate fetching and analyzing 5 pages
    for i in 0..5 {
        let url = format!("https://example.com/page{}", i);
        let parsed_page = make_test_page(&url);

        let ctx = AnalysisContext {
            page: &parsed_page,
            status_code: Some(200),
            headers: &[],
            response_time: Some(Duration::from_millis(100)),
            redirect_chain: &[],
        };

        let findings = registry.analyze(&ctx, &config);

        // Store page
        let page = make_page_data(&url);
        storage.insert_page(&crawl_id, &page).unwrap();

        // Store findings
        for finding in &findings {
            let issue = Issue {
                id: uuid::Uuid::new_v4().to_string(),
                page_id: page.id.clone(),
                category: finding.category.clone(),
                severity: finding.severity.clone(),
                code: finding.code.clone(),
                title: finding.title.clone(),
                description: finding.description.clone(),
                element: None,
                recommendation: finding.recommendation.clone(),
            };
            storage.insert_issue(&issue).unwrap();
        }

        // Update metrics
        metrics.record_page_success(1024, 100, 50, 10, findings.len() as u64);
    }

    // Verify final state
    assert_eq!(
        metrics
            .pages_crawled
            .load(std::sync::atomic::Ordering::Relaxed),
        5
    );
    assert!(
        metrics
            .findings_generated
            .load(std::sync::atomic::Ordering::Relaxed)
            > 0
    );
}
