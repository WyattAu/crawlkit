//! The page's declared language — WCAG 3.1.1 (Level A).
//!
//! # Why this exists
//!
//! The parser has always extracted `has_lang_attribute` and `html_lang`, and no
//! analyzer ever read them. Verified by searching the tree: the only references
//! outside `parser/` were test fixtures setting the field to `false`. A WCAG
//! Level A success criterion — one of the first things an accessibility auditor
//! checks, and the easiest for a screen reader to get wrong — went unreported
//! on every page of every crawl.
//!
//! Found by the testbed: a fixture with no `lang` attribute produced no finding
//! at all, which is what prompted this file.
//!
//! # What is reported
//!
//! - a missing `lang` attribute on `<html>` — the Level A failure
//! - a `lang` present but not a well-formed BCP 47 primary tag, which screen
//!   readers cannot match to a pronunciation dictionary
//!
//! An *invalid* value is a separate, less severe finding from an absent one, so
//! they are separate codes rather than one "bad lang" finding.

use crate::analyzers::Analyzer;
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};

/// `<html>` carries no `lang` attribute at all.
pub const LANG_MISSING: &str = "A11Y-LANG-MISSING";
/// `lang` is present but not a well-formed language tag.
pub const LANG_MALFORMED: &str = "A11Y-LANG-MALFORMED";

/// Whether `tag` is a well-formed BCP 47 primary subtag, optionally with region.
///
/// Deliberately accepts the common real forms (`en`, `en-GB`, `zh-Hans`, `pt-BR`)
/// and rejects the things that actually break screen readers: an empty value, a
/// value with spaces, one starting with a digit, or one that is absurdly long.
///
/// Two limits, stated rather than hidden:
///
/// - a bare two-letter subtag is accepted as a language, which also accepts
///   region codes misused as languages (`GB`); separating ISO 639-1 from
///   ISO 3166-1 means shipping both lists, which is not worth it for a check
///   whose purpose is catching values a screen reader cannot use at all
/// - private-use singletons (`x-custom`) are rejected, though they are legal
///   BCP 47, because no screen reader has a pronunciation table for one
///
/// Full BCP 47 validation (extlangs, variants, extensions) is not attempted.
#[must_use]
fn is_plausible_bcp47(tag: &str) -> bool {
    let tag = tag.trim();
    if tag.is_empty() || tag.len() > 35 {
        return false;
    }
    let subtags: Vec<&str> = tag.split('-').collect();
    if subtags.len() > 4 {
        return false;
    }
    let Some(first) = subtags.first() else {
        return false;
    };
    // Primary language subtag: 2-8 letters, and it must not start with a digit.
    let primary_ok = (2..=8).contains(&first.len())
        && first.starts_with(|c: char| c.is_ascii_alphabetic())
        && first.chars().all(|c| c.is_ascii_alphabetic());
    if !primary_ok {
        return false;
    }
    subtags[1..].iter().all(|s| {
        // Region (2-3 letters or 3 digits), script (4 letters), or variant
        // (5-8 alphanumerics).
        let alpha = s.chars().all(|c| c.is_ascii_alphanumeric());
        alpha && !s.is_empty() && (s.len() == 4 || s.len() >= 5 || s.len() <= 3)
    })
}

pub struct HtmlLangAnalyzer;

impl Default for HtmlLangAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlLangAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HtmlLangAnalyzer {
    fn name(&self) -> &str {
        "html-lang"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let url = ctx.page.url.to_string();
        if ctx.page.has_lang_attribute {
            let Some(tag) = ctx.page.html_lang.as_deref() else {
                return Vec::new();
            };
            if !is_plausible_bcp47(tag) {
                return vec![Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: LANG_MALFORMED.to_string(),
                    title: format!("lang attribute is not a usable language tag: {tag:?}"),
                    description: format!(
                        "The html element declares lang={tag:?}, which is not a well-formed \
                         BCP 47 language tag. Screen readers match this value against a \
                         pronunciation table and will fall back to a default voice, so a \
                         page in one language may be read as another."
                    ),
                    url,
                    recommendation: "Use a valid BCP 47 tag: a two- or three-letter language \
                                     code, optionally followed by a region — for example en, \
                                     en-GB, pt-BR or zh-Hans."
                        .to_string(),
                }];
            }
            return Vec::new();
        }

        vec![Finding {
            severity: Severity::Error,
            category: IssueCategory::Accessibility,
            code: LANG_MISSING.to_string(),
            title: "html element has no lang attribute".to_string(),
            description: "The html element carries no lang attribute. WCAG 3.1.1 (Language of \
                          Page) is a Level A requirement: screen readers use it to choose a \
                          pronunciation dictionary, and without it a page may be read in the \
                          wrong voice or letter-by-letter."
                .to_string(),
            url,
            recommendation: "Add lang to the html element with the page's primary language, \
                             for example <html lang=\"en\">."
                .to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plausible_tags() {
        for tag in ["en", "en-GB", "pt-BR", "zh-Hans", "de", "arb"] {
            assert!(is_plausible_bcp47(tag), "{tag} should be accepted");
        }
    }

    #[test]
    fn implausible_tags() {
        for tag in [
            "",
            "   ",
            "e",             // too short to be a language subtag
            "1n",            // starts with a digit
            "en GB",         // space, not a subtag separator
            "a-b-c-d-e",     // more subtags than any real tag
            &"9".repeat(36), // absurdly long
        ] {
            assert!(!is_plausible_bcp47(tag), "{tag:?} should be rejected");
        }
    }

    /// The two known limits of the predicate, pinned so they are decisions
    /// rather than accidents. See the function docs.
    #[test]
    fn known_limits() {
        // Accepted, though `GB` is a region code misused as a language.
        assert!(is_plausible_bcp47("GB"));
        // Rejected, though legal BCP 47: no screen reader has a table for it.
        assert!(!is_plausible_bcp47("x-custom"));
    }

    /// The regression this analyzer exists for: a page with no `lang` at all.
    /// `ContentLanguageValidator` returns early in exactly this case, and its
    /// test asserts that it emits nothing — so the Level A failure was silent.
    #[test]
    fn missing_lang_is_reported() {
        use crate::meta::MetaTags;
        use crate::parser::ParsedPage;

        let page = ParsedPage {
            url: "https://shop.example/".to_string(),
            meta: MetaTags::default(),
            headings: vec![],
            links: vec![],
            images: vec![],
            forms: vec![],
            scripts: vec![],
            styles: vec![],
            structured_data: vec![],
            word_count: 0,
            sentence_count: 0,
            landmarks: vec![],
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
        let page: &'static ParsedPage = Box::leak(Box::new(page));
        let ctx = crate::AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            user_agent: None,
            body_size: None,
            compressed_size: None,
            content_encoding: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let found = HtmlLangAnalyzer::new().analyze(&ctx);
        assert_eq!(found.len(), 1, "expected exactly one finding");
        assert_eq!(found[0].code, LANG_MISSING);
    }
}
