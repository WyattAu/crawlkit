//! CSP directive validator and color contrast accessibility analyzers.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. Public names and behavior are preserved through re-exports.

#![allow(clippy::unwrap_used)]

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// CspDirectiveValidator — CSPDIR001
// =========================================================================

pub struct CspDirectiveValidator;
impl Default for CspDirectiveValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspDirectiveValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CspDirectiveValidator {
    fn name(&self) -> &str {
        "csp-directive-validator"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str());
        if let Some(value) = csp {
            let lower = value.to_lowercase();
            let recommended = [
                "default-src",
                "script-src",
                "style-src",
                "img-src",
                "connect-src",
            ];
            for dir in &recommended {
                if !lower.contains(dir) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPDIR001".to_string(),
                        title: format!("CSP missing {dir} directive"),
                        description: format!("The Content-Security-Policy header does not include a '{dir}' directive. Without it, resources of this type are governed by default-src or are unrestricted."),
                        url: url.to_string(),
                        recommendation: format!("Add a '{dir}' directive to the Content-Security-Policy header."),
                    });
                }
            }
        }
        findings
    }
}

// =========================================================================
// ColorContrastTextAnalyzer — COLRCT001
// =========================================================================

pub struct ColorContrastTextAnalyzer;
impl Default for ColorContrastTextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl ColorContrastTextAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl ColorContrastTextAnalyzer {
    fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let h = hex.trim().trim_start_matches('#');
        match h.len() {
            3 => {
                let r = u8::from_str_radix(&h[0..1], 16).ok()?;
                let g = u8::from_str_radix(&h[1..2], 16).ok()?;
                let b = u8::from_str_radix(&h[2..3], 16).ok()?;
                Some((r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&h[0..2], 16).ok()?;
                let g = u8::from_str_radix(&h[2..4], 16).ok()?;
                let b = u8::from_str_radix(&h[4..6], 16).ok()?;
                Some((r, g, b))
            }
            _ => None,
        }
    }

    fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
        let f = |c: u8| -> f64 {
            let s = c as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
    }

    fn contrast_ratio(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
        let l1 = Self::relative_luminance(fg.0, fg.1, fg.2);
        let l2 = Self::relative_luminance(bg.0, bg.1, bg.2);
        let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn extract_text_color_pairs(body: &str) -> Vec<((u8, u8, u8), (u8, u8, u8))> {
        use regex::Regex;
        let re = Regex::new(r#"style\s*=\s*["'][^"']*color\s*:\s*(#[0-9a-fA-F]{3,6})[^"']*background(?:-color)?\s*:\s*(#[0-9a-fA-F]{3,6})["']"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        let mut pairs = Vec::new();
        for cap in re.captures_iter(body) {
            if let (Some(fg), Some(bg)) = (Self::parse_hex(&cap[1]), Self::parse_hex(&cap[2])) {
                pairs.push((fg, bg));
            }
        }
        let re2 = Regex::new(r#"style\s*=\s*["'][^"']*background(?:-color)?\s*:\s*(#[0-9a-fA-F]{3,6})[^"']*color\s*:\s*(#[0-9a-fA-F]{3,6})["']"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        for cap in re2.captures_iter(body) {
            if let (Some(bg), Some(fg)) = (Self::parse_hex(&cap[1]), Self::parse_hex(&cap[2])) {
                pairs.push((fg, bg));
            }
        }
        pairs
    }
}

impl Analyzer for ColorContrastTextAnalyzer {
    fn name(&self) -> &str {
        "color-contrast-text"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let body = ctx.body.unwrap_or("");
        let pairs = Self::extract_text_color_pairs(body);
        let mut low = 0;
        for (fg, bg) in &pairs {
            let ratio = Self::contrast_ratio(*fg, *bg);
            if ratio < 4.5 {
                low += 1;
            }
        }
        if low > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "COLRCT001".to_string(),
                title: "Low text color contrast ratio".to_string(),
                description: format!("{low} inline style(s) have a contrast ratio below 4.5:1. WCAG 1.4.3 requires at least 4.5:1 for normal text."),
                url: url.to_string(),
                recommendation: "Ensure text color contrasts at least 4.5:1 with its background.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// ColorContrastLinkAnalyzer — COLRCL001
// =========================================================================

pub struct ColorContrastLinkAnalyzer;
impl Default for ColorContrastLinkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl ColorContrastLinkAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl ColorContrastLinkAnalyzer {
    fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let h = hex.trim().trim_start_matches('#');
        match h.len() {
            3 => {
                let r = u8::from_str_radix(&h[0..1], 16).ok()?;
                let g = u8::from_str_radix(&h[1..2], 16).ok()?;
                let b = u8::from_str_radix(&h[2..3], 16).ok()?;
                Some((r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&h[0..2], 16).ok()?;
                let g = u8::from_str_radix(&h[2..4], 16).ok()?;
                let b = u8::from_str_radix(&h[4..6], 16).ok()?;
                Some((r, g, b))
            }
            _ => None,
        }
    }

    fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
        let f = |c: u8| -> f64 {
            let s = c as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
    }

    fn contrast_ratio(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
        let l1 = Self::relative_luminance(fg.0, fg.1, fg.2);
        let l2 = Self::relative_luminance(bg.0, bg.1, bg.2);
        let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn extract_link_color_pairs(body: &str) -> Vec<((u8, u8, u8), (u8, u8, u8))> {
        use regex::Regex;
        let re = Regex::new(r#"style\s*=\s*["'][^"']*color\s*:\s*(#[0-9a-fA-F]{3,6})[^"']*background(?:-color)?\s*:\s*(#[0-9a-fA-F]{3,6})["']"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        let mut pairs = Vec::new();
        for cap in re.captures_iter(body) {
            if let (Some(fg), Some(bg)) = (Self::parse_hex(&cap[1]), Self::parse_hex(&cap[2])) {
                pairs.push((fg, bg));
            }
        }
        pairs
    }
}

impl Analyzer for ColorContrastLinkAnalyzer {
    fn name(&self) -> &str {
        "color-contrast-link"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let body = ctx.body.unwrap_or("");
        let pairs = Self::extract_link_color_pairs(body);
        let mut low = 0;
        for (fg, bg) in &pairs {
            let ratio = Self::contrast_ratio(*fg, *bg);
            if ratio < 3.0 {
                low += 1;
            }
        }
        if low > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "COLRCL001".to_string(),
                title: "Link color contrast too low".to_string(),
                description: format!("{low} color pair(s) have a contrast ratio below 3:1, making links difficult to distinguish from surrounding text."),
                url: url.to_string(),
                recommendation: "Ensure link colors contrast at least 3:1 with the background and 3:1 with surrounding text.".to_string(),
            });
        }
        findings
    }
}
