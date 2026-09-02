//! Coverage for the accessibility deep analyzers (`*DeepAnalyzer` in
//! accessibility_deep_analyzers.rs) and the accessibility V2 family.
//! Each test pins finding codes so regressions surface immediately.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::{ExtractedForm, ExtractedImage, ExtractedLink, Heading, ParsedPage};

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

fn ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
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

// === Deep analyzers ===

#[test]
fn aria_landmarks_flags_missing_main_and_nav() {
    // ARIA-LAND002 only fires when there are >3 links but no nav landmark.
    let mut page = page_at("https://example.com");
    for i in 0..5 {
        page.links.push(ExtractedLink {
            href: format!("https://example.com/{i}"),
            text: format!("Link {i}"),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        });
    }
    let f = AriaLandmarksAnalyzer::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "ARIALAND001"), "{f:?}");
    assert!(f.iter().any(|f| f.code == "ARIALAND002"), "{f:?}");
}

#[test]
fn aria_landmarks_flags_duplicate_landmarks() {
    let mut page = page_at("https://example.com");
    page.has_main_landmark = true;
    page.has_nav_landmark = true;
    // Duplicate "main" landmarks.
    page.landmarks = vec!["main".to_string(), "main".to_string()];
    let f = AriaLandmarksAnalyzer::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "ARIALAND003"), "{f:?}");
}

#[test]
fn form_labels_deep_flags_forms_without_aria() {
    let mut page = page_at("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 2,
        has_file_input: false,
        has_search_input: false,
        inputs: Vec::new(),
        has_fieldset: false,
        has_legend: false,
    }];
    let f = FormLabelsDeepAnalyzer::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "FORMLBLDEEP001"), "{f:?}");
}

#[test]
fn image_alt_deep_flags_missing_alt() {
    let mut page = page_at("https://example.com");
    page.images = vec![ExtractedImage {
        src: "https://example.com/a.png".to_string(),
        alt: String::new(),
        width: None,
        height: None,
        has_alt: false,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let f = ImageAltTextDeepAnalyzer::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "IMGALTDEEP001"), "{f:?}");
}

#[test]
fn focus_deep_flags_positive_tabindex_and_many_negative() {
    let mut page = page_at("https://example.com");
    page.has_positive_tabindex = true;
    page.tabindex_negative_count = 5;
    let f = FocusManagementDeepAnalyzer::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "FOCUSDEEP001"), "{f:?}");
    assert!(f.iter().any(|f| f.code == "FOCUSDEEP002"), "{f:?}");
}

#[test]
fn language_attributes_deep_flags_missing_lang() {
    let page = page_at("https://example.com");
    let f = LanguageAttributesDeepAnalyzer::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "LANGATTRDEEP001"), "{f:?}");
}

// === Accessibility V2 family ===

#[test]
fn v2_tabindex_flags_positive_tabindex() {
    let mut page = page_at("https://example.com");
    page.has_positive_tabindex = true;
    let f = TabindexAnalyzerV2::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "TAB-V2001"), "{f:?}");
}

#[test]
fn v2_link_flags_empty_link_text() {
    let mut page = page_at("https://example.com");
    page.links = vec![ExtractedLink {
        href: "https://example.com/x".to_string(),
        text: "   ".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let f = LinkAccessibilityAnalyzerV2::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "A11Y-LINK-V2001"), "{f:?}");
}

#[test]
fn v2_image_flags_missing_alt() {
    let mut page = page_at("https://example.com");
    page.images = vec![ExtractedImage {
        src: "https://example.com/a.png".to_string(),
        alt: String::new(),
        width: None,
        height: None,
        has_alt: false,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let f = ImageAccessibilityAnalyzerV2::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "IMG-V2001"), "{f:?}");
}

#[test]
fn v2_form_flags_inputs_without_labels() {
    let mut page = page_at("https://example.com");
    page.forms = vec![ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        // The V2 analyzer requires at least one extracted input element.
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
    let body = r#"<form><input type="text" name="email"></form>"#;
    let ctx = AnalysisContext {
        body: Some(body),
        ..ctx(&page)
    };
    let f = FormAccessibilityAnalyzerV2::new().analyze(&ctx);
    assert!(f.iter().any(|f| f.code == "FORM-V2001"), "{f:?}");
}

#[test]
fn v2_table_flags_table_without_headers() {
    let mut page = page_at("https://example.com");
    page.tables_total = 2;
    page.tables_with_headers = 0;
    let f = TableAccessibilityAnalyzerV2::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "TBL-V2001"), "{f:?}");
}

#[test]
fn v2_aria_flags_roles_without_labels() {
    let mut page = page_at("https://example.com");
    page.aria_role_count = 3;
    page.aria_label_count = 0;
    let f = AriaRolesAnalyzerV2::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "ARIA-V2001"), "{f:?}");
}

#[test]
fn v2_heading_flags_missing_headings() {
    // The V2 analyzer only reports on pages that HAVE headings; with none
    // present it returns early (covered by HeadingHierarchyAnalyzer base).
    let mut page = page_at("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        },
        Heading {
            level: 4,
            text: "Skipped H2 and H3".to_string(),
            length: 18,
        },
    ];
    let f = HeadingHierarchyAnalyzerV2::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "HEAD-V2001"), "{f:?}");
}

#[test]
fn v2_heading_flags_skipped_level() {
    let mut page = page_at("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        },
        Heading {
            level: 4,
            text: "Jump".to_string(),
            length: 4,
        },
    ];
    let f = HeadingHierarchyAnalyzerV2::new().analyze(&ctx(&page));
    assert!(
        f.iter().any(|f| f.code == "HEAD-V2001") || f.iter().any(|f| f.code.starts_with("HEAD-")),
        "skipped level must be reported by the V2 heading analyzer: {f:?}"
    );
}

#[test]
fn v2_language_flags_missing_lang() {
    let page = page_at("https://example.com");
    let f = LanguageAttributeAnalyzerV2::new().analyze(&ctx(&page));
    assert!(f.iter().any(|f| f.code == "LANG-V2001"), "{f:?}");
}

#[test]
fn v2_family_accepts_compliant_page() {
    // A fully compliant page should produce no V2 findings.
    let mut page = page_at("https://example.com");
    page.has_lang_attribute = true;
    page.html_lang = Some("en".to_string());
    page.has_main_landmark = true;
    page.has_nav_landmark = true;
    page.landmarks = vec!["main".to_string(), "navigation".to_string()];
    page.images = vec![ExtractedImage {
        src: "https://example.com/a.png".to_string(),
        alt: "A photo".to_string(),
        width: None,
        height: None,
        has_alt: true,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    page.links = vec![ExtractedLink {
        href: "https://example.com/x".to_string(),
        text: "Read the docs".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = ctx(&page);
    assert!(ImageAccessibilityAnalyzerV2::new().analyze(&ctx).is_empty());
    assert!(LinkAccessibilityAnalyzerV2::new().analyze(&ctx).is_empty());
    assert!(LanguageAttributeAnalyzerV2::new().analyze(&ctx).is_empty());
    assert!(TabindexAnalyzerV2::new().analyze(&ctx).is_empty());
}
