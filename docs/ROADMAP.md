# crawlkit — Development Roadmap

**crawlkit** is a high-performance Rust-based site crawler and SEO analysis toolkit.

---

## Phase 0: Foundation (Week 1–2)

**Goal:** Functional crawl loop — fetch pages, follow links, store results, expose CLI.

### Tasks

| # | Task | Depends On | Acceptance Criteria |
|---|------|------------|---------------------|
| 0.1 | Cargo workspace setup | — | Workspace compiles clean on `cargo build --workspace`; `rustfmt.toml` and `clippy.toml` committed |
| 0.2 | CI/CD scaffold (GitHub Actions) | 0.1 | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` all pass on push |
| 0.3 | README with install + usage | 0.1 | README covers build, install, basic example; renders without errors |
| 0.4 | Core HTTP client | 0.1 | Fetches arbitrary URLs with configurable timeout, retries, and User-Agent; handles TLS, redirects, HTTP/2 |
| 0.5 | URL queue (BFS/fIFO) | — | Deduplicates URLs by canonical form; respects `robots.txt` disallow rules (Phase 0 basic, Phase 1 full) |
| 0.6 | Rate limiter | 0.5 | Token-bucket limiter per domain; configurable concurrency and delay; no request fired after limit |
| 0.7 | Basic HTML parser (link extraction) | 0.4 | Extracts `href` from `<a>`, `<link>`, `<area>`; normalizes relative URLs; output links for queue feed |
| 0.8 | Meta tag extraction | 0.7 | Parses `<title>`, `<meta name="description">`, canonical `<link rel="canonical">`, OG tags, Twitter tags |
| 0.9 | SQLite storage layer | — | `pages` and `links` tables created via migrations; insert, query, upsert all work; concurrent-safe (`r2d2` or `sqlx` pool) |
| 0.10 | CLI framework (clap) | 0.4, 0.9 | `crawlkit crawl <url>` runs a crawl; flags: `--max-pages`, `--delay`, `--concurrency`, `--output` |

### Dependency Graph

```
0.1
├─ 0.2
├─ 0.3
├─ 0.4
│   ├─ 0.7
│   │   └─ 0.8
│   └─ 0.10
├─ 0.5
│   ├─ 0.6
│   └─ 0.10
└─ 0.9
    └─ 0.10
```

---

## Phase 1: Core Analyzers (Week 3–4)

**Goal:** First-class SEO signal extraction. Every analyzer outputs a typed struct stored in SQLite.

### Tasks

| # | Task | Depends On | Acceptance Criteria |
|---|------|------------|---------------------|
| 1.1 | HTTP status analyzer | 0.9 | Categorizes responses into 2xx/3xx/4xx/5xx buckets; records status code, headers, response time |
| 1.2 | Redirect chain tracker | 1.1 | Records full redirect chain (`301` → `302` → `200`); flags chains > 5 hops or loops |
| 1.3 | Canonical URL validator | 0.8 | Compares `<link rel="canonical">` to final URL; flags mismatches, self-referencing, missing canonical |
| 1.4 | Hreflang validator | 0.8 | Parses `hreflang` annotations; verifies reciprocal links; flags missing/incorrect locale codes |
| 1.5 | Sitemap analyzer | 0.4 | Fetches and parses XML sitemaps (including sitemap index); validates `<lastmod>`, `<changefreq>`, `<priority>`; compares discovered vs. listed URLs |
| 1.6 | Robots.txt analyzer | 0.4 | Parses directives per user-agent; enforces `Crawl-delay`, `Disallow`, `Allow`; validates sitemap reference |
| 1.7 | Meta tag analyzer | 0.8 | Validates title length (30–60 chars), description length (120–160 chars), OG completeness (title, image, url, type), Twitter Card completeness |
| 1.8 | Heading hierarchy analyzer | 0.7 | Validates `<h1>` exists exactly once; flags skipped heading levels (e.g., `<h1>` → `<h3>`) |

### Dependency Graph

```
0.8
├─ 1.3
├─ 1.4
└─ 1.7

0.4
├─ 1.5
└─ 1.6

0.7
└─ 1.8

1.1
└─ 1.2

0.9 (all analyzers write results)
```

---

## Phase 2: Content Analysis (Week 5–6)

**Goal:** Deep content quality signals — links, images, structured data, readability.

### Tasks

| # | Task | Depends On | Acceptance Criteria |
|---|------|------------|---------------------|
| 2.1 | Internal link analyzer | 0.7, 0.9 | Counts internal vs. external links per page; flags broken links (4xx/5xx); identifies orphan pages (0 inbound) |
| 2.2 | External link analyzer | 2.1 | Validates external links via HEAD request; records response code, `nofollow` attribute, anchor text |
| 2.3 | Orphan page detector | 0.5, 2.1 | Compares crawled URLs against inbound-link index; returns set of pages with zero internal inbound links |
| 2.4 | Image analyzer | 0.7 | Extracts all `<img>` tags; records `src`, `alt`, `width`, `height`, format, file size; flags missing `alt`, oversized images (> 500 KB) |
| 2.5 | Structured data validator (JSON-LD) | 0.7 | Parses `<script type="application/ld+json">`; validates against Schema.org types; reports errors, warnings |
| 2.6 | Structured data validator (Microdata) | 0.7 | Parses `itemscope`/`itemprop` attributes; same validation as 2.5 |
| 2.7 | Content quality analyzer | — | Computes Flesch-Kincaid readability score, keyword density (top 10 terms), content-to-markup ratio |
| 2.8 | Word count & content length | 0.7 | Counts visible text words, characters, sentences; stores per page |

### Dependency Graph

```
0.7
├─ 2.1
│   ├─ 2.2
│   └─ 2.3
├─ 2.4
├─ 2.5
├─ 2.6
└─ 2.8

0.9
└─ (all analyzers)

2.7 (independent — reads page body text)
```

---

## Phase 3: Security & Performance (Week 7–8)

**Goal:** Security posture and performance metrics.

### Tasks

| # | Task | Depends On | Acceptance Criteria |
|---|------|------------|---------------------|
| 3.1 | Security header analyzer | 1.1 | Inspects `Content-Security-Policy`, `Strict-Transport-Security`, `X-Frame-Options`, `X-Content-Type-Options`, `Referrer-Policy`, `Permissions-Policy`; scores completeness |
| 3.2 | SSL certificate validator | 0.4 | Connects via TLS; validates cert chain, expiry, subject/SAN match; flags self-signed, expired, soon-to-expire (< 30 days) |
| 3.3 | Core Web Vitals (Lighthouse API) | 0.4 | Calls Lighthouse API (or local Chrome DevTools) for LCP, FID, CLS; stores per-page metrics |
| 3.4 | Page speed analyzer | 3.3 | Computes TTFB, TTI, total bundle size; breaks down by resource type (JS, CSS, image, font) |
| 3.5 | Mobile-friendliness checker | 0.7 | Validates viewport meta, tap targets, font sizing, responsive layout hints |

### Dependency Graph

```
0.4
├─ 3.2
└─ 3.3
    └─ 3.4

1.1
└─ 3.1

0.7
└─ 3.5
```

---

## Phase 4: Advanced Features (Week 9–10)

**Goal:** Extensibility, social signals, and comparison capabilities.

### Tasks

| # | Task | Depends On | Acceptance Criteria |
|---|------|------------|---------------------|
| 4.1 | Accessibility analyzer (WCAG 2.1) | 0.7, 2.4 | Checks alt text coverage, ARIA attributes, color contrast (via headless Chrome), focus order, semantic HTML usage |
| 4.2 | Social media analyzer | 0.8 | Validates OG image dimensions (≥ 1200×630), Twitter Card type presence, social preview rendering |
| 4.3 | Crawl comparison (before/after) | 0.9 | Stores crawl snapshots; diff tool shows new/removed/changed pages, status-code shifts, score deltas |
| 4.4 | Plugin system for custom analyzers | — | Trait-based plugin API; load `.so`/`.dylib` or WASM plugins; register analyzer, receive page data, return findings |
| 4.5 | Scheduled crawls | 0.10, 4.3 | Cron-style scheduling via CLI or config; runs crawl, stores snapshot, computes diff against previous |

### Dependency Graph

```
0.7, 2.4
└─ 4.1

0.8
└─ 4.2

0.9
└─ 4.3
    └─ 4.5

0.10
└─ 4.5

4.4 (independent — API design)
```

---

## Phase 5: Export & Reporting (Week 11–12)

**Goal:** Multiple output formats and a monitoring dashboard.

### Tasks

| # | Task | Depends On | Acceptance Criteria |
|---|------|------------|---------------------|
| 5.1 | CSV export | 0.9 | Exports any analyzer's results to CSV with correct headers and escaping; configurable columns |
| 5.2 | JSON export | 0.9 | Full JSON dump of crawl results; schema-versioned; streamable for large crawls |
| 5.3 | HTML report (interactive) | 0.9, 5.2 | Single-file HTML with embedded charts (Chart.js or similar); summary dashboard, per-page detail, filterable |
| 5.4 | Markdown summary | 0.9 | Auto-generated MD with key metrics: total pages, broken links, avg load time, SEO score distribution |
| 5.5 | SQLite query interface | 0.9 | Exposes read-only SQLite access; ships pre-built queries for common analyses; documented schema |
| 5.6 | Dashboard for monitoring | 0.9, 5.2 | Web UI (could be static HTML + API) showing crawl progress, live stats, historical trends |

### Dependency Graph

```
0.9
├─ 5.1
├─ 5.2
│   └─ 5.3
├─ 5.4
├─ 5.5
└─ 5.6
```

---

## Phase 6: Polish & Release (Week 13–14)

**Goal:** Production readiness, documentation, and v1.0 release.

### Tasks

| # | Task | Depends On | Acceptance Criteria |
|---|------|------------|---------------------|
| 6.1 | Performance profiling & optimization | All phases | Flamegraph-identified hotspots optimized; throughput ≥ 50 pages/sec on reference hardware |
| 6.2 | Memory profiling & optimization | 6.1 | Peak RSS < 500 MB for 10,000-page crawl; no leaks detected via `valgrind` / `miri` |
| 6.3 | Documentation: README overhaul | All phases | Install, quickstart, CLI reference, configuration, architecture diagram |
| 6.4 | Documentation: API docs (rustdoc) | All phases | All public items documented; `cargo doc --no-deps` passes warning-free |
| 6.5 | Documentation: examples | 0.4, 0.9 | ≥ 5 runnable examples (basic crawl, custom analyzer, export, plugin, comparison) |
| 6.6 | Binary releases (cross-compile) | 0.1 | `release.yml` produces Linux (glibc + musl), macOS (Intel + ARM), Windows; each < 10 MB |
| 6.7 | GitHub Actions CI/CD full pipeline | 0.2 | lint → test → build → release → publish; triggered on tag push |
| 6.8 | Beta testing round | 6.7 | ≥ 3 external testers; issue tracker with resolved bugs |
| 6.9 | v1.0.0 release | 6.8 | CHANGELOG, git tag, GitHub Release with binaries, crates.io publish |

### Dependency Graph

```
All phases
├─ 6.1
│   └─ 6.2
├─ 6.3
├─ 6.4
├─ 6.5
└─ 0.1
    └─ 6.6
        └─ 6.7
            └─ 6.8
                └─ 6.9
```

---

## Phase 7: Advanced Integrations (Week 15–18)

**Goal:** JavaScript rendering, external data integrations, programmatic API access, and standards compliance hardening.

### Tasks

| # | Task | Depends On | Acceptance Criteria |
|---|------|------------|---------------------|
| 7.1 | Opt-in Playwright JS rendering | 0.4, 3.5 | `--javascript` flag enables Chromium rendering; resource warnings on activation; falls back to HTTP-only if unavailable; memory isolation per browser context |
| 7.2 | JS render decision engine | 7.1 | Auto-detects SPA indicators (div#app, framework signatures); user-configurable URL patterns for JS rendering |
| 7.3 | REST API server (Axum) | 0.4, 0.9 | `crawlkit serve` starts HTTP server; endpoints: POST /crawl, GET /crawl/:id, GET /crawl/:id/results, DELETE /crawl/:id, GET /health, GET /docs |
| 7.4 | API key authentication | 7.3 | Keys stored hashed in SQLite; X-API-Key header; argon2id hashing; key lifecycle (create, revoke, list) |
| 7.5 | Per-key rate limiting | 7.4 | Token bucket per API key; 429 with Retry-After; configurable burst + sustained limits |
| 7.6 | OpenAPI documentation | 7.3 | Auto-generated via utoipa; Swagger UI at /api/v1/docs; valid Swagger 3.0 JSON |
| 7.7 | Ahrefs backlink adapter | 7.3 | Fetches backlink data via Ahrefs API; rate limited; graceful degradation if unavailable |
| 7.8 | Majestic backlink adapter | 7.7 | Same interface as 7.7; Majestic API integration |
| 7.9 | Google Search Console adapter | 7.7 | Fetches own-site link data; OAuth2 authentication; respects API quotas |
| 7.10 | Internal link graph builder | 0.7, 2.1 | Directed graph from crawl data; PageRank computation (damping 0.85); orphan detection |
| 7.11 | Link graph visualization | 7.10 | DOT export (Graphviz); HTML interactive (D3.js force-directed); CSV adjacency list |
| 7.12 | Google Analytics RUM import | 7.3 | Reporting API v4 integration; maps page paths to crawl URLs; 28-day aggregation window |
| 7.13 | CrUX field data integration | 7.12 | PageSpeed Insights API or BigQuery; LCP, INP, CLS, FCP, TTFB p75 values |
| 7.14 | Merged lab + field report | 7.12, 7.13 | Displays lab and field data side-by-side; highlights significant deltas; priority scoring by real-user impact |
| 7.15 | Feature flag system | 7.1, 7.3, 7.7, 7.12 | TOML config for runtime feature toggles; immutable per-crawl session |
| 7.16 | Observability stack | 7.3 | tracing-subscriber (structured JSON logs); metrics + prometheus exporter; OpenTelemetry trace export |
| 7.17 | Audit trail | 7.3 | Append-only audit log; every state-change event logged with timestamp, config hash, details |
| 7.18 | Circuit breaker (per-domain) | 0.4 | Opens after 5 consecutive failures; half-open after 60s cooldown; configurable thresholds |
| 7.19 | Backpressure (bounded channels) | 0.4 | All pipeline channels bounded; producer blocks when consumer full; semaphore for concurrency |
| 7.20 | Encryption at rest (optional) | 0.9 | SQLCipher for SQLite; AES-256-GCM for export files; key from file/env/keyring |
| 7.21 | Determinism guarantees | 0.4 | Same URL + same config → same output; seed-based PRNG; deterministic URL hashing |
| 7.22 | Resource isolation | 7.1 | Per-crawl memory/disk/CPU budgets; graceful abort if exceeded |

### Dependency Graph

```
0.4, 0.9
├─ 7.3
│   ├─ 7.4
│   │   └─ 7.5
│   ├─ 7.6
│   ├─ 7.7
│   │   ├─ 7.8
│   │   └─ 7.9
│   ├─ 7.12
│   │   └─ 7.13
│   │       └─ 7.14
│   ├─ 7.15 (depends on 7.1, 7.3, 7.7, 7.12)
│   ├─ 7.16
│   └─ 7.17
│
0.4
├─ 7.1
│   ├─ 7.2
│   └─ 7.22
├─ 7.18
└─ 7.19
│
0.7, 2.1
└─ 7.10
    └─ 7.11
│
0.9
└─ 7.20
│
0.4
└─ 7.21
```

### Standards Compliance Tracks

Phase 7 includes cross-cutting standards compliance work. These tasks are tracked here but may be implemented incrementally across other Phase 7 tasks.

#### FAANG Track

| Task | Acceptance Criteria |
|------|---------------------|
| Code review process | All PRs require ≥ 1 approval; security changes require ≥ 2 |
| Feature flags (7.15) | Runtime toggles for JS rendering, API mode, backlink analysis, RUM integration |
| Rollback strategy | Crawl snapshots immutable; compare command for before/after diff; SQLite backups before migration |
| Observability (7.16) | Structured JSON logs, Prometheus metrics, OpenTelemetry traces |

#### HFT Track

| Task | Acceptance Criteria |
|------|---------------------|
| Determinism (7.21) | Same input → same output; seed-based PRNG; no randomized behavior without explicit seed |
| Reliability targets | 99.9% crawl completion rate; circuit breaker prevents cascade failures |
| Resource isolation (7.22) | Per-crawl memory/CPU/disk budgets; browser context isolation |
| Benchmark regression | CI detects > 5% throughput regression on reference hardware |

#### Defense Track

| Task | Acceptance Criteria |
|------|---------------------|
| Audit trail (7.17) | Append-only log; every state-change event recorded; tamper-evident chaining |
| Input validation | All URLs validated; depth ≤ 20; page limit ≥ 1; patterns validated |
| Encryption at rest (7.20) | SQLCipher optional; AES-256-GCM exports; key never hardcoded |
| Dependency auditing | `cargo audit` + `cargo deny` in CI |

#### ECN Track

| Task | Acceptance Criteria |
|------|---------------------|
| Backpressure (7.19) | Bounded channels; producer blocks when consumer full; semaphore concurrency |
| Circuit breaker (7.18) | Opens after 5 failures; half-open after 60s; per-domain isolation |
| Idempotency | URL + status + content hash as key; skip re-crawl if unchanged within TTL |
| Exactly-once | Best-effort via idempotency; documented trade-off |

---

## Milestones

| Milestone | Target Date | Deliverable | Exit Criteria |
|-----------|-------------|-------------|---------------|
| **v0.1.0** | Week 2 | Basic crawler with HTTP status + link extraction | Crawls a site, stores pages in SQLite, outputs link graph |
| **v0.2.0** | Week 4 | Core SEO analyzers | Status, redirect, canonical, hreflang, sitemap, robots, meta, heading analyzers all functional |
| **v0.3.0** | Week 6 | Content + image analysis | Link analyzer, structured data, readability, image audit all pass integration tests |
| **v0.4.0** | Week 8 | Security + performance analysis | Security headers, SSL validation, Core Web Vitals, mobile check all produce reports |
| **v0.5.0** | Week 10 | Advanced features + plugins | WCAG analysis, social audit, crawl diff, plugin loading all verified |
| **v0.6.0** | Week 12 | Export + reporting | CSV, JSON, HTML, MD exports; dashboard; SQLite query interface |
| **v0.7.0** | Week 18 | Advanced integrations + standards compliance | JS rendering, REST API, backlinks, RUM, observability, audit trail, circuit breaker, encryption |
| **v1.0.0** | Week 20 | Full release | Cross-platform binaries, complete docs, ≥ 90% coverage, beta feedback resolved, standards compliance ≥ 85% |

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Pages per second** | ≥ 50 | Crawling 10k pages on reference hardware (8-core, 16 GB RAM) |
| **Memory usage** | < 500 MB | Peak RSS during 10k-page crawl (valgrind / `dhat`) |
| **Test coverage** | > 90% | `cargo tarpaulin` report |
| **Documentation** | Complete | All public APIs documented; ≥ 5 examples; architecture diagram in README |
| **Binary size** | < 10 MB | `ls -lh` on release binary (Linux x86_64) |
| **Startup time** | < 100 ms | Time from process start to first HTTP request (measured via `strace -T`) |

---

## Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| HTML parsing edge cases (malformed pages) | High — missed links, incorrect meta extraction | Medium | Use `lol_html` (Servo's streaming parser); fuzz with `cargo-fuzz` |
| Rate limiting complexity (per-domain, concurrent) | Medium — blocked by target sites | Medium | Implement token-bucket with jitter; configurable per-domain limits |
| SQLite contention under high concurrency | Medium — slow writes, deadlocks | Low | Use WAL mode; batch writes; `r2d2` connection pool |
| Cross-compilation issues (Windows, macOS ARM) | Low — CI failures | Medium | Use `cross` for Linux targets; matrix build for macOS/Windows |
| Plugin system scope creep | High — delays v1.0 | Medium | Defer WASM plugins to v1.1; native `.so`/`.dylib` only for v1.0 |
| Playwright dependency bloat | Medium — binary size, memory | Medium | Opt-in only; document resource warnings; fallback to HTTP-only |
| External API rate limits (Ahrefs, Majestic) | Medium — incomplete data | Low | Graceful degradation; cache responses; configurable rate limits |
| API authentication security | High — unauthorized access | Low | Argon2id hashing; API key rotation; audit logging |
| Encryption key management | High — data exposure | Low | Support file/env/keyring; document key rotation procedure |
| Backpressure complexity | Medium — pipeline stalls | Medium | Bounded channels with documented capacity; monitoring via metrics |
| Circuit breaker false positives | Medium — skipped valid URLs | Low | Configurable thresholds; half-open testing; per-domain isolation |
| RUM API changes (Google) | Medium — broken integration | Medium | Abstract adapter interface; version pinning; fallback to lab-only data |

---

*Last updated: 2026-07-22*
