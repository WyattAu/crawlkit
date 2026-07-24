# Plugin Development Guide

This guide covers creating custom WASM plugins for crawlkit.

## Overview

Plugins extend crawlkit's analyzer pipeline with custom SEO checks. They run in a
sandboxed WASM environment for security.

## Prerequisites

- Rust 1.75+ with `wasm32-wasi` target
- crawlkit-plugin-sdk crate

```bash
rustup target add wasm32-wasi
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
cargo build --target wasm32-wasi --release
```

The `.wasm` file will be at `target/wasm32-wasi/release/my_seo_plugin.wasm`.

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

### 6. Install the plugin

```bash
# Copy plugin to crawlkit plugin directory
mkdir -p ~/.crawlkit/plugins/my-seo-plugin
cp target/wasm32-wasi/release/my_seo_plugin.wasm ~/.crawlkit/plugins/my-seo-plugin/
cp crawlkit-plugin.toml ~/.crawlkit/plugins/my-seo-plugin/

# Or use the CLI
crawlkit plugin install ./my-seo-plugin/
```

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
