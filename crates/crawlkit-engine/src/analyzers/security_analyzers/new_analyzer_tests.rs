use super::*;
use crate::analyzers::{
    CertificateTransparencyAnalyzer, CookieHttpOnlyFlagValidator, CookieSecureFlagValidator,
    ExpectCTAnalyzer, FeaturePolicyAnalyzer,
};
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

fn make_ctx<'a>(
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

// DnsRebindingAnalyzer tests

#[test]
fn test_dns_rebinding_no_cors() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(DnsRebindingAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_dns_rebinding_wildcard_with_creds() {
    let headers = vec![
        ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
        (
            "Access-Control-Allow-Credentials".to_string(),
            "true".to_string(),
        ),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(DnsRebindingAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "DNSREBIND001"));
}

#[test]
fn test_dns_rebinding_wildcard_without_creds() {
    let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(DnsRebindingAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_dns_rebinding_wildcard_with_local_refs() {
    let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
    let page = make_page("https://example.com");
    let body = "Connect to 127.0.0.1 for local access";
    let ctx = make_ctx(&page, Some(200), &headers, Some(body));
    assert!(DnsRebindingAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "DNSREBIND002"));
}

#[test]
fn test_dns_rebinding_name() {
    assert_eq!(DnsRebindingAnalyzer::new().name(), "dns-rebinding");
}

#[test]
fn test_dns_rebinding_default() {
    let _ = DnsRebindingAnalyzer::default();
}

#[test]
fn test_dns_rebinding_specific_origin_no_finding() {
    let headers = vec![(
        "Access-Control-Allow-Origin".to_string(),
        "https://other.com".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(DnsRebindingAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_dns_rebinding_body_with_localhost() {
    let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
    let page = make_page("https://example.com");
    let body = "Visit localhost:8080 for admin panel";
    let ctx = make_ctx(&page, Some(200), &headers, Some(body));
    assert!(DnsRebindingAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "DNSREBIND002"));
}

#[test]
fn test_dns_rebinding_body_with_192_168() {
    let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
    let page = make_page("https://example.com");
    let body = "Connect to 192.168.1.1 for local access";
    let ctx = make_ctx(&page, Some(200), &headers, Some(body));
    assert!(DnsRebindingAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "DNSREBIND002"));
}

#[test]
fn test_dns_rebinding_body_no_local_refs() {
    let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
    let page = make_page("https://example.com");
    let body = "This is a normal page with no local IPs";
    let ctx = make_ctx(&page, Some(200), &headers, Some(body));
    assert!(DnsRebindingAnalyzer::new().analyze(&ctx).is_empty());
}

// SubresourceIntegrityAnalyzer tests

#[test]
fn test_sri_no_body() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_sri_name() {
    assert_eq!(
        SubresourceIntegrityAnalyzer::new().name(),
        "subresource-integrity"
    );
}

#[test]
fn test_sri_default() {
    let _ = SubresourceIntegrityAnalyzer::default();
}

#[test]
fn test_sri_external_script_without_sri() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![crate::parser::ScriptInfo {
        src: Some("https://cdn.example.com/lib.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
    }];
    let body =
        r#"<html><head><script src="https://cdn.example.com/lib.js"></script></head></html>"#;
    let ctx = make_ctx(&page, Some(200), &[], Some(body));
    assert!(SubresourceIntegrityAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "SRISCRIPT001"));
}

#[test]
fn test_sri_external_script_with_sri() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![crate::parser::ScriptInfo {
        src: Some("https://cdn.example.com/lib.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: true,
    }];
    let body = r#"<html><head><script src="https://cdn.example.com/lib.js" integrity="sha384-abc" crossorigin="anonymous"></script></head></html>"#;
    let ctx = make_ctx(&page, Some(200), &[], Some(body));
    assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_sri_internal_script_no_finding() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![crate::parser::ScriptInfo {
        src: Some("/app.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
    }];
    let body = r#"<html><head><script src="/app.js"></script></head></html>"#;
    let ctx = make_ctx(&page, Some(200), &[], Some(body));
    assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_sri_no_scripts() {
    let page = make_page("https://example.com");
    let body = "<html><head></head></html>";
    let ctx = make_ctx(&page, Some(200), &[], Some(body));
    assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_sri_multiple_external_without_sri() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![
        crate::parser::ScriptInfo {
            src: Some("https://cdn.example.com/a.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        },
        crate::parser::ScriptInfo {
            src: Some("https://cdn.example.com/b.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        },
    ];
    let body = r#"<html><head><script src="https://cdn.example.com/a.js"></script><script src="https://cdn.example.com/b.js"></script></head></html>"#;
    let ctx = make_ctx(&page, Some(200), &[], Some(body));
    let findings = SubresourceIntegrityAnalyzer::new().analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].description.contains("2"));
}

#[test]
fn test_sri_mixed_sri_and_no_sri() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![crate::parser::ScriptInfo {
        src: Some("https://cdn.example.com/bad.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
    }];
    let body =
        r#"<html><head><script src="https://cdn.example.com/bad.js"></script></head></html>"#;
    let ctx = make_ctx(&page, Some(200), &[], Some(body));
    assert!(SubresourceIntegrityAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "SRISCRIPT001"));
}

#[test]
fn test_sri_protocol_relative_url() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![crate::parser::ScriptInfo {
        src: Some("//cdn.example.com/lib.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
    }];
    let body = r#"<html><head><script src="//cdn.example.com/lib.js"></script></head></html>"#;
    let ctx = make_ctx(&page, Some(200), &[], Some(body));
    assert!(SubresourceIntegrityAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "SRISCRIPT001"));
}

#[test]
fn test_sri_no_script_src() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![crate::parser::ScriptInfo {
        src: None,
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
    }];
    let body = "<html><head><script>console.log('hi')</script></head></html>";
    let ctx = make_ctx(&page, Some(200), &[], Some(body));
    assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
}

// FeaturePolicyAnalyzer tests

#[test]
fn test_feature_policy_no_headers() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(FeaturePolicyAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "FP001"));
}

#[test]
fn test_feature_policy_has_feature_policy() {
    let headers = vec![("Feature-Policy".to_string(), "camera 'none'".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_feature_policy_has_permissions_policy() {
    let headers = vec![("Permissions-Policy".to_string(), "camera=()".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_feature_policy_case_insensitive() {
    let headers = vec![("permissions-policy".to_string(), "camera=()".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_feature_policy_empty_value() {
    let headers = vec![("Permissions-Policy".to_string(), "".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    // Header exists even if empty — analyzer only checks presence
    assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_feature_policy_name() {
    assert_eq!(FeaturePolicyAnalyzer::new().name(), "feature-policy");
}

#[test]
fn test_feature_policy_default() {
    let _ = FeaturePolicyAnalyzer::default();
}

#[test]
fn test_feature_policy_404_still_checks() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(404), &[], None);
    assert!(FeaturePolicyAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "FP001"));
}

#[test]
fn test_feature_policy_both_headers() {
    let headers = vec![
        ("Feature-Policy".to_string(), "camera 'none'".to_string()),
        ("Permissions-Policy".to_string(), "camera=()".to_string()),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_feature_policy_info_severity() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = FeaturePolicyAnalyzer::new().analyze(&ctx);
    assert_eq!(findings[0].severity, Severity::Info);
}

// ExpectCTAnalyzer tests

#[test]
fn test_expect_ct_no_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(ExpectCTAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "ECT001"));
}

#[test]
fn test_expect_ct_has_header() {
    let headers = vec![(
        "Expect-CT".to_string(),
        "max-age=86400, enforce".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_expect_ct_non_200_skipped() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(301), &[], None);
    assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_expect_ct_case_insensitive() {
    let headers = vec![("expect-ct".to_string(), "max-age=86400".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_expect_ct_name() {
    assert_eq!(ExpectCTAnalyzer::new().name(), "expect-ct");
}

#[test]
fn test_expect_ct_default() {
    let _ = ExpectCTAnalyzer::default();
}

#[test]
fn test_expect_ct_404_no_finding() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(404), &[], None);
    assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_expect_ct_info_severity() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = ExpectCTAnalyzer::new().analyze(&ctx);
    assert_eq!(findings[0].severity, Severity::Info);
}

#[test]
fn test_expect_ct_200_with_empty_header() {
    let headers = vec![("Expect-CT".to_string(), "".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_expect_ct_500_no_finding() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(500), &[], None);
    assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
}

// CertificateTransparencyAnalyzer tests

#[test]
fn test_ct_no_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(CertificateTransparencyAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "CT001"));
}

#[test]
fn test_ct_has_enforce() {
    let headers = vec![(
        "Expect-CT".to_string(),
        "max-age=86400, enforce".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CertificateTransparencyAnalyzer::new()
        .analyze(&ctx)
        .is_empty());
}

#[test]
fn test_ct_report_only() {
    let headers = vec![(
        "Expect-CT".to_string(),
        "max-age=86400, enforce".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CertificateTransparencyAnalyzer::new()
        .analyze(&ctx)
        .is_empty());
}

#[test]
fn test_ct_non_200_skipped() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(301), &[], None);
    assert!(CertificateTransparencyAnalyzer::new()
        .analyze(&ctx)
        .is_empty());
}

#[test]
fn test_ct_name() {
    assert_eq!(
        CertificateTransparencyAnalyzer::new().name(),
        "certificate-transparency"
    );
}

#[test]
fn test_ct_default() {
    let _ = CertificateTransparencyAnalyzer::default();
}

#[test]
fn test_ct_404_no_finding() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(404), &[], None);
    assert!(CertificateTransparencyAnalyzer::new()
        .analyze(&ctx)
        .is_empty());
}

#[test]
fn test_ct_info_severity() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = CertificateTransparencyAnalyzer::new().analyze(&ctx);
    assert_eq!(findings[0].severity, Severity::Info);
}

#[test]
fn test_ct_header_without_enforce() {
    let headers = vec![("Expect-CT".to_string(), "max-age=0".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    // Has Expect-CT header but without "enforce" — CT analyzer requires enforce
    assert!(CertificateTransparencyAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "CT001"));
}

#[test]
fn test_ct_500_no_finding() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(500), &[], None);
    assert!(CertificateTransparencyAnalyzer::new()
        .analyze(&ctx)
        .is_empty());
}

// CorsMisconfigurationAnalyzer tests

#[test]
fn test_cors_no_headers() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(CorsMisconfigurationAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_cors_wildcard_with_creds() {
    let headers = vec![
        ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
        (
            "Access-Control-Allow-Credentials".to_string(),
            "true".to_string(),
        ),
    ];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CorsMisconfigurationAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "CORS001-MISCONFIG"));
}

#[test]
fn test_cors_wildcard_on_sensitive() {
    let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
    let page = make_page("https://example.com/api/data");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CorsMisconfigurationAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "CORS002-MISCONFIG"));
}

#[test]
fn test_cors_wildcard_on_non_sensitive() {
    let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CorsMisconfigurationAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_cors_specific_origin() {
    let headers = vec![(
        "Access-Control-Allow-Origin".to_string(),
        "https://other.com".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CorsMisconfigurationAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_cors_name() {
    assert_eq!(
        CorsMisconfigurationAnalyzer::new().name(),
        "cors-misconfiguration"
    );
}

#[test]
fn test_cors_default() {
    let _ = CorsMisconfigurationAnalyzer::default();
}

// AriaLabelAnalyzer tests

#[test]
fn test_aria_label_no_roles() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_aria_label_roles_with_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 3;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_aria_label_roles_without_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaLabelAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "ARIALABEL001"));
}

#[test]
fn test_aria_label_name() {
    assert_eq!(AriaLabelAnalyzer::new().name(), "aria-label");
}

#[test]
fn test_aria_label_default() {
    let _ = AriaLabelAnalyzer::default();
}

#[test]
fn test_aria_label_one_role_one_label() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 1;
    page.aria_label_count = 1;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_aria_label_multiple_roles_fewer_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 5;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaLabelAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "ARIALABEL001"));
}

#[test]
fn test_aria_label_more_labels_than_roles() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 2;
    page.aria_label_count = 5;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_aria_label_equal_counts() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 3;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_aria_label_warning_severity() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 2;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = AriaLabelAnalyzer::new().analyze(&ctx);
    assert_eq!(findings[0].severity, Severity::Warning);
}

// TableCaptionAnalyzer tests

#[test]
fn test_table_caption_no_tables() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_table_caption_all_have_captions() {
    let mut page = make_page("https://example.com");
    page.tables_total = 3;
    page.tables_with_captions = 3;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_table_caption_missing() {
    let mut page = make_page("https://example.com");
    page.tables_total = 3;
    page.tables_with_captions = 1;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "TABLECAP001"));
}

#[test]
fn test_table_caption_name() {
    assert_eq!(TableCaptionAnalyzer::new().name(), "table-caption");
}

#[test]
fn test_table_caption_default() {
    let _ = TableCaptionAnalyzer::default();
}

#[test]
fn test_table_caption_one_table_with_caption() {
    let mut page = make_page("https://example.com");
    page.tables_total = 1;
    page.tables_with_captions = 1;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_table_caption_one_table_no_caption() {
    let mut page = make_page("https://example.com");
    page.tables_total = 1;
    page.tables_with_captions = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "TABLECAP001"));
}

#[test]
fn test_table_caption_all_missing() {
    let mut page = make_page("https://example.com");
    page.tables_total = 5;
    page.tables_with_captions = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "TABLECAP001"));
}

#[test]
fn test_table_caption_half_have_captions() {
    let mut page = make_page("https://example.com");
    page.tables_total = 4;
    page.tables_with_captions = 2;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "TABLECAP001"));
}

#[test]
fn test_table_caption_warning_severity() {
    let mut page = make_page("https://example.com");
    page.tables_total = 2;
    page.tables_with_captions = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TableCaptionAnalyzer::new().analyze(&ctx);
    assert_eq!(findings[0].severity, Severity::Info);
}

// SkipLinkAnalyzer tests

#[test]
fn test_skip_link_no_nav() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(SkipLinkAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_skip_link_has_nav_with_skip() {
    let mut page = make_page("https://example.com");
    page.has_nav_landmark = true;
    page.has_skip_link = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(SkipLinkAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_skip_link_has_nav_without_skip() {
    let mut page = make_page("https://example.com");
    page.has_nav_landmark = true;
    page.has_skip_link = false;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(SkipLinkAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "SKIPLINK001"));
}

#[test]
fn test_skip_link_name() {
    assert_eq!(SkipLinkAnalyzer::new().name(), "skip-link");
}

#[test]
fn test_skip_link_default() {
    let _ = SkipLinkAnalyzer::default();
}

#[test]
fn test_skip_link_main_landmark_with_skip() {
    let mut page = make_page("https://example.com");
    page.has_main_landmark = true;
    page.has_skip_link = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(SkipLinkAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_skip_link_main_landmark_without_skip() {
    let mut page = make_page("https://example.com");
    page.has_nav_landmark = true;
    page.has_main_landmark = false;
    page.has_skip_link = false;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(SkipLinkAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "SKIPLINK001"));
}

#[test]
fn test_skip_link_no_landmarks_no_finding() {
    let mut page = make_page("https://example.com");
    page.has_nav_landmark = false;
    page.has_main_landmark = false;
    page.has_skip_link = false;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(SkipLinkAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_skip_link_warning_severity() {
    let mut page = make_page("https://example.com");
    page.has_nav_landmark = true;
    page.has_skip_link = false;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = SkipLinkAnalyzer::new().analyze(&ctx);
    assert_eq!(findings[0].severity, Severity::Warning);
}

#[test]
fn test_skip_link_both_nav_and_main_without_skip() {
    let mut page = make_page("https://example.com");
    page.has_nav_landmark = true;
    page.has_main_landmark = true;
    page.has_skip_link = false;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(SkipLinkAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "SKIPLINK001"));
}

// TabindexAnalyzer tests

#[test]
fn test_tabindex_no_positive() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TabindexAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_tabindex_positive() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TabindexAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "TABINDEX001"));
}

#[test]
fn test_tabindex_negative() {
    let mut page = make_page("https://example.com");
    page.tabindex_negative_count = 5;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TabindexAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "TABINDEX002"));
}

#[test]
fn test_tabindex_both() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    page.tabindex_negative_count = 2;
    let ctx = make_ctx(&page, Some(200), &[], None);
    let findings = TabindexAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "TABINDEX001"));
    assert!(findings.iter().any(|f| f.code == "TABINDEX002"));
}

#[test]
fn test_tabindex_name() {
    assert_eq!(TabindexAnalyzer::new().name(), "tabindex");
}

#[test]
fn test_tabindex_default() {
    let _ = TabindexAnalyzer::default();
}

// PermissionsPolicyAnalyzerV2 tests

#[test]
fn test_perm_v2_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(PermissionsPolicyAnalyzerV2::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "PERM-V2001"));
}

#[test]
fn test_perm_v2_present() {
    let page = make_page("https://example.com");
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=(), microphone=(), geolocation=(), payment=()".to_string(),
    )];
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(PermissionsPolicyAnalyzerV2::new().analyze(&ctx).is_empty());
}

// FormInputLabelAnalyzer tests

#[test]
fn test_form_input_label_no_forms() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(FormInputLabelAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_form_input_label_with_label() {
    let mut page = make_page("https://example.com");
    page.forms = vec![crate::parser::ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![crate::parser::ExtractedInput {
            input_type: Some("text".to_string()),
            name: Some("email".to_string()),
            id: None,
            has_label: true,
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
    assert!(FormInputLabelAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_form_input_label_missing_label() {
    let mut page = make_page("https://example.com");
    page.forms = vec![crate::parser::ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![crate::parser::ExtractedInput {
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
    assert!(FormInputLabelAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "FILABEL001"));
}

// LinkTextAnalyzer tests

#[test]
fn test_link_text_no_links() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LinkTextAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_link_text_empty() {
    let mut page = make_page("https://example.com");
    page.links = vec![crate::parser::ExtractedLink {
        href: "https://example.com/target".to_string(),
        text: "".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LinkTextAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "LINKTEXT001"));
}

#[test]
fn test_link_text_generic() {
    let mut page = make_page("https://example.com");
    page.links = vec![crate::parser::ExtractedLink {
        href: "https://example.com/target".to_string(),
        text: "click here".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LinkTextAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "LINKTEXT002"));
}

#[test]
fn test_link_text_good() {
    let mut page = make_page("https://example.com");
    page.links = vec![crate::parser::ExtractedLink {
        href: "https://example.com/target".to_string(),
        text: "About our company".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LinkTextAnalyzer::new().analyze(&ctx).is_empty());
}

// ImageAltTextAnalyzer tests

#[test]
fn test_image_alt_no_images() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(ImageAltTextAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_image_alt_missing() {
    let mut page = make_page("https://example.com");
    page.images = vec![crate::parser::ExtractedImage {
        src: "https://example.com/photo.jpg".to_string(),
        alt: String::new(),
        width: None,
        height: None,
        has_alt: false,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(ImageAltTextAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "IMGALT001"));
}

#[test]
fn test_image_alt_empty() {
    let mut page = make_page("https://example.com");
    page.images = vec![crate::parser::ExtractedImage {
        src: "https://example.com/photo.jpg".to_string(),
        alt: String::new(),
        width: None,
        height: None,
        has_alt: false,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(ImageAltTextAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "IMGALT001"));
}

#[test]
fn test_image_alt_present() {
    let mut page = make_page("https://example.com");
    page.images = vec![crate::parser::ExtractedImage {
        src: "https://example.com/photo.jpg".to_string(),
        alt: "A scenic mountain view".to_string(),
        width: None,
        height: None,
        has_alt: true,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(ImageAltTextAnalyzer::new().analyze(&ctx).is_empty());
}

// AriaRoleAnalyzer tests

#[test]
fn test_aria_role_no_roles() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaRoleAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_aria_role_without_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaRoleAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "ARIAROLE001"));
}

#[test]
fn test_aria_role_with_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 3;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaRoleAnalyzer::new().analyze(&ctx).is_empty());
}

// StrictTransportSecurityAnalyzerV2 tests

#[test]
fn test_hsts_v2_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(StrictTransportSecurityAnalyzerV2::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "HSTS-V2001"));
}

#[test]
fn test_hsts_v2_low_max_age() {
    let page = make_page("https://example.com");
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=3600".to_string(),
    )];
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(StrictTransportSecurityAnalyzerV2::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "HSTS-V2002"));
}

#[test]
fn test_hsts_v2_no_include_subdomains() {
    let page = make_page("https://example.com");
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000".to_string(),
    )];
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = StrictTransportSecurityAnalyzerV2::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HSTS-V2003"));
}

#[test]
fn test_hsts_v2_valid() {
    let page = make_page("https://example.com");
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000; includeSubDomains; preload".to_string(),
    )];
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(StrictTransportSecurityAnalyzerV2::new()
        .analyze(&ctx)
        .is_empty());
}

// XssProtectionAnalyzerV2 tests

#[test]
fn test_xss_v2_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(XssProtectionAnalyzerV2::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "XSS-V2001"));
}

#[test]
fn test_xss_v2_disabled() {
    let page = make_page("https://example.com");
    let headers = vec![("X-XSS-Protection".to_string(), "0".to_string())];
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(XssProtectionAnalyzerV2::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "XSS-V2002"));
}

// ContentTypeSniffingAnalyzerV2 tests

#[test]
fn test_ct_v2_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(ContentTypeSniffingAnalyzerV2::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "CT-V2001"));
}

#[test]
fn test_ct_v2_invalid_value() {
    let page = make_page("https://example.com");
    let headers = vec![("X-Content-Type-Options".to_string(), "sniff".to_string())];
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(ContentTypeSniffingAnalyzerV2::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "CT-V2002"));
}

#[test]
fn test_ct_v2_valid() {
    let page = make_page("https://example.com");
    let headers = vec![("X-Content-Type-Options".to_string(), "nosniff".to_string())];
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(ContentTypeSniffingAnalyzerV2::new()
        .analyze(&ctx)
        .is_empty());
}

// === HstsPreloadListValidator tests ===

#[test]
fn test_hsts_preload_valid() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000; includeSubDomains; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(HstsPreloadListValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_hsts_preload_no_preload() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=31536000; includeSubDomains".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(HstsPreloadListValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_hsts_preload_low_max_age() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=300; includeSubDomains; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let f = HstsPreloadListValidator::new().analyze(&ctx);
    assert!(f.iter().any(|f| f.code == "HSTSPRELOAD001"));
}

#[test]
fn test_hsts_preload_no_isd() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=63072000; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(HstsPreloadListValidator::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "HSTSPRELOAD001"));
}

#[test]
fn test_hsts_preload_no_hsts() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(HstsPreloadListValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_hsts_preload_name() {
    assert_eq!(
        HstsPreloadListValidator::new().name(),
        "hsts-preload-list-validator"
    );
}

#[test]
fn test_hsts_preload_default() {
    let _ = HstsPreloadListValidator::default();
}

#[test]
fn test_hsts_preload_category() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=300; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let findings = HstsPreloadListValidator::new().analyze(&ctx);
    for f in &findings {
        assert_eq!(f.category, IssueCategory::Security);
    }
}

#[test]
fn test_hsts_preload_severity() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=300; preload".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert_eq!(
        HstsPreloadListValidator::new().analyze(&ctx)[0].severity,
        Severity::Warning
    );
}

// === CspDirectiveValidator tests ===

#[test]
fn test_csp_dir_missing_default_src() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "script-src 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let f = CspDirectiveValidator::new().analyze(&ctx);
    assert!(f.iter().any(|f| f.code == "CSPDIR001-VALIDATOR"));
}

#[test]
fn test_csp_dir_all_present() {
    let headers = vec![("Content-Security-Policy".to_string(), "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CspDirectiveValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_csp_dir_no_csp() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(CspDirectiveValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_csp_dir_name() {
    assert_eq!(
        CspDirectiveValidator::new().name(),
        "csp-directive-validator"
    );
}

#[test]
fn test_csp_dir_default() {
    let _ = CspDirectiveValidator::default();
}

#[test]
fn test_csp_dir_category() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "script-src 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    for f in CspDirectiveValidator::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Security);
    }
}

#[test]
fn test_csp_dir_multiple_missing() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "script-src 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let f = CspDirectiveValidator::new().analyze(&ctx);
    assert!(f.len() >= 3);
}

#[test]
fn test_csp_dir_severity() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "script-src 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert_eq!(
        CspDirectiveValidator::new().analyze(&ctx)[0].severity,
        Severity::Warning
    );
}

#[test]
fn test_csp_dir_empty_value() {
    let headers = vec![("Content-Security-Policy".to_string(), "".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    let f = CspDirectiveValidator::new().analyze(&ctx);
    assert!(!f.is_empty());
}

// === CookieSecureFlagValidator tests ===

#[test]
fn test_cookie_secure_missing() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session=abc; HttpOnly".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CookieSecureFlagValidator::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "COOKIESEC001-VALIDATOR"));
}

#[test]
fn test_cookie_secure_present() {
    let headers = vec![("Set-Cookie".to_string(), "session=abc; Secure".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CookieSecureFlagValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_cookie_secure_http_page_skipped() {
    let headers = vec![("Set-Cookie".to_string(), "session=abc".to_string())];
    let page = make_page("http://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CookieSecureFlagValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_cookie_secure_no_cookies() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(CookieSecureFlagValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_cookie_secure_name() {
    assert_eq!(
        CookieSecureFlagValidator::new().name(),
        "cookie-secure-flag"
    );
}

#[test]
fn test_cookie_secure_default() {
    let _ = CookieSecureFlagValidator::default();
}

#[test]
fn test_cookie_secure_category() {
    let headers = vec![("Set-Cookie".to_string(), "session=abc".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    for f in CookieSecureFlagValidator::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Security);
    }
}

#[test]
fn test_cookie_secure_severity() {
    let headers = vec![("Set-Cookie".to_string(), "session=abc".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert_eq!(
        CookieSecureFlagValidator::new().analyze(&ctx)[0].severity,
        Severity::Warning
    );
}

// === CookieHttpOnlyFlagValidator tests ===

#[test]
fn test_cookie_httponly_missing() {
    let headers = vec![("Set-Cookie".to_string(), "session=abc; Secure".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CookieHttpOnlyFlagValidator::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "COOKIEHTTP001"));
}

#[test]
fn test_cookie_httponly_present() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session=abc; HttpOnly".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CookieHttpOnlyFlagValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_cookie_httponly_both_flags() {
    let headers = vec![(
        "Set-Cookie".to_string(),
        "session=abc; Secure; HttpOnly".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    assert!(CookieHttpOnlyFlagValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_cookie_httponly_no_cookies() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(CookieHttpOnlyFlagValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_cookie_httponly_name() {
    assert_eq!(
        CookieHttpOnlyFlagValidator::new().name(),
        "cookie-httponly-flag"
    );
}

#[test]
fn test_cookie_httponly_default() {
    let _ = CookieHttpOnlyFlagValidator::default();
}

#[test]
fn test_cookie_httponly_category() {
    let headers = vec![("Set-Cookie".to_string(), "session=abc".to_string())];
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &headers, None);
    for f in CookieHttpOnlyFlagValidator::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Security);
    }
}

// === MixedContentFormValidator tests ===

#[test]
fn test_mixed_form_http_action() {
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
    assert!(MixedContentFormValidator::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "MIXFRM001"));
}

#[test]
fn test_mixed_form_https_action() {
    let body = r#"<form action="https://example.com/submit">"#;
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
    assert!(MixedContentFormValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_mixed_form_http_page_skipped() {
    let body = r#"<form action="http://example.com/submit">"#;
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
    assert!(MixedContentFormValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_mixed_form_no_body() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(MixedContentFormValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_mixed_form_name() {
    assert_eq!(
        MixedContentFormValidator::new().name(),
        "mixed-content-form"
    );
}

#[test]
fn test_mixed_form_default() {
    let _ = MixedContentFormValidator::default();
}

#[test]
fn test_mixed_form_category() {
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
    for f in MixedContentFormValidator::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Security);
    }
}

#[test]
fn test_mixed_form_severity() {
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
    assert_eq!(
        MixedContentFormValidator::new().analyze(&ctx)[0].severity,
        Severity::Error
    );
}

// === MixedContentScriptValidator tests ===

#[test]
fn test_mixed_script_http() {
    let body = r#"<script src="http://cdn.example.com/app.js"></script>"#;
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
    assert!(MixedContentScriptValidator::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "MIXSCR001"));
}

#[test]
fn test_mixed_script_https() {
    let body = r#"<script src="https://cdn.example.com/app.js"></script>"#;
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
    assert!(MixedContentScriptValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_mixed_script_http_page_skipped() {
    let body = r#"<script src="http://cdn.example.com/app.js"></script>"#;
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
    assert!(MixedContentScriptValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_mixed_script_no_body() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(MixedContentScriptValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_mixed_script_name() {
    assert_eq!(
        MixedContentScriptValidator::new().name(),
        "mixed-content-script"
    );
}

#[test]
fn test_mixed_script_default() {
    let _ = MixedContentScriptValidator::default();
}

#[test]
fn test_mixed_script_category() {
    let body = r#"<script src="http://cdn.example.com/app.js"></script>"#;
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
    for f in MixedContentScriptValidator::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Security);
    }
}

// === MixedContentImageValidator tests ===

#[test]
fn test_mixed_img_http() {
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
    assert!(MixedContentImageValidator::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "MIXIMG001"));
}

#[test]
fn test_mixed_img_https() {
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
    assert!(MixedContentImageValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_mixed_img_http_page_skipped() {
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
    assert!(MixedContentImageValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_mixed_img_no_body() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(MixedContentImageValidator::new().analyze(&ctx).is_empty());
}

#[test]
fn test_mixed_img_name() {
    assert_eq!(
        MixedContentImageValidator::new().name(),
        "mixed-content-image"
    );
}

#[test]
fn test_mixed_img_default() {
    let _ = MixedContentImageValidator::default();
}

#[test]
fn test_mixed_img_category() {
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
    for f in MixedContentImageValidator::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Security);
    }
}

// === LandmarkMainAnalyzer tests ===

#[test]
fn test_landmark_main_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LandmarkMainAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "LANDMAIN001"));
}

#[test]
fn test_landmark_main_present() {
    let mut page = make_page("https://example.com");
    page.has_main_landmark = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LandmarkMainAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_landmark_main_name() {
    assert_eq!(LandmarkMainAnalyzer::new().name(), "landmark-main");
}

#[test]
fn test_landmark_main_default() {
    let _ = LandmarkMainAnalyzer::default();
}

#[test]
fn test_landmark_main_category() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    for f in LandmarkMainAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_landmark_main_severity() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert_eq!(
        LandmarkMainAnalyzer::new().analyze(&ctx)[0].severity,
        Severity::Error
    );
}

// === LandmarkNavAnalyzer tests ===

#[test]
fn test_landmark_nav_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LandmarkNavAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "LANDNAV001"));
}

#[test]
fn test_landmark_nav_present() {
    let mut page = make_page("https://example.com");
    page.has_nav_landmark = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LandmarkNavAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_landmark_nav_name() {
    assert_eq!(LandmarkNavAnalyzer::new().name(), "landmark-nav");
}

#[test]
fn test_landmark_nav_default() {
    let _ = LandmarkNavAnalyzer::default();
}

#[test]
fn test_landmark_nav_category() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    for f in LandmarkNavAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

// === LandmarkBannerAnalyzer tests ===

#[test]
fn test_landmark_banner_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LandmarkBannerAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "LANDBAN001"));
}

#[test]
fn test_landmark_banner_present() {
    let mut page = make_page("https://example.com");
    page.landmarks.push("banner".to_string());
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LandmarkBannerAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_landmark_banner_header_role() {
    let mut page = make_page("https://example.com");
    page.landmarks.push("header".to_string());
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(LandmarkBannerAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_landmark_banner_name() {
    assert_eq!(LandmarkBannerAnalyzer::new().name(), "landmark-banner");
}

#[test]
fn test_landmark_banner_default() {
    let _ = LandmarkBannerAnalyzer::default();
}

#[test]
fn test_landmark_banner_category() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    for f in LandmarkBannerAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

// === HeadingLevelSkipAnalyzer tests ===

#[test]
fn test_heading_skip_h1_to_h3() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        crate::parser::Heading {
            level: 1,
            text: "H1".into(),
            length: 2,
        },
        crate::parser::Heading {
            level: 3,
            text: "H3".into(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(HeadingLevelSkipAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "HEADSKIP001"));
}

#[test]
fn test_heading_skip_no_skip() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        crate::parser::Heading {
            level: 1,
            text: "H1".into(),
            length: 2,
        },
        crate::parser::Heading {
            level: 2,
            text: "H2".into(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(HeadingLevelSkipAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_heading_skip_single() {
    let mut page = make_page("https://example.com");
    page.headings = vec![crate::parser::Heading {
        level: 1,
        text: "H1".into(),
        length: 2,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(HeadingLevelSkipAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_heading_skip_empty() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(HeadingLevelSkipAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_heading_skip_name() {
    assert_eq!(HeadingLevelSkipAnalyzer::new().name(), "heading-level-skip");
}

#[test]
fn test_heading_skip_default() {
    let _ = HeadingLevelSkipAnalyzer::default();
}

#[test]
fn test_heading_skip_category() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        crate::parser::Heading {
            level: 1,
            text: "H1".into(),
            length: 2,
        },
        crate::parser::Heading {
            level: 3,
            text: "H3".into(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200), &[], None);
    for f in HeadingLevelSkipAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

// === FormLabelAssociationAnalyzer tests ===

#[test]
fn test_form_label_assoc_missing() {
    let mut page = make_page("https://example.com");
    page.forms = vec![crate::parser::ExtractedForm {
        action: None,
        method: "post".into(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![crate::parser::ExtractedInput {
            input_type: Some("text".into()),
            name: Some("email".into()),
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
    assert!(FormLabelAssociationAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "FORMLAB001"));
}

#[test]
fn test_form_label_assoc_with_label() {
    let mut page = make_page("https://example.com");
    page.forms = vec![crate::parser::ExtractedForm {
        action: None,
        method: "post".into(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![crate::parser::ExtractedInput {
            input_type: Some("text".into()),
            name: Some("email".into()),
            id: None,
            has_label: true,
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
    assert!(FormLabelAssociationAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_form_label_assoc_with_aria() {
    let mut page = make_page("https://example.com");
    page.forms = vec![crate::parser::ExtractedForm {
        action: None,
        method: "post".into(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![crate::parser::ExtractedInput {
            input_type: Some("text".into()),
            name: Some("email".into()),
            id: None,
            has_label: false,
            aria_label: Some("Email".into()),
            aria_labelledby: None,
            aria_describedby: None,
            placeholder: None,
            required: false,
        }],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(FormLabelAssociationAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_form_label_assoc_no_forms() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(FormLabelAssociationAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_form_label_assoc_name() {
    assert_eq!(
        FormLabelAssociationAnalyzer::new().name(),
        "form-label-association"
    );
}

#[test]
fn test_form_label_assoc_default() {
    let _ = FormLabelAssociationAnalyzer::default();
}

#[test]
fn test_form_label_assoc_category() {
    let mut page = make_page("https://example.com");
    page.forms = vec![crate::parser::ExtractedForm {
        action: None,
        method: "post".into(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![crate::parser::ExtractedInput {
            input_type: Some("text".into()),
            name: Some("email".into()),
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
    for f in FormLabelAssociationAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

// === TableHeaderScopeAnalyzer tests ===

#[test]
fn test_tbl_scope_missing() {
    let mut page = make_page("https://example.com");
    page.tables_total = 3;
    page.tables_with_headers = 1;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableHeaderScopeAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "TBLSCOP001"));
}

#[test]
fn test_tbl_scope_all_have() {
    let mut page = make_page("https://example.com");
    page.tables_total = 3;
    page.tables_with_headers = 3;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableHeaderScopeAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_tbl_scope_no_tables() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableHeaderScopeAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_tbl_scope_name() {
    assert_eq!(TableHeaderScopeAnalyzer::new().name(), "table-header-scope");
}

#[test]
fn test_tbl_scope_default() {
    let _ = TableHeaderScopeAnalyzer::default();
}

#[test]
fn test_tbl_scope_category() {
    let mut page = make_page("https://example.com");
    page.tables_total = 1;
    page.tables_with_headers = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    for f in TableHeaderScopeAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

// === TableCaptionPresenceAnalyzer tests ===

#[test]
fn test_tbl_cap_missing() {
    let mut page = make_page("https://example.com");
    page.tables_total = 2;
    page.tables_with_captions = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionPresenceAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "TBLCAP001"));
}

#[test]
fn test_tbl_cap_all_have() {
    let mut page = make_page("https://example.com");
    page.tables_total = 3;
    page.tables_with_captions = 3;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionPresenceAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_tbl_cap_no_tables() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(TableCaptionPresenceAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_tbl_cap_name() {
    assert_eq!(
        TableCaptionPresenceAnalyzer::new().name(),
        "table-caption-presence"
    );
}

#[test]
fn test_tbl_cap_default() {
    let _ = TableCaptionPresenceAnalyzer::default();
}

#[test]
fn test_tbl_cap_category() {
    let mut page = make_page("https://example.com");
    page.tables_total = 1;
    page.tables_with_captions = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    for f in TableCaptionPresenceAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

// === AnchorTextGenericAnalyzer tests ===

#[test]
fn test_anch_gen_click_here() {
    let mut page = make_page("https://example.com");
    page.links = vec![crate::parser::ExtractedLink {
        href: "/page".into(),
        text: "click here".into(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AnchorTextGenericAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "ANCHGEN001"));
}

#[test]
fn test_anch_gen_read_more() {
    let mut page = make_page("https://example.com");
    page.links = vec![crate::parser::ExtractedLink {
        href: "/page".into(),
        text: "read more".into(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AnchorTextGenericAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "ANCHGEN001"));
}

#[test]
fn test_anch_gen_good_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![crate::parser::ExtractedLink {
        href: "/page".into(),
        text: "About our pricing".into(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AnchorTextGenericAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_anch_gen_empty_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![crate::parser::ExtractedLink {
        href: "/page".into(),
        text: "".into(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AnchorTextGenericAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_anch_gen_no_links() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AnchorTextGenericAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_anch_gen_name() {
    assert_eq!(
        AnchorTextGenericAnalyzer::new().name(),
        "anchor-text-generic"
    );
}

#[test]
fn test_anch_gen_default() {
    let _ = AnchorTextGenericAnalyzer::default();
}

#[test]
fn test_anch_gen_category() {
    let mut page = make_page("https://example.com");
    page.links = vec![crate::parser::ExtractedLink {
        href: "/page".into(),
        text: "click here".into(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200), &[], None);
    for f in AnchorTextGenericAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

// === AriaRequiredAttributesAnalyzer tests ===

#[test]
fn test_aria_req_roles_no_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaRequiredAttributesAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "ARIAREQ001"));
}

#[test]
fn test_aria_req_with_labels() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 3;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaRequiredAttributesAnalyzer::new()
        .analyze(&ctx)
        .is_empty());
}

#[test]
fn test_aria_req_no_roles() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(AriaRequiredAttributesAnalyzer::new()
        .analyze(&ctx)
        .is_empty());
}

#[test]
fn test_aria_req_name() {
    assert_eq!(
        AriaRequiredAttributesAnalyzer::new().name(),
        "aria-required-attributes"
    );
}

#[test]
fn test_aria_req_default() {
    let _ = AriaRequiredAttributesAnalyzer::default();
}

#[test]
fn test_aria_req_category() {
    let mut page = make_page("https://example.com");
    page.aria_role_count = 2;
    page.aria_label_count = 0;
    let ctx = make_ctx(&page, Some(200), &[], None);
    for f in AriaRequiredAttributesAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

// === FocusOrderPositiveTabindexAnalyzer tests ===

#[test]
fn test_tabpos_positive() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(FocusOrderPositiveTabindexAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "TABPOS001"));
}

#[test]
fn test_tabpos_no_positive() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(FocusOrderPositiveTabindexAnalyzer::new()
        .analyze(&ctx)
        .is_empty());
}

#[test]
fn test_tabpos_name() {
    assert_eq!(
        FocusOrderPositiveTabindexAnalyzer::new().name(),
        "focus-order-positive-tabindex"
    );
}

#[test]
fn test_tabpos_default() {
    let _ = FocusOrderPositiveTabindexAnalyzer::default();
}

#[test]
fn test_tabpos_category() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    for f in FocusOrderPositiveTabindexAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

#[test]
fn test_tabpos_severity() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert_eq!(
        FocusOrderPositiveTabindexAnalyzer::new().analyze(&ctx)[0].severity,
        Severity::Error
    );
}

// === ColorContrastTextAnalyzer tests ===

#[test]
fn test_colrct_no_body() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(ColorContrastTextAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_colrct_good_contrast() {
    let body = r#"<p style="color: #000000; background-color: #ffffff">Text</p>"#;
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
    assert!(ColorContrastTextAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_colrct_low_contrast() {
    let body = r#"<p style="color: #888888; background-color: #999999">Text</p>"#;
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
    assert!(ColorContrastTextAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "COLRCT001"));
}

#[test]
fn test_colrct_name() {
    assert_eq!(
        ColorContrastTextAnalyzer::new().name(),
        "color-contrast-text"
    );
}

#[test]
fn test_colrct_default() {
    let _ = ColorContrastTextAnalyzer::default();
}

#[test]
fn test_colrct_category() {
    let body = r#"<p style="color: #888888; background-color: #999999">Text</p>"#;
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
    for f in ColorContrastTextAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}

// === ColorContrastLinkAnalyzer tests ===

#[test]
fn test_colrcl_no_body() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200), &[], None);
    assert!(ColorContrastLinkAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_colrcl_good_contrast() {
    let body = r#"<a style="color: #0000ff; background-color: #ffffff">Link</a>"#;
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
    assert!(ColorContrastLinkAnalyzer::new().analyze(&ctx).is_empty());
}

#[test]
fn test_colrcl_low_contrast() {
    let body = r#"<a style="color: #cccccc; background-color: #dddddd">Link</a>"#;
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
    assert!(ColorContrastLinkAnalyzer::new()
        .analyze(&ctx)
        .iter()
        .any(|f| f.code == "COLRCL001"));
}

#[test]
fn test_colrcl_name() {
    assert_eq!(
        ColorContrastLinkAnalyzer::new().name(),
        "color-contrast-link"
    );
}

#[test]
fn test_colrcl_default() {
    let _ = ColorContrastLinkAnalyzer::default();
}

#[test]
fn test_colrcl_category() {
    let body = r#"<a style="color: #cccccc; background-color: #dddddd">Link</a>"#;
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
    for f in ColorContrastLinkAnalyzer::new().analyze(&ctx) {
        assert_eq!(f.category, IssueCategory::Accessibility);
    }
}
