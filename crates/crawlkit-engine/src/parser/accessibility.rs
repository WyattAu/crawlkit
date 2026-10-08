//! WCAG-oriented landmark, ARIA, and table-structure extraction.

use scraper::Html;

use super::selectors;
use super::HtmlParser;

impl HtmlParser {
    // ---------------------------------------------------------------------------
    // Accessibility data extraction
    // ---------------------------------------------------------------------------

    /// Returns (landmarks, has_skip_link, has_main, has_nav, has_positive_tabindex,
    /// tabindex_neg_count, aria_role_count, aria_label_count, has_lang, html_lang,
    /// has_aria_hidden, tables_with_headers, tables_total, tables_with_captions).
    pub(super) fn extract_accessibility(
        document: &Html,
        html_raw: Option<&str>,
    ) -> (
        Vec<String>,
        bool,
        bool,
        bool,
        bool,
        usize,
        usize,
        usize,
        bool,
        Option<String>,
        bool,
        usize,
        usize,
        usize,
    ) {
        let mut landmarks = Vec::new();
        let mut has_skip_link = false;
        let mut has_main = false;
        let mut has_nav = false;
        let mut has_positive_tabindex = false;
        let mut tabindex_negative_count = 0usize;
        let mut aria_role_count = 0usize;
        let mut aria_label_count = 0usize;
        let mut has_aria_hidden = false;
        let mut tables_with_headers = 0usize;
        let mut tables_total = 0usize;
        let mut tables_with_captions = 0usize;

        // Detect html lang from the raw HTML string directly (scraper's
        // Element.attr() is unreliable). The value must come from the `<html>`
        // start tag itself: searching the document for `lang="` also matches
        // `hreflang="` and `data-lang="`, which reported a language a page never
        // declared and silenced the missing-lang check on exactly the pages that
        // declare alternates.
        let mut has_lang = false;
        let mut html_lang: Option<String> = None;
        if let Some(raw) = html_raw {
            if let Some(lang) = html_tag_lang(raw) {
                has_lang = true;
                html_lang = Some(lang);
            }
        }

        // Landmark detection via semantic HTML elements
        if document.select(selectors::header()).next().is_some() {
            landmarks.push("banner".to_string());
        }
        if document.select(selectors::nav()).next().is_some() {
            landmarks.push("navigation".to_string());
            has_nav = true;
        }
        if document.select(selectors::main()).next().is_some() {
            landmarks.push("main".to_string());
            has_main = true;
        }
        if document.select(selectors::aside()).next().is_some() {
            landmarks.push("complementary".to_string());
        }
        if document.select(selectors::footer()).next().is_some() {
            landmarks.push("contentinfo".to_string());
        }
        if document.select(selectors::form()).next().is_some() {
            landmarks.push("form".to_string());
        }
        if document.select(selectors::section_aria()).next().is_some() {
            landmarks.push("region".to_string());
        }
        if document.select(selectors::role_banner()).next().is_some()
            && !landmarks.contains(&"banner".to_string())
        {
            landmarks.push("banner".to_string());
        }
        if document
            .select(selectors::role_navigation())
            .next()
            .is_some()
            && !landmarks.contains(&"navigation".to_string())
        {
            landmarks.push("navigation".to_string());
            has_nav = true;
        }
        if document.select(selectors::role_main()).next().is_some()
            && !landmarks.contains(&"main".to_string())
        {
            landmarks.push("main".to_string());
            has_main = true;
        }
        if document
            .select(selectors::role_complementary())
            .next()
            .is_some()
            && !landmarks.contains(&"complementary".to_string())
        {
            landmarks.push("complementary".to_string());
        }
        if document
            .select(selectors::role_contentinfo())
            .next()
            .is_some()
            && !landmarks.contains(&"contentinfo".to_string())
        {
            landmarks.push("contentinfo".to_string());
        }

        // Skip link: first <a> whose href starts with "#" and contains "skip" in text or class
        for el in document.select(selectors::anchor_fragment()) {
            let text: String = el.text().collect::<Vec<_>>().join("").to_lowercase();
            let class = el.value().attr("class").unwrap_or("").to_lowercase();
            let id = el.value().attr("href").unwrap_or("").to_lowercase();
            if text.contains("skip") || class.contains("skip") || id.contains("skip") {
                has_skip_link = true;
                break;
            }
        }

        // Tabindex and ARIA attribute scanning
        for node_ref in document.root_element().descendants() {
            let el = match node_ref.value() {
                scraper::Node::Element(e) => e,
                _ => continue,
            };
            // tabindex
            if let Some(ti) = el.attr("tabindex") {
                if let Ok(val) = ti.parse::<i32>() {
                    if val > 0 {
                        has_positive_tabindex = true;
                    } else if val == -1 {
                        tabindex_negative_count += 1;
                    }
                }
            }

            // ARIA roles
            if el.attr("role").is_some() {
                aria_role_count += 1;
            }

            // ARIA labels
            if el.attr("aria-label").is_some() || el.attr("aria-labelledby").is_some() {
                aria_label_count += 1;
            }

            // aria-hidden
            if el.attr("aria-hidden") == Some("true") {
                has_aria_hidden = true;
            }
        }

        // Table accessibility
        let table_sel = selectors::table();
        let th_sel = selectors::th();
        let caption_sel = selectors::caption();
        for table in document.select(table_sel) {
            tables_total += 1;
            if table.select(th_sel).next().is_some() {
                tables_with_headers += 1;
            }
            if table.select(caption_sel).next().is_some() {
                tables_with_captions += 1;
            }
        }

        (
            landmarks,
            has_skip_link,
            has_main,
            has_nav,
            has_positive_tabindex,
            tabindex_negative_count,
            aria_role_count,
            aria_label_count,
            has_lang,
            html_lang,
            has_aria_hidden,
            tables_with_headers,
            tables_total,
            tables_with_captions,
        )
    }
}

/// The `lang` attribute of the `<html>` start tag, if it declares one.
///
/// Three constraints make this stricter than a document-wide search:
///
/// - only the `<html>` start tag is considered, so `hreflang="en"` or
///   `data-lang="en"` elsewhere in the document cannot satisfy it
/// - the attribute name is matched case-insensitively, because `LANG="EN"` is
///   valid HTML and was previously invisible, which reported a missing lang on
///   a page that had one
/// - the value may be double-quoted, single-quoted, or unquoted; the scan for
///   the tag's closing `>` honours quoting, because attribute values may contain
///   `>` (CSS custom properties do)
///
/// Returns `None` when there is no `<html>` tag or it declares no `lang`.
fn html_tag_lang(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    let mut i = 0usize;
    let open = loop {
        let rel = lower[i..].find("<html")?;
        let at = i + rel;
        let after = at + 5;
        // A name boundary, so `<htmlparser>`-style prefixes do not match.
        let ok = match lower.as_bytes().get(after) {
            None => true,
            Some(b) => matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'>'),
        };
        if ok {
            break after;
        }
        i = after;
    };

    // The `>` that closes the start tag, honouring quoted attribute values.
    let mut quote: Option<char> = None;
    let mut tag_end = lower.len();
    for (idx, c) in lower[open..].char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                q @ ('"' | '\'') => quote = Some(q),
                '>' => {
                    tag_end = open + idx;
                    break;
                }
                _ => {}
            },
        }
    }
    let tag = &raw[open..tag_end];

    // Searched in the tag's own lowercase text: `lower` is the whole document,
    // and indexing a document offset into the tag is how the first version of
    // this looked up the wrong bytes and reported every page as missing a lang.
    let tag_lower = tag.to_ascii_lowercase();
    let bytes = tag.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = tag_lower[from..].find("lang") {
        let at = from + rel;
        let delimited = at > 0 && matches!(bytes[at - 1], b' ' | b'\t' | b'\n' | b'\r' | b'/');
        let after = at + 4;
        if delimited && matches!(bytes.get(after), Some(b'=')) {
            let rest = &tag[after + 1..];
            return match rest.chars().next() {
                Some(q @ ('"' | '\'')) => {
                    let body = &rest[q.len_utf8()..];
                    let end = body.find(q)?;
                    Some(body[..end].to_string())
                }
                Some(_) => rest
                    .split(|c: char| c == '>' || c.is_whitespace())
                    .next()
                    .map(String::from),
                None => None,
            };
        }
        from = at + 4;
    }
    None
}
