//! WWW/apex canonical-host consistency from links the crawl already has.
//!
//! # Why this exists
//!
//! A site reachable at both `example.com` and `www.example.com` splits its
//! crawl signals in two unless one host form permanently redirects to the
//! other. When a single page links to *both* forms of its own site, the
//! crawler (and every visitor) is being invited to two hosts that both serve
//! content — the classic canonical-ambiguity defect. No existing analyzer
//! compares host forms; canonical/hreflang validators compare the canonical
//! tag against the page URL, not the site's two front doors.
//!
//! # What is reported
//!
//! One Warning per page, code `WWAPEX001`, when the page's own host and its
//! links collectively reference **both** the apex and the `www.` form of the
//! same site, and the recorded transport observation shows no redirect
//! between the two forms. One defect per host, so one finding per page — the
//! report layer collapses repeats across pages.
//!
//! # What is deliberately NOT done
//!
//! - **No DNS lookups, no network requests.** "Both resolve" is evidenced the
//!   only way an analyzer can see it: both forms are linked from one page,
//!   which means the crawler will fetch both. The [`Analyzer`] contract is
//!   passive (see `tech_detection.rs`, GAP G7).
//! - **No verdict from the canonical tag alone.** A canonical URL names a
//!   preferred form but does not redirect visitors; the ambiguity finding
//!   stands even when a canonical is present. Canonical *correctness* is the
//!   `CanonicalUrlValidator` family's job.
//! - **Only the page's own site is judged.** Both forms of a *different*
//!   domain linked from one page are not this page's canonical-ambiguity
//!   defect and are ignored.

use std::collections::HashSet;

use crate::analyzers::Analyzer;
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};
use url::Url;

/// `www.example.com` → `example.com`; anything else unchanged.
fn apex_form(host: &str) -> &str {
    host.strip_prefix("www.").unwrap_or(host)
}

/// Lowercased host of `url`, or `None` for non-special or malformed URLs.
fn host_of(url: &Url) -> Option<String> {
    url.host_str().map(|h| h.to_ascii_lowercase())
}

/// Passive www/apex canonical-host consistency check.
pub struct WwwApexConsistencyAnalyzer;

impl Default for WwwApexConsistencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl WwwApexConsistencyAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WwwApexConsistencyAnalyzer {
    fn name(&self) -> &str {
        "www-apex-consistency"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let url = ctx.page.url.as_str();
        let Some(page_host) = Url::parse(url).ok().as_ref().and_then(host_of) else {
            return Vec::new();
        };
        let base = apex_form(&page_host).to_string();

        // Host forms of this site the page sends the crawler to. Every link
        // counts, external or not: the parser flags the opposite host *form*
        // of the same site as "external" (different host string), which is
        // exactly the pair this analyzer compares.
        let mut linked_forms: HashSet<String> = HashSet::new();
        linked_forms.insert(page_host);
        for link in &ctx.page.links {
            if let Some(host) = Url::parse(link.href.as_str())
                .ok()
                .as_ref()
                .and_then(host_of)
            {
                if apex_form(&host) == base {
                    linked_forms.insert(host);
                }
            }
        }

        let apex_linked = linked_forms.contains(base.as_str());
        let www_linked = linked_forms.contains(&format!("www.{base}"));
        if !(apex_linked && www_linked) {
            return Vec::new();
        }

        // The escape hatch: a recorded hop between the two forms proves one
        // redirects to the other, which resolves the ambiguity.
        let redirect_between_forms = ctx.redirect_chain.iter().any(|hop| {
            let (Some(from), Some(to)) = (host_of(&hop.from), host_of(&hop.to)) else {
                return false;
            };
            apex_form(&from) == base
                && apex_form(&to) == base
                && from != to
                && linked_forms.contains(&from)
                && linked_forms.contains(&to)
        });
        if redirect_between_forms {
            return Vec::new();
        }

        vec![Finding {
            severity: Severity::Warning,
            category: IssueCategory::Seo,
            code: "WWAPEX001".to_string(),
            title: "Apex and www hosts both used without redirect".to_string(),
            description: format!(
                "This page links to both {base} and www.{base}, so the crawl fetches \
                 both host forms, and no redirect between them was observed. The two \
                 forms split crawl signals and can be treated as duplicate hosts by \
                 search engines unless one permanently redirects to the other."
            ),
            url: url.to_string(),
            recommendation: "Pick one canonical host form and 301-redirect the other \
                             to it, then link internally with the canonical form only."
                .to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ExtractedLink, ParsedPage};
    use crate::RedirectHop;

    fn link(href: &str) -> ExtractedLink {
        ExtractedLink {
            href: href.to_string(),
            text: String::new(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }
    }

    fn fixture_page(url: &str, links: Vec<ExtractedLink>) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: vec![],
            links,
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
            has_lang_attribute: true,
            html_lang: Some("en".to_string()),
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn hop(from: &str, to: &str) -> RedirectHop {
        RedirectHop {
            from: Url::parse(from).expect("static test URL"),
            to: Url::parse(to).expect("static test URL"),
            status_code: 301,
        }
    }

    fn ctx<'a>(page: &'a ParsedPage, chain: &'a [RedirectHop]) -> AnalysisContext<'a> {
        AnalysisContext {
            redirect_chain: chain,
            ..AnalysisContext::new(page)
        }
    }

    #[test]
    fn links_to_both_host_forms_are_reported() {
        let page = fixture_page(
            "https://example.com/",
            vec![
                link("https://example.com/about"),
                link("https://www.example.com/about"),
            ],
        );
        let findings = WwwApexConsistencyAnalyzer::new().analyze(&ctx(&page, &[]));
        assert_eq!(findings.len(), 1, "both front doors linked: {findings:?}");
        assert_eq!(findings[0].code, "WWAPEX001");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].description.contains("example.com"));
    }

    #[test]
    fn apex_page_linking_www_counts_its_own_host() {
        // The page itself is one form; a single opposite-form link suffices.
        let page = fixture_page(
            "https://example.com/",
            vec![link("https://www.example.com/")],
        );
        let findings = WwwApexConsistencyAnalyzer::new().analyze(&ctx(&page, &[]));
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn single_host_form_is_silent() {
        let page = fixture_page(
            "https://example.com/",
            vec![link("https://example.com/a"), link("https://example.com/b")],
        );
        assert!(WwwApexConsistencyAnalyzer::new()
            .analyze(&ctx(&page, &[]))
            .is_empty());
    }

    #[test]
    fn observed_redirect_between_forms_resolves_ambiguity() {
        let page = fixture_page(
            "https://www.example.com/",
            vec![
                link("https://www.example.com/a"),
                link("https://example.com/b"),
            ],
        );
        let chain = [hop("https://www.example.com/", "https://example.com/")];
        assert!(WwwApexConsistencyAnalyzer::new()
            .analyze(&ctx(&page, &chain))
            .is_empty());
    }

    #[test]
    fn both_forms_of_a_different_site_are_ignored() {
        let page = fixture_page(
            "https://example.com/",
            vec![link("https://other.org/"), link("https://www.other.org/")],
        );
        assert!(
            WwwApexConsistencyAnalyzer::new()
                .analyze(&ctx(&page, &[]))
                .is_empty(),
            "another domain's host split is not this page's defect"
        );
    }

    #[test]
    fn subdomains_do_not_count_as_apex() {
        // `cdn.example.com` is not the apex form of example.com, so a page on
        // cdn.example.com linking apex+www is not its own ambiguity.
        let page = fixture_page(
            "https://cdn.example.com/page",
            vec![
                link("https://example.com/"),
                link("https://www.example.com/"),
            ],
        );
        let findings = WwwApexConsistencyAnalyzer::new().analyze(&ctx(&page, &[]));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn analyzer_name() {
        assert_eq!(
            WwwApexConsistencyAnalyzer::new().name(),
            "www-apex-consistency"
        );
    }
}
