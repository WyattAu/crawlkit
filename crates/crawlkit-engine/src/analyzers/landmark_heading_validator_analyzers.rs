//! Landmark and heading hierarchy accessibility validators.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. Public names and behavior are preserved through re-exports.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// LandmarkMainAnalyzer — LANDMAIN001
// =========================================================================

pub struct LandmarkMainAnalyzer;
impl Default for LandmarkMainAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkMainAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LandmarkMainAnalyzer {
    fn name(&self) -> &str {
        "landmark-main"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        if !ctx.page.has_main_landmark {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LANDMAIN001".to_string(),
                title: "Page missing main landmark".to_string(),
                description: "No <main> element or role=\"main\" found. Screen readers use landmarks for quick navigation.".to_string(),
                url: ctx.page.url.to_string(),
                recommendation: "Wrap primary content in a <main> element.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// LandmarkNavAnalyzer — LANDNAV001
// =========================================================================

pub struct LandmarkNavAnalyzer;
impl Default for LandmarkNavAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkNavAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LandmarkNavAnalyzer {
    fn name(&self) -> &str {
        "landmark-nav"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        if !ctx.page.has_nav_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LANDNAV001".to_string(),
                title: "Page missing navigation landmark".to_string(),
                description: "No <nav> element or role=\"navigation\" found. Navigation landmarks help screen reader users.".to_string(),
                url: ctx.page.url.to_string(),
                recommendation: "Wrap navigation links in a <nav> element.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// LandmarkBannerAnalyzer — LANDBAN001
// =========================================================================

pub struct LandmarkBannerAnalyzer;
impl Default for LandmarkBannerAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkBannerAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LandmarkBannerAnalyzer {
    fn name(&self) -> &str {
        "landmark-banner"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let has_banner = ctx
            .page
            .landmarks
            .iter()
            .any(|l| l == "banner" || l == "header");
        if !has_banner {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LANDBAN001".to_string(),
                title: "Page missing banner/header landmark".to_string(),
                description: "No <header> element or role=\"banner\" found. The banner landmark contains site-wide content like logo and navigation.".to_string(),
                url: ctx.page.url.to_string(),
                recommendation: "Wrap the site header in a <header> element.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// HeadingLevelSkipAnalyzer — HEADSKIP001
// =========================================================================

pub struct HeadingLevelSkipAnalyzer;
impl Default for HeadingLevelSkipAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingLevelSkipAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HeadingLevelSkipAnalyzer {
    fn name(&self) -> &str {
        "heading-level-skip"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.len() < 2 {
            return findings;
        }
        let mut prev_level: Option<u8> = None;
        for heading in &ctx.page.headings {
            if let Some(prev) = prev_level {
                if heading.level > prev + 1 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "HEADSKIP001".to_string(),
                        title: "Heading level skip detected".to_string(),
                        description: format!(
                            "Heading jumps from H{prev} to H{}, skipping intermediate levels.",
                            heading.level
                        ),
                        url: url.to_string(),
                        recommendation: format!(
                            "Use H{} after H{prev} to maintain document outline.",
                            prev + 1
                        ),
                    });
                    break;
                }
            }
            prev_level = Some(heading.level);
        }
        findings
    }
}
