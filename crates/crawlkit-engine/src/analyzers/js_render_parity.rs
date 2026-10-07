//! Content that exists only after JavaScript runs — invisible to most crawlers.
//!
//! # Why this matters more each quarter
//!
//! Of the six major web crawlers, only **Googlebot** and **AppleBot** execute
//! JavaScript. `GPTBot`, `ClaudeBot`, `PerplexityBot` and `CCBot` fetch raw HTML
//! only. A page whose content is injected client-side is therefore fully present
//! for Google and completely invisible to every AI answer engine — which is the
//! surface where a growing share of discovery now happens.
//!
//! Industry audit data puts the raw-versus-rendered mismatch on roughly a third
//! of sites, most often JavaScript hydration that leaves text out of the served
//! HTML.
//!
//! # What this checks
//!
//! Given a page that was fetched *and* rendered, compare what the two documents
//! actually expose:
//!
//! | Signal | Meaning |
//! |---|---|
//! | Text in rendered, absent from raw | Content no non-JS crawler can read |
//! | Links in rendered, absent from raw | Navigation no non-JS crawler can follow |
//! | Structured data injected by JS | Rich results depend on hydration succeeding |
//! | Title or description rewritten by JS | The SERP snippet differs per crawler |
//!
//! Each is a *different* problem with a *different* fix, so they are separate
//! codes rather than one combined "parity" finding.
//!
//! # What this deliberately does not do
//!
//! It does not report "the page needs SSR" as advice. Client-side rendering is a
//! legitimate architecture; the defect is specifically that content a crawler
//! needs is absent from the served HTML. A fully client-rendered page whose text
//! happens to be in the HTML has no defect here.
//!
//! Comparison is on normalized text and on absolute link paths, so attribute
//! ordering, quoting and whitespace differences between the two documents do not
//! register as changes.

#[cfg(feature = "full")]
use std::collections::BTreeSet;

#[cfg(feature = "full")]
use crate::analyzers::html_text::{attr_value, decode_entities, strip_tags};
use crate::analyzers::Analyzer;
#[cfg(feature = "full")]
use crate::parser::HtmlParser;
// The analyzer's whole body is behind `feature = "full"`, because
// `AnalysisContext::rendered` is `Option<&RenderedPage>` with it and `Option<&()>`
// without. Importing unconditionally would warn on every `core` build, and the
// thresholds below are equally only read under that feature.
#[cfg(feature = "full")]
#[cfg(feature = "full")]
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};

/// Text added by JavaScript that a non-JS crawler cannot read.
pub const JSRENDER_TEXT: &str = "JSRENDER-TEXT-MISSING";
/// Links added by JavaScript that a non-JS crawler cannot follow.
pub const JSRENDER_LINKS: &str = "JSRENDER-LINKS-MISSING";
/// Structured data that only exists after hydration.
pub const JSRENDER_SCHEMA: &str = "JSRENDER-SCHEMA-MISSING";
/// A `<title>` or meta description that JavaScript overwrites.
pub const JSRENDER_META: &str = "JSRENDER-META-REWRITTEN";

/// Fraction of visible text that must appear in the served HTML before the
/// difference is reported.
///
/// Set low deliberately. A page that leaks a cookie banner or a "skip to content"
/// link into the rendered DOM has a small text delta that means nothing, and a
/// rule that fires on that trains users to ignore it. Content that is *entirely*
/// client-rendered is unambiguous, and the threshold catches that while ignoring
/// chrome.
#[cfg(feature = "full")]
const TEXT_DELTA_THRESHOLD: f64 = 0.20;

/// Word-count floor below which missing text is noise rather than missing content.
///
/// Applied to the words present only in the rendered document. Three leaked
/// words are a cookie banner; thirty are a page nobody can read without running
/// scripts.
#[cfg(feature = "full")]
const MIN_SIGNIFICANT_WORDS: usize = 25;

/// Lowercased, punctuation-stripped words, for comparing two documents.
#[cfg(feature = "full")]
fn words(html: &str) -> BTreeSet<String> {
    // Strip script and style wholesale: their contents are not visible text and
    // a framework's inline bundle or theme stylesheet would otherwise dominate the
    // comparison.
    //
    // Whichever of the two comes *first in the document* must be handled first.
    // Preferring `<script` and only then looking for `<style` skips an earlier
    // stylesheet entirely, because a single `find` over the whole remainder reports
    // the later `<script>` and the earlier `<style>` is never visited. On
    // wyattsnotes.wyattau.com the theme stylesheet precedes the first script, so
    // its CSS was counted as page text and a real 12% delta was reported as 35%.
    let mut cleaned = String::with_capacity(html.len());
    let mut rest = html;
    loop {
        let lower = rest.to_ascii_lowercase();
        let script = lower.find("<script").map(|i| (i, "</script>"));
        let style = lower.find("<style").map(|i| (i, "</style>"));
        // Earliest position wins; `>` breaks a tie so `<script>` nests are handled
        // consistently.
        let next = match (script, style) {
            (Some((s, se)), Some((t, te))) => {
                if s <= t {
                    Some((s, se))
                } else {
                    Some((t, te))
                }
            }
            (Some(x), None) | (None, Some(x)) => Some(x),
            (None, None) => None,
        };
        let Some((start, close)) = next else {
            cleaned.push_str(rest);
            break;
        };
        cleaned.push_str(&rest[..start]);
        let after = &rest[start..];
        // An unterminated element means the rest of the document is that
        // element's body, so there is nothing left to keep.
        match lower[start..].find(close) {
            Some(end) => rest = &after[end + close.len()..],
            None => break,
        }
    }

    let stripped = strip_tags(&cleaned);
    stripped
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|w| w.len() > 1)
        .collect()
}

/// Remove tags and decode the handful of entities that affect word comparison.
///
/// A naive `<[^>]*>` strip is wrong for real HTML: attribute *values* may legally
/// contain `>`, and minifiers emit them unquoted-but-quoted-in-value constantly —
/// Starlight ships `<html style="--wn-line-height: 1.7; … a > b …">`. Stripping to
/// the first `>` ends the "tag" mid-attribute and the remainder leaks out as if it
/// were page text. On wyattsnotes.wyattau.com that leaked the whole theme
/// stylesheet (`monokai`, `papercolor`, `0px`, `auto`, …) into the word comparison
/// and inflated a 12% real delta into a reported 35%.
///
/// Origin-relative link targets, so `/a` and `https://host/a` compare equal.
///
/// The host is deliberately dropped. Both documents are versions of the *same*
/// page, so scheme and host are constant across them; keeping the host would
/// instead let the resolution base leak into the comparison, because a relative
/// href resolves against the base while an absolute one carries its own. Resolving
/// both against `base_url` and then discarding the origin is what makes the two
/// forms meet.
#[cfg(feature = "full")]
fn link_paths(html: &str, base_url: &url::Url) -> BTreeSet<String> {
    let parsed = HtmlParser::parse(html, base_url);
    parsed
        .links
        .iter()
        .filter_map(|l| l.href.parse::<url::Url>().ok())
        .map(|u| {
            let mut s = u.path().to_string();
            if let Some(q) = u.query() {
                s.push('?');
                s.push_str(q);
            }
            s
        })
        .collect()
}

/// Number of parseable JSON-LD blocks, used to detect schema injected by hydration.
///
/// Delegates to [`HtmlParser`] rather than searching for the string
/// `application/ld+json`. A substring search cannot tell a real script element from
/// the same text appearing inside a JavaScript string -- which is how a client-side
/// schema injector writes it. Counting those made a served page with no structured
/// data at all look as though it already had one, suppressing the finding.
///
/// It also discards unparseable blocks, so a malformed block is not counted as
/// present on either side.
#[cfg(feature = "full")]
fn parseable_jsonld_count(html: &str, base_url: &url::Url) -> usize {
    HtmlParser::parse(html, base_url).structured_data.len()
}

/// Read an attribute value from a start tag, quoted or not.
///
/// Extract `<title>` and the meta description, for rewrite detection.
#[cfg(feature = "full")]
fn head_signatures(html: &str) -> (Option<String>, Option<String>) {
    let lower = html.to_ascii_lowercase();
    let head_end = lower.find("</head>").unwrap_or(lower.len());
    let head = &html[..head_end.min(html.len())];

    let title = {
        let l = head.to_ascii_lowercase();
        l.find("<title").and_then(|s| {
            let after = &l[s..];
            let gt = after.find('>')? + s + 1;
            let close = after[after.find('>')? + 1..].find("</title>")? + gt;
            // Decoded here rather than at each comparison site: a caller that
            // forgets to decode sees `Wyatt&#39;s Notes` differ from `Wyatt's Notes`
            // and reports a rewrite that never happened.
            Some(decode_entities(head[gt..close].trim()))
        })
    };

    // Every `<meta>` tag is scanned rather than splitting on a literal
    // `name="description"`: attribute order and quoting both vary between the
    // served and the rendered document, and a literal split matches only one shape.
    let description = head.match_indices("<meta").find_map(|(at, _)| {
        let rest = &head[at..];
        let end = rest.find('>')?;
        let tag = &rest[..end];
        if attr_value(tag, "name").as_deref() == Some("description") {
            attr_value(tag, "content").map(|v| decode_entities(&v))
        } else {
            None
        }
    });
    (title, description)
}

pub struct JsRenderParityAnalyzer;

impl Default for JsRenderParityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl JsRenderParityAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for JsRenderParityAnalyzer {
    fn name(&self) -> &str {
        "js-render-parity"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        // Feature-gated: `rendered` is a `()` without the `full` feature. The
        // `cfg` blocks are block expressions rather than a `return`, so the
        // non-`full` build has no reachable `return` to warn about.
        #[cfg(not(feature = "full"))]
        {
            let _ = ctx;
            Vec::new()
        }
        #[cfg(feature = "full")]
        {
            let Some(rendered) = ctx.rendered else {
                // No render happened, so there is nothing to compare. Reporting
                // "content may be hidden" without having rendered would be a
                // guess, and guessing is what this whole product exists to avoid.
                return Vec::new();
            };
            // The served document is NOT `ctx.body`. The pipeline replaces its working
            // copy of the body with the rendered DOM whenever a page is rendered,
            // so `ctx.body` and `rendered.html` are the same string and comparing
            // them always reports "no difference". The served response body is
            // carried on `RenderedPage::source_html`.
            //
            // When a render happened but the served body was not retained, the
            // comparison is not merely degraded, it is impossible: there is no
            // second document. Returning nothing is correct here, and asserting a
            // difference would be arithmetic on one value.
            let Some(raw) = rendered.source_html.as_deref() else {
                return Vec::new();
            };

            let url = ctx.page.url.to_string();
            let mut out = Vec::new();

            // --- text ---
            let raw_words = words(raw);
            let rendered_words = words(&rendered.html);
            let missing: Vec<&String> = rendered_words.difference(&raw_words).collect();
            let rendered_total = rendered_words.len();

            // The floor is on the *missing* words, not on the page's total.
            //
            // These two gates are not equivalent. Counting missing words directly
            // is stricter, and the difference is entirely in the small-page case:
            // a 10-word cookie banner on a 40-word page is a 25% ratio, which a
            // ratio-only gate reports as missing content, and which is obviously
            // not. A ratio expresses how much, which on a short page says nothing
            // about whether what is missing is content or chrome. An absolute
            // count of unreadable words is the quantity that carries that meaning.
            if missing.len() >= MIN_SIGNIFICANT_WORDS && rendered_total > 0 {
                let delta = missing.len() as f64 / rendered_total as f64;
                if delta >= TEXT_DELTA_THRESHOLD {
                    let sample: Vec<&str> = missing.iter().take(8).map(|s| s.as_str()).collect();
                    out.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Seo,
                        code: JSRENDER_TEXT.to_string(),
                        title: "Content only exists after JavaScript runs".to_string(),
                        description: format!(
                            "{:.0}% of the rendered page's text ({}/{} words) is absent from the \
                             served HTML. Crawlers that do not execute JavaScript -- which \
                             includes GPTBot, ClaudeBot, PerplexityBot and CCBot -- cannot read \
                             it. Example words: {}.",
                            delta * 100.0,
                            missing.len(),
                            rendered_total,
                            sample.join(", ")
                        ),
                        url: url.clone(),
                        recommendation: "Server-render the primary content, or prerender it at \
                                         build time, so it is present in the HTML response. \
                                         Client-only rendering hides it from every non-JS \
                                         crawler."
                            .to_string(),
                    });
                }
            }

            // --- links ---
            let Ok(page_url) = url::Url::parse(&url) else {
                return out;
            };
            let raw_links = link_paths(raw, &page_url);
            let rendered_links = link_paths(&rendered.html, &page_url);
            let missing_links: Vec<&String> = rendered_links.difference(&raw_links).collect();
            if !missing_links.is_empty() {
                let sample: Vec<&str> = missing_links.iter().take(5).map(|s| s.as_str()).collect();
                out.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Links,
                    code: JSRENDER_LINKS.to_string(),
                    title: "Links only exist after JavaScript runs".to_string(),
                    description: format!(
                        "{} link(s) present in the rendered DOM are absent from the served \
                         HTML, so a non-JS crawler cannot discover or follow them: {}.",
                        missing_links.len(),
                        sample.join(", ")
                    ),
                    url: url.clone(),
                    recommendation: "Render navigation and body links into the HTML response. \
                                     Crawler-discovered links are how a page gets indexed at \
                                     all; links that appear only after hydration are invisible."
                        .to_string(),
                });
            }

            // --- structured data ---
            let raw_ld = parseable_jsonld_count(raw, &page_url);
            let rendered_ld = parseable_jsonld_count(&rendered.html, &page_url);
            if rendered_ld > raw_ld {
                out.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: JSRENDER_SCHEMA.to_string(),
                    title: "Structured data is injected by JavaScript".to_string(),
                    description: format!(
                        "The rendered DOM contains {rendered_ld} JSON-LD block(s) against {raw_ld} \
                         in the served HTML. Structured data that depends on hydration \
                         succeeding is unavailable whenever it does not."
                    ),
                    url: url.clone(),
                    recommendation: "Emit JSON-LD in the server-rendered HTML rather than \
                                     injecting it client-side."
                        .to_string(),
                });
            }

            // --- meta rewritten ---
            let (raw_title, raw_desc) = head_signatures(raw);
            let (rendered_title, rendered_desc) = head_signatures(&rendered.html);
            let title_changed = match (&raw_title, &rendered_title) {
                (Some(a), Some(b)) => normalize_ws(a) != normalize_ws(b),
                _ => false,
            };
            let desc_changed = match (&raw_desc, &rendered_desc) {
                (Some(a), Some(b)) => normalize_ws(a) != normalize_ws(b),
                _ => false,
            };
            if title_changed || desc_changed {
                let mut what = Vec::new();
                if title_changed {
                    what.push("title");
                }
                if desc_changed {
                    what.push("description");
                }
                out.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: JSRENDER_META.to_string(),
                    title: "JavaScript rewrites the page metadata".to_string(),
                    description: format!(
                        "The {} differs between the served HTML and the rendered DOM, so the \
                         SERP snippet depends on whether the crawler runs JavaScript. Search \
                         engines that do not will show the served-HTML version.",
                        what.join(" and ")
                    ),
                    url,
                    recommendation: "Set the final title and description in the HTML response \
                                     rather than rewriting them after load."
                        .to_string(),
                });
            }

            out
        }
    }
}

#[cfg(feature = "full")]
fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// Gated on the feature the analyzer actually does work under: `AnalysisContext`
// carries `Option<&RenderedPage>` with `full` and `Option<&()>` without it, so
// the fixtures cannot be built otherwise.
#[cfg(all(test, feature = "full"))]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;
    use url::Url;

    /// Raw HTML with no content; hydration fills it in. The archetypal case.
    const RAW_EMPTY: &str = r#"<!doctype html><html lang="en"><head>
        <title>Shop</title><meta name="description" content="A shop">
        </head><body><div id="root"></div>
        <script src="/app.js"></script></body></html>"#;

    /// The same page after hydration: text, links and JSON-LD all client-side.
    ///
    /// The paragraph runs to real paragraph length deliberately. A fixture of three
    /// words would sit under the analyzer's floor and prove nothing about whether
    /// the rule fires on a realistic page.
    const RENDERED_FULL: &str = r#"<!doctype html><html lang="en"><head>
        <title>Shop</title><meta name="description" content="A shop">
        </head><body><div id="root">
        <h1>Our Products</h1>
        <p>We sell carefully sourced goods for every occasion, delivered worldwide.
        Each piece is chosen for how it wears over years rather than seasons, and
        every order is packed by hand at our workshop before it leaves.</p>
        <p>Returns are free within thirty days, and our team answers questions
        directly rather than through a ticket queue.</p>
        <a href="/products">Browse the catalogue</a>
        <script type="application/ld+json">{"@type":"Store"}</script>
        </div></body></html>"#;

    /// Leaked so a context can borrow it for `'static`; these are test fixtures
    /// built a handful of times per run.
    fn base() -> url::Url {
        url::Url::parse("https://shop.example/").unwrap()
    }

    fn page() -> &'static ParsedPage {
        Box::leak(Box::new(ParsedPage {
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
        }))
    }

    /// Builds a context that mirrors what the crawl pipeline actually produces.
    ///
    /// The important detail: `body` is the **rendered** document, not the served
    /// one. `pipeline::Pipeline::analyze` does `*body_text = page.html.clone()` as
    /// soon as a page is rendered, so by the time analyzers run the working body has
    /// been replaced by the post-script DOM. A harness that put the served document
    /// in `body` would model a pipeline that does not exist, and every test written
    /// against it would pass while the analyzer reported nothing in production --
    /// which is exactly what happened before this fixture was corrected.
    fn ctx_with<'a>(served: &'a str, rendered: Option<&'a str>) -> AnalysisContext<'a> {
        let page: &'static ParsedPage = page();
        let dom = rendered.unwrap_or(served);
        AnalysisContext {
            page,
            // The pipeline overwrites this with the rendered DOM.
            body: Some(dom),
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
            rendered: rendered.map(|h| {
                Box::leak(Box::new(crate::playwright::RenderedPage {
                    final_url: "https://shop.example/".to_string(),
                    html: h.to_string(),
                    // Mirrors the pipeline: `source_html` is the served response
                    // body, while `html` and `body` are both what the DOM looked
                    // like afterwards.
                    source_html: Some(served.to_string()),
                    console_messages: vec![],
                    network_requests: vec![],
                    wasm_errors: vec![],
                    page_errors: vec![],
                    render_time: std::time::Duration::from_millis(10),
                    memory_used: 0,
                })) as &crate::playwright::RenderedPage
            }),
        }
    }

    fn codes(served: &str, rendered: Option<&str>) -> Vec<String> {
        JsRenderParityAnalyzer::new()
            .analyze(&ctx_with(served, rendered))
            .into_iter()
            .map(|f| f.code)
            .collect()
    }

    /// The whole point of the analyzer: content that only exists after hydration
    /// is invisible to every crawler that does not run JavaScript.
    #[test]
    fn client_rendered_content_is_reported() {
        let found = codes(RAW_EMPTY, Some(RENDERED_FULL));
        assert!(
            found.contains(&JSRENDER_TEXT.to_string()),
            "text injected by JS must be reported, got {found:?}"
        );
        assert!(
            found.contains(&JSRENDER_LINKS.to_string()),
            "links injected by JS must be reported, got {found:?}"
        );
        assert!(
            found.contains(&JSRENDER_SCHEMA.to_string()),
            "JSON-LD injected by JS must be reported, got {found:?}"
        );
    }

    /// A server-rendered page has nothing to report, however it is built.
    #[test]
    fn server_rendered_page_reports_nothing() {
        let served = r#"<!doctype html><html lang="en"><head><title>Shop</title>
            <meta name="description" content="A shop"></head><body>
            <h1>Our Products</h1>
            <p>We sell carefully sourced goods for every occasion, delivered worldwide.</p>
            <a href="/products">Browse the catalogue</a>
            <script type="application/ld+json">{"@type":"Store"}</script>
            </body></html>"#;
        assert_eq!(codes(served, Some(served)), Vec::<String>::new());
    }

    /// No render means no comparison. Reporting "content may be hidden" without
    /// having rendered would be a guess.
    #[test]
    fn no_render_reports_nothing() {
        assert_eq!(codes(RAW_EMPTY, None), Vec::<String>::new());
    }

    /// Attribute-order and quoting differences between the two documents are not
    /// content changes.
    #[test]
    fn markup_differences_are_not_content_differences() {
        let a = r#"<!doctype html><html><head><title>T</title></head><body>
            <p class="x">Hello there friend</p><a href="/a">Link text</a></body></html>"#;
        let b = r#"<!DOCTYPE html><HTML><HEAD><TITLE>T</TITLE></HEAD><BODY>
            <p class='y'>Hello there friend</p><a href='/a'>Link text</a></BODY></HTML>"#;
        assert_eq!(codes(a, Some(b)), Vec::<String>::new());
    }

    /// Script and style contents are not visible text. A framework's inline
    /// bundle must not read as "missing content".
    #[test]
    fn script_and_style_bodies_are_not_counted_as_text() {
        let html = r#"<html><body><script>
            var a = 1; var b = 2; function widget(){return alpha beta gamma delta}
            </script><style>.x{color:red}</style></body></html>"#;
        assert!(words(html).is_empty(), "got {:?}", words(html));
    }

    /// A cookie banner leaking into the rendered DOM is chrome, not content.
    #[test]
    fn small_text_deltas_are_ignored() {
        let raw = r#"<html><head><title>T</title></head><body>
            <p>alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi</p>
            </body></html>"#;
        let rendered = format!("{raw}<div>Accept cookies</div>");
        let found = codes(raw, Some(&rendered));
        assert!(
            !found.contains(&JSRENDER_TEXT.to_string()),
            "a 3-word delta must not fire, got {found:?}"
        );
    }

    /// Below the word floor there is nothing meaningful to compare.
    #[test]
    fn tiny_pages_are_ignored() {
        let raw = "<html><head><title>T</title></head><body></body></html>";
        let rendered = "<html><head><title>T</title></head><body><p>hi</p></body></html>";
        assert_eq!(codes(raw, Some(rendered)), Vec::<String>::new());
    }

    /// Links are compared as origin-relative paths, so the same link written
    /// relative in one document and absolute in the other still matches. If the
    /// host were part of the key this pair would differ and every such link would
    /// be reported as "added by JavaScript".
    #[test]
    fn relative_and_absolute_links_match() {
        let base = Url::parse("https://shop.example/").unwrap();
        let relative = link_paths(r#"<a href="/x">x</a>"#, &base);
        let absolute = link_paths(r#"<a href="https://shop.example/x">x</a>"#, &base);
        assert!(relative.contains("/x"), "got {relative:?}");
        assert_eq!(relative, absolute);
    }

    /// The floor is an absolute count of unreadable words, so a short page's small
    /// ratio cannot inflate chrome into missing content. A 3-word cookie banner on
    /// a 4-word page is a 75% delta and is still not a defect.
    #[test]
    fn short_pages_do_not_inflate_chrome_into_missing_content() {
        // 3 of 4 words unreadable: 75%, far over the ratio threshold.
        let raw = "<html><head><title>t</title></head><body><p>alpha</p></body></html>";
        let rendered =
            "<html><head><title>t</title></head><body><p>alpha</p><p>Accept cookies now</p></body></html>";
        let found = codes(raw, Some(rendered));
        assert!(
            !found.contains(&JSRENDER_TEXT.to_string()),
            "a 75% ratio on 4 words is chrome, not missing content; got {found:?}"
        );
    }

    /// The other side: a page whose content genuinely does not exist in the
    /// served HTML is reported once the unreadable word count clears the floor.
    #[test]
    fn missing_content_is_reported_once_the_count_clears_the_floor() {
        let raw = r#"<html><head><title>T</title></head><body><div id="root"></div></body></html>"#;
        let body = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima \
                    mike november oscar papa quebec romeo sierra tango uniform victor whiskey \
                    xray yankee zulu";
        let rendered = format!(
            r#"<html><head><title>T</title></head><body><div id="root"><p>{body}</p></div></body></html>"#
        );
        assert!(
            codes(raw, Some(&rendered)).contains(&JSRENDER_TEXT.to_string()),
            "26 unreadable words must be reported"
        );
    }

    /// Leakage of page chrome -- a cookie banner -- is a handful of words and
    /// must stay silent. This is the other side of the floor above.
    #[test]
    fn chrome_leakage_does_not_fire() {
        let raw = r#"<html><head><title>T</title></head><body>
            <p>alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike
            november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee
            zulu</p></body></html>"#;
        let rendered = format!("{raw}<div>Accept cookies</div>");
        assert!(!codes(raw, Some(&rendered)).contains(&JSRENDER_TEXT.to_string()));
    }

    /// A metadata rewrite is a real, separate defect: the snippet a crawler sees
    /// depends on whether it runs JavaScript.
    #[test]
    fn client_side_metadata_rewrite_is_reported() {
        let raw = r#"<html><head><title>Shop</title>
            <meta name="description" content="A shop"></head><body><p>hi</p></body></html>"#;
        let rendered = r#"<html><head><title>Shop | Official</title>
            <meta name="description" content="A shop"></head><body><p>hi</p></body></html>"#;
        let found = codes(raw, Some(rendered));
        assert!(found.contains(&JSRENDER_META.to_string()), "got {found:?}");
    }

    /// The contract the pipeline owes this analyzer. If `source_html` is absent
    /// there is no served document to compare against, so the analyzer must stay
    /// silent rather than report a difference computed from one value.
    #[test]
    fn missing_source_html_reports_nothing() {
        let page = page();
        let ctx = AnalysisContext {
            page,
            body: Some(RENDERED_FULL),
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
            rendered: Some(Box::leak(Box::new(crate::playwright::RenderedPage {
                final_url: "https://shop.example/".to_string(),
                html: RENDERED_FULL.to_string(),
                source_html: None,
                console_messages: vec![],
                network_requests: vec![],
                wasm_errors: vec![],
                page_errors: vec![],
                render_time: std::time::Duration::ZERO,
                memory_used: 0,
            })) as &crate::playwright::RenderedPage),
        };
        let found = JsRenderParityAnalyzer::new().analyze(&ctx);
        assert!(
            found.is_empty(),
            "with no served document there is nothing to compare; got {:?}",
            found.iter().map(|f| &f.code).collect::<Vec<_>>()
        );
    }

    /// Regression: the pipeline overwrites the body with the rendered DOM, so
    /// `ctx.body` is the *rendered* document once a page is rendered. Comparing
    /// `ctx.body` against `rendered.html` compares a value with itself and can
    /// never report a difference. This pins the correct source.
    #[test]
    fn analysis_reads_the_served_body_not_the_overwritten_one() {
        let served =
            r#"<html><head><title>T</title></head><body><div id="root"></div></body></html>"#;
        let ctx = ctx_with(served, Some(RENDERED_FULL));
        assert_eq!(
            ctx.body,
            Some(RENDERED_FULL),
            "the harness must reproduce the pipeline, where ctx.body is the rendered DOM"
        );
        assert_ne!(
            ctx.body,
            ctx.rendered.map(|r| r.source_html.as_deref().unwrap()),
            "otherwise this test cannot tell the two sources apart"
        );
        let found = JsRenderParityAnalyzer::new()
            .analyze(&ctx)
            .into_iter()
            .map(|f| f.code)
            .collect::<Vec<_>>();
        assert!(
            found.contains(&JSRENDER_TEXT.to_string()),
            "must read RenderedPage::source_html, not ctx.body; got {found:?}"
        );
    }

    /// A real browser's before-and-after for the same page, captured independently
    /// of this analyzer. `served.html` is the response body; `dom.html` is Chrome's
    /// post-script DOM. See `tests/fixtures/jsrender/PROVENANCE.md`.
    ///
    /// This is the test that would have caught the analyzer reading `ctx.body`
    /// instead of `RenderedPage::source_html`: the pipeline overwrites `ctx.body`
    /// with the rendered DOM, so the analyzer saw one document compared with
    /// itself and reported nothing on a page whose entire content is injected by
    /// script. Hand-written fixtures passed throughout, because the author of a
    /// fixture already knows which half is supposed to differ.
    #[test]
    fn real_browser_capture_is_detected() {
        const SERVED: &str = include_str!("../../tests/fixtures/jsrender/served.html");
        const DOM: &str = include_str!("../../tests/fixtures/jsrender/dom.html");

        // Precondition: the capture really does differ, so a pass cannot come
        // from the two halves being accidentally equal. Compared on *visible
        // text*, not on bytes -- the injected markup is written inside the
        // `<script>` source, so a raw substring check finds it in both documents
        // and would assert nothing.
        assert_ne!(SERVED, DOM, "the fixture pair must actually differ");
        assert!(
            words(DOM).contains("carefully") && !words(SERVED).contains("carefully"),
            "the injected copy must be visible text in the rendered DOM only; \
             served has {:?}",
            words(SERVED)
        );

        let found = codes(SERVED, Some(DOM));
        assert!(
            found.contains(&JSRENDER_TEXT.to_string()),
            "injected text must be reported; got {found:?}"
        );
        assert!(
            found.contains(&JSRENDER_LINKS.to_string()),
            "injected links must be reported; got {found:?}"
        );
        assert!(
            found.contains(&JSRENDER_SCHEMA.to_string()),
            "injected JSON-LD must be reported; got {found:?}"
        );
        assert!(
            found.contains(&JSRENDER_META.to_string()),
            "the script-rewritten title must be reported; got {found:?}"
        );
    }

    /// The same capture, presented as a page that was never rendered. Nothing
    /// should be claimed about content crawlkit did not observe.
    #[test]
    fn real_capture_without_a_render_reports_nothing() {
        const SERVED: &str = include_str!("../../tests/fixtures/jsrender/served.html");
        assert!(codes(SERVED, None).is_empty());
    }

    /// Minified output drops attribute quotes. A matcher written against
    /// `name="description"` therefore finds nothing on Astro/Next.js builds --
    /// which are exactly the sites most likely to be client-rendered, so the check
    /// would report "no rewrite" for the population it most needs to cover.
    ///
    /// Third time this session a grep assuming quoted attributes has misled me, so
    /// it is pinned here rather than left to inspection.
    #[test]
    fn metadata_is_read_from_minified_unquoted_attributes() {
        let minified = concat!(
            r#"<html><head><meta charset=utf-8>"#,
            r#"<meta content="A shop for tools and hardware." name=description>"#,
            r#"<title>Shop</title></head><body><p>hi</p></body></html>"#
        );
        let (_, desc) = head_signatures(minified);
        assert_eq!(
            desc.as_deref(),
            Some("A shop for tools and hardware."),
            "unquoted attribute values must be read"
        );
    }

    /// The same tag with quotes, and attributes in both orders.
    #[test]
    fn metadata_is_read_from_quoted_attributes_in_any_order() {
        let a = r#"<meta name="description" content="Hello there">"#;
        let b = r#"<meta content="Hello there" name="description">"#;
        for tag in [a, b] {
            let head = format!("<html><head>{tag}</head><body></body></html>");
            let (_, desc) = head_signatures(&head);
            assert_eq!(desc.as_deref(), Some("Hello there"), "failed for {tag}");
        }
    }

    /// `name` must not match inside a longer attribute name.
    #[test]
    fn attribute_name_matching_requires_a_delimiter() {
        assert_eq!(attr_value(r#"<meta data-name="description""#, "name"), None);
        assert_eq!(attr_value(r#"<meta hostname="description""#, "name"), None);
        assert_eq!(
            attr_value(r#"<meta name="description""#, "name").as_deref(),
            Some("description")
        );
    }

    /// An unquoted value ends at whitespace, and must not swallow the next attribute.
    #[test]
    fn unquoted_values_terminate_at_whitespace() {
        let tag = "<meta content=Hello name=description>";
        assert_eq!(attr_value(tag, "content").as_deref(), Some("Hello"));
    }

    /// A `charset` or `viewport` meta must not be mistaken for the description.
    #[test]
    fn non_description_meta_tags_are_ignored() {
        let head = r#"<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"></head><body></body></html>"#;
        assert_eq!(head_signatures(head).1, None);
    }

    /// A JSON-LD script written as *text inside* another script is not structured
    /// data. Client-side injectors build it exactly this way, and counting the
    /// string made a served page with no schema at all look like it already had
    /// one -- which suppresses the very finding this analyzer exists to raise.
    #[test]
    fn jsonld_inside_a_javascript_string_is_not_structured_data() {
        let injector = r#"<html><head><title>T</title></head><body><div id="root"></div>
            <script>
              const html = '<script type="application/ld+json">{"@type":"Store"}<\/script>';
              document.getElementById('root').innerHTML = html;
            </script></body></html>"#;
        assert_eq!(
            parseable_jsonld_count(injector, &base()),
            0,
            "a schema literal inside JS source is not a served JSON-LD block"
        );
        // And once actually injected, it is one.
        let after = r#"<html><head><title>T</title></head><body><div id="root">
            <script type="application/ld+json">{"@type":"Store"}</script>
            </div></body></html>"#;
        assert_eq!(parseable_jsonld_count(after, &base()), 1);
        assert!(
            codes(injector, Some(after)).contains(&JSRENDER_SCHEMA.to_string()),
            "schema injected by script must still be reported"
        );
    }

    /// `>` inside a quoted attribute value must not end the tag.
    ///
    /// Starlight ships `<html style="--wn-line-height: 1.7; … a > b …">`. A
    /// strip-to-first-`>` ends the tag mid-attribute, and everything after the
    /// stray `>` — an entire theme stylesheet — is then counted as page text. On
    /// wyattsnotes.wyattau.com that turned a 12% real delta into a reported 35%.
    #[test]
    fn angle_bracket_inside_an_attribute_value_does_not_leak_text() {
        let html = r#"<html style="--line-height: 1.7; --x: a > b"><body><p>Real prose here.</p></body></html>"#;
        let found = words(html);
        assert!(
            found.contains("prose"),
            "the paragraph must be visible text; got {found:?}"
        );
        for leaked in ["line", "height", "style"] {
            assert!(
                !found.contains(leaked),
                "attribute contents must not become text ({leaked} leaked); got {found:?}"
            );
        }
    }

    /// Comments must not contribute words either.
    #[test]
    fn comment_text_is_not_page_text() {
        let html = "<body><p>Real prose.</p><!-- buildId: 9f2a hidden comment --></body>";
        let found = words(html);
        assert!(found.contains("prose"));
        assert!(
            !found.contains("hidden") && !found.contains("buildid"),
            "comment text leaked; got {found:?}"
        );
    }

    /// An entity-encoded title and its decoded literal are the same title. Without
    /// decoding, minifiers that switch between the two make every page look as
    /// though a script rewrote its metadata — which is how this false positive
    /// reached a live audit on two real pages.
    #[test]
    fn entity_encoded_and_literal_titles_are_the_same_title() {
        let encoded = r#"<html><head><title>Wyatt&#39;s Notes | Free</title></head><body><p>hi</p></body></html>"#;
        let literal = r#"<html><head><title>Wyatt's Notes | Free</title></head><body><p>hi</p></body></html>"#;
        let (a, b) = (head_signatures(encoded).0, head_signatures(literal).0);
        assert_eq!(a, b, "entity and literal forms must compare equal");
        assert!(
            !codes(encoded, Some(literal)).contains(&JSRENDER_META.to_string()),
            "an entity change is not a metadata rewrite"
        );
    }

    /// The same must hold for descriptions, in both attribute orders.
    #[test]
    fn entity_encoded_and_literal_descriptions_are_the_same_description() {
        let a = r#"<meta name="description" content="Tom &amp; Jerry &quot;quoted&quot;">"#;
        let b = r#"<meta content='Tom & Jerry "quoted"' name=description>"#;
        let head = |tag: &str| format!("<html><head>{tag}</head><body></body></html>");
        assert_eq!(
            head_signatures(&head(a)).1,
            head_signatures(&head(b)).1,
            "descriptions must decode before comparison"
        );
    }

    /// A stylesheet appearing *before* the first script must still be removed.
    ///
    /// The original loop searched the whole remainder for `<script` before ever
    /// looking for `<style`, so a page whose stylesheet precedes its first script
    /// kept the stylesheet and counted its CSS as page text. Starlight's theme
    /// stylesheet sits before its scripts, which turned a 12% real delta into a
    /// reported 35% on two live pages.
    #[test]
    fn a_stylesheet_before_the_first_script_is_removed() {
        let html = "<html><head><style>.x{align-items:center;padding:.001em}</style></head>\
                   <body><p>Real prose.</p><script>var a=1;</script></body></html>";
        let found = words(html);
        assert!(found.contains("prose"), "got {found:?}");
        for css in ["alignitems", "padding", "001em"] {
            assert!(
                !found.contains(css),
                "{css} leaked from the stylesheet: {found:?}"
            );
        }
    }

    /// The inverse order, and interleaving, must behave identically.
    #[test]
    fn script_and_style_removal_is_order_independent() {
        let a = "<style>.a{margin:0}</style><script>var x=1;</script><p>Prose here.</p>";
        let b = "<script>var x=1;</script><style>.a{margin:0}</style><p>Prose here.</p>";
        let c =
            "<p>Prose.</p><style>.a{margin:0}</style><script>var x=1;</script><style>.b{}</style>";
        for html in [a, b, c] {
            let found = words(html);
            assert!(
                found.contains("prose"),
                "prose missing for {html}: {found:?}"
            );
            assert!(
                !found.contains("margin"),
                "css leaked for {html}: {found:?}"
            );
        }
        assert_eq!(words(a), words(b), "order must not change the result");
    }

    /// Adjacent inline elements must not merge into one token.
    ///
    /// Without a separator at tag boundaries, `<button>简体中文</button><button>Paper
    /// </button>` yields `简体中文paper`, and the comparison then reports a document
    /// as containing a word it never had. That is how `themepaperdarklightsepia`
    /// came to be "missing" from a page that never contained it.
    #[test]
    fn adjacent_inline_elements_do_not_merge_into_one_word() {
        let html =
            "<div><button>简体中文</button><button>Paper</button><button>Dark</button></div>";
        let found = words(html);
        assert!(
            found.contains("paper") && found.contains("dark"),
            "got {found:?}"
        );
        for glued in ["简体中文paper", "paperdark", "themepaper"] {
            assert!(
                !found.contains(glued),
                "{glued} was fabricated; got {found:?}"
            );
        }
    }

    /// Real adjacent prose must still read as separate words, not one token.
    #[test]
    fn adjacent_text_nodes_stay_separate() {
        let html = "<p>hello</p><p>world</p><span>again</span>";
        let found = words(html);
        for w in ["hello", "world", "again"] {
            assert!(found.contains(w), "{w} missing from {found:?}");
        }
        assert!(!found.contains("helloworld"), "got {found:?}");
    }

    #[test]
    fn jsonld_block_counting_handles_multiple_blocks() {
        assert_eq!(
            parseable_jsonld_count(
                r#"<script type="application/ld+json">{"@type":"Store"}</script>"#,
                &base()
            ),
            1
        );
        assert_eq!(
            parseable_jsonld_count(
                r#"<script type="application/ld+json">{"@type":"Store"}</script>
                   <script type="application/ld+json">{"@type":"WebPage"}</script>"#,
                &base()
            ),
            2
        );
        assert_eq!(parseable_jsonld_count("<p>none</p>", &base()), 0);
    }
}
