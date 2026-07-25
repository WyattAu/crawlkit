# crawlkit Dead Module Wiring Plan

*Status: Proposed. Requires stakeholder approval before execution.*
*Version: 2.0.0 | Last updated: 2026-07-25*

## Overview

14 modules (~40% of `crawlkit-engine`) are fully implemented but never wired into the crawl loop, CLI, or API. This plan integrates each module at the correct abstraction layer.

## Module Classification

### Tier 1: Crawl Loop Integration (wire into `run_crawl`)

| Module | Purpose | Integration Point | Effort |
|--------|---------|-------------------|--------|
| `dns.rs` | DNS cache + prefetch | Replace reqwest internal DNS | 8h |
| `circuit_breaker.rs` | Per-domain fault isolation | Wrap HTTP fetch calls | 4h |
| `observability.rs` | Atomic metrics counters | Replace inline counters | 4h |
| `resource_monitor.rs` | Resource limit enforcement | Replace inline RSS check | 4h |
| `backpressure.rs` | Bounded pipeline channels | Wrap queue + fetch pipeline | 6h |
| `determinism.rs` | Seed-based reproducibility | Add `--seed` CLI flag | 2h |

### Tier 2: CLI/API Integration (wire into commands)

| Module | Purpose | Integration Point | Effort |
|--------|---------|-------------------|--------|
| `feature_flags.rs` | Runtime feature toggles | Wire CLI flags to feature checks | 4h |
| `js_render_decision.rs` | SPA detection | Wire before Playwright dispatch | 2h |
| `advanced_features.rs` | Alerts + scheduler | Wire into API for monitoring | 8h |
| `playwright.rs` | JS rendering | Wire `--javascript` flag | 16h |

### Tier 3: Storage/API Integration (wire into data layer)

| Module | Purpose | Integration Point | Effort |
|--------|---------|-------------------|--------|
| `encryption.rs` | AES-256-GCM at rest | Wrap Storage writes | 8h |

### Tier 4: Deferred (requires architectural decision)

| Module | Purpose | Decision Required | Effort |
|--------|---------|-------------------|--------|
| `plugin.rs` | Native plugin loading | Needs libloading, sandboxing design | 24h |
| `enterprise.rs` | RBAC/SSO/SLO | Needs auth architecture decision | 40h |
| `link_graph.rs` | Duplicate PageRank | Merge into `backlinks.rs` or delete | 8h |

## Detailed Wiring Specifications

### 1. dns.rs -- DNS Cache + Prefetch

**Current state:** `DnsCache` with TTL eviction, `DnsPrefetcher` with background prefetch queue. Never instantiated.

**Wiring plan:**
- Instantiate `DnsCache` + `DnsPrefetcher` in `run_crawl()` initialization
- Spawn `DnsPrefetcher::run()` as background tokio task
- After queue push, enqueue URL hostname for prefetch
- Before `client.fetch()`, check `DnsCache::resolve()` for cached IP
- On fetch success, insert resolved IP into cache
- On fetch failure, evict stale entries

**CLI flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--dns-cache-size <N>` | 10000 | Maximum cache entries |
| `--dns-prefetch-concurrency <N>` | 32 | Concurrent prefetch tasks |
| `--dns-ttl-secs <N>` | 300 | Cache entry TTL |

**Expected improvement:** 10-30 ms per request on repeated domains (DNS cache hit avoids recursive resolution).

### 2. circuit_breaker.rs -- Per-Domain Fault Isolation

**Current state:** `CircuitBreaker` with Closed/Open/HalfOpen states. `CircuitBreakerRegistry` for per-domain management. Never instantiated.

**Wiring plan:**
- Instantiate `CircuitBreakerRegistry` in `run_crawl()` initialization
- Before `client.fetch()`, check `registry.is_allowed(domain)`
- On fetch success, call `breaker.record_success()`
- On fetch failure, call `breaker.record_failure()`
- When circuit is Open, skip domain (log warning, increment counter)
- Expose circuit state via observability metrics

**CLI flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--circuit-breaker-threshold <N>` | 5 | Consecutive failures before Open |
| `--circuit-breaker-cooldown <SECS>` | 60 | Cooldown before HalfOpen transition |

**Expected improvement:** Prevents cascading failures when target domains are down. Avoids wasting time on unresponsive endpoints.

### 3. observability.rs -- Atomic Metrics

**Current state:** `Metrics` with atomic counters for pages, bytes, timings, connections. `SharedMetrics` wrapper. Never instantiated.

**Wiring plan:**
- Instantiate `Metrics` in `run_crawl()` initialization
- Replace inline counters (`pages_crawled`, `issues_found`) with `metrics.record_page_success()` / `metrics.record_page_failure()`
- After crawl completes, export `MetricsSnapshot` to JSON
- Expose `MetricsSnapshot` via `/metrics` endpoint alongside Prometheus

**CLI flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--metrics-json <PATH>` | — | Export metrics snapshot to file |

**Expected improvement:** Structured performance data for regression detection. Enables benchmark comparison between runs.

### 4. resource_monitor.rs -- Resource Limit Enforcement

**Current state:** `ResourceMonitor` with memory, CPU, disk, duration, page count limits. `ResourceLimits` configuration. Never instantiated. CLI uses inline `get_process_rss_bytes()`.

**Wiring plan:**
- Instantiate `ResourceMonitor` with configurable `ResourceLimits` in `run_crawl()`
- Replace inline RSS check (lines 508-520) with `monitor.check_limits()`
- Call `monitor.record_page()` after each successful fetch
- Call `monitor.record_bytes()` after each fetch
- When limits exceeded, log warning and break crawl loop

**CLI flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--memory-limit-mb <N>` | 512 | Maximum memory usage |
| `--cpu-limit-percent <N>` | 90 | Maximum CPU usage |
| `--disk-limit-mb <N>` | 1024 | Maximum disk usage |

**Expected improvement:** Proper resource management with configurable limits. Replaces ad-hoc RSS check with structured monitoring.

### 5. backpressure.rs -- Bounded Pipeline

**Current state:** `BackpressureController` with semaphore-based concurrency control. `BoundedPipeline<T>` with bounded mpsc channels. Never instantiated.

**Wiring plan:**
- Instantiate `BackpressureController` with `max_concurrent` = CLI concurrency
- Wrap fetch dispatch: acquire permit before `client.fetch()`
- `BackpressurePermit` drops after fetch+store completes
- Replace `tokio::sync::Mutex<UrlQueue>` with `BoundedPipeline<UrlEntry>` for queue operations

**CLI flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--backpressure-capacity <N>` | 1000 | Bounded channel capacity |

**Expected improvement:** Prevents memory explosion under high concurrency. Bounded channels ensure producers block when consumers are overwhelmed.

### 6. determinism.rs -- Seed-Based Reproducibility

**Current state:** `DeterminismController` with seed-based PRNG, content hashing, deterministic sorting. Never instantiated.

**Wiring plan:**
- Instantiate `DeterminismController` with optional seed in `run_crawl()`
- If `--seed <N>` provided, use controller for:
  - Deterministic URL ordering in queue
  - Deterministic content hash (reproducible dedup)
  - Seed-specific user-agent rotation
- Export seed + configuration to crawl metadata for replay

**CLI flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--seed <N>` | — | Random seed for reproducible crawls |

**Expected improvement:** Reproducible crawl results for A/B testing and regression detection.

### 7. feature_flags.rs -- Runtime Feature Toggles

**Current state:** 8 feature flag constants defined but never checked. TOML-based configuration. `SharedFeatureFlags` wrapper.

**Wiring plan:**
- Load feature flags from `crawlkit.toml` `[features]` section
- Check flags at integration points:

| Flag | Controls |
|------|----------|
| `FLAG_JS_RENDERING` | Playwright dispatch for JS pages |
| `FLAG_AI_ANALYZERS` | Include AI analyzers in registry |
| `FLAG_WASM_ANALYZERS` | Include WASM analyzers |
| `FLAG_BACKLINK_ANALYSIS` | Run backlink analysis |
| `FLAG_RUM_INTEGRATION` | Fetch CrUX data |
| `FLAG_ENCRYPTION_AT_REST` | Encrypt storage |
| `FLAG_AUDIT_TRAIL` | Record audit events |
| `FLAG_OBSERVABILITY` | Collect metrics |

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

### 8. js_render_decision.rs -- SPA Detection

**Current state:** `JsRenderDecisionEngine` with SPA detection heuristics. Never instantiated.

**Wiring plan:**
- Instantiate `JsRenderDecisionEngine` in `run_crawl()`
- After HTML parse, call `engine.decide(&parsed, &url)`
- If `Render`, dispatch to Playwright (if enabled)
- If `Skip`, use static HTML (current behavior)

**Integration:** Feeds directly into Playwright wiring (#9).

### 9. playwright.rs -- JavaScript Rendering

**Current state:** Full Playwright integration via Node.js subprocess. `PlaywrightRenderer` with context isolation. Never called. CLI `--javascript` flag is accepted but ignored.

**Wiring plan:**
- If `--javascript` flag is set AND `FLAG_JS_RENDERING` enabled:
  - Instantiate `PlaywrightRenderer` with `PlaywrightConfig`
  - Check `PlaywrightDetector::is_available()` for browser binary
  - After `JsRenderDecisionEngine` returns `Render`:
    - Call `renderer.render(&url, &html)` to get JS-rendered content
    - Replace static HTML with rendered content before analysis
- If Playwright not available, log warning and fall back to static HTML

**CLI flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--javascript` | — | Enable JS rendering (already exists) |
| `--playwright-timeout <SECS>` | 30 | Render timeout |
| `--playwright-max-memory <MB>` | 512 | Renderer memory limit |

**Expected improvement:** Proper JS rendering for SPAs (React, Vue, Angular). Currently these pages return empty bodies.

### 10. encryption.rs -- Encryption at Rest

**Current state:** AES-256-GCM encryption with file/env/keyring key sources. Never instantiated.

**Wiring plan:**
- If `FLAG_ENCRYPTION_AT_REST` enabled:
  - Instantiate `EncryptionManager` with `EncryptionConfig` from env or config
  - Before `storage.insert_page()`, encrypt sensitive fields (body, title, description)
  - After `storage.get_pages()`, decrypt fields
  - Key source priority: env `CRAWLKIT_ENCRYPTION_KEY` > config file > keyring

**CLI flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--encrypt` | — | Enable encryption |
| `--encryption-key <KEY>` | — | Base64-encoded AES-256 key |

**Expected improvement:** Data protection for sensitive crawl targets. Compliance with data protection requirements.

### 11. advanced_features.rs -- Alerts + Scheduler

**Current state:** `AlertManager` for threshold monitoring, `CrawlScheduler` for periodic crawls, `TrendTracker` for historical trends. Never instantiated.

**Wiring plan:**
- **AlertManager:** Wire into API. After crawl completes, check alert rules. Fire webhook notification on threshold breach.
- **CrawlScheduler:** Wire into API. Background task checks schedule and triggers crawls. Partially implemented in `/api/v1/schedules`.
- **TrendTracker:** Wire into export. Record metrics snapshot for trend analysis. Expose via `/api/v1/trends`.

**Integration:** Primarily API-side, not crawl loop.

### 12. link_graph.rs -- Merge into backlinks.rs

**Current state:** Separate PageRank implementation divergent from `backlinks.rs`. Used only in tests/benchmarks.

**Wiring plan:**
- Audit `link_graph.rs` for unique functionality:
  - `to_dot()` -- graph export (not in `backlinks.rs`)
  - `to_csv()` -- CSV export (not in `backlinks.rs`)
  - `orphan_pages()` -- already in `backlinks.rs`
- Merge unique methods into `BacklinkAnalyzer`
- Delete `link_graph.rs`
- Update tests to use `BacklinkAnalyzer`

**Expected improvement:** Single PageRank implementation eliminates divergence risk.

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

12. `link_graph.rs` -- merge into `backlinks.rs`
13. `plugin.rs` -- architectural decision
14. `enterprise.rs` -- architectural decision

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

## Verification Criteria

After each module is wired:

- All 560+ existing tests pass
- Zero `cargo clippy --all-targets -- -D warnings` errors
- `cargo fmt --check` clean
- New unit tests for integration points
- Integration test for crawl loop with new component
- CLI flag help text updated
- Documentation updated in `web/src/content/docs/`
