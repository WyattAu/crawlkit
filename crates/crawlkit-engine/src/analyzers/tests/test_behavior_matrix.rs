//! Behavior matrix tests for overlapping analyzer generations.
//!
//! These tests document intentional differences before any analyzer is removed
//! from the default registry.

use crate::analyzers::{
    AnalysisContext, Analyzer, FormLabelsDeepAnalyzerV2, FormLabelsDeepDeepValidator,
    HeadingHierarchyDeepAnalyzerV2, HeadingHierarchyDeepDeepValidator, LinkTextQualityAnalyzerV2,
    LinkTextQualityDeepValidator, TableAccessibilityDeepAnalyzerV2,
    TableAccessibilityDeepDeepValidator,
};
use crate::meta::MetaTags;
use crate::parser::ExtractedLink;
use crate::parser::{ExtractedForm, ExtractedInput, ParsedPage};

fn page_with_unlabeled_input() -> ParsedPage {
    ParsedPage {
        url: "https://example.com/form".to_string(),
        meta: MetaTags::default(),
        headings: Vec::new(),
        links: Vec::new(),
        images: Vec::new(),
        forms: vec![ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![ExtractedInput {
                name: None,
                input_type: Some("text".to_string()),
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
        }],
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

#[test]
fn heading_generations_have_distinct_codes_and_explicit_scope() {
    let mut page = page_with_unlabeled_input();
    page.headings = vec![
        crate::parser::Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        },
        crate::parser::Heading {
            level: 3,
            text: "Skipped".to_string(),
            length: 7,
        },
    ];
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

    let deep = HeadingHierarchyDeepAnalyzerV2::new().analyze(&ctx);
    let deep_deep = HeadingHierarchyDeepDeepValidator::new().analyze(&ctx);

    assert!(deep.iter().any(|f| f.code == "HHIER-V2003"));
    assert!(deep_deep.iter().any(|f| f.code == "HHIER-V2004"));
    assert_ne!(
        deep.iter().map(|f| &f.code).collect::<Vec<_>>(),
        deep_deep.iter().map(|f| &f.code).collect::<Vec<_>>()
    );
}

#[test]
fn link_generations_separate_generic_and_empty_signals() {
    let mut page = page_with_unlabeled_input();
    page.links = vec![
        ExtractedLink {
            href: "/empty".to_string(),
            text: String::new(),
            rel: Vec::new(),
            is_external: false,
            aria_label: None,
            img_alt: None,
        },
        ExtractedLink {
            href: "/generic".to_string(),
            text: "click here".to_string(),
            rel: Vec::new(),
            is_external: false,
            aria_label: None,
            img_alt: None,
        },
    ];
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

    let v2 = LinkTextQualityAnalyzerV2::new().analyze(&ctx);
    let deep = LinkTextQualityDeepValidator::new().analyze(&ctx);

    assert!(v2.iter().any(|f| f.code == "LINKTQ-V2001"));
    assert!(v2.iter().any(|f| f.code == "LINKTQ-V2002-V2"));
    assert!(deep.iter().any(|f| f.code == "LINKTQ-V2001-DEEP"));
    assert!(deep.iter().any(|f| f.code == "LINKTQ-V2002-DEEP"));
}

#[test]
fn table_generations_have_distinct_codes_and_same_core_signal() {
    let mut page = page_with_unlabeled_input();
    page.tables_total = 2;
    page.tables_with_headers = 0;
    page.tables_with_captions = 0;
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

    let deep = TableAccessibilityDeepAnalyzerV2::new().analyze(&ctx);
    let deep_deep = TableAccessibilityDeepDeepValidator::new().analyze(&ctx);

    assert!(deep.iter().any(|f| f.code == "TABACC-V2001"));
    assert!(deep.iter().any(|f| f.code == "TABACC-V2002"));
    assert_eq!(deep_deep.len(), 1);
    assert_eq!(deep_deep[0].code, "TABACC-V2001-DEEP-DEEP");
    assert_eq!(deep[0].category, deep_deep[0].category);
}

#[test]
fn form_label_generations_have_distinct_codes_and_same_core_signal() {
    let page = page_with_unlabeled_input();
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

    let deep = FormLabelsDeepAnalyzerV2::new().analyze(&ctx);
    let deep_deep = FormLabelsDeepDeepValidator::new().analyze(&ctx);

    assert_eq!(deep.len(), 1);
    assert_eq!(deep_deep.len(), 1);
    assert_eq!(deep[0].code, "FORMLBL-V2001");
    assert_eq!(deep_deep[0].code, "FORMLBL-V2001-DEEP-DEEP");
    assert_eq!(deep[0].category, deep_deep[0].category);
}
