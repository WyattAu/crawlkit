//! # crawlkit-plugin-sdk
//!
//! SDK for building crawlkit WASM plugins.
//!
//! ## Overview
//!
//! This crate provides the types and traits needed to create custom SEO analyzers
//! that run as WASM plugins in crawlkit's sandboxed environment.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use crawlkit_plugin_sdk::{Analyzer, Finding, Severity, AnalysisContext};
//!
//! pub struct MyAnalyzer;
//! impl MyAnalyzer { pub fn new() -> Self { Self } }
//!
//! impl Analyzer for MyAnalyzer {
//!     fn name(&self) -> &str { "my-analyzer" }
//!
//!     fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
//!         let mut findings = Vec::new();
//!         if ctx.html.contains("something") {
//!             findings.push(Finding {
//!                 severity: Severity::Warning,
//!                 category: "custom".into(),
//!                 code: "CUSTOM001".into(),
//!                 title: "Something detected".into(),
//!                 description: "The page contains something".into(),
//!                 url: ctx.url.clone(),
//!                 recommendation: "Remove something".into(),
//!             });
//!         }
//!         findings
//!     }
//! }
//!
//! // Export for WASM
//! crawlkit_plugin_sdk::export_analyzer!(MyAnalyzer);
//! ```
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod analyzer;
mod context;
mod export;
mod finding;

pub use analyzer::Analyzer;
pub use context::AnalysisContext;
pub use finding::{Finding, Severity};

/// Host-ABI allocator internals used by the `export_analyzer!` macro.
///
/// Not intended for direct use by plugin authors.
pub mod exported {
    pub use crate::export::{alloc_raw, free_raw};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn re_exports_are_accessible() {
        let _f = Finding {
            severity: Severity::Info,
            category: "c".into(),
            code: "C".into(),
            title: "t".into(),
            description: "d".into(),
            url: "u".into(),
            recommendation: "r".into(),
        };
        let _ctx = AnalysisContext {
            url: "u".into(),
            html: "h".into(),
            status_code: None,
            headers: Vec::new(),
            response_time_ms: None,
        };
    }

    #[test]
    fn full_analyzer_workflow() {
        struct SeoAnalyzer;

        impl Analyzer for SeoAnalyzer {
            fn name(&self) -> &str {
                "seo-analyzer"
            }

            fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
                let mut findings = Vec::new();
                if !ctx.html.contains("<title>") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: "seo".into(),
                        code: "SEO001".into(),
                        title: "Missing title tag".into(),
                        description: "The page does not have a <title> tag".into(),
                        url: ctx.url.clone(),
                        recommendation: "Add a <title> tag in the <head> section".into(),
                    });
                }
                if ctx.html.contains("meta name=\"description\"") {
                } else {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: "seo".into(),
                        code: "SEO002".into(),
                        title: "Missing meta description".into(),
                        description: "No meta description found".into(),
                        url: ctx.url.clone(),
                        recommendation: "Add a meta description tag".into(),
                    });
                }
                findings
            }
        }

        let analyzer = SeoAnalyzer;
        assert_eq!(analyzer.name(), "seo-analyzer");

        let ctx = AnalysisContext {
            url: "https://example.com".into(),
            html: "<html><body>No head here</body></html>".into(),
            status_code: Some(200),
            headers: Vec::new(),
            response_time_ms: None,
        };
        let findings = analyzer.analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].code, "SEO001");
        assert_eq!(findings[1].code, "SEO002");
    }

    #[test]
    fn finding_severity_roundtrip_json() {
        let f = Finding {
            severity: Severity::Critical,
            category: "sec".into(),
            code: "S1".into(),
            title: "Critical issue".into(),
            description: "Bad".into(),
            url: "https://x.com".into(),
            recommendation: "Fix".into(),
        };
        let json = serde_json::to_string(&f).unwrap();
        let restored: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.severity, Severity::Critical);
    }
}
