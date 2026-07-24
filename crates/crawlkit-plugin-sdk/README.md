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

## Documentation

- [API Documentation](https://docs.rs/crawlkit-plugin-sdk)
- [Plugin Development Guide](https://github.com/WyattAu/crawlkit/blob/main/docs/PLUGIN_DEVELOPMENT.md)
- [GitHub Repository](https://github.com/WyattAu/crawlkit)
