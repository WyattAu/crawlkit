//! Panic-isolation contract for `AnalyzerRegistry::analyze`.
//!
//! A panicking analyzer must never abort the crawl: the registry converts
//! the panic into a single `ANALYZER-PANIC` error finding and every other
//! analyzer still runs.

use crate::analyzers::{AnalysisContext, Analyzer, AnalyzerRegistry, Finding};
use crate::meta::MetaTags;
use crate::parser::ParsedPage;
use crate::types::{IssueCategory, Severity};

/// Analyzer that always panics with a string payload.
struct PanickingAnalyzer;

impl Analyzer for PanickingAnalyzer {
    fn name(&self) -> &str {
        "panicking-analyzer"
    }

    fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
        panic!("boom: intentional test panic");
    }
}

/// Analyzer that always panics with a non-string payload.
struct OpaquePanickingAnalyzer;

impl Analyzer for OpaquePanickingAnalyzer {
    fn name(&self) -> &str {
        "opaque-panicking-analyzer"
    }

    fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
        std::panic::panic_any(42_u32);
    }
}

/// Well-behaved analyzer that emits one normal finding.
struct HealthyAnalyzer;

impl Analyzer for HealthyAnalyzer {
    fn name(&self) -> &str {
        "healthy-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        vec![Finding {
            severity: Severity::Info,
            category: IssueCategory::Seo,
            code: "HEALTHY-001".to_string(),
            title: "Healthy analyzer ran".to_string(),
            description: "The healthy analyzer completed normally.".to_string(),
            url: ctx.page.url.to_string(),
            recommendation: "No action required.".to_string(),
        }]
    }
}

fn make_page() -> ParsedPage {
    ParsedPage {
        url: "https://example.com/page".to_string(),
        meta: MetaTags::default(),
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

fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
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
fn panicking_analyzer_is_isolated_and_healthy_analyzer_still_runs() {
    let registry = AnalyzerRegistry::with_analyzers(vec![
        Box::new(PanickingAnalyzer),
        Box::new(HealthyAnalyzer),
    ]);
    let page = make_page();
    let ctx = make_ctx(&page);

    let findings = registry.analyze(&ctx);

    // The panic became a single isolated error finding.
    let panics: Vec<_> = findings
        .iter()
        .filter(|f| f.code == "ANALYZER-PANIC")
        .collect();
    assert_eq!(
        panics.len(),
        1,
        "expected exactly one ANALYZER-PANIC finding"
    );
    assert_eq!(panics[0].severity, Severity::Error);
    assert!(
        panics[0].title.contains("panicking-analyzer"),
        "panic finding must name the failing analyzer: {}",
        panics[0].title
    );
    assert!(
        panics[0]
            .description
            .contains("boom: intentional test panic"),
        "panic detail should carry the panic message: {}",
        panics[0].description
    );
    assert_eq!(panics[0].url, "https://example.com/page");

    // The healthy analyzer was unaffected.
    assert!(
        findings.iter().any(|f| f.code == "HEALTHY-001"),
        "healthy analyzer must still run after a sibling panic"
    );
}

#[test]
fn non_string_panic_payload_is_handled() {
    let registry = AnalyzerRegistry::with_analyzers(vec![Box::new(OpaquePanickingAnalyzer)]);
    let page = make_page();
    let ctx = make_ctx(&page);

    let findings = registry.analyze(&ctx);

    let panics: Vec<_> = findings
        .iter()
        .filter(|f| f.code == "ANALYZER-PANIC")
        .collect();
    assert_eq!(panics.len(), 1);
    assert!(
        panics[0].description.contains("unknown panic payload"),
        "opaque payloads must fall back to a placeholder: {}",
        panics[0].description
    );
}

#[test]
fn all_panicking_registry_still_returns_canonical_output() {
    let registry = AnalyzerRegistry::with_analyzers(vec![
        Box::new(OpaquePanickingAnalyzer),
        Box::new(PanickingAnalyzer),
    ]);
    let page = make_page();
    let ctx = make_ctx(&page);

    let findings = registry.analyze(&ctx);

    // One isolated finding per panicking analyzer, still canonically ordered.
    assert_eq!(findings.len(), 2);
    let codes: Vec<_> = findings.iter().map(|f| f.code.as_str()).collect();
    let mut sorted = codes.clone();
    sorted.sort();
    assert_eq!(codes, sorted, "output must remain canonically ordered");
    assert!(codes.iter().all(|c| *c == "ANALYZER-PANIC"));
}
