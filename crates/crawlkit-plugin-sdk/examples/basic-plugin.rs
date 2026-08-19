//! Example crawlkit plugin built with `export_analyzer!`.
//!
//! Compiled to `wasm32-unknown-unknown`, this module is loaded by the
//! engine's wasmtime host in the `wasm_abi_tests` end-to-end conformance
//! test — proving the SDK-generated ABI and the host loader agree.

use crawlkit_plugin_sdk::{AnalysisContext, Analyzer, Finding, Severity};

pub struct TitleLengthAnalyzer;

impl TitleLengthAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TitleLengthAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TitleLengthAnalyzer {
    fn name(&self) -> &str {
        "title-length"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let Some(start) = ctx.html.find("<title>") else {
            return vec![Finding {
                severity: Severity::Error,
                category: "seo".into(),
                code: "TITLE001".into(),
                title: "Missing <title>".into(),
                description: "Page has no title element".into(),
                url: ctx.url.clone(),
                recommendation: "Add a <title> element".into(),
            }];
        };
        let Some(end) = ctx.html[start + 7..].find("</title>") else {
            return Vec::new();
        };
        let title = &ctx.html[start + 7..start + 7 + end];
        if title.len() > 60 {
            vec![Finding {
                severity: Severity::Warning,
                category: "seo".into(),
                code: "TITLE002".into(),
                title: "Title too long".into(),
                description: format!("Title is {} bytes; keep under 60", title.len()),
                url: ctx.url.clone(),
                recommendation: "Shorten the title".into(),
            }]
        } else {
            Vec::new()
        }
    }
}

crawlkit_plugin_sdk::export_analyzer!(TitleLengthAnalyzer);

fn main() {
    // The value of this example is the set of exported ABI symbols; when
    // compiled for wasm32-unknown-unknown there is no meaningful entry
    // point to run.
}
