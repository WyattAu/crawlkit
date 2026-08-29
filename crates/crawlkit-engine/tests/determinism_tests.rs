//! Determinism replay tests for crawlkit-engine.
//!
//! Proves the determinism rails end-to-end:
//! - Seeded user-agent rotation is a pure function of `(url, seed)`.
//! - `DeterminismController::derive_seed` is a pure function of
//!   `(seed, context)`; only the stream variant is order-sensitive.
//! - Exports are byte-identical for identical input, even when pages and
//!   findings were inserted in deliberately unsorted order.
//! - `AnalyzerRegistry::analyze` returns canonically ordered findings.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use chrono::Utc;
use url::Url;

use crawlkit_engine::analyzers::{AnalysisContext, Analyzer, AnalyzerRegistry, Finding};
use crawlkit_engine::export::{
    export_csv, export_html, export_json, export_markdown, CsvColumnSelector,
};
use crawlkit_engine::http::{HttpClientConfig, UserAgentRotator};
use crawlkit_engine::meta::MetaTags;
use crawlkit_engine::parser::{Heading, ParsedPage};
use crawlkit_engine::storage::{Issue, IssueCategory, PageData, Severity, Storage};
use crawlkit_engine::{CrawlConfig, DeterminismController};

// ---------------------------------------------------------------------------
// Seeded UA rotation
// ---------------------------------------------------------------------------

#[test]
fn ua_for_url_is_stable_across_instances_and_repeated_calls() {
    let agents: Vec<String> = (1..=6).map(|i| format!("agent-{i}")).collect();
    let rotator_a = UserAgentRotator::new(agents.clone());
    let rotator_b = UserAgentRotator::new(agents);

    let urls: Vec<String> = (0..50)
        .map(|i| format!("https://example.com/pages/{i}"))
        .collect();

    // Round-robin state changes must not affect seeded selection.
    for _ in 0..10 {
        let _ = rotator_a.next();
        let _ = rotator_b.next();
    }

    for url in &urls {
        let first = rotator_a.ua_for_url(url, 1234);
        assert_eq!(
            first,
            rotator_a.ua_for_url(url, 1234),
            "repeated call returned a different agent for {url}"
        );
        assert_eq!(
            first,
            rotator_b.ua_for_url(url, 1234),
            "different instance returned a different agent for {url}"
        );
    }
}

#[test]
fn ua_for_url_different_seed_changes_selection() {
    // Small pool: different seeds should distribute differently for at
    // least half the URLs (statistically overwhelming with a 64-bit hash).
    let agents: Vec<String> = (1..=5).map(|i| format!("agent-{i}")).collect();
    let rotator = UserAgentRotator::new(agents);

    let urls: Vec<String> = (0..100)
        .map(|i| format!("https://example.com/p/{i}"))
        .collect();
    let differing = urls
        .iter()
        .filter(|u| rotator.ua_for_url(u, 1) != rotator.ua_for_url(u, 2))
        .count();

    assert!(
        differing >= urls.len() / 2,
        "seeds 1 and 2 only differed on {differing}/{} URLs",
        urls.len()
    );
}

#[test]
fn ua_for_url_covers_pool_deterministically() {
    // With enough URLs every pool member should be selected (coverage
    // sanity — deterministic since DefaultHasher is stable).
    let agents: Vec<String> = (1..=8).map(|i| format!("agent-{i}")).collect();
    let rotator = UserAgentRotator::new(agents.clone());
    let selected: BTreeSet<&str> = (0..256)
        .map(|i| rotator.ua_for_url(&format!("https://example.com/x/{i}"), 99))
        .collect();
    let expected: BTreeSet<&str> = agents.iter().map(String::as_str).collect();
    assert_eq!(selected, expected);
}

#[test]
fn http_client_config_seed_builder() {
    let base = HttpClientConfig::from(&CrawlConfig::default());
    assert_eq!(base.seed, None, "unseeded by default");

    let seeded = base.clone().with_seed(42);
    assert_eq!(seeded.seed, Some(42));
    // The builder only touches the seed.
    assert_eq!(seeded.timeout, base.timeout);
    assert_eq!(seeded.max_body_size, base.max_body_size);
    assert_eq!(seeded.allow_http, base.allow_http);
}

// ---------------------------------------------------------------------------
// DeterminismController
// ---------------------------------------------------------------------------

#[test]
fn derive_seed_is_pure_and_stream_is_order_sensitive() {
    let ctrl = DeterminismController::new(42);

    // Pure variant: same context → same seed, regardless of interleaving.
    let first = ctrl.derive_seed("page/1");
    assert_eq!(first, ctrl.derive_seed("page/1"));
    let _ = ctrl.derive_seed("page/2");
    let _ = ctrl.derive_seed_stream("page/2");
    assert_eq!(ctrl.derive_seed("page/1"), first);

    // A fresh controller with the same base seed agrees.
    let ctrl2 = DeterminismController::new(42);
    assert_eq!(ctrl.derive_seed("page/1"), ctrl2.derive_seed("page/1"));

    // Different base seed → different derivation (statistically).
    let ctrl3 = DeterminismController::new(43);
    assert_ne!(ctrl.derive_seed("page/1"), ctrl3.derive_seed("page/1"));

    // Stream variant: unique per call, same context.
    assert_ne!(
        ctrl.derive_seed_stream("page/1"),
        ctrl.derive_seed_stream("page/1")
    );
}

// ---------------------------------------------------------------------------
// Export byte-determinism
// ---------------------------------------------------------------------------

/// Build a crawl whose storage insertion order is deliberately NOT the
/// canonical export order (URLs and issue codes both shuffled).
fn seed_unsorted_crawl(storage: &Storage, crawl_id: &str) {
    let page_data = |id: &str, path: &str, status: u16| PageData {
        id: id.to_string(),
        url: Url::parse(&format!("https://example.com{path}")).unwrap(),
        final_url: Url::parse(&format!("https://example.com{path}")).unwrap(),
        status_code: status,
        title: Some(format!("Title {path}")),
        description: Some("Desc".to_string()),
        canonical_url: None,
        word_count: Some(100),
        load_time_ms: Some(50),
        body_size: Some(2048),
        fetched_at: Utc::now(),
        links: vec![
            Url::parse("https://example.com/zeta").unwrap(),
            Url::parse("https://example.com/alpha").unwrap(),
        ],
        tenant_id: None,
        etag: None,
        last_modified: None,
        cwv_lcp: None,
        cwv_cls: None,
        cwv_inp: None,
        has_structured_data: None,
        schema_types: None,
        viewport_ok: None,
        has_csp: None,
        has_hsts: None,
        images_total: None,
        images_missing_alt: None,
        h1_count: None,
        heading_count: None,
    };

    // Deliberately unsorted insertion order.
    let pages = vec![
        page_data("p-zeta", "/zeta", 200),
        page_data("p-mid", "/mid", 404),
        page_data("p-alpha", "/alpha", 200),
        page_data("p-beta", "/beta", 301),
    ];
    storage.insert_pages(crawl_id, &pages).unwrap();

    let issue = |id: &str, page_id: &str, code: &str, element: Option<&str>| Issue {
        id: id.to_string(),
        page_id: page_id.to_string(),
        category: IssueCategory::Seo,
        severity: Severity::Warning,
        code: code.to_string(),
        title: format!("issue {code}"),
        description: "desc".to_string(),
        element: element.map(str::to_string),
        recommendation: "fix".to_string(),
        tenant_id: None,
    };

    // Deliberately unsorted by (code, element), spread over pages, with a
    // MIX of categories and severities so aggregate stats maps have
    // multiple entries (their serialization order must also be canonical).
    let issues = vec![
        issue("i-5", "p-beta", "ZZZ900", None),
        issue("i-1", "p-alpha", "AAA100", Some("meta[name=x]")),
        issue("i-3", "p-zeta", "MMM300", Some("a.link")),
        issue("i-2", "p-alpha", "AAA100", Some("meta[name=a]")),
        issue("i-4", "p-zeta", "MMM200", None),
        Issue {
            category: IssueCategory::Images,
            severity: Severity::Error,
            ..issue("i-6", "p-mid", "CCC400", None)
        },
        Issue {
            category: IssueCategory::Http,
            severity: Severity::Info,
            ..issue("i-7", "p-mid", "CCC500", None)
        },
        Issue {
            category: IssueCategory::Performance,
            severity: Severity::Critical,
            ..issue("i-8", "p-beta", "BBB600", None)
        },
    ];
    storage.insert_issues(&issues).unwrap();
    storage.finish_crawl(crawl_id, 4, issues.len()).unwrap();
}

#[test]
fn exports_are_byte_identical_across_runs() {
    let storage = Storage::new_in_memory().unwrap();
    let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
    seed_unsorted_crawl(&storage, &crawl_id);

    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();

    for (dir, run) in [(dir_a.path(), "a"), (dir_b.path(), "b")] {
        let json = export_json(&storage, &crawl_id, false).unwrap();
        let json_pretty = export_json(&storage, &crawl_id, true).unwrap();
        let markdown = export_markdown(&storage, &crawl_id).unwrap();
        let html = export_html(&storage, &crawl_id).unwrap();
        let csv = export_csv(&storage, &crawl_id, &CsvColumnSelector::all()).unwrap();

        std::fs::write(dir.join("report.json"), &json).unwrap();
        std::fs::write(dir.join("report-pretty.json"), &json_pretty).unwrap();
        std::fs::write(dir.join("report.md"), &markdown).unwrap();
        std::fs::write(dir.join("report.html"), &html).unwrap();
        std::fs::write(dir.join("report.csv"), &csv).unwrap();
        let _ = run; // both runs use identical fixtures
    }

    for name in [
        "report.json",
        "report-pretty.json",
        "report.md",
        "report.html",
        "report.csv",
    ] {
        let bytes_a = std::fs::read(dir_a.path().join(name)).unwrap();
        let bytes_b = std::fs::read(dir_b.path().join(name)).unwrap();
        assert_eq!(
            bytes_a, bytes_b,
            "{name} differs between identical export runs"
        );
    }
}

#[test]
fn exports_emit_canonical_ordering() {
    let storage = Storage::new_in_memory().unwrap();
    let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
    seed_unsorted_crawl(&storage, &crawl_id);

    let json = export_json(&storage, &crawl_id, false).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let pages = v["pages"].as_array().unwrap();

    // Pages sorted by URL string.
    let urls: Vec<&str> = pages.iter().map(|p| p["url"].as_str().unwrap()).collect();
    let mut sorted = urls.clone();
    sorted.sort();
    assert_eq!(urls, sorted, "pages must be exported in URL order");

    // Findings sorted by code (element as tiebreak).
    for page in pages {
        let codes: Vec<(String, String)> = page["issues"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|i| {
                        (
                            i["code"].as_str().unwrap().to_string(),
                            i["element"].as_str().unwrap_or_default().to_string(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut sorted = codes.clone();
        sorted.sort();
        assert_eq!(codes, sorted, "issues must be sorted by (code, element)");
    }

    // Links sorted within each page.
    for page in pages {
        let links: Vec<&str> = page["links"]
            .as_array()
            .map(|a| a.iter().filter_map(|l| l.as_str()).collect())
            .unwrap_or_default();
        let mut sorted = links.clone();
        sorted.sort();
        assert_eq!(links, sorted, "links must be sorted");
    }
}

// ---------------------------------------------------------------------------
// AnalyzerRegistry canonical ordering
// ---------------------------------------------------------------------------

fn minimal_page(url: &str) -> ParsedPage {
    ParsedPage {
        url: url.to_string(),
        meta: MetaTags::default(),
        headings: vec![Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        }],
        links: Vec::new(),
        images: Vec::new(),
        forms: Vec::new(),
        scripts: Vec::new(),
        styles: Vec::new(),
        structured_data: Vec::new(),
        word_count: 10,
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

struct NoisyAnalyzer {
    codes: &'static [&'static str],
}

impl Analyzer for NoisyAnalyzer {
    fn name(&self) -> &str {
        "noisy"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        self.codes
            .iter()
            .map(|code| Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: (*code).to_string(),
                title: format!("finding {code}"),
                description: "test".to_string(),
                url: ctx.page.url.clone(),
                recommendation: "none".to_string(),
            })
            .collect()
    }
}

#[test]
fn analyzer_registry_orders_findings_canonically_and_repeatably() {
    let registry = AnalyzerRegistry::with_analyzers(vec![
        // Each analyzer emits its codes in a deliberately non-canonical order.
        Box::new(NoisyAnalyzer {
            codes: &["CCC003", "AAA001", "BBB002"],
        }),
        Box::new(NoisyAnalyzer {
            codes: &["AAA003", "AAA002"],
        }),
    ]);

    let page = minimal_page("https://example.com/page");
    let ctx = AnalysisContext {
        page: &page,
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
    };

    let run_keys = || -> Vec<String> {
        registry
            .analyze(&ctx)
            .iter()
            .map(|f| format!("{}|{}", f.code, f.url))
            .collect()
    };

    let first = run_keys();
    let second = run_keys();

    // Canonically ordered by (code, url).
    let mut sorted = first.clone();
    sorted.sort();
    assert_eq!(first, sorted, "findings must be sorted by (code, url)");

    // Repeated runs are identical (replay proof).
    assert_eq!(first, second);
    assert_eq!(first.len(), 5);
}

// ---------------------------------------------------------------------------
// A11Y001 decorative-image regression (WCAG H67)
// ---------------------------------------------------------------------------

/// Real-world pattern from kingstonpeptides.com: trust badges with
/// `alt=""` + `aria-hidden="true"` and adjacent text carrying the meaning.
/// This is the textbook H67 decorative-image technique and must NOT be
/// flagged by A11Y001.
#[test]
fn decorative_badges_with_empty_alt_and_aria_hidden_are_not_flagged() {
    use crawlkit_engine::parser::HtmlParser;
    use crawlkit_engine::{AccessibilityAnalyzer, AnalysisContext, Analyzer};

    let html = r#"<!DOCTYPE html>
<html lang="en"><head><title>KP</title></head><body>
  <div class="trust-badges">
    <div><img src="/images/badges/coa-badge.svg" alt="" class="h-4 w-4" aria-hidden="true" />
      <span>HPLC Certified</span></div>
    <div><img src="/images/badges/verified-badge.svg" alt="" aria-hidden="true" />
      <span>99%+ Purity Guaranteed</span></div>
    <div><img src="/images/badges/ssl-badge.svg" alt="" aria-hidden="true" />
      <span>SSL Secured</span></div>
  </div>
  <img src="/images/product.png" alt="BPC-157 peptide vial" />
</body></html>"#;

    let url = url::Url::parse("https://kingstonpeptides.com/").unwrap();
    let page = HtmlParser::parse(html, &url);

    // Parser extracted the decorative semantics correctly.
    let badge = page
        .images
        .iter()
        .find(|i| i.src.contains("coa-badge"))
        .unwrap();
    assert!(
        badge.has_alt,
        "alt=\"\" must count as having an alt attribute"
    );
    assert!(badge.aria_hidden, "aria-hidden=true must be parsed");

    let ctx = AnalysisContext {
        page: &page,
        body: Some(html),
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
    };
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(
        findings.iter().all(|f| f.code != "A11Y001"),
        "decorative badges must not trigger A11Y001; got: {:?}",
        findings.iter().map(|f| f.code.as_str()).collect::<Vec<_>>()
    );
}

/// Empty alt WITHOUT aria-hidden is still valid H67 (axe-core semantics):
/// the author has explicitly declared the image decorative.
#[test]
fn empty_alt_alone_is_not_flagged() {
    use crawlkit_engine::parser::HtmlParser;
    use crawlkit_engine::{AccessibilityAnalyzer, AnalysisContext, Analyzer};

    let html = r#"<html><body><img src="/divider.png" alt="" /></body></html>"#;
    let url = url::Url::parse("https://example.com/").unwrap();
    let page = HtmlParser::parse(html, &url);
    let ctx = AnalysisContext {
        page: &page,
        body: Some(html),
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
    };
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().all(|f| f.code != "A11Y001"));
}

/// A missing alt attribute entirely remains a genuine WCAG 1.1.1 failure.
#[test]
fn missing_alt_attribute_is_still_flagged() {
    use crawlkit_engine::parser::HtmlParser;
    use crawlkit_engine::{AccessibilityAnalyzer, AnalysisContext, Analyzer};

    let html = r#"<html><body><img src="/photo.jpg" /></body></html>"#;
    let url = url::Url::parse("https://example.com/").unwrap();
    let page = HtmlParser::parse(html, &url);
    assert!(!page.images[0].has_alt);

    let ctx = AnalysisContext {
        page: &page,
        body: Some(html),
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
    };
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y001"));
}
