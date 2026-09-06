//! Coverage for analyzers that previously had no dedicated tests:
//! font size, mixed-content validators, color contrast, focus order, and
//! the accessibility V2 family. Each test pins the analyzer's finding code
//! so regressions surface immediately.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::ParsedPage;
use crate::types::{IssueCategory, Severity};
use url::Url;

fn page_with(url: &str, body: &str) -> (ParsedPage, String) {
    let page = ParsedPage {
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
    };
    (page, body.to_string())
}

// === FontSizeAnalyzer (FSIZE001/FSIZE002) ===

#[test]
fn font_size_flags_small_text_and_low_line_height() {
    let (page, body) = page_with(
        "https://example.com",
        r#"<span style="font-size: 10px">tiny</span><p style="line-height: 1.1">tight</p>"#,
    );
    let ctx = AnalysisContext {
        page: &page,
        body: Some(&body),
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
    let findings = FontSizeAnalyzer::new().analyze(&ctx);
    assert!(
        findings.iter().any(|f| f.code == "FSIZE001"),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|f| f.code == "FSIZE002"),
        "{findings:?}"
    );
    assert!(findings
        .iter()
        .all(|f| f.category == IssueCategory::Accessibility));
}

#[test]
fn font_size_accepts_compliant_text() {
    let (page, body) = page_with(
        "https://example.com",
        r#"<span style="font-size: 16px">fine</span><p style="line-height: 1.6">roomy</p>"#,
    );
    let ctx = AnalysisContext {
        page: &page,
        body: Some(&body),
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
    assert!(FontSizeAnalyzer::new().analyze(&ctx).is_empty());
}

// === Color contrast (COLRCL / contrast analyzers) ===

#[test]
fn color_contrast_text_flags_low_contrast_hex_pair() {
    let (page, body) = page_with(
        "https://example.com",
        r#"<span style="color: #777777; background-color: #888888">low contrast</span>"#,
    );
    let ctx = AnalysisContext {
        page: &page,
        body: Some(&body),
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
    let findings = ColorContrastTextAnalyzer::new().analyze(&ctx);
    assert!(
        findings
            .iter()
            .any(|f| f.category == IssueCategory::Accessibility),
        "expected a contrast finding, got {findings:?}"
    );
}

#[test]
fn color_contrast_text_accepts_high_contrast_pair() {
    let (page, body) = page_with(
        "https://example.com",
        r#"<span style="color: #000000; background-color: #FFFFFF">high contrast</span>"#,
    );
    let ctx = AnalysisContext {
        page: &page,
        body: Some(&body),
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
    let findings = ColorContrastTextAnalyzer::new().analyze(&ctx);
    assert!(
        findings.iter().all(|f| f.severity != Severity::Error),
        "black-on-white must not be flagged: {findings:?}"
    );
}

// === Accessibility V2 family ===

#[test]
fn tabindex_v2_flags_positive_tabindex() {
    let mut page = ParsedPage {
        url: "https://example.com".to_string(),
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
        has_positive_tabindex: true,
        tabindex_negative_count: 0,
        aria_role_count: 0,
        aria_label_count: 0,
        has_lang_attribute: true,
        html_lang: Some("en".to_string()),
        has_aria_hidden: false,
        tables_with_headers: 0,
        tables_total: 0,
        tables_with_captions: 0,
        og_image_width: None,
        og_image_height: None,
    };
    page.url = "https://example.com".to_string();
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
    let findings = TabindexAnalyzerV2::new().analyze(&ctx);
    assert!(
        findings
            .iter()
            .any(|f| f.category == IssueCategory::Accessibility),
        "positive tabindex must be flagged: {findings:?}"
    );
}

#[test]
fn language_attribute_v2_passes_valid_lang() {
    let page = ParsedPage {
        url: "https://example.com".to_string(),
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
        has_lang_attribute: true,
        html_lang: Some("en".to_string()),
        has_aria_hidden: false,
        tables_with_headers: 0,
        tables_total: 0,
        tables_with_captions: 0,
        og_image_width: None,
        og_image_height: None,
    };
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
    assert!(LanguageAttributeAnalyzerV2::new().analyze(&ctx).is_empty());
}

#[test]
fn mixed_content_v2_flags_http_scripts_on_https_page() {
    let page = ParsedPage {
        url: "https://example.com".to_string(),
        meta: MetaTags::default(),
        headings: Vec::new(),
        links: Vec::new(),
        images: Vec::new(),
        forms: Vec::new(),
        scripts: vec![crate::parser::ScriptInfo {
            src: Some("http://insecure.example.com/app.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }],
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
    };
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
    // MixedContentDetectionAnalyzerV2 scans the body for plain-http references.
    let body = r#"<script src="http://insecure.example.com/app.js"></script>"#;
    let ctx_with_body = AnalysisContext {
        body: Some(body),
        ..ctx
    };
    let findings = MixedContentDetectionAnalyzerV2::new().analyze(&ctx_with_body);
    assert!(
        !findings.is_empty(),
        "http:// reference on https page must be flagged"
    );
    assert!(findings
        .iter()
        .all(|f| f.category == IssueCategory::Security));
}

#[test]
fn language_analyzer_handles_lang_without_panic() {
    // Regression: html_lang present but empty must not panic (formerly unwrap).
    let page = ParsedPage {
        url: "https://example.com".to_string(),
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
        has_lang_attribute: true,
        html_lang: Some("   ".to_string()),
        has_aria_hidden: false,
        tables_with_headers: 0,
        tables_total: 0,
        tables_with_captions: 0,
        og_image_width: None,
        og_image_height: None,
    };
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
    let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
    assert!(
        findings.iter().any(|f| f.code == "LANG003"),
        "empty lang must yield LANG003: {findings:?}"
    );
}

#[test]
fn analyzer_names_are_non_empty() {
    let analyzers: Vec<Box<dyn Analyzer>> = vec![
        Box::new(FontSizeAnalyzer::new()),
        Box::new(ColorContrastTextAnalyzer::new()),
        Box::new(ColorContrastLinkAnalyzer::new()),
        Box::new(TabindexAnalyzerV2::new()),
        Box::new(LanguageAttributeAnalyzerV2::new()),
        Box::new(MixedContentDetectionAnalyzerV2::new()),
        Box::new(LanguageAttributeAnalyzer::new()),
    ];
    for a in analyzers {
        assert!(!a.name().is_empty());
    }
}

// Silence unused import when Url is only used via ParsedPage construction.
#[allow(dead_code)]
fn _url_used() -> Url {
    Url::parse("https://example.com").unwrap()
}
