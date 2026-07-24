# crawlkit Dead Module Wiring Plan

Status: Proposed. Requires stakeholder approval before execution.

---

## Overview

14 modules (~40% of crawlkit-core) are fully implemented but never wired into the
crawl loop, CLI, or API. This plan integrates each module at the correct abstraction
layer, transforming scaffolding into production code.

## Module Classification

### Tier 1: Crawl Loop Integration (wire into run_crawl)

These modules enhance the core crawl pipeline directly.

| Module | Purpose | Integration Point | Effort |
|--------|---------|-------------------|--------|
| `dns.rs` | DNS cache + prefetch | Replace reqwest internal DNS | 8h |
| `circuit_breaker.rs` | Per-domain fault isolation | Wrap HTTP fetch calls | 4h |
| `observability.rs` | Atomic metrics counters | Replace inline counters in crawl loop | 4h |
| `resource_monitor.rs` | Resource limit enforcement | Replace inline RSS check | 4h |
| `backpressure.rs` | Bounded pipeline channels | Wrap queue + fetch pipeline | 6h |
| `determinism.rs` | Seed-based reproducibility | Add --seed CLI flag | 2h |

### Tier 2: CLI/API Integration (wire into commands)

These modules enhance the CLI and API surface.

| Module | Purpose | Integration Point | Effort |
|--------|---------|-------------------|--------|
| `feature_flags.rs` | Runtime feature toggles | Wire CLI flags to feature checks | 4h |
| `js_render_decision.rs` | SPA detection | Wire before Playwright dispatch | 2h |
| `advanced_features.rs` | Alerts + scheduler | Wire into API for monitoring | 8h |
| `playwright.rs` | JS rendering | Wire --javascript flag | 16h |

### Tier 3: Storage/API Integration (wire into data layer)

| Module | Purpose | Integration Point | Effort |
|--------|---------|-------------------|--------|
| `encryption.rs` | AES-256-GCM at rest | Wrap Storage writes | 8h |

### Tier 4: Deferred (requires architectural decision)

| Module | Purpose | Decision Required | Effort |
|--------|---------|-------------------|--------|
| `plugin.rs` | Native plugin loading | Needs libloading, sandboxing design | 24h |
| `enterprise.rs` | RBAC/SSO/SLO | Needs auth architecture decision | 40h |
| `link_graph.rs` | Duplicate PageRank | Merge into backlinks.rs or delete | 8h |

---

## Detailed Wiring Specifications

### 1. dns.rs -- DNS Cache + Prefetch

**Current state:** `DnsCache` with TTL eviction, `DnsPrefetcher` with background
prefetch queue. Never instantiated.

**Wiring plan:**
- Create `DnsCache` + `DnsPrefetcher` in `run_crawl()` initialization
- Spawn `DnsPrefetcher::run()` as background tokio task
- After queue push, enqueue URL hostname for prefetch
- Before `client.fetch()`, check `DnsCache::resolve()` for cached IP
- On fetch success, insert resolved IP into cache
- On fetch failure, evict stale entries

**CLI flags:**
- `--dns-cache-size <N>` (default: 10000)
- `--dns-prefetch-concurrency <N>` (default: 32)
- `--dns-ttl-secs <N>` (default: 300)

**Expected improvement:** 10-30ms per request on repeated domains (DNS cache hit
avoids recursive resolution).

### 2. circuit_breaker.rs -- Per-Domain Fault Isolation

**Current state:** `CircuitBreaker` with Closed/Open/HalfOpen states.
`CircuitBreakerRegistry` for per-domain management. Never instantiated.

**Wiring plan:**
- Create `CircuitBreakerRegistry` in `run_crawl()` initialization
- Before `client.fetch()`, check `registry.is_allowed(domain)`
- On fetch success, call `breaker.record_success()`
- On fetch failure, call `breaker.record_failure()`
- When circuit is Open, skip domain (log warning, increment counter)
- Expose circuit state via observability metrics

**CLI flags:**
- `--circuit-breaker-threshold <N>` (default: 5 consecutive failures)
- `--circuit-breaker-cooldown <SECS>` (default: 60)

**Expected improvement:** Prevents cascading failures when target domains are down.
Avoids wasting time on unresponsive endpoints.

### 3. observability.rs -- Atomic Metrics

**Current state:** `Metrics` with atomic counters for pages, bytes, timings,
connections. `SharedMetrics` wrapper. Never instantiated. API uses its own
Prometheus metrics.

**Wiring plan:**
- Create `Metrics` in `run_crawl()` initialization
- Replace inline counters (`pages_crawled`, `issues_found`, etc.) with
  `metrics.record_page_success()` / `metrics.record_page_failure()`
- After crawl completes, export `MetricsSnapshot` to JSON
- In API, expose `MetricsSnapshot` via `/metrics` endpoint alongside Prometheus
- Add `--metrics-json <PATH>` CLI flag for metrics export

**CLI flags:**
- `--metrics-json <PATH>` (export metrics snapshot to file)

**Expected improvement:** Structured performance data for regression detection.
Enables benchmark comparison between runs.

### 4. resource_monitor.rs -- Resource Limit Enforcement

**Current state:** `ResourceMonitor` with memory, CPU, disk, duration, page count
limits. `ResourceLimits` configuration. Never instantiated. CLI uses inline
`get_process_rss_bytes()`.

**Wiring plan:**
- Create `ResourceMonitor` with configurable `ResourceLimits` in `run_crawl()`
- Replace inline RSS check (lines 508-520) with `monitor.check_limits()`
- Call `monitor.record_page()` after each successful fetch
- Call `monitor.record_bytes()` after each fetch
- When limits exceeded, log warning and break crawl loop
- Expose resource usage via observability metrics

**CLI flags:**
- `--memory-limit-mb <N>` (default: 512)
- `--cpu-limit-percent <N>` (default: 90)
- `--disk-limit-mb <N>` (default: 1024)

**Expected improvement:** Proper resource management with configurable limits.
Replaces ad-hoc RSS check with structured monitoring.

### 5. backpressure.rs -- Bounded Pipeline

**Current state:** `BackpressureController` with semaphore-based concurrency
control. `BoundedPipeline<T>` with bounded mpsc channels. Never instantiated.

**Wiring plan:**
- Create `BackpressureController` with `max_concurrent` = CLI concurrency
- Wrap fetch dispatch: acquire permit before `client.fetch()`
- `BackpressurePermit` drops after fetch+store completes
- Replace `tokio::sync::Mutex<UrlQueue>` with `BoundedPipeline<UrlEntry>`
  for queue operations
- Monitor `active_tasks` count for observability

**CLI flags:**
- `--backpressure-capacity <N>` (default: 1000, bounded channel capacity)

**Expected improvement:** Prevents memory explosion under high concurrency.
Bounded channels ensure producers block when consumers are overwhelmed.

### 6. determinism.rs -- Seed-Based Reproducibility

**Current state:** `DeterminismController` with seed-based PRNG, content hashing,
deterministic sorting. Never instantiated.

**Wiring plan:**
- Create `DeterminismController` with optional seed in `run_crawl()`
- If `--seed <N>` provided, use controller for:
  - Deterministic URL ordering in queue
  - Deterministic content hash (reproducible dedup)
  - Seed-specific user-agent rotation
- Export seed + configuration to crawl metadata for replay
- Add `--replay <CRAWL_ID>` CLI flag to re-run with stored seed

**CLI flags:**
- `--seed <N>` (random seed for reproducible crawls)

**Expected improvement:** Reproducible crawl results for A/B testing and
regression detection.

### 7. feature_flags.rs -- Runtime Feature Toggles

**Current state:** 8 feature flag constants defined but never checked.
TOML-based configuration. `SharedFeatureFlags` wrapper.

**Wiring plan:**
- Load feature flags from `crawlkit.toml` `[features]` section
- Check flags at integration points:
  - `FLAG_JS_RENDERING`: if true, use Playwright for JS pages
  - `FLAG_AI_ANALYZERS`: if true, include AI analyzers in registry
  - `FLAG_WASM_ANALYZERS`: if true, include WASM analyzers
  - `FLAG_BACKLINK_ANALYSIS`: if true, run backlink analysis
  - `FLAG_RUM_INTEGRATION`: if true, fetch CrUX data
  - `FLAG_ENCRYPTION_AT_REST`: if true, encrypt storage
  - `FLAG_AUDIT_TRAIL`: if true, record audit events
  - `FLAG_OBSERVABILITY`: if true, collect metrics
- Add `--enable-<feature>` / `--disable-<feature>` CLI flags

**Config example:**
```toml
[features]
js_rendering = false
ai_analyzers = true
wasm_analyzers = true
backlink_analysis = true
encryption_at_rest = false
```

**Expected improvement:** Runtime configurability without recompilation.
Enables feature experimentation.

### 8. js_render_decision.rs -- SPA Detection

**Current state:** `JsRenderDecisionEngine` with SPA detection heuristics.
Never instantiated. Would feed into Playwright dispatch.

**Wiring plan:**
- Create `JsRenderDecisionEngine` in `run_crawl()`
- After HTML parse, call `engine.decide(&parsed, &url)`
- If decision is `Render`, dispatch to Playwright (if enabled)
- If decision is `Skip`, use static HTML (current behavior)
- Log decisions for analysis

**Integration:** Feeds directly into Playwright wiring (#9).

### 9. playwright.rs -- JavaScript Rendering

**Current state:** Full Playwright integration via Node.js subprocess.
`PlaywrightRenderer` with context isolation. Never called. CLI `--javascript`
flag is accepted but ignored.

**Wiring plan:**
- If `--javascript` flag is set AND `FLAG_JS_RENDERING` is enabled:
  - Create `PlaywrightRenderer` with `PlaywrightConfig`
  - Check `PlaywrightDetector::is_available()` for browser binary
  - After `JsRenderDecisionEngine` returns `Render`:
    - Call `renderer.render(&url, &html)` to get JS-rendered content
    - Replace static HTML with rendered content before analysis
  - Track `active_contexts` for resource management
- If Playwright not available, log warning and fall back to static HTML

**CLI flags:**
- `--javascript` (enable JS rendering, already exists)
- `--playwright-timeout <SECS>` (default: 30)
- `--playwright-max-memory <MB>` (default: 512)

**Expected improvement:** Proper JS rendering for SPAs (React, Vue, Angular).
Currently these pages return empty bodies.

### 10. encryption.rs -- Encryption at Rest

**Current state:** AES-256-GCM encryption with file/env/keyring key sources.
Never instantiated.

**Wiring plan:**
- If `FLAG_ENCRYPTION_AT_REST` is enabled:
  - Create `EncryptionManager` with `EncryptionConfig` from env or config
  - Before `storage.insert_page()`, encrypt sensitive fields (body, title,
    description)
  - After `storage.get_pages()`, decrypt fields
  - Key source priority: env `CRAWLKIT_ENCRYPTION_KEY` > config file > keyring
- If no key configured, log warning and run unencrypted

**CLI flags:**
- `--encrypt` (enable encryption)
- `--encryption-key <KEY>` (base64-encoded AES-256 key)

**Expected improvement:** Data protection for sensitive crawl targets.
Compliance with data protection requirements.

### 11. advanced_features.rs -- Alerts + Scheduler

**Current state:** `AlertManager` for threshold monitoring, `CrawlScheduler`
for periodic crawls, `TrendTracker` for historical trends. Never instantiated.

**Wiring plan:**
- **AlertManager:** Wire into API. After crawl completes, check alert rules.
  If threshold exceeded, fire webhook notification.
- **CrawlScheduler:** Wire into API. Background task checks schedule and
  triggers crawls. Already partially implemented in API (`/api/v1/schedules`).
- **TrendTracker:** Wire into export. After crawl, record metrics snapshot
  for trend analysis. Expose via `/api/v1/trends` endpoint.

**Integration:** Primarily API-side, not crawl loop.

### 12. link_graph.rs -- Merge into backlinks.rs

**Current state:** Separate PageRank implementation divergent from backlinks.rs.
Used only in tests/benchmarks.

**Wiring plan:**
- Audit `link_graph.rs` for unique functionality:
  - `to_dot()` -- graph export (not in backlinks.rs)
  - `to_csv()` -- CSV export (not in backlinks.rs)
  - `orphan_pages()` -- already in backlinks.rs
- Merge unique methods into `BacklinkAnalyzer`
- Delete `link_graph.rs`
- Update tests to use `BacklinkAnalyzer`

**Expected improvement:** Single PageRank implementation eliminates divergence risk.

---

## Execution Order

### Phase A: Crawl Loop Hardening (Tier 1)
1. `observability.rs` -- metrics foundation
2. `resource_monitor.rs` -- resource limits
3. `circuit_breaker.rs` -- fault isolation
4. `dns.rs` -- DNS caching
5. `backpressure.rs` -- bounded pipeline
6. `determinism.rs` -- reproducibility

### Phase B: Feature Integration (Tier 2)
7. `feature_flags.rs` -- runtime toggles
8. `js_render_decision.rs` -- SPA detection
9. `playwright.rs` -- JS rendering
10. `advanced_features.rs` -- alerts + scheduler

### Phase C: Data Layer (Tier 3)
11. `encryption.rs` -- encryption at rest

### Phase D: Cleanup (Tier 4)
12. `link_graph.rs` -- merge into backlinks.rs
13. `plugin.rs` -- architectural decision
14. `enterprise.rs` -- architectural decision

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| DNS cache stale entries | Medium | TTL eviction, cache size limit |
| Circuit breaker false positives | Medium | Configurable thresholds, cooldown |
| Playwright not installed | Low | Graceful fallback to static HTML |
| Encryption key management | High | env > config > keyring priority |
| Feature flag conflicts | Low | Validation on startup |
| Memory increase from caching | Medium | Resource monitor limits |
| Breaking existing tests | High | Incremental wiring, test after each |

---

## Verification Criteria

After each module is wired:
- [ ] All 430+ existing tests pass
- [ ] Zero clippy `-D warnings` errors
- [ ] `cargo fmt` clean
- [ ] New unit tests for integration points
- [ ] Integration test for crawl loop with new component
- [ ] CLI flag help text updated
- [ ] Documentation updated in web/src/content/docs/

---

*Generated: 2026-07-24 | Version: 0.4.0*
