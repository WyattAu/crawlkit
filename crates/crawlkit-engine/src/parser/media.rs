//! Image extraction and attribute analysis.

use scraper::Html;

use super::selectors;
use super::ExtractedImage;
use super::HtmlParser;

impl HtmlParser {
    // ---------------------------------------------------------------------------
    // Images
    // ---------------------------------------------------------------------------
    pub(super) fn extract_images(document: &Html) -> Vec<ExtractedImage> {
        let selector = selectors::img();

        document
            .select(selector)
            .map(|el| {
                let src = el.value().attr("src").unwrap_or("").to_string();
                let alt = el.value().attr("alt").unwrap_or("").to_string();
                let width = el.value().attr("width").and_then(|w| w.parse::<u32>().ok());
                let height = el
                    .value()
                    .attr("height")
                    .and_then(|h| h.parse::<u32>().ok());
                let has_alt = el.value().attr("alt").is_some();
                let is_lazy_loaded = el
                    .value()
                    .attr("loading")
                    .map(|l| l == "lazy")
                    .unwrap_or(false)
                    || el.value().attr("data-src").is_some();

                ExtractedImage {
                    src,
                    alt,
                    width,
                    height,
                    has_alt,
                    is_lazy_loaded,
                }
            })
            .collect()
    }
}
