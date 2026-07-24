# crawlkit Technical Roadmap

Status: Living document. Updated after each audit cycle.

---

## Current State (v0.4.0)

- 430 tests passing, zero clippy warnings in production code, zero rustfmt diffs
- 28 analyzers (23 core + 4 AI + 1 WASM)
- CLI + REST API + HTML/JSON/CSV/Markdown export
- CI/CD: format, clippy -D warnings, tests, security audit, supply chain audit, release builds, MSRV check
- Pre-commit hook: fmt, clippy, build, tests, security audit, secret scan, unsafe code check
- Astro Starlight documentation site: 13 pages, 823 words indexed
- Supply chain security: cargo-deny for license compliance and advisory auditing

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

| Item | Severity | Location | Notes |
|------|----------|----------|-------|
| Duplicate PageRank implementations | Medium | link_graph.rs, backlinks.rs | Consolidate into single impl |
| Duplicate CrawlStats SQL | Medium | storage.rs, export.rs | export.rs should call Storage::get_stats |
| Duplicate Severity enums | Low | storage.rs, ai_bots.rs | Merge or clarify naming |
| `AuditTrail::clear()` breaks chain | Low | audit.rs | Remove or restrict access |
| `DnsCache` memory tracking inaccuracy | Low | dns.rs | Include Vec size in new_size calc |
| `BoundedPipeline::is_full` semantics | Low | backpressure.rs | Use try_send or proper capacity check |
| Error type erasure in CrawlError | Low | storage/export/compare | Add distinct CrawlError variants |
| `rate_limit` domain_buckets unbounded | Low | ratelimit.rs | Add LRU eviction for long-running crawls |
| `plugin.rs` load_plugin no dynamic loading | Low | plugin.rs | Requires libloading integration |
| N+1 query pattern in export.rs | Low | export.rs | Batch reads for CSV/JSON/HTML export |
| `circuit_breaker` TOCTOU race | Low | circuit_breaker.rs | State getter has side effects |
| Analyzer code duplication (14 patterns) | Low | analyzers.rs | Extract shared utilities (syllables, sentences, flesch) |
| `dns.rs` memory accounting race | Low | dns.rs | DashMap read-then-write window |
| `lru` crate unsound IterMut | Low | Cargo.lock | Track RUSTSEC-2026-0002, await fix |
