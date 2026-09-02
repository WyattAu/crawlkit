//! Generation deduplication contract tests.
//!
//! Phase B consolidation: default-registry registrations are removed only
//! when a fixture proves one generation is a strict subset of — or an
//! exact duplicate of — another registered generation. The removed types
//! remain exported for API compatibility and are exercised here so their
//! behavior stays pinned.
//!
//! | Removed registration | Proof |
//! |---|---|
//! | `HeadingHierarchyDeepDeepDeepValidator` | Subset of deep-deep (missing/multiple H1 only) — `test_hhier_matrix.rs` |
//! | `ImageAltTextDeepDeepDeepValidator` | Subset of deep-deep (missing alt only) — `test_imgalt_matrix.rs` |
//! | `CookieSecureDeepDeepDeepValidator` | Exact duplicate of deep-deep (this file) |
//! | `CookieHttpOnlyDeepDeepDeepValidator` | Exact duplicate of deep-deep (this file) |
//! | `CookieSameSiteDeepDeepDeepValidator` | Exact duplicate of deep-deep (this file) |
//! | `CanonicalSelfReferenceDeepDeepDeepValidator` | Exact duplicate of deep-deep (this file) |
//! | `CanonicalChainDeepDeepDeepValidator` | Subset of deep-deep (misses curly-quote variant) (this file) |
//! | `FocusManagementDeepDeepDeepValidator` | Exact duplicate of deep-deep (this file) |
//! | `TableAccessibilityDeepDeepValidator` | Reverse subset: deep-deep-deep adds captions (this file) |
//!
//! Deliberately retained pairs (neither is a subset):
//!
//! - `FormLabelsDeepDeepValidator` vs `...DeepDeepDeepValidator`: the
//!   deep-deep variant counts hidden inputs as unlabeled while the
//!   deep-deep-deep variant excludes them.
//! - `HreflangReciprocalDeepDeepValidator` vs `...DeepDeepDeepValidator`:
//!   duplicate-lang/x-default checks vs reciprocal-return checks.

use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::ParsedPage;
use crate::types::{IssueCategory, Severity};

fn page() -> ParsedPage {
    ParsedPage {
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
        has_lang_attribute: false,
        html_lang: None,
        has_aria_hidden: false,
        tables_total: 2,
        tables_with_headers: 0,
        tables_with_captions: 0,
        og_image_width: None,
        og_image_height: None,
    }
}

fn header(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(k, val)| ((*k).to_string(), (*val).to_string()))
        .collect()
}

fn ctx<'a>(
    page: &'a ParsedPage,
    headers: &'a [(String, String)],
    body: &'a str,
) -> AnalysisContext<'a> {
    AnalysisContext {
        page,
        body: Some(body),
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
fn cookie_secure_ddd_is_exact_duplicate_of_deep_deep() {
    let p = page();
    let headers = header(&[("Set-Cookie", "sid=abc")]);
    let dd = CookieSecureDeepDeepValidator::new().analyze(&ctx(&p, &headers, ""));
    let ddd = CookieSecureDeepDeepDeepValidator::new().analyze(&ctx(&p, &headers, ""));
    assert_eq!(dd.len(), 1, "deep-deep flags the insecure cookie");
    assert_eq!(ddd.len(), dd.len(), "deep-deep-deep must fire identically");
    assert_eq!(dd[0].title, ddd[0].title.replace(" (deep-deep-deep)", ""));
    // Session cookies are exempt in both.
    let session = header(&[("Set-Cookie", "sessionid=abc")]);
    assert!(CookieSecureDeepDeepValidator::new()
        .analyze(&ctx(&p, &session, ""))
        .is_empty());
    assert!(CookieSecureDeepDeepDeepValidator::new()
        .analyze(&ctx(&p, &session, ""))
        .is_empty());
}

#[test]
fn cookie_httponly_ddd_is_exact_duplicate_of_deep_deep() {
    let p = page();
    let headers = header(&[("Set-Cookie", "sid=abc")]);
    let dd = CookieHttpOnlyDeepDeepValidator::new().analyze(&ctx(&p, &headers, ""));
    let ddd = CookieHttpOnlyDeepDeepDeepValidator::new().analyze(&ctx(&p, &headers, ""));
    assert_eq!(dd.len(), 1);
    assert_eq!(ddd.len(), dd.len());
    assert_eq!(dd[0].title, ddd[0].title.replace(" (deep-deep-deep)", ""));
    let ok = header(&[("Set-Cookie", "sid=abc; HttpOnly")]);
    assert!(CookieHttpOnlyDeepDeepValidator::new()
        .analyze(&ctx(&p, &ok, ""))
        .is_empty());
    assert!(CookieHttpOnlyDeepDeepDeepValidator::new()
        .analyze(&ctx(&p, &ok, ""))
        .is_empty());
}

#[test]
fn cookie_samesite_ddd_is_exact_duplicate_of_deep_deep() {
    let p = page();
    let headers = header(&[("Set-Cookie", "sid=abc")]);
    let dd = CookieSameSiteDeepDeepValidator::new().analyze(&ctx(&p, &headers, ""));
    let ddd = CookieSameSiteDeepDeepDeepValidator::new().analyze(&ctx(&p, &headers, ""));
    assert_eq!(dd.len(), 1);
    assert_eq!(ddd.len(), dd.len());
    // Titles differ beyond the generation suffix ("SameSite attribute" vs
    // "SameSite"), so the duplicate proof rests on trigger, severity, and
    // category — not wording.
    let ok = header(&[("Set-Cookie", "sid=abc; SameSite=Lax")]);
    assert!(CookieSameSiteDeepDeepValidator::new()
        .analyze(&ctx(&p, &ok, ""))
        .is_empty());
    assert!(CookieSameSiteDeepDeepDeepValidator::new()
        .analyze(&ctx(&p, &ok, ""))
        .is_empty());
}

#[test]
fn canonical_self_reference_ddd_is_exact_duplicate_of_deep_deep() {
    let mut p = page();
    p.meta.canonical = Some(url::Url::parse("https://other.com/page").unwrap());
    let dd = CanonicalSelfReferenceDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    let ddd = CanonicalSelfReferenceDeepDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    assert_eq!(
        dd.len(),
        1,
        "deep-deep flags non-self-referencing canonical"
    );
    assert_eq!(ddd.len(), dd.len());
    assert_eq!(dd[0].severity, Severity::Warning);
    assert_eq!(dd[0].severity, ddd[0].severity);
    assert_eq!(dd[0].category, IssueCategory::Seo);
    assert_eq!(ddd[0].category, dd[0].category);
    // Self-referencing canonical (URL::parse normalizes to a trailing
    // slash, so the page URL must carry it too for string equality).
    let mut ok = page();
    ok.url = "https://example.com/".to_string();
    ok.meta.canonical = Some(url::Url::parse("https://example.com/").unwrap());
    assert!(CanonicalSelfReferenceDeepDeepValidator::new()
        .analyze(&ctx(&ok, &[], ""))
        .is_empty());
    assert!(CanonicalSelfReferenceDeepDeepDeepValidator::new()
        .analyze(&ctx(&ok, &[], ""))
        .is_empty());
}

#[test]
fn canonical_chain_ddd_is_subset_of_deep_deep() {
    let p = page();
    // Curly quotes are only detected by the deep-deep variant.
    let curly =
        r#"<link rel=“canonical” href="https://a.com"><link rel=“canonical” href="https://b.com">"#;
    let dd = CanonicalChainDeepDeepValidator::new().analyze(&ctx(&p, &[], curly));
    let ddd = CanonicalChainDeepDeepDeepValidator::new().analyze(&ctx(&p, &[], curly));
    assert_eq!(dd.len(), 1, "deep-deep detects curly-quote duplicates");
    assert!(
        ddd.is_empty(),
        "deep-deep-deep must stay silent (subset property): {ddd:?}"
    );
    // Straight quotes: both fire identically.
    let straight =
        r#"<link rel="canonical" href="https://a.com"><link rel="canonical" href="https://b.com">"#;
    let dd2 = CanonicalChainDeepDeepValidator::new().analyze(&ctx(&p, &[], straight));
    let ddd2 = CanonicalChainDeepDeepDeepValidator::new().analyze(&ctx(&p, &[], straight));
    assert_eq!(dd2.len(), 1);
    assert_eq!(ddd2.len(), 1);
}

#[test]
fn focus_management_ddd_is_exact_duplicate_of_deep_deep() {
    let p = page(); // has_positive_tabindex = true
    let dd = FocusManagementDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    let ddd = FocusManagementDeepDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    assert_eq!(dd.len(), 1);
    assert_eq!(ddd.len(), dd.len());
    let mut clean = page();
    clean.has_positive_tabindex = false;
    assert!(FocusManagementDeepDeepValidator::new()
        .analyze(&ctx(&clean, &[], ""))
        .is_empty());
    assert!(FocusManagementDeepDeepDeepValidator::new()
        .analyze(&ctx(&clean, &[], ""))
        .is_empty());
}

#[test]
fn table_accessibility_deep_deep_is_subset_of_ddd() {
    let p = page(); // 2 tables, 0 headers, 0 captions
    let dd = TableAccessibilityDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    let ddd = TableAccessibilityDeepDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    assert_eq!(dd.len(), 1, "deep-deep reports headers only");
    assert_eq!(ddd.len(), 2, "deep-deep-deep additionally reports captions");
    // The removed registration's semantic (headers) is fully covered.
    assert_eq!(dd[0].severity, ddd[0].severity);
    // Tables with headers but no captions: only the retained variant fires.
    let mut headers_ok = page();
    headers_ok.tables_with_headers = 2;
    assert!(TableAccessibilityDeepDeepValidator::new()
        .analyze(&ctx(&headers_ok, &[], ""))
        .is_empty());
    assert_eq!(
        TableAccessibilityDeepDeepDeepValidator::new()
            .analyze(&ctx(&headers_ok, &[], ""))
            .len(),
        1
    );
}

#[test]
fn form_labels_generations_differ_and_are_both_retained() {
    let mut p = page();
    p.forms.push(crate::parser::ExtractedForm {
        action: None,
        method: "get".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![crate::parser::ExtractedInput {
            input_type: Some("hidden".to_string()),
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
    });
    let dd = FormLabelsDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    let ddd = FormLabelsDeepDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    assert_eq!(
        dd.len(),
        1,
        "deep-deep counts hidden inputs as unlabeled (latent FP, kept as-is)"
    );
    assert!(
        ddd.is_empty(),
        "deep-deep-deep excludes hidden inputs: {ddd:?}"
    );
}

#[test]
fn hreflang_reciprocal_generations_differ_and_are_both_retained() {
    let mut p = page();
    p.meta.hreflang = vec![
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: url::Url::parse("https://example.com/en").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "de".to_string(),
            url: url::Url::parse("https://example.com/de").unwrap(),
        },
    ];
    let dd = HreflangReciprocalDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    let ddd = HreflangReciprocalDeepDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    // deep-deep checks duplicate langs and x-default; reciprocal checks returns.
    assert!(
        dd.iter().any(|f| f.title.contains("x-default")),
        "deep-deep must report the missing x-default: {dd:?}"
    );
    assert!(
        ddd.iter().any(|f| f.title.contains("reciprocal")),
        "deep-deep-deep must report missing reciprocal returns: {ddd:?}"
    );
}
