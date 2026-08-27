use serde::{Deserialize, Serialize};
use url::Url;

/// Open Graph protocol tags for social media previews.
///
/// Extracted from `<meta property="og:*">` tags. Used by the
/// [`SocialMediaAnalyzer`](crate::SocialMediaAnalyzer) to check
/// completeness of social sharing metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenGraphTags {
    /// `og:title` — title for social sharing.
    pub title: Option<String>,
    /// `og:description` — description for social sharing.
    pub description: Option<String>,
    /// `og:image` — image URL for social preview.
    pub image: Option<String>,
    /// `og:url` — canonical URL for social sharing.
    pub url: Option<String>,
    /// `og:type` — content type (website, article, etc.).
    pub r#type: Option<String>,
    /// `og:site_name` — site name.
    pub site_name: Option<String>,
    /// `og:locale` — content locale (e.g., "en_US").
    pub locale: Option<String>,
    /// Additional OG properties not covered by named fields (e.g., `og:video`, `og:video:url`).
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub extra: std::collections::HashMap<String, String>,
}

impl OpenGraphTags {
    /// Get an OG property by key, checking named fields first then extras.
    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "title" => self.title.as_deref(),
            "description" => self.description.as_deref(),
            "image" => self.image.as_deref(),
            "url" => self.url.as_deref(),
            "type" => self.r#type.as_deref(),
            "site_name" => self.site_name.as_deref(),
            "locale" => self.locale.as_deref(),
            _ => self.extra.get(key).map(|s| s.as_str()),
        }
    }

    /// Insert an OG property, using named fields for known keys and extras for others.
    pub fn insert(&mut self, key: String, value: String) {
        match key.as_str() {
            "title" => self.title = Some(value),
            "description" => self.description = Some(value),
            "image" => self.image = Some(value),
            "url" => self.url = Some(value),
            "type" => self.r#type = Some(value),
            "site_name" => self.site_name = Some(value),
            "locale" => self.locale = Some(value),
            _ => {
                self.extra.insert(key, value);
            }
        }
    }
}

/// Twitter Card tags for Twitter/X previews.
///
/// Extracted from `<meta name="twitter:*">` tags. Used by the
/// [`SocialMediaAnalyzer`](crate::SocialMediaAnalyzer) to check
/// Twitter Card completeness.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TwitterTags {
    /// `twitter:card` — card type (summary, summary_large_image).
    pub card: Option<String>,
    /// `twitter:site` — site Twitter handle.
    pub site: Option<String>,
    /// `twitter:creator` — author Twitter handle.
    pub creator: Option<String>,
    /// `twitter:title` — title for Twitter sharing.
    pub title: Option<String>,
    /// `twitter:description` — description for Twitter sharing.
    pub description: Option<String>,
    /// `twitter:image` — image URL for Twitter card.
    pub image: Option<String>,
    /// `twitter:image:alt` — alt text for Twitter card image.
    pub image_alt: Option<String>,
    /// `twitter:player` — embedded player URL for player cards.
    pub player: Option<String>,
    /// `twitter:player:stream` — video stream URL for player cards.
    pub player_stream: Option<String>,
}

/// A single hreflang alternate link.
///
/// Represents a `<link rel="alternate" hreflang="..." href="...">` tag
/// for international SEO. Validated by the [`HreflangValidator`](crate::HreflangValidator).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HreflangTag {
    /// Language code (e.g., "en", "fr-CA", "x-default").
    pub lang: String,
    /// URL of the alternate language version.
    pub url: Url,
}

/// Complete set of meta tags extracted from a page.
///
/// Aggregates all SEO-relevant meta information including title,
/// description, canonical URL, robots directives, Open Graph,
/// Twitter Cards, and hreflang tags. Provides helper methods
/// for common checks.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::MetaTags;
///
/// let meta = MetaTags {
///     title: Some("My Page".to_string()),
///     robots: Some("noindex, nofollow".to_string()),
///     ..Default::default()
/// };
/// assert!(meta.is_noindex());
/// assert!(meta.is_nofollow());
/// assert_eq!(meta.title_length(), Some(7));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetaTags {
    /// Page title (`<title>` tag).
    pub title: Option<String>,
    /// Meta description.
    pub description: Option<String>,
    /// Canonical URL (`<link rel="canonical">`).
    pub canonical: Option<Url>,
    /// Robots directive (`<meta name="robots">`).
    pub robots: Option<String>,
    /// Page language (`<html lang="...">`).
    pub language: Option<String>,
    /// Character encoding (`<meta charset="...">`).
    pub charset: Option<String>,
    /// Viewport meta tag.
    pub viewport: Option<String>,
    /// Open Graph tags.
    pub og: OpenGraphTags,
    /// Twitter Card tags.
    pub twitter: TwitterTags,
    /// Hreflang alternate links.
    pub hreflang: Vec<HreflangTag>,
}

impl MetaTags {
    /// Returns the length of the title, if present.
    pub fn title_length(&self) -> Option<usize> {
        self.title.as_ref().map(|t| t.len())
    }

    /// Returns the length of the description, if present.
    pub fn description_length(&self) -> Option<usize> {
        self.description.as_ref().map(|d| d.len())
    }

    /// Whether the page has a `<meta name="robots" content="noindex">` directive.
    pub fn is_noindex(&self) -> bool {
        self.robots
            .as_deref()
            .map(|r| r.split(',').any(|v| v.trim() == "noindex"))
            .unwrap_or(false)
    }

    /// Whether the page has a `<meta name="robots" content="nofollow">` directive.
    pub fn is_nofollow(&self) -> bool {
        self.robots
            .as_deref()
            .map(|r| r.split(',').any(|v| v.trim() == "nofollow"))
            .unwrap_or(false)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_tags_defaults() {
        let meta = MetaTags::default();
        assert!(meta.title.is_none());
        assert!(meta.description.is_none());
        assert!(meta.canonical.is_none());
        assert!(meta.og.title.is_none());
        assert!(meta.twitter.card.is_none());
        assert!(meta.hreflang.is_empty());
    }

    #[test]
    fn test_title_length() {
        let mut meta = MetaTags::default();
        assert_eq!(meta.title_length(), None);

        meta.title = Some("Hello World".into());
        assert_eq!(meta.title_length(), Some(11));
    }

    #[test]
    fn test_description_length() {
        let mut meta = MetaTags::default();
        assert_eq!(meta.description_length(), None);

        meta.description = Some("A short description".into());
        assert_eq!(meta.description_length(), Some(19));
    }

    #[test]
    fn test_is_noindex() {
        let mut meta = MetaTags::default();
        assert!(!meta.is_noindex());

        meta.robots = Some("noindex".into());
        assert!(meta.is_noindex());

        meta.robots = Some("index, nofollow".into());
        assert!(!meta.is_noindex());

        meta.robots = Some("noindex, nofollow".into());
        assert!(meta.is_noindex());
    }

    #[test]
    fn test_is_nofollow() {
        let mut meta = MetaTags::default();
        assert!(!meta.is_nofollow());

        meta.robots = Some("nofollow".into());
        assert!(meta.is_nofollow());

        meta.robots = Some("noindex, nofollow".into());
        assert!(meta.is_nofollow());
    }

    #[test]
    fn test_open_graph_serialization() {
        let og = OpenGraphTags {
            title: Some("My Page".into()),
            description: Some("Desc".into()),
            image: Some("https://example.com/img.png".into()),
            url: Some("https://example.com".into()),
            r#type: Some("website".into()),
            site_name: Some("Example".into()),
            locale: Some("en_US".into()),
            extra: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&og).unwrap();
        let deser: OpenGraphTags = serde_json::from_str(&json).unwrap();
        assert_eq!(og.title, deser.title);
        assert_eq!(og.image, deser.image);
    }

    #[test]
    fn test_twitter_tags_serialization() {
        let tw = TwitterTags {
            card: Some("summary_large_image".into()),
            site: Some("@example".into()),
            creator: Some("@author".into()),
            title: Some("Title".into()),
            description: Some("Desc".into()),
            image: Some("https://example.com/tw.png".into()),
            image_alt: Some("Alt text".into()),
            player: None,
            player_stream: None,
        };
        let json = serde_json::to_string(&tw).unwrap();
        let deser: TwitterTags = serde_json::from_str(&json).unwrap();
        assert_eq!(tw.card, deser.card);
        assert_eq!(tw.image_alt, deser.image_alt);
    }

    #[test]
    fn test_hreflang_tag() {
        let tag = HreflangTag {
            lang: "en".into(),
            url: Url::parse("https://example.com/en").unwrap(),
        };
        assert_eq!(tag.lang, "en");
        let json = serde_json::to_string(&tag).unwrap();
        let deser: HreflangTag = serde_json::from_str(&json).unwrap();
        assert_eq!(tag.url, deser.url);
    }
}
