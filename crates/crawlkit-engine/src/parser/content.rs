//! Head, meta, and text-content extraction.

use scraper::{Html, Selector};
use url::Url;

use super::selectors;
use super::{Heading, HtmlParser, MetaTags};
use crate::meta::{HreflangTag, OpenGraphTags, TwitterTags};

impl HtmlParser {
    // ---------------------------------------------------------------------------
    // Meta tags
    // ---------------------------------------------------------------------------
    pub(super) fn extract_meta(document: &Html, page_url: &Url) -> MetaTags {
        let mut title = Self::select_text(document, "title");

        // Fall back to <meta property="og:title"> or <meta name="twitter:title">
        if title.is_none() {
            title = Self::get_meta_content(document, "og:title");
        }
        if title.is_none() {
            title = Self::get_meta_content(document, "twitter:title");
        }

        let description = Self::get_meta_content(document, "description")
            .or_else(|| Self::get_meta_content(document, "og:description"))
            .or_else(|| Self::get_meta_content(document, "twitter:description"));

        let canonical = Self::get_attr(document, "link[rel=canonical]", "href")
            .and_then(|href| page_url.join(&href).ok());

        let robots = Self::get_meta_content(document, "robots");
        let language = Self::get_attr(document, "html", "lang")
            .or_else(|| Self::get_meta_content(document, "language"));
        let charset = Self::get_attr(document, "meta[charset]", "charset").or_else(|| {
            Self::get_meta_content(document, "content-type").and_then(|ct| {
                ct.split(';')
                    .find(|p| p.trim().starts_with("charset="))
                    .map(|p| p.trim().trim_start_matches("charset=").to_string())
            })
        });
        let viewport = Self::get_meta_content(document, "viewport");

        let og = OpenGraphTags {
            title: Self::get_meta_content(document, "og:title"),
            description: Self::get_meta_content(document, "og:description"),
            image: Self::get_meta_content(document, "og:image"),
            url: Self::get_meta_content(document, "og:url"),
            r#type: Self::get_meta_content(document, "og:type"),
            site_name: Self::get_meta_content(document, "og:site_name"),
            locale: Self::get_meta_content(document, "og:locale"),
        };

        let twitter = TwitterTags {
            card: Self::get_meta_content(document, "twitter:card"),
            site: Self::get_meta_content(document, "twitter:site"),
            creator: Self::get_meta_content(document, "twitter:creator"),
            title: Self::get_meta_content(document, "twitter:title"),
            description: Self::get_meta_content(document, "twitter:description"),
            image: Self::get_meta_content(document, "twitter:image"),
            image_alt: Self::get_meta_content(document, "twitter:image:alt"),
        };

        let hreflang = Self::extract_hreflang(document, page_url);

        MetaTags {
            title,
            description,
            canonical,
            robots,
            language,
            charset,
            viewport,
            og,
            twitter,
            hreflang,
        }
    }

    pub(super) fn extract_hreflang(document: &Html, page_url: &Url) -> Vec<HreflangTag> {
        let selector = selectors::link_hreflang();

        document
            .select(selector)
            .filter_map(|el| {
                let lang = el.value().attr("hreflang")?.to_string();
                let href = el.value().attr("href")?;
                let url = page_url.join(href).ok()?;
                Some(HreflangTag { lang, url })
            })
            .collect()
    }

    // ---------------------------------------------------------------------------
    // Headings
    // ---------------------------------------------------------------------------
    pub(super) fn extract_headings(document: &Html) -> Vec<Heading> {
        let mut headings = Vec::new();
        for level in 1..=6 {
            let selector = match Selector::parse(&format!("h{level}")) {
                Ok(s) => s,
                Err(_) => continue,
            };
            for el in document.select(&selector) {
                let text: String = el.text().collect::<Vec<_>>().join("").trim().to_string();
                if !text.is_empty() {
                    headings.push(Heading {
                        level,
                        length: text.len(),
                        text,
                    });
                }
            }
        }
        headings
    }

    // ---------------------------------------------------------------------------
    // Text statistics
    // ---------------------------------------------------------------------------
    /// Word and sentence counts over the same visible-text corpus.
    ///
    /// Both values come from a single tree walk (body text minus
    /// script/style/noscript/svg) so they are consistent by construction —
    /// averaging words by sentences from different corpora produces
    /// meaningless ratios.
    pub(super) fn count_text_stats(document: &Html) -> (usize, usize) {
        let body = selectors::body();
        let script = selectors::script();
        let style = selectors::style();
        let noscript = selectors::noscript();

        // Collect the set of node IDs to skip (script/style/noscript elements).
        let mut skip_ids = std::collections::HashSet::new();
        for sel in [script, style, noscript] {
            for el in document.select(sel) {
                skip_ids.insert(el.id());
            }
        }

        let root = document
            .select(body)
            .next()
            .map(|el| el.id())
            .unwrap_or_else(|| document.root_element().id());

        let mut text = String::new();
        let tree = &document.tree;

        fn collect_text(
            tree: &ego_tree::Tree<scraper::Node>,
            node_id: ego_tree::NodeId,
            skip: &std::collections::HashSet<ego_tree::NodeId>,
            text: &mut String,
        ) {
            let Some(node) = tree.get(node_id) else {
                return; // Node not found; skip safely
            };
            match node.value() {
                scraper::Node::Element(el) => {
                    let tag = el.name();
                    if tag == "script" || tag == "style" || tag == "noscript" || tag == "svg" {
                        return;
                    }
                }
                scraper::Node::Text(t) => {
                    text.push_str(t);
                    text.push(' ');
                }
                _ => {}
            }
            for child_id in node.children() {
                let child_id = child_id.id();
                if !skip.contains(&child_id) {
                    collect_text(tree, child_id, skip, text);
                }
            }
        }

        collect_text(tree, root, &skip_ids, &mut text);

        let words = text.split_whitespace().filter(|w| !w.is_empty()).count();
        let sentences = count_sentence_runs(&text);
        (words, sentences)
    }
}

/// Count sentence-ending runs in visible text.
///
/// A run of consecutive `.`, `!`, or `?` characters (e.g. `...`, `!?`)
/// counts as a single sentence terminator; a trailing run only counts if
/// the text actually ends with one.
fn count_sentence_runs(text: &str) -> usize {
    let mut count = 0usize;
    let mut in_terminator = false;
    for c in text.chars() {
        if c == '.' || c == '!' || c == '?' {
            in_terminator = true;
        } else if in_terminator {
            count += 1;
            in_terminator = false;
        }
    }
    if in_terminator {
        count += 1;
    }
    count
}
