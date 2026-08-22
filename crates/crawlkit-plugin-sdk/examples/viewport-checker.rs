//! Example crawlkit plugin: flags missing or suspicious viewport meta tags.
//!
//! Demonstrates a slightly richer analyzer than `basic-plugin` — string
//! inspection with severity branching — and is published in the
//! first-party plugin index (`plugins/index/`).

use crawlkit_plugin_sdk::{AnalysisContext, Analyzer, Finding, Severity};

pub struct ViewportChecker;

impl ViewportChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ViewportChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ViewportChecker {
    fn name(&self) -> &str {
        "viewport-checker"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let needle = "<meta name=\"viewport\"";
        let html_lower_prefix: String = ctx
            .html
            .chars()
            .take(4096)
            .flat_map(|c| c.to_lowercase())
            .collect();

        if !ctx.html.contains(needle) && !html_lower_prefix.contains(needle) {
            return vec![Finding {
                severity: Severity::Error,
                category: "mobile".into(),
                code: "VP001".into(),
                title: "Missing viewport meta tag".into(),
                description: "No <meta name=\"viewport\"> found in the document head; \
                              mobile browsers will render a desktop-width layout."
                    .into(),
                url: ctx.url.clone(),
                recommendation: "Add <meta name=\"viewport\" \
                                  content=\"width=device-width, initial-scale=1\">."
                    .into(),
            }];
        }

        // Present: check for the legacy fixed-width anti-pattern.
        if ctx.html.contains("width=device-width") || !ctx.html.contains("width=") {
            return Vec::new();
        }
        vec![Finding {
            severity: Severity::Warning,
            category: "mobile".into(),
            code: "VP002".into(),
            title: "Fixed-width viewport".into(),
            description: "Viewport declares a fixed width instead of \
                          device-width, forcing horizontal scaling on phones."
                .into(),
            url: ctx.url.clone(),
            recommendation: "Use content=\"width=device-width, initial-scale=1\".".into(),
        }]
    }
}

crawlkit_plugin_sdk::export_analyzer!(ViewportChecker);

fn main() {
    // WASM library target; the exported ABI symbols are the entry points.
}
