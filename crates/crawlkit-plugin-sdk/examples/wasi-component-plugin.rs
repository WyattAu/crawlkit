//! Example: WASI Preview 2 component plugin.
//!
//! WASI plugins are component-model executables implementing the
//! `crawlkit:plugin/analyze` interface — the same contract that the
//! engine's `wasi_preview2` adapter (`WasiPlugin`, behind the engine's
//! `wasi-preview2` feature) loads. Compared to the core WASM ABI
//! (`export_analyzer!`), components exchange *typed* strings with the
//! host instead of raw pointers and NUL-terminated JSON, and they can use
//! standard WASI capabilities (I/O, CLI, HTTP) when the host links them.
//!
//! This example shows both sides of the contract:
//!
//! 1. **Host side** (this file, compiled with `--features wasi`): loads a
//!    built component via `wasmtime::component::Component`, generates the
//!    typed bindings with `wasmtime::component::bindgen!`, links the
//!    host-provided `crawlkit:plugin/context` interface, and calls the
//!    component's `analyze` export.
//! 2. **Guest side** (`GUEST_SOURCE` below, built with
//!    `cargo component build`): a complete component implementing the
//!    interface.
//!
//! # Build and run
//!
//! ```bash
//! # Host harness (requires wasmtime >= 47 with component-model support)
//! cargo run -p crawlkit-plugin-sdk --features wasi \
//!     --example wasi-component-plugin -- /path/to/wasi_title_analyzer.wasm
//!
//! # Guest component
//! cargo install cargo-component
//! # ... scaffold the project as printed by the harness, then:
//! cargo component build --release
//! ```
//!
//! The findings JSON returned by `analyze` uses the exact same schema as
//! the core ABI (`crawlkit_plugin_analyze`), so findings produced by
//! either ABI flow through the same reporting pipeline.

#[cfg(feature = "wasi")]
mod demo {
    // Typed host-side bindings for the crawlkit WASI plugin contract.
    //
    // The world below is what a crawlkit WASI component must implement:
    // it exports `crawlkit:plugin/analyze` and may import the
    // host-provided `crawlkit:plugin/context` interface to read the
    // structured analysis context (the same JSON blob the core ABI
    // exposes via `crawlkit_host.get_context`).
    //
    // In a real component this WIT lives in the guest's `wit/` directory;
    // it is inlined here so the host bindings stay in one file.
    wasmtime::component::bindgen!({
        inline: r#"
        package crawlkit:plugin;

        interface context {
            /// Structured analysis context (AnalysisContext as JSON), set
            /// by the host before each `analyze` call.
            get-context: func() -> option<string>;
        }

        interface analyze {
            /// Analyze one page. Returns a JSON array of findings (same
            /// schema as the core ABI) or an error string.
            analyze: func(html: string, url: string) -> result<string, string>;
        }

        world crawlkit-analyzer {
            import context;
            export analyze;
        }
        "#,
    });

    /// Host state handed to the component.
    struct HostState {
        /// The AnalysisContext JSON blob returned by `get-context`.
        context_json: Option<String>,
    }

    impl crawlkit::plugin::context::Host for HostState {
        fn get_context(&mut self) -> Option<String> {
            self.context_json.clone()
        }
    }

    /// Complete guest component source (`src/lib.rs` of a
    /// `cargo component new` project).
    pub const GUEST_SOURCE: &str = r##"
// Cargo.toml
//
// [package]
// name = "wasi-title-analyzer"
// version = "0.1.0"
// edition = "2021"
//
// [lib]
// crate-type = ["cdylib"]
//
// [dependencies]
// wit-bindgen = "0.57"
// serde_json = "1"
//
// [package.metadata.component]
// package = "crawlkit:plugin"
//
// wit/plugin.wit — the crawlkit:plugin package from the top of this
// example (interface context, interface analyze, world crawlkit-analyzer).

wit_bindgen::generate!({
    path: "wit",
    world: "crawlkit-analyzer",
});

use exports::crawlkit::plugin::analyze::Guest;

struct TitleAnalyzer;

impl Guest for TitleAnalyzer {
    fn analyze(html: String, url: String) -> Result<String, String> {
        let mut findings: Vec<serde_json::Value> = Vec::new();

        // 1. Plain HTML checks.
        if !html.contains("<title") {
            findings.push(serde_json::json!({
                "severity": "error",
                "category": "seo",
                "code": "TITLE001",
                "title": "Missing <title>",
                "description": "The page has no <title> element",
                "url": url,
                "recommendation": "Add a <title> element to the document head",
            }));
        }

        // 2. Structured host context via crawlkit:plugin/context#get-context.
        if let Some(ctx_json) = crawlkit::plugin::context::get_context() {
            let ctx: serde_json::Value =
                serde_json::from_str(&ctx_json).unwrap_or(serde_json::Value::Null);
            if let Some(code) = ctx.get("status_code").and_then(|s| s.as_u64()) {
                if (400..=599).contains(&code) {
                    findings.push(serde_json::json!({
                        "severity": "warning",
                        "category": "http",
                        "code": "SOFT404",
                        "title": "Error page analyzed as content",
                        "description": format!(
                            "This URL returned HTTP {code} but was still analyzed"
                        ),
                        "url": url,
                        "recommendation": "Fix internal links pointing at the error response",
                    }));
                }
            }
        }

        Ok(serde_json::to_string(&findings).unwrap_or_else(|_| "[]".to_string()))
    }
}

export!(TitleAnalyzer);
"##;

    /// crawlkit-plugin.toml for the component above.
    pub const MANIFEST: &str = r#"
[plugin]
name = "wasi-title-analyzer"
version = "0.1.0"
api_version = "1.0"
author = "Your Name"
description = "WASI component title analyzer"
license = "MIT"
kind = "wasi-component"          # selects the WASI Preview 2 adapter
trust_level = "untrusted"
wasm_hash = "<sha256 of the .wasm>"

[plugin.entry]
wasm = "wasi_title_analyzer.wasm"
"#;

    const SAMPLE_URL: &str = "https://example.com/";
    const SAMPLE_HTML: &str = "<html><head></head><body><h1>hello</h1></body></html>";

    pub fn run() {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            print_guide();
            return;
        }
        match load_and_analyze(&args[1]) {
            Ok(findings_json) => println!("findings: {findings_json}"),
            Err(err) => eprintln!("error: {err}"),
        }
    }

    /// Compile the component, link the host interfaces, and call
    /// `crawlkit:plugin/analyze#analyze` on a sample page.
    fn load_and_analyze(wasm_path: &str) -> Result<String, String> {
        // Same resource-limit posture as the engine: fuel metering on.
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&config).map_err(|e| format!("engine: {e}"))?;

        // Compile as a *component* (not a core wasm Module).
        let component = wasmtime::component::Component::from_file(&engine, wasm_path)
            .map_err(|e| format!("compile component: {e}"))?;

        let mut linker = wasmtime::component::Linker::<HostState>::new(&engine);
        crawlkit::plugin::context::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(&mut linker, |state| state)
        .map_err(|e| format!("link crawlkit:plugin/context: {e}"))?;

        let mut store = wasmtime::Store::new(
            &engine,
            HostState {
                context_json: Some(format!(
                    r#"{{"url":"{SAMPLE_URL}","status_code":200,"headers":[],"response_time_ms":120}}"#
                )),
            },
        );
        store
            .set_fuel(50_000_000u64)
            .map_err(|e| format!("set fuel: {e}"))?;

        let analyzer = CrawlkitAnalyzer::instantiate(&mut store, &component, &linker)
            .map_err(|e| format!("instantiate: {e}"))?;

        // Typed call: the outer Result is a host trap, the inner Result is
        // the WIT `result<string, string>` (findings JSON or error).
        analyzer
            .crawlkit_plugin_analyze()
            .call_analyze(&mut store, SAMPLE_HTML, SAMPLE_URL)
            .map_err(|e| format!("call analyze: {e}"))?
    }

    fn print_guide() {
        println!("WASI Preview 2 component plugin example");
        println!("=========================================");
        println!();
        println!("Pass a built component to run it against a sample page:");
        println!("  cargo run -p crawlkit-plugin-sdk --features wasi \\");
        println!("      --example wasi-component-plugin -- plugin.wasm");
        println!();
        println!("The crawlkit WASI plugin contract (world `crawlkit-analyzer`):");
        println!();
        println!("  package crawlkit:plugin;");
        println!();
        println!("  interface context {{ get-context: func() -> option<string>; }}");
        println!(
            "  interface analyze {{ analyze: func(html: string, url: string) \
             -> result<string, string>; }}"
        );
        println!();
        println!("Guest component source (build with `cargo component build --release`):");
        println!();
        for line in GUEST_SOURCE.lines() {
            println!("{line}");
        }
        println!();
        println!("crawlkit-plugin.toml:");
        println!();
        for line in MANIFEST.lines() {
            println!("{line}");
        }
    }
}

#[cfg(feature = "wasi")]
fn main() {
    demo::run();
}

#[cfg(not(feature = "wasi"))]
fn main() {
    println!("This example needs the `wasi` feature (wasmtime >= 47, component-model):");
    println!("  cargo run -p crawlkit-plugin-sdk --features wasi --example wasi-component-plugin");
}
