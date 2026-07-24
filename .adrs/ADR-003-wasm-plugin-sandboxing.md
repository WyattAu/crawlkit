# ADR-003: WASM Plugin Sandboxing

## Status
Accepted

## Context
Third-party plugins must be isolated from the host system for security.

## Decision
Use WASM sandboxing with configurable permissions.

## Consequences
### Positive
- No file system access unless explicitly granted
- No network access unless explicitly granted
- Memory limited to 64MB per plugin instance
- CPU limited to 100ms per analyze() call

### Negative
- Performance overhead for sandboxed execution
- More complex plugin development experience

### Risks
- Sandbox escapes (mitigated by wasmtime's security model)
- Permission escalation (mitigated by strict permission checks)

## References
- wasmtime sandboxing: https://wasmtime.dev/docs/security/
- WASI capability-based security: https://wasi.dev/
