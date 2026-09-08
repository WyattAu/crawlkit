# WASI Promotion Evaluation

**Date:** 2026-09-08
**Scope:** Evaluate promoting `wasi-preview2` from an opt-in feature to the default feature set of `crawlkit-engine`.
**Verdict up front:** **Promote in `crawlkit-engine` defaults is acceptable; do NOT silently add it to the `crawlkit` CLI's `full` feature yet.** Details and conditions below.

---

## Current state

- `crawlkit-engine` already depends on `wasmtime` (v47) in its `full` default feature set for the core WASM plugin ABI. The WASI feature only *adds*:
  - `wasmtime/component-model` (re-enables component-model support in the already-present wasmtime crate)
  - `wasmtime-wasi`, `wasmtime-wasi-http` (both v47, first-party wasmtime project crates)
  - `http`, `http-body`, `http-body-util`, `hyper` — **all of which are already in the dependency graph via `reqwest` 0.12 in `full`**, so these are mostly shared, not new code
- The `crawlkit` CLI crate depends on `crawlkit-engine` with `default-features = false` and enables `full` explicitly. **`crawlkit`'s `full` does NOT forward `wasi-preview2`.** Consequence: adding `wasi-preview2` to `crawlkit-engine`'s default features would change nothing for CLI users — only for crates.io consumers of the `crawlkit-engine` library crate.
- The implementation (`crates/crawlkit-engine/src/plugin/wasi_preview2.rs`) is complete (Gates 3–5) and all 10 unit tests pass (`cargo test -p crawlkit-engine --features wasi-preview2 --lib plugin::wasi_preview2` → 10 passed, 0 failed).

## 1. Binary size implications (measured)

Built on this machine, workspace release profile (LTO, `codegen-units = 1`, `strip = true`, `panic = "abort"`), commit 287c19a4:

| Build | Size | Delta |
|---|---|---|
| `cargo build -p crawlkit --release` (no WASI) | 25,874,032 B (24.7 MiB) | — |
| `cargo build -p crawlkit --release --features crawlkit-engine/wasi-preview2` | 32,050,448 B (30.6 MiB) | **+6.2 MB (+23.9%)** |

Debug-profile reference points (not user-facing, but useful for attribution):

- `libwasmtime_wasi.rlib`: ~108 MB; `libwasmtime_wasi_http.rlib`: ~27 MB (debug rlibs)
- `crawlkit` debug binary: 519.9 MB → 686.6 MB with the feature (+32%)

**Assessment:** +6 MB on a ~25 MB binary is a real but moderate cost. It is small relative to wasmtime, which is *already* in the default binary. If WASI plugins are a headline feature of the marketplace story, this is a defensible price; for minimal deployments, `--no-default-features` remains available.

## 2. Compile time (measured)

- Engine library only, incremental debug: **1m17s** (this includes recompiling wasmtime with `component-model` plus `wasmtime-wasi`/`wasmtime-wasi-http` on a warm cache).
- Full CLI release build on the same warm cache:
  - without feature: **12m23s**
  - with feature: **19m27s**
  - ⇒ **≈ +7 minutes (~+60%)** on this machine, most of it in `wasmtime-wasi` compilation and the longer serial LTO link.

**Assessment:** Meaningful. CI matrices that build with the feature, and maintainer release builds, get noticeably slower. Cold-cache builds (fresh clones, CI runners without sccache) will be worse. This is the strongest argument *against* putting WASI into the CLI's `full` today without a deliberate decision. Mitigations: sccache/SwiftCache in CI, or splitting wasi into a separate `crawlkit-engine-wasi` facade crate later.

## 3. Security concerns

Reviewed `crates/crawlkit-engine/src/plugin/wasi_preview2.rs` (957 lines) and its linkage in `plugin/mod.rs`. The sandbox is deny-by-default and well-hardened:

- **No filesystem**: zero preopens; manifest requests for `filesystem`/`env_vars` are rejected outright (wasi_preview2.rs:341-347)
- **No sockets**: `wasi:sockets` linked with no addresses allowed
- **stdio**: guest output goes to bounded in-memory pipes (1 MiB/stream, write beyond capacity traps — tested at wasi_preview2.rs:657), never the host's stdio
- **HTTP outcalls**: dual-gated (manifest `permissions.network` AND `WasmConfig::allow_plugin_network`, wasi_preview2.rs:444-449), SSRF-validated per request via `is_public_http_url` (blocks loopback/private/link-local/metadata IPs — tested), timeouts clamped to 10 s, response bodies capped at 1 MiB, redirects never followed
- **Resource limits**: fuel + epoch interruption + wall-clock watchdog, identical to the core ABI
- **Trust chain**: hash/signature verification before the bytes reach the compiler (wasi_preview2.rs:368-377)
- **Fail-closed**: `define_unknown_imports_as_traps` makes unimplemented interfaces trap rather than silently no-op

Remaining concerns with promotion (none blocking):

1. **Attack surface growth in every binary.** Linking `wasmtime-wasi` + `wasmtime-wasi-http` + the hyper client into all default builds means more code subject to CVEs and more for fuzzing/audit to cover, even for users who never load a WASI plugin. The risk is supply-chain surface, not sandbox weakening.
2. **Misconfiguration risk.** The network capability is present in the binary; an embedder that flips `allow_plugin_network = true` without reviewing `permissions.network` in third-party manifests gets a real exfiltration path (SSRF-guarded, but public-internet exfil is possible by design). This is already true today for opt-in users; promotion widens the audience.
3. **Public enum surface.** `PluginInstance::Wasi` is `#[cfg(feature = "wasi-preview2")]`-gated (plugin/mod.rs:751). Promoting to default makes the variant unconditionally visible — exhaustively-matching downstream code compiled against a no-WASI build will see a new variant. This is a minor-version semver event and should be called out in the changelog.

## 4. Impact on existing users

| Audience | Effect of promoting to `crawlkit-engine` defaults |
|---|---|
| `crawlkit` CLI users (binary installs) | **None** — the CLI builds the engine with `default-features = false` and only enables `full`. Promotion alone ships them nothing (neither the benefits nor the costs). |
| Library consumers of `crawlkit-engine` with default features | New transitive deps (`wasmtime-wasi`, `wasmtime-wasi-http`, hyper stack via the feature), longer builds, larger artifacts. `PluginInstance::Wasi` becomes visible. No code breakage unless they exhaustively match `PluginInstance`. |
| `--no-default-features` / `wasm` users | None. |
| Current opt-in users of `wasi-preview2` | None (already have it). |

**Key insight:** promoting the *engine* feature is nearly a no-op operationally — but it also delivers nothing to CLI users. If the goal is "WASI plugins work out of the box with `crawlkit`", the actual change required is forwarding the feature in the `crawlkit` crate (`full = [..., "crawlkit-engine/wasi-preview2"]` or a dedicated pass-through feature), and that is the change that carries the +6 MB / +7 min cost for everyone.

## Recommendation

**Two-step promotion; do not skip step 1.**

1. **Now:** add `wasi-preview2` to `crawlkit-engine`'s `default` (or `full`) feature set. Cost is limited to library consumers, security posture is solid, all tests green, and it signals that WASI components are a supported configuration. Changelog must note the `PluginInstance::Wasi` variant and the new transitive dependencies.
2. **Deferred, gated on a product decision:** forward the feature into the `crawlkit` CLI. Accept the measured +6.2 MB release binary and ~+7 min release compile only if WASI plugins are being shipped as a headline capability in that release; otherwise ship it as an opt-in CLI feature (e.g. `crawlkit --features wasi-preview2`) and add sccache to CI first.

Explicitly not recommended: forwarding to the CLI `full` feature silently, without signing off on the size/compile-time budget — that is the one promotion path with a broad, unrequested user-facing cost.
