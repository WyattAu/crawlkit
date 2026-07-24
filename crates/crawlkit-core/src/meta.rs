use serde::{Deserialize, Serialize};
use url::Url;

/// Open Graph protocol tags for social media previews.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenGraphTags {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub url: Option<String>,
    pub r#type: Option<String>,
    pub site_name: Option<String>,
    pub locale: Option<String>,
}

/// Twitter Card tags for Twitter/X previews.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TwitterTags {
    pub card: Option<String>,
    pub site: Option<String>,
    pub creator: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub image_alt: Option<String>,
}

/// A single hreflang alternate link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HreflangTag {
    pub lang: String,
    pub url: Url,
}

/// Complete set of meta tags extracted from a page.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetaTags {
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical: Option<Url>,
    pub robots: Option<String>,
    pub language: Option<String>,
    pub charset: Option<String>,
    pub viewport: Option<String>,
    pub og: OpenGraphTags,
    pub twitter: TwitterTags,
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
