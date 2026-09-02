//! Visual accessibility analyzers for color contrast and keyboard focus.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. Public
//! analyzer names and behavior are preserved through re-exports in `mod.rs`.

#![allow(clippy::unwrap_used)]

use regex::Regex;

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------
// Color Contrast Analyzer (WCAG 1.4.3)
// ---------------------------------------------------------------------------

pub struct ColorContrastAnalyzer;

impl ColorContrastAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.trim().trim_start_matches('#');
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some((r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some((r, g, b))
            }
            _ => None,
        }
    }

    fn parse_named_color(name: &str) -> Option<(u8, u8, u8)> {
        match name.trim().to_lowercase().as_str() {
            "black" => Some((0, 0, 0)),
            "white" => Some((255, 255, 255)),
            "red" => Some((255, 0, 0)),
            "green" => Some((0, 128, 0)),
            "blue" => Some((0, 0, 255)),
            "yellow" => Some((255, 255, 0)),
            "gray" | "grey" => Some((128, 128, 128)),
            "silver" => Some((192, 192, 192)),
            "navy" => Some((0, 0, 128)),
            "maroon" => Some((128, 0, 0)),
            "olive" => Some((128, 128, 0)),
            "teal" => Some((0, 128, 128)),
            "aqua" | "cyan" => Some((0, 255, 255)),
            "fuchsia" | "magenta" => Some((255, 0, 255)),
            "lime" => Some((0, 255, 0)),
            "orange" => Some((255, 165, 0)),
            "pink" => Some((255, 192, 203)),
            "purple" => Some((128, 0, 128)),
            _ => None,
        }
    }

    fn parse_rgb_function(val: &str) -> Option<(u8, u8, u8)> {
        let re = Regex::new(r"rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)").ok()?;
        let caps = re.captures(val)?;
        let r: u8 = caps[1].parse().ok()?;
        let g: u8 = caps[2].parse().ok()?;
        let b: u8 = caps[3].parse().ok()?;
        Some((r, g, b))
    }

    fn parse_color_value(val: &str) -> Option<(u8, u8, u8)> {
        let trimmed = val.trim();
        if trimmed.starts_with('#') {
            return Self::parse_hex_color(trimmed);
        }
        if trimmed.starts_with("rgb(") {
            return Self::parse_rgb_function(trimmed);
        }
        Self::parse_named_color(trimmed)
    }

    fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
        let fn_channel = |c: u8| -> f64 {
            let s = c as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * fn_channel(r) + 0.7152 * fn_channel(g) + 0.0722 * fn_channel(b)
    }

    pub(crate) fn contrast_ratio(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
        let l1 = Self::relative_luminance(fg.0, fg.1, fg.2);
        let l2 = Self::relative_luminance(bg.0, bg.1, bg.2);
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
    }

    fn extract_color_pairs(html: &str) -> Vec<((u8, u8, u8), (u8, u8, u8))> {
        let mut pairs = Vec::new();
        let Ok(re) = Regex::new(
            r#"style\s*=\s*["'][^"']*color\s*:\s*([^;"']+)[^"']*background(?:-color)?\s*:\s*([^;"']+)["']"#,
        ) else {
            return pairs;
        };
        for cap in re.captures_iter(html) {
            if let (Some(fg), Some(bg)) = (
                Self::parse_color_value(&cap[1]),
                Self::parse_color_value(&cap[2]),
            ) {
                pairs.push((fg, bg));
            }
        }

        let Ok(re2) = Regex::new(
            r#"style\s*=\s*["'][^"']*background(?:-color)?\s*:\s*([^;"']+)[^"']*color\s*:\s*([^;"']+)["']"#,
        ) else {
            return pairs;
        };
        for cap in re2.captures_iter(html) {
            if let (Some(bg), Some(fg)) = (
                Self::parse_color_value(&cap[1]),
                Self::parse_color_value(&cap[2]),
            ) {
                pairs.push((fg, bg));
            }
        }
        pairs
    }
}

impl Default for ColorContrastAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ColorContrastAnalyzer {
    fn name(&self) -> &str {
        "color-contrast"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let body = ctx.body.unwrap_or("");

        let pairs = Self::extract_color_pairs(body);
        let mut low_contrast_count = 0;
        let mut similar_count = 0;

        for (fg, bg) in &pairs {
            let ratio = Self::contrast_ratio(*fg, *bg);
            if ratio < 3.0 {
                similar_count += 1;
            } else if ratio < 4.5 {
                low_contrast_count += 1;
            }
        }

        if similar_count > 0 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "CONTR001".to_string(),
                title: "Text color too similar to background color".to_string(),
                description: format!(
                    "{} inline style(s) have a contrast ratio below 3:1, making text \
                     extremely difficult to read. WCAG 1.4.3 requires a minimum contrast ratio \
                     of 4.5:1 for normal text.",
                    similar_count
                ),
                url: url.to_string(),
                recommendation: "Ensure text color contrasts sufficiently with its background. \
                                 Use a contrast checker tool to verify WCAG AA compliance."
                    .to_string(),
            });
        }

        if low_contrast_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "CONTR002".to_string(),
                title: "Low color contrast ratio (below 4.5:1)".to_string(),
                description: format!(
                    "{} inline style(s) have a contrast ratio between 3:1 and 4.5:1. WCAG 1.4.3 \
                     requires at least 4.5:1 for normal text and 3:1 for large text.",
                    low_contrast_count
                ),
                url: url.to_string(),
                recommendation: "Increase the contrast ratio to at least 4.5:1 for normal text \
                                 and 3:1 for large text (18px+ or 14px bold)."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Focus Order Analyzer
// ---------------------------------------------------------------------------

pub struct FocusOrderAnalyzer;

impl FocusOrderAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn check_positive_tabindex(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        if ctx.page.has_positive_tabindex {
            f.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "A11Y-FOCUS001".to_string(),
                title: "Positive tabindex values disrupt tab order".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, making \
                              keyboard navigation unpredictable. Users expect a sequential tab \
                              flow matching the visual layout."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Remove positive tabindex values. Use tabindex=\"0\" to add \
                                 elements to the natural tab order or tabindex=\"-1\" for \
                                 programmatic focus only."
                    .to_string(),
            });
        }
    }

    fn check_focus_styles(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let body = ctx.body.unwrap_or("");
        let has_focus_style = body.contains(":focus")
            || body.contains(":focus-visible")
            || body.contains(":focus-within");
        let interactive_count = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.text.trim().is_empty())
            .count()
            + ctx.page.forms.len();

        if interactive_count > 0 && !has_focus_style {
            f.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y-FOCUS002".to_string(),
                title: "No visible focus indicators found".to_string(),
                description: format!(
                    "Page has {} interactive element(s) but no :focus or :focus-visible CSS \
                     rules were detected. Keyboard users rely on visible focus indicators to \
                     know which element is active.",
                    interactive_count
                ),
                url: url.to_string(),
                recommendation: "Add :focus and/or :focus-visible CSS rules with a visible \
                                 outline or background change. Ensure the indicator has \
                                 sufficient contrast (3:1 minimum)."
                    .to_string(),
            });
        }
    }
}

impl Default for FocusOrderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FocusOrderAnalyzer {
    fn name(&self) -> &str {
        "focus-order"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut f = Vec::new();
        let url = &ctx.page.url;
        self.check_positive_tabindex(ctx, url, &mut f);
        self.check_focus_styles(ctx, url, &mut f);
        f
    }
}
