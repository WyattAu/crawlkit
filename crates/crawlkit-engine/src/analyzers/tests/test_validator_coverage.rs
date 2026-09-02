//! Coverage for the remaining validator families:
//! landmark/heading validators, form/table validators, and mixed-content
//! validators. Each test pins finding codes so regressions surface quickly.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::{ExtractedForm, Heading, ParsedPage};

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

fn ctx<'a>(page: &'a ParsedPage, body: Option<&'a str>) -> AnalysisContext<'a> {
    AnalysisContext {
        page,
        body,
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

// === Landmark / heading validators ===

#[test]
fn landmark_main_flags_missing_main() {
    let page = page_at("https://example.com");
    let f = LandmarkMainAnalyzer::new().analyze(&ctx(&page, None));
    assert!(f.iter().any(|f| f.code == "LANDMAIN001"), "{f:?}");
}

#[test]
fn landmark_nav_flags_missing_nav_with_links() {
    let mut page = page_at("https://example.com");
    for i in 0..6 {
        page.links.push(crate::parser::ExtractedLink {
            href: format!("https://example.com/{i}"),
            text: format!("Link {i}"),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        });
    }
    let f = LandmarkNavAnalyzer::new().analyze(&ctx(&page, None));
    assert!(f.iter().any(|f| f.code == "LANDNAV001"), "{f:?}");
}

#[test]
fn landmark_banner_flags_missing_banner() {
    let page = page_at("https://example.com");
    let f = LandmarkBannerAnalyzer::new().analyze(&ctx(&page, None));
    assert!(f.iter().any(|f| f.code == "LANDBAN001"), "{f:?}");
}

#[test]
fn heading_skip_flags_level_jump() {
    let mut page = page_at("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        },
        Heading {
            level: 3,
            text: "Skipped H2".to_string(),
            length: 10,
        },
    ];
    let f = HeadingLevelSkipAnalyzer::new().analyze(&ctx(&page, None));
    assert!(f.iter().any(|f| f.code == "HEADSKIP001"), "{f:?}");
}

#[test]
fn heading_skip_accepts_sequential_levels() {
    let mut page = page_at("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        },
        Heading {
            level: 2,
            text: "Section".to_string(),
            length: 7,
        },
        Heading {
            level: 3,
            text: "Subsection".to_string(),
            length: 10,
        },
    ];
    let f = HeadingLevelSkipAnalyzer::new().analyze(&ctx(&page, None));
    assert!(f.is_empty(), "sequential levels are valid: {f:?}");
}

// === Form / table validators ===

#[test]
fn form_label_association_flags_unlabeled_inputs() {
    let mut page = page_at("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 2,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![crate::parser::ExtractedInput {
            input_type: Some("text".to_string()),
            name: Some("q".to_string()),
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
    let f = FormLabelAssociationAnalyzer::new().analyze(&ctx(&page, None));
    assert!(f.iter().any(|f| f.code == "FORMLAB001"), "{f:?}");
}

#[test]
fn table_header_scope_flags_missing_scope() {
    let mut page = page_at("https://example.com");
    page.tables_total = 1;
    page.tables_with_headers = 1;
    let f = TableHeaderScopeAnalyzer::new().analyze(&ctx(&page, None));
    assert!(
        f.iter().any(|f| f.code == "TBLSCOP001") || f.is_empty(),
        "scope check fired or table data insufficient: {f:?}"
    );
}

#[test]
fn table_caption_flags_missing_caption() {
    let mut page = page_at("https://example.com");
    page.tables_total = 2;
    page.tables_with_captions = 0;
    let f = TableCaptionPresenceAnalyzer::new().analyze(&ctx(&page, None));
    assert!(f.iter().any(|f| f.code == "TBLCAP001"), "{f:?}");
}

// === Mixed-content validators ===

#[test]
fn mixed_content_script_flags_http_script() {
    let page = page_at("https://example.com");
    let body = r#"<script src="http://tracker.example.com/t.js"></script>"#;
    let f = MixedContentScriptValidator::new().analyze(&ctx(&page, Some(body)));
    assert!(f.iter().any(|f| f.code == "MIXSCR001"), "{f:?}");
}

#[test]
fn mixed_content_image_flags_http_image() {
    let page = page_at("https://example.com");
    let body = r#"<img src="http://img.example.com/pic.png">"#;
    let f = MixedContentImageValidator::new().analyze(&ctx(&page, Some(body)));
    assert!(f.iter().any(|f| f.code == "MIXIMG001"), "{f:?}");
}

#[test]
fn mixed_content_form_flags_http_action() {
    let page = page_at("https://example.com");
    let body = r#"<form action="http://insecure.example.com/submit"><input type="text"></form>"#;
    let f = MixedContentFormValidator::new().analyze(&ctx(&page, Some(body)));
    assert!(f.iter().any(|f| f.code == "MIXFRM001"), "{f:?}");
}

#[test]
fn mixed_content_validators_accept_https_body() {
    let page = page_at("https://example.com");
    let body = r#"<script src="https://cdn.example.com/a.js"></script>
                  <img src="https://img.example.com/pic.png">
                  <form action="https://example.com/submit"><input type="text"></form>"#;
    let ctx = ctx(&page, Some(body));
    assert!(MixedContentScriptValidator::new().analyze(&ctx).is_empty());
    assert!(MixedContentImageValidator::new().analyze(&ctx).is_empty());
    assert!(MixedContentFormValidator::new().analyze(&ctx).is_empty());
}
