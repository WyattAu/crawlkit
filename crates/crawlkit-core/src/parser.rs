use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::meta::{HreflangTag, MetaTags, OpenGraphTags, TwitterTags};

/// Errors that can occur during HTML parsing.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub length: usize,
}

/// An image extracted from the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedImage {
    pub src: String,
    pub alt: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_alt: bool,
    pub is_lazy_loaded: bool,
}

/// A link extracted from the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedLink {
    pub href: String,
    pub text: String,
    pub rel: Vec<String>,
    pub is_external: bool,
}

/// A form detected on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedForm {
    pub action: Option<String>,
    pub method: String,
    pub input_count: usize,
    pub has_file_input: bool,
    pub has_search_input: bool,
}

/// Script tag information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub src: Option<String>,
    pub r#async: bool,
    pub defer: bool,
    pub script_type: Option<String>,
}

/// Style/link stylesheet information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleInfo {
    pub href: Option<String>,
    pub media: Option<String>,
    pub is_inline: bool,
}

/// Structured data extracted from JSON-LD `<script>` blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredData {
    pub context: Option<String>,
    pub r#type: Option<String>,
    pub data: serde_json::Value,
}

/// Complete parsed representation of a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPage {
    pub url: String,
    pub meta: MetaTags,
    pub headings: Vec<Heading>,
    pub links: Vec<ExtractedLink>,
    pub images: Vec<ExtractedImage>,
    pub forms: Vec<ExtractedForm>,
    pub scripts: Vec<ScriptInfo>,
    pub styles: Vec<StyleInfo>,
    pub structured_data: Vec<StructuredData>,
    pub word_count: usize,
}

/// HTML parser that extracts structured data from raw HTML.
pub struct HtmlParser;

impl HtmlParser {
    /// Parse an HTML document and extract all SEO-relevant data.
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
        let charset = Self::get_attr(document, "meta[charset]", "charset")
            .or_else(|| {
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
                let href = el.value().attr("href")?.to_string();
                let text: String = el.text().collect::<Vec<_>>().join("").trim().to_string();

                let rel: Vec<String> = el
                    .value()
                    .attr("rel")
                    .map(|r| r.split_whitespace().map(String::from).collect())
                    .unwrap_or_default();

                let is_external = page_url
                    .join(&href)
                    .ok()
                    .map(|resolved| {
                        resolved.domain().unwrap_or("") != page_domain
                    })
                    .unwrap_or(false);

                Some(ExtractedLink {
                    href,
                    text,
                    rel,
                    is_external,
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
                let width = el
                    .value()
                    .attr("width")
                    .and_then(|w| w.parse::<u32>().ok());
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
                    || el
                        .value()
                        .attr("data-src")
                        .is_some();

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

        let input_sel = match Selector::parse("input") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        document
            .select(&selector)
            .map(|form| {
                let action = form.value().attr("action").map(String::from);
                let method = form
                    .value()
                    .attr("method")
                    .unwrap_or("get")
                    .to_lowercase();

                let inputs: Vec<_> = form.select(&input_sel).collect();
                let input_count = inputs.len();
                let has_file_input = inputs
                    .iter()
                    .any(|i| i.value().attr("type") == Some("file"));
                let has_search_input = inputs.iter().any(|i| {
                    i.value().attr("type") == Some("search")
                        || i.value().attr("role") == Some("search")
                });

                ExtractedForm {
                    action,
                    method,
                    input_count,
                    has_file_input,
                    has_search_input,
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
            let node = tree.get(node_id).expect("node must exist");
            match node.value() {
                scraper::Node::Element(el) => {
                    let tag = el.name();
                    if tag == "script" || tag == "style" || tag == "noscript" || tag == "svg" {
                        return;
                    }
                }
                scraper::Node::Text(t) => {
                    text.push_str(&t);
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

        collect_text(&tree, root, &skip_ids, &mut text);

        text.split_whitespace()
            .filter(|w| !w.is_empty())
            .count()
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
        if let Some(sel) = Selector::parse(&by_name).ok() {
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
        if let Some(sel) = Selector::parse(&by_prop).ok() {
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
mod tests {
    use super::*;

    fn test_url() -> Url {
        Url::parse("https://example.com/page").unwrap()
    }

    #[test]
    fn test_parse_title() {
        let html = r#"<!DOCTYPE html><html><head><title>My Page</title></head><body></body></html>"#;
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

        assert_eq!(page.links[0].href, "/internal");
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
