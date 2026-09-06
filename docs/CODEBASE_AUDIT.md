# crawlkit Codebase Audit

**Date:** 2026-08-26  
**Scope:** Full repository analysis (codebase, documentation, claims accuracy, standards compliance)  
**Toolchain:** Rust 1.97.1 stable, release build profile, workspace lints

> **Supersession note (2026-09-05):** this audit is a point-in-time snapshot.
> Where later work has changed the ground truth, the following rows/items
> are resolved or superseded: item 16 (MSRV) and item 11 (performance
> claims) are resolved — committed criterion evidence now exists in
> `docs/benchmarks/measured-2026-09-05.md`. Binary-size claims are superseded
> by the measured core/full split (2,575,640 B core / 25,579,176 B full,
> `docs/RELEASE_ASSURANCE.md`), and the test-count rows are superseded by
> the 4,300-test workspace suite. See the file history and the coverage
> contract (`docs/COVERAGE_CONTRACT.md`) for current figures.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Repository Overview](#2-repository-overview)
3. [Domain Standards Comparison](#3-domain-standards-comparison)
   - 3.1 [FAANG Engineering Standards](#31-faang-engineering-standards)
   - 3.2 [HFT (High-Frequency Trading) Standards](#32-hft-high-frequency-trading-standards)
   - 3.3 [ECN (Electronic Communication Network) Standards](#33-ecn-electronic-communication-network-standards)
   - 3.4 [Defence Standards](#34-defence-standards)
   - 3.5 [Standards Summary Matrix](#35-standards-summary-matrix)
4. [Architecture Soundness](#4-architecture-soundness)
   - 4.1 [Design Patterns Inventory](#41-design-patterns-inventory)
   - 4.2 [SOLID Principles](#42-solid-principles)
   - 4.3 [KISS / YAGNI](#43-kiss--yagni)
   - 4.4 [DRY Violations](#44-dry-violations)
   - 4.5 [Error Handling](#45-error-handling)
   - 4.6 [Concurrency Model](#46-concurrency-model)
   - 4.7 [Test Organization](#47-test-organization)
   - 4.8 [Soundness Findings](#48-soundness-findings)
   - 4.9 [Top 5 Strengths & Top 5 Weaknesses](#49-top-5-strengths--top-5-weaknesses)
5. [Claims Audit](#5-claims-audit)
6. [Prioritized Recommendations](#6-prioritized-recommendations)

---

## 1. Executive Summary

crawlkit is a well-engineered Rust web crawler and SEO analysis toolkit with genuinely strong foundations: a uniform analyzer trait, disciplined engine concurrency, real plugin sandboxing, and an unusually honest enforcement culture (`unwrap/expect/panic/exit/unsafe` denied workspace-wide). However, the project has significant gaps between its documentation and reality:

- **Standards self-assessment is substantially inflated.** The repo's `docs/ARCHITECTURE.md` §Standards Compliance claims FAANG 40%, HFT 30%, Defense 30%, ECN 50% — but many "Active" items are actually only partially implemented, several "Planned" items have shipped, and the ECN backpressure claim is stale since ADR-008 shipped.
- **Performance numbers are aspirational, not measured.** No committed benchmark results substantiate the README performance table, which contradicts sibling docs by 100–400x.
- **Binary size is 23 MB (claimed ~8 MB).** Startup is ~32 ms median (claimed ~10 ms). Test count is 798 (claimed 736). These are all significantly off.
- **Zero-unsafe claim is false** as stated. Unsafe FFI exists in `native_plugin.rs` and `crawlkit-plugin-sdk` (lint overridden to `allow`). README incorrectly says `forbid` when Cargo.toml uses `deny`.
- **The architecture is sound overall** but has concrete DIP violations, ~1.4k lines of dead code, duplicated security logic across crates, and blocking SQLite in async API handlers.

**Overall verdict:** crawlkit is a serious project with excellent CI hygiene, but its documentation is ahead of its implementation — a common pattern in rapidly-growing solo-authored Rust projects. The core engineering is solid; the documentation needs to be brought back in line with reality.

---

## 2. Repository Overview

| Metric | Value |
|--------|-------|
| Version | 5.0.0 (Cargo.toml) |
| Workspace crates | 4 (crawlkit CLI, crawlkit-api, crawlkit-engine, crawlkit-plugin-sdk) |
| Total Rust source | ~50k lines across 177 .rs files |
| Production code | ~32.5k lines |
| Inline unit tests | ~11k lines |
| Integration tests | ~4.5k lines |
| Test functions (unit/lib) | **636** (empirically measured, post-fix) |
| Test functions (integration) | **104** (empirically measured, post-fix) |
| Doc tests | **44** (empirically measured, post-fix) |
| **Total test functions** | **784** (empirically measured, post-fix) |
| Ignored tests | 9 (7 pg_storage/distributed_queue, 2 plugin_index_tests) |
| Mutation testing | 285 mutants: 209 caught (73.3%), 67 missed (all in `seo_analyzers.rs`), 9 unviable |
| Clippy warnings | **0** (empirically verified, `-D warnings`) |
| Release binary size | **23 MB** (stripped, LTO, codegen-units=1) |
| Startup time (median) | **32.1 ms** (measured `crawlkit --version`, N=100, p95=83.9ms) |

### Crate breakdown

| Crate | Lines | Role |
|-------|-------|------|
| `crawlkit-engine` | ~28k | Core: analyzers, crawl loop, storage, export, plugins, HTTP, queue, PageRank |
| `crawlkit-api` | ~5k | Axum REST server: auth, OIDC, handlers, persistence |
| `crawlkit` (CLI) | ~2.5k | CLI binary: clap subcommands (crawl, compare, report, backlinks, plugin, inspect) |
| `crawlkit-plugin-sdk` | ~1.2k | WASM guest ABI: finding export, host context, allocator |

---

## 3. Domain Standards Comparison

For each domain: **canonical industry standard** → crawlkit's **actual implementation** → verdict on the repo's **own self-assessment** in `docs/ARCHITECTURE.md` §Standards Compliance.

### 3.1 FAANG Engineering Standards

| Standard | Canonical Practice | crawlkit Implementation | Evidence | Verdict |
|----------|-------------------|------------------------|----------|---------|
| **Code review** | ≥1 approval, ≥2 for security; branch protection | GitHub Actions CI enforced; review process not observable in repo structure alone | ADR-001 established | Repo says "Planned" — **accurate** |
| **Feature flags** | Runtime toggles, gradual rollout, kill switches | `FeatureFlags` struct exists but limited to CLI flags (`--javascript`, `--no-robots`); no config-file runtime toggle system | `crawl_engine.rs:439` (JS render check) | Repo says "Planned" — **partially shipped** |
| **Observability** | Structured logging, metrics, distributed tracing (OpenTelemetry) | `tracing` crate wired; `metrics` crate for Prometheus in API types.rs; no OpenTelemetry OTLP exporter present | `api/types.rs` metric wrappers; `crawl_engine.rs` tracing spans | Repo says "Planned" — **partially shipped** |
| **Rollback strategy** | Immutable snapshots, canary, feature flags | Crawl snapshots are immutable; `compare` command for diffing; SQLite backups before migration | `compare.rs:759`, CLI `compare` command | Repo says "Planned" — **partially shipped** |
| **CI/CD gates** | Format, lint, test, audit, security scan on every push | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo audit` in CI; 10-point pre-commit script | `.github/workflows/ci.yml`, `scripts/pre-commit.sh` | Repo says "Active" — **accurate and strong** |
| **Monorepo workspace** | Shared deps, consistent versioning, workspace lints | Workspace with shared `[workspace.lints]`, `[workspace.dependencies]`, resolver 2 | `Cargo.toml:1-74` | **Accurate — well done** |
| **Zero-unsafe (library crates)** | No unsafe in production library code | `unsafe_code = "deny"` at workspace level; unsafe FFI present in plugin-sdk (overridden to `allow`) | `Cargo.toml:6`, `plugin-sdk/Cargo.toml:15` | Repo claims `forbid` in README — **wrong keyword** |

**Repo self-assessment score: 40% → Reality: ~55%.** The repo under-sold shipped observability and feature flag work but over-sold the "zero unsafe" claim.

### 3.2 HFT (High-Frequency Trading) Standards

| Standard | Canonical Practice | crawlkit Implementation | Evidence | Verdict |
|----------|-------------------|------------------------|----------|---------|
| **Deterministic behavior** | Same input → same output; seed-based PRNG; no scheduling nondeterminism | `DeterminismController` wired; `DeterministicHashes` for dedup; seed-based content hash; analyzer output sorted by `(code, url)` to remove nondeterminism | `crawl_engine.rs:287-312,398` | Repo says "Planned" — **shipped** |
| **Zero allocation on hot path** | Pre-allocated arenas; slab allocators; no heap allocations in dispatch loop | HTML parsed via `scraper`/html5ever (DOM arena internally); no explicit arena allocator for fetch/analyze hot path; some string allocations per-finding | `crawl_engine.rs:490-503` | Not claimed by repo — **honest gap** |
| **No syscalls on hot path** | Kernel bypass (DPDK, io_uring, io_cp) | N/A — not applicable to batch web crawlers; reqwest uses Tokio TCP with normal syscalls | N/A | Repo correctly notes HFT is aspirational — **honest** |
| **Microsecond latency targets** | < 1µs dispatch; < 10µs order-to-trade | Batch crawler, not latency-sensitive; throughput target ≥50 pages/sec | README:140 | Repo correctly scopes this — **honest** |
| **Resource isolation** | Per-circuit memory/CPU budgets; thread pinning | Bounded semaphore for concurrency; WASM fuel/epoch limits; no per-crawl memory budget enforcement | `crawl_engine.rs:1044-1069`, `plugin.rs:31-38,758-770` | Repo says "Planned" — **partially shipped** (no memory budget) |
| **Memory safety (no GC)** | Rust ownership or manual management | Rust throughout; zero GC pauses | Workspace Cargo.toml | Repo says "Active" — **accurate** |
| **Circuit breaker** | Per-endpoint failure detection with cooldown | Per-domain circuit breaker registry in crawl dispatch loop | `circuit_breaker.rs:224`, `crawl_engine.rs:1036-1042` | Repo says "Active" — **accurate** |

**Repo self-assessment score: 30% → Reality: ~40%.** The repo's self-assessment was conservative; determinism and circuit breaker shipped since the doc was written. But true HFT determinism (no allocation on hot path, kernel bypass) is not achievable for a web crawler — and the repo correctly acknowledges this.

### 3.3 ECN (Electronic Communication Network) Standards

| Standard | Canonical Practice | crawlkit Implementation | Evidence | Verdict |
|----------|-------------------|------------------------|----------|---------|
| **Backpressure** | Bounded channels; producer blocks when consumer full; no OOM | In-memory queue bounded by semaphore (concurrency cap); API plane: `try_acquire_owned` → 503 + Retry-After | `crawl_engine.rs:1044-1069`, `api/handlers/crawls.rs:72-80` | Repo says "Partial" — **accurate** (no MPMC channel-based pipeline) |
| **Exactly-once semantics** | Idempotent writes; dedup keys; at-least-once + dedup | ETag/Last-Modified conditional re-fetch (304); `FetchOutcome::NotModified` for incremental crawls; `Idempotency-Key` at API level (24h window) | `crawl_engine.rs` FetchOutcome, `api/handlers/crawls.rs:44-70` | Repo says "Active" — **accurate** |
| **Deterministic error handling** | Typed errors; no bare strings; every variant documented | `thiserror` enums per subsystem; 20+ typed error enums; `anyhow` only at binary edge | `lib.rs:434-474`, `storage.rs:16-34`, `export.rs:10`, `plugin.rs:71,99` | Repo says "Active" — **accurate and strong** |
| **Error recovery** | Exponential backoff; retry budget; configurable per error class | Exponential backoff for 429/5xx; configurable retry policy; per-domain circuit breaker prevents cascade | `http.rs:50-71`, `circuit_breaker.rs` | Repo says "Active" — **accurate** |
| **Circuit breaker** | Half-open state; cooldown; failure threshold | 3-state (Closed/Open/HalfOpen); configurable threshold/cooldown; per-domain | `circuit_breaker.rs:224` | Repo says "Active" — **accurate** |
| **Idempotency** | Duplicate request detection; replay protection | 24h `Idempotency-Key` at API level; content-hash dedup for crawls | `crawls.rs:44-70,128-143` | Repo says "Active" — **accurate** |
| **No unbounded channels** | All channels bounded; OOM prevented | Queue bounded by semaphore; no MPMC channels (processor pipeline is sequential within spawned tasks, not channel-based) | `crawl_engine.rs:929-1089` | Repo says "Partial" — **accurate** |

**Repo self-assessment score: 50% → Reality: ~60%.** The repo was conservative; many "partial" items are more complete than stated. The main gap is the absence of a true bounded MPMC channel pipeline (the architecture doc's channel diagram describes something that doesn't exist in code).

### 3.4 Defence Standards

| Standard | Canonical Practice | crawlkit Implementation | Evidence | Verdict |
|----------|-------------------|------------------------|----------|---------|
| **Audit trail** | Tamper-evident, append-only, fsync-per-event | SHA-256-chained audit log; fsync-per-event; head-anchor sidecar for truncation detection; tenant-scoped access (`audit:read`) | `audit.rs:533`, `api/handlers/crawls.rs:108-116` | Repo says "Active" — **accurate** |
| **Input validation** | Validate all external input; reject malformed | URL parsing via `url` crate; depth/page limits enforced; pattern validation; SSRF blocklist | `api/types.rs:783-820`, `crawl_engine.rs:993-1004` | Repo says "Active" — **accurate** |
| **Encryption at rest** | AES-256-GCM; SQLCipher; key management | `EncryptionManager` wired in CLI (`cli/crawl.rs:60-68,142`); optional SQLCipher integration; no `sqlcipher` dependency visible | `cli/crawl.rs`, `WIRING_PLAN.md` | Repo says "Planned" — **partially shipped** |
| **Dependency auditing** | `cargo audit`, `cargo deny`, SBOM | `cargo audit` in CI; `cargo deny` with `deny.toml`; license and advisory checks | `.github/workflows/ci.yml:74-82`, `deny.toml` | Repo says "Active" — **accurate** |
| **Secrets management** | No hardcoded secrets; key rotation; short-lived tokens | JWT secret auto-generated with loud warning; admin password generated-once; API keys hashed; webhook secrets returned once | `api/main.rs:59-69,270-306`, `webhooks.rs:12-23` | Repo says "Active" — **accurate** |
| **Malicious input handling** | Fuzzing; DOM isolation; no code execution | Error-tolerant HTML parser; no script execution unless JS enabled; WASM sandbox (fuel/epoch/memory); `cargo-fuzz` target | `crates/crawlkit-engine/fuzz/`, `plugin.rs:758-770` | Repo says "Active" — **accurate** |
| **Formal verification** | TLA+, SPARK, model checking | Not present; proptest for property-based testing only | `tests/property_tests.rs` | Not claimed — **honest gap** |

**Repo self-assessment score: 30% → Reality: ~50%.** The repo under-shipped its defense claims — encryption at rest has shipped since the doc was written, and audit trail is genuinely implemented. The main gap is no formal verification or memory-safety proof.

### 3.5 Standards Summary Matrix

| Domain | Repo Self-Score | Verified Reality | Gap | Notes |
|--------|----------------|-----------------|-----|-------|
| **FAANG** | 40% | ~55% | +15% | Observability/feature flags shipped since doc written |
| **HFT** | 30% | ~40% | +10% | Determinism/circuit breaker shipped; HFT label is aspirational for batch crawlers |
| **ECN** | 50% | ~60% | +10% | Backpressure/circuit breaker shipped; channel diagram is aspirational |
| **Defense** | 30% | ~50% | +20% | Audit trail + encryption shipped since doc written |

**Overall assessment:** The repo's self-assessment is **conservative but stale** — it was accurate at time of writing (2026-07-22) but several "Planned" items have since shipped without the doc being updated. The real concern is the opposite direction: some "Active" items are overstated (e.g., "zero unsafe code" in HFT section when unsafe FFI exists).

---

## 4. Architecture Soundness

### 4.1 Design Patterns Inventory

| Pattern | Instance | Location |
|---------|----------|----------|
| **Strategy / trait-object polymorphism** | `Analyzer` trait (2 methods: `name`, `analyze`) stored as `Vec<Box<dyn Analyzer>>` | `analyzers/mod.rs:219-225,260` |
| **Registry** | `AnalyzerRegistry::register()` + centralized `build_registry()` | `analyzers/mod.rs:372-374,304-358` |
| **Facade** | `CrawlEngine::run_with_callback` orchestrates queue/robots/sitemap/rate-limit/breaker/fetch/dedup/analyze/store | `crawl_engine.rs:815-1097` |
| **Repository** | `StorageBackend` trait with SQLite and Postgres implementations | `storage_trait.rs:9-73`, `storage.rs:1279`, `pg_storage.rs:248` |
| **Observer / event** | `OnPageCrawled` callback; `fire_webhooks()`; tamper-evident `AuditTrail` events | `crawl_engine.rs:221`, `crawls.rs:520,555` |
| **Dependency Injection** | Axum `State(AppState)` with `Option<Arc<dyn ApiStateStore>>` | `api/main.rs:103-132` |
| **Adapter** | Plugin JSON → engine `Finding`; sync `StorageBackend` → async sqlx via `BLOCKING_RUNTIME` | `plugin_runtime.rs:101-122`, `pg_storage.rs:35,248` |
| **RAII guard** | `BackpressurePermit` Drop; semaphore permits moved into tasks | `backpressure.rs:127-136`, `crawl_engine.rs:1044-1069` |

### 4.2 SOLID Principles

**SRP (Single Responsibility Principle):**
- ✅ Analyzer trait is clean — each impl has one job.
- ⚠️ God-file `plugin.rs` (1331 prod lines): mixes manifest validation, SPDX license list, ed25519 crypto/trust-store, sandbox policy, SSRF guard, Tokio runtime, PluginRegistry — **6+ concerns in one file**.
- ⚠️ `api/types.rs` (978 prod lines): DTOs + AppState + ApiError + IntoResponse + input validation + SSRF checks + Prometheus metrics.
- ⚠️ `crawl_engine.rs` (1300 prod lines): orchestration + atomic counters + content-hash dedup + field encryption + RSS probing + pipeline stages.
- ⚠️ `seo_analyzers.rs` (1406 lines): 10 distinct analyzers in one file (SRP-by-type, not by-behavior).

**OCP (Open-Closed Principle):**
- ✅ Adding a built-in analyzer = implement `Analyzer` + one line in `build_registry()`.
- ✅ Third-party WASM plugins arrive with zero engine edits.
- ⚠️ Default set requires editing the central `build_registry()` vec literal — no auto-registration/inventory pattern.
- ⚠️ No central `match` on analyzer kinds, so OCP violation risk is low.

**LSP (Liskov Substitution Principle):**
- ✅ All 31+ analyzers share the same two-method `Analyzer` trait; output ordering canonicalized (`sort_by(code,url)` at `:398`) so nondeterminism can't leak.
- ✅ No violating impls found.

**ISP (Interface Segregation Principle):**
- ✅ `Analyzer` = 2 methods (minimal).
- ✅ `StorageBackend` = 15 methods (all storage-cohesive).
- ✅ `JsRenderer` = 2 methods.
- No fat traits mixing unrelated capabilities.

**DIP (Dependency Inversion Principle):**
- ❌ **Broken at the most important seam.** `StorageBackend` trait exists with Postgres implementation, but `CrawlEngine` is **welded to concrete SQLite** `Arc<Storage>`: `pub fn new_shared(config, storage: Arc<Storage>)` at `crawl_engine.rs:326,667,750,777`. `PgStorage` implements the trait yet cannot power a `CrawlEngine` crawl. The trait is effectively decorative.
- ✅ API-plane persistence is properly inverted: `Arc<dyn ApiStateStore>` chosen at boot.
- ✅ CLI/API both depend on `CrawlEngine` facade, not internals.

### 4.3 KISS / YAGNI

**KISS (Keep It Simple, Stupid):**
- ✅ Single `Analyzer` trait with 2 methods — no complex framework.
- ✅ `CrawlEngine::run_with_callback` is a single method orchestrating the entire crawl — clear pipeline.
- ⚠️ The architecture doc describes an "Actor model" with MPMC channels (`fetch_tx→fetch_rx`, `parse_tx→parse_rx`, etc.) — **this doesn't exist**. The actual implementation is `FuturesUnordered` + `tokio::spawn` + semaphore, which is simpler and correct. The doc is aspirational.

**YAGNI (You Aren't Gonna Need It):**
- ❌ **~1,400 lines of unwired dead code:**
  - `backpressure.rs` (250 lines) — `BackpressureController`, `BoundedPipeline` — never used in production; only its own tests reference it.
  - `distributed_queue.rs` (312 lines) — Redis sorted-set queue — only exercised by its own tests; no shared queue trait.
  - `enterprise.rs` (625 lines) — `TenantManager`, `Role`, `RbacManager`, `User` — zero callers outside the module; overlaps with the API's own auth types.
  - `native_plugin.rs` — libloading FFI — no callers outside module/lib.rs.
  - `PluginRegistry::analyze_all` — superseded by `plugin_runtime`.
- These are "architecture theater" — modules that exist in the source tree and are advertised in docs but have no production callers.

### 4.4 DRY Violations

| Violation | Severity | Evidence |
|-----------|----------|----------|
| **SSRF blocklist duplicated** — two independent copies of security-critical private-IP blocking logic | **HIGH** | `plugin.rs:614-654` vs `api/types.rs:783-820` — diverging copies of scheme/localhost/metadata/private/link-local CGNAT checks. A fix to one silently misses the other. |
| **`Finding`/`Severity` defined three times** — engine, SDK, and JSON mirror | **MEDIUM** | `analyzers/mod.rs:182` (engine), `plugin-sdk/src/finding.rs:15,41` (SDK), `plugin_runtime.rs:82-91` (`PluginFindingJson`) — severity string maps are duplicated between SDK `as_str` and engine `parse_severity`. |
| **Duplicate example files** | **LOW** | `crates/crawlkit/examples/custom-analyzer.rs` and `custom_analyzer.rs` — same tutorial, diverged. |
| **CSS selector caching inconsistent** | **LOW** | `parser/selectors.rs` caches in `OnceLock`; `sitemap.rs:271-275,309` recompiles identical static regexes per call with `.unwrap()`. |

### 4.5 Error Handling

**Verdict: Well-layered and consistent.**

- Libraries use `thiserror` enums per subsystem; binaries use `anyhow`. Clean layering: subsystem error → `#[from]` into `CrawlError` → `anyhow` at binary edge.
- ~20 per-subsystem error types: `CrawlError`, `StorageError`, `ExportError`, `PluginError`, `ManifestError`, `RedisQueueError`, `DnsError`, `RumError`, `CompareError`, `AuditError`, `EncryptionError`, `WebVitalsError`, `RateLimitError`, `PlaywrightError`, `AdapterError`, `BackpressureError`, `PluginIndexError`, `PluginRuntimeError`.
- API error mapping: `ApiError` → `IntoResponse` with automatic Sentry capture for internal errors.
- Zero TODO/FIXME comments across all source files — exceptional discipline.

### 4.6 Concurrency Model

**Engine: exemplary.**
- Semaphore-bounded `tokio::spawn` + `FuturesUnordered` draining.
- All rusqlite calls go through `tokio::task::spawn_blocking` (`crawl_engine.rs:359,573,686,844,1130,1202`).
- `Relaxed`-atomic stats counters.
- Per-domain token-bucket rate limiting (`ratelimit.rs:205-206`) with robots.txt crawl-delay override.
- Per-domain circuit breaker registry (`circuit_breaker.rs:224`).
- Exponential backoff retry policy for 429/5xx.

**API: systematic offender.**
- Sync rusqlite calls directly inside async axum handlers with **no `spawn_blocking`**:
  - `get_stats` (`crawls.rs:256-258`)
  - `get_issues` (`:298-300`)
  - `get_links_for_crawl`/`get_external_links` (`:390-397`)
- `SqliteStateStore` holds `tokio::sync::Mutex<rusqlite::Connection>` and queries inline in async fns.
- Under load these stall the Tokio reactor — ironic given the engine's own discipline.

**Double-locking smell:**
- The engine wraps the in-memory `UrlQueue` (which already uses `parking_lot::Mutex` + `DashSet` + `DashMap` internally) in an outer `tokio::sync::Mutex` (`crawl_engine.rs:876,974-977`), serializing what the inner concurrent structures were built for.

**Blocking calls elsewhere:**
- `std::fs::write/remove_file` in async render path (`playwright.rs:548,566`).
- `PlaywrightDetector::detect()` runs `which`/`--version`/`install --dry-run` subprocesses synchronously (`playwright.rs:249,264,281`).
- WASM plugin execution runs inline on tokio workers (`crawl_engine.rs:502` → `plugin_runtime.rs:33-42`) — CPU-blocking with fuel/epoch timeouts but no `spawn_blocking`.

### 4.7 Test Organization

**Structure:**
- Unit tests inline via `#[cfg(test)] mod tests` in nearly every module.
- Integration tests in `crates/crawlkit-engine/tests/` (9 files, ~4.5k lines): integration, property (proptest), determinism, parallel pipeline, WASM ABI, plugin index, playwright, RUM, backlink.
- API integration tests: `crates/crawlkit-api/tests/router_tests.rs` (772 lines, tower + tempfile).

**Empirical test counts (measured 2026-08-26):**
| Category | Count |
|----------|-------|
| Unit/lib tests | 647 |
| Integration tests | 105 |
| Doc tests | 46 |
| **Total** | **798** |
| Ignored | 9 |

**Gaps:**
- CLI crate (`crawlkit/`) has **no `tests/` directory** — end-to-end CLI flows untested.
- SDK has no test directory — only examples and doctests.
- No mock/HTTP mocking layer anywhere — tests use real in-memory SQLite and real components.
- Mutation testing: **285 mutants → 209 caught (73.3%), 67 missed** — all 67 missed mutants concentrated in `seo_analyzers.rs`, indicating weak assertions in that file.

### 4.8 Soundness Findings

| Finding | Severity | Details |
|---------|----------|---------|
| **Unsafe FFI exists despite "zero unsafe" claim** | HIGH | `native_plugin.rs` starts with `#![allow(unsafe_code)]` (line 13); 5 unsafe blocks for libloading FFI. `plugin-sdk/export.rs` has ~15 unsafe blocks for WASM ABI alloc/free. SDK Cargo.toml overrides `unsafe_code = "allow"` (`plugin-sdk/Cargo.toml:15`). Pre-commit script excludes `native_plugin.rs` from its own scan. |
| **`unsafe_code = "deny"` not "forbid"** | MEDIUM | README says `forbid`, Cargo.toml says `deny`. `deny` can be re-allowed by sub-crates; `forbid` cannot. The SDK successfully overrides to `allow`. |
| **Blocking SQLite in async API** | HIGH | Direct rusqlite calls in axum handlers will stall the Tokio reactor under load. Engine code correctly uses `spawn_blocking`; API code does not. |
| **WASM plugin runs inline on Tokio workers** | MEDIUM | `crawl_engine.rs:502` executes wasmtime synchronously; bounded by fuel/epoch but still CPU-blocking. Should use `spawn_blocking`. |
| **Dead code shipped and advertised** | MEDIUM | ~1.4k lines: `backpressure.rs`, `distributed_queue.rs`, `enterprise.rs`, `native_plugin.rs` — all have zero production callers. |
| **Duplicated security-critical logic** | HIGH | SSRF blocklist in two diverging copies across crates. |
| **Hardcoded port with no env override** | LOW | API binds `0.0.0.0:4000` with no port env var (`api/main.rs:208`). |
| **Stale user-agent in Playwright JS** | LOW | `playwright.rs:468` hardcodes `'crawlkit/0.4.0'` vs workspace version 5.0.0. |
| **Customer-specific paths in generic code** | LOW | `analyzers/mod.rs:66-67` contains `"/certifications"` and `"/research-use"` — leaked from one deployment. |
| **Site-specific utility-page heuristic** | LOW | `is_utility_page` in `analyzers/mod.rs:66-67` has domain-specific business logic baked into generic engine code. |

### 4.9 Top 5 Strengths & Top 5 Weaknesses

**Strengths:**

1. **Uniform analyzer extension point with deterministic parallelism** — One tiny object-safe `Analyzer` trait (2 methods) implemented by all 31+ built-ins and third-party WASM plugins; canonical output sorting removes scheduling nondeterminism by construction (`analyzers/mod.rs:398`).
2. **Disciplined concurrency in the engine** — Semaphore-bounded spawned fetchers, `FuturesUnordered` draining, `Relaxed`-atomic stats, consistent `spawn_blocking` for every SQLite touchpoint — exemplary Tokio usage.
3. **Serious plugin sandbox** — Fuel metering, epoch-based wall-clock timeout, memory caps, deny-by-default capabilities, hash + ed25519 trust-chain verification with fail-closed policy and SSRF-guarded host fetch (`plugin.rs:14-67,400-530,690-724`).
4. **Enforced hygiene culture** — Workspace-denied `unwrap/expect/panic/exit/unsafe` actually inherited by every app crate, zero TODO/FIXME, exhaustive rustdoc with doctests, ADRs that match shipped code (ADR-008 ↔ `crawls.rs:74-143`), mutation testing in-tree.
5. **Clean layering between binaries and core** — CLI and API both drive the engine solely through the `CrawlEngine` facade; API-plane persistence is genuinely pluggable SQLite↔Postgres behind `Arc<dyn ApiStateStore>`.

**Weaknesses:**

1. **DIP broken at the most important seam** — `StorageBackend` exists with Postgres implementation, but `CrawlEngine` is welded to concrete SQLite `Arc<Storage>` — Postgres crawl data is impossible without refactor, and the trait is decorative for the main path.
2. **Blocking SQLite in async API handlers** — Direct rusqlite calls in axum handlers with no `spawn_blocking` will stall the reactor under load; contradicts the engine's own best practice.
3. **~1.4k lines of unwired "architecture theater"** — `backpressure.rs`, Redis `DistributedQueue`, `enterprise.rs`, `native_plugin.rs` have zero production callers; the queue also lacks a common trait so in-memory vs Redis is not substitutable.
4. **Security logic duplicated across crates** — SSRF/private-IP blocklist exists in two diverging copies; a fix to one silently misses the other.
5. **Hotspot files and leaky specifics** — `plugin.rs` (crypto+sandbox+registry+SSRF in one), `crawl_engine.rs` (orchestration+counters+dedup+encryption), `seo_analyzers.rs` (10 analyzers, all 67 mutation-testing survivors live here); plus hardcoded port, stale UA, customer-specific paths in generic code.

---

## 5. Claims Audit

### README.md claims

| # | Claim | Verdict | Evidence |
|---|-------|---------|----------|
| 1 | **31 analyzers** | **ACCURATE** (with caveat) | Registry registers 31: 26 core + 4 AI + 1 WASM. Confirmed by `assert_eq!(registry.len(), 31)` test at `mod.rs:1204`. However, README table lists only 30 names — **omits `HeadingHierarchyAnalyzer`** (`seo_analyzers.rs:583`). Stale counts in `docs/benchmarks.md` ("18 analyzers") and `mod.rs` doc comments ("28+"). |
| 2 | **736 tests passing** | **STALE** | Empirically measured at HEAD: **798** test functions (647 unit/lib + 105 integration + 46 doc tests). 736 was accurate at an older revision (git log shows the badge was set when the codebase had ~713 functions + ~33 doc tests). |
| 3 | **Throughput ≥ 50 pages/sec** | **UNVERIFIABLE** | No committed benchmark results. `docs/benchmarks.md` presents numbers without raw data. `PERFORMANCE_BENCHMARKS_FINAL.md` claims 245–620 p/s (contradicting README's ≥50). `docs/ARCHITECTURE.md` says "52-68 (varies by network)". Three docs give three different numbers; none backed by criterion output. |
| 4 | **Memory (10k pages) ~200 MB** | **UNVERIFIABLE / CONTRADICTED** | README says ~200MB; `PERFORMANCE_BENCHMARKS.md` says 300MB; `PERFORMANCE_BENCHMARKS_FINAL.md` says 380MB; `docs/ARCHITECTURE.md` says "~380MB avg". Four docs give four numbers. |
| 5 | **Startup time ~10 ms** | **INFLATED** | Empirically measured: **32.1 ms median** (N=100, p95=83.9ms, p99=123.6ms). Overstated by ~3x. |
| 6 | **Binary size ~8 MB** | **INFLATED** | Empirically measured: **23 MB** (release, stripped, LTO, codegen-units=1). Overstated by ~3x. |
| 7 | **Full analyzer suite ~25 µs/page** | **UNVERIFIABLE** | No committed criterion data for this metric. `docs/benchmarks.md` says 25µs for "18 analyzers"; the actual suite runs 31. The actual criterion benchmark exists (`analyzer_full_suite`) but results aren't committed. |
| 8 | **HTML parse (5 KB) ~45 µs** | **UNVERIFIABLE** | Criterion benchmark `parse_5kb_page` exists; no committed results. `PERFORMANCE_BENCHMARKS.md` says p50=5ms (100x higher). |
| 9 | **PageRank (1K nodes) ~4 ms** | **MISSTATED** | Benchmark is `link_graph_pagerank_100_nodes` (100, not 1K). No 1K-node benchmark exists. |
| 10 | **Zero clippy warnings** | **ACCURATE** | Empirically verified: `cargo clippy --workspace --all-targets -- -D warnings` passes with zero output. Enforcement hard-wired in CI (`ci.yml:42`), pre-commit (`pre-commit.sh:59-60`), and justfile. |
| 11 | **Zero unsafe code** | **FALSE** | README says `unsafe_code = "forbid"` — wrong keyword (Cargo.toml:6 says `deny`). Unsafe FFI in `native_plugin.rs` (5 blocks) and `plugin-sdk/export.rs` (~15 blocks). SDK overrides lint to `allow`. |
| 12 | **736 tests (badge)** | **STALE** | Badge should read **798** (empirically measured). |
| 13 | **Rust 1.94+** | **ACCURATE** | `rust-version = "1.94.0"` in `Cargo.toml:25`. Enforcement is consistent: CI `msrv` job installs toolchain `1.94.0` (`.github/workflows/ci.yml`), `scripts/pre-commit.sh` runs `cargo +1.94.0 check --workspace`, and the `justfile` `msrv` recipe does the same. No 1.85 references remain. |

### Documentation claims

| Claim | Source | Verdict | Evidence |
|-------|--------|---------|----------|
| **Competitor numbers in COMPETITIVE_ANALYSIS.md** | `docs/COMPETITIVE_ANALYSIS.md` | **UNSOURCED / INVENTED** | No citations anywhere. Competitor numbers ("~200 pages/sec" for Screaming Frog, etc.) are fabricated estimates. crawlkit's own row claims 500+ pgs/sec vs README's ≥50 — self-contradictory. |
| **"Actor model" architecture** | `docs/ARCHITECTURE.md:950` | **ASPIRATIONAL** | The doc describes MPMC channel-based actor model; actual implementation is `FuturesUnordered` + semaphore. |
| **"Bloom filter + hash set" URL dedup** | `docs/ARCHITECTURE.md:157` | **ASPIRATIONAL** | Actual implementation: `DashSet` dedup — no Bloom filter. |
| **"Arena Allocator for HTML parsing"** | `docs/ARCHITECTURE.md:1034-1040` | **PARTIALLY ACCURATE** | `scraper`/html5ever uses `ego-tree` (arena-like DOM); but the described "allocate page body → parse → extract → drop arena" flow is aspirational. |
| **CLI commands `export` and `schedule`** | `docs/ARCHITECTURE.md:689-698` | **DOES NOT EXIST** | Actual CLI: `crawl`, `compare`, `report`, `backlinks`, `plugin`, `inspect`. No `export` or `schedule` commands. |
| **Mock strategy (MockFetcher, MockStorage)** | `docs/ARCHITECTURE.md:1217-1243` | **ASPIRATIONAL** | Zero mock implementations exist anywhere in the codebase. Tests use real in-memory SQLite. |
| **Dockerfile `rust:1.75`** | `docs/ARCHITECTURE.md:1671` | **STALE** | MSRV is 1.94.0; should be `rust:1.94`. |
| **API bind address `0.0.0.0:8080`** | `docs/ARCHITECTURE.md:1796` | **WRONG** | Actual: `0.0.0.0:4000` hardcoded (`api/main.rs:208`). |
| **SSL certificate chain validation** | README | **OVERSTATED** | `SslCertificateValidator::empty()` registered in production (`analyzers/mod.rs:320`); with `cert_info: None` it returns nothing. Expiry/chain checks exist but never receive real cert data from the TLS session. |
| **ENTERPRISE_ARCHITECTURE.md: RS256 JWT** | `ENTERPRISE_ARCHITECTURE.md:247` | **FALSE** | Actual: HS256 HMAC with shared secret (`auth.rs:216`). |
| **ENTERPRISE_ARCHITECTURE.md: SAML** | `ENTERPRISE_ARCHITECTURE.md` | **NOT IMPLEMENTED** | OIDC is implemented; SAML is not. |
| **WIRING_PLAN.md: backpressure module wired** | `WIRING_PLAN.md` | **STALE** | `backpressure.rs` has zero production callers as of 2026-08-26. |
| **WIRING_PLAN.md: DNS prefetch wired** | `WIRING_PLAN.md` | **STALE** | `dns.rs` DnsPrefetcher exported at `lib.rs:318` but used by nothing. |
| **"Plugin failures never abort a crawl"** | README | **ACCURATE** | `plugin_runtime.rs:33-42` — errors logged and swallowed, returns empty findings. |
| **Argon2 password hashing** | README | **ACCURATE** | `argon2 = "0.5"` dependency; `auth.rs:182-190` with per-user salt generation. |
| **OIDC support** | README | **ACCURATE** | Real implementation: discovery, JWKS, PKCE S256, id_token validation (`oidc.rs:151-350`). |

---

## 6. Prioritized Recommendations

### P0 — Correctness / Security

1. **Fix the SSRF blocklist duplication.** Extract private-IP/SSRF validation into a shared crate or module used by both `plugin.rs` and `api/types.rs`. Two diverging copies of security-critical code is a vulnerability.
2. **Remove the "zero unsafe code" claim** or change to "zero unsafe in core engine crates." The plugin-sdk and native_plugin.rs legitimately use unsafe for FFI — document this honestly.
3. **Fix the README: `forbid` → `deny`.** `forbid` cannot be overridden; `deny` can — and is. Accuracy matters.
4. **Wire `SslCertificateValidator` to real cert data** or remove it from the default registry. Currently it's a no-op in production.

### P1 — Architecture / Design

5. **Wire `CrawlEngine` to `StorageBackend` trait** instead of concrete `Arc<Storage>`. This is the single most impactful DIP fix — it enables Postgres-powered crawls and makes the trait non-decorative.
6. **Move blocking SQLite calls in API handlers to `spawn_blocking`.** The engine does this consistently; the API should follow suit.
7. **Extract `plugin.rs` into focused modules** (manifest, crypto, trust-store, SSRF, registry) — 1331 lines mixing 6+ concerns violates SRP.
8. **Delete or gate dead code** (~1.4k lines): `backpressure.rs`, `distributed_queue.rs`, `enterprise.rs`, `native_plugin.rs`. If planned, mark as `#[cfg(feature = "unstable")]` or remove from `lib.rs` public exports.
9. **Extract SSRF validation and `Finding`/`Severity` types** into a shared crate to eliminate cross-crate duplication.

### P2 — Documentation Accuracy

10. **Synchronize test count badge.** Current: 736. Actual: 798.
11. ~~**Remove or qualify all performance claims** until backed by committed criterion output.~~ **RESOLVED (2026-09-05)** — raw criterion output and environment metadata are committed in `docs/benchmarks/2026-09-05/` with a summary in `docs/benchmarks/measured-2026-09-05.md`; contradictory prose must now defer to that evidence.
12. **Fix binary size claim** (~8 MB → ~23 MB) and **startup time claim** (~10ms → ~32ms median).
13. **Update `docs/ARCHITECTURE.md` §Standards Compliance** to reflect shipped items (determinism, circuit breaker, encryption, audit trail) and remove aspirational descriptions (actor model, Bloom filter, MockFetcher, channel pipeline).
14. **Update `docs/COMPETITIVE_ANALYSIS.md`** — either source the competitor numbers or remove fabricated claims. Self-contradictory numbers (500+ vs ≥50) damage credibility.
15. **Update `ENTERPRISE_ARCHITECTURE.md`** — RS256 → HS256; remove SAML; mark shipped items as Active.
16. ~~**Fix MSRV inconsistency** — `pre-commit.sh:113` and `justfile:67,98` check 1.85.0 but workspace MSRV is 1.94.0.~~ **RESOLVED** — all enforcement paths (`scripts/pre-commit.sh:105`, `justfile` `msrv` recipe, CI `msrv` job) now pin 1.94.0, matching `rust-version` in `Cargo.toml`.

### P3 — Code Quality

17. **Replace regex recompilation in `sitemap.rs`** with `OnceLock` (like `selectors.rs` already does) — 6 static regexes with `.unwrap()` per call.
18. **Remove customer-specific paths** (`"/certifications"`, `"/research-use"`) from generic `is_utility_page` heuristic in `analyzers/mod.rs:66-67`.
19. **Fix stale user-agent** in Playwright JS generation (`playwright.rs:468`): `crawlkit/0.4.0` → version from `Cargo.toml`.
20. **Add CLI end-to-end tests** — the `crawlkit` binary crate has no test directory; key user-facing flows are untested.

---

*Audit performed 2026-08-26 by static analysis + empirical verification (cargo clippy, cargo test, cargo build, startup timing). All evidence includes file:line references.*
