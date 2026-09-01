//! Behavior matrix tests for overlapping analyzer generations.
//!
//! These tests document intentional differences before any analyzer is removed
//! from the default registry.

use crate::analyzers::{
    AnalysisContext, Analyzer, FormLabelsDeepAnalyzerV2, FormLabelsDeepDeepValidator,
};
use crate::meta::MetaTags;
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
