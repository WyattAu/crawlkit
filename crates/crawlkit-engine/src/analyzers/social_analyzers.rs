#![allow(clippy::unwrap_used, clippy::manual_range_contains, clippy::redundant_closure)]
use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------
// 18. Social Media Analyzer
// ---------------------------------------------------------------------------

pub struct SocialMediaAnalyzer;

impl SocialMediaAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Minimum recommended OG image width (pixels).
    const OG_MIN_WIDTH: u32 = 1200;
    /// Minimum recommended OG image height (pixels).
    const OG_MIN_HEIGHT: u32 = 630;
}

impl Default for SocialMediaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SocialMediaAnalyzer {
    fn name(&self) -> &str {
        "social-media"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // --- OG image dimensions ---
        if ctx.page.og_image_width.is_none() && ctx.page.og_image_height.is_none() {
            // No OG image dimensions at all — check if there's an OG image
            if ctx.page.meta.og.image.is_some() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Social,
                    code: "SOCIAL001".to_string(),
                    title: "OG image missing dimensions".to_string(),
                    description: "og:image is set but og:image:width and og:image:height \
                                  are missing. Social platforms may crop or scale the image \
                                  incorrectly."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add <meta property=\"og:image:width\" content=\"1200\"> \
                                     and <meta property=\"og:image:height\" content=\"630\">."
                        .to_string(),
                });
            }
        } else {
            // Check dimensions meet minimum requirements
            if let Some(width) = ctx.page.og_image_width {
                if width < Self::OG_MIN_WIDTH {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "SOCIAL002".to_string(),
                        title: "OG image too narrow".to_string(),
                        description: format!(
                            "og:image:width is {width}px, below the recommended minimum of \
                             {}px.",
                            Self::OG_MIN_WIDTH
                        ),
                        url: url.clone(),
                        recommendation: format!(
                            "Use an image at least {}x{} pixels for optimal social previews.",
                            Self::OG_MIN_WIDTH,
                            Self::OG_MIN_HEIGHT
                        ),
                    });
                }
            }
            if let Some(height) = ctx.page.og_image_height {
                if height < Self::OG_MIN_HEIGHT {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "SOCIAL003".to_string(),
                        title: "OG image too short".to_string(),
                        description: format!(
                            "og:image:height is {height}px, below the recommended minimum of \
                             {}px.",
                            Self::OG_MIN_HEIGHT
                        ),
                        url: url.clone(),
                        recommendation: format!(
                            "Use an image at least {}x{} pixels for optimal social previews.",
                            Self::OG_MIN_WIDTH,
                            Self::OG_MIN_HEIGHT
                        ),
                    });
                }
            }
        }

        // --- Twitter Card type ---
        match &ctx.page.meta.twitter.card {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Social,
                    code: "SOCIAL004".to_string(),
                    title: "Missing Twitter Card type".to_string(),
                    description: "No twitter:card meta tag found. Twitter/X will not render \
                                  a rich preview without it."
                        .to_string(),
                    url: url.clone(),
                    recommendation:
                        "Add <meta name=\"twitter:card\" content=\"summary_large_image\">."
                            .to_string(),
                });
            }
            Some(card_type) => {
                let valid_types = ["summary", "summary_large_image", "app", "player"];
                if !valid_types.contains(&card_type.as_str()) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "SOCIAL005".to_string(),
                        title: "Invalid Twitter Card type".to_string(),
                        description: format!(
                            "twitter:card value \"{card_type}\" is not a recognized card type."
                        ),
                        url: url.clone(),
                        recommendation: "Use one of: summary, summary_large_image, app, player."
                            .to_string(),
                    });
                }
            }
        }

        // --- Social preview completeness ---
        let og_required = [
            ("og:title", ctx.page.meta.og.title.is_some()),
            ("og:description", ctx.page.meta.og.description.is_some()),
            ("og:image", ctx.page.meta.og.image.is_some()),
            ("og:url", ctx.page.meta.og.url.is_some()),
            ("og:type", ctx.page.meta.og.r#type.is_some()),
        ];

        let missing_og: Vec<&str> = og_required
            .iter()
            .filter(|(_, present)| !present)
            .map(|(name, _)| *name)
            .collect();

        if !missing_og.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Social,
                code: "SOCIAL006".to_string(),
                title: "Incomplete Open Graph tags".to_string(),
                description: format!(
                    "Missing OG tags: {}. Social previews may be incomplete.",
                    missing_og.join(", ")
                ),
                url: url.clone(),
                recommendation: "Add all required OG tags for complete social media previews."
                    .to_string(),
            });
        }

        let twitter_required = [
            ("twitter:title", ctx.page.meta.twitter.title.is_some()),
            (
                "twitter:description",
                ctx.page.meta.twitter.description.is_some(),
            ),
            ("twitter:image", ctx.page.meta.twitter.image.is_some()),
        ];

        let missing_twitter: Vec<&str> = twitter_required
            .iter()
            .filter(|(_, present)| !present)
            .map(|(name, _)| *name)
            .collect();

        if !missing_twitter.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Social,
                code: "SOCIAL007".to_string(),
                title: "Incomplete Twitter Card tags".to_string(),
                description: format!(
                    "Missing Twitter tags: {}. Twitter/X previews may fall back to OG tags.",
                    missing_twitter.join(", ")
                ),
                url: url.clone(),
                recommendation: "Add Twitter-specific tags for optimal X/Twitter previews."
                    .to_string(),
            });
        }

        // --- Social preview summary ---
        let og_score = og_required.iter().filter(|(_, p)| *p).count();
        let twitter_score = twitter_required.iter().filter(|(_, p)| *p).count();
        let total = og_required.len() + twitter_required.len();
        let score = og_score + twitter_score;

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Social,
            code: "SOCIAL008".to_string(),
            title: "Social preview completeness score".to_string(),
            description: format!(
                "Social metadata score: {score}/{total} (OG: {og_score}/{og_len}, Twitter: \
                 {twitter_score}/{tw_len}).",
                og_len = og_required.len(),
                tw_len = twitter_required.len(),
            ),
            url: url.clone(),
            recommendation: if score < total {
                "Add missing social meta tags to improve how your page appears when shared."
                    .to_string()
            } else {
                "Social metadata is complete.".to_string()
            },
        });

        findings
    }
}

// ---------------------------------------------------------------------------
// Open Graph Image Validator
// ---------------------------------------------------------------------------

pub struct OpenGraphImageValidator;

impl OpenGraphImageValidator {
    pub fn new() -> Self {
        Self
    }

    /// Check if an image URL has a supported format extension.
    fn is_supported_format(url: &str) -> Option<&'static str> {
        let lower = url.to_lowercase();
        // Strip query parameters and fragments for extension detection
        let path = lower.split(['?', '#']).next().unwrap_or(&lower);
        let ext = path.rsplit('.').next()?;
        match ext {
            "jpg" | "jpeg" => Some("jpeg"),
            "png" => Some("png"),
            "gif" => Some("gif"),
            "webp" => Some("webp"),
            "avif" => Some("avif"),
            "svg" => Some("svg"),
            "bmp" => Some("bmp"),
            "tiff" | "tif" => Some("tiff"),
            _ => None,
        }
    }

    /// Validate an image URL for basic structural correctness.
    fn is_valid_image_url(url: &str) -> bool {
        if url.is_empty() {
            return false;
        }
        // Must be a valid URL
        if let Ok(parsed) = url::Url::parse(url) {
            matches!(parsed.scheme(), "http" | "https" | "data")
        } else {
            false
        }
    }

    /// Check if the image URL is a data URI.
    fn is_data_uri(url: &str) -> bool {
        url.starts_with("data:")
    }

    /// Extract MIME type from a data URI.
    fn data_uri_mime_type(url: &str) -> Option<&str> {
        if let Some(rest) = url.strip_prefix("data:") {
            rest.split(';').next()
        } else {
            None
        }
    }

    /// Check if a MIME type is a supported image format.
    fn is_supported_mime(mime: &str) -> bool {
        matches!(
            mime,
            "image/jpeg"
                | "image/png"
                | "image/gif"
                | "image/webp"
                | "image/avif"
                | "image/svg+xml"
                | "image/bmp"
                | "image/tiff"
        )
    }
}

impl Default for OpenGraphImageValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for OpenGraphImageValidator {
    fn name(&self) -> &str {
        "og-image-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let og_image = match &ctx.page.meta.og.image {
            Some(img) => img,
            None => return findings, // No OG image — handled by SocialMediaAnalyzer
        };

        // OGIMG001: OG image tag present but URL is invalid
        if !Self::is_valid_image_url(og_image) {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Social,
                code: "OGIMG001".to_string(),
                title: "Invalid OG image URL".to_string(),
                description: format!(
                    "og:image contains an invalid URL: \"{og_image}\". Social platforms \
                     will not be able to display this image."
                ),
                url: url.clone(),
                recommendation: "Set og:image to a valid, publicly accessible HTTP or HTTPS \
                                 image URL."
                    .to_string(),
            });
            return findings;
        }

        // OGIMG002: OG image width/height missing or wrong
        let has_width = ctx.page.og_image_width.is_some();
        let has_height = ctx.page.og_image_height.is_some();

        if !has_width && !has_height {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Social,
                code: "OGIMG002".to_string(),
                title: "OG image missing dimensions".to_string(),
                description: "og:image is present but og:image:width and og:image:height \
                              are both missing. Social platforms may not render the image \
                              correctly."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <meta property=\"og:image:width\" content=\"1200\"> \
                                 and <meta property=\"og:image:height\" content=\"630\">."
                    .to_string(),
            });
        } else {
            // Check width
            if let Some(width) = ctx.page.og_image_width {
                if width == 0 {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Social,
                        code: "OGIMG002".to_string(),
                        title: "OG image width is zero".to_string(),
                        description: "og:image:width is set to 0, which is invalid."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Set og:image:width to the actual image width in pixels."
                            .to_string(),
                    });
                } else if width < 200 || width > 10000 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "OGIMG002".to_string(),
                        title: "OG image width outside expected range".to_string(),
                        description: format!(
                            "og:image:width is {width}px. Expected width is between 200 and \
                             10000 pixels for social media images."
                        ),
                        url: url.clone(),
                        recommendation: "Set og:image:width to the actual image width in pixels \
                                         (recommended: 1200px for optimal sharing)."
                            .to_string(),
                    });
                }
            }
            // Check height
            if let Some(height) = ctx.page.og_image_height {
                if height == 0 {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Social,
                        code: "OGIMG002".to_string(),
                        title: "OG image height is zero".to_string(),
                        description: "og:image:height is set to 0, which is invalid."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Set og:image:height to the actual image height in pixels."
                            .to_string(),
                    });
                } else if height < 200 || height > 10000 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "OGIMG002".to_string(),
                        title: "OG image height outside expected range".to_string(),
                        description: format!(
                            "og:image:height is {height}px. Expected height is between 200 and \
                             10000 pixels for social media images."
                        ),
                        url: url.clone(),
                        recommendation: "Set og:image:height to the actual image height in pixels \
                                         (recommended: 630px for optimal sharing)."
                            .to_string(),
                    });
                }
            }
            // Check for mismatched width/height ratio
            if let (Some(w), Some(h)) = (ctx.page.og_image_width, ctx.page.og_image_height) {
                if w > 0 && h > 0 {
                    let ratio = w as f64 / h as f64;
                    // Standard OG ratio is ~1.91:1 (1200x630). Allow 0.5 to 4.0.
                    if ratio < 0.5 || ratio > 4.0 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Social,
                            code: "OGIMG002".to_string(),
                            title: "OG image aspect ratio is unusual".to_string(),
                            description: format!(
                                "OG image dimensions {w}x{h} give an aspect ratio of \
                                 {ratio:.2}:1. Social platforms typically use ~1.91:1 \
                                 (1200x630)."
                            ),
                            url: url.clone(),
                            recommendation: "Use an image with a ~1.91:1 aspect ratio \
                                             (e.g., 1200x630) for optimal social previews."
                                .to_string(),
                        });
                    }
                }
            }
        }

        // OGIMG003: OG image format not supported
        if Self::is_data_uri(og_image) {
            // Data URI — check MIME type
            if let Some(mime) = Self::data_uri_mime_type(og_image) {
                if !Self::is_supported_mime(mime) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "OGIMG003".to_string(),
                        title: "OG image format not supported".to_string(),
                        description: format!(
                            "og:image uses a data URI with MIME type \"{mime}\" which may \
                             not be supported by all social platforms."
                        ),
                        url: url.clone(),
                        recommendation: "Use a standard image format (JPEG, PNG, GIF, or WebP) \
                                         hosted at a publicly accessible URL."
                            .to_string(),
                    });
                }
            }
        } else if let Some(format) = Self::is_supported_format(og_image) {
            // Known format — check if it's SVG (which social platforms don't support well)
            if format == "svg" {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Social,
                    code: "OGIMG003".to_string(),
                    title: "OG image is SVG format".to_string(),
                    description: "og:image points to an SVG image. Most social platforms do \
                                  not support SVG for social previews."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Use a raster image format (JPEG, PNG, or GIF) instead of \
                                     SVG for og:image."
                        .to_string(),
                });
            }
        } else {
            // No recognized extension — could be an issue
            // Only flag if there IS an extension (missing extension is common for CDN URLs)
            let path = og_image.split(['?', '#']).next().unwrap_or(og_image);
            if path.rsplit('.').next().is_some_and(|ext| !ext.is_empty()) {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Social,
                    code: "OGIMG003".to_string(),
                    title: "OG image format may be unsupported".to_string(),
                    description: format!(
                        "og:image URL \"{og_image}\" has an unrecognized file extension. \
                         Social platforms may not render this image."
                    ),
                    url: url.clone(),
                    recommendation: "Use a standard image format (JPEG, PNG, GIF, or WebP) \
                                     for og:image."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Twitter Player Validator
// ---------------------------------------------------------------------------

pub struct TwitterPlayerValidator;

impl TwitterPlayerValidator {
    pub fn new() -> Self {
        Self
    }

    /// Check if a URL has valid video dimensions encoded as WxH.
    fn has_valid_stream_dimensions(url: &str) -> bool {
        if url.is_empty() {
            return false;
        }
        let lower = url.to_lowercase();
        let has_width = lower.contains("width=") || lower.contains("width%3d");
        let has_height = lower.contains("height=") || lower.contains("height%3d");
        let has_wxh = lower.contains("640x") || lower.contains("480x") || lower.contains("854x");
        has_width || has_height || has_wxh || url.contains('#')
    }
}

impl Default for TwitterPlayerValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TwitterPlayerValidator {
    fn name(&self) -> &str {
        "twitter-player"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let has_player = ctx.page.meta.twitter.player.is_some();
        let has_stream = ctx.page.meta.twitter.player_stream.is_some();
        let card_type = ctx.page.meta.twitter.card.as_deref();

        // Only check player tags when the card type is "player"
        if card_type != Some("player") {
            return findings;
        }

        // TWPL001: twitter:player present but twitter:player:stream missing
        if has_player && !has_stream {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Social,
                code: "TWPL001".to_string(),
                title: "twitter:player:stream missing".to_string(),
                description: "A twitter:player tag is present but twitter:player:stream is \
                              missing. The player:stream URL is required for the player card \
                              to render the video content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <meta name=\"twitter:player:stream\" content=\"URL_TO_VIDEO\"> \
                                 with a direct URL to the video file (MP4 recommended)."
                    .to_string(),
            });
        }

        // TWPL002: twitter:player:stream dimensions invalid or missing
        if has_stream {
            if let Some(stream_url) = &ctx.page.meta.twitter.player_stream {
                if !Self::has_valid_stream_dimensions(stream_url) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Social,
                        code: "TWPL002".to_string(),
                        title: "twitter:player:stream missing dimensions".to_string(),
                        description: "The twitter:player:stream URL does not include valid \
                                      video dimensions. Twitter requires explicit width and \
                                      height for player cards to render correctly."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Include width and height in the player:stream URL or \
                                         ensure the video player page specifies dimensions. \
                                         Twitter recommends 640x480 minimum."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// SocialPreviewOptimizer
// =========================================================================

/// Validates social preview metadata (OG tags) for completeness and correctness.
pub struct SocialPreviewOptimizer;

impl Default for SocialPreviewOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SocialPreviewOptimizer {
    pub fn new() -> Self {
        Self
    }

    /// Validate an image URL for basic structural correctness.
    fn is_valid_image_url(url: &str) -> bool {
        if url.is_empty() {
            return false;
        }
        if let Ok(parsed) = url::Url::parse(url) {
            matches!(parsed.scheme(), "http" | "https" | "data")
        } else {
            false
        }
    }
}

impl Analyzer for SocialPreviewOptimizer {
    fn name(&self) -> &str {
        "social-preview-optimizer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // SPREV001: OG title missing
        if ctx.page.meta.og.title.is_none() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Social,
                code: "SPREV001".to_string(),
                title: "OG title missing".to_string(),
                description: "No og:title meta tag was found. Social platforms use this to \
                              display the page title when shared."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <meta property=\"og:title\" content=\"Your Page Title\"> \
                                 for consistent social sharing titles."
                    .to_string(),
            });
        }

        // SPREV002: OG description missing
        if ctx.page.meta.og.description.is_none() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Social,
                code: "SPREV002".to_string(),
                title: "OG description missing".to_string(),
                description: "No og:description meta tag was found. Social platforms use this \
                              to display a preview snippet when shared."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <meta property=\"og:description\" content=\"A brief \
                                 description\"> for richer social previews."
                    .to_string(),
            });
        }

        // SPREV003: OG image URL invalid
        if let Some(og_image) = &ctx.page.meta.og.image {
            if !Self::is_valid_image_url(og_image) {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Social,
                    code: "SPREV003".to_string(),
                    title: "OG image URL invalid".to_string(),
                    description: format!(
                        "og:image contains an invalid URL: \"{}\". Social platforms will \
                         not be able to display this image.",
                        og_image
                    ),
                    url: url.clone(),
                    recommendation: "Set og:image to a valid, publicly accessible HTTP or HTTPS \
                                     image URL (minimum 1200x630 pixels recommended)."
                        .to_string(),
                });
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests_social_preview {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
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

    fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
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
    fn test_sprev001_og_title_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SPREV001"));
    }

    #[test]
    fn test_sprev001_og_title_present() {
        let mut page = make_page("https://example.com");
        page.meta.og.title = Some("My Page".to_string());
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SPREV001"));
    }

    #[test]
    fn test_sprev002_og_description_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SPREV002"));
    }

    #[test]
    fn test_sprev002_og_description_present() {
        let mut page = make_page("https://example.com");
        page.meta.og.description = Some("A description".to_string());
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SPREV002"));
    }

    #[test]
    fn test_sprev003_og_image_invalid() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = Some("not-a-valid-url".to_string());
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SPREV003"));
    }

    #[test]
    fn test_sprev003_og_image_valid() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = Some("https://example.com/image.png".to_string());
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SPREV003"));
    }

    #[test]
    fn test_sprev003_og_image_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        // No image = no SPREV003 (handled by SPREV001/SPREV002)
        assert!(!findings.iter().any(|f| f.code == "SPREV003"));
    }

    #[test]
    fn test_sprev_all_present() {
        let mut page = make_page("https://example.com");
        page.meta.og.title = Some("Title".to_string());
        page.meta.og.description = Some("Desc".to_string());
        page.meta.og.image = Some("https://example.com/img.png".to_string());
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_sprev003_empty_url() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = Some(String::new());
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SPREV003"));
    }

    #[test]
    fn test_sprev003_data_uri() {
        let mut page = make_page("https://example.com");
        page.meta.og.image = Some("data:image/png;base64,abc".to_string());
        let ctx = make_ctx(&page);
        let findings = SocialPreviewOptimizer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SPREV003"));
    }

    #[test]
    fn test_is_valid_image_url_cases() {
        assert!(SocialPreviewOptimizer::is_valid_image_url("https://example.com/img.png"));
        assert!(SocialPreviewOptimizer::is_valid_image_url("http://example.com/img.jpg"));
        assert!(SocialPreviewOptimizer::is_valid_image_url("data:image/png;base64,abc"));
        assert!(!SocialPreviewOptimizer::is_valid_image_url(""));
        assert!(!SocialPreviewOptimizer::is_valid_image_url("not-a-url"));
        assert!(!SocialPreviewOptimizer::is_valid_image_url("ftp://example.com/img.png"));
    }
}
