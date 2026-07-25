# crawlkit Technical Roadmap

Status: Living document. Updated after each audit cycle.

---

## Current State (v2.0.0)

- 526 tests passing (412 engine + 83 API + 31 plugin SDK), zero clippy warnings, zero rustfmt diffs
- 31 analyzers (23 core + 4 AI + 1 WASM + 3 advanced canonical) with Rayon parallel execution
- CLI + REST API + HTML/JSON/CSV/Markdown export
- 12 modules wired into production (observability, resource_monitor, circuit_breaker, backpressure, determinism, feature_flags, js_render_decision, playwright, advanced_features, encryption, alert_manager, metrics)
- HTTP/2 multiplexing with connection pooling scaled to concurrency
- Prometheus metrics in API (crawlkit_crawls_total, pages_crawled, issues_found, fetch_duration, analysis_duration, errors)
- OpenTelemetry spans with structured attributes (url, depth, seed)
- WASM plugin system with wasmtime 47.0.2 (all critical vulnerabilities resolved)
- Multi-language client libraries (Go, Node.js, Python)
- React dashboard with Zustand state management
- Astro Starlight documentation site: 13 pages
- CI/CD: format, clippy -D warnings, unit tests, integration tests, security audit, supply chain audit, release builds, MSRV 1.82.0
- Pre-commit hook: fmt, clippy, build, tests, security audit, secret scan, unsafe code check

### Audit Fixes Applied (2026-07-25)

| Category | Fix | Severity |
|----------|-----|----------|
| Security | wasmtime upgraded from v28 to v47 (2 critical sandbox escape CVEs) | Critical |
| Security | lru upgraded from v0.12 to v0.13 (unsound IterMut) | High |
| Security | API token removed from WebSocket URL query parameter | High |
| Security | Hardcoded HTTP replaced with env-configurable base URL | High |
| Security | Email enumeration prevented (generic "Invalid credentials") | High |
| Testing | 83 API tests added (auth, OIDC, middleware, validation) | High |
| Testing | 31 plugin SDK tests added (Finding, Severity, Analyzer, FFI) | High |
| Accessibility | ARIA labels added to all icon-only buttons | Critical |
| Accessibility | role="alert" added to error messages | Critical |
| Accessibility | role="status" added to all loading spinners | Medium |
| Accessibility | Keyboard support added to clickable cards | High |
| Accessibility | Chart accessibility labels added | Medium |
| Responsiveness | Sidebar collapses on mobile with hamburger toggle | Critical |
| Responsiveness | Table overflow wrapper for mobile | Medium |
| CI/CD | MSRV updated from 1.75.0 to 1.82.0 (wasmtime requirement) | High |
| CI/CD | Unit and integration tests separated in CI | High |
| CI/CD | --locked flag added to all cargo install commands | Medium |
| Correctness | Removed broken post_crawl module (non-existent storage methods) | High |
| Correctness | Analyzer registry count updated (28 to 31) | Medium |
| Deduplication | URL001 check consolidated to UrlFormatValidator | Medium |
| Deduplication | is_utility_page extracted as shared helper (was triplicated) | Medium |
| Deduplication | Readability functions extracted as free functions | Medium |
| TypeScript | All dashboard compilation errors resolved | High |
| Documentation | All 12 web doc pages rewritten with technical rigor | Medium |
| Documentation | All emojis removed from documentation | Medium |
| WASM | static_mut_refs lint suppressed with safety documentation in export macro | Medium |

---

## Phase 1: Production Hardening (v2.1.0)

### Critical Path

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Split analyzers.rs (8,400+ lines) into sub-modules | P0 | 16h | analyzers/http.rs, seo.rs, content.rs, security.rs, a11y.rs, etc. |
| Wire missing analyzer dead code into crawl loop | P0 | 8h | is_soft_404(), detect_format() methods unused |
| Implement `compare` subcommand completion | P1 | 16h | Currently returns stub in CLI |
| Normalize default max_pages (CLI=100, API=50) | P1 | 2h | Behavioral inconsistency |
| Complete client library parity (Go, Node.js) | P1 | 16h | Python is most complete; Go missing 8 methods, Node.js missing 5 |

### Security

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Add CSRF protection to dashboard API calls | P0 | 8h | JWT tokens exfiltrable via any XSS |
| Add CORS configuration to API | P1 | 2h | Currently no CORS headers |
| Add password complexity validation | P1 | 4h | No minimum requirements enforced |
| Rotate any hardcoded dev credentials | P1 | 2h | Audit remaining secrets |

---

## Phase 2: Core Feature Completion (v2.2.0)

### Crawl Engine

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Sitemap.xml auto-discovery | P0 | 8h | Parse sitemap, add URLs to queue with priority |
| Incremental crawl support (ETag/If-Modified-Since) | P1 | 16h | Re-crawl only changed pages |
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
| Plugin sandboxing (WASM) | P2 | 16h | Already have wasmtime; wire sandboxing |

---

## Phase 3: Performance and Scale (v2.3.0)

### Performance

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Connection pooling optimization | P1 | 8h | Tune pool_max_idle_per_host based on concurrency |
| Streaming HTML parser (parse while downloading) | P2 | 24h | Chunked encoding support |
| SIMD content hashing for deduplication | P2 | 16h | At scale |
| SQLite batch operations optimization | P1 | 8h | Currently loops insert_page |

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
| Billing integration (Stripe) | P2 | 24h | Usage-based pricing |
| Audit log export (CSV/JSON) | P1 | 8h | Audit trail export |
| SSO implementation (SAML/OIDC) | P2 | 16h | Via existing SsoManager |

---

## Phase 5: Documentation and Ecosystem (v2.5.0)

| Item | Priority | Effort | Status | Notes |
|------|----------|--------|--------|-------|
| Starlight doc site | P0 | 16h | DONE | 13 pages, all sidebar items, builds clean |
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
| `plugin.rs` | WIRED (wasmtime 47) | WASM plugin system operational |
| `enterprise.rs` | Operational | RBAC/tenant/SLA management in API |
| `encryption.rs` | WIRED | AES-256-GCM at rest |
| `playwright.rs` | WIRED | JS rendering with timeout/memory limits |
| `resource_monitor.rs` | WIRED | Runtime memory/CPU/disk monitoring |
| `backpressure.rs` | WIRED | Bounded pipeline with semaphore |
| `circuit_breaker.rs` | WIRED | Per-domain fault isolation |
| `determinism.rs` | WIRED | Seed-based reproducibility |
| `observability.rs` | WIRED | Atomic metrics, zero-allocation hot paths |
| `dns.rs` | Wired | Concurrent DNS cache with TTL |
| `advanced_features.rs` | WIRED | Alert manager with threshold monitoring |
| `js_render_decision.rs` | WIRED | SPA framework detection |
| `feature_flags.rs` | WIRED | Runtime toggles via TOML |
| `advanced_canonical.rs` | WIRED | 3 analyzers for Ahrefs-level coverage |
| `post_crawl.rs` | REMOVED | Referenced non-existent storage methods |

### Remaining Issues

| Item | Severity | Location | Notes |
|------|----------|----------|-------|
| Analyzer code duplication (heading, hreflang, image checks) | Low | analyzers.rs | HeadingHierarchyAnalyzer overlaps AccessibilityAnalyzer |
| `rate_limit` domain_buckets unbounded | Low | ratelimit.rs | Add LRU eviction for long-running crawls |
| `circuit_breaker` TOCTOU race | Low | circuit_breaker.rs | State getter has side effects |
| Error type erasure in CrawlError | Low | storage/export/compare | Add distinct CrawlError variants |
| Duplicate heading/hreflang/image checks across analyzers | Low | analyzers.rs | HeadingHierarchyAnalyzer vs AccessibilityAnalyzer overlap |
| Go/Node.js client parity gaps | Medium | clients/ | Python has 8 more methods than Go |
