#![allow(clippy::unwrap_used)]

//! Font-size and line-height accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use regex::Regex;

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks inline and stylesheet font sizes and line heights for accessibility.
pub struct FontSizeAnalyzer;

impl FontSizeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn parse_font_size_px(value: &str) -> Option<f64> {
        let trimmed = value.trim().to_lowercase();
        if let Some(px) = trimmed.strip_suffix("px") {
            return px.trim().parse::<f64>().ok();
        }
        if let Some(pt) = trimmed.strip_suffix("pt") {
            return pt.trim().parse::<f64>().ok().map(|v| v * 96.0 / 72.0);
        }
        None
    }

    fn parse_line_height(value: &str) -> Option<f64> {
        let trimmed = value.trim();
        trimmed.parse::<f64>().ok()
    }

    fn extract_inline_font_sizes(html: &str) -> Vec<f64> {
        let Ok(re) = Regex::new(r#"style\s*=\s*["'][^"']*font-size\s*:\s*([^;"']+)["']"#) else {
            return Vec::new();
        };
        let mut sizes = Vec::new();
        for cap in re.captures_iter(html) {
            if let Some(px) = Self::parse_font_size_px(&cap[1]) {
                sizes.push(px);
            }
        }
        sizes
    }

    fn extract_style_block_font_sizes(html: &str) -> Vec<f64> {
        let Ok(re) = Regex::new(r#"font-size\s*:\s*([^;}]+)"#) else {
            return Vec::new();
        };
        let mut sizes = Vec::new();
        for cap in re.captures_iter(html) {
            if let Some(px) = Self::parse_font_size_px(&cap[1]) {
                sizes.push(px);
            }
        }
        sizes
    }

    fn extract_inline_line_heights(html: &str) -> Vec<f64> {
        let Ok(re) = Regex::new(r#"style\s*=\s*["'][^"']*line-height\s*:\s*([^;"']+)["']"#) else {
            return Vec::new();
        };
        let mut heights = Vec::new();
        for cap in re.captures_iter(html) {
            if let Some(lh) = Self::parse_line_height(&cap[1]) {
                heights.push(lh);
            }
        }
        heights
    }

    fn extract_style_block_line_heights(html: &str) -> Vec<f64> {
        let Ok(re) = Regex::new(r#"line-height\s*:\s*([^;}]+)"#) else {
            return Vec::new();
        };
        let mut heights = Vec::new();
        for cap in re.captures_iter(html) {
            if let Some(lh) = Self::parse_line_height(&cap[1]) {
                heights.push(lh);
            }
        }
        heights
    }

    fn check_small_font_sizes(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let body = ctx.body.unwrap_or("");
        let mut all_sizes: Vec<f64> = Self::extract_inline_font_sizes(body);
        all_sizes.extend(Self::extract_style_block_font_sizes(body));
        let small: Vec<f64> = all_sizes
            .into_iter()
            .filter(|&s| s > 0.0 && s < 12.0)
            .collect();
        if !small.is_empty() {
            let examples: Vec<String> = small.iter().take(5).map(|s| format!("{s:.0}px")).collect();
            f.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FSIZE001".to_string(),
                title: "Text smaller than 12px detected".to_string(),
                description: format!(
                    "{} element(s) have font-size below 12px (e.g., {}). WCAG 1.4.4 requires \
                     text to be resizable up to 200% without loss of content or functionality.",
                    small.len(),
                    examples.join(", ")
                ),
                url: url.to_string(),
                recommendation: "Use a minimum font size of 12px (0.75rem) for body text. Use \
                                 relative units (rem, em) so text can be resized by the user."
                    .to_string(),
            });
        }
    }

    fn check_line_height(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let body = ctx.body.unwrap_or("");
        let mut all_heights: Vec<f64> = Self::extract_inline_line_heights(body);
        all_heights.extend(Self::extract_style_block_line_heights(body));
        let insufficient: Vec<f64> = all_heights
            .into_iter()
            .filter(|&lh| lh > 0.0 && lh < 1.5)
            .collect();
        if !insufficient.is_empty() {
            let examples: Vec<String> = insufficient
                .iter()
                .take(5)
                .map(|lh| format!("{lh:.1}"))
                .collect();
            f.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FSIZE002".to_string(),
                title: "Insufficient line-height for body text".to_string(),
                description: format!(
                    "{} element(s) have line-height below 1.5 (e.g., {}). WCAG 1.4.12 recommends \
                     a line-height of at least 1.5 times the font size for body text.",
                    insufficient.len(),
                    examples.join(", ")
                ),
                url: url.to_string(),
                recommendation: "Set line-height to at least 1.5 for body text and 1.5 times \
                                 the font size for headings."
                    .to_string(),
            });
        }
    }
}

impl Default for FontSizeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FontSizeAnalyzer {
    fn name(&self) -> &str {
        "font-size"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut f = Vec::new();
        let url = &ctx.page.url;
        self.check_small_font_sizes(ctx, url, &mut f);
        self.check_line_height(ctx, url, &mut f);
        f
    }
}
