# ADR-011: WASI Preview 2 Evaluation (B5)

## Status

Proposed (2026-08-23) — evaluation in progress; **gate 2 PASSED**
(wasmtime 47.0.4 `component-model` feature compiles clean against
MSRV 1.94 — 167-crate dep tree, zero errors; spike in
/tmp, recorded 2026-08-23). Remaining gates: 1, 3, 4, 5.

## Context

crawlkit's plugin sandbox (ADR-003/006) uses a bespoke host ABI:
`crawlkit_plugin_init/alloc/analyze/free` plus two custom host functions
(`crawlkit_host.fetch`, `crawlkit_host.get_context`). WASI Preview 2
(component model) is the standardizing path for exactly this shape of
host/guest split — wasmtime (already our runtime) is its reference
implementation, and Preview 3 work continues toward WebAssembly
standardization.

Motivations to evaluate:

1. **Standard capability grants**: WASI defines permissioned interfaces
   (`wasi:http/outgoing-handler`, `wasi:io`, clocks, random) with
   per-component allow lists — a standards-based replacement for our
   hand-rolled `allow_plugin_network` + SSRF-proxy design.
2. **Guest language breadth**: any language targeting WASI components
   (Rust, Go via TinyGo, JS/Python via componentize) instead of requiring
   our SDK's raw ABI.
3. **WIT-typed interface**: the analyzer contract could become a
   `.wit` world — typed, versioned, tool-checked — replacing the JSON
   string protocol on both sides.

## Decision

Evaluate for one release cycle (target: decision by v4.6) along five
axes, with hard evaluation criteria per axis:

| Axis | Adopt-if | Measurement plan |
|---|---|---|
| Guest overhead | Component-analyze within 2× current raw-ABI cost (≤2 ms/page marginal) | Bench: title-length as both raw-ABI plugin and WASI component against the same corpus |
| MSRV/toolchain cost | No MSRV bump; wasmtime 47's component-model support compiles on 1.94 | Spike: build a minimal wit-component against the pinned wasmtime |
| Capability parity | Network access expressible as `wasi:http` grant with per-host allow lists at least as strict as the current SSRF proxy | Port the B2 test suite semantics (blocklist, 10s timeout, 1 MiB cap) to a component |
| Migration coexistence | v1 ABI plugins load alongside components for ≥2 majors (both paths in `PluginRegistry`) | Design only — loader dispatch on manifest field `abi = "wasi-p2"` |
| Determinism | Component analysis output byte-identical across runs (per ADR-007 contract) | Determinism test port: same fixtures → identical findings JSON |

**Explicit non-goal for the evaluation**: replacing the first-party
plugin set. If adopted, WASI becomes an *additional* ABI; the existing
index, trust chain (ADR-006), and installed base keep working.

## Consequences

**If adopted:**
- `crawlkit_host.fetch` eventually deprecates in favor of `wasi:http`
  grants; the SSRF policy moves to the bind-time allow list (a security
  improvement — enforcement at grant time, not call time).
- The SDK gains a `wit-bindgen` path; plugin authors choose raw ABI
  (minimal, zero-dep) or WASI (standard, multi-language).
- New dependency surface: `wasmtime-component-macro`,
  `wit-component` tooling in the build; tracked via deny.toml.

**If rejected:** record the measured numbers here; revisit when
wasmtime's component story reaches a stable Preview 3 or guests
materialize that the raw ABI cannot support.

**Either way:** the evaluation corpus (dual-build title-length, ported
B2 tests) lands as tests, not throwaway spikes.

## Alternatives Considered

1. **Adopt immediately** — rejected: unmeasured performance/capability
   claims violate the evidence discipline this project runs on (the
   dogfood protocol exists precisely because unvalidated behavior is
   where bugs live).
2. **Never adopt** — rejected as a standing decision: the raw ABI is
   ours alone; a standards path that passes the five gates would reduce
   long-term maintenance and grow the guest ecosystem for free.
3. **hybrid plugin-SDK emits both ABIs** — deferred to the evaluation
   result; if component overhead is acceptable the SDK can emit dual
   builds transparently.

## References

- ADR-003 (sandbox), ADR-006 (trust chain), ADR-007 (determinism),
  ADR-009 (dogfood evidence discipline)
- B2 network capability: `crawlkit_host.fetch` + SSRF policy in
  `plugin.rs`
- wasmtime component-model docs (v47 tree) — the pinned runtime
