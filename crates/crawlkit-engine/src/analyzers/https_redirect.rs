//! HTTPS-redirect verification from transport the crawl already recorded.
//!
//! # Why this exists
//!
//! A site that serves content over plain HTTP — or redirects `http://` URLs
//! anywhere other than to their `https://` equivalent — ships a downgrade
//! path to every visitor and every crawler that ever follows an `http://`
//! link. The redirect analyzers (REDIR*, MIXPROT001) judge the *shape* of
//! recorded redirect chains; none of them answers the plain question "did
//! this host upgrade HTTP to HTTPS?"
//!
//! # What is reported
//!
//! One Warning per page, code `HTTPSRED001`, when the recorded transport
//! observation shows an `http://` request that did **not** land on its
//! `https://` equivalent — either it redirected within HTTP, redirected to a
//! non-HTTPS scheme, or was served directly over plain HTTP. A page requested
//! over HTTPS, or an `http://` request observed upgrading to `https://`,
//! produces nothing.
//!
//! # What is deliberately NOT done
//!
//! - **No network requests.** The [`crate::analyzers::Analyzer`] contract is
//!   passive by design (see `tech_detection.rs`, GAP G7): an analyzer cannot
//!   add a single request to a crawl. The evidence here is the one hop the
//!   fetch layer can prove — the URL it was asked for and the URL that served
//!   the response (`crawl_engine::pipeline::observation_hop`) — produced by
//!   requests the crawl's own politeness and rate limiting already governed.
//! - **No judgement of redirect status.** Whether an upgrade used 301 vs 302
//!   is `MixedProtocolRedirectValidator`'s finding (MIXPROT001); this
//!   analyzer only checks the destination scheme.
//! - **One finding per page.** Several bad `http://` URLs on one host still
//!   describe one host-level defect; the report layer collapses repeats.

use crate::analyzers::Analyzer;
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};

/// Passive HTTPS-redirect verification from recorded transport observations.
pub struct HttpsRedirectAnalyzer;

impl Default for HttpsRedirectAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpsRedirectAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HttpsRedirectAnalyzer {
    fn name(&self) -> &str {
        "https-redirect"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let url = ctx.page.url.as_str();

        // An http:// request that redirected to https:// is the good case:
        // silent. Anything else it did — stayed on http, hopped to another
        // scheme — is the defect. One finding per page: the first recorded
        // bad hop carries the evidence for the whole page.
        for hop in ctx.redirect_chain {
            if hop.from.scheme() != "http" {
                continue;
            }
            if hop.to.scheme() == "https" {
                return Vec::new();
            }
            return vec![Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "HTTPSRED001".to_string(),
                title: "HTTP URL did not redirect to HTTPS".to_string(),
                description: format!(
                    "The crawl requested {} over plain HTTP and was redirected to {}, \
                     which is not its HTTPS equivalent. Visitors following HTTP links \
                     are not upgraded to an encrypted connection.",
                    hop.from, hop.to
                ),
                url: url.to_string(),
                recommendation: "Redirect every http:// URL (301 or 308) to its \
                                 https:// equivalent, and serve no content over plain \
                                 HTTP."
                    .to_string(),
            }];
        }

        // No redirect was recorded for an http:// page. With a success status
        // that means the content was served over plain HTTP directly — no
        // redirect exists to upgrade it. Error responses (404s, redirects the
        // crawl recorded elsewhere) say nothing about host configuration and
        // are left alone.
        let served_directly_over_http = url.starts_with("http://")
            && ctx.redirect_chain.is_empty()
            && ctx.status_code.is_some_and(|s| (200..300).contains(&s));
        if served_directly_over_http {
            return vec![Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "HTTPSRED001".to_string(),
                title: "Page served over plain HTTP without redirect".to_string(),
                description: format!(
                    "The crawl requested {url} and received the response over plain \
                     HTTP with no redirect to HTTPS. The content is available to \
                     network observers in the clear."
                ),
                url: url.to_string(),
                recommendation: "Serve the site over HTTPS and 301-redirect every \
                                 http:// URL to its https:// equivalent."
                    .to_string(),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;
    use crate::RedirectHop;
    use url::Url;

    fn fixture_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
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

    fn hop(from: &str, to: &str, status: u16) -> RedirectHop {
        RedirectHop {
            from: Url::parse(from).expect("static test URL"),
            to: Url::parse(to).expect("static test URL"),
            status_code: status,
        }
    }

    fn ctx<'a>(
        page: &'a ParsedPage,
        chain: &'a [RedirectHop],
        status: Option<u16>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            redirect_chain: chain,
            status_code: status,
            ..AnalysisContext::new(page)
        }
    }

    #[test]
    fn https_page_with_no_redirect_is_silent() {
        let page = fixture_page("https://example.com/");
        let findings = HttpsRedirectAnalyzer::new().analyze(&ctx(&page, &[], Some(200)));
        assert!(
            findings.is_empty(),
            "a normal HTTPS page must produce no findings"
        );
    }

    #[test]
    fn http_upgraded_to_https_is_silent() {
        let page = fixture_page("http://example.com/");
        let chain = [hop("http://example.com/", "https://example.com/", 200)];
        let findings = HttpsRedirectAnalyzer::new().analyze(&ctx(&page, &chain, Some(200)));
        assert!(
            findings.is_empty(),
            "an http→https upgrade is correct behaviour: {findings:?}"
        );
    }

    #[test]
    fn http_redirecting_within_http_is_reported() {
        let page = fixture_page("http://example.com/old");
        let chain = [hop("http://example.com/old", "http://example.com/new", 200)];
        let findings = HttpsRedirectAnalyzer::new().analyze(&ctx(&page, &chain, Some(200)));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "HTTPSRED001");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(
            findings[0].description.contains("http://example.com/new"),
            "the destination must be quoted as evidence: {}",
            findings[0].description
        );
    }

    #[test]
    fn http_served_directly_is_reported() {
        let page = fixture_page("http://example.com/");
        let findings = HttpsRedirectAnalyzer::new().analyze(&ctx(&page, &[], Some(200)));
        assert_eq!(
            findings.len(),
            1,
            "plain-HTTP 200 is the defect: {findings:?}"
        );
        assert_eq!(findings[0].code, "HTTPSRED001");
    }

    #[test]
    fn http_error_response_is_not_judged() {
        // A 404 over HTTP says nothing about whether the host upgrades;
        // flagging it would be noise on every broken link.
        let page = fixture_page("http://example.com/missing");
        let findings = HttpsRedirectAnalyzer::new().analyze(&ctx(&page, &[], Some(404)));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn one_finding_despite_multiple_bad_hops() {
        let page = fixture_page("http://example.com/a");
        let chain = [
            hop("http://example.com/a", "http://example.com/b", 200),
            hop("http://example.com/b", "http://example.com/c", 200),
        ];
        let findings = HttpsRedirectAnalyzer::new().analyze(&ctx(&page, &chain, Some(200)));
        assert_eq!(
            findings.len(),
            1,
            "several bad hops describe one host-level defect"
        );
    }

    #[test]
    fn analyzer_name() {
        assert_eq!(HttpsRedirectAnalyzer::new().name(), "https-redirect");
    }
}
