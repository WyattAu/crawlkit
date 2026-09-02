//! Coverage for deep security-header analyzers (XCTO/Referrer/XFO/
//! Permissions/COI deep generations) and the legacy DNS/SRI/CORS family.
//! Each test pins finding codes so regressions surface immediately.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::{ParsedPage, ScriptInfo};

fn page_at(url: &str) -> ParsedPage {
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

fn ctx<'a>(
    page: &'a ParsedPage,
    status: Option<u16>,
    headers: &'a [(String, String)],
    body: Option<&'a str>,
) -> AnalysisContext<'a> {
    AnalysisContext {
        page,
        body,
        status_code: status,
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

fn hdr(name: &str, value: &str) -> (String, String) {
    (name.to_string(), value.to_string())
}

// === Deep security headers ===

#[test]
fn xcto_deep_flags_missing_header() {
    let page = page_at("https://example.com");
    let ctx = ctx(&page, Some(200), &[], None);
    let f = XContentTypeOptionsDeepAnalyzer::new().analyze(&ctx);
    assert!(f.iter().any(|f| f.code == "XCTODEEP001"), "{f:?}");
}

#[test]
fn xcto_deep_flags_wrong_value() {
    let page = page_at("https://example.com");
    let headers = vec![hdr("X-Content-Type-Options", "allow-sniffing")];
    let ctx = ctx(&page, Some(200), &headers, None);
    let f = XContentTypeOptionsDeepAnalyzer::new().analyze(&ctx);
    assert!(f.iter().any(|f| f.code == "XCTODEEP002"), "{f:?}");
}

#[test]
fn referrer_deep_flags_missing_and_unsafe() {
    let page = page_at("https://example.com");
    let f = ReferrerPolicyDeepAnalyzer::new().analyze(&ctx(&page, Some(200), &[], None));
    assert!(f.iter().any(|f| f.code == "RPDEEP001"), "{f:?}");

    let headers = vec![hdr("Referrer-Policy", "unsafe-url")];
    let f = ReferrerPolicyDeepAnalyzer::new().analyze(&ctx(&page, Some(200), &headers, None));
    assert!(f.iter().any(|f| f.code == "RPDEEP002"), "{f:?}");
}

#[test]
fn xfo_deep_flags_missing_header() {
    let page = page_at("https://example.com");
    let f = XFrameOptionsDeepAnalyzer::new().analyze(&ctx(&page, Some(200), &[], None));
    assert!(f.iter().any(|f| f.code == "XFODEEP001"), "{f:?}");
}

#[test]
fn permissions_deep_flags_missing_header() {
    let page = page_at("https://example.com");
    let f = PermissionsPolicyDeepAnalyzer::new().analyze(&ctx(&page, Some(200), &[], None));
    assert!(f.iter().any(|f| f.code == "PERMPDEEP001"), "{f:?}");
}

#[test]
fn coi_deep_flags_missing_and_partial_isolation() {
    let page = page_at("https://example.com");
    let f = CrossOriginIsolationDeepAnalyzer::new().analyze(&ctx(&page, Some(200), &[], None));
    assert!(
        f.iter().any(|f| f.code == "COISODEEP001") && f.iter().any(|f| f.code == "COISODEEP002"),
        "both COEP and COOP missing: {f:?}"
    );

    // Partial isolation: both headers present but not the full require-corp/same-origin pair.
    let headers = vec![
        hdr("Cross-Origin-Embedder-Policy", "credentialless"),
        hdr("Cross-Origin-Opener-Policy", "same-origin-allow-popups"),
    ];
    let f = CrossOriginIsolationDeepAnalyzer::new().analyze(&ctx(&page, Some(200), &headers, None));
    assert!(
        f.iter().any(|f| f.code == "COISODEEP003"),
        "partial isolation must be reported: {f:?}"
    );
}

// === Legacy DNS / SRI / CORS family ===

#[test]
fn dns_rebinding_flags_public_ip_header() {
    let page = page_at("https://example.com");
    let headers = vec![
        hdr("Access-Control-Allow-Origin", "*"),
        hdr("Access-Control-Allow-Credentials", "true"),
    ];
    let f = DnsRebindingAnalyzer::new().analyze(&ctx(&page, Some(200), &headers, None));
    assert!(f.iter().any(|f| f.code == "DNSREBIND001"), "{f:?}");
}

#[test]
fn sri_flags_external_script_without_integrity() {
    let mut page = page_at("https://example.com");
    page.scripts = vec![ScriptInfo {
        src: Some("https://cdn.example.com/lib.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
    }];
    // The analyzer checks ctx.body for integrity attributes, so provide one.
    let body = r#"<script src="https://cdn.example.com/lib.js"></script>"#;
    let f = SubresourceIntegrityAnalyzer::new().analyze(&ctx(&page, Some(200), &[], Some(body)));
    assert!(f.iter().any(|f| f.code == "SRISCRIPT001"), "{f:?}");
}

#[test]
fn cors_misconfig_flags_wildcard_with_credentials() {
    let page = page_at("https://example.com/api/data");
    let headers = vec![
        hdr("Access-Control-Allow-Origin", "*"),
        hdr("Access-Control-Allow-Credentials", "true"),
    ];
    let f = CorsMisconfigurationAnalyzer::new().analyze(&ctx(&page, Some(200), &headers, None));
    assert!(f.iter().any(|f| f.code == "CORS001-MISCONFIG"), "{f:?}");
}

#[test]
fn cors_misconfig_accepts_specific_origin() {
    let page = page_at("https://example.com/api/data");
    let headers = vec![
        hdr("Access-Control-Allow-Origin", "https://app.example.com"),
        hdr("Access-Control-Allow-Credentials", "true"),
    ];
    let f = CorsMisconfigurationAnalyzer::new().analyze(&ctx(&page, Some(200), &headers, None));
    assert!(f.is_empty(), "specific origin with creds is valid: {f:?}");
}

// === Permissions-Policy V2 ===

#[test]
fn permissions_v2_flags_missing_header_and_unrestricted_features() {
    let page = page_at("https://example.com");
    let f = PermissionsPolicyAnalyzerV2::new().analyze(&ctx(&page, Some(200), &[], None));
    assert!(f.iter().any(|f| f.code == "PERM-V2001"), "{f:?}");

    let headers = vec![hdr("Permissions-Policy", "camera=(self)")];
    let f = PermissionsPolicyAnalyzerV2::new().analyze(&ctx(&page, Some(200), &headers, None));
    assert!(f.iter().any(|f| f.code == "PERM-V2002"), "{f:?}");
}

// === Mixed content (legacy) and tabindex (legacy) ===

#[test]
fn mixed_content_flags_http_asset_on_https_page() {
    let page = page_at("https://example.com");
    let body = r#"<script src="http://insecure.example.com/a.js"></script>"#;
    let f = MixedContentDetectionAnalyzer::new().analyze(&ctx(&page, Some(200), &[], Some(body)));
    assert!(
        !f.is_empty(),
        "http script on https page must be flagged: {f:?}"
    );
    assert!(f
        .iter()
        .all(|f| f.category == crate::types::IssueCategory::Security));
}

#[test]
fn tabindex_flags_positive_and_negative_usage() {
    let mut page = page_at("https://example.com");
    page.has_positive_tabindex = true;
    page.tabindex_negative_count = 2;
    let f = TabindexAnalyzer::new().analyze(&ctx(&page, Some(200), &[], None));
    assert!(f.iter().any(|f| f.code == "TABINDEX001"), "{f:?}");
    assert!(f.iter().any(|f| f.code == "TABINDEX002"), "{f:?}");
}
