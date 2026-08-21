# crawlkit Technical Roadmap

Status: Living document. Updated after each audit cycle.

Last Updated: 2026-07-26

---

## Current State (v2.1.0)

- 736 tests passing (614 unit+bins, 81 integration incl. 19 router-level + 7 WASM-ABI tests, 43 doc; 6 ignored), plus 49 client-SDK tests and 41 dashboard tests, zero clippy warnings, zero rustfmt diffs
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
- CI/CD: 10 jobs (format, clippy, unit tests, doc tests, integration tests, security audit, supply chain audit, fuzz test, release builds, MSRV 1.94.0)
- Pre-commit hook: 11 checks (fmt, clippy, build, unit tests, doc tests, integration tests, security audit, secret scan, unsafe code, MSRV, dead code)
- (deferred) formal verification of queue/circuit-breaker invariants — not started
- Property-based testing with proptest (21 tests across parser, storage, queue, rate limiter, meta)
- Typed TypeScript API client (zero `as unknown as` casts)
- CSRF origin validation on state-changing API requests
- Password complexity validation (12-char minimum, mixed case, digits, special chars)

### Audit Fixes Applied (2026-07-26)

| Category | Fix | Severity |
|----------|-----|----------|
| Security | Replace hardcoded admin password with cryptographically random generation | Critical |
| Security | Redact API key secrets in list endpoints (show last 4 chars only) | Critical |
| Security | Convert hash_password from panic-on-failure to Result return type | Critical |
| Security | Add CSRF origin validation middleware on state-changing API requests | Critical |
| Security | Split router into public/protected; health/metrics/login no longer require auth | High |
| Security | Add password complexity validation (12-char minimum, mixed case, digits, special) | High |
| Security | WASM plugin sandbox: fuel limits (10B instructions), memory bounds (64MB), pointer validation | Critical |
| Security | Circuit breaker race condition: removed side-effecting state(), use CAS for atomic transitions | High |
| Security | DNS cache deadlock: drop DashMap read guard before insert | High |
| Security | Audit trail: include all fields in SHA-256 hash chain for tamper evidence | High |
| Security | HTTP body size enforcement: truncate chunks exceeding max_body_size | High |
| Security | Storage: replace unreachable!() panics with about:invalid fallback on corrupted URLs | High |
| Security | All GitHub Actions SHA-pinned to prevent supply-chain attacks | High |
| Security | CI workflows: least-privilege permissions blocks on all workflows | High |
| CI/CD | Migrate deny.toml to cargo-deny 0.19+ format (removed deprecated keys) | High |
| CI/CD | Replace cargo-deny/tarpaulin/fuzz install-from-source with SHA-pinned actions | High |
| CI/CD | Add test-integration to build job dependencies | High |
| CI/CD | Unify MSRV to 1.85.0 across Cargo.toml, CI, justfile, pre-commit | High |
| Correctness | Deduplicate storage.rs: extract row_to_page_data/row_to_issue helpers | High |
| Correctness | Deduplicate issues query: extract build_issues_query/execute_issues_query | High |
| Correctness | Replace .unwrap_or(None) with match + tracing::warn for DB errors | High |
| Correctness | Replace let _ = with proper error logging on finish_crawl | Medium |
| Correctness | Remove unnecessary .clone() on page_data.title/description | Medium |
| Correctness | Fix missing BingWebmasterAdapter in backlink registry | Medium |
| Correctness | Add from_env() to BingWebmasterAdapter | Medium |
| Correctness | Fix query_tracker GscSearchResult field name (key vs query) | Medium |
| Correctness | Add missing ctr field to QueryWithPosition | Medium |
| Correctness | Remove duplicate URL001 check between AdvancedCanonicalAnalyzer and UrlFormatValidator | Medium |
| Correctness | Remove hardcoded domain from article_generator meta description | Medium |
| Correctness | Remove unused async from query_tracker methods | Low |
| Correctness | Fix pre-commit regex and MSRV version | Low |
| Correctness | Fix gui_snapshot_test.sh r.route -> r.path bug | Low |
| Testing | Add 21 property-based tests (proptest) for parser, storage, queue, rate limiter, meta | High |
| Testing | Add 13 password validation tests | Medium |
| UI/UX | Wire elevation/transition/spacing design tokens into Tailwind config | Medium |
| UI/UX | Add prefers-reduced-motion media query for vestibular accessibility | Medium |
| UI/UX | Replace inline style in Card.tsx with Tailwind shadow-elevation1 classes | Medium |
| UI/UX | Add focus-visible ring to ErrorBoundary button | Low |
| UI/UX | Update use_crawls hook to accept full CrawlConfig shape | Low |
| UI/UX | Remove unused @radix-ui deps (dropdown-menu, select, tabs, tailwind-merge) | Low |
| Dashboard | Type the API client: 15 new typed methods, zero `as unknown as` casts | High |
| Dashboard | Add 9 new TypeScript interfaces for API request/response types | Medium |
| Dashboard | Fix FindingCard to use corrected Finding fields (category, code, element, recommendation) | Medium |
| Documentation | README rewritten: updated test count (642), MSRV (1.85.0), added quality gates section | Medium |
| Documentation | VERSION.md updated with completed phases | Low |

---

## Phase 1: Production Hardening (v2.1.0)

### Critical Path

| Item | Priority | Effort | Notes | Status |
|------|----------|--------|-------|--------|
| Split analyzers.rs (8,400+ lines) into sub-modules | P0 | 16h | analyzers/http.rs, seo.rs, content.rs, security.rs, a11y.rs | DONE |
| Extract shared crawl loop into crawlkit-engine | P0 | 16h | CLI and API both implement crawl loops independently | DONE |
| Unify Plugin SDK types with Engine types | P0 | 8h | Finding, Severity, Analyzer trait duplicated with incompatible signatures | DONE |
| Deduplicate storage.rs (get_pages/get_issues) | P0 | 8h | Extracted row_to_page_data and build_issues_query helpers | DONE |
| Implement compare subcommand completion | P1 | 16h | Currently returns stub in CLI | DONE |
| Normalize default max_pages (CLI=100, API=50) | P1 | 2h | Behavioral inconsistency | DONE |
| Complete client library parity (Go, Node.js) | P1 | 16h | Python is most complete | TODO |

### Security

| Item | Priority | Effort | Notes | Status |
|------|----------|--------|-------|--------|
| Replace hardcoded admin password with random generation | P0 | 4h | Was 'admin123' | DONE |
| Redact API keys in list endpoints | P0 | 4h | Shows full secret keys | DONE |
| Add CSRF protection to dashboard API calls | P0 | 8h | JWT tokens exfiltrable via any XSS | DONE |
| Split router: public routes (health/metrics/login) skip auth | P0 | 4h | Auth middleware was on all routes | DONE |
| Add password complexity validation | P0 | 4h | 12-char minimum, mixed case, digits, special | DONE |
| Add CORS configuration to API | P1 | 2h | Currently no CORS headers | TODO |
| Implement CSP headers for dashboard | P1 | 4h | No Content Security Policy | TODO |

### Quality

| Item | Priority | Effort | Notes | Status |
|------|----------|--------|-------|--------|
| Wire design tokens into Tailwind config | P0 | 4h | Spatial Materialism design system | DONE |
| Add prefers-reduced-motion support | P1 | 2h | WCAG accessibility | DONE |
| Remove unused dashboard dependencies | P1 | 2h | 4 unused @radix-ui packages | DONE |
| Type the API client (eliminate `as unknown as` casts) | P0 | 8h | All methods return Record<string, unknown> | DONE |
| Add React error boundaries | P1 | 4h | Component crashes white-screen the entire app | DONE |
| Add property-based testing (proptest) | P0 | 8h | 21 tests: parser, storage, queue, rate limiter, meta | DONE |
| Add transition system for page/state changes | P2 | 8h | Amoebic UI fluid transitions | TODO |

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
| crates.io publication | P0 | 4h | DONE | v2.0.0 already published |
| Example library expansion | P1 | 8h | TODO | CI integration, custom storage, plugins |
| Benchmark regression CI | P1 | 4h | DONE | benchmarks.yml with regression detection |
| Benchmark baseline establishment | P1 | 4h | DONE | Benchmarks compile, baseline on reference hardware |

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
- Property-based test coverage for all core modules

---

## Known Technical Debt

### Module Status

| Module | Status | Notes |
|--------|--------|-------|
| `analyzers/` | SPLIT | 23 analyzers in sub-modules |
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
| `storage.rs` | DEDUPLICATED | Shared row mappers and query builders |
| `property_tests.rs` | NEW | 21 property-based tests with proptest |

### Remaining Issues

| Item | Severity | Location | Notes |
|------|----------|----------|-------|
| Plugin SDK types diverge from Engine types | Medium | plugin-sdk / engine | Finding, Severity, Analyzer trait duplicated |
| rate_limit domain_buckets unbounded | Low | ratelimit.rs | Add LRU eviction for long crawls |
| Error type erasure in CrawlError | Low | storage/export/compare | Add distinct CrawlError variants |
| BingWebmasterAdapter trait stubs | Low | backlink_adapters.rs | fetch_backlinks/get_domain_rating return empty |
| query_tracker trend always Stable | Low | query_tracker.rs | No actual trend calculation over time |
| CSP headers not implemented | Medium | API/dashboard | Content Security Policy missing |
| CORS configuration missing | Medium | API | No CORS headers configured |
