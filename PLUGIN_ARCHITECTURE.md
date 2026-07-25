# crawlkit Plugin System Architecture

*Status: Proposed. Requires stakeholder approval before execution.*
*Version: 2.0.0 | Last updated: 2026-07-25*

## Overview

Third-party extension system for crawlkit's analyzer pipeline. Two execution models: WASM sandbox for untrusted plugins, C ABI for trusted internal plugins.

## Design Principles

1. **Security first** -- All third-party plugins execute in WASM sandbox
2. **ABI stability** -- WASM target provides stable ABI across versions
3. **Cross-platform** -- Single `.wasm` artifact runs on Linux, macOS, Windows
4. **Performance** -- Trusted plugins use C ABI for near-native throughput
5. **Extensibility** -- Full access to analyzer type system via SDK

## Plugin Types

| Type | Target | Trust | Use Case |
|------|--------|-------|----------|
| WASM | `wasm32-wasi` | Untrusted | Third-party, marketplace, community |
| C ABI | `.so` / `.dylib` / `.dll` | Trusted | Internal, performance-critical |

## Plugin Manifest

Every plugin requires a `crawlkit-plugin.toml`:

```toml
[plugin]
name = "my-custom-analyzer"
version = "1.0.0"
api_version = "1.0"
author = "Plugin Author"
description = "Custom SEO analyzer"
license = "Apache-2.0"
trust_level = "untrusted"  # "trusted" | "untrusted"

[plugin.entry]
wasm = "plugin.wasm"
# native = "libplugin.so"  # mutually exclusive with wasm

[plugin.permissions]
network = false
filesystem = false
env_vars = []

[plugin.analyzer]
name = "my-custom"
category = "custom"
description = "My custom analyzer"
severity = "warning"
```

## WASM Plugin Interface

### Exports

```rust
#[no_mangle]
pub extern "C" fn crawlkit_plugin_init() -> i32;

#[no_mangle]
pub extern "C" fn crawlkit_plugin_metadata() -> *mut u8;

#[no_mangle]
pub extern "C" fn crawlkit_plugin_analyze(
    html_ptr: *const u8,
    html_len: usize,
    url_ptr: *const u8,
    url_len: usize,
) -> *mut u8;

#[no_mangle]
pub extern "C" fn crawlkit_plugin_free_string(ptr: *mut u8);

#[no_mangle]
pub extern "C" fn crawlkit_plugin_api_version() -> *mut u8;
```

### Memory Model

- Plugin allocates via WASM allocator (linear memory)
- Host reads plugin memory through WASM linear memory interface
- Host frees plugin memory by calling `crawlkit_plugin_free_string()`
- No shared memory between host and plugin instance

## C ABI Plugin Interface

### Types

```c
typedef struct {
    const char* name;
    const char* version;
    const char* api_version;
} crawlkit_plugin_info_t;

typedef struct {
    const char* category;
    const char* severity;
    const char* code;
    const char* title;
    const char* description;
    const char* recommendation;
} crawlkit_finding_t;

typedef struct {
    crawlkit_finding_t* findings;
    size_t count;
} crawlkit_analysis_result_t;
```

### Entry Points

```c
crawlkit_plugin_info_t* crawlkit_plugin_init(void);

crawlkit_analysis_result_t* crawlkit_plugin_analyze(
    const char* html, size_t html_len,
    const char* url, size_t url_len
);

void crawlkit_plugin_free_result(crawlkit_analysis_result_t* result);
```

## Plugin Loading Flow

```
1. Scan plugin directories for crawlkit-plugin.toml
2. Parse manifest, validate api_version compatibility
3. Load WASM module (wasmtime) or shared library (libloading)
4. Call crawlkit_plugin_init() -- check return code
5. Call crawlkit_plugin_metadata() -- validate metadata
6. Register plugin in PluginRegistry
7. Wire plugin analyze() into AnalyzerRegistry
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `crawlkit plugin list` | List installed plugins |
| `crawlkit plugin install <path>` | Install from directory or archive |
| `crawlkit plugin test <path> --html "<html>"` | Test with sample HTML |
| `crawlkit plugin remove <name>` | Uninstall plugin |
| `crawlkit plugin search <query>` | Search marketplace (future) |

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/plugins` | List installed plugins |
| POST | `/api/v1/plugins` | Install (upload .wasm or .zip) |
| GET | `/api/v1/plugins/{name}` | Get plugin details |
| DELETE | `/api/v1/plugins/{name}` | Uninstall plugin |
| POST | `/api/v1/plugins/{name}/test` | Test with HTML |
| GET | `/api/v1/plugins/{name}/stats` | Usage statistics |

## Security Model

### WASM Sandbox Constraints

| Resource | Default | Grant |
|----------|---------|-------|
| File system | Denied | Explicit grant |
| Network | Denied | Explicit grant |
| Environment variables | Denied | Explicit grant |
| Memory limit | 64 MB per instance | Configurable |
| CPU limit | 100 ms per `analyze()` call | Configurable |
| Host memory | No access | Enforced by WASM |

### C ABI Trust Requirements

- Loaded only from configured plugin directories
- Manifest must declare `trust_level = "trusted"`
- Plugin author must be in trusted authors list
- Signature verification planned

## Plugin SDK

The `crawlkit-plugin-sdk` crate provides the `Analyzer` trait:

```rust
use crawlkit_plugin_sdk::{Analyzer, Finding, Severity, AnalysisContext};

pub struct MyAnalyzer;

impl Analyzer for MyAnalyzer {
    fn name(&self) -> &str { "my-custom" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        if ctx.html.contains("something") {
            findings.push(Finding {
                category: "custom".into(),
                severity: Severity::Warning,
                code: "CUSTOM001".into(),
                title: "Something detected".into(),
                description: "The page contains something".into(),
                recommendation: "Remove something".into(),
            });
        }
        findings
    }
}

crawlkit_plugin_sdk::export_analyzer!(MyAnalyzer);
```

## API Versioning

| Rule | Constraint |
|------|-----------|
| Major version | Must match between plugin and host |
| Minor version | May differ (forward compatible) |
| Compatibility check | Performed at load time |

Format: `major.minor` (e.g., `1.0`).

## Testing

- Unit tests for plugin loading and manifest parsing
- Integration tests for WASM execution via wasmtime
- Fuzzing for malformed plugin binaries and manifests
- Benchmark for per-plugin overhead measurement

## Implementation Phases

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1. Core Loading | wasmtime integration, directory scanning, manifest validation | 8h |
| 2. Plugin SDK | `crawlkit-plugin-sdk` crate, Analyzer trait, WASM build script | 8h |
| 3. CLI Commands | `crawlkit plugin` subcommand (list, install, test, remove) | 4h |
| 4. API Endpoints | Plugin REST endpoints (upload, list, test, remove, stats) | 4h |
| 5. Example Plugin | Reference analyzer plugin, development documentation | 4h |
| 6. Marketplace | Plugin registry, automated testing, submission process | 16h |
| **Total (core)** | | **28h** |
| **Total (with marketplace)** | | **44h** |
