# Plugin Development Guide

This guide covers creating custom WASM plugins for crawlkit.

## Overview

Plugins extend crawlkit's analyzer pipeline with custom SEO checks. They run in a
sandboxed WASM environment for security.

## Prerequisites

- Rust 1.94.0 or newer with the `wasm32-unknown-unknown` target
- crawlkit-plugin-sdk crate

```bash
rustup target add wasm32-unknown-unknown
```

## Quick Start

### 1. Create a new crate

```bash
cargo new my-seo-plugin
cd my-seo-plugin
```

### 2. Add dependencies

```toml
[dependencies]
crawlkit-plugin-sdk = "1.0.0"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 3. Implement the analyzer

```rust
use crawlkit_plugin_sdk::{Analyzer, Finding, Severity, AnalysisContext};

pub struct MyAnalyzer;

impl MyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MyAnalyzer {
    fn name(&self) -> &str {
        "my-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Your analysis logic here
        if ctx.html.contains("something") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: "custom".into(),
                code: "CUSTOM001".into(),
                title: "Something detected".into(),
                description: "The page contains something".into(),
                url: ctx.url.clone(),
                recommendation: "Remove something".into(),
            });
        }

        findings
    }
}

// Export for WASM
crawlkit_plugin_sdk::export_analyzer!(MyAnalyzer);
```

### 4. Build for WASM

```bash
cargo build --target wasm32-unknown-unknown --release
```

The `.wasm` file will be at `target/wasm32-unknown-unknown/release/my_seo_plugin.wasm`.

### 5. Create plugin manifest

Create `crawlkit-plugin.toml` in the plugin directory:

```toml
[plugin]
name = "my-seo-plugin"
version = "1.0.0"
api_version = "1.0"
author = "Your Name"
description = "My custom SEO analyzer"
license = "Apache-2.0"
trust_level = "untrusted"

[plugin.entry]
wasm = "my_seo_plugin.wasm"

[plugin.permissions]
network = false
filesystem = false
env_vars = []

[plugin.analyzer]
name = "my-analyzer"
category = "custom"
description = "My custom analyzer"
severity = "warning"
```

### 6. Sign the plugin

Crawlkit verifies an ed25519 trust chain before a plugin is compiled —
unsigned plugins are rejected by default. After building the `.wasm`,
sign it so it loads (see [Signing and Verifying Plugins](#signing-and-verifying-plugins) for the full walkthrough):

```bash
crawlkit plugin keygen --out ./keys          # once per author/machine
crawlkit plugin sign --plugin ./my-seo-plugin --key ./keys/plugin-signing.key
crawlkit plugin verify --plugin ./my-seo-plugin
```

Signing adds three fields to the manifest:

```toml
[plugin]
# ... fields above ...
wasm_hash = "9f2c..."   # hex sha256 of the .wasm
signature = "ab12..."   # hex ed25519 signature over the raw 32-byte sha256 digest
signed_by = "1f29..."   # key id: first 16 hex chars of the signer's public key
```

### 7. Install the plugin

```bash
# Copy plugin to crawlkit plugin directory
mkdir -p ~/.crawlkit/plugins/my-seo-plugin
cp target/wasm32-wasi/release/my_seo_plugin.wasm ~/.crawlkit/plugins/my-seo-plugin/
cp crawlkit-plugin.toml ~/.crawlkit/plugins/my-seo-plugin/

# Or use the CLI
crawlkit plugin install ./my-seo-plugin/
```

## Signing and Verifying Plugins

Plugins are trusted via a manifest signature chain verified *before* the
`.wasm` is handed to the WASM compiler:

1. the manifest's `wasm_hash` must match the actual sha256 of the `.wasm`;
2. `signature` must be a valid ed25519 signature over the raw 32-byte
   sha256 digest (not over the hex string);
3. `signed_by` must be the key id of a public key in the engine's built-in
   trust store (`TRUSTED_PLUGIN_KEYS` in `crates/crawlkit-engine/src/plugin.rs`).

The default policy (`PluginVerification::Required`) rejects any plugin
missing or failing these checks. Embedders loading untrusted local plugins
can opt into `PluginVerification::AllowUnsigned`, which logs a warning for
unsigned plugins — but a *present* hash or signature that fails
verification is always rejected, under every policy.

### Walkthrough: keygen → build → sign → verify

```bash
# 1. Generate an ed25519 signing keypair (once).
#    Creates keys/plugin-signing.key (secret — keep safe) and
#    keys/plugin-signing.pub (public).
crawlkit plugin keygen --out ./keys

# 2. Build the plugin for WASM.
cargo build --target wasm32-unknown-unknown --release

# 3. Sign the plugin directory: hashes the .wasm referenced by the
#    manifest, signs the digest, and writes wasm_hash / signature /
#    signed_by into crawlkit-plugin.toml.
crawlkit plugin sign --plugin ./my-seo-plugin --key ./keys/plugin-signing.key

# 4. Verify the trust chain (same check the loader performs).
crawlkit plugin verify --plugin ./my-seo-plugin
```

Note: a plugin signed with your own key loads under the `Required` policy
only if your public key is added to the engine's trust store — key
addition and rotation happen via PR and a release. For local development,
either add your dev key to the trust store or run with an `AllowUnsigned`
plugin configuration.

## API Reference

### AnalysisContext

```rust
pub struct AnalysisContext {
    pub url: String,           // Page URL
    pub html: String,          // HTML content
    pub status_code: Option<u16>,  // HTTP status
    pub headers: Vec<(String, String)>,  // Response headers
    pub response_time_ms: Option<u64>,  // Response time
}
```

### Finding

```rust
pub struct Finding {
    pub severity: Severity,    // Critical/Error/Warning/Info
    pub category: String,      // Issue category
    pub code: String,          // Machine-readable code
    pub title: String,         // Short title
    pub description: String,   // Detailed description
    pub url: String,           // Page URL
    pub recommendation: String, // How to fix
}
```

### Severity

```rust
pub enum Severity {
    Critical,  // Must fix
    Error,     // Should fix
    Warning,   // Recommended
    Info,      // Informational
}
```

## Testing

```bash
# Test with sample HTML
crawlkit plugin test ./my-seo-plugin/ --html "<html><head><title>Test</title></head></html>"

# Test with a URL
crawlkit plugin test ./my-seo-plugin/ --url https://example.com
```

## Security

- Plugins run in WASM sandbox
- No file/network access unless granted
- Memory limited to 64MB
- CPU limited to 100ms per call
- All inputs are validated
- ed25519 manifest signatures verified before compilation (fail-closed by default)

## Troubleshooting

### Plugin fails to load

- Check `crawlkit-plugin.toml` syntax
- Verify WASM file exists at specified path
- Ensure API version is compatible (1.x)

### Plugin crashes

- Check for null pointer dereferences
- Verify memory allocation/deallocation
- Test with minimal HTML first

### Plugin returns no findings

- Verify analyzer is registered
- Check finding serialization
- Test with known-good HTML

## Structured context (B4)

Beyond the raw HTML string, your analyzer can read the page's structured
context — status code, headers, response time, and a parsed summary —
through the host function `crawlkit_host.get_context`. The SDK wraps it:

```rust
use crawlkit_plugin_sdk::host::{self, HostContext};

fn analyze_response(ctx: &HostContext) -> Vec<Finding> {
    match ctx.status_code {
        Some(404) => { /* soft-404 handling */ }
        Some(code @ 500..=599) => { /* server error page */ }
        _ => {}
    }
    if let Some(parsed) = &ctx.parsed {
        // parsed.title / parsed.description / parsed.word_count /
        // parsed.headings / parsed.link_count / parsed.image_count / parsed.lang
    }
    vec![]
}

// inside impl Analyzer::analyze:
if let Some(Ok(host_ctx)) = host::context() {
    return analyze_response(&host_ctx);
}
```

Guarantees:

- **No manifest declaration needed** — the context exposes nothing the
  raw HTML input doesn't already convey; it is precomputed convenience.
- **Graceful degradation** — `host::context()` returns `None` when the
  plugin is run without context (e.g. via a plain loader), so plugins
  work in both modes.
- **Find the JSON shape** in `crawlkit_plugin_sdk::host::HostContext`;
  the engine writes it before each `analyze_with_context` call.

A complete example ships as
`crates/crawlkit-plugin-sdk/examples/soft-404.rs` (flags error pages
that were still analyzed).
