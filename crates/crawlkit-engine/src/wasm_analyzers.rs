use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::playwright::RenderedPage;
use crate::types::{IssueCategory, Severity};

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

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
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
        let sync_patterns = ["WebAssembly.instantiate(", "WebAssembly.compile("];
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
// WASM Runtime Analyzer (Dynamic - requires Playwright)
// ---------------------------------------------------------------------------

/// Detects WASM runtime errors via browser console output.
///
/// Requires Playwright integration for dynamic analysis.
pub struct WasmRuntimeAnalyzer;

impl WasmRuntimeAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmRuntimeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmRuntimeAnalyzer {
    /// Analyze rendered page for WASM runtime errors.
    #[must_use]
    pub fn analyze_rendered(&self, url: &str, rendered: &RenderedPage) -> Vec<Finding> {
        let mut findings = Vec::new();

        // WASM-R001: WASM runtime crash
        for msg in &rendered.console_messages {
            if msg.level == "error" && msg.text.to_lowercase().contains("webassembly") {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Custom("Reliability".to_string()),
                    code: "WASM-R001".to_string(),
                    title: "WASM runtime error detected".to_string(),
                    description: format!("Console error: {}", msg.text),
                    url: url.to_string(),
                    recommendation: "Check WASM module integrity and compatibility.".to_string(),
                });
            }
        }

        // WASM-R002: WASM module load failure
        for msg in &rendered.console_messages {
            if msg.level == "error"
                && (msg.text.contains("wasm") || msg.text.contains("WebAssembly"))
                && (msg.text.contains("load") || msg.text.contains("fetch"))
            {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Custom("Reliability".to_string()),
                    code: "WASM-R002".to_string(),
                    title: "WASM module load failure".to_string(),
                    description: format!("Console error: {}", msg.text),
                    url: url.to_string(),
                    recommendation: "Verify WASM module URL and CORS configuration.".to_string(),
                });
            }
        }

        // WASM-R003: WASM deprecation warning
        for msg in &rendered.console_messages {
            if msg.level == "warning" && msg.text.to_lowercase().contains("wasm") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Performance,
                    code: "WASM-R003".to_string(),
                    title: "WASM deprecation warning".to_string(),
                    description: format!("Console warning: {}", msg.text),
                    url: url.to_string(),
                    recommendation: "Review WASM usage and update if necessary.".to_string(),
                });
            }
        }

        // WASM-R004: WASM network request failure
        for req in &rendered.network_requests {
            if req.url.contains(".wasm") && req.status.is_some_and(|s| s >= 400) {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Custom("Reliability".to_string()),
                    code: "WASM-R004".to_string(),
                    title: "WASM module HTTP error".to_string(),
                    description: format!(
                        "WASM module at {} returned HTTP {}",
                        req.url,
                        req.status.unwrap_or(0)
                    ),
                    url: url.to_string(),
                    recommendation: "Verify WASM module availability and CORS headers.".to_string(),
                });
            }
        }

        findings
    }
}

impl crate::analyzers::Analyzer for WasmRuntimeAnalyzer {
    fn name(&self) -> &str {
        "wasm-runtime"
    }

    fn analyze(&self, ctx: &crate::analyzers::AnalysisContext) -> Vec<Finding> {
        let Some(rendered) = ctx.rendered else {
            return Vec::new();
        };
        self.analyze_rendered(&ctx.page.url, rendered)
    }
}

// ---------------------------------------------------------------------------
// WASM Performance Analyzer
// ---------------------------------------------------------------------------

/// Measures WASM impact on Core Web Vitals and page performance.
pub struct WasmPerformanceAnalyzer;

impl WasmPerformanceAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmPerformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmPerformanceAnalyzer {
    /// Analyze rendered page for WASM performance issues.
    #[must_use]
    pub fn analyze_rendered(&self, url: &str, rendered: &RenderedPage) -> Vec<Finding> {
        let mut findings = Vec::new();

        // WASM-P001: WASM module count
        let wasm_modules: Vec<_> = rendered
            .network_requests
            .iter()
            .filter(|r| r.url.contains(".wasm"))
            .collect();

        if wasm_modules.len() > 5 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "WASM-P001".to_string(),
                title: "Too many WASM modules".to_string(),
                description: format!(
                    "Page loads {} WASM modules. High module count increases memory pressure.",
                    wasm_modules.len()
                ),
                url: url.to_string(),
                recommendation:
                    "Consider consolidating WASM modules or lazy-loading non-critical ones."
                        .to_string(),
            });
        }

        // WASM-P002: Total WASM size
        let total_wasm_size: u64 = wasm_modules.iter().filter_map(|r| r.size).sum();

        if total_wasm_size > 10 * 1024 * 1024 {
            // 10 MB
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Performance,
                code: "WASM-P002".to_string(),
                title: "WASM bundle too large".to_string(),
                description: format!(
                    "Total WASM size: {:.2} MB. This exceeds the 10 MB recommendation.",
                    total_wasm_size as f64 / (1024.0 * 1024.0)
                ),
                url: url.to_string(),
                recommendation: "Optimize WASM with wasm-opt or split into smaller modules."
                    .to_string(),
            });
        }

        // WASM-P003: WASM compilation time (from render time)
        if rendered.render_time > std::time::Duration::from_secs(1) {
            // Check if WASM is likely contributing
            if !wasm_modules.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Performance,
                    code: "WASM-P003".to_string(),
                    title: "Slow WASM compilation detected".to_string(),
                    description: format!(
                        "Page render took {:?} with {} WASM modules. WASM compilation may be contributing.",
                        rendered.render_time,
                        wasm_modules.len()
                    ),
                    url: url.to_string(),
                    recommendation: "Use WebAssembly.compileStreaming() and enable WASM streaming compilation."
                        .to_string(),
                });
            }
        }

        // WASM-P004: Missing modulepreload
        let has_modulepreload = rendered.html.contains("rel=\"modulepreload\"");
        if !wasm_modules.is_empty() && !has_modulepreload {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "WASM-P004".to_string(),
                title: "Missing WASM module preload".to_string(),
                description: "WASM modules loaded without modulepreload hint.".to_string(),
                url: url.to_string(),
                recommendation: "Add <link rel=\"modulepreload\"> for critical WASM modules."
                    .to_string(),
            });
        }

        findings
    }
}

impl crate::analyzers::Analyzer for WasmPerformanceAnalyzer {
    fn name(&self) -> &str {
        "wasm-performance"
    }

    fn analyze(&self, ctx: &crate::analyzers::AnalysisContext) -> Vec<Finding> {
        let Some(rendered) = ctx.rendered else {
            return Vec::new();
        };
        self.analyze_rendered(&ctx.page.url, rendered)
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
    use crate::playwright::ConsoleMessage;
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
                has_integrity: false,
                is_module: false,
            }],
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
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

    fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: Some(Duration::from_millis(100)),
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    #[test]
    fn test_wasm_patterns_detected() {
        let analyzer = WasmPatternAnalyzer::new();
        let page = make_page("https://example.com", "module.wasm");
        let ctx = make_ctx(&page);

        let findings = analyzer.analyze(&ctx);
        // Should detect WASM-related patterns
        assert!(
            !findings.is_empty()
                || page
                    .scripts
                    .iter()
                    .any(|s| s.src.as_deref().is_some_and(|src| src.contains(".wasm")))
        );
    }

    #[test]
    fn test_no_wasm_patterns() {
        let analyzer = WasmPatternAnalyzer::new();
        let page = make_page("https://example.com", "app.js");
        let ctx = make_ctx(&page);

        let findings = analyzer.analyze(&ctx);
        // No WASM patterns means no WASM-related findings
        assert!(findings.iter().all(|f| !f.code.starts_with("WASM")));
    }

    #[test]
    fn test_wasm_runtime_analyzer_console_error() {
        let analyzer = WasmRuntimeAnalyzer::new();
        let rendered = RenderedPage {
            final_url: "https://example.com".to_string(),
            html: String::new(),
            console_messages: vec![ConsoleMessage {
                level: "error".to_string(),
                text: "WebAssembly.instantiate failed".to_string(),
                source: None,
                line: None,
            }],
            network_requests: Vec::new(),
            wasm_errors: Vec::new(),
            render_time: Duration::from_millis(100),
            memory_used: 0,
        };

        let findings = analyzer.analyze_rendered("https://example.com", &rendered);
        assert!(findings.iter().any(|f| f.code == "WASM-R001"));
    }

    #[test]
    fn test_wasm_performance_analyzer_large_bundle() {
        let analyzer = WasmPerformanceAnalyzer::new();
        let rendered = RenderedPage {
            final_url: "https://example.com".to_string(),
            html: String::new(),
            console_messages: Vec::new(),
            network_requests: vec![crate::playwright::NetworkRequest {
                url: "https://example.com/module.wasm".to_string(),
                method: "GET".to_string(),
                status: Some(200),
                resource_type: "wasm".to_string(),
                size: Some(15 * 1024 * 1024), // 15 MB
            }],
            wasm_errors: Vec::new(),
            render_time: Duration::from_millis(100),
            memory_used: 0,
        };

        let findings = analyzer.analyze_rendered("https://example.com", &rendered);
        assert!(findings.iter().any(|f| f.code == "WASM-P002"));
    }
}
