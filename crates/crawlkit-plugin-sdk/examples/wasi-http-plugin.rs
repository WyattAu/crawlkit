//! Example: WASI component plugin with HTTP outcalls.
//!
//! Demonstrates a WASI Preview 2 component that fetches additional data
//! from an external API during analysis via the standard
//! `wasi:http/outgoing-handler` interface — something the core WASM ABI
//! cannot do (raw `wasm32-unknown-unknown` plugins have no host calls).
//!
//! # Capability model
//!
//! HTTP access is capability-gated on *both* sides:
//!
//! 1. The plugin manifest must request it:
//!
//!    ```toml
//!    [plugin.permissions]
//!    network = true
//!    ```
//!
//! 2. The host must grant it: `WasmConfig.allow_plugin_network = true`.
//!
//! When granted, the engine still enforces SSRF protections on every
//! request: no redirects, a 1 MiB response cap, and a 10-second timeout.
//! Each WASI interface is only available if the host explicitly links it,
//! so components cannot reach the network unless the host provides
//! `wasi:http` (plus `wasi:io`) implementations.
//!
//! # Current engine status (Gate 5)
//!
//! The engine's `wasi_preview2` adapter compiles and runs HTTP components,
//! but until `wasmtime-wasi` / `wasmtime-wasi-http` host implementations
//! are linked, unlinked WASI imports are defined as traps
//! (`define_unknown_imports_as_traps`). This harness mirrors that behavior
//! exactly: instantiating the component succeeds, and the trap fires when
//! the component first calls `outgoing-handler.handle` — with a message
//! pointing at the missing capability. Linking the real WASI HTTP host
//! (see `wasmtime-wasi`, declared behind this crate's `wasi` feature) is
//! what turns this example from "traps" into "fetches".
//!
//! # Build and run
//!
//! ```bash
//! # Host harness (requires wasmtime >= 47 with component-model support)
//! cargo run -p crawlkit-plugin-sdk --features wasi \
//!     --example wasi-http-plugin -- /path/to/domain_reputation.wasm
//!
//! # Guest component (vendored wasi-http + wasi-io WIT, see GUEST_SOURCE)
//! cargo install cargo-component
//! cargo component build --release
//! ```

#[cfg(feature = "wasi")]
mod demo {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Host state for the component, mirroring
    /// `crawlkit_engine::plugin::wasi_preview2::WasiPluginState`.
    struct HostState {
        /// Analysis context (AnalysisContext as JSON) served through
        /// `crawlkit:plugin/context#get-context`.
        context_json: Option<String>,
        /// Whether the network capability was granted to this plugin.
        allow_network: bool,
    }

    /// Sample inputs the harness analyzes.
    const SAMPLE_URL: &str = "https://example.com/";
    const SAMPLE_HTML: &str =
        "<html><head></head><body><a href=\"/1\">1</a><a href=\"/2\">2</a></body></html>";

    /// Wall-clock budget for the analysis call, enforced via epoch
    /// interruption (same mechanism as the engine).
    const TIMEOUT_MS: u64 = 10_000;
    const MAX_FUEL: u64 = 50_000_000;

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

    /// Load a component and call its `analyze` export, mirroring the
    /// engine's `WasiPlugin::analyze` (raw `Val` API, traps for unlinked
    /// WASI imports, fuel + epoch limits).
    fn load_and_analyze(wasm_path: &str) -> Result<String, String> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = wasmtime::Engine::new(&config).map_err(|e| format!("engine: {e}"))?;

        let component = wasmtime::component::Component::from_file(&engine, wasm_path)
            .map_err(|e| format!("compile component: {e}"))?;

        // Trap on any WASI import the host has not linked: components that
        // import wasi:http run, but their first HTTP call traps until the
        // real wasi:http host implementation is linked (Gate 5). Linking
        // wasmtime-wasi / wasmtime-wasi-http here is the upgrade path.
        let mut linker = wasmtime::component::Linker::<HostState>::new(&engine);
        linker
            .define_unknown_imports_as_traps(&component)
            .map_err(|e| format!("define unknown imports: {e}"))?;
        link_crawlkit_context(&mut linker)?;

        // The network capability is granted only when both the manifest
        // requests it and the host config allows it. This harness grants
        // it to mirror `allow_plugin_network = true`.
        let mut store = wasmtime::Store::new(
            &engine,
            HostState {
                context_json: Some(format!(
                    r#"{{"url":"{SAMPLE_URL}","status_code":200,"headers":[],"response_time_ms":120}}"#
                )),
                allow_network: true,
            },
        );
        store
            .set_fuel(MAX_FUEL)
            .map_err(|e| format!("set fuel: {e}"))?;
        store.set_epoch_deadline(1);

        let watchdog = Watchdog::spawn(engine, TIMEOUT_MS);

        if store.data().allow_network {
            println!("network capability: granted (the host SSRF-validates every request)");
        } else {
            println!("network capability: not granted; the component's HTTP calls will fail");
        }

        let instance = linker
            .instantiate(&mut store, &component)
            .map_err(|e| format!("instantiate: {e}"))?;

        let analyze_func = instance
            .get_func(&mut store, "crawlkit:plugin/analyze#analyze")
            .ok_or_else(|| {
                "component does not export crawlkit:plugin/analyze#analyze".to_string()
            })?;

        let mut results = [wasmtime::component::Val::Bool(false)];
        let result = analyze_func.call(
            &mut store,
            &[
                wasmtime::component::Val::String(SAMPLE_HTML.to_string()),
                wasmtime::component::Val::String(SAMPLE_URL.to_string()),
            ],
            &mut results,
        );

        watchdog.stop();

        result.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("epoch") || msg.contains("interrupt") {
                format!("component exceeded the {TIMEOUT_MS}ms analysis timeout")
            } else if msg.contains("all fuel consumed") {
                format!("component exhausted its {MAX_FUEL} instruction fuel budget")
            } else if msg.contains("unlinked") {
                format!(
                    "the component called a WASI import this host has not linked; \
                     HTTP outcalls require the host to link wasi:http/wasi-io \
                     (wasmtime-wasi + wasmtime-wasi-http). Original trap: {msg}"
                )
            } else {
                format!("analyze failed: {e}")
            }
        })?;

        extract_findings(&results[0])
    }

    /// Link the host-side `crawlkit:plugin/context#get-context` function,
    /// exactly as the engine does (raw `Val` API — `option<string>` maps to
    /// `Val::Option`).
    fn link_crawlkit_context(
        linker: &mut wasmtime::component::Linker<HostState>,
    ) -> Result<(), String> {
        let mut instance = linker
            .instance("crawlkit:plugin/context")
            .map_err(|e| format!("crawlkit:plugin/context instance: {e}"))?;
        instance
            .func_new(
                "get-context",
                |ctx: wasmtime::StoreContextMut<'_, HostState>,
                 _ty: wasmtime::component::types::ComponentFunc,
                 _params: &[wasmtime::component::Val],
                 results: &mut [wasmtime::component::Val]| {
                    results[0] = match ctx.data().context_json.clone() {
                        Some(json) => wasmtime::component::Val::Option(Some(Box::new(
                            wasmtime::component::Val::String(json),
                        ))),
                        None => wasmtime::component::Val::Option(None),
                    };
                    Ok(())
                },
            )
            .map_err(|e| format!("link get-context: {e}"))?;
        Ok(())
    }

    /// Unpack the WIT `result<string, string>` return value the same way
    /// the engine's adapter does.
    fn extract_findings(value: &wasmtime::component::Val) -> Result<String, String> {
        match value {
            wasmtime::component::Val::Result(result_val) => match result_val {
                Ok(Some(inner)) => match inner.as_ref() {
                    wasmtime::component::Val::String(json) => Ok(json.clone()),
                    other => Err(format!("unexpected ok payload: {other:?}")),
                },
                Ok(None) => {
                    Err("analyze returned ok(()) but a JSON string is expected".to_string())
                }
                Err(Some(inner)) => Err(format!("analyze returned error: {inner:?}")),
                Err(None) => Err("analyze returned an error".to_string()),
            },
            wasmtime::component::Val::String(json) => Ok(json.clone()),
            other => Err(format!("unexpected return value: {other:?}")),
        }
    }

    /// Wall-clock watchdog: bumps the engine epoch after `timeout_ms`,
    /// aborting any in-flight guest call (see `EpochWatchdog` in the
    /// engine).
    struct Watchdog {
        stop_flag: Arc<AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    impl Watchdog {
        fn spawn(engine: wasmtime::Engine, timeout_ms: u64) -> Self {
            let stop_flag = Arc::new(AtomicBool::new(false));
            let thread_flag = stop_flag.clone();
            let handle = std::thread::spawn(move || {
                let deadline =
                    std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
                while std::time::Instant::now() < deadline {
                    if thread_flag.load(Ordering::Relaxed) {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                engine.increment_epoch();
            });
            Self {
                stop_flag,
                handle: Some(handle),
            }
        }

        fn stop(mut self) {
            self.stop_flag.store(true, Ordering::Relaxed);
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    fn print_guide() {
        println!("WASI HTTP plugin example (wasi:http/outgoing-handler)");
        println!("======================================================");
        println!();
        println!("Pass a built component to run it against a sample page:");
        println!("  cargo run -p crawlkit-plugin-sdk --features wasi \\");
        println!("      --example wasi-http-plugin -- plugin.wasm");
        println!();
        println!("Capabilities: the manifest must request [plugin.permissions] network = true,");
        println!("and the host must set WasmConfig.allow_plugin_network = true. The engine");
        println!("enforces no-redirect, 1 MiB response cap, and 10s timeout per request.");
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

    /// Complete guest component source (`src/lib.rs` of a
    /// `cargo component new` project).
    ///
    /// WIT layout expected by `wit_bindgen::generate!`:
    ///
    /// ```text
    /// wit/
    ///   plugin.wit                    <- the crawlkit:plugin package
    ///   deps/wasi-http/types.wit      <- vendored from
    ///   deps/wasi-http/outgoing-handler.wit   github.com/WebAssembly/wasi-http
    ///   deps/wasi-io/poll.wit         (v0.2.x releases)
    ///   deps/wasi-io/streams.wit
    ///   deps/wasi-io/error.wit
    /// ```
    pub const GUEST_SOURCE: &str = r##"
// Cargo.toml
//
// [package]
// name = "domain-reputation"
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
// target = "wasip2"    # default for cargo component

wit_bindgen::generate!({
    path: "wit",
    world: "http-analyzer",
});

use exports::crawlkit::plugin::analyze::Guest;
use wasi::http::outgoing_handler;
use wasi::http::types::{Fields, Method, OutgoingRequest, Scheme};
use wasi::io::streams::StreamError;

// wit/plugin.wit — the component's world:
//
//   package crawlkit:plugin;
//
//   interface analyze {
//       analyze: func(html: string, url: string) -> result<string, string>;
//   }
//
//   world http-analyzer {
//       import wasi:http/types@0.2.0;
//       import wasi:http/outgoing-handler@0.2.0;
//       import wasi:io/error@0.2.0;
//       import wasi:io/poll@0.2.0;
//       import wasi:io/streams@0.2.0;
//       export analyze;
//   }

/// Fetch a URL and return the response body as a string. The host
/// validates every request (SSRF policy: no redirects, 1 MiB cap, 10s
/// timeout) and rejects disallowed ones via the `handle` error.
fn fetch(url: &str) -> Result<String, String> {
    // Minimal URL split: scheme://authority/path?query
    let (scheme, rest) = url.split_once("://").ok_or("invalid URL")?;
    let (authority, path_and_query) = match rest.split_once('/') {
        Some((host, path)) => (host, format!("/{path}")),
        None => (rest, "/".to_string()),
    };

    let request = OutgoingRequest::new(Fields::new());
    request.set_method(&Method::Get).map_err(|_| "bad method")?;
    request
        .set_scheme(Some(&if scheme == "http" {
            Scheme::Http
        } else {
            Scheme::Https
        }))
        .map_err(|_| "bad scheme")?;
    request.set_authority(Some(authority)).map_err(|_| "bad authority")?;
    request
        .set_path_with_query(Some(&path_and_query))
        .map_err(|_| "bad path")?;

    // Send the request; `None` options means "use host defaults" (the host
    // applies its own timeout/size limits).
    let future = outgoing_handler::handle(request, None)
        .map_err(|err| format!("outgoing request rejected by host: {err:?}"))?;

    // Block until the response headers arrive, then unpack
    // option<result<result<incoming-response, error-code>, ()>>.
    future.subscribe().block();
    let response = match future.get() {
        Some(Ok(Ok(response))) => response,
        Some(Ok(Err(err))) => return Err(format!("HTTP error: {err:?}")),
        Some(Err(())) => return Err("response future dropped by the runtime".to_string()),
        None => return Err("response future not ready".to_string()),
    };

    let status = response.status();
    if status != 200 {
        return Err(format!("unexpected HTTP status {status}"));
    }

    // Stream the body to the end.
    let body = response.consume().map_err(|_| "body already consumed")?;
    let stream = body.stream().map_err(|_| "body stream already taken")?;
    let mut payload = Vec::new();
    loop {
        match stream.blocking_read(64 * 1024) {
            Ok(chunk) => payload.extend_from_slice(&chunk),
            Err(StreamError::Closed) => break,
            Err(StreamError::LastOperationFailed(err)) => {
                return Err(format!("body read failed: {err:?}"));
            }
        }
    }
    drop(stream);
    wasi::http::types::IncomingBody::finish(body);

    Ok(String::from_utf8_lossy(&payload).into_owned())
}

struct DomainReputationAnalyzer;

impl Guest for DomainReputationAnalyzer {
    fn analyze(html: String, url: String) -> Result<String, String> {
        let mut findings: Vec<serde_json::Value> = Vec::new();

        // 1. Local check: excessive outbound linking.
        let link_count = html.matches("<a ").count();
        if link_count > 100 {
            findings.push(serde_json::json!({
                "severity": "warning",
                "category": "links",
                "code": "LINKS001",
                "title": "Excessive outbound links",
                "description": format!("The page contains {link_count} <a> elements"),
                "url": url,
                "recommendation": "Review outbound linking; link farms are devalued",
            }));
        }

        // 2. Enrichment: query an external reputation API for the host.
        let authority = url
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(&url)
            .split('/')
            .next()
            .unwrap_or(&url)
            .to_string();

        match fetch(&format!("https://reputation.example.com/v1/domain?host={authority}")) {
            Ok(body) => {
                let blocklisted = serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v.get("blocklisted").and_then(|b| b.as_bool()))
                    .unwrap_or(false);
                if blocklisted {
                    findings.push(serde_json::json!({
                        "severity": "critical",
                        "category": "security",
                        "code": "REPUT001",
                        "title": "Domain is blocklisted",
                        "description": format!("The reputation service flagged {authority}"),
                        "url": url,
                        "recommendation": "Investigate the domain before trusting its content",
                    }));
                }
            }
            // Network failures must degrade gracefully, never fail the analysis.
            Err(err) => {
                findings.push(serde_json::json!({
                    "severity": "info",
                    "category": "http",
                    "code": "REPUT002",
                    "title": "Reputation lookup failed",
                    "description": err,
                    "url": url,
                    "recommendation": "None; the analysis continued without enrichment",
                }));
            }
        }

        Ok(serde_json::to_string(&findings).unwrap_or_else(|_| "[]".to_string()))
    }
}

export!(DomainReputationAnalyzer);
"##;

    /// crawlkit-plugin.toml for the component above.
    pub const MANIFEST: &str = r#"
[plugin]
name = "domain-reputation"
version = "0.1.0"
api_version = "1.0"
author = "Your Name"
description = "WASI component enriching analysis with an external reputation API"
license = "MIT"
kind = "wasi-component"
trust_level = "untrusted"
wasm_hash = "<sha256 of the .wasm>"

[plugin.entry]
wasm = "domain_reputation.wasm"

[plugin.permissions]
network = true                   # required for wasi:http/outgoing-handler
"#;
}

#[cfg(feature = "wasi")]
fn main() {
    demo::run();
}

#[cfg(not(feature = "wasi"))]
fn main() {
    println!("This example needs the `wasi` feature (wasmtime >= 47, component-model):");
    println!("  cargo run -p crawlkit-plugin-sdk --features wasi --example wasi-http-plugin");
}
