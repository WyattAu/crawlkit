use super::*;
use crate::analyzers::{
    AriaRolesAnalyzer, CookieAnalyzer, FocusManagementAnalyzer, FormLabelAnalyzer,
    HeadingOrderAnalyzer, ImageAccessibilityAnalyzer, LandmarkRegionsAnalyzer,
    LinkAccessibilityAnalyzer, TableAccessibilityAnalyzer,
};
use crate::analyzers::{
    ContentSecurityPolicyAnalyzer, ContentTypeSniffingAnalyzer, CrossOriginEmbedderPolicyAnalyzer,
    CrossOriginOpenerPolicyAnalyzer, CrossOriginResourcePolicyAnalyzer, MixedContentAnalyzer,
    PermissionsPolicyAnalyzerNew, ReferrerPolicyAnalyzer, StrictTransportSecurityAnalyzer,
    XContentTypeOptionsAnalyzer, XFrameOptionsAnalyzer, XPermittedCrossDomainPoliciesAnalyzer,
    XSSProtectionAnalyzer,
};
use crate::meta::MetaTags;
use crate::parser::{ExtractedImage, ExtractedLink, Heading, ParsedPage};
use url::Url;

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

fn make_ctx<'a>(
    page: &'a ParsedPage,
    status: Option<u16>,
    headers: &'a [(String, String)],
    content_type: Option<&'a str>,
) -> AnalysisContext<'a> {
    AnalysisContext {
        page,
        body: None,
        status_code: status,
        headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type,
        rendered: None,
    }
}

// ===== ContentSecurityPolicyAnalyzer tests =====

#[test]
fn test_csp_no_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(ContentSecurityPolicyAnalyzer::new()
        .analyze(&ctx)
        .is_empty());
}

#[test]
fn test_csp_unsafe_inline() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "script-src 'self' 'unsafe-inline'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CSP001"));
}

#[test]
fn test_csp_no_frame_ancestors() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "default-src 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CSP002"));
}

#[test]
fn test_csp_valid() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "default-src 'self'; script-src 'self'; frame-ancestors 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_csp_both_issues() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "script-src 'self' 'unsafe-inline'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CSP001"));
    assert!(findings.iter().any(|f| f.code == "CSP002"));
}

#[test]
fn test_csp_frame_ancestors_none() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "default-src 'self'; frame-ancestors 'none'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "CSP002"));
}

#[test]
fn test_csp_case_insensitive_header_lookup() {
    let headers = vec![(
        "content-security-policy".to_string(),
        "default-src 'self'; frame-ancestors 'none'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_csp_script_src_in_other_directive_not_flagged() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "style-src 'self' 'unsafe-inline'; frame-ancestors 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "CSP001"));
}

#[test]
fn test_csp_empty_script_src_value() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "script-src; frame-ancestors 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "CSP001"));
}

#[test]
fn test_csp_multiple_script_src_directives() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "default-src 'self'; script-src 'self' 'unsafe-inline'; script-src-elem 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CSP001"));
}

#[test]
fn test_csp_nonce_instead_of_unsafe_inline() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "script-src 'self' 'nonce-abc123'; frame-ancestors 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_csp_empty_csp_value() {
    let headers = vec![("Content-Security-Policy".to_string(), "".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CSP002"));
}

// ===== ReferrerPolicyAnalyzer tests =====

#[test]
fn test_referrer_no_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "REF001"));
}

#[test]
fn test_referrer_unsafe_url() {
    let headers = vec![("Referrer-Policy".to_string(), "unsafe-url".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "REF002"));
}

#[test]
fn test_referrer_valid() {
    let headers = vec![(
        "Referrer-Policy".to_string(),
        "strict-origin-when-cross-origin".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_referrer_no_referrer() {
    let headers = vec![("Referrer-Policy".to_string(), "no-referrer".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_referrer_case_insensitive() {
    let headers = vec![("referrer-policy".to_string(), "unsafe-url".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "REF002"));
}

#[test]
fn test_referrer_origin() {
    let headers = vec![("Referrer-Policy".to_string(), "origin".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_referrer_same_origin() {
    let headers = vec![("Referrer-Policy".to_string(), "same-origin".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_referrer_strict_origin() {
    let headers = vec![("Referrer-Policy".to_string(), "strict-origin".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_referrer_no_referrer_when_downgrade() {
    let headers = vec![(
        "Referrer-Policy".to_string(),
        "no-referrer-when-downgrade".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_referrer_unsafe_url_with_whitespace() {
    let headers = vec![("Referrer-Policy".to_string(), "  unsafe-url  ".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "REF002"));
}

#[test]
fn test_referrer_empty_value() {
    let headers = vec![("Referrer-Policy".to_string(), "".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_referrer_both_findings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "REF001");
}

// ===== XFrameOptionsAnalyzer tests =====

#[test]
fn test_xfo_no_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], Some("text/html"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XFO001"));
}

#[test]
fn test_xfo_allowall() {
    let headers = vec![("X-Frame-Options".to_string(), "ALLOWALL".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XFO002"));
}

#[test]
fn test_xfo_deny() {
    let headers = vec![("X-Frame-Options".to_string(), "DENY".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xfo_sameorigin() {
    let headers = vec![("X-Frame-Options".to_string(), "SAMEORIGIN".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xfo_non_html_page_ignored() {
    let page = make_page("https://example.com/image.png");
    let ctx = make_ctx(&page, Some(200), &[], Some("image/png"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xfo_case_insensitive_header() {
    let headers = vec![("x-frame-options".to_string(), "DENY".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xfo_allowall_case_insensitive() {
    let headers = vec![("x-frame-options".to_string(), "allowall".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XFO002"));
}

#[test]
fn test_xfo_no_content_type_treated_as_html() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XFO001"));
}

#[test]
fn test_xfo_xml_content_type_ignored() {
    let page = make_page("https://example.com/feed.xml");
    let ctx = make_ctx(&page, Some(200), &[], Some("application/xml"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xfo_json_content_type_ignored() {
    let page = make_page("https://example.com/api/data");
    let ctx = make_ctx(&page, Some(200), &[], Some("application/json"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xfo_javascript_content_type_ignored() {
    let page = make_page("https://example.com/app.js");
    let ctx = make_ctx(&page, Some(200), &[], Some("application/javascript"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xfo_css_content_type_ignored() {
    let page = make_page("https://example.com/style.css");
    let ctx = make_ctx(&page, Some(200), &[], Some("text/css"));
    let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

// ===== MixedContentAnalyzer tests =====

#[test]
fn test_mixed_no_resources() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_mixed_http_resources_on_https() {
    let body = r#"<img src="http://cdn.example.com/photo.jpg">"#;
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MIXED001"));
}

#[test]
fn test_mixed_http_form_on_https() {
    let body = r#"<form action="http://example.com/submit">"#;
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MIXED002"));
}

#[test]
fn test_mixed_all_https_no_finding() {
    let body = r#"<img src="https://cdn.example.com/photo.jpg">"#;
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_mixed_http_page_not_checked() {
    let body = r#"<img src="http://cdn.example.com/photo.jpg">"#;
    let page = make_page("http://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    // HTTP pages don't get mixed content warnings
    assert!(findings.is_empty());
}

#[test]
fn test_mixed_multiple_http_resources() {
    let body = r#"
            <img src="http://cdn.example.com/photo1.jpg">
            <img src="http://cdn.example.com/photo2.jpg">
            <script src="http://cdn.example.com/app.js"></script>
            <link href="http://cdn.example.com/style.css" rel="stylesheet">
        "#;
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MIXED001"));
    let f = findings.iter().find(|f| f.code == "MIXED001").unwrap();
    assert!(f.description.contains("4"));
}

#[test]
fn test_mixed_relative_urls_not_flagged() {
    let body = r#"<img src="/photo.jpg">"#;
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_mixed_both_resource_and_form() {
    let body = r#"
            <img src="http://cdn.example.com/photo.jpg">
            <form action="http://example.com/submit">
        "#;
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MIXED001"));
    assert!(findings.iter().any(|f| f.code == "MIXED002"));
}

#[test]
fn test_mixed_form_with_single_quotes() {
    let body = r#"<form action='http://example.com/submit'>"#;
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MIXED002"));
}

#[test]
fn test_mixed_data_uris_not_flagged() {
    let body = r#"<img src="data:image/png;base64,abc123">"#;
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = MixedContentAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

// ===== CookieAnalyzer tests =====

#[test]
fn test_cookie_no_cookies() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = CookieAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_cookie_missing_secure() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session=abc123; HttpOnly; Path=/".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = CookieAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COOKIE001"));
}

#[test]
fn test_cookie_missing_httponly() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session=abc123; Secure; Path=/".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = CookieAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COOKIE002"));
}

#[test]
fn test_cookie_both_flags_missing() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session=abc123; Path=/".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = CookieAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COOKIE001"));
    assert!(findings.iter().any(|f| f.code == "COOKIE002"));
}

#[test]
fn test_cookie_all_flags_present() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session=abc123; Secure; HttpOnly; Path=/".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = CookieAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_cookie_http_page_not_checked() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session=abc123; Path=/".to_string(),
    )];
    let page = make_page("http://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = CookieAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_cookie_multiple_cookies() {
    let headers = vec![
        (
            "Set-Cookie".to_string(),
            "session=abc123; Path=/".to_string(),
        ),
        ("Set-Cookie".to_string(), "token=xyz789; Path=/".to_string()),
    ];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = CookieAnalyzer::new().analyze(&ctx);
    // Both cookies missing both flags = 4 findings
    assert_eq!(findings.len(), 4);
}

#[test]
fn test_cookie_case_insensitive_flags() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session=abc123; secure; httponly; Path=/".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = CookieAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_cookie_session_cookie_name_extracted() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session_id=abc123; Path=/".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = CookieAnalyzer::new().analyze(&ctx);
    let f = findings.iter().find(|f| f.code == "COOKIE001").unwrap();
    assert!(f.description.contains("session_id"));
}

// ===== XContentTypeOptionsAnalyzer tests =====

#[test]
fn test_xcto_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XCTO001"));
}

#[test]
fn test_xcto_nosniff() {
    let headers = vec![("X-Content-Type-Options".to_string(), "nosniff".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xcto_wrong_value() {
    let headers = vec![("X-Content-Type-Options".to_string(), "sniff".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XCTO002"));
}

#[test]
fn test_xcto_case_insensitive() {
    let headers = vec![("x-content-type-options".to_string(), "NOSNIFF".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xcto_whitespace_around_nosniff() {
    let headers = vec![(
        "X-Content-Type-Options".to_string(),
        "  nosniff  ".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

// ===== XPermittedCrossDomainPoliciesAnalyzer tests =====

#[test]
fn test_xpcdp_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XPCDP001"));
}

#[test]
fn test_xpcdp_none() {
    let headers = vec![(
        "X-Permitted-Cross-Domain-Policies".to_string(),
        "none".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_xpcdp_all() {
    let headers = vec![(
        "X-Permitted-Cross-Domain-Policies".to_string(),
        "all".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XPCDP002"));
}

#[test]
fn test_xpcdp_case_insensitive() {
    let headers = vec![(
        "x-permitted-cross-domain-policies".to_string(),
        "ALL".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XPCDP002"));
}

#[test]
fn test_xpcdp_master_only() {
    let headers = vec![(
        "X-Permitted-Cross-Domain-Policies".to_string(),
        "master-only".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
    // master-only is not "all" and not missing
    assert!(!findings.iter().any(|f| f.code == "XPCDP002"));
    assert!(!findings.iter().any(|f| f.code == "XPCDP001"));
}

// ===== CrossOriginResourcePolicyAnalyzer tests =====

#[test]
fn test_corp_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = CrossOriginResourcePolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CORP001"));
}

#[test]
fn test_corp_same_origin() {
    let headers = vec![(
        "Cross-Origin-Resource-Policy".to_string(),
        "same-origin".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginResourcePolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_corp_cross_origin() {
    let headers = vec![(
        "Cross-Origin-Resource-Policy".to_string(),
        "cross-origin".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginResourcePolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_corp_case_insensitive() {
    let headers = vec![(
        "cross-origin-resource-policy".to_string(),
        "same-origin".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginResourcePolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

// ===== LandmarkRegionsAnalyzer tests =====

#[test]
fn test_landmark_missing_all() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LAND001"));
    assert!(findings.iter().any(|f| f.code == "LAND002"));
    assert!(findings.iter().any(|f| f.code == "LAND003"));
}

#[test]
fn test_landmark_has_main() {
    let mut page = make_page("https://example.com");
    page.has_main_landmark = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LAND001"));
}

#[test]
fn test_landmark_has_nav() {
    let mut page = make_page("https://example.com");
    page.has_nav_landmark = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LAND002"));
}

#[test]
fn test_landmark_has_banner() {
    let mut page = make_page("https://example.com");
    page.landmarks.push("banner".to_string());
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LAND003"));
}

#[test]
fn test_landmark_has_header_role() {
    let mut page = make_page("https://example.com");
    page.landmarks.push("header".to_string());
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LAND003"));
}

#[test]
fn test_landmark_all_present_no_findings() {
    let mut page = make_page("https://example.com");
    page.has_main_landmark = true;
    page.has_nav_landmark = true;
    page.landmarks.push("banner".to_string());
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_landmark_only_main_missing() {
    let mut page = make_page("https://example.com");
    page.has_nav_landmark = true;
    page.landmarks.push("banner".to_string());
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LAND001"));
    assert!(!findings.iter().any(|f| f.code == "LAND002"));
    assert!(!findings.iter().any(|f| f.code == "LAND003"));
}

#[test]
fn test_landmark_severity_levels() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
    let land001 = findings.iter().find(|f| f.code == "LAND001").unwrap();
    assert_eq!(land001.severity, Severity::Error);
    let land002 = findings.iter().find(|f| f.code == "LAND002").unwrap();
    assert_eq!(land002.severity, Severity::Warning);
    let land003 = findings.iter().find(|f| f.code == "LAND003").unwrap();
    assert_eq!(land003.severity, Severity::Info);
}

#[test]
fn test_landmark_all_use_accessibility_category() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_landmark_analyzer_name() {
    assert_eq!(LandmarkRegionsAnalyzer::new().name(), "landmark-regions");
}

// ===== HeadingOrderAnalyzer tests =====

#[test]
fn test_heading_order_skip_level() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "H1".to_string(),
            length: 2,
        },
        Heading {
            level: 3,
            text: "H3".to_string(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HORDER001"));
}

#[test]
fn test_heading_order_non_sequential() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "H1".to_string(),
            length: 2,
        },
        Heading {
            level: 2,
            text: "H2".to_string(),
            length: 2,
        },
        Heading {
            level: 3,
            text: "H3".to_string(),
            length: 2,
        },
        Heading {
            level: 2,
            text: "H2b".to_string(),
            length: 3,
        },
        Heading {
            level: 4,
            text: "H4".to_string(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HORDER002"));
}

#[test]
fn test_heading_order_valid_sequence() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "H1".to_string(),
            length: 2,
        },
        Heading {
            level: 2,
            text: "H2".to_string(),
            length: 2,
        },
        Heading {
            level: 3,
            text: "H3".to_string(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_heading_order_same_level_repeated() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 2,
            text: "H2a".to_string(),
            length: 3,
        },
        Heading {
            level: 2,
            text: "H2b".to_string(),
            length: 3,
        },
        Heading {
            level: 2,
            text: "H2c".to_string(),
            length: 3,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_heading_order_single_heading() {
    let mut page = make_page("https://example.com");
    page.headings = vec![Heading {
        level: 1,
        text: "Only".to_string(),
        length: 4,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_heading_order_no_headings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_heading_order_skip_from_h2_to_h4() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 2,
            text: "H2".to_string(),
            length: 2,
        },
        Heading {
            level: 4,
            text: "H4".to_string(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HORDER001"));
}

#[test]
fn test_heading_order_use_accessibility_category() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "H1".to_string(),
            length: 2,
        },
        Heading {
            level: 3,
            text: "H3".to_string(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_heading_order_analyzer_name() {
    assert_eq!(HeadingOrderAnalyzer::new().name(), "heading-order");
}

#[test]
fn test_heading_order_descend_then_ascent() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 3,
            text: "H3".to_string(),
            length: 2,
        },
        Heading {
            level: 2,
            text: "H2".to_string(),
            length: 2,
        },
        Heading {
            level: 3,
            text: "H3b".to_string(),
            length: 3,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HORDER002"));
}

// ===== FormLabelAnalyzer tests =====

#[test]
fn test_form_label_missing_label() {
    use crate::parser::{ExtractedForm, ExtractedInput};
    let mut page = make_page("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![ExtractedInput {
            input_type: Some("text".to_string()),
            name: Some("email".to_string()),
            id: None,
            has_label: false,
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            placeholder: None,
            required: false,
        }],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FormLabelAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "FLABEL001"));
}

#[test]
fn test_form_label_with_aria_label() {
    use crate::parser::{ExtractedForm, ExtractedInput};
    let mut page = make_page("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![ExtractedInput {
            input_type: Some("text".to_string()),
            name: Some("email".to_string()),
            id: None,
            has_label: false,
            aria_label: Some("Email address".to_string()),
            aria_labelledby: None,
            aria_describedby: None,
            placeholder: None,
            required: false,
        }],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FormLabelAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "FLABEL001"));
}

#[test]
fn test_form_label_with_label_element() {
    use crate::parser::{ExtractedForm, ExtractedInput};
    let mut page = make_page("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![ExtractedInput {
            input_type: Some("email".to_string()),
            name: Some("user_email".to_string()),
            id: Some("email".to_string()),
            has_label: true,
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            placeholder: None,
            required: true,
        }],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FormLabelAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_form_label_multiple_inputs_mixed() {
    use crate::parser::{ExtractedForm, ExtractedInput};
    let mut page = make_page("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 2,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![
            ExtractedInput {
                input_type: Some("text".to_string()),
                name: Some("name".to_string()),
                id: None,
                has_label: true,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            },
            ExtractedInput {
                input_type: Some("email".to_string()),
                name: Some("email".to_string()),
                id: None,
                has_label: false,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            },
        ],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FormLabelAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "FLABEL001"));
    let f = findings.iter().find(|f| f.code == "FLABEL001").unwrap();
    assert!(f.description.contains("email"));
}

#[test]
fn test_form_label_with_aria_labelledby() {
    use crate::parser::{ExtractedForm, ExtractedInput};
    let mut page = make_page("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![ExtractedInput {
            input_type: Some("text".to_string()),
            name: Some("search".to_string()),
            id: None,
            has_label: false,
            aria_label: None,
            aria_labelledby: Some("search-label".to_string()),
            aria_describedby: None,
            placeholder: None,
            required: false,
        }],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FormLabelAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "FLABEL001"));
}

#[test]
fn test_form_label_no_forms() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FormLabelAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_form_label_use_accessibility_category() {
    use crate::parser::{ExtractedForm, ExtractedInput};
    let mut page = make_page("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![ExtractedInput {
            input_type: Some("text".to_string()),
            name: Some("field".to_string()),
            id: None,
            has_label: false,
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            placeholder: None,
            required: false,
        }],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FormLabelAnalyzer::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_form_label_analyzer_name() {
    assert_eq!(FormLabelAnalyzer::new().name(), "form-labels");
}

#[test]
fn test_form_label_unnamed_input() {
    use crate::parser::{ExtractedForm, ExtractedInput};
    let mut page = make_page("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![ExtractedInput {
            input_type: Some("text".to_string()),
            name: None,
            id: None,
            has_label: false,
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            placeholder: None,
            required: false,
        }],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FormLabelAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "FLABEL001"));
    let f = findings.iter().find(|f| f.code == "FLABEL001").unwrap();
    assert!(f.description.contains("input (type=\"text\")"));
}

// ===== TableAccessibilityAnalyzer tests =====

#[test]
fn test_table_acc_missing_headers() {
    let mut page = make_page("https://example.com");
    page.tables_total = 3;
    page.tables_with_headers = 1;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TACC001"));
}

#[test]
fn test_table_acc_missing_caption() {
    let mut page = make_page("https://example.com");
    page.tables_total = 2;
    page.tables_with_captions = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TACC002"));
}

#[test]
fn test_table_acc_all_have_headers_and_captions() {
    let mut page = make_page("https://example.com");
    page.tables_total = 5;
    page.tables_with_headers = 5;
    page.tables_with_captions = 5;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_table_acc_no_tables() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_table_acc_large_table_missing_scope() {
    let mut page = make_page("https://example.com");
    page.tables_total = 15;
    page.tables_with_headers = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TACC003"));
}

#[test]
fn test_table_acc_small_table_no_scope_finding() {
    let mut page = make_page("https://example.com");
    page.tables_total = 5;
    page.tables_with_headers = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "TACC003"));
}

#[test]
fn test_table_acc_use_accessibility_category() {
    let mut page = make_page("https://example.com");
    page.tables_total = 1;
    page.tables_with_headers = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_table_acc_analyzer_name() {
    assert_eq!(
        TableAccessibilityAnalyzer::new().name(),
        "table-accessibility"
    );
}

#[test]
fn test_table_acc_all_have_captions_no_headers() {
    let mut page = make_page("https://example.com");
    page.tables_total = 3;
    page.tables_with_headers = 0;
    page.tables_with_captions = 3;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TACC001"));
    assert!(!findings.iter().any(|f| f.code == "TACC002"));
}

#[test]
fn test_table_acc_description_contains_counts() {
    let mut page = make_page("https://example.com");
    page.tables_total = 5;
    page.tables_with_headers = 2;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
    let tacc001 = findings.iter().find(|f| f.code == "TACC001").unwrap();
    assert!(tacc001.description.contains("3 of 5"));
}

// ===== LinkAccessibilityAnalyzer tests =====

#[test]
fn test_link_acc_empty_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: String::new(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LNKACC001"));
}

#[test]
fn test_link_acc_generic_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "click here".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LNKACC002"));
}

#[test]
fn test_link_acc_nondescriptive_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "link".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LNKACC003"));
}

#[test]
fn test_link_acc_good_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/pricing".to_string(),
        text: "View our pricing plans".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_link_acc_with_aria_label() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: String::new(),
        rel: vec![],
        is_external: false,
        aria_label: Some("Go to page".to_string()),
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LNKACC001"));
}

#[test]
fn test_link_acc_with_img_alt() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: String::new(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: Some("Logo link".to_string()),
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LNKACC001"));
}

#[test]
fn test_link_acc_multiple_generic_texts() {
    let mut page = make_page("https://example.com");
    page.links = vec![
        ExtractedLink {
            href: "/a".to_string(),
            text: "read more".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        },
        ExtractedLink {
            href: "/b".to_string(),
            text: "click here".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
    let generic = findings.iter().filter(|f| f.code == "LNKACC002").count();
    assert_eq!(generic, 2);
}

#[test]
fn test_link_acc_use_accessibility_category() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: String::new(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_link_acc_analyzer_name() {
    assert_eq!(
        LinkAccessibilityAnalyzer::new().name(),
        "link-accessibility"
    );
}

#[test]
fn test_link_acc_here_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "here".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LNKACC003"));
}

// ===== ImageAccessibilityAnalyzer tests =====

#[test]
fn test_img_acc_missing_alt() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/photo.jpg".to_string(),
        alt: String::new(),
        width: None,
        height: None,
        has_alt: false,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "IMGACC001"));
}

#[test]
fn test_img_acc_empty_alt_non_decorative() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/photo.jpg".to_string(),
        alt: String::new(),
        width: None,
        height: None,
        has_alt: true,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "IMGACC002"));
}

#[test]
fn test_img_acc_empty_alt_decorative() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/photo.jpg".to_string(),
        alt: String::new(),
        width: None,
        height: None,
        has_alt: true,
        is_lazy_loaded: false,
        aria_hidden: true,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_img_acc_alt_equals_filename() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/images/sunset.jpg".to_string(),
        alt: "sunset".to_string(),
        width: None,
        height: None,
        has_alt: true,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "IMGACC003"));
}

#[test]
fn test_img_acc_good_alt_text() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/images/sunset.jpg".to_string(),
        alt: "Beautiful sunset over the ocean".to_string(),
        width: None,
        height: None,
        has_alt: true,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_img_acc_no_images() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_img_acc_use_accessibility_category() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/a.png".to_string(),
        alt: String::new(),
        width: None,
        height: None,
        has_alt: false,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_img_acc_analyzer_name() {
    assert_eq!(
        ImageAccessibilityAnalyzer::new().name(),
        "image-accessibility"
    );
}

#[test]
fn test_img_acc_multiple_missing_alt() {
    let mut page = make_page("https://example.com");
    page.images = vec![
        ExtractedImage {
            src: "/a.png".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: false,
            is_lazy_loaded: false,
            aria_hidden: false,
        },
        ExtractedImage {
            src: "/b.jpg".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: false,
            is_lazy_loaded: false,
            aria_hidden: false,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
    let imgacc001 = findings.iter().filter(|f| f.code == "IMGACC001").count();
    assert_eq!(imgacc001, 2);
}

#[test]
fn test_img_acc_filename_from_src() {
    assert_eq!(
        ImageAccessibilityAnalyzer::filename_from_src("/images/photo.jpg"),
        Some("photo.jpg")
    );
    assert_eq!(
        ImageAccessibilityAnalyzer::filename_from_src("https://cdn.com/img.png"),
        Some("img.png")
    );
    assert_eq!(
        ImageAccessibilityAnalyzer::filename_from_src("/noext"),
        Some("noext")
    );
}

// ===== AriaRolesAnalyzer tests =====

#[test]
fn test_aria_roles_with_no_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = AriaRolesAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ARIA001"));
}

#[test]
fn test_aria_roles_with_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 3;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = AriaRolesAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_aria_roles_partial_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 5;
    page.aria_label_count = 2;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = AriaRolesAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ARIA002"));
}

#[test]
fn test_aria_roles_no_roles() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = AriaRolesAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_aria_roles_more_labels_than_roles() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 2;
    page.aria_label_count = 5;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = AriaRolesAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_aria_roles_use_accessibility_category() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 1;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = AriaRolesAnalyzer::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_aria_roles_analyzer_name() {
    assert_eq!(AriaRolesAnalyzer::new().name(), "aria-roles");
}

#[test]
fn test_aria_roles_description_contains_count() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 7;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = AriaRolesAnalyzer::new().analyze(&ctx);
    let aria001 = findings.iter().find(|f| f.code == "ARIA001").unwrap();
    assert!(aria001.description.contains("7"));
}

#[test]
fn test_aria_roles_single_role_no_label() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 1;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = AriaRolesAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ARIA001"));
    assert!(!findings.iter().any(|f| f.code == "ARIA002"));
}

// ===== FocusManagementAnalyzer tests =====

#[test]
fn test_focus_positive_tabindex() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FocusManagementAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "FOCUS001"));
}

#[test]
fn test_focus_no_positive_tabindex() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = false;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FocusManagementAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "FOCUS001"));
}

#[test]
fn test_focus_no_focus_styles() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = false;
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "Go".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FocusManagementAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "FOCUS002"));
}

#[test]
fn test_focus_has_focus_visible_style() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = false;
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "Go".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let body = "<style>:focus-visible { outline: 2px solid blue; }</style>";
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = FocusManagementAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "FOCUS002"));
}

#[test]
fn test_focus_has_focus_style() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = false;
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "Go".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let body = "<style>:focus { outline: 2px solid blue; }</style>";
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
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
    let findings = FocusManagementAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "FOCUS002"));
}

#[test]
fn test_focus_no_interactive_elements() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = false;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FocusManagementAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "FOCUS002"));
}

#[test]
fn test_focus_use_accessibility_category() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FocusManagementAnalyzer::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_focus_analyzer_name() {
    assert_eq!(FocusManagementAnalyzer::new().name(), "focus-management");
}

#[test]
fn test_focus_both_issues() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "Go".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FocusManagementAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "FOCUS001"));
    assert!(findings.iter().any(|f| f.code == "FOCUS002"));
}

#[test]
fn test_focus_severity_levels() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "Go".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FocusManagementAnalyzer::new().analyze(&ctx);
    let focus001 = findings.iter().find(|f| f.code == "FOCUS001").unwrap();
    assert_eq!(focus001.severity, Severity::Error);
    let focus002 = findings.iter().find(|f| f.code == "FOCUS002").unwrap();
    assert_eq!(focus002.severity, Severity::Warning);
}

// ===== LanguageAttributeAnalyzer (security) tests =====

#[test]
fn test_lang_acc_missing_lang() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = false;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LANGACC001"));
}

#[test]
fn test_lang_acc_has_lang() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = true;
    page.html_lang = Some("en".to_string());
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LANGACC001"));
}

#[test]
fn test_lang_acc_too_short_value() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = true;
    page.html_lang = Some("e".to_string());
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LANGACC002"));
}

#[test]
fn test_lang_acc_valid_value() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = true;
    page.html_lang = Some("en".to_string());
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LANGACC002"));
}

#[test]
fn test_lang_acc_hreflang_mismatch() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = true;
    page.html_lang = Some("fr".to_string());
    page.word_count = 100;
    page.meta.hreflang = vec![crate::meta::HreflangTag {
        lang: "en".to_string(),
        url: Url::parse("https://example.com/en").unwrap(),
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LANGACC002"));
}

#[test]
fn test_lang_acc_hreflang_match() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = true;
    page.html_lang = Some("en".to_string());
    page.word_count = 100;
    page.meta.hreflang = vec![crate::meta::HreflangTag {
        lang: "en".to_string(),
        url: Url::parse("https://example.com/en").unwrap(),
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LANGACC002"));
}

#[test]
fn test_lang_acc_use_accessibility_category() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = false;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_lang_acc_analyzer_name() {
    assert_eq!(
        LanguageAttributeAnalyzer::new().name(),
        "language-attribute"
    );
}

#[test]
fn test_lang_acc_empty_hreflang_no_mismatch() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = true;
    page.html_lang = Some("de".to_string());
    page.word_count = 100;
    page.meta.hreflang = vec![];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LANGACC002"));
}

#[test]
fn test_lang_acc_zero_words_no_mismatch() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = true;
    page.html_lang = Some("fr".to_string());
    page.word_count = 0;
    page.meta.hreflang = vec![crate::meta::HreflangTag {
        lang: "en".to_string(),
        url: Url::parse("https://example.com/en").unwrap(),
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LANGACC002"));
}

// ===== StrictTransportSecurityAnalyzer tests =====

#[test]
fn test_hsts_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "STRICT001"));
}

#[test]
fn test_hsts_valid() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000; includeSubDomains".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "STRICT001"));
    assert!(!findings.iter().any(|f| f.code == "STRICT002"));
}

#[test]
fn test_hsts_too_short() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=300".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "STRICT002"));
}

#[test]
fn test_hsts_case_insensitive() {
    let headers = vec![(
        "strict-transport-security".to_string(),
        "max-age=63072000".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_hsts_with_preload() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000; includeSubDomains; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_hsts_exact_boundary() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
    // Exactly 31536000 is valid
    assert!(!findings.iter().any(|f| f.code == "STRICT002"));
}

#[test]
fn test_hsts_whitespace_around_max_age() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "  max-age=31536000  ".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_hsts_missing_max_age_param() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "includeSubDomains".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
    // No max-age parsed → treated as missing/valid (no STRICT002)
    assert!(!findings.iter().any(|f| f.code == "STRICT002"));
}

// ===== XSSProtectionAnalyzer tests =====

#[test]
fn test_xss_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XSS001"));
}

#[test]
fn test_xss_mode_block() {
    let headers = vec![("X-XSS-Protection".to_string(), "1; mode=block".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XSS002"));
}

#[test]
fn test_xss_enabled_no_mode_block() {
    let headers = vec![("X-XSS-Protection".to_string(), "1".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
    // Present and not mode=block → no findings
    assert!(findings.is_empty());
}

#[test]
fn test_xss_case_insensitive() {
    let headers = vec![("x-xss-protection".to_string(), "1; mode=block".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XSS002"));
}

#[test]
fn test_xss_zero_disabled() {
    let headers = vec![("X-XSS-Protection".to_string(), "0".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
    // Present, not mode=block → no findings
    assert!(findings.is_empty());
}

#[test]
fn test_xss_whitespace_around_value() {
    let headers = vec![(
        "X-XSS-Protection".to_string(),
        "  1; mode=block  ".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XSS002"));
}

#[test]
fn test_xss_no_header_and_no_csp() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "XSS001"));
}

#[test]
fn test_xss_multiple_headers_last_wins() {
    let headers = vec![
        ("X-XSS-Protection".to_string(), "1".to_string()),
        ("X-XSS-Protection".to_string(), "1; mode=block".to_string()),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
    // Our get_header returns first match, which is "1" (no mode=block)
    assert!(findings.is_empty());
}

// ===== ContentTypeSniffingAnalyzer tests =====

#[test]
fn test_ctsniff_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CTSNIFF001"));
}

#[test]
fn test_ctsniff_nosniff() {
    let headers = vec![("X-Content-Type-Options".to_string(), "nosniff".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_ctsniff_wrong_value() {
    let headers = vec![("X-Content-Type-Options".to_string(), "sniff".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CTSNIFF002"));
}

#[test]
fn test_ctsniff_case_insensitive() {
    let headers = vec![("x-content-type-options".to_string(), "NOSNIFF".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_ctsniff_whitespace_around_nosniff() {
    let headers = vec![(
        "X-Content-Type-Options".to_string(),
        "  nosniff  ".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_ctsniff_empty_value() {
    let headers = vec![("X-Content-Type-Options".to_string(), "".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
    // Empty string is not "nosniff"
    assert!(findings.iter().any(|f| f.code == "CTSNIFF002"));
}

#[test]
fn test_ctsniff_uppercase() {
    let headers = vec![("X-Content-Type-Options".to_string(), "NOSNIFF".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_ctsniff_no_header_implies_vulnerable() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CTSNIFF001"));
    assert!(!findings.iter().any(|f| f.code == "CTSNIFF002"));
}

// =========================================================================
// PermissionsPolicyAnalyzerNew tests
// =========================================================================

#[test]
fn test_pperm_missing_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PPERM001"));
}

#[test]
fn test_pperm_camera_not_restricted() {
    let headers = vec![("Permissions-Policy".to_string(), "camera=self".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PPERM002"));
}

#[test]
fn test_pperm_camera_restricted() {
    let headers = vec![("Permissions-Policy".to_string(), "camera=()".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_pperm_valid_with_multiple_features() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=(), microphone=(), geolocation=()".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_pperm_camera_self_restricted() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=(self)".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_pperm_no_camera_feature() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "microphone=()".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_pperm_empty_header_value() {
    let headers = vec![("Permissions-Policy".to_string(), "".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "PPERM001"));
    assert!(!findings.iter().any(|f| f.code == "PPERM002"));
}

#[test]
fn test_pperm_case_insensitive_camera() {
    let headers = vec![("Permissions-Policy".to_string(), "Camera=self".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PPERM002"));
}

// =========================================================================
// CrossOriginEmbedderPolicyAnalyzer tests
// =========================================================================

#[test]
fn test_coep_missing_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COEP001"));
}

#[test]
fn test_coep_not_require_corp() {
    let headers = vec![(
        "Cross-Origin-Embedder-Policy".to_string(),
        "credentialless".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COEP002"));
}

#[test]
fn test_coep_require_corp_valid() {
    let headers = vec![(
        "Cross-Origin-Embedder-Policy".to_string(),
        "require-corp".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_coep_unsafe_none() {
    let headers = vec![(
        "Cross-Origin-Embedder-Policy".to_string(),
        "unsafe-none".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COEP002"));
}

#[test]
fn test_coep_case_sensitive() {
    let headers = vec![(
        "Cross-Origin-Embedder-Policy".to_string(),
        "Require-Corp".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COEP002"));
}

#[test]
fn test_coep_empty_value() {
    let headers = vec![("Cross-Origin-Embedder-Policy".to_string(), "".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "COEP001"));
    assert!(findings.iter().any(|f| f.code == "COEP002"));
}

#[test]
fn test_coep_with_whitespace() {
    let headers = vec![(
        "Cross-Origin-Embedder-Policy".to_string(),
        " require-corp ".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_coep_no_headers() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert!(findings.iter().any(|f| f.code == "COEP001"));
}

// =========================================================================
// CrossOriginOpenerPolicyAnalyzer tests
// =========================================================================

#[test]
fn test_coop_missing_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COOP001"));
}

#[test]
fn test_coop_not_same_origin() {
    let headers = vec![(
        "Cross-Origin-Opener-Policy".to_string(),
        "same-origin-allow-popups".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COOP002"));
}

#[test]
fn test_coop_same_origin_valid() {
    let headers = vec![(
        "Cross-Origin-Opener-Policy".to_string(),
        "same-origin".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_coop_unsafe_none() {
    let headers = vec![(
        "Cross-Origin-Opener-Policy".to_string(),
        "unsafe-none".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COOP002"));
}

#[test]
fn test_coop_case_sensitive() {
    let headers = vec![(
        "Cross-Origin-Opener-Policy".to_string(),
        "Same-Origin".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "COOP002"));
}

#[test]
fn test_coop_empty_value() {
    let headers = vec![("Cross-Origin-Opener-Policy".to_string(), "".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "COOP001"));
    assert!(findings.iter().any(|f| f.code == "COOP002"));
}

#[test]
fn test_coop_with_whitespace() {
    let headers = vec![(
        "Cross-Origin-Opener-Policy".to_string(),
        " same-origin ".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_coop_no_headers() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert!(findings.iter().any(|f| f.code == "COOP001"));
}
