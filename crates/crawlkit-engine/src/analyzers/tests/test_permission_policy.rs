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
fn test_perm_no_header() {
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &[]);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PERM001"));
}

#[test]
fn test_perm_camera_not_restricted() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=*".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PERM002"));
}

#[test]
fn test_perm_microphone_not_restricted() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "microphone=*".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "PERM002"));
}

#[test]
fn test_perm_camera_restricted() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=()".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "PERM002"));
    assert!(!findings.iter().any(|f| f.code == "PERM001"));
}

#[test]
fn test_perm_all_restricted() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=(), microphone=(), geolocation=()".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_perm_camera_self_restricted() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=(self)".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "PERM002"));
}

#[test]
fn test_perm_multiple_features_mixed() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=(), microphone=*, geolocation=()".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    // camera restricted, geolocation restricted, but microphone not
    let perm002: Vec<&Finding> = findings.iter().filter(|f| f.code == "PERM002").collect();
    assert_eq!(perm002.len(), 1);
}

#[test]
fn test_perm_lowercase_header_name() {
    let headers = vec![(
        "permissions-policy".to_string(),
        "camera=(), microphone=()".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_perm_no_issue_when_feature_not_mentioned() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "geolocation=()".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "PERM002"));
}

#[test]
fn test_perm_empty_policy_value() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    // Empty value means no header effectively, but technically present
    assert!(!findings.iter().any(|f| f.code == "PERM001"));
    assert!(!findings.iter().any(|f| f.code == "PERM002"));
}

#[test]
fn test_perm_only_microphone_unrestricted() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=(), microphone=*, geolocation=(), gyroscope=()".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    let perm002: Vec<&Finding> = findings.iter().filter(|f| f.code == "PERM002").collect();
    assert_eq!(perm002.len(), 1);
    assert!(perm002[0].description.contains("microphone"));
}

#[test]
fn test_perm_both_unrestricted() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=*, microphone=*".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    let perm002: Vec<&Finding> = findings.iter().filter(|f| f.code == "PERM002").collect();
    assert_eq!(perm002.len(), 2);
}

#[test]
fn test_perm_multiple_restricted_features() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=(), microphone=(), geolocation=(), gyroscope=()".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_perm_with_other_directives() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "accelerometer=(), camera=(), geolocation=(), gyroscope=(), \
         magnetometer=(), microphone=(), payment=(), usb=()"
            .to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_perm_camera_with_self_and_unrestricted() {
    let headers = vec![(
        "Permissions-Policy".to_string(),
        "camera=(self), microphone=*".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = make_ctx_with_headers(&page, &headers);
    let findings = PermissionPolicyAnalyzer::new().analyze(&ctx);
    let perm002: Vec<&Finding> = findings.iter().filter(|f| f.code == "PERM002").collect();
    assert_eq!(perm002.len(), 1);
    assert!(perm002[0].description.contains("microphone"));
}
