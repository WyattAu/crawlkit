use crate::analyzers::*;
use crate::parser::ParsedPage;
use crate::meta::MetaTags;

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
fn test_hsts_no_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &[]);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_hsts_missing_include_subdomains() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HSTS001"));
}

#[test]
fn test_hsts_missing_preload() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000; includeSubDomains".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HSTS002"));
    assert!(!findings.iter().any(|f| f.code == "HSTS001"));
}

#[test]
fn test_hsts_max_age_too_low() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=300; includeSubDomains; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HSTS003"));
}

#[test]
fn test_hsts_perfect() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=63072000; includeSubDomains; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_hsts_max_age_exact_31536000() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000; includeSubDomains; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "HSTS003"));
}

#[test]
fn test_hsts_case_insensitive_directives() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000; INCLUDESUBDOMAINS; PRELOAD".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_hsts_all_issues() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=100".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HSTS001"));
    assert!(findings.iter().any(|f| f.code == "HSTS002"));
    assert!(findings.iter().any(|f| f.code == "HSTS003"));
}

#[test]
fn test_hsts_max_age_missing() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "includeSubDomains".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    // includeSubDomains is present, so HSTS001 should NOT fire
    assert!(!findings.iter().any(|f| f.code == "HSTS001"));
    assert!(findings.iter().any(|f| f.code == "HSTS002"));
}

#[test]
fn test_hsts_partial_include_subdomains() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HSTS001"));
    assert!(!findings.iter().any(|f| f.code == "HSTS002"));
}

#[test]
fn test_hsts_only_preload_missing() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=63072000; includeSubDomains".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "HSTS001"));
    assert!(findings.iter().any(|f| f.code == "HSTS002"));
    assert!(!findings.iter().any(|f| f.code == "HSTS003"));
}

#[test]
fn test_hsts_max_age_just_below() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31535999; includeSubDomains; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HSTS003"));
}

#[test]
fn test_hsts_max_age_very_high() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=315360000; includeSubDomains; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_hsts_lowercase_header_name() {
    let headers = vec![(
        "strict-transport-security".to_string(),
        "max-age=31536000; includeSubDomains; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_hsts_directives_case_insensitive() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "Max-Age=31536000; includesubdomains; PRELOAD".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = HstsPreloadAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}
