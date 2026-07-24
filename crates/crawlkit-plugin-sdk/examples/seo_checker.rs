//! Example: SEO Checker Plugin
//!
//! This plugin checks for common SEO issues.
//!
//! Build with:
//! ```bash
//! cargo build --target wasm32-wasi --example seo_checker
//! ```

use crawlkit_plugin_sdk::{AnalysisContext, Analyzer, Finding, Severity};

pub struct SeoChecker;

impl SeoChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SeoChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SeoChecker {
    fn name(&self) -> &str {
        "seo-checker"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check for title tag
        if !ctx.html.contains("<title") {
            findings.push(Finding {
                severity: Severity::Error,
                category: "seo".into(),
                code: "SEO001".into(),
                title: "Missing title tag".into(),
                description: "The page does not have a title tag".into(),
                url: ctx.url.clone(),
                recommendation: "Add a <title> tag to the page head".into(),
            });
        }

        // Check for meta description
        if !ctx.html.contains("meta name=\"description\"") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: "seo".into(),
                code: "SEO002".into(),
                title: "Missing meta description".into(),
                description: "The page does not have a meta description".into(),
                url: ctx.url.clone(),
                recommendation: "Add a meta description tag".into(),
            });
        }

        // Check for h1 tag
        if !ctx.html.contains("<h1") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: "seo".into(),
                code: "SEO003".into(),
                title: "Missing H1 tag".into(),
                description: "The page does not have an H1 heading".into(),
                url: ctx.url.clone(),
                recommendation: "Add an H1 heading to the page".into(),
            });
        }

        // Check for alt attributes on images
        let img_count = ctx.html.matches("<img").count();
        let alt_count = ctx.html.matches("alt=\"").count();
        if img_count > 0 && alt_count < img_count {
            findings.push(Finding {
                severity: Severity::Warning,
                category: "seo".into(),
                code: "SEO004".into(),
                title: "Images missing alt text".into(),
                description: format!(
                    "Found {} images but only {} have alt text",
                    img_count, alt_count
                ),
                url: ctx.url.clone(),
                recommendation: "Add alt attributes to all images".into(),
            });
        }

        // Check for canonical URL
        if !ctx.html.contains("rel=\"canonical\"") {
            findings.push(Finding {
                severity: Severity::Info,
                category: "seo".into(),
                code: "SEO005".into(),
                title: "Missing canonical URL".into(),
                description: "The page does not have a canonical URL".into(),
                url: ctx.url.clone(),
                recommendation: "Add a canonical URL tag".into(),
            });
        }

        findings
    }
}

fn main() {
    println!("SEO Checker Plugin");
    println!("This plugin checks for common SEO issues.");
    println!("Build with: cargo build --target wasm32-wasi --example seo_checker");
}

// Export for WASM
crawlkit_plugin_sdk::export_analyzer!(SeoChecker);
