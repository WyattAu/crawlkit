//! Custom analyzer example demonstrating how to implement a custom analyzer.

use crawlkit_engine::analyzers::{AnalysisContext, Analyzer, Finding};
use crawlkit_engine::storage::{IssueCategory, Severity};
use crawlkit_engine::CrawlConfig;

/// A custom analyzer that checks for excessive use of external resources.
///
/// This analyzer demonstrates how to implement the `Analyzer` trait
/// to create custom SEO checks.
struct ExternalResourceAnalyzer;

impl Analyzer for ExternalResourceAnalyzer {
    fn name(&self) -> &str {
        "external-resource"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Count external links
        let external_links = ctx
            .page
            .links
            .iter()
            .filter(|link| {
                url.host_str() != link.url.host_str()
            })
            .count();

        // Warn if too many external links
        if external_links > 10 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CUSTOM001".to_string(),
                title: "Excessive external links".to_string(),
                description: format!(
                    "Page has {} external links. Too many outbound links may dilute "
                    "link equity and affect SEO.",
                    external_links
                ),
                url: url.to_string(),
                recommendation: "Reduce the number of external links or add "
                    "rel=\"nofollow\" to non-essential links."
                    .to_string(),
            });
        }

        findings
    }
}

fn main() {
    println!("Custom analyzer example:");
    println!("  This example demonstrates how to implement a custom analyzer.");
    println!("  The ExternalResourceAnalyzer checks for excessive external links.");
    println!();
    println!("  To use this analyzer, add it to the AnalyzerRegistry:");
    println!("    let mut registry = AnalyzerRegistry::new(&config);");
    println!("    registry.register(Box::new(ExternalResourceAnalyzer));");
}
