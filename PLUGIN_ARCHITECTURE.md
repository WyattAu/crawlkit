# crawlkit Plugin System Architecture

Status: Proposed. Requires stakeholder approval before execution.

---

## Overview

The plugin system enables third-party extensions to crawlkit's analyzer pipeline.
Plugins run in a sandboxed WASM environment for security, with an optional C ABI
mode for trusted internal plugins.

## Design Principles

1. **Security first** -- All third-party plugins run in WASM sandbox
2. **ABI stability** -- WASM target provides stable ABI across versions
3. **Cross-platform** -- Same .wasm file works on Linux, macOS, Windows
4. **Performance** -- Trusted plugins can use C ABI for near-native speed
5. **Extensibility** -- Plugin SDK provides full access to analyzer types

## Plugin Types

### WASM Plugins (Untrusted)

- Compiled to `wasm32-wasi` target
- Sandboxed: no file/network/memory access unless granted
- ABI: function pointers via WASM exports
- Use case: Third-party, marketplace, community plugins

### C ABI Plugins (Trusted)

- Compiled as shared library (.so/.dylib/.dll)
- Full system access (trusted code)
- ABI: C function pointers via `libloading`
- Use case: Internal plugins, performance-critical extensions

## Plugin Manifest

Every plugin must include a `crawlkit-plugin.toml` manifest:

```toml
[plugin]
name = "my-custom-analyzer"
version = "1.0.0"
api_version = "1.0"
author = "Plugin Author"
description = "Custom SEO analyzer for specific use case"
license = "Apache-2.0"
trust_level = "untrusted"  # "trusted" | "untrusted"

[plugin.entry]
# WASM plugins
wasm = "plugin.wasm"
# C ABI plugins (mutually exclusive with wasm)
# native = "libplugin.so"

[plugin.permissions]
# WASM permissions (ignored for C ABI)
network = false
filesystem = false
env_vars = []

[plugin.analyzer]
name = "my-custom"
category = "custom"
description = "My custom analyzer"
severity = "warning"
```

## Plugin Interface (WASM)

### Exports

```rust
/// Initialize the plugin. Called once on load.
/// Returns 0 on success, non-zero on error.
#[no_mangle]
pub extern "C" fn crawlkit_plugin_init() -> i32;

/// Get plugin metadata as JSON string.
/// Caller must free with crawlkit_plugin_free_string().
#[no_mangle]
pub extern "C" fn crawlkit_plugin_metadata() -> *mut u8;

/// Analyze HTML content. Returns JSON findings as string.
/// Caller must free with crawlkit_plugin_free_string().
#[no_mangle]
pub extern "C" fn crawlkit_plugin_analyze(
    html_ptr: *const u8,
    html_len: usize,
    url_ptr: *const u8,
    url_len: usize,
) -> *mut u8;

/// Free a string returned by plugin functions.
#[no_mangle]
pub extern "C" fn crawlkit_plugin_free_string(ptr: *mut u8);

/// Get API version this plugin was compiled against.
#[no_mangle]
pub extern "C" fn crawlkit_plugin_api_version() -> *mut u8;
```

### Memory Model

- Plugin allocates memory via WASM allocator
- Host reads plugin memory via WASM linear memory
- Host frees plugin memory by calling `crawlkit_plugin_free_string()`
- No shared memory between host and plugin (sandboxed)

## Plugin Interface (C ABI)

### Exports

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

// Entry points
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
7. Wire plugin's analyze() into AnalyzerRegistry
```

## CLI Commands

```bash
# List installed plugins
crawlkit plugin list

# Install a plugin
crawlkit plugin install ./my-plugin/

# Test a plugin with sample HTML
crawlkit plugin test ./my-plugin/ --html "<html>...</html>"

# Remove a plugin
crawlkit plugin remove my-custom-analyzer

# Search marketplace (future)
crawlkit plugin search "seo"
crawlkit plugin install marketplace://author/plugin-name
```

## API Endpoints

```
GET    /api/v1/plugins              # List installed plugins
POST   /api/v1/plugins              # Install plugin (upload .wasm or .zip)
GET    /api/v1/plugins/{name}       # Get plugin details
DELETE /api/v1/plugins/{name}       # Uninstall plugin
POST   /api/v1/plugins/{name}/test  # Test plugin with HTML
GET    /api/v1/plugins/{name}/stats # Plugin usage statistics
```

## Security Model

### WASM Sandbox

- No file system access (unless explicitly granted)
- No network access (unless explicitly granted)
- No environment variable access (unless explicitly granted)
- Memory limited to 64MB per plugin instance
- CPU limited to 100ms per analyze() call
- No access to host process memory

### C ABI Trust

- Only loaded from configured plugin directories
- Manifest must specify `trust_level = "trusted"`
- Plugin author must be in trusted authors list
- Plugin signature verification (future)

## Plugin SDK

The `crawlkit-plugin-sdk` crate provides:

```rust
// Plugin author imports
use crawlkit_plugin_sdk::{Analyzer, Finding, Severity, AnalysisContext};

// Implement analyzer
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

// Export for WASM
crawlkit_plugin_sdk::export_analyzer!(MyAnalyzer);
```

## Versioning

- Plugin API version: `1.0` (major.minor)
- Backward compatible: major version must match
- Forward compatible: minor version can differ
- crawlkit checks `api_version` compatibility on load

## Testing

- Unit tests for plugin loading
- Integration tests for WASM execution
- Fuzzing for malformed plugins
- Benchmark for plugin overhead measurement

## Implementation Phases

### Phase 1: Core Loading (8h)
- Add wasmtime dependency
- Implement WASM plugin loading in plugin.rs
- Add plugin directory scanning
- Validate manifests and API versions

### Phase 2: Plugin SDK (8h)
- Create `crawlkit-plugin-sdk` crate
- Define Analyzer trait, Finding types
- Add build script for WASM target
- Publish to crates.io

### Phase 3: CLI Commands (4h)
- Add `crawlkit plugin` subcommand
- Implement list, install, test, remove
- Add plugin directory configuration

### Phase 4: API Endpoints (4h)
- Add plugin endpoints to REST API
- Implement upload, list, test, remove
- Add plugin statistics

### Phase 5: Example Plugin (4h)
- Create example analyzer plugin
- Document plugin development process
- Add to documentation site

### Phase 6: Marketplace (16h)
- Design marketplace schema
- Implement plugin registry
- Add automated testing
- Create plugin submission process

**Total: ~44h for core, ~60h with marketplace**
