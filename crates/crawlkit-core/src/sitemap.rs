//! Sitemap.xml fetching, parsing, and caching.
//!
//! Supports both `<urlset>` (standard sitemaps) and `<sitemapindex>`
//! (sitemap index files). Sitemap URLs are discovered from robots.txt
//! via `RobotsTxtCache::sitemaps()`.

use std::sync::Arc;

use dashmap::DashMap;
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::analyzers::SitemapEntry;
use crate::http::HttpClient;

/// Cache for parsed sitemap data.
///
/// Fetches and parses sitemaps once per URL, then serves cached results.
pub struct SitemapCache {
    /// Sitemap URL -> parsed entries.
    cache: DashMap<String, Vec<SitemapEntry>>,
    /// HTTP client for fetching.
    client: Arc<HttpClient>,
}

impl SitemapCache {
    /// Create a new cache.
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self {
            cache: DashMap::new(),
            client,
        }
    }

    /// Fetch and parse a sitemap URL.
    ///
    /// Returns the list of sitemap entries. Handles both `<urlset>` and
    /// `<sitemapindex>` formats. For indexes, recursively fetches child
    /// sitemaps (up to 2 levels deep).
    pub async fn fetch(&self, sitemap_url: &str) -> Vec<SitemapEntry> {
        self.fetch_inner(sitemap_url, 0).await
    }

    async fn fetch_inner(&self, sitemap_url: &str, depth: usize) -> Vec<SitemapEntry> {
        if depth > 2 {
            return Vec::new();
        }

        // Check cache
        if let Some(entries) = self.cache.get(sitemap_url) {
            return entries.clone();
        }

        // Fetch
        let url = match url::Url::parse(sitemap_url) {
            Ok(u) => u,
            Err(_) => return Vec::new(),
        };

        let body = match self.client.fetch(&url).await {
            Ok(result) if result.status_code == 200 => result.body,
            _ => return Vec::new(),
        };

        // Parse
        let entries = match parse_sitemap(&body) {
            Ok(SitemapContent::UrlSet(entries)) => entries,
            Ok(SitemapContent::SitemapIndex(child_urls)) => {
                // Recursively fetch child sitemaps
                let mut all_entries = Vec::new();
                for child_url in &child_urls {
                    let child_entries = Box::pin(self.fetch_inner(child_url, depth + 1)).await;
                    all_entries.extend(child_entries);
                }
                all_entries
            }
            Err(_) => return Vec::new(),
        };

        self.cache.insert(sitemap_url.to_string(), entries.clone());
        entries
    }

    /// Get all entries from multiple sitemap URLs.
    pub async fn fetch_all(&self, urls: &[String]) -> Vec<SitemapEntry> {
        let mut all = Vec::new();
        for url in urls {
            all.extend(self.fetch(url).await);
        }
        all
    }

    /// Get the total number of cached entries.
    pub fn cached_count(&self) -> usize {
        self.cache.iter().map(|e| e.value().len()).sum()
    }
}

/// Parsed sitemap content.
enum SitemapContent {
    UrlSet(Vec<SitemapEntry>),
    SitemapIndex(Vec<String>),
}

/// Parse sitemap XML content.
///
/// Detects `<urlset>` vs `<sitemapindex>` root element and dispatches
/// accordingly.
fn parse_sitemap(xml: &str) -> Result<SitemapContent, SitemapError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut root_tag = None;

    // Find root element
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                root_tag = Some(String::from_utf8_lossy(e.name().as_ref()).to_string());
                break;
            }
            Ok(Event::Eof) => break,
            Ok(_) => continue,
            Err(_) => return Err(SitemapError::InvalidXml),
        }
    }

    let root = root_tag.ok_or(SitemapError::InvalidXml)?;

    match root.as_str() {
        "urlset" => parse_urlset(reader, &mut buf),
        "sitemapindex" => parse_sitemapindex(reader, &mut buf),
        _ => Err(SitemapError::UnknownRoot),
    }
}

/// Parse a `<urlset>` sitemap.
fn parse_urlset(
    mut reader: Reader<&[u8]>,
    buf: &mut Vec<u8>,
) -> Result<SitemapContent, SitemapError> {
    let mut entries = Vec::new();
    let mut current_url: Option<String> = None;
    let mut current_lastmod: Option<String> = None;
    let mut current_changefreq: Option<String> = None;
    let mut current_priority: Option<f64> = None;
    let mut in_url = false;
    let mut current_tag = String::new();

    loop {
        buf.clear();
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if current_tag == "url" {
                    in_url = true;
                    current_url = None;
                    current_lastmod = None;
                    current_changefreq = None;
                    current_priority = None;
                }
            }
            Ok(Event::Text(e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if in_url {
                    match current_tag.as_str() {
                        "loc" => current_url = Some(text),
                        "lastmod" => current_lastmod = Some(text),
                        "changefreq" => current_changefreq = Some(text),
                        "priority" => {
                            current_priority = text.parse().ok();
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "url" && in_url {
                    if let Some(loc) = current_url.take() {
                        entries.push(SitemapEntry {
                            url: loc,
                            lastmod: current_lastmod.take(),
                            changefreq: current_changefreq.take(),
                            priority: current_priority.take(),
                        });
                    }
                    in_url = false;
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    Ok(SitemapContent::UrlSet(entries))
}

/// Parse a `<sitemapindex>` sitemap.
fn parse_sitemapindex(
    mut reader: Reader<&[u8]>,
    buf: &mut Vec<u8>,
) -> Result<SitemapContent, SitemapError> {
    let mut child_urls = Vec::new();
    let mut current_tag = String::new();

    loop {
        buf.clear();
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                if current_tag == "loc" {
                    child_urls.push(String::from_utf8_lossy(e.as_ref()).to_string());
                }
            }
            Ok(Event::End(_)) => {
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    Ok(SitemapContent::SitemapIndex(child_urls))
}

/// Sitemap parsing errors.
#[derive(Debug)]
pub enum SitemapError {
    InvalidXml,
    UnknownRoot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_urlset() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/</loc>
    <lastmod>2024-01-15</lastmod>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
  <url>
    <loc>https://example.com/about</loc>
    <lastmod>2024-01-10</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>
</urlset>"#;

        match parse_sitemap(xml).unwrap() {
            SitemapContent::UrlSet(entries) => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].url, "https://example.com/");
                assert_eq!(entries[0].lastmod, Some("2024-01-15".to_string()));
                assert_eq!(entries[0].changefreq, Some("daily".to_string()));
                assert_eq!(entries[0].priority, Some(1.0));
                assert_eq!(entries[1].url, "https://example.com/about");
            }
            _ => panic!("Expected UrlSet"),
        }
    }

    #[test]
    fn test_parse_sitemapindex() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap>
    <loc>https://example.com/sitemap-pages.xml</loc>
  </sitemap>
  <sitemap>
    <loc>https://example.com/sitemap-posts.xml</loc>
  </sitemap>
</sitemapindex>"#;

        match parse_sitemap(xml).unwrap() {
            SitemapContent::SitemapIndex(urls) => {
                assert_eq!(urls.len(), 2);
                assert_eq!(urls[0], "https://example.com/sitemap-pages.xml");
                assert_eq!(urls[1], "https://example.com/sitemap-posts.xml");
            }
            _ => panic!("Expected SitemapIndex"),
        }
    }

    #[test]
    fn test_parse_empty_urlset() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
</urlset>"#;

        match parse_sitemap(xml).unwrap() {
            SitemapContent::UrlSet(entries) => {
                assert_eq!(entries.len(), 0);
            }
            _ => panic!("Expected UrlSet"),
        }
    }
}
