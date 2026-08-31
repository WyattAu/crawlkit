//! Image accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks images for accessible alternative text.
pub struct ImageAccessibilityAnalyzer;

impl ImageAccessibilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub(crate) fn filename_from_src(src: &str) -> Option<&str> {
        src.rsplit('/').next().and_then(|s| {
            let s = s.trim_start_matches('\\');
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
    }
}

impl Default for ImageAccessibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ImageAccessibilityAnalyzer {
    fn name(&self) -> &str {
        "image-accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for img in &ctx.page.images {
            if !img.has_alt {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Accessibility,
                    code: "IMGACC001".to_string(),
                    title: "Image missing alt attribute".to_string(),
                    description: format!(
                        "Image \"{}\" has no alt attribute. Screen readers cannot convey \
                         the image content to visually impaired users.",
                        img.src
                    ),
                    url: url.to_string(),
                    recommendation: "Add an alt attribute to every <img> element. Use descriptive text for meaningful images and alt=\"\" for decorative ones."
                        .to_string(),
                });
            } else if img.alt.is_empty() && !img.aria_hidden {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "IMGACC002".to_string(),
                    title: "Image with empty alt on non-decorative image".to_string(),
                    description: format!(
                        "Image \"{}\" has alt=\"\" but is not marked as aria-hidden. If this image \
                         conveys meaningful content, it needs descriptive alt text. If decorative, add aria-hidden=\"true\".",
                        img.src
                    ),
                    url: url.to_string(),
                    recommendation: "For meaningful images, provide descriptive alt text. For decorative images, use alt=\"\" AND aria-hidden=\"true\"."
                        .to_string(),
                });
            } else if !img.alt.is_empty() {
                if let Some(filename) = Self::filename_from_src(&img.src) {
                    let filename_no_ext = filename.split('.').next().unwrap_or(filename);
                    let alt_lower = img.alt.trim().to_lowercase();
                    let filename_lower = filename_no_ext.to_lowercase();
                    if alt_lower == filename_lower {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Accessibility,
                            code: "IMGACC003".to_string(),
                            title: "Image alt text identical to filename".to_string(),
                            description: format!(
                                "Image \"{}\" has alt text \"{}\" which matches the filename. \
                                 Alt text should describe the image content, not repeat the file name.",
                                img.src, img.alt
                            ),
                            url: url.to_string(),
                            recommendation: "Replace the filename-based alt text with a description of what the image shows."
                                .to_string(),
                        });
                    }
                }
            }
        }

        findings
    }
}
