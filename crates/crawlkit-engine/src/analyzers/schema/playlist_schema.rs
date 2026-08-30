#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct PlaylistSchemaValidator;

impl Default for PlaylistSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaylistSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PlaylistSchemaValidator {
    fn name(&self) -> &str {
        "playlist-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Playlist") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PLAYLIST001".to_string(),
                    title: "Playlist schema missing name".to_string(),
                    description: "A Playlist structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the playlist title.".to_string(),
                });
            }

            let num_tracks = data.get("numberOfItems").and_then(|v| v.as_u64());
            if num_tracks == Some(0) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PLAYLIST002".to_string(),
                    title: "Playlist schema has zero tracks".to_string(),
                    description: "The Playlist has numberOfItems set to 0. A playlist should have at least one track."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add tracks or set numberOfItems to a positive value.".to_string(),
                });
            }

            if data.get("numTracks").is_none() && data.get("track").is_none()
                && data.get("numberOfItems").is_none()
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PLAYLIST003".to_string(),
                    title: "Playlist schema missing track information".to_string(),
                    description: "The Playlist has no track or numberOfItems property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"track\" or \"numberOfItems\" to describe playlist content."
                        .to_string(),
                });
            }
        }

        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::StructuredData;

    fn make_page(url: &str) -> crate::parser::ParsedPage {
        crate::parser::ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a crate::parser::ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    #[test]
    fn test_playlist_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playlist".to_string()),
            data: serde_json::json!({"@type": "Playlist"}),
        }];
        assert!(PlaylistSchemaValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "PLAYLIST001"));
    }

    #[test]
    fn test_playlist_zero_tracks() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playlist".to_string()),
            data: serde_json::json!({"@type": "Playlist", "name": "My List", "numberOfItems": 0}),
        }];
        assert!(PlaylistSchemaValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "PLAYLIST002"));
    }

    #[test]
    fn test_playlist_missing_track_info() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playlist".to_string()),
            data: serde_json::json!({"@type": "Playlist", "name": "My List"}),
        }];
        assert!(PlaylistSchemaValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "PLAYLIST003"));
    }

    #[test]
    fn test_playlist_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playlist".to_string()),
            data: serde_json::json!({"@type": "Playlist", "name": "My List", "numberOfItems": 5}),
        }];
        assert!(PlaylistSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_playlist_non_playlist_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        assert!(PlaylistSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_playlist_no_data() {
        let page = make_page("https://example.com");
        assert!(PlaylistSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_playlist_with_track_array() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playlist".to_string()),
            data: serde_json::json!({"@type": "Playlist", "name": "My List", "track": [{"@type": "MusicRecording"}]}),
        }];
        assert!(PlaylistSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }

    #[test]
    fn test_playlist_all_issues() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playlist".to_string()),
            data: serde_json::json!({"@type": "Playlist"}),
        }];
        let findings = PlaylistSchemaValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "PLAYLIST001"));
        assert!(findings.iter().any(|f| f.code == "PLAYLIST003"));
    }

    #[test]
    fn test_playlist_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playlist".to_string()),
            data: serde_json::json!({"@type": "Playlist", "name": ""}),
        }];
        assert!(PlaylistSchemaValidator::new().analyze(&make_ctx(&page)).iter().any(|f| f.code == "PLAYLIST001"));
    }

    #[test]
    fn test_playlist_with_num_tracks_property() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playlist".to_string()),
            data: serde_json::json!({"@type": "Playlist", "name": "My List", "numTracks": 10}),
        }];
        assert!(PlaylistSchemaValidator::new().analyze(&make_ctx(&page)).is_empty());
    }
}
