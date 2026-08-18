//! Hyperlink discovery and extraction.

use scraper::Html;
use url::Url;

use super::selectors;
use super::ExtractedLink;
use super::HtmlParser;

impl HtmlParser {
    // ---------------------------------------------------------------------------
    // Links
    // ---------------------------------------------------------------------------
    pub(super) fn extract_links(document: &Html, page_url: &Url) -> Vec<ExtractedLink> {
        let selector = selectors::anchor_href();

        let page_domain = page_url.domain().unwrap_or("");

        document
            .select(selector)
            .filter_map(|el| {
                let raw_href = el.value().attr("href")?.to_string();

                // Skip Cloudflare email obfuscation links
                if raw_href.contains("/cdn-cgi/l/email-protection") {
                    return None;
                }

                let text: String = el.text().collect::<Vec<_>>().join("").trim().to_string();

                let rel: Vec<String> = el
                    .value()
                    .attr("rel")
                    .map(|r| r.split_whitespace().map(String::from).collect())
                    .unwrap_or_default();

                // Resolve relative URLs against the page URL
                let resolved_url = page_url.join(&raw_href).ok()?;
                let href = resolved_url.to_string();

                let is_external = resolved_url.domain().unwrap_or("") != page_domain;

                // Accessibility: extract aria-label and img alt for link text analysis
                let aria_label = el.value().attr("aria-label").map(String::from);
                let img_alt = el
                    .select(selectors::img())
                    .next()
                    .and_then(|img| img.value().attr("alt"))
                    .map(String::from);

                Some(ExtractedLink {
                    href,
                    text,
                    rel,
                    is_external,
                    aria_label,
                    img_alt,
                })
            })
            .collect()
    }
}

/// Extracts links from a partial or complete HTML document.
///
/// Used internally by [`HtmlParser::parse_stream`] to extract links from
/// accumulated HTML chunks before the full document is available.
pub(super) struct LinkExtractor<'a> {
    base_url: &'a Url,
    page_domain: &'a str,
    seen: std::collections::HashSet<String>,
}

impl<'a> LinkExtractor<'a> {
    pub(super) fn new(base_url: &'a Url) -> Self {
        Self {
            base_url,
            page_domain: base_url.domain().unwrap_or(""),
            seen: std::collections::HashSet::new(),
        }
    }

    /// Extract links from the document, deduplicating by href.
    pub(super) fn extract_links(&mut self, document: &Html) -> Vec<ExtractedLink> {
        let selector = selectors::anchor_href();

        document
            .select(selector)
            .filter_map(|el| {
                let raw_href = el.value().attr("href")?.to_string();

                if raw_href.contains("/cdn-cgi/l/email-protection") {
                    return None;
                }

                let resolved_url = self.base_url.join(&raw_href).ok()?;
                let href = resolved_url.to_string();

                if !self.seen.insert(href.clone()) {
                    return None;
                }

                let text: String = el.text().collect::<Vec<_>>().join("").trim().to_string();

                let rel: Vec<String> = el
                    .value()
                    .attr("rel")
                    .map(|r| r.split_whitespace().map(String::from).collect())
                    .unwrap_or_default();

                let is_external = resolved_url.domain().unwrap_or("") != self.page_domain;

                let aria_label = el.value().attr("aria-label").map(String::from);
                let img_alt = el
                    .select(selectors::img())
                    .next()
                    .and_then(|img| img.value().attr("alt"))
                    .map(String::from);

                Some(ExtractedLink {
                    href,
                    text,
                    rel,
                    is_external,
                    aria_label,
                    img_alt,
                })
            })
            .collect()
    }
}
