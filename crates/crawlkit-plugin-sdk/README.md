# crawlkit-plugin-sdk

SDK for building crawlkit WASM plugins.

## Overview

This crate provides the types and traits needed to create custom SEO analyzers
that run as WASM plugins in crawlkit's sandboxed environment.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
crawlkit-plugin-sdk = "1.0.0"
```

## Example

```rust
use crawlkit_plugin_sdk::{Analyzer, Finding, Severity, AnalysisContext};

pub struct MyAnalyzer;

impl Analyzer for MyAnalyzer {
    fn name(&self) -> &str { "my-analyzer" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        // Your analysis logic here
        findings
    }
}

// Export for WASM
crawlkit_plugin_sdk::export_analyzer!(MyAnalyzer);
```

## Building for WASM

```bash
# Install WASM target
rustup target add wasm32-wasi

# Build plugin
cargo build --target wasm32-wasi --release

# The .wasm file will be in target/wasm32-wasi/release/
```

## WASI Preview 2 Components vs. the Core WASM ABI

crawlkit supports two plugin ABIs. The **core WASM ABI** (default) is a raw
`wasm32-unknown-unknown` module that exchanges NUL-terminated JSON through
hand-written exports (`crawlkit_plugin_init/alloc/analyze/free`). A **WASI
Preview 2** plugin is a component-model executable that talks to the host
through typed WIT interfaces and may use standard WASI capabilities — most
notably HTTP outcalls for enrichment during analysis.

| | Core WASM ABI (`export_analyzer!`) | WASI Preview 2 component |
|---|---|---|
| Build target | `wasm32-unknown-unknown` / `wasm32-wasi` | `wasip2` (via `cargo-component`) |
| Host entry point | `crawlkit_plugin_init` / `_alloc` / `_analyze` / `_free` | `crawlkit:plugin/analyze#analyze` |
| Data exchange | Raw pointers + NUL-terminated JSON | Typed strings: `analyze(html, url) -> result<string, string>` |
| Host context | `crawlkit_host.get_context` | `crawlkit:plugin/context#get-context` |
| HTTP outcalls | Not available | `wasi:http/outgoing-handler` (capability-gated) |
| Sandboxing | Fuel + epoch interruption limits | Same limits, plus capability-based imports (each WASI interface must be explicitly linked by the host) |
| Manifest `kind` | `"wasm"` (default when absent) | `"wasi-component"` |
| Engine gate | Always available | Engine built with the `wasi-preview2` feature |
| Requirements | Any wasmtime | **wasmtime >= 47** with component-model support |

### WASI component example

The SDK ships two runnable examples behind the `wasi` feature (this pulls in
optional `wasmtime` / `wasmtime-wasi` dependencies; the library itself never
requires them):

- [`examples/wasi-component-plugin.rs`](examples/wasi-component-plugin.rs) —
  a minimal component implementing `crawlkit:plugin/analyze`, plus a host
  harness that loads it with `wasmtime::component::bindgen!` bindings.
- [`examples/wasi-http-plugin.rs`](examples/wasi-http-plugin.rs) — a component
  that enriches its analysis with an external API call through
  `wasi:http/outgoing-handler`, and a host harness mirroring the engine's
  capability checks.

```bash
# Inspect the guest component source and build recipe
cargo run -p crawlkit-plugin-sdk --features wasi --example wasi-component-plugin

# Run the harness against a built component
cargo run -p crawlkit-plugin-sdk --features wasi \
    --example wasi-component-plugin -- /path/to/plugin.wasm
```

A component is built with [`cargo-component`](https://github.com/bytecodealliance/cargo-component):

```bash
cargo install cargo-component
cargo component new my-wasi-plugin
cargo component build --release
```

Its manifest selects the WASI adapter:

```toml
[plugin]
name = "my-wasi-plugin"
kind = "wasi-component"
api_version = "1.0"
# ...

[plugin.entry]
wasm = "my_wasi_plugin.wasm"

# Optional: request HTTP outcalls (also requires
# WasmConfig.allow_plugin_network = true on the host)
[plugin.permissions]
network = true
```

Network access is always SSRF-validated by the host (no redirects, 1 MiB
response cap, 10s timeout). Until the host links real `wasi:http`
implementations, components importing `wasi:http` run but their first HTTP
call traps (see the engine's `wasi_preview2` module, Gate 5).

## Documentation

- [API Documentation](https://docs.rs/crawlkit-plugin-sdk)
- [Plugin Development Guide](https://github.com/WyattAu/crawlkit/blob/main/docs/PLUGIN_DEVELOPMENT.md)
- [GitHub Repository](https://github.com/WyattAu/crawlkit)
