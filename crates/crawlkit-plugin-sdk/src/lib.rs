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

mod analyzer;
mod context;
mod export;
mod finding;

pub use analyzer::Analyzer;
pub use context::AnalysisContext;
pub use finding::{Finding, Severity};
