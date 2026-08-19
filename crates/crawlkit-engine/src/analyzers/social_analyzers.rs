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
