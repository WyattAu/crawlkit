# crawlkit Technical Roadmap

Status: Living document. Updated after each audit cycle.

Last Updated: 2026-07-25

---

## Current State (v2.0.0)

- 584 tests passing (412 engine + 83 API + 31 plugin SDK + 25 doc tests + 21 integration + 3 backlink + 3 playwright + 3 RUM + 2 SDK doc tests), zero clippy warnings, zero rustfmt diffs
- 31 analyzers (23 core + 4 AI + 1 WASM + 3 advanced canonical) with Rayon parallel execution
- CLI + REST API + HTML/JSON/CSV/Markdown export
- 12 modules wired into production (observability, resource_monitor, circuit_breaker, backpressure, determinism, feature_flags, js_render_decision, playwright, advanced_features, encryption, alert_manager, metrics)
- HTTP/2 multiplexing with connection pooling scaled to concurrency
- Prometheus metrics in API (crawlkit_crawls_total, pages_crawled, issues_found, fetch_duration, analysis_duration, errors)
- OpenTelemetry spans with structured attributes (url, depth, seed)
- WASM plugin system with wasmtime 47.0.2 (fuel limits, memory bounds, pointer validation)
- Multi-language client libraries (Go, Node.js, Python)
- React dashboard with Zustand state management, code splitting, WCAG 2.1 AA compliance
- Astro Starlight documentation site: 13 pages
- CI/CD: 10 jobs (format, clippy, unit tests, doc tests, integration tests, security audit, supply chain audit, fuzz test, release builds, MSRV 1.82.0)
- Pre-commit hook: 11 checks (fmt, clippy, build, unit tests, doc tests, integration tests, security audit, secret scan, unsafe code, MSRV, dead code)
- Lean4 formal verification scaffolding for circuit breaker, PageRank, and audit chain

### Audit Fixes Applied (2026-07-25)

| Category | Fix | Severity |
|----------|-----|----------|
| Security | WASM plugin sandbox: fuel limits (10B instructions), memory bounds (64MB), pointer validation | Critical |
| Security | Circuit breaker race condition: removed side-effecting state(), use CAS for atomic transitions | High |
| Security | DNS cache deadlock: drop DashMap read guard before insert | High |
| Security | Audit trail: include all fields in SHA-256 hash chain for tamper evidence | High |
| Security | HTTP body size enforcement: truncate chunks exceeding max_body_size | High |
| Security | Storage: replace unreachable!() panics with about:invalid fallback on corrupted URLs | High |
| Security | All GitHub Actions SHA-pinned to prevent supply-chain attacks | High |
| Security | CI workflows: least-privilege permissions blocks on all workflows | High |
| Correctness | Backpressure: remove unused channel allocation from with_channel() | High |
| Correctness | Resource monitor: enforce disk and open file limits in exceeded_limits() | High |
| Correctness | Wire tenant parameter to page_data.tenant_id in crawl loop | Medium |
| Correctness | Remove duplicate OG/Twitter checks from MetaTagAnalyzer (SOCIAL006/007 cover these) | Medium |
| Accessibility | Dashboard: aria-describedby on Input errors, skip-to-content link, aria-current on NavLinks | High |
| Accessibility | Dashboard: Escape key handler and focus return for mobile sidebar | High |
| Accessibility | Dashboard: sr-only loading text on all spinner elements | Medium |
| Accessibility | Dashboard: fix color contrast on severity badges (green-500->green-700, etc.) | Medium |
| Performance | Dashboard: React.lazy() code splitting for route components | Medium |
| Correctness | Dashboard: fix findings never loading in ResultsPage | High |
| Deduplication | Remove 5 dead dashboard files (websocket_service, auth_service, models) | Medium |
| Deduplication | Remove redundant enable_ai/enable_wasm fields from CrawlParams | Low |
| Infrastructure | Pre-commit hook: upgraded from 7 to 11 checks | Medium |
| Infrastructure | GUI snapshot testing script for dashboard DOM/screenshot capture | Low |
| Documentation | All 5 documentation files rewritten to technical standard (no emojis) | Medium |
| Formal Verification | Lean4 proof scaffolding for circuit breaker, PageRank, audit chain | Low |

---

## Phase 1: Production Hardening (v2.1.0)

### Critical Path

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Split analyzers.rs (8,400+ lines) into sub-modules | P0 | 16h | analyzers/http.rs, seo.rs, content.rs, security.rs, a11y.rs |
| Extract shared crawl loop into crawlkit-engine | P0 | 16h | CLI and API both implement crawl loops independently |
| Unify Plugin SDK types with Engine types | P0 | 8h | Finding, Severity, Analyzer trait duplicated with incompatible signatures |
| Implement compare subcommand completion | P1 | 16h | Currently returns stub in CLI |
| Normalize default max_pages (CLI=100, API=50) | P1 | 2h | Behavioral inconsistency |
| Complete client library parity (Go, Node.js) | P1 | 16h | Python is most complete |

### Security

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Add CSRF protection to dashboard API calls | P0 | 8h | JWT tokens exfiltrable via any XSS |
| Add CORS configuration to API | P1 | 2h | Currently no CORS headers |
| Add password complexity validation | P1 | 4h | No minimum requirements enforced |
| Implement CSP headers for dashboard | P1 | 4h | No Content Security Policy |

### Quality

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Type the API client (eliminate `as unknown as` casts) | P0 | 8h | All methods return Record<string, unknown> |
| Add React error boundaries | P1 | 4h | Component crashes white-screen the entire app |
| Define shadow/elevation tokens in dashboard globals.css | P1 | 4h | Spatial Materialism design system |
| Add transition system for page/state changes | P2 | 8h | Amoebic UI fluid transitions |

---

## Phase 2: Core Feature Completion (v2.2.0)

### Crawl Engine

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Incremental crawl support (ETag/If-Modified-Since) | P0 | 16h | Re-crawl only changed pages |
| JavaScript rendering integration into crawl loop | P1 | 24h | Wire Playwright when --javascript flag set |
| Multi-tenant crawl isolation | P2 | 16h | Per-tenant rate limits, storage partitioning |

### Analyzers

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Wire is_soft_404 into HttpStatusAnalyzer | P1 | 4h | Dead method exists but is never called |
| Wire detect_format into ImageAnalyzer | P1 | 4h | Dead method exists but is never called |
| Image format validation (WebP, AVIF) | P2 | 8h | Check for modern formats |
| Core Web Vitals via Lighthouse | P2 | 24h | Requires Chrome DevTools Protocol |

### Plugin System

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Implement libloading dynamic loading | P1 | 16h | Currently skeleton only |
| Plugin manifest validation | P2 | 8h | Schema validation, version compatibility |

---

## Phase 3: Performance and Scale (v2.3.0)

### Performance

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Connection pooling optimization | P1 | 8h | Tune pool_max_idle_per_host based on concurrency |
| Streaming HTML parser (parse while downloading) | P2 | 24h | Chunked encoding support |
| SQLite batch operations optimization | P1 | 8h | Currently loops insert_page |
| Rate limiter LRU eviction for long-running crawls | P1 | 4h | domain_buckets currently unbounded |

### Scale

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Distributed crawl coordination (Redis-backed queue) | P2 | 40h | Multi-instance support |
| Sharded SQLite storage (partition by domain) | P2 | 24h | For >1M pages |
| Export batch optimization (N+1 query elimination) | P1 | 8h | export.rs pattern |

---

## Phase 4: Enterprise (v2.4.0)

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Webhook notifications | P1 | 8h | Crawl completion, threshold alerts |
| Scheduled crawls (cron) | P1 | 16h | API endpoint for recurring crawls |
| Team management UI | P2 | 40h | User invites, role assignment |
| Audit log export (CSV/JSON) | P1 | 8h | Audit trail export |
| SSO implementation (SAML/OIDC) | P2 | 16h | Via existing SsoManager |

---

## Phase 5: Documentation and Ecosystem (v2.5.0)

| Item | Priority | Effort | Status | Notes |
|------|----------|--------|--------|-------|
| Starlight doc site | P0 | 16h | DONE | 13 pages, builds clean |
| Rustdoc coverage > 80% | P1 | 8h | TODO | cargo doc warnings cleanup |
| crates.io publication | P0 | 4h | TODO | Version, description, keywords |
| Example library expansion | P1 | 8h | TODO | CI integration, custom storage, plugins |
| Benchmark regression CI | P1 | 4h | DONE | benchmarks.yml with regression detection |

---

## Phase 6: v3.0.0 Release

### Go/No-Go Criteria

- All P0 items from Phases 1-5 complete
- >95% branch coverage on critical paths
- Zero critical security vulnerabilities
- All CLI subcommands fully implemented
- Documentation site live and complete
- Performance: >50 pages/sec on reference hardware
- Memory: <500MB for 10k page crawl
- Cross-platform releases: Linux, macOS, Windows (x86_64 + aarch64)
- crates.io published with documentation
- Client library parity across Go, Node.js, Python

---

## Known Technical Debt

### Module Status

| Module | Status | Notes |
|--------|--------|-------|
| `analyzers.rs` | 8,400+ lines, needs split | 23 analyzers + registry in single file |
| `plugin.rs` | HARDENED | Fuel limits, memory bounds, pointer validation |
| `enterprise.rs` | Operational | RBAC/tenant/SLA management in API |
| `encryption.rs` | WIRED | AES-256-GCM at rest |
| `playwright.rs` | WIRED | JS rendering with timeout/memory limits |
| `resource_monitor.rs` | WIRED | Runtime memory/CPU/disk/file monitoring |
| `backpressure.rs` | WIRED | Semaphore-based bounded pipeline |
| `circuit_breaker.rs` | HARDENED | Atomic CAS transitions, no side effects |
| `determinism.rs` | WIRED | Seed-based reproducibility |
| `observability.rs` | WIRED | Atomic metrics, zero-allocation hot paths |
| `dns.rs` | HARDENED | Deadlock-free concurrent DNS cache |
| `advanced_features.rs` | WIRED | Alert manager with threshold monitoring |
| `js_render_decision.rs` | WIRED | SPA framework detection |
| `feature_flags.rs` | WIRED | Runtime toggles via TOML |
| `advanced_canonical.rs` | WIRED | 3 analyzers for Ahrefs-level coverage |
| `post_crawl.rs` | WIRED | Cross-page canonical/sitemap analysis |

### Remaining Issues

| Item | Severity | Location | Notes |
|------|----------|----------|-------|
| CLI and API duplicate crawl loops | High | main.rs / api main.rs | Extract shared function into engine |
| Plugin SDK types diverge from Engine types | Medium | plugin-sdk / engine | Finding, Severity, Analyzer trait duplicated |
| API client uses `Record<string, unknown>` | Medium | dashboard/services/ | Forces `as unknown as` casts everywhere |
| analyzer.rs 8,400+ lines | Medium | analyzers.rs | Needs decomposition into sub-modules |
| rate_limit domain_buckets unbounded | Low | ratelimit.rs | Add LRU eviction for long crawls |
| Error type erasure in CrawlError | Low | storage/export/compare | Add distinct CrawlError variants |
