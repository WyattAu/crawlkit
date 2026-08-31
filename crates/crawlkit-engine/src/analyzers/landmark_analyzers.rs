//! Landmark accessibility analyzer.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. The public
//! analyzer name and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Checks for main, navigation, and banner landmark regions.
pub struct LandmarkRegionsAnalyzer;

impl LandmarkRegionsAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LandmarkRegionsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LandmarkRegionsAnalyzer {
    fn name(&self) -> &str {
        "landmark-regions"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if !ctx.page.has_main_landmark {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LAND001".to_string(),
                title: "Missing main landmark region".to_string(),
                description: "No <main> element or role=\"main\" found. Screen reader users rely on landmark regions to quickly navigate to the primary content of a page."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Wrap the primary page content in a <main> element or add role=\"main\" to the primary content container."
                    .to_string(),
            });
        }

        if !ctx.page.has_nav_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LAND002".to_string(),
                title: "Missing navigation landmark".to_string(),
                description: "No <nav> element or role=\"navigation\" found. Navigation landmarks allow screen reader users to jump directly to site navigation."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Wrap primary navigation links in a <nav> element or add role=\"navigation\" to the navigation container."
                    .to_string(),
            });
        }

        let has_banner = ctx
            .page
            .landmarks
            .iter()
            .any(|l| l == "banner" || l == "header");
        if !has_banner {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LAND003".to_string(),
                title: "Missing banner/header landmark".to_string(),
                description: "No <header> element or role=\"banner\" found. The banner landmark typically contains the site logo, search, and primary navigation."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Wrap the site header in a <header> element. For the banner role, ensure it is a direct child of <body>."
                    .to_string(),
            });
        }

        findings
    }
}
