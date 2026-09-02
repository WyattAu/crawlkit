use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::ParsedPage;

fn make_page(url: &str) -> ParsedPage {
    ParsedPage {
        url: url.to_string(),
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

fn make_ctx_with_headers<'a>(
    page: &'a ParsedPage,
    headers: &'a [(String, String)],
) -> AnalysisContext<'a> {
    AnalysisContext {
        page,
        body: None,
        status_code: Some(200),
        headers,
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
fn test_co_no_headers() {
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &[]);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COEP001-ISOLATION"));
    assert!(findings.iter().any(|f| f.code == "COOP002-ISOLATION"));
}

#[test]
fn test_co_coep_present() {
    let headers = vec![(
        "Cross-Origin-Embedder-Policy".to_string(),
        "require-corp".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "COEP001-ISOLATION"));
    assert!(findings.iter().any(|f| f.code == "COOP002-ISOLATION"));
}

#[test]
fn test_co_coop_present() {
    let headers = vec![(
        "Cross-Origin-Opener-Policy".to_string(),
        "same-origin".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COEP001-ISOLATION"));
    assert!(!findings.iter().any(|f| f.code == "COOP002-ISOLATION"));
}

#[test]
fn test_co_both_present() {
    let headers = vec![
        (
            "Cross-Origin-Embedder-Policy".to_string(),
            "require-corp".to_string(),
        ),
        (
            "Cross-Origin-Opener-Policy".to_string(),
            "same-origin".to_string(),
        ),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_co_case_insensitive_header_names() {
    let headers = vec![
        (
            "cross-origin-embedder-policy".to_string(),
            "require-corp".to_string(),
        ),
        (
            "cross-origin-opener-policy".to_string(),
            "same-origin".to_string(),
        ),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_co_only_coep_present() {
    let headers = vec![
        (
            "Cross-Origin-Embedder-Policy".to_string(),
            "require-corp".to_string(),
        ),
        (
            "Cross-Origin-Resource-Policy".to_string(),
            "same-origin".to_string(),
        ),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "COEP001-ISOLATION"));
    assert!(findings.iter().any(|f| f.code == "COOP002-ISOLATION"));
}

#[test]
fn test_co_only_coop_present() {
    let headers = vec![
        (
            "Cross-Origin-Opener-Policy".to_string(),
            "same-origin".to_string(),
        ),
        (
            "Cross-Origin-Resource-Policy".to_string(),
            "same-origin".to_string(),
        ),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COEP001-ISOLATION"));
    assert!(!findings.iter().any(|f| f.code == "COOP002-ISOLATION"));
}

#[test]
fn test_co_corp_only_no_coep_no_coop() {
    let headers = vec![(
        "Cross-Origin-Resource-Policy".to_string(),
        "same-origin".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COEP001-ISOLATION"));
    assert!(findings.iter().any(|f| f.code == "COOP002-ISOLATION"));
}

#[test]
fn test_co_coep_wrong_value_still_present() {
    let headers = vec![(
        "Cross-Origin-Embedder-Policy".to_string(),
        "unsafe-none".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    // Only checks presence, not value
    assert!(!findings.iter().any(|f| f.code == "COEP001-ISOLATION"));
}

#[test]
fn test_co_all_cross_origin_headers_present() {
    let headers = vec![
        (
            "Cross-Origin-Embedder-Policy".to_string(),
            "require-corp".to_string(),
        ),
        (
            "Cross-Origin-Opener-Policy".to_string(),
            "same-origin".to_string(),
        ),
        (
            "Cross-Origin-Resource-Policy".to_string(),
            "same-site".to_string(),
        ),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_co_only_coep_wrong_value() {
    let headers = vec![(
        "Cross-Origin-Embedder-Policy".to_string(),
        "unsafe-none".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "COEP001-ISOLATION"));
    assert!(findings.iter().any(|f| f.code == "COOP002-ISOLATION"));
}

#[test]
fn test_co_mixed_case_header_names() {
    let headers = vec![
        (
            "cross-origin-embedder-Policy".to_string(),
            "require-corp".to_string(),
        ),
        (
            "Cross-Origin-OPENER-policy".to_string(),
            "same-origin".to_string(),
        ),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = CrossOriginIsolationAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}
