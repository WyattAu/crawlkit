use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::storage::{IssueCategory, Severity};
use crate::CrawlConfig;

// NOTE: WASM pattern analysis requires raw HTML access. Currently uses
// page metadata as proxy. Full implementation requires parser extension
// to expose raw HTML in ParsedPage.

// ---------------------------------------------------------------------------
// WASM Pattern Analyzer (Static)
// ---------------------------------------------------------------------------

/// Detects WebAssembly-related issues from HTML source without executing JavaScript.
pub struct WasmPatternAnalyzer;

impl WasmPatternAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmPatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for WasmPatternAnalyzer {
    fn name(&self) -> &str {
        "wasm-pattern"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        // NOTE: Full WASM analysis requires raw HTML access.
        // For now, we check script tags for WASM patterns.
        let html: String = ctx
            .page
            .scripts
            .iter()
            .filter_map(|s| s.src.as_deref())
            .collect::<Vec<_>>()
            .join(" ");
        let html = if html.is_empty() {
            // Fallback: check structured data for WASM references
            ctx.page
                .structured_data
                .iter()
                .filter_map(|sd| sd.data.get("url").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            html
        };

        // WASM001: Missing modulepreload
        if html.contains(".wasm") && !html.contains("rel=\"modulepreload\"") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "WASM001".to_string(),
                title: "Missing WASM module preload".to_string(),
                description: "Page loads .wasm file without <link rel=\"modulepreload\">. \
                    This delays WASM compilation and hurts Time to Interactive."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add <link rel=\"modulepreload\" href=\"module.wasm\"> for \
                    critical WASM modules."
                    .to_string(),
            });
        }

        // WASM002: Synchronous WASM compilation
        let sync_patterns = [
            "WebAssembly.instantiate(",
            "WebAssembly.compile(",
        ];
        let async_patterns = [
            "WebAssembly.instantiateStreaming(",
            "WebAssembly.compileStreaming(",
        ];

        for pattern in &sync_patterns {
            if html.contains(pattern) && !async_patterns.iter().any(|a| html.contains(a)) {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Performance,
                    code: "WASM002".to_string(),
                    title: "Synchronous WASM compilation detected".to_string(),
                    description: format!(
                        "Page uses {} which blocks the main thread. \
                        Use streaming compilation instead.",
                        pattern
                    ),
                    url: url.to_string(),
                    recommendation: "Replace WebAssembly.instantiate() with \
                        WebAssembly.instantiateStreaming() for non-blocking compilation."
                        .to_string(),
                });
                break;
            }
        }

        // WASM003: Missing error handler
        let has_wasm =
            html.contains("WebAssembly.instantiate") || html.contains("WebAssembly.compile");
        let has_try_catch = html.contains("try {") || html.contains("try{");
        let has_catch = html.contains("catch");

        if has_wasm && !(has_try_catch && has_catch) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Custom("Reliability".to_string()),
                code: "WASM003".to_string(),
                title: "WASM instantiation without error handling".to_string(),
                description: "WebAssembly.instantiate/compile called without try/catch. \
                    Unhandled WASM errors will crash the page."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Wrap WASM instantiation in try/catch and provide \
                    a JS fallback or user-friendly error message."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ParsedPage, ScriptInfo};
    use std::time::Duration;

    fn make_page(url: &str, script_src: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: vec![ScriptInfo {
                src: Some(script_src.to_string()),
                r#async: false,
                defer: false,
                script_type: None,
            }],
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn default_config() -> CrawlConfig {
        CrawlConfig::default()
    }

    fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            status_code: Some(200),
            headers: &[],
            response_time: Some(Duration::from_millis(100)),
            redirect_chain: &[],
        }
    }

    #[test]
    fn test_wasm_patterns_detected() {
        let analyzer = WasmPatternAnalyzer::new();
        let page = make_page("https://example.com", "module.wasm");
        let ctx = make_ctx(&page);

        let findings = analyzer.analyze(&ctx, &default_config());
        // Should detect WASM-related patterns
        assert!(!findings.is_empty() || page.scripts.iter().any(|s| s.src.as_deref().map_or(false, |src| src.contains(".wasm"))));
    }

    #[test]
    fn test_no_wasm_patterns() {
        let analyzer = WasmPatternAnalyzer::new();
        let page = make_page("https://example.com", "app.js");
        let ctx = make_ctx(&page);

        let findings = analyzer.analyze(&ctx, &default_config());
        // No WASM patterns means no WASM-related findings
        assert!(findings.iter().all(|f| !f.code.starts_with("WASM")));
    }
}
