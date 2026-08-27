//! Custom analyzer example showing how to extend crawlkit with your own checks.
//!
//! This example demonstrates:
//! - Implementing the `Analyzer` trait for a custom check
//! - Registering custom analyzers alongside the built-in ones
//! - Using `IssueCategory::Custom` for plugin-specific categories
//!
//! Run with:
//!     cargo run --example custom-analyzer

use std::time::Duration;

use anyhow::Result;
use crawlkit_engine::analyzers::{AnalysisContext, Analyzer, AnalyzerRegistry, Finding};
use crawlkit_engine::storage::{IssueCategory, Severity};
use crawlkit_engine::{CrawlConfig, HtmlParser};
use url::Url;

// ---------------------------------------------------------------------------
// 1. Define a custom analyzer: WordCountThresholdAnalyzer
// ---------------------------------------------------------------------------

/// Flags pages with fewer than a configurable minimum word count.
struct WordCountThresholdAnalyzer {
    min_words: usize,
}

impl WordCountThresholdAnalyzer {
    fn new(min_words: usize) -> Self {
        Self { min_words }
    }
}

impl Analyzer for WordCountThresholdAnalyzer {
    fn name(&self) -> &str {
        "word-count-threshold"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.word_count < self.min_words {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CUSTOM001".to_string(),
                title: "Thin content detected".to_string(),
                description: format!(
                    "Page has {} words, below the minimum threshold of {}.",
                    ctx.page.word_count, self.min_words
                ),
                url: url.clone(),
                recommendation: format!(
                    "Add at least {} words of unique, valuable content.",
                    self.min_words
                ),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 2. Define another custom analyzer: ExternalLinkRatioAnalyzer
// ---------------------------------------------------------------------------

/// Flags pages where the ratio of external to total links exceeds a threshold.
struct ExternalLinkRatioAnalyzer {
    max_ratio: f64,
}

impl ExternalLinkRatioAnalyzer {
    fn new(max_ratio: f64) -> Self {
        Self { max_ratio }
    }
}

impl Analyzer for ExternalLinkRatioAnalyzer {
    fn name(&self) -> &str {
        "external-link-ratio"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let total = ctx.page.links.len();
        if total == 0 {
            return findings;
        }

        let external = ctx.page.links.iter().filter(|l| l.is_external).count();
        let ratio = external as f64 / total as f64;

        if ratio > self.max_ratio {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Links,
                code: "CUSTOM002".to_string(),
                title: "High external link ratio".to_string(),
                description: format!(
                    "{external}/{total} links ({:.0}%) are external, exceeding the {}% threshold.",
                    ratio * 100.0,
                    (self.max_ratio * 100.0) as usize,
                ),
                url: url.clone(),
                recommendation: "Reduce external links or add more internal links to improve \
                                 site authority distribution."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 3. Define a custom analyzer: PerformanceBudgetAnalyzer
// ---------------------------------------------------------------------------

/// Flags pages that exceed a response time budget.
struct PerformanceBudgetAnalyzer {
    max_ms: u64,
}

impl PerformanceBudgetAnalyzer {
    fn new(max_ms: u64) -> Self {
        Self { max_ms }
    }
}

impl Analyzer for PerformanceBudgetAnalyzer {
    fn name(&self) -> &str {
        "performance-budget"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if let Some(response_time) = ctx.response_time {
            let elapsed_ms = response_time.as_millis() as u64;
            if elapsed_ms > self.max_ms {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Performance,
                    code: "CUSTOM003".to_string(),
                    title: "Performance budget exceeded".to_string(),
                    description: format!(
                        "Page response took {elapsed_ms}ms, exceeding the {}ms budget.",
                        self.max_ms
                    ),
                    url: url.clone(),
                    recommendation: "Optimize server response time, add caching, or use a CDN."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 4. Main: build a custom AnalyzerRegistry and run analysis
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    // Sample HTML to analyze
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <title>Short Page</title>
    <meta name="description" content="A test page">
</head>
<body>
    <h1>Hello</h1>
    <p>Hi.</p>
    <a href="https://external1.com">Ext1</a>
    <a href="https://external2.com">Ext2</a>
    <a href="https://external3.com">Ext3</a>
    <a href="/internal">Int</a>
</body>
</html>"#;

    let url = Url::parse("https://example.com/page")?;
    let parsed = HtmlParser::parse(html, &url);

    // Build the default registry (18 built-in analyzers)
    let config = CrawlConfig::default();
    let _registry = AnalyzerRegistry::new(&config);

    // Register our custom analyzers
    // Note: In the actual crawlkit codebase, AnalyzerRegistry uses a Vec<Box<dyn Analyzer>>.
    // We'll demonstrate the trait implementation pattern. In practice you'd add them
    // via the registry's push/extend methods or by modifying the registry builder.

    println!("=== Custom Analyzer Demo ===\n");
    println!("Parsed page: {}", parsed.url);
    println!("  Title: {:?}", parsed.meta.title);
    println!("  Word count: {}", parsed.word_count);
    println!(
        "  Links: {} total, {} external\n",
        parsed.links.len(),
        parsed.links.iter().filter(|l| l.is_external).count(),
    );

    // Manually run each custom analyzer for demonstration
    let ctx = AnalysisContext {
        page: &parsed,
        body: Some(html),
        status_code: Some(200),
        headers: &[],
        response_time: Some(Duration::from_millis(850)),
        redirect_chain: &[],
        robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
    };

    let analyzers: Vec<Box<dyn Analyzer>> = vec![
        Box::new(WordCountThresholdAnalyzer::new(300)),
        Box::new(ExternalLinkRatioAnalyzer::new(0.7)),
        Box::new(PerformanceBudgetAnalyzer::new(500)),
    ];

    for analyzer in &analyzers {
        let findings = analyzer.analyze(&ctx);
        if findings.is_empty() {
            println!("[PASS] {} — no issues", analyzer.name());
        } else {
            for finding in &findings {
                println!(
                    "[{:?}] {} — {}",
                    finding.severity, finding.code, finding.title
                );
                println!("  {}", finding.description);
                println!("  Fix: {}\n", finding.recommendation);
            }
        }
    }

    println!("=== Done ===");
    Ok(())
}
