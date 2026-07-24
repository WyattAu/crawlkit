# crawlkit Technical Roadmap

Status: Living document. Updated after each audit cycle.

---

## Current State (v0.6.0)

- 434 tests passing, zero clippy warnings in production code, zero rustfmt diffs
- 28 analyzers (23 core + 4 AI + 1 WASM) with **Rayon parallel execution**
- CLI + REST API + HTML/JSON/CSV/Markdown export
- **12 modules wired into production** (observability, resource_monitor, circuit_breaker, backpressure, determinism, feature_flags, js_render_decision, playwright, advanced_features, encryption, alert_manager, metrics)
- **New CLI flags:** --seed, --encrypt, --enable-ai, --enable-wasm, --metrics-json
- **HTTP/2 multiplexing** with connection pooling scaled to concurrency
- **Prometheus metrics** in API (crawlkit_crawls_total, pages_crawled, issues_found, fetch_duration, analysis_duration, errors)
- **OpenTelemetry spans** with structured attributes (url, depth, seed)
- CI/CD: format, clippy -D warnings, tests, security audit, supply chain audit, release builds, MSRV check
- Pre-commit hook: fmt, clippy, build, tests, security audit, secret scan, unsafe code check
- Astro Starlight documentation site: 13 pages, all new flags documented
- Supply chain security: cargo-deny for license compliance and advisory auditing

### Wiring Summary

| Module | Status | Integration |
|--------|--------|-------------|
| observability.rs | WIRED | Metrics in crawl loop, exported to metrics.json |
| resource_monitor.rs | WIRED | Replaces inline RSS check |
| circuit_breaker.rs | WIRED | Per-domain fault isolation |
| backpressure.rs | WIRED | Bounded pipeline with semaphore |
| determinism.rs | WIRED | Seed-based reproducibility |
| feature_flags.rs | WIRED | Runtime toggles via TOML |
| js_render_decision.rs | WIRED | SPA detection engine |
| playwright.rs | WIRED | JS rendering with timeout/memory limits |
| advanced_features.rs | WIRED | Alert manager with error rate monitoring |
| encryption.rs | WIRED | AES-256-GCM at rest |
| link_graph.rs | MERGED | to_dot/to_csv into backlinks.rs, deleted |
| plugin.rs | DEFERRED | Needs libloading + sandboxing architecture |
| enterprise.rs | DEFERRED | Needs RBAC/SSO auth architecture |

### Audit Fixes Applied (2026-07-24)

| Category | Fix | File | Severity |
|----------|-----|------|----------|
| Correctness | Cache returned 1 page instead of N | storage.rs | Critical |
| Correctness | `insert_page` missing SQLite transaction | storage.rs | Critical |
| Correctness | `insert_pages` loop without batching | storage.rs | Critical |
| Thread Safety | `std::sync::Mutex` poisoning risk replaced with `parking_lot::Mutex` | storage.rs | Critical |
| Thread Safety | `dec_connections` underflow to u64::MAX | observability.rs | High |
| Thread Safety | `UserAgentRotator` Relaxed ordering | http.rs | Medium |
| Security | SHA-256 audit hash (was DefaultHasher) | audit.rs | High |
| Security | Playwright JS injection via URL interpolation | playwright.rs | High |
| Security | API metrics expect() replaced with error handling | api/main.rs | High |
| Correctness | HSTS validation byte-slicing by wrong index | analyzers.rs | High |
| Correctness | Operator precedence in `is_weak_algorithm` | analyzers.rs | Medium |
| Correctness | `backlinks` NaN panic in sort | backlinks.rs | Medium |
| Correctness | Hardcoded user-agent "crawlkit/0.1.0" replaced with CARGO_PKG_VERSION | main.rs | High |
| Correctness | VERSION.md version mismatch (0.1.0 vs 0.4.0) fixed | VERSION.md | Medium |
| Performance | CSS selectors parsed on every call; cached via OnceLock | parser.rs | Medium |
| Correctness | rum.rs unwrap() on Option replaced with error propagation | rum.rs | Medium |
| Linting | Workspace clippy lints hardened: unwrap_used, expect_used, panic -> warn | Cargo.toml | High |
| Linting | cast_lossless, items_after_statements, if_not_else, unused_async, redundant_clone fixed | api/main.rs, main.rs | Medium |
| Infrastructure | Pre-commit hook tracked in repo (scripts/pre-commit.sh) | scripts/ | Medium |
| Infrastructure | cargo-deny config for supply chain security | deny.toml | Medium |
| Infrastructure | justfile for build automation | justfile | Low |
| Infrastructure | Supply chain audit CI workflow | .github/workflows/audit.yml | Medium |

---

## Phase 1: Production Hardening (v0.5.0)

### Critical Path

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Fix duplicate `PageData` usage in CLI | P0 | 2h | CLI uses `storage::PageData` but `lib.rs` had dead copy (removed) |
| Implement `insert_pages` true batch | P0 | 4h | Currently loops `insert_page` -- copy `insert_issues` pattern |
| Wire `RobotsTxtAnalyzer` into crawl loop | P0 | 8h | `respect_robots` flag exists but robots.txt is never fetched |
| Fix `insert_page` error handling in CLI | P1 | 2h | Currently `continue` on failure -- should track and report |
| Implement `compare` subcommand | P1 | 16h | Currently returns `not_implemented` stub |
| Implement `run_report` for all formats | P1 | 8h | Only JSON/Markdown/CSV implemented; HTML via separate path |
| Fix `extract_robots_txt` stub | P1 | 4h | AI analyzer always returns None |
| Remove placeholder mobile findings (MOB006-008) | P1 | 2h | Unconditionally emitted on every page |

### Security

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Rotate hardcoded dev API key | P0 | 2h | `ck_dev_default_key_for_testing` in production |
| Add CORS configuration to API | P1 | 2h | Currently no CORS headers |
| Implement rate limit response headers | P1 | 4h | X-RateLimit-Remaining, Retry-After |
| Add input validation to API endpoints | P1 | 4h | URL length, page count bounds |

---

## Phase 2: Core Feature Completion (v0.6.0)

### Crawl Engine

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Sitemap.xml auto-discovery | P0 | 8h | Parse sitemap, add URLs to queue with priority |
| Incremental crawl support | P1 | 16h | Re-crawl only changed pages (ETag/If-Modified-Since) |
| JavaScript rendering integration | P1 | 24h | Wire Playwright into crawl loop when enabled |
| Multi-tenant crawl isolation | P2 | 16h | Per-tenant rate limits, storage partitioning |

### Analyzers

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Integrate `HttpStatusAnalyzer::is_soft_404` | P1 | 4h | Dead method -- wire into analyze() |
| Integrate `ImageAnalyzer::detect_format` | P1 | 4h | Dead method -- wire into analyze() |
| Image format validation | P2 | 8h | Check for modern formats (WebP, AVIF) |
| Core Web Vitals via Lighthouse | P2 | 24h | Requires Chrome DevTools Protocol |

### Plugin System

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Implement `libloading` dynamic loading | P1 | 16h | Currently skeleton only |
| Plugin manifest validation | P2 | 8h | Schema validation, version compatibility |
| Plugin sandboxing | P2 | 16h | WASM-based plugin isolation |

---

## Phase 3: Performance and Scale (v0.7.0)

### Performance

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Connection pooling optimization | P1 | 8h | Tune `pool_max_idle_per_host` based on concurrency |
| Parallel analyzer execution | P1 | 8h | Rayon-based parallel dispatch |
| Streaming HTML parser | P2 | 24h | Parse while downloading (chunked encoding) |
| SIMD content hashing | P2 | 16h | For deduplication at scale |

### Scale

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Distributed crawl coordination | P2 | 40h | Redis-backed queue for multi-instance |
| Sharded SQLite storage | P2 | 24h | Partition by domain for >1M pages |
| Metrics export (Prometheus) | P1 | 8h | Expose `/metrics` endpoint |
| Structured logging (OpenTelemetry) | P1 | 8h | Replace tracing-subscriber with OTLP |

---

## Phase 4: Enterprise (v0.8.0)

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Webhook notifications | P1 | 8h | Crawl completion, threshold alerts |
| Scheduled crawls (cron) | P1 | 16h | API endpoint for recurring crawls |
| Team management UI | P2 | 40h | User invites, role assignment |
| Billing integration | P2 | 24h | Stripe, usage-based pricing |
| Audit log export | P1 | 8h | CSV/JSON export of audit trail |
| SSO implementation | P2 | 16h | SAML/OIDC via existing SsoManager |

---

## Phase 5: Documentation and Ecosystem (v0.9.0)

| Item | Priority | Effort | Status | Notes |
|------|----------|--------|--------|-------|
| Complete Starlight doc site | P0 | 16h | DONE | 13 pages, all sidebar items, builds clean |
| Rustdoc coverage > 80% | P1 | 8h | TODO | `cargo doc` warnings cleanup |
| crates.io publication | P0 | 4h | TODO | Version, description, keywords |
| Example library expansion | P1 | 8h | TODO | CI integration, custom storage, plugins |
| Benchmark regression CI | P1 | 4h | TODO | Criterion in CI, regression alerts |

---

## Phase 6: v1.0.0 Release

### Go/No-Go Criteria

- [ ] All P0 items from Phases 1-5 complete
- [ ] >95% branch coverage on critical paths
- [ ] Zero critical security vulnerabilities
- [ ] All CLI subcommands fully implemented
- [ ] Documentation site live and complete
- [ ] Performance: >50 pages/sec on reference hardware
- [ ] Memory: <500MB for 10k page crawl
- [ ] Cross-platform releases: Linux, macOS, Windows
- [ ] crates.io published with documentation

---

## Known Technical Debt

### Critical: Dead Code (~40% of modules)

The following modules are fully implemented but never wired into the crawl loop, CLI, or API. They represent scaffolding from earlier development phases. Decision required: wire them in or delete.

| Module | Status | Recommendation |
|--------|--------|----------------|
| `link_graph.rs` | Dead (divergent PageRank from backlinks.rs) | Delete; merge into backlinks |
| `plugin.rs` | Dead (TODO: libloading integration) | Delete or move to separate crate |
| `enterprise.rs` | Dead (RBAC/SSO/SLO never instantiated) | Delete or move to separate crate |
| `encryption.rs` | Dead (never instantiated) | Delete or feature-gate |
| `playwright.rs` | Dead (--javascript flag ignored) | Wire in or delete |
| `resource_monitor.rs` | Dead (CLI uses inline RSS check) | Replace inline check or delete |
| `backpressure.rs` | Dead (never instantiated) | Delete |
| `circuit_breaker.rs` | Dead (never instantiated) | Wire into HTTP client or delete |
| `determinism.rs` | Dead (never instantiated) | Delete |
| `observability.rs` | Dead (API uses Prometheus instead) | Delete |
| `dns.rs` | Dead (reqwest handles DNS internally) | Delete |
| `advanced_features.rs` | Dead (alerts/scheduler never used) | Delete |
| `js_render_decision.rs` | Dead (Playwright not wired) | Delete |
| `feature_flags.rs` | Dead (flags defined but never checked) | Wire in or delete |

### Resolved in This Audit

| Item | Status | Fix |
|------|--------|-----|
| Duplicate CrawlStats SQL | RESOLVED | export.rs now calls Storage::get_stats() |
| Duplicate Severity enums | RESOLVED | Unified to storage::Severity |
| Duplicate PageRank | OPEN | link_graph.rs divergent impl (see above) |

### Remaining Issues

| Item | Severity | Location | Notes |
|------|----------|----------|-------|
| `AuditTrail::clear()` breaks chain | Low | audit.rs | Remove or restrict access |
| `DnsCache` memory tracking inaccuracy | Low | dns.rs | Include Vec size in new_size calc |
| `BoundedPipeline::is_full` semantics | Low | backpressure.rs | Use try_send or proper capacity check |
| Error type erasure in CrawlError | Low | storage/export/compare | Add distinct CrawlError variants |
| `rate_limit` domain_buckets unbounded | Low | ratelimit.rs | Add LRU eviction for long-running crawls |
| N+1 query pattern in export.rs | Low | export.rs | Batch reads for CSV/JSON/HTML export |
| `circuit_breaker` TOCTOU race | Low | circuit_breaker.rs | State getter has side effects |
| Analyzer code duplication (14 patterns) | Low | analyzers.rs | Extract shared utilities (syllables, sentences, flesch) |
| `dns.rs` memory accounting race | Low | dns.rs | DashMap read-then-write window |
| `lru` crate unsound IterMut | Low | Cargo.lock | Track RUSTSEC-2026-0002, await fix |
