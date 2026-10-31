//! Passive technology (stack) detection — GAP G7.
//!
//! # Why this exists
//!
//! Knowing what a site runs on (WordPress, Next.js, Shopify, nginx…) frames
//! every other finding: a WordPress-specific plugin path, a Next.js hydration
//! script, a Shopify cart cookie. Two competitors ship this; crawlkit reported
//! the server header only as a security disclosure warning and never as stack
//! inventory.
//!
//! # What is reported
//!
//! One Info finding per detected technology, code `tech:{name}`, with the
//! evidence that matched in the description. Signatures are compiled in below
//! and written from public knowledge (the Wappalyzer rules are GPL, so nothing
//! is copied from them).
//!
//! # What is deliberately NOT done
//!
//! - **No network requests.** Every rule matches data the crawl already
//!   stored: response headers, the raw body, Set-Cookie names. This analyzer
//!   cannot add a single request to a crawl, which is the G7 acceptance
//!   invariant.
//! - **No version extraction.** `server: nginx/1.24.0` detects nginx; the
//!   version belongs to `ServerHeaderAnalyzer`'s disclosure finding.
//! - **No fingerprinting beyond the curated table.** A signature that matches
//!   a common string on unrelated sites (react, jquery) would invent
//!   detections; needles here are distinctive paths, headers, and cookie
//!   prefixes. Lower confidence is recorded honestly rather than bumped.
//!
//! Signatures are deduplicated by technology name per page: a WordPress page
//! matching `wp-content` *and* the generator meta *and* a `wp-` cookie yields
//! one finding, and the strongest evidence in table order.

use std::collections::HashSet;

use crate::analyzers::is_auditable_as_document;
use crate::analyzers::Analyzer;
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};

/// How a signature matches data already stored from the crawl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchRule {
    /// A response header whose value contains a needle (case-insensitive).
    HeaderValue {
        /// Header to inspect (`server`, `x-powered-by`).
        header: &'static str,
        /// Lowercase needle the value must contain.
        contains: &'static str,
    },
    /// A lowercase substring that must appear in the raw HTML body.
    HtmlContains(&'static str),
    /// A needle the `<meta name="generator">` content must contain
    /// (case-insensitive).
    MetaGeneratorContains(&'static str),
    /// A Set-Cookie cookie name must start with this lowercase prefix.
    CookieName(&'static str),
}

/// The broad category a detected technology belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TechKind {
    /// Content management system.
    Cms,
    /// Static site generator.
    StaticSiteGenerator,
    /// Application framework (client or server).
    Framework,
    /// Server-side language or runtime.
    Language,
    /// E-commerce platform.
    Ecommerce,
    /// Web server software.
    WebServer,
    /// CDN / edge network.
    Cdn,
    /// Analytics or tag management.
    Analytics,
}

impl TechKind {
    fn as_str(self) -> &'static str {
        match self {
            TechKind::Cms => "CMS",
            TechKind::StaticSiteGenerator => "static site generator",
            TechKind::Framework => "framework",
            TechKind::Language => "language",
            TechKind::Ecommerce => "e-commerce",
            TechKind::WebServer => "web server",
            TechKind::Cdn => "CDN",
            TechKind::Analytics => "analytics",
        }
    }
}

/// How sure the signature is, stated in the finding rather than hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confidence {
    /// Distinctive evidence: a vendor path, a vendor-set header, a generator
    /// meta. False positives are essentially unknown.
    High,
    /// Shared or guessable evidence: a cookie prefix another product could
    /// set, a generic script name. Reported, but flagged.
    Medium,
}

impl Confidence {
    fn as_str(self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
        }
    }
}

/// One detection rule: a technology, what it looks like, and how sure a match
/// is. All fields are static so the whole table is a `const`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TechSignature {
    /// Display name, also lowercased into the finding code (`tech:{name}`).
    name: &'static str,
    /// Broad category.
    kind: TechKind,
    /// Where and what to match.
    rule: MatchRule,
    /// How sure a match is.
    confidence: Confidence,
}

const fn sig(
    name: &'static str,
    kind: TechKind,
    rule: MatchRule,
    confidence: Confidence,
) -> TechSignature {
    TechSignature {
        name,
        kind,
        rule,
        confidence,
    }
}

/// The signature table. Order is the report order for a page that matches
/// several technologies, and is part of the analyzer's determinism contract.
///
/// Written from public knowledge (vendor docs, common response headers and
/// asset paths). The Wappalyzer rule set is GPL-licensed and deliberately not
/// consulted — keeping this table original keeps crawlkit Apache-2.0 clean.
const TECH_SIGNATURES: &[TechSignature] = &[
    // --- Response headers: `server` -------------------------------------
    sig(
        "Nginx",
        TechKind::WebServer,
        MatchRule::HeaderValue {
            header: "server",
            contains: "nginx",
        },
        Confidence::High,
    ),
    sig(
        "Apache",
        TechKind::WebServer,
        MatchRule::HeaderValue {
            header: "server",
            contains: "apache",
        },
        Confidence::High,
    ),
    sig(
        "LiteSpeed",
        TechKind::WebServer,
        MatchRule::HeaderValue {
            header: "server",
            contains: "litespeed",
        },
        Confidence::High,
    ),
    sig(
        "Microsoft IIS",
        TechKind::WebServer,
        MatchRule::HeaderValue {
            header: "server",
            contains: "microsoft-iis",
        },
        Confidence::High,
    ),
    sig(
        "Cloudflare",
        TechKind::Cdn,
        MatchRule::HeaderValue {
            header: "server",
            contains: "cloudflare",
        },
        Confidence::High,
    ),
    // --- Response headers: `x-powered-by` -------------------------------
    sig(
        "PHP",
        TechKind::Language,
        MatchRule::HeaderValue {
            header: "x-powered-by",
            contains: "php",
        },
        Confidence::High,
    ),
    sig(
        "ASP.NET",
        TechKind::Framework,
        MatchRule::HeaderValue {
            header: "x-powered-by",
            contains: "asp.net",
        },
        Confidence::High,
    ),
    sig(
        "Express",
        TechKind::Framework,
        MatchRule::HeaderValue {
            header: "x-powered-by",
            contains: "express",
        },
        Confidence::High,
    ),
    sig(
        "Next.js",
        TechKind::Framework,
        MatchRule::HeaderValue {
            header: "x-powered-by",
            contains: "next.js",
        },
        Confidence::High,
    ),
    // --- HTML: `<meta name="generator">` --------------------------------
    sig(
        "WordPress",
        TechKind::Cms,
        MatchRule::MetaGeneratorContains("wordpress"),
        Confidence::High,
    ),
    sig(
        "Drupal",
        TechKind::Cms,
        MatchRule::MetaGeneratorContains("drupal"),
        Confidence::High,
    ),
    sig(
        "Joomla",
        TechKind::Cms,
        MatchRule::MetaGeneratorContains("joomla"),
        Confidence::High,
    ),
    sig(
        "Hugo",
        TechKind::StaticSiteGenerator,
        MatchRule::MetaGeneratorContains("hugo"),
        Confidence::High,
    ),
    sig(
        "Jekyll",
        TechKind::StaticSiteGenerator,
        MatchRule::MetaGeneratorContains("jekyll"),
        Confidence::High,
    ),
    sig(
        "Squarespace",
        TechKind::Cms,
        MatchRule::MetaGeneratorContains("squarespace"),
        Confidence::High,
    ),
    sig(
        "Webflow",
        TechKind::Cms,
        MatchRule::MetaGeneratorContains("webflow"),
        Confidence::High,
    ),
    // --- HTML: asset / script fingerprints ------------------------------
    sig(
        "WordPress",
        TechKind::Cms,
        MatchRule::HtmlContains("wp-content"),
        Confidence::High,
    ),
    sig(
        "WooCommerce",
        TechKind::Ecommerce,
        MatchRule::HtmlContains("/wp-content/plugins/woocommerce"),
        Confidence::High,
    ),
    sig(
        "Next.js",
        TechKind::Framework,
        MatchRule::HtmlContains("/_next/static"),
        Confidence::High,
    ),
    sig(
        "Nuxt",
        TechKind::Framework,
        MatchRule::HtmlContains("/_nuxt/"),
        Confidence::High,
    ),
    sig(
        "Shopify",
        TechKind::Ecommerce,
        MatchRule::HtmlContains("cdn.shopify.com"),
        Confidence::High,
    ),
    sig(
        "Google Tag Manager",
        TechKind::Analytics,
        MatchRule::HtmlContains("googletagmanager.com"),
        Confidence::High,
    ),
    sig(
        "Google Analytics",
        TechKind::Analytics,
        MatchRule::HtmlContains("google-analytics.com"),
        Confidence::High,
    ),
    // --- Cookie names (prefix match, lowercase) -------------------------
    sig(
        "WordPress",
        TechKind::Cms,
        MatchRule::CookieName("wp-"),
        Confidence::Medium,
    ),
    sig(
        "WooCommerce",
        TechKind::Ecommerce,
        MatchRule::CookieName("wp_woocommerce_session_"),
        Confidence::Medium,
    ),
    sig(
        "Shopify",
        TechKind::Ecommerce,
        MatchRule::CookieName("_shopify_"),
        Confidence::Medium,
    ),
    sig(
        "Cloudflare",
        TechKind::Cdn,
        MatchRule::CookieName("__cf_bm"),
        Confidence::Medium,
    ),
    sig(
        "Django",
        TechKind::Framework,
        MatchRule::CookieName("csrftoken"),
        Confidence::Medium,
    ),
    sig(
        "Laravel",
        TechKind::Framework,
        MatchRule::CookieName("laravel_session"),
        Confidence::Medium,
    ),
];

/// Value of the first `<meta name="generator">` tag, if any.
///
/// A deliberately small scanner rather than a full HTML parse: the parsed page
/// does not retain the generator attribute, so this re-reads the raw body.
/// Handles attribute order (`content` before or after `name`) and both quoted
/// and unquoted attribute values. Indices stay aligned between the lowercased
/// scan copy and the original because `to_ascii_lowercase` preserves byte
/// length.
fn meta_generator(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0usize;
    while let Some(rel) = lower[cursor..].find("<meta") {
        let tag_start = cursor + rel;
        let after = &lower[tag_start..];
        let tag_len = after.find('>').map_or(after.len(), |i| i + 1);
        let tag_lower = &lower[tag_start..tag_start + tag_len];
        if declares_generator(tag_lower) {
            let tag_orig = &html[tag_start..tag_start + tag_len];
            if let Some(value) = attr_value(tag_lower, tag_orig, "content") {
                return Some(value);
            }
        }
        if tag_len == after.len() {
            // Unterminated final tag: nothing further can match.
            break;
        }
        cursor = tag_start + tag_len;
    }
    None
}

/// Whether a lowercased `<meta …>` tag names the generator attribute.
fn declares_generator(tag_lower: &str) -> bool {
    const NAME: &str = "name=generator";
    if tag_lower.contains("name=\"generator\"") || tag_lower.contains("name='generator'") {
        return true;
    }
    // Unquoted: `name=generator` must end at whitespace or the tag close, so
    // `name=generatorfoo` does not count.
    tag_lower.match_indices(NAME).any(|(i, _)| {
        tag_lower[i + NAME.len()..]
            .chars()
            .next()
            .is_none_or(|c| c.is_whitespace() || c == '>' || c == '/')
    })
}

/// Value of `attr` in a `<meta>` tag, preserving case.
///
/// Matches against `tag_lower` (finds the attribute regardless of case) but
/// slices `tag_orig` at the same indices, which is safe because the lowercase
/// copy has identical byte length. Quoted and unquoted values are supported.
fn attr_value(tag_lower: &str, tag_orig: &str, attr: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!("{attr}={quote}");
        if let Some(rel) = tag_lower.find(&needle) {
            let start = rel + needle.len();
            let rest = &tag_orig[start..];
            let end = rest.find(quote)?;
            return Some(rest[..end].to_string());
        }
    }
    // Unquoted value: runs to whitespace, a slash, or the tag close.
    let needle = format!("{attr}=");
    let rel = tag_lower.find(&needle)?;
    let rest = &tag_orig[rel + needle.len()..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .unwrap_or(rest.len());
    (end > 0).then(|| rest[..end].to_string())
}

/// Case-insensitive `contains` for header values: lowercase both sides. The
/// needles in [`TECH_SIGNATURES`] are already lowercase.
fn header_matches(value: &str, needle: &str) -> bool {
    value.to_ascii_lowercase().contains(needle)
}

/// The first cookie name among the Set-Cookie headers that starts with
/// `prefix` (case-insensitive), with the matched name as evidence.
fn matching_cookie_name(headers: &[(String, String)], prefix: &str) -> Option<String> {
    for (name, value) in headers {
        if !name.eq_ignore_ascii_case("set-cookie") {
            continue;
        }
        let cookie_name = value.split('=').next().unwrap_or("").trim();
        if cookie_name.to_ascii_lowercase().starts_with(prefix) {
            return Some(cookie_name.to_string());
        }
    }
    None
}

/// The first value of `header` (case-insensitive name lookup).
fn header_value<'a>(headers: &'a [(String, String)], header: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(header))
        .map(|(_, v)| v.as_str())
}

impl MatchRule {
    /// Whether this rule reads the raw HTML body, and so only applies to
    /// responses auditable as HTML documents.
    fn reads_html(self) -> bool {
        matches!(
            self,
            MatchRule::HtmlContains(_) | MatchRule::MetaGeneratorContains(_)
        )
    }

    /// Test this rule against the stored response data. Returns the human
    /// readable evidence on a match — the finding shows what was seen, so a
    /// reported technology can be verified by hand.
    fn match_context(self, ctx: &AnalysisContext) -> Option<String> {
        match self {
            MatchRule::HeaderValue { header, contains } => {
                let value = header_value(ctx.headers, header)?;
                header_matches(value, contains)
                    .then(|| format!("the `{header}: {value}` response header"))
            }
            MatchRule::HtmlContains(needle) => {
                let body = ctx.body?;
                body.to_ascii_lowercase()
                    .contains(needle)
                    .then(|| format!("an HTML reference to `{needle}`"))
            }
            MatchRule::MetaGeneratorContains(needle) => {
                let body = ctx.body?;
                let generator = meta_generator(body)?;
                header_matches(&generator, needle)
                    .then(|| format!("the generator meta tag (`{generator}`)"))
            }
            MatchRule::CookieName(prefix) => {
                let name = matching_cookie_name(ctx.headers, prefix)?;
                Some(format!("the `{name}` cookie"))
            }
        }
    }
}

/// Passive technology (stack) detection from data the crawl already stored.
pub struct TechDetectionAnalyzer;

impl Default for TechDetectionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TechDetectionAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TechDetectionAnalyzer {
    fn name(&self) -> &str {
        "tech-detection"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let url = ctx.page.url.to_string();
        // HTML fingerprints mean nothing on a stylesheet or a PDF. Header and
        // cookie evidence is media-independent and still applies.
        let html_auditable = is_auditable_as_document(ctx.content_type);

        let mut findings = Vec::new();
        let mut detected: HashSet<&'static str> = HashSet::new();
        for signature in TECH_SIGNATURES {
            if detected.contains(&signature.name) {
                // A stronger rule earlier in the table already reported it.
                continue;
            }
            if signature.rule.reads_html() && !html_auditable {
                continue;
            }
            let Some(evidence) = signature.rule.match_context(ctx) else {
                // No match here does not consume the name: a later rule for
                // the same technology may still match.
                continue;
            };
            detected.insert(signature.name);
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Custom("tech".to_string()),
                code: format!("tech:{}", signature.name.to_lowercase()),
                title: format!("{} detected ({})", signature.name, signature.kind.as_str()),
                description: format!(
                    "This page runs {name}, a {kind}, identified from {evidence}. \
                     Detection is passive: the signature matched data already \
                     stored from the crawl response, so no additional request \
                     was made. Confidence: {confidence}.",
                    name = signature.name,
                    kind = signature.kind.as_str(),
                    confidence = signature.confidence.as_str(),
                ),
                url: url.clone(),
                recommendation: "No action required. Technology findings build a \
                                 stack inventory for the site; review the listed \
                                 evidence only if the technology is unexpected."
                    .to_string(),
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    /// A minimal parsed page. No parser run: detection must work from the
    /// raw body and headers alone.
    fn fixture_page() -> ParsedPage {
        ParsedPage {
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

    /// A context with the given headers/body; content type defaults to HTML.
    fn ctx_with<'a>(
        page: &'a ParsedPage,
        headers: &'a [(String, String)],
        body: Option<&'a str>,
        content_type: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            headers,
            body,
            content_type,
            ..AnalysisContext::new(page)
        }
    }

    fn leaked_headers(pairs: &[(&str, &str)]) -> &'static [(String, String)] {
        Box::leak(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }

    #[test]
    fn plain_page_reports_nothing() {
        let page = fixture_page();
        let headers = leaked_headers(&[]);
        let ctx = ctx_with(&page, headers, Some("<html><body>hi</body></html>"), None);
        assert!(
            TechDetectionAnalyzer::new().analyze(&ctx).is_empty(),
            "a page with no signatures must produce no findings"
        );
    }

    #[test]
    fn server_header_detects_nginx() {
        let page = fixture_page();
        let headers = leaked_headers(&[("Server", "nginx/1.24.0")]);
        let ctx = ctx_with(&page, headers, None, None);
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "tech:nginx");
        assert_eq!(found[0].severity, Severity::Info);
        assert!(
            found[0].description.contains("`server: nginx/1.24.0`"),
            "evidence must quote the header: {}",
            found[0].description
        );
    }

    #[test]
    fn x_powered_by_detects_php_case_insensitively() {
        let page = fixture_page();
        let headers = leaked_headers(&[("X-Powered-By", "PHP/8.2.1")]);
        let ctx = ctx_with(&page, headers, None, None);
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "tech:php");
    }

    #[test]
    fn generator_meta_detects_wordpress() {
        let page = fixture_page();
        let headers = leaked_headers(&[]);
        let html = "<html><head><meta name=\"generator\" \
                    content=\"WordPress 6.4.3\"></head></html>";
        let ctx = ctx_with(&page, headers, Some(html), None);
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "tech:wordpress");
        assert!(
            found[0].description.contains("WordPress 6.4.3"),
            "evidence must quote the generator: {}",
            found[0].description
        );
    }

    #[test]
    fn generator_meta_handles_attribute_order_and_single_quotes() {
        let page = fixture_page();
        let headers = leaked_headers(&[]);
        let html = "<html><head><meta content='Hugo 0.111.0' name='generator'>\
                    </head></html>";
        let ctx = ctx_with(&page, headers, Some(html), None);
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        assert_eq!(found.len(), 1, "content-before-name must still match");
        assert_eq!(found[0].code, "tech:hugo");
    }

    #[test]
    fn asset_path_detects_wordpress_without_generator() {
        // The regression this guards: a name consumed by a non-matching rule
        // must not hide a later rule for the same technology.
        let page = fixture_page();
        let headers = leaked_headers(&[]);
        let html = "<html><body><link rel='stylesheet' \
                    href='/wp-content/themes/a/style.css'></body></html>";
        let ctx = ctx_with(&page, headers, Some(html), None);
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "tech:wordpress");
    }

    #[test]
    fn one_finding_per_technology_even_with_three_signals() {
        let page = fixture_page();
        let headers = leaked_headers(&[("Set-Cookie", "wp-settings-1=1; Path=/")]);
        let html = "<html><head><meta name=\"generator\" content=\"WordPress\">\
                    </head><body><script src=\"/wp-content/plugins/x.js\">\
                    </script></body></html>";
        let ctx = ctx_with(&page, headers, Some(html), None);
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        assert_eq!(
            found.len(),
            1,
            "generator, asset path and cookie are one CMS"
        );
        assert_eq!(found[0].code, "tech:wordpress");
        // The strongest rule — the generator meta — provides the evidence.
        assert!(found[0].description.contains("generator meta tag"));
    }

    #[test]
    fn next_js_asset_path() {
        let page = fixture_page();
        let headers = leaked_headers(&[]);
        let html = "<html><body><script src=\"/_next/static/chunks/main.js\">\
                    </script></body></html>";
        let ctx = ctx_with(&page, headers, Some(html), None);
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "tech:next.js");
    }

    #[test]
    fn shopify_strongest_rule_wins_over_cookie() {
        let page = fixture_page();
        let headers = leaked_headers(&[("Set-Cookie", "_shopify_s=abc; Path=/")]);
        let html = "<html><body><script \
                    src=\"https://cdn.shopify.com/shopifycloud/.js\"></script>\
                    </body></html>";
        let ctx = ctx_with(&page, headers, Some(html), None);
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        assert_eq!(found.len(), 1, "cdn script and cookie are one platform");
        assert_eq!(found[0].code, "tech:shopify");
        assert!(
            found[0].description.contains("cdn.shopify.com"),
            "the high-confidence rule supplies the evidence: {}",
            found[0].description
        );
    }

    #[test]
    fn cookie_prefix_detects_django_at_medium_confidence() {
        let page = fixture_page();
        let headers = leaked_headers(&[("Set-Cookie", "csrftoken=AbCdEf; Path=/")]);
        let ctx = ctx_with(&page, headers, None, None);
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "tech:django");
        assert!(found[0].description.contains("Confidence: medium"));
        assert!(found[0].description.contains("`csrftoken` cookie"));
    }

    #[test]
    fn html_fingerprints_skip_non_documents_but_headers_do_not() {
        let page = fixture_page();
        let headers = leaked_headers(&[("Server", "nginx"), ("Set-Cookie", "csrftoken=x; Path=/")]);
        let html = "<html><script src=\"/_next/static/x.js\"></script></html>";
        let ctx = ctx_with(&page, headers, Some(html), Some("image/png"));
        let found = TechDetectionAnalyzer::new().analyze(&ctx);
        let codes: Vec<&str> = found.iter().map(|f| f.code.as_str()).collect();
        assert!(
            !codes.contains(&"tech:next.js"),
            "HTML fingerprints must not fire on a PNG: {codes:?}"
        );
        assert_eq!(
            codes,
            vec!["tech:nginx", "tech:django"],
            "header and cookie evidence is media-independent: {codes:?}"
        );
    }

    #[test]
    fn output_order_is_deterministic() {
        let page = fixture_page();
        let headers = leaked_headers(&[("Server", "cloudflare"), ("Set-Cookie", "__cf_bm=x")]);
        let html = "<html><body><script src=\"/_next/static/a.js\"></script>\
                    <script src=\"https://cdn.shopify.com/b.js\"></script>\
                    </body></html>";
        let first = {
            let ctx = ctx_with(&page, headers, Some(html), None);
            TechDetectionAnalyzer::new().analyze(&ctx)
        };
        let second = {
            let ctx = ctx_with(&page, headers, Some(html), None);
            TechDetectionAnalyzer::new().analyze(&ctx)
        };
        assert_eq!(first.len(), 3, "next.js, shopify, cloudflare expected");
        // Table order: header rules precede HTML asset rules, so the
        // Cloudflare `server` header is reported first.
        let codes: Vec<&str> = first.iter().map(|f| f.code.as_str()).collect();
        assert_eq!(
            codes,
            vec!["tech:cloudflare", "tech:next.js", "tech:shopify"]
        );
        // `Finding` is not `PartialEq`; compare the fields that carry the
        // analyzer's output (codes, titles, descriptions, evidence).
        let signature = |findings: &[Finding]| -> Vec<(String, String, String)> {
            findings
                .iter()
                .map(|f| (f.code.clone(), f.title.clone(), f.description.clone()))
                .collect()
        };
        assert_eq!(signature(&first), signature(&second));
    }

    #[test]
    fn generator_scanner_ignores_non_generator_metas() {
        assert_eq!(
            meta_generator("<meta name=\"viewport\" content=\"width=1\">"),
            None
        );
        assert_eq!(
            meta_generator("<meta name=\"generator\" content=\"WordPress 6\">"),
            Some("WordPress 6".to_string())
        );
        // `name=generatorfoo` must not count as an unquoted generator.
        assert_eq!(meta_generator("<meta name=generatorfoo content=x>"), None);
        assert_eq!(
            meta_generator("<meta name=generator content=x>"),
            Some("x".to_string())
        );
    }
}
