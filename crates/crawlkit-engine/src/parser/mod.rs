//! HTML parsing and feature extraction.
//!
//! Splits document analysis into focused submodules:
//! [`content`] (meta/headings/word count), [`links`], [`media`], [`forms`],
//! [`assets`] (scripts/styles/JSON-LD), [`accessibility`], and
//! [`streaming`] (incremental chunked parsing).

mod accessibility;
mod assets;
mod content;
mod forms;
mod links;
mod media;
mod selectors;
mod streaming;

#[cfg(test)]
#[cfg(test)]
mod tests;

pub use streaming::{ParserEvent, StreamingHtmlParser};

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::meta::MetaTags;

/// A heading extracted from the page (H1–H6).
///
/// Used by the [`HeadingHierarchyAnalyzer`](crate::HeadingHierarchyAnalyzer)
/// to check heading hierarchy and count H1 tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    /// Heading level (1-6).
    pub level: u8,
    /// Text content of the heading.
    pub text: String,
    /// Character length of the heading text.
    pub length: usize,
}

/// An image extracted from the page.
///
/// Used by the [`ImageAnalyzer`](crate::ImageAnalyzer) to check for
/// missing alt text, lazy loading, and dimension attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedImage {
    /// Image source URL.
    pub src: String,
    /// Alt text for accessibility.
    pub alt: String,
    /// Width attribute (if specified).
    pub width: Option<u32>,
    /// Height attribute (if specified).
    pub height: Option<u32>,
    /// Whether the image has an alt attribute.
    pub has_alt: bool,
    /// Whether the image uses lazy loading (`loading="lazy"` or `data-src`).
    pub is_lazy_loaded: bool,
}

/// A link extracted from the page.
///
/// Used by the [`LinkAnalyzer`](crate::LinkAnalyzer) to compute internal/external
/// link counts, nofollow detection, and orphan page identification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedLink {
    /// Resolved href URL.
    pub href: String,
    /// Link anchor text.
    pub text: String,
    /// Rel attribute values (e.g., "nofollow", "noopener").
    pub rel: Vec<String>,
    /// Whether the link points to a different domain.
    pub is_external: bool,
    /// ARIA label for the link (accessibility).
    pub aria_label: Option<String>,
    /// Alt text from images inside the link (accessibility).
    pub img_alt: Option<String>,
}

/// An input element inside a form.
///
/// Tracks accessibility attributes (labels, ARIA) and input metadata
/// for form analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedInput {
    /// Input type attribute (text, email, file, etc.).
    pub input_type: Option<String>,
    /// Input name attribute.
    pub name: Option<String>,
    /// Input id attribute.
    pub id: Option<String>,
    /// Whether the input has an associated label.
    pub has_label: bool,
    /// ARIA label for the input.
    pub aria_label: Option<String>,
    /// ARIA labelledby reference.
    pub aria_labelledby: Option<String>,
    /// ARIA describedby reference.
    pub aria_describedby: Option<String>,
    /// Placeholder text.
    pub placeholder: Option<String>,
    /// Whether the input is required.
    pub required: bool,
}

/// A form detected on the page.
///
/// Used by accessibility analyzers to check form structure, labeling,
/// and fieldset/legend usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedForm {
    /// Form action URL.
    pub action: Option<String>,
    /// HTTP method (get, post).
    pub method: String,
    /// Number of input elements.
    pub input_count: usize,
    /// Whether the form contains a file input.
    pub has_file_input: bool,
    /// Whether the form contains a search input.
    pub has_search_input: bool,
    /// Extracted input elements with accessibility info.
    pub inputs: Vec<ExtractedInput>,
    /// Whether the form uses a fieldset.
    pub has_fieldset: bool,
    /// Whether the form uses a legend.
    pub has_legend: bool,
}

/// Script tag information.
///
/// Tracks script loading attributes for performance analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    /// External script source URL.
    pub src: Option<String>,
    /// Whether the script has the `async` attribute.
    pub r#async: bool,
    /// Whether the script has the `defer` attribute.
    pub defer: bool,
    /// Script type attribute (e.g., "application/ld+json").
    pub script_type: Option<String>,
}

/// Style/link stylesheet information.
///
/// Tracks stylesheet loading for performance and render-blocking analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleInfo {
    /// External stylesheet URL.
    pub href: Option<String>,
    /// Media query (e.g., "print", "screen").
    pub media: Option<String>,
    /// Whether this is an inline `<style>` block.
    pub is_inline: bool,
}

/// Structured data extracted from JSON-LD `<script>` blocks.
///
/// Contains the parsed `@context`, `@type`, and full JSON data for
/// schema validation and analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredData {
    /// The `@context` value (e.g., `<https://schema.org>`).
    pub context: Option<String>,
    /// The `@type` value (e.g., "Article", "Product").
    pub r#type: Option<String>,
    /// The full structured data JSON.
    pub data: serde_json::Value,
}

/// Complete parsed representation of a page.
///
/// Contains all SEO-relevant data extracted from raw HTML by [`HtmlParser`],
/// including meta tags, headings, links, images, forms, scripts, styles,
/// structured data, accessibility landmarks, and social media metadata.
///
/// This is the primary input to the analysis engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPage {
    /// The page URL.
    pub url: String,
    /// Extracted meta tags.
    pub meta: MetaTags,
    /// Headings (H1-H6) in document order.
    pub headings: Vec<Heading>,
    /// All links on the page.
    pub links: Vec<ExtractedLink>,
    /// All images on the page.
    pub images: Vec<ExtractedImage>,
    /// Forms detected on the page.
    pub forms: Vec<ExtractedForm>,
    /// Script tags on the page.
    pub scripts: Vec<ScriptInfo>,
    /// Stylesheets (external and inline).
    pub styles: Vec<StyleInfo>,
    /// JSON-LD structured data blocks.
    pub structured_data: Vec<StructuredData>,
    /// Word count of visible text content.
    pub word_count: usize,

    // Accessibility fields
    /// Landmark roles found on the page (e.g. "banner", "main", "navigation").
    pub landmarks: Vec<String>,
    /// Whether the page has a skip-to-content link.
    pub has_skip_link: bool,
    /// Whether the page has a `<main>` element or role="main".
    pub has_main_landmark: bool,
    /// Whether the page has a `<nav>` element or role="navigation".
    pub has_nav_landmark: bool,
    /// Whether any element has tabindex > 0 (positive tabindex).
    pub has_positive_tabindex: bool,
    /// Number of elements with tabindex=-1 (removed from tab order).
    pub tabindex_negative_count: usize,
    /// Number of ARIA roles used on the page.
    pub aria_role_count: usize,
    /// Number of elements with aria-label or aria-labelledby.
    pub aria_label_count: usize,
    /// Whether the html element has a lang attribute.
    pub has_lang_attribute: bool,
    /// The html lang attribute value.
    pub html_lang: Option<String>,
    /// Whether any element uses aria-hidden="true".
    pub has_aria_hidden: bool,
    /// Table accessibility summary.
    pub tables_with_headers: usize,
    pub tables_total: usize,
    pub tables_with_captions: usize,

    // Social media fields
    /// OG image width (from og:image:width meta tag).
    pub og_image_width: Option<u32>,
    /// OG image height (from og:image:height meta tag).
    pub og_image_height: Option<u32>,
}

/// HTML parser that extracts structured data from raw HTML.
///
/// Stateless parser using the `scraper` crate for CSS selector-based
/// extraction. All methods are static and thread-safe.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::HtmlParser;
/// use url::Url;
///
/// let html = r#"<!DOCTYPE html><html><head><title>Test</title></head>
/// <body><h1>Hello</h1><a href="/link">link</a></body></html>"#;
/// let url = Url::parse("https://example.com/page").unwrap();
/// let page = HtmlParser::parse(html, &url);
///
/// assert_eq!(page.meta.title.as_deref(), Some("Test"));
/// assert_eq!(page.headings.len(), 1);
/// assert_eq!(page.links.len(), 1);
/// ```
pub struct HtmlParser;

impl HtmlParser {
    /// Parse an HTML document and extract all SEO-relevant data.
    ///
    /// Extracts meta tags, headings, links, images, forms, scripts,
    /// stylesheets, structured data (JSON-LD), accessibility landmarks,
    /// and social media metadata.
    ///
    /// The underlying HTML5 parser is error-tolerant by design: malformed
    /// input yields a best-effort DOM rather than an error, so parsing is
    /// infallible.
    pub fn parse(html: &str, url: &Url) -> ParsedPage {
        let document = Html::parse_document(html);

        let meta = Self::extract_meta(&document, url);
        let headings = Self::extract_headings(&document);
        let links = Self::extract_links(&document, url);
        let images = Self::extract_images(&document);
        let forms = Self::extract_forms(&document);
        let scripts = Self::extract_scripts(&document);
        let styles = Self::extract_styles(&document);
        let structured_data = Self::extract_structured_data(&document);
        let word_count = Self::count_words(&document);

        let accessibility = Self::extract_accessibility(&document, Some(html));
        let social = Self::extract_social(&document);

        ParsedPage {
            url: url.to_string(),
            meta,
            headings,
            links,
            images,
            forms,
            scripts,
            styles,
            structured_data,
            word_count,
            landmarks: accessibility.0,
            has_skip_link: accessibility.1,
            has_main_landmark: accessibility.2,
            has_nav_landmark: accessibility.3,
            has_positive_tabindex: accessibility.4,
            tabindex_negative_count: accessibility.5,
            aria_role_count: accessibility.6,
            aria_label_count: accessibility.7,
            has_lang_attribute: accessibility.8,
            html_lang: accessibility.9,
            has_aria_hidden: accessibility.10,
            tables_with_headers: accessibility.11,
            tables_total: accessibility.12,
            tables_with_captions: accessibility.13,
            og_image_width: social.0,
            og_image_height: social.1,
        }
    }

    // ---------------------------------------------------------------------------
    // Social media data extraction
    // ---------------------------------------------------------------------------

    /// Returns (og_image_width, og_image_height).
    fn extract_social(document: &Html) -> (Option<u32>, Option<u32>) {
        let width =
            Self::get_meta_content(document, "og:image:width").and_then(|w| w.parse::<u32>().ok());
        let height =
            Self::get_meta_content(document, "og:image:height").and_then(|h| h.parse::<u32>().ok());
        (width, height)
    }

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------
    fn select_text(document: &Html, selector_str: &str) -> Option<String> {
        let selector = Selector::parse(selector_str).ok()?;
        let el = document.select(&selector).next()?;
        let text: String = el.text().collect::<Vec<_>>().join("").trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// Get the `content` attribute of `<meta name="X" content="...">` or
    /// `<meta property="X" content="...">`.
    fn get_meta_content(document: &Html, name_or_property: &str) -> Option<String> {
        // Try name= first
        let by_name = format!("meta[name=\"{name_or_property}\"]");
        if let Ok(sel) = Selector::parse(&by_name) {
            if let Some(val) = document
                .select(&sel)
                .next()
                .and_then(|el| el.value().attr("content"))
            {
                let val = val.trim().to_string();
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }

        // Try property= (OG tags)
        let by_prop = format!("meta[property=\"{name_or_property}\"]");
        if let Ok(sel) = Selector::parse(&by_prop) {
            if let Some(val) = document
                .select(&sel)
                .next()
                .and_then(|el| el.value().attr("content"))
            {
                let val = val.trim().to_string();
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }

        None
    }

    /// Get an attribute value from the first element matching a selector.
    fn get_attr(document: &Html, selector_str: &str, attr: &str) -> Option<String> {
        let selector = Selector::parse(selector_str).ok()?;
        document
            .select(&selector)
            .next()
            .and_then(|el| el.value().attr(attr))
            .map(String::from)
    }
}
