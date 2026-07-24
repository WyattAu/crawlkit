# WASM Error Detection Analyzers — Implementation Spec

**ADR Reference:** ADR-001
**Status:** Proposed
**Estimated Effort:** 2-3 weeks (static), 2 weeks (runtime, post-Phase 7.1)

---

## Overview

Three analyzers detect WebAssembly-related issues invisible to standard HTML/JS analysis:

1. **WasmPatternAnalyzer** — Static detection of WASM patterns in HTML/JS
2. **WasmRuntimeAnalyzer** — Dynamic detection via Playwright console errors
3. **WasmPerformanceAnalyzer** — WASM impact on Core Web Vitals

---

## Analyzer 1: WasmPatternAnalyzer (Static)

### Purpose
Detect WASM-related issues from HTML source without executing JavaScript.

### Detection Signals

| Signal ID | Pattern | Severity | Category | Description |
|-----------|---------|----------|----------|-------------|
| WASM001 | `<script>` fetches `.wasm` without `<link rel="modulepreload">` | Warning | Performance | Missing preload delays WASM compilation |
| WASM002 | `WebAssembly.instantiate` in synchronous context | Error | Performance | Synchronous WASM compilation blocks main thread |
| WASM003 | No error handler around `WebAssembly.instantiate` or `WebAssembly.compile` | Warning | Reliability | Unhandled WASM errors crash page |
| WASM004 | `.wasm` file > 5MB (detected via `Content-Length` or inline size hints) | Warning | Performance | Large WASM bundles hurt LCP/TTI |
| WASM005 | WASM module imported but no fallback for browsers without WASM support | Info | Compatibility | ~2% of browsers lack WASM support |
| WASM006 | `WebAssembly.instantiate` with `StreamingCompilation` but no `WebAssembly.compileStreaming` | Info | Performance | Suboptimal compilation path |

### Implementation

```rust
pub struct WasmPatternAnalyzer;

impl Analyzer for WasmPatternAnalyzer {
    fn name(&self) -> &str {
        "wasm-pattern"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let html = &ctx.page.raw_html; // New field needed on ParsedPage
        let url = &ctx.page.url;

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
                url: url.clone(),
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
                    url: url.clone(),
                    recommendation: "Replace WebAssembly.instantiate() with \
                        WebAssembly.instantiateStreaming() for non-blocking compilation."
                        .to_string(),
                });
                break;
            }
        }

        // WASM003: Missing error handler
        let has_wasm = html.contains("WebAssembly.instantiate")
            || html.contains("WebAssembly.compile");
        let has_try_catch = html.contains("try {") || html.contains("try{");
        let has_catch = html.contains("catch");
        if has_wasm && !(has_try_catch && has_catch) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Reliability,
                code: "WASM003".to_string(),
                title: "WASM instantiation without error handling".to_string(),
                description: "WebAssembly.instantiate/compile called without try/catch. \
                    Unhandled WASM errors will crash the page."
                    .to_string(),
                url: url.clone(),
                recommendation: "Wrap WASM instantiation in try/catch and provide \
                    a JS fallback or user-friendly error message."
                    .to_string(),
            });
        }

        findings
    }
}
```

### Test Vectors

| ID | Input | Expected Finding |
|----|-------|------------------|
| WASM-TV-001 | HTML with `<script src="app.wasm">` but no `<link rel="modulepreload">` | WASM001 |
| WASM-TV-002 | HTML with `WebAssembly.instantiate(bytes)` (synchronous) | WASM002 |
| WASM-TV-003 | HTML with `WebAssembly.instantiateStreaming(fetch("m.wasm"))` | No WASM002 |
| WASM-TV-004 | HTML with `WebAssembly.instantiate` inside `try {} catch {}` | No WASM003 |
| WASM-TV-005 | HTML with `WebAssembly.instantiate` but no try/catch | WASM003 |
| WASM-TV-006 | HTML with no WASM patterns | No findings |

---

## Analyzer 2: WasmRuntimeAnalyzer (Dynamic — Requires Playwright)

### Purpose
Detect WASM runtime errors via browser console output.

### Dependency
Requires Phase 7.1 (Playwright integration). This analyzer is **gated** on Playwright availability.

### Detection Signals

| Signal ID | Console Event | Severity | Category | Description |
|-----------|---------------|----------|----------|-------------|
| WASM-R001 | `pageerror` containing "WebAssembly" | Error | Reliability | WASM runtime crash |
| WASM-R002 | `console.error` containing "wasm" or "WebAssembly" | Error | Reliability | WASM module load failure |
| WASM-R003 | `console.warn` containing "wasm" | Warning | Performance | WASM degradation warning |
| WASM-R004 | Network request for `.wasm` returning 4xx/5xx | Error | Reliability | WASM module not found |
| WASM-R005 | WASM instantiation time > 500ms | Warning | Performance | Slow WASM compilation |

### Implementation (Post-Phase 7.1)

```rust
pub struct WasmRuntimeAnalyzer;

impl Analyzer for WasmRuntimeAnalyzer {
    fn name(&self) -> &str {
        "wasm-runtime"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        // Requires extended AnalysisContext with console_logs: Vec<ConsoleEntry>
        // and wasm_timings: Option<WasmTimings>
        //
        // Implementation deferred to Phase 7.1 (Playwright integration)
        //
        // Pseudocode:
        // for log in &ctx.console_logs {
        //     if log.level == Error && log.message.contains("WebAssembly") {
        //         findings.push(Finding { code: "WASM-R001", ... });
        //     }
        // }
        //
        // if let Some(timings) = &ctx.wasm_timings {
        //     if timings.instantiation_ms > 500 {
        //         findings.push(Finding { code: "WASM-R005", ... });
        //     }
        // }

        Vec::new() // Placeholder until Playwright integration
    }
}
```

### Extended AnalysisContext

```rust
pub struct AnalysisContext<'a> {
    pub page: &'a ParsedPage,
    pub status_code: Option<u16>,
    pub headers: &'a [(String, String)],
    pub response_time: Option<Duration>,
    pub redirect_chain: &'a [RedirectHop],
    // New fields for dynamic analysis:
    pub console_logs: &'a [ConsoleEntry],     // From Playwright
    pub wasm_timings: Option<WasmTimings>,    // From Playwright
}

pub struct ConsoleEntry {
    pub level: ConsoleLevel,  // Log, Warn, Error, Info
    pub message: String,
    pub source: String,       // URL of originating script
    pub timestamp_ms: u64,
}

pub enum ConsoleLevel { Log, Warn, Error, Info }

pub struct WasmTimings {
    pub module_count: usize,
    pub total_size_bytes: u64,
    pub instantiation_ms: u64,
    pub compilation_ms: u64,
}
```

---

## Analyzer 3: WasmPerformanceAnalyzer

### Purpose
Measure WASM impact on Core Web Vitals and page performance.

### Detection Signals

| Signal ID | Metric | Threshold | Severity | Description |
|-----------|--------|-----------|----------|-------------|
| WASM-P001 | WASM module count | > 5 | Warning | Too many WASM modules increase memory pressure |
| WASM-P002 | Total WASM size | > 10MB | Error | WASM bundle too large for mobile |
| WASM-P003 | WASM compilation time | > 1s | Error | Blocks interactive-ready |
| WASM-P004 | WASM memory usage | > 50MB | Warning | Excessive memory consumption |
| WASM-P005 | Missing modulepreload | Any | Warning | Delays compilation start |

### Implementation

Static signals (WASM-P001, WASM-P005) can ship with v1.0.
Dynamic signals (WASM-P002, WASM-P003, WASM-P004) require Playwright.

---

## File Locations

| File | Path | Purpose |
|------|------|---------|
| `wasm_analyzers.rs` | `crates/crawlkit-engine/src/wasm_analyzers.rs` | All 3 WASM analyzers |
| `wasm_test_vectors.toml` | `crates/crawlkit-engine/test_vectors/wasm_test_vectors.toml` | Test data |
| `wasm_integration_test.rs` | `crates/crawlkit-engine/tests/wasm_integration_test.rs` | Integration tests |

---

## Registration

Add to `AnalyzerRegistry::default()` in `analyzers.rs`:

```rust
// In AnalyzerRegistry::default()
Self::with_analyzers(vec![
    // ... existing 23 analyzers ...
    Box::new(WasmPatternAnalyzer::new()),
    // WASM runtime analyzer gated on Playwright:
    // Box::new(WasmRuntimeAnalyzer::new()),
    // Box::new(WasmPerformanceAnalyzer::new()),
])
```

---

## Dependencies

| Dependency | Version | Purpose | Status |
|------------|---------|---------|--------|
| `scraper` | 0.18+ | HTML parsing | Already in Cargo.toml |
| `url` | 2.4+ | URL parsing | Already in Cargo.toml |
| `playwright` | 0.7+ | Runtime analysis | Phase 7.1 dependency |

---

*Generated: 2026-07-23 | Version: 1.0.0*
