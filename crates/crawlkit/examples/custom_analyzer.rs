//! Example: Create a custom analyzer.
//!
//! Run with: cargo run --example custom_analyzer

use crawlkit_core::analyzers::{AnalysisContext, Analyzer, Finding};
use crawlkit_core::storage::{IssueCategory, Severity};
use crawlkit_core::CrawlConfig;

/// Custom analyzer that checks for specific content patterns.
pub struct ContentPatternAnalyzer {
    keywords: Vec<String>,
}

impl ContentPatternAnalyzer {
    pub fn new(keywords: Vec<String>) -> Self {
        Self { keywords }
    }
}

impl Analyzer for ContentPatternAnalyzer {
    fn name(&self) -> &str {
        "content-pattern"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let content = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        for keyword in &self.keywords {
            if !content.to_lowercase().contains(&keyword.to_lowercase()) {
                findings.push(Finding {
                    category: IssueCategory::Content,
                    severity: Severity::Warning,
                    code: "KEYWORD001".to_string(),
                    title: format!("Missing keyword: {}", keyword),
                    description: format!(
                        "The page content does not contain the keyword '{}'",
                        keyword
                    ),
                    url: ctx.page.url.clone(),
                    recommendation: format!("Add '{}' to the page content", keyword),
                });
            }
        }

        findings
    }
}

fn main() {
    let analyzer = ContentPatternAnalyzer::new(vec![
        "SEO".to_string(),
        "crawler".to_string(),
        "analysis".to_string(),
    ]);

    println!("Custom analyzer: {}", analyzer.name());
    println!("Keywords: {:?}", vec!["SEO", "crawler", "analysis"]);
    println!("Implement AnalysisContext and call analyzer.analyze() to test.");
}
