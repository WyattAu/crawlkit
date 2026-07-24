# ADR-001: Plugin System Architecture

## Status
Accepted

## Context
crawlkit needs a plugin system to allow third-party extensions to the analyzer pipeline.

## Decision
Use WASM (WebAssembly) for sandboxed plugin execution via wasmtime.

## Consequences
### Positive
- Sandboxed execution prevents arbitrary code execution
- Cross-platform compatibility (same .wasm file works everywhere)
- ABI stability (WASM is a stable target)
- Memory safety (sandbox prevents buffer overflows)

### Negative
- 2-10x slower than native execution
- Requires wasmtime runtime dependency (~15MB)
- Plugin authors need WASM compilation toolchain

### Risks
- WASM performance overhead for CPU-bound analysis
- Plugin ABI changes may break existing plugins

## Alternatives Considered
- **C ABI (libloading):** Maximum performance but no sandboxing
- **Hybrid (C ABI + WASM):** Best of both worlds but more complex

## References
- wasmtime documentation: https://wasmtime.dev/
- WASI specification: https://wasi.dev/
