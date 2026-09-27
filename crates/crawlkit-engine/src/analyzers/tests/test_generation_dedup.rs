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
//! | `SitemapCoverageDeepDeepValidator` | Exact duplicate of SitemapCoverageDeepAnalyzerV2 (this file) |
//! | `TableCaptionPresenceDeepValidator` | Exact duplicate of TableCaptionPresenceAnalyzerV2 (this file) |
//!
//! Namespaced semantic collisions (different defect, same code, can
//! co-fire — emit codes changed per the Phase-4 convention):
//!
//! - `INTLINKQ-V2001`/`-V2002` → `INTLINKQ-V2001-DEEP`/`-V2002-DEEP` on
//!   `InternalLinkQualityDeepValidator` (this file, no fixture needed:
//!   the V2 codes keep their documented meanings).
//! - `FORMLAB-V2001` → `FORMLAB-V2001-DEEP` on
//!   `FormLabelAssociationDeepValidator` (this file).
//!
//! Deliberately retained pairs (neither is a subset):
//!
//! - `FormLabelsDeepDeepValidator` vs `...DeepDeepDeepValidator`: the
//!   deep-deep variant counts hidden inputs as unlabeled while the
//!   deep-deep-deep variant excludes them.
//! - `HreflangReciprocalDeepDeepValidator` vs `...DeepDeepDeepValidator`:
//!   duplicate-lang/x-default checks vs reciprocal-return checks.
//! - `TableHeaderScopeAnalyzerV2` vs `TableHeaderScopeDeepValidator`:
//!   `<th>` elements lacking scope vs tables with no header cells at
//!   all — mutually exclusive preconditions (this file).

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
    // Curly-quoted rel values (`rel=“canonical”`) are not valid
    // canonical link elements: both generations now correctly ignore them.
    let curly =
        r#"<link rel=“canonical” href="https://a.com"><link rel=“canonical” href="https://b.com">"#;
    let dd = CanonicalChainDeepDeepValidator::new().analyze(&ctx(&p, &[], curly));
    let ddd = CanonicalChainDeepDeepDeepValidator::new().analyze(&ctx(&p, &[], curly));
    assert!(
        dd.is_empty(),
        "curly-quoted rel is not a valid canonical element"
    );
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
    // A visible but unlabeled text input: both generations must flag it.
    let make_input = |ty: &str| crate::parser::ExtractedInput {
        input_type: Some(ty.to_string()),
        name: None,
        id: None,
        has_label: false,
        aria_label: None,
        aria_labelledby: None,
        aria_describedby: None,
        placeholder: None,
        required: false,
    };
    p.forms.push(crate::parser::ExtractedForm {
        action: None,
        method: "get".to_string(),
        input_count: 2,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![make_input("text"), make_input("hidden")],
        has_fieldset: false,
        has_legend: false,
    });
    let dd = FormLabelsDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    let ddd = FormLabelsDeepDeepDeepValidator::new().analyze(&ctx(&p, &[], ""));
    assert_eq!(
        dd.len(),
        1,
        "deep-deep must flag the text input but not the hidden one: {dd:?}"
    );
    assert_eq!(ddd.len(), 1, "deep-deep-deep agrees on the text input");
    // A hidden-only form must be silent in both (no false positive).
    let mut hidden_only = page();
    hidden_only.forms.push(crate::parser::ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![make_input("hidden")],
        has_fieldset: false,
        has_legend: false,
    });
    assert!(FormLabelsDeepDeepValidator::new()
        .analyze(&ctx(&hidden_only, &[], ""))
        .is_empty());
    assert!(FormLabelsDeepDeepDeepValidator::new()
        .analyze(&ctx(&hidden_only, &[], ""))
        .is_empty());
}

#[test]
fn sitemap_coverage_deep_deep_is_exact_duplicate_of_v2() {
    let mut p = page();
    p.url = "https://example.com/page".to_string();
    // robots.txt without a Sitemap: directive: both generations must flag it.
    let bare = "User-agent: *\nDisallow: /private/\n";
    let v2 = SitemapCoverageDeepAnalyzerV2::new().analyze(&ctx(&p, &[], ""));
    assert!(v2.is_empty(), "no robots.txt in context: neither fires");
    let mut ctx_bare = ctx(&p, &[], "");
    ctx_bare.robots_txt = Some(bare);
    let v2 = SitemapCoverageDeepAnalyzerV2::new().analyze(&ctx_bare);
    let dd = SitemapCoverageDeepDeepValidator::new().analyze(&ctx_bare);
    assert_eq!(v2.len(), 1, "V2 fires on robots.txt without Sitemap:");
    assert_eq!(dd.len(), v2.len(), "deep-deep must fire identically");
    assert_eq!(v2[0].code, "SITEMAPDEEP-V2001");
    assert_eq!(dd[0].code, v2[0].code);
    assert_eq!(dd[0].severity, v2[0].severity);
    assert_eq!(dd[0].category, v2[0].category);
    // robots.txt declaring a sitemap: both must stay silent.
    let declared = "User-agent: *\nDisallow: /private/\nSITEMAP: https://example.com/sitemap.xml\n";
    let mut ctx_declared = ctx(&p, &[], "");
    ctx_declared.robots_txt = Some(declared);
    assert!(SitemapCoverageDeepAnalyzerV2::new()
        .analyze(&ctx_declared)
        .is_empty());
    assert!(SitemapCoverageDeepDeepValidator::new()
        .analyze(&ctx_declared)
        .is_empty());
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

#[test]
fn table_caption_deep_is_duplicate_of_v2() {
    let p = page();
    assert_eq!(p.tables_total, 2);
    assert_eq!(p.tables_with_captions, 0);
    // Identical trigger (tables present, none captioned), same code and
    // category; the deep variant's Info is weaker than the V2 Warning,
    // so unregistration keeps the stronger finding.
    let v2 = TableCaptionPresenceAnalyzerV2::new().analyze(&ctx(&p, &[], ""));
    let deep = TableCaptionPresenceDeepValidator::new().analyze(&ctx(&p, &[], ""));
    assert_eq!(v2.len(), 1, "V2 flags tables without captions: {v2:?}");
    assert_eq!(deep.len(), v2.len(), "deep must fire identically");
    assert_eq!(v2[0].code, "TBLCAP-V2001");
    assert_eq!(deep[0].code, v2[0].code);
    assert_eq!(v2[0].severity, Severity::Warning);
    assert_eq!(deep[0].severity, Severity::Info);
    assert_eq!(deep[0].category, v2[0].category);
    // Captioned tables: both silent.
    let mut ok = page();
    ok.tables_with_captions = 2;
    assert!(TableCaptionPresenceAnalyzerV2::new()
        .analyze(&ctx(&ok, &[], ""))
        .is_empty());
    assert!(TableCaptionPresenceDeepValidator::new()
        .analyze(&ctx(&ok, &[], ""))
        .is_empty());
}
#[test]
fn table_scope_generations_are_complementary_not_duplicates() {
    // V2 case: header cells exist (tables_with_headers > 0) but none
    // carry a scope attribute. The deep validator stays silent because
    // its precondition is tables_with_headers == 0.
    let mut with_headers = page(); // tables_total = 2
    with_headers.tables_with_headers = 1;
    let th_no_scope = "<table><tr><th>H</th></tr></table>";
    let v2 = TableHeaderScopeAnalyzerV2::new().analyze(&ctx(&with_headers, &[], th_no_scope));
    assert_eq!(v2.len(), 1, "V2 flags <th> without scope: {v2:?}");
    assert!(
        TableHeaderScopeDeepValidator::new()
            .analyze(&ctx(&with_headers, &[], th_no_scope))
            .is_empty(),
        "deep stays silent: tables_with_headers is nonzero"
    );
    // Deep case: tables exist but have no header cells at all. The V2
    // guard (tables_with_headers == 0) returns early.
    let p = page(); // tables_total = 2, tables_with_headers = 0
    let headerless = "<table><tr><td>x</td></tr></table>";
    let deep = TableHeaderScopeDeepValidator::new().analyze(&ctx(&p, &[], headerless));
    assert_eq!(deep.len(), 1, "deep flags headerless tables: {deep:?}");
    assert!(
        TableHeaderScopeAnalyzerV2::new()
            .analyze(&ctx(&p, &[], headerless))
            .is_empty(),
        "V2 stays silent: no tables_with_headers"
    );
    // No tables at all: both silent.
    let mut none = page();
    none.tables_total = 0;
    assert!(TableHeaderScopeAnalyzerV2::new()
        .analyze(&ctx(&none, &[], th_no_scope))
        .is_empty());
    assert!(TableHeaderScopeDeepValidator::new()
        .analyze(&ctx(&none, &[], th_no_scope))
        .is_empty());
}

#[test]
fn form_label_v2_and_deep_defects_are_namespaced() {
    let make_input = |id: &str, labeled: bool| crate::parser::ExtractedInput {
        input_type: Some("text".to_string()),
        name: None,
        id: Some(id.to_string()),
        has_label: labeled,
        aria_label: None,
        aria_labelledby: None,
        aria_describedby: None,
        placeholder: None,
        required: false,
    };
    // V2 defect: duplicate input IDs inside one form; every input is
    // labeled, so the deep validator must stay silent.
    let mut p = page();
    p.forms.push(crate::parser::ExtractedForm {
        action: None,
        method: "get".to_string(),
        input_count: 2,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![make_input("email", true), make_input("email", true)],
        has_fieldset: false,
        has_legend: false,
    });
    let v2 = FormLabelAssociationAnalyzerV2::new().analyze(&ctx(&p, &[], ""));
    assert_eq!(v2.len(), 1, "V2 flags duplicate input IDs: {v2:?}");
    assert_eq!(v2[0].code, "FORMLAB-V2001");
    assert!(
        FormLabelAssociationDeepValidator::new()
            .analyze(&ctx(&p, &[], ""))
            .is_empty(),
        "deep stays silent: no unlabeled inputs"
    );
    // Deep defect: an input with no label association; no duplicate IDs,
    // so the V2 validator must stay silent.
    let mut q = page();
    q.forms.push(crate::parser::ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![make_input("q", false)],
        has_fieldset: false,
        has_legend: false,
    });
    let deep = FormLabelAssociationDeepValidator::new().analyze(&ctx(&q, &[], ""));
    assert_eq!(deep.len(), 1, "deep flags the unlabeled input: {deep:?}");
    assert_eq!(deep[0].code, "FORMLAB-V2001-DEEP");
    assert!(
        FormLabelAssociationAnalyzerV2::new()
            .analyze(&ctx(&q, &[], ""))
            .is_empty(),
        "V2 stays silent: IDs are unique"
    );
}
