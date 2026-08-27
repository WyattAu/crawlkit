//! Script, stylesheet, and structured-data (JSON-LD) extraction.

use scraper::Html;

use super::selectors;
use super::HtmlParser;
use super::{ScriptInfo, StructuredData, StyleInfo};

impl HtmlParser {
    // ---------------------------------------------------------------------------
    // Scripts
    // ---------------------------------------------------------------------------
    pub(super) fn extract_scripts(document: &Html) -> Vec<ScriptInfo> {
        let selector = selectors::script();

        document
            .select(selector)
            .map(|el| {
                let src = el.value().attr("src").map(String::from);
                let r#async = el.value().attr("async").is_some();
                let defer = el.value().attr("defer").is_some();
                let script_type = el.value().attr("type").map(String::from);
                let has_integrity = el.value().attr("integrity").is_some();

                ScriptInfo {
                    src,
                    r#async,
                    defer,
                    script_type,
                    has_integrity,
                }
            })
            .collect()
    }

    // ---------------------------------------------------------------------------
    // Styles
    // ---------------------------------------------------------------------------
    pub(super) fn extract_styles(document: &Html) -> Vec<StyleInfo> {
        let mut styles = Vec::new();

        // External stylesheets via <link rel="stylesheet">
        let link_sel = selectors::link_stylesheet();

        for el in document.select(link_sel) {
            let href = el.value().attr("href").map(String::from);
            let media = el.value().attr("media").map(String::from);
            let has_integrity = el.value().attr("integrity").is_some();

            styles.push(StyleInfo {
                href,
                media,
                is_inline: false,
                has_integrity,
            });
        }

        // Inline <style> blocks
        let style_sel = selectors::style();

        for el in document.select(style_sel) {
            let has_content = !el.text().collect::<String>().trim().is_empty();
            if has_content {
                styles.push(StyleInfo {
                    href: None,
                    media: None,
                    is_inline: true,
                    has_integrity: false,
                });
            }
        }

        styles
    }

    // ---------------------------------------------------------------------------
    // Structured data (JSON-LD)
    // ---------------------------------------------------------------------------
    pub(super) fn extract_structured_data(document: &Html) -> Vec<StructuredData> {
        let selector = selectors::script_ld_json();

        document
            .select(selector)
            .filter_map(|el| {
                let raw: String = el.text().collect();
                let raw = raw.trim();
                if raw.is_empty() {
                    return None;
                }

                let value: serde_json::Value = match serde_json::from_str(raw) {
                    Ok(v) => v,
                    Err(_) => return None,
                };

                let context = value
                    .get("@context")
                    .and_then(|c| c.as_str())
                    .map(String::from);
                let r#type = value
                    .get("@type")
                    .and_then(|t| t.as_str())
                    .map(String::from);

                Some(StructuredData {
                    context,
                    r#type,
                    data: value,
                })
            })
            .collect()
    }
}
