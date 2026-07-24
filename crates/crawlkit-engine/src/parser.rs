use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::meta::{HreflangTag, MetaTags, OpenGraphTags, TwitterTags};

/// Cached CSS selectors compiled once, reused on every parse call.
/// `OnceLock` guarantees thread-safe lazy initialization with zero cost after first use.
/// All selector patterns are static compile-time-known strings.
#[allow(clippy::expect_used)]
mod selectors {
    use scraper::Selector;
    use std::sync::OnceLock;

    fn cached(pattern: &str) -> &Selector {
        static CELL: OnceLock<Selector> = OnceLock::new();
        CELL.get_or_init(|| Selector::parse(pattern).expect("static CSS selector is valid"))
    }

    pub fn html() -> &'static Selector {
        cached("html")
    }
    pub fn header() -> &'static Selector {
        cached("header")
    }
    pub fn nav() -> &'static Selector {
        cached("nav")
    }
    pub fn main() -> &'static Selector {
        cached("main")
    }
    pub fn aside() -> &'static Selector {
        cached("aside")
    }
    pub fn footer() -> &'static Selector {
        cached("footer")
    }
    pub fn form() -> &'static Selector {
        cached("form")
    }
    pub fn section_aria() -> &'static Selector {
        cached("section[aria-label], section[aria-labelledby]")
    }
    pub fn role_banner() -> &'static Selector {
        cached("[role=banner]")
    }
    pub fn role_navigation() -> &'static Selector {
        cached("[role=navigation]")
    }
    pub fn role_main() -> &'static Selector {
        cached("[role=main]")
    }
    pub fn role_complementary() -> &'static Selector {
        cached("[role=complementary]")
    }
    pub fn role_contentinfo() -> &'static Selector {
        cached("[role=contentinfo]")
    }
    pub fn input_select_textarea() -> &'static Selector {
        cached("input, select, textarea")
    }
}

/// Errors that can occur during HTML parsing.
///
/// Covers CSS selector compilation failures, URL resolution errors,
/// and JSON-LD parsing issues.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("selector compilation failed: {0}")]
    Selector(String),

    #[error("URL resolution failed: {0}")]
    UrlResolution(#[from] url::ParseError),

    #[error("JSON-LD parse error: {0}")]
    JsonLd(String),
}

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
    /// The `@context` value (e.g., "https://schema.org").
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
/// use crawlkit_engine::{HtmlParser, parser::ParseError};
/// use url::Url;
///
/// let html = r#"<!DOCTYPE html><html><head><title>Test</title></head>
/// <body><h1>Hello</h1><a href="/link">link</a></body></html>"#;
/// let url = Url::parse("https://example.com/page").unwrap();
/// let page = HtmlParser::parse(html, &url).unwrap();
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
    /// # Errors
    ///
    /// Currently always returns `Ok`. The `ParseError` type is reserved
    /// for future selector compilation or URL resolution failures.
    pub fn parse(html: &str, url: &Url) -> Result<ParsedPage, ParseError> {
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

        let accessibility = Self::extract_accessibility(&document);
        let social = Self::extract_social(&document);

        Ok(ParsedPage {
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
        })
    }

    // ---------------------------------------------------------------------------
    // Meta tags
    // ---------------------------------------------------------------------------

    fn extract_meta(document: &Html, page_url: &Url) -> MetaTags {
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

    fn extract_hreflang(document: &Html, page_url: &Url) -> Vec<HreflangTag> {
        let selector = match Selector::parse("link[hreflang]") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        document
            .select(&selector)
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

    fn extract_headings(document: &Html) -> Vec<Heading> {
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
    // Links
    // ---------------------------------------------------------------------------

    fn extract_links(document: &Html, page_url: &Url) -> Vec<ExtractedLink> {
        let selector = match Selector::parse("a[href]") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let page_domain = page_url.domain().unwrap_or("");

        document
            .select(&selector)
            .filter_map(|el| {
                let raw_href = el.value().attr("href")?.to_string();
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
                    .select(&Selector::parse("img").ok()?)
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

    // ---------------------------------------------------------------------------
    // Images
    // ---------------------------------------------------------------------------

    fn extract_images(document: &Html) -> Vec<ExtractedImage> {
        let selector = match Selector::parse("img") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        document
            .select(&selector)
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

    // ---------------------------------------------------------------------------
    // Forms
    // ---------------------------------------------------------------------------

    fn extract_forms(document: &Html) -> Vec<ExtractedForm> {
        let selector = match Selector::parse("form") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let input_sel = match Selector::parse("input, select, textarea") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let label_sel = match Selector::parse("label") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        document
            .select(&selector)
            .map(|form| {
                let action = form.value().attr("action").map(String::from);
                let method = form.value().attr("method").unwrap_or("get").to_lowercase();

                let inputs: Vec<_> = form.select(&input_sel).collect();
                let input_count = inputs.len();
                let has_file_input = inputs
                    .iter()
                    .any(|i| i.value().attr("type") == Some("file"));
                let has_search_input = inputs.iter().any(|i| {
                    i.value().attr("type") == Some("search")
                        || i.value().attr("role") == Some("search")
                });

                // Collect all label `for` targets within this form
                let label_for_ids: std::collections::HashSet<String> = form
                    .select(&label_sel)
                    .filter_map(|l| l.value().attr("for").map(String::from))
                    .collect();

                // Collect all input nodes that are descendants of a <label>
                let inputs_in_labels: std::collections::HashSet<ego_tree::NodeId> = {
                    let inner_input_sel = selectors::input_select_textarea();
                    form.select(&label_sel)
                        .flat_map(|label| label.select(inner_input_sel))
                        .map(|input| input.id())
                        .collect()
                };

                let extracted_inputs: Vec<ExtractedInput> = inputs
                    .iter()
                    .map(|input| {
                        let input_type = input.value().attr("type").map(String::from);
                        let name = input.value().attr("name").map(String::from);
                        let id = input.value().attr("id").map(String::from);
                        let aria_label = input.value().attr("aria-label").map(String::from);
                        let aria_labelledby =
                            input.value().attr("aria-labelledby").map(String::from);
                        let aria_describedby =
                            input.value().attr("aria-describedby").map(String::from);
                        let placeholder = input.value().attr("placeholder").map(String::from);
                        let required = input.value().attr("required").is_some()
                            || input.value().attr("aria-required") == Some("true");

                        let has_explicit_label = id
                            .as_ref()
                            .map(|id_val| label_for_ids.contains(id_val))
                            .unwrap_or(false);

                        let has_implicit_label = inputs_in_labels.contains(&input.id());

                        let has_label = has_explicit_label
                            || has_implicit_label
                            || aria_label.is_some()
                            || aria_labelledby.is_some();

                        ExtractedInput {
                            input_type,
                            name,
                            id,
                            has_label,
                            aria_label,
                            aria_labelledby,
                            aria_describedby,
                            placeholder,
                            required,
                        }
                    })
                    .collect();

                let has_fieldset = Selector::parse("fieldset")
                    .ok()
                    .and_then(|s| form.select(&s).next().is_some().then_some(true))
                    .unwrap_or(false);
                let has_legend = Selector::parse("legend")
                    .ok()
                    .and_then(|s| form.select(&s).next().is_some().then_some(true))
                    .unwrap_or(false);

                ExtractedForm {
                    action,
                    method,
                    input_count,
                    has_file_input,
                    has_search_input,
                    inputs: extracted_inputs,
                    has_fieldset,
                    has_legend,
                }
            })
            .collect()
    }

    // ---------------------------------------------------------------------------
    // Scripts
    // ---------------------------------------------------------------------------

    fn extract_scripts(document: &Html) -> Vec<ScriptInfo> {
        let selector = match Selector::parse("script") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        document
            .select(&selector)
            .map(|el| {
                let src = el.value().attr("src").map(String::from);
                let r#async = el.value().attr("async").is_some();
                let defer = el.value().attr("defer").is_some();
                let script_type = el.value().attr("type").map(String::from);

                ScriptInfo {
                    src,
                    r#async,
                    defer,
                    script_type,
                }
            })
            .collect()
    }

    // ---------------------------------------------------------------------------
    // Styles
    // ---------------------------------------------------------------------------

    fn extract_styles(document: &Html) -> Vec<StyleInfo> {
        let mut styles = Vec::new();

        // External stylesheets via <link rel="stylesheet">
        let link_sel = match Selector::parse("link[rel=stylesheet]") {
            Ok(s) => s,
            Err(_) => return styles,
        };

        for el in document.select(&link_sel) {
            let href = el.value().attr("href").map(String::from);
            let media = el.value().attr("media").map(String::from);

            styles.push(StyleInfo {
                href,
                media,
                is_inline: false,
            });
        }

        // Inline <style> blocks
        let style_sel = match Selector::parse("style") {
            Ok(s) => s,
            Err(_) => return styles,
        };

        for el in document.select(&style_sel) {
            let has_content = !el.text().collect::<String>().trim().is_empty();
            if has_content {
                styles.push(StyleInfo {
                    href: None,
                    media: None,
                    is_inline: true,
                });
            }
        }

        styles
    }

    // ---------------------------------------------------------------------------
    // Structured data (JSON-LD)
    // ---------------------------------------------------------------------------

    fn extract_structured_data(document: &Html) -> Vec<StructuredData> {
        let selector = match Selector::parse("script[type=\"application/ld+json\"]") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        document
            .select(&selector)
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

    // ---------------------------------------------------------------------------
    // Word count
    // ---------------------------------------------------------------------------

    fn count_words(document: &Html) -> usize {
        let body = Selector::parse("body").ok();
        let script = Selector::parse("script").ok();
        let style = Selector::parse("style").ok();
        let noscript = Selector::parse("noscript").ok();

        // Collect the set of node IDs to skip (script/style/noscript elements).
        let mut skip_ids = std::collections::HashSet::new();
        for sel in [&script, &style, &noscript].into_iter().flatten() {
            for el in document.select(sel) {
                skip_ids.insert(el.id());
            }
        }

        let root = body
            .as_ref()
            .and_then(|sel| document.select(sel).next())
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

        text.split_whitespace().filter(|w| !w.is_empty()).count()
    }

    // ---------------------------------------------------------------------------
    // Accessibility data extraction
    // ---------------------------------------------------------------------------

    /// Returns (landmarks, has_skip_link, has_main, has_nav, has_positive_tabindex,
    /// tabindex_neg_count, aria_role_count, aria_label_count, has_lang, html_lang,
    /// has_aria_hidden, tables_with_headers, tables_total, tables_with_captions).
    fn extract_accessibility(
        document: &Html,
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

        // Check html lang
        let has_lang;
        let html_lang;
        if let Some(html_el) = document.select(selectors::html()).next() {
            html_lang = html_el.value().attr("lang").map(String::from);
            has_lang = html_lang.is_some();
        } else {
            has_lang = false;
            html_lang = None;
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
        if let Ok(sel) = Selector::parse("a[href^=\"#\"]") {
            for el in document.select(&sel) {
                let text: String = el.text().collect::<Vec<_>>().join("").to_lowercase();
                let class = el.value().attr("class").unwrap_or("").to_lowercase();
                let id = el.value().attr("href").unwrap_or("").to_lowercase();
                if text.contains("skip") || class.contains("skip") || id.contains("skip") {
                    has_skip_link = true;
                    break;
                }
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
        if let Ok(table_sel) = Selector::parse("table") {
            if let Ok(th_sel) = Selector::parse("th") {
                if let Ok(caption_sel) = Selector::parse("caption") {
                    for table in document.select(&table_sel) {
                        tables_total += 1;
                        if table.select(&th_sel).next().is_some() {
                            tables_with_headers += 1;
                        }
                        if table.select(&caption_sel).next().is_some() {
                            tables_with_captions += 1;
                        }
                    }
                }
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn test_url() -> Url {
        Url::parse("https://example.com/page").unwrap()
    }

    #[test]
    fn test_parse_title() {
        let html =
            r#"<!DOCTYPE html><html><head><title>My Page</title></head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.meta.title.as_deref(), Some("My Page"));
    }

    #[test]
    fn test_parse_meta_description() {
        let html = r#"<!DOCTYPE html><html><head>
            <meta name="description" content="A great page">
        </head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.meta.description.as_deref(), Some("A great page"));
    }

    #[test]
    fn test_parse_canonical() {
        let html = r#"<!DOCTYPE html><html><head>
            <link rel="canonical" href="/canonical-page">
        </head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert!(page.meta.canonical.is_some());
        assert!(page
            .meta
            .canonical
            .unwrap()
            .as_str()
            .contains("canonical-page"));
    }

    #[test]
    fn test_parse_open_graph() {
        let html = r#"<!DOCTYPE html><html><head>
            <meta property="og:title" content="OG Title">
            <meta property="og:description" content="OG Desc">
            <meta property="og:image" content="https://example.com/img.png">
            <meta property="og:url" content="https://example.com">
            <meta property="og:type" content="website">
        </head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.meta.og.title.as_deref(), Some("OG Title"));
        assert_eq!(page.meta.og.description.as_deref(), Some("OG Desc"));
        assert_eq!(
            page.meta.og.image.as_deref(),
            Some("https://example.com/img.png")
        );
        assert_eq!(page.meta.og.r#type.as_deref(), Some("website"));
    }

    #[test]
    fn test_parse_twitter_cards() {
        let html = r#"<!DOCTYPE html><html><head>
            <meta name="twitter:card" content="summary_large_image">
            <meta name="twitter:site" content="@example">
            <meta name="twitter:creator" content="@author">
            <meta name="twitter:title" content="TW Title">
            <meta name="twitter:description" content="TW Desc">
            <meta name="twitter:image" content="https://example.com/tw.png">
        </head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(
            page.meta.twitter.card.as_deref(),
            Some("summary_large_image")
        );
        assert_eq!(page.meta.twitter.site.as_deref(), Some("@example"));
        assert_eq!(page.meta.twitter.creator.as_deref(), Some("@author"));
        assert_eq!(page.meta.twitter.title.as_deref(), Some("TW Title"));
    }

    #[test]
    fn test_parse_headings() {
        let html = r#"<!DOCTYPE html><html><body>
            <h1>Main Title</h1>
            <h2>Section</h2>
            <h2>Another</h2>
            <h3>Sub</h3>
        </body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.headings.len(), 4);
        assert_eq!(page.headings[0].level, 1);
        assert_eq!(page.headings[0].text, "Main Title");
        assert_eq!(page.headings[1].level, 2);
        assert_eq!(page.headings[2].level, 2);
        assert_eq!(page.headings[3].level, 3);
    }

    #[test]
    fn test_extract_links() {
        let html = r#"<!DOCTYPE html><html><body>
            <a href="/internal">Internal</a>
            <a href="https://external.com/page">External</a>
            <a href="/page" rel="nofollow noopen">Nofollow</a>
        </body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.links.len(), 3);

        assert_eq!(page.links[0].href, "https://example.com/internal");
        assert_eq!(page.links[0].text, "Internal");
        assert!(!page.links[0].is_external);

        assert_eq!(page.links[1].href, "https://external.com/page");
        assert!(page.links[1].is_external);

        assert!(page.links[2].rel.contains(&"nofollow".to_string()));
    }

    #[test]
    fn test_extract_images() {
        let html = r#"<!DOCTYPE html><html><body>
            <img src="/img1.png" alt="Picture" width="100" height="200">
            <img src="/img2.jpg">
            <img src="/img3.webp" loading="lazy" data-src="/img3-real.webp">
        </body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.images.len(), 3);

        assert_eq!(page.images[0].src, "/img1.png");
        assert_eq!(page.images[0].alt, "Picture");
        assert_eq!(page.images[0].width, Some(100));
        assert_eq!(page.images[0].height, Some(200));
        assert!(page.images[0].has_alt);
        assert!(!page.images[0].is_lazy_loaded);

        assert!(!page.images[1].has_alt);

        assert!(page.images[2].is_lazy_loaded);
    }

    #[test]
    fn test_extract_forms() {
        let html = r#"<!DOCTYPE html><html><body>
            <form action="/submit" method="post">
                <input type="text" name="q">
                <input type="file" name="doc">
            </form>
            <form>
                <input type="search" name="s">
            </form>
        </body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.forms.len(), 2);

        assert_eq!(page.forms[0].action.as_deref(), Some("/submit"));
        assert_eq!(page.forms[0].method, "post");
        assert_eq!(page.forms[0].input_count, 2);
        assert!(page.forms[0].has_file_input);

        assert!(page.forms[1].has_search_input);
    }

    #[test]
    fn test_extract_scripts() {
        let html = r#"<!DOCTYPE html><html><body>
            <script src="/app.js" async></script>
            <script defer src="/lib.js"></script>
            <script type="application/ld+json">{"@type":"WebSite"}</script>
            <script>console.log("hi")</script>
        </body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        // 4 total script tags
        assert_eq!(page.scripts.len(), 4);
        assert!(page.scripts[0].r#async);
        assert!(page.scripts[1].defer);
        assert_eq!(
            page.scripts[2].script_type.as_deref(),
            Some("application/ld+json")
        );
    }

    #[test]
    fn test_extract_styles() {
        let html = r#"<!DOCTYPE html><html><head>
            <link rel="stylesheet" href="/style.css">
            <link rel="stylesheet" href="/print.css" media="print">
            <style>body { margin: 0; }</style>
        </head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.styles.len(), 3);
        assert!(!page.styles[0].is_inline);
        assert_eq!(page.styles[0].href.as_deref(), Some("/style.css"));
        assert_eq!(page.styles[1].media.as_deref(), Some("print"));
        assert!(page.styles[2].is_inline);
    }

    #[test]
    fn test_extract_json_ld() {
        let html = r#"<!DOCTYPE html><html><body>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": "Test"
            }
            </script>
        </body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.structured_data.len(), 1);
        assert_eq!(
            page.structured_data[0].context.as_deref(),
            Some("https://schema.org")
        );
        assert_eq!(page.structured_data[0].r#type.as_deref(), Some("Article"));
    }

    #[test]
    fn test_word_count() {
        let html = r#"<!DOCTYPE html><html><body>
            <h1>Hello World</h1>
            <p>This is a test paragraph with some words.</p>
            <script>var x = 1;</script>
            <style>.a { color: red; }</style>
        </body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        // "Hello World" = 2, "This is a test paragraph with some words." = 8
        assert_eq!(page.word_count, 10);
    }

    #[test]
    fn test_word_count_excludes_script_and_style() {
        let html = r#"<!DOCTYPE html><html><body>
            <p>Visible text here.</p>
            <script>function foo() { return "not counted"; }</script>
            <style>.hidden { display: none; }</style>
            <noscript>JavaScript is required</noscript>
        </body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        // Only "Visible text here." should count = 3
        assert_eq!(page.word_count, 3);
    }

    #[test]
    fn test_hreflang() {
        let html = r#"<!DOCTYPE html><html><head>
            <link rel="alternate" hreflang="en" href="https://example.com/en">
            <link rel="alternate" hreflang="fr" href="https://example.com/fr">
            <link rel="alternate" hreflang="x-default" href="https://example.com">
        </head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.meta.hreflang.len(), 3);
        assert_eq!(page.meta.hreflang[0].lang, "en");
        assert_eq!(page.meta.hreflang[1].lang, "fr");
        assert_eq!(page.meta.hreflang[2].lang, "x-default");
    }

    #[test]
    fn test_robots_meta() {
        let html = r#"<!DOCTYPE html><html><head>
            <meta name="robots" content="noindex, nofollow">
        </head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert!(page.meta.is_noindex());
        assert!(page.meta.is_nofollow());
    }

    #[test]
    fn test_language_and_charset() {
        let html = r#"<!DOCTYPE html><html lang="en"><head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
        </head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.meta.language.as_deref(), Some("en"));
        assert!(page.meta.charset.is_some());
        assert!(page.meta.viewport.is_some());
    }

    #[test]
    fn test_empty_html() {
        let page = HtmlParser::parse("", &test_url()).unwrap();
        assert!(page.meta.title.is_none());
        assert!(page.headings.is_empty());
        assert!(page.links.is_empty());
        assert!(page.images.is_empty());
        assert_eq!(page.word_count, 0);
    }

    #[test]
    fn test_title_fallback_to_og() {
        let html = r#"<!DOCTYPE html><html><head>
            <meta property="og:title" content="OG Fallback Title">
        </head><body></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        assert_eq!(page.meta.title.as_deref(), Some("OG Fallback Title"));
    }

    #[test]
    fn test_parsed_page_serialization() {
        let html = r#"<!DOCTYPE html><html><head><title>Test</title></head>
        <body><h1>Hi</h1><a href="/link">link</a></body></html>"#;
        let page = HtmlParser::parse(html, &test_url()).unwrap();
        let json = serde_json::to_string(&page).unwrap();
        let deser: ParsedPage = serde_json::from_str(&json).unwrap();
        assert_eq!(page.meta.title, deser.meta.title);
        assert_eq!(page.headings.len(), deser.headings.len());
        assert_eq!(page.word_count, deser.word_count);
    }
}
