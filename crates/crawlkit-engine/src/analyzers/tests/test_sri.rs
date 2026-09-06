use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::{ParsedPage, ScriptInfo, StyleInfo};

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

fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
    AnalysisContext {
        page,
        body: None,
        status_code: status,
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
fn test_sri_no_scripts_or_styles() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_sri_local_script_no_issue() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![ScriptInfo {
        src: Some("/app.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
        is_module: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SRI001"));
}

#[test]
fn test_sri_external_script_without_integrity() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![ScriptInfo {
        src: Some("https://cdn.example.com/lib.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
        is_module: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SRI001"));
}

#[test]
fn test_sri_external_script_with_integrity() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![ScriptInfo {
        src: Some("https://cdn.example.com/lib.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: true,
        is_module: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SRI001"));
}

#[test]
fn test_sri_external_stylesheet_without_integrity() {
    let mut page = make_page("https://example.com");
    page.styles = vec![StyleInfo {
        href: Some("https://cdn.example.com/style.css".to_string()),
        media: None,
        is_inline: false,
        has_integrity: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SRI002"));
}

#[test]
fn test_sri_external_stylesheet_with_integrity() {
    let mut page = make_page("https://example.com");
    page.styles = vec![StyleInfo {
        href: Some("https://cdn.example.com/style.css".to_string()),
        media: None,
        is_inline: false,
        has_integrity: true,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SRI002"));
}

#[test]
fn test_sri_inline_style_no_issue() {
    let mut page = make_page("https://example.com");
    page.styles = vec![StyleInfo {
        href: None,
        media: None,
        is_inline: true,
        has_integrity: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_sri_multiple_external_scripts_mixed() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![
        ScriptInfo {
            src: Some("https://cdn.example.com/a.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        },
        ScriptInfo {
            src: Some("https://cdn.example.com/b.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: true,
            is_module: false,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SRI001"));
    let finding = findings.iter().find(|f| f.code == "SRI001").unwrap();
    assert!(finding.description.contains("1 external script(s)"));
}

#[test]
fn test_sri_protocol_relative_url() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![ScriptInfo {
        src: Some("//cdn.example.com/lib.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
        is_module: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SRI001"));
}

#[test]
fn test_sri_both_scripts_and_styles() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![ScriptInfo {
        src: Some("https://cdn.example.com/app.js".to_string()),
        r#async: false,
        defer: false,
        script_type: None,
        has_integrity: false,
        is_module: false,
    }];
    page.styles = vec![StyleInfo {
        href: Some("https://cdn.example.com/style.css".to_string()),
        media: None,
        is_inline: false,
        has_integrity: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SRI001"));
    assert!(findings.iter().any(|f| f.code == "SRI002"));
}

#[test]
fn test_sri_no_cross_origin_scripts_all_local() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![
        ScriptInfo {
            src: Some("/js/a.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        },
        ScriptInfo {
            src: Some("/js/b.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SRI001"));
}

#[test]
fn test_sri_no_cross_origin_stylesheets() {
    let mut page = make_page("https://example.com");
    page.styles = vec![
        StyleInfo {
            href: Some("/css/a.css".to_string()),
            media: None,
            is_inline: false,
            has_integrity: false,
        },
        StyleInfo {
            href: None,
            media: None,
            is_inline: true,
            has_integrity: false,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SRI002"));
}

#[test]
fn test_sri_only_external_without_integrity_counted() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![
        ScriptInfo {
            src: Some("https://cdn.example.com/a.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        },
        ScriptInfo {
            src: Some("https://cdn.example.com/b.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        },
        ScriptInfo {
            src: Some("/local.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SRI001"));
    let finding = findings.iter().find(|f| f.code == "SRI001").unwrap();
    assert!(finding.description.contains("2 external script(s)"));
}

#[test]
fn test_sri_all_with_integrity_no_findings() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![ScriptInfo {
        src: Some("https://cdn.example.com/lib.js".to_string()),
        r#async: true,
        defer: false,
        script_type: None,
        has_integrity: true,
        is_module: false,
    }];
    page.styles = vec![StyleInfo {
        href: Some("https://cdn.example.com/style.css".to_string()),
        media: Some("screen".to_string()),
        is_inline: false,
        has_integrity: true,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_sri_only_local_no_findings() {
    let mut page = make_page("https://example.com");
    page.scripts = vec![ScriptInfo {
        src: Some("/app.js".to_string()),
        r#async: false,
        defer: true,
        script_type: None,
        has_integrity: false,
        is_module: false,
    }];
    page.styles = vec![StyleInfo {
        href: Some("/style.css".to_string()),
        media: None,
        is_inline: false,
        has_integrity: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = SriAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}
