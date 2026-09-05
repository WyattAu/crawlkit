# Changelog

All notable changes to crawlkit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — coverage and release controls
- Coverage CI now measures native engine library and API all-target surfaces separately, including the existing router integration tests without instrumenting the WASM target
- Added persistence corruption, invalid-timestamp, login-lockout, disabled-account, tenant-scoping, and administrative-validation regression coverage; API all-target coverage now measures 60.38%
- Added webhook secret-boundary, event-validation, and cross-tenant-deletion regression coverage plus schedule tenant-scoped CRUD/partial-update and boundary-validation coverage (5 new router integration tests)
- Added marketplace lifecycle, rating-aggregation/bounds, RBAC, and test-plugin coverage plus API-key CRUD/redaction, tenant CRUD/isolation, and admin-only audit-trail coverage (9 new router integration tests); API all-target coverage now measures 64.14%

### Added — core/full feature boundary
- Core builds now compile without the full runtime: plugin, post-crawl, crawl-map, integration, and browser-specific CLI surfaces are gated behind `full` while `log-analyze` remains available in core mode
- `RenderedPageSummary` provides an owned, serializable rendering contract, with full Playwright output adapted through `RenderedPage::summary()`
- Full-only tests, benchmarks, examples, and CLI integration tests declare `required-features = ["full"]`
- Measured stripped release artifacts: core CLI 2,575,640 bytes (2.46 MiB) with `--no-default-features`; full CLI 25,579,176 bytes (24.39 MiB) with `--features full`

### Added — analyzer robustness and profiles
- **Analyzer profiles** (`core` 9 / `standard` 21 / `deep` 20 / `full`), selectable via `crawlkit crawl --analyzer-profile` or engine config; `standard` is now the recommended routine default with one canonical analyzer per major family
- **Per-analyzer panic isolation**: `AnalyzerRegistry::analyze` wraps every analyzer in `catch_unwind`; a panicking analyzer degrades to an `ANALYZER-PANIC` error finding instead of aborting the crawl
- **Registry finding-code uniqueness guard**: runtime fixture proving no two registered analyzers emit the same code on the same page
- **Behavior matrices** for 11 overlapping families (color-contrast, focus, heading-hierarchy, image-alt, anchor-text, link-quality, CSP, cookies, forms, tables, links) documenting finding-code ownership with dedicated fixtures
- **Generation dedup contract** (`test_generation_dedup.rs`) pinning the proof for every removed registration and every intentionally retained divergent pair
- 9 new coverage test modules; ~110 new tests (suite now 4,257 across the workspace)

### Changed — finding-code ownership (breaking for code consumers)
- Redundant analyzer generations received explicit namespaces: `FOCUS-V2001-DEEP-DEEP(-DEEP)`, `HHIER-V2002/3-DEEP-DEEP(-DEEP)`, `IMGALT-V2001/2-DEEP-DEEP(-DEEP)`, `ANCHGEN-V2001-DEEP`, `COLRCL-V2001-DEEP`, plus the earlier CORS/cookie/CSP/COEP/COOP namespacing
- `HHIER-V2002` semantic collision resolved: it now exclusively means "empty headings" (V2 generation); the V8 "missing H1" meaning moved to `HHIER-V2002-DEEP-DEEP`
- **Nine redundant registrations removed from the default registry** (833 → 778): the cookie Secure/HttpOnly/SameSite, canonical self-reference, canonical chain, focus-management, heading-hierarchy, and image-alt deep-deep-deep validators (exact duplicates or strict subsets), plus the reverse-subset `TableAccessibilityDeepDeepValidator`. All removed types remain exported and behavior-pinned; only their default registration changed
- `FormLabelsDeepDeepValidator` no longer counts hidden/submit/button/reset/image inputs as unlabeled (false positive fix, matching the deep-deep-deep semantics)
- Production analyzer code contains zero `unwrap`/`expect` calls

### Fixed
- 17 broken intra-doc links across `crawlkit-engine` and `crawlkit-api`; `cargo doc --workspace` now builds with zero warnings

### Validation
- Engine unit tests: 3,477 core tests and 3,944 full tests passing independently; full CLI smoke tests pass
- Workspace: 4,273 tests passing, 0 failed; fmt + clippy `-D warnings` clean on both primary crates
- Core and full engine/CLI all-target checks pass; core artifact is below the 10 MB target while full runtime size is reported separately
- Live crawl regression (kingstonpeptides.com, 10 pages, 500 ms delay): full profile 954 findings / 100 codes (unchanged — removed duplicates only fired on defects this site lacks), standard 80 / 8, core 30 / 3

## [4.4.1] - 2026-08-23

### Security — release integrity
- **GPG-signed `checksums.txt`** on this and all future releases (single detached signature covers all artifacts + SBOMs); signing is unconditional now that secrets exist; public key attached to the release (`release-public-key-0F8C446E31A16C97.asc`)

### Added
- ADR-011: WASI Preview 2 evaluation — five adoption gates with hard criteria; gate 2 (toolchain/MSRV) already passed via a wasmtime 47.0.4 component-model spike
- `scripts/oss-fuzz/`: ready-to-submit OSS-Fuzz integration (project.yaml, Dockerfile, build.sh) + submission instructions in docs/OSS_FUZZ.md

### Validation
- Dogfood run with in-crawl plugins against kingstonpeptides.com: soft-404 plugin active (count=1), 0 findings — correct, all 100 pages HTTP 200; a11y canary still 0; warning set unchanged

## [4.4.0] - 2026-08-23

### Added — Plugins run during crawls
- **Installed plugins execute automatically on every fetched page**, alongside built-in analyzers, with the B4 structured context (url/status/headers/parsed summary). Findings flow into storage/exports under `plugin:<category>` categories
- `CrawlEngineConfig::plugin_dirs` (empty = disabled; opt-in for embedders); CLI `--plugins <dir>` (repeatable); default roots `~/.crawlkit/plugins` + `$CRAWLKIT_PLUGIN_DIRS`
- Engine `plugin_runtime` module: `load_plugins_from_dir`, `CrawlPlugin`, `parse_plugin_findings` (malformed third-party JSON degrades to empty), `build_context_json`, `default_plugin_dirs`
- Failure semantics: unloadable plugins are skipped with a logged error; per-page plugin errors (incl. traps) contribute no findings and NEVER abort the crawl — both E2E-tested against a real wasm32 plugin and a trapping WAT module
- CLI smoke-verified end to end: `plugin install` → `crawl` → "crawl plugins active count=1" in logs, findings persisted

## [4.3.0] - 2026-08-23

### Added — Structured guest context (B4)
- Host function `crawlkit_host.get_context`: plugins receive the analysis context (url, status_code, response_time_ms, headers, parsed-page summary) as a NUL-terminated JSON string, precomputed by the host — no more inferring everything from raw HTML
- `WasmPlugin::analyze_with_context(html, url, ctx_json)` — context stored in host state per call; plain `analyze` unchanged (v1 ABI intact: guests opt in by importing the function; the host always links it since it leaks nothing beyond the HTML input)
- SDK module `crawlkit_plugin_sdk::host`: typed `HostContext`/`ParsedSummary` with safe wrappers `host::context()` / `host::context_json()`; graceful `None` degradation without context
- New example plugin `soft-404` (flags error pages that were still analyzed) demonstrating the context API
- Conformance: WAT guest verifying the JSON round-trip + null-without-context + no-leak-between-calls; full wasm32 soft-404 test (404 fires / 200 clean / no-context no-op)

## [4.2.0] - 2026-08-23

### Added — Marketplace day-one content
- **First-party plugin index** seeded in-repo at `plugins/index/` (ADR-010): `title-length` and `viewport-checker`, git-versioned, content-addressed, signed. Install straight from the raw GitHub URL — zero server infrastructure
- New SDK example plugin `viewport-checker` (missing/fixed-width viewport detection) with functional host-ABI tests (VP001/VP002/clean)
- `scripts/build-plugin-index.sh` — rebuild + re-sign the index (deliberate release events per ADR-010)
- README marketplace section with install-from-GitHub instructions

### Fixed
- **Remote-index artifact resolution** (`plugin_index`): relative `wasm_path` entries under an `https://` index now resolve against the index URL base (previously fell through to local filesystem). Resolution matrix unit-tested; `resolve_artifact_source` public
- **UN M49 region bug** (found by new fixtures for the mutants pass): `is_valid_locale` required 4-digit numeric regions; correct is 3-digit (`es-419` Spanish-Latin America is a canonical hreflang value that was wrongly rejected)

### Quality
- Direct fixtures for `KeywordAnalyzer::compute_tfidf`/`keyword_density` (hand-computed idf/density math) and `HreflangValidator::is_valid_locale` / `SitemapAnalyzer::is_valid_lastmod` (full branch coverage) — closes the arithmetic-mutant survival gaps found in the seo_analyzers mutants pass
- Dogfood gate wired into `verify-release-readiness.sh` (opt-in via `DOGFOOD=1`, per ADR-009)

## [4.1.0] - 2026-08-22

### Added — Plugin marketplace (B3, ADR-006/009 lineage)
- **`crawlkit plugin install/list/remove`**: git/file-based plugin distribution with zero server infrastructure. An index is one versioned TOML file (`plugin-index.toml`); artifacts are content-addressed (sha256) and ed25519-signed against the built-in trust store
- Engine: `plugin_index` module — `parse_plugin_index`, `install_plugin`, `list_installed_plugins`, `PluginIndexEntry`, `PluginIndexError`; `verify_plugin_artifact` (public in-memory trust verification, used pre-install). Installed layout loads directly under the default Required policy
- Security ordering: hash + signature verified BEFORE anything is written to the install root; tampered/untrusted/unknown entries install nothing (tested)
- 6-test conformance suite: index parse roundtrip; full wasm32 build → sign → index → install → load → analyze chain; tamper rejection; untrusted-signer rejection; unknown-name error; malformed index
- `docs/PLUGIN_MARKETPLACE.md` rewritten with shipped-reality banner

### Changed
- `crawlkit-seo-wasm` crate removed (dead legacy wasm_bindgen experiment, zero dependents/tests); readiness script updated

## [4.0.0] - 2026-08-22

### Breaking Changes
- **BREAKING**: `ParsedPage` gains `sentence_count: usize` (exhaustively-constructible struct adds a field; semver-checks gate enforced the major bump). Stored/serialized pages deserialize via `#[serde(default)]`
- Finding codes removed (output data changes): `ISEO006` "multi-language content detected" (hreflang presence is correct i18n, not a defect — fired on 100% of properly configured multilingual pages), `AI-AB007` "missing speakable schema" (optional niche enhancement, pure noise)

### Fixed
- **WC004 sentence-length bug** (found by the kingstonpeptides.com dogfood validation): the old implementation divided full-page `word_count` by a headings-only sentence count, reporting impossible averages (147-190 "words/sentence"). Both stats now come from the parser's single visible-text walk (consistent corpus by construction); terminator runs (`...`, `!?`) collapse to one sentence end; a corpus with words but no terminators counts as one implicit sentence

### Improved
- `IMG004` now names up to 3 offending image `src`s (e.g. the repeated footer badges) — actionable without re-inspection
- WC001 stats drop the misleading headings-based character count

### Quality
- cargo-mutants kill score on `analyzers/mod.rs`: 53.6% → **87.7%** after direct hand-computed fixtures for `flesch_kincaid_grade`/`flesch_reading_ease` (remaining misses include a provably-equivalent mutant)
- Live re-validation against kingstonpeptides.com: warning set unchanged (both true positives), noise findings eliminated, WC004 averages now plausible (23-37)

## [3.0.1] - 2026-08-22

### Security
- wasmtime 47.0.2 → 47.0.4 (RUSTSEC-2026-0222/0223 cleared; MSRV 1.94 unlocked)
- sqlx 0.8.6 → 0.9.0 (RUSTSEC-2024-0363 cleared); dynamic tenant query now carries audited `AssertSqlSafe` annotation; direct `rand` stays 0.8

### CI / Release
- cargo-semver-checks gate ENFORCING on main (prior failures were an invalid `--baseline` flag; correct: `--baseline-rev`; all 4 crates pass vs v3.0.0)
- Release matrix: build+package only (tests gated in CI main + release preflight); fixes Windows cancellations (wasm_abi_tests triggers a second full wasm32 compile on runners)
- 5-platform binaries shipped (Windows + macOS-ARM64 restored)

### Added
- `scripts/dogfood.sh` — standing pre-release production-site crawl with triage summary (protocol born from the KP WCAG-H67 bug)
- cargo-mutants baseline: analyzers/mod.rs 53.6% kill score, gaps documented in `.cargo/mutants.toml`

## [3.0.0] - 2026-08-19

### Breaking Changes (engine API)
- **BREAKING**: `Analyzer::analyze` no longer takes `&CrawlConfig` (zero of 33 implementations used it); `AnalyzerRegistry::analyze(&ctx)` follows — see `docs/MIGRATION.md`
- **BREAKING**: `HtmlParser::parse` returns `ParsedPage` directly — the `ParseError` type is removed (the error-tolerant HTML5 parser made it infallible); `StreamingHtmlParser::parse` likewise — see `docs/MIGRATION.md`
- **BREAKING**: `CrawlError::Storage` now carries the structured `StorageError` type (was `String`); new `CrawlError::Internal` for non-storage failures — see `docs/MIGRATION.md`

### Security
- API: closed cross-tenant exposure in `GET /crawls/{id}/stats|findings|backlinks` — unknown crawl ids now default-deny instead of falling through to unscoped storage queries
- API: RBAC enforced on administrative surfaces (tenants, API keys, marketplace, user deletion) via `require_permission`; new `tenant:*`, `marketplace:*`, and `audit:read` permissions on the admin role; non-admins can no longer delete tenant-mate users
- API: session revocation is now enforced by the JWT middleware; sessions are registered on login, refresh, and OIDC callback
- API: brute-force lockout on `/auth/login` (5 failures per email per 15 min → 15 min lockout)
- API: OIDC hardening — id_tokens are validated against the provider JWKS (signature, `iss`, `aud`, `exp`), PKCE (S256) and nonce binding added to the authorization flow
- API: SSRF hardening — server-side outbound HTTP client re-validates every redirect hop against the blocklist (5-hop cap); webhook destinations re-validated at delivery time
- API: `requests_per_minute` bounded (1..=10,000) on API key creation
- API: admin bootstrap password can be supplied via `ADMIN_PASSWORD` (no longer logged when provided); password generator modulo bias removed
- API: `/metrics` requires an API key by default; `METRICS_PUBLIC=true` reopens it with a startup warning
- Engine: HTML/Markdown export escapes crawled page content (titles, URLs, finding text) — closes report-injection via crafted page titles
- Engine: WASM sandbox wall-clock timeout enforced via wasmtime epoch interruption (watchdog thread + `Trap::Interrupt` detection) — runaway plugins terminate at `max_analysis_timeout_ms` instead of running indefinitely
- Engine: WASM capability enforcement is fail-closed — manifests requesting `network`, `filesystem`, or `env_vars` permissions are rejected at load (the sandbox grants none of them)
- Audit: persistent tamper-evident audit trail (`AUDIT_LOG_PATH`, JSONL + SHA-256 chain, fsync per event, chain verification on open, head-anchor sidecar detects tail truncation); `AuditTrail::clear` refuses to clear persistent trails; `GET /audit` is admin-only with tenant filtering for non-admins
- Release: artifacts are GPG-signed via a single `checksums.txt` signature covering all five platform archives and the SBOM
- Engine: WASM plugin trust chain — ed25519 manifest signing (`wasm_hash`/`signature`/`signed_by`), signature REQUIRED by default against a built-in trust store, fail-closed hash verification before wasmtime compile; `crawlkit plugin keygen/sign/verify` CLI; `allow_unsigned_plugins` escape hatch for local development

### Fixed
- API/storage crawl-id dissociation: `CrawlResult::storage_crawl_id` now binds the public crawl id to the engine-owned storage row; stats/findings/backlinks resolve to the row that actually contains pages
- Plugin SDK exported a different ABI than the host consumed (`alloc`/`free` missing, non-NUL-terminated results) — SDK now implements the host ABI with a sound header-based guest allocator; added host<->guest conformance tests (WAT fixture + wasm32-compiled SDK example)
- Duplicate analyzer registration lists merged into `AnalyzerRegistry` single registration site (engine list had drifted, omitting 3 analyzers); "empty" sitemap/SSL analyzers no longer emit per-page noise findings (`SITEMAP001`, `SSL007`)
- Engine: `PgStorage` sync-trait bridge no longer panics outside a Tokio runtime — uses a dedicated global blocking runtime instead of `Handle::current().block_on` (regression-tested from plain-sync and `spawn_blocking` contexts)
- Go client: `StartCrawl` now accepts `202 Accepted` (previously always failed against the real API)

### Changed
- `crawlkit-api` restructured as lib + bin to enable integration testing of the router
- Engine: crawl-lifetime storage calls (`start_crawl`, incremental lookups, `finish_crawl`) moved off the async runtime onto the blocking pool
- Engine: `run_with_callback` decomposed — queue prefill (seed + incremental + sitemaps) and finish/report extracted into `prefill_queue`/`finish_and_report`

### Added
- Determinism rails: seeded user-agent rotation (per-URL stable hash, `HttpClientConfig::with_seed` wired through `CrawlEngineConfig.seed`); findings and exports canonically ordered — identical input now produces byte-identical reports (regression-tested); `DeterminismController::derive_seed` made pure with an explicit order-sensitive `derive_seed_stream`; robots.txt and sitemap XML fuzz targets (2.5M+ clean runs); `.cargo/mutants.toml` baseline config
- API backpressure + idempotency: crawl submissions bounded by `MAX_CONCURRENT_CRAWLS` (503 + Retry-After at capacity, scheduler skips gracefully); `Idempotency-Key` on `POST /api/v1/crawls` replays the original crawl within a 24h window
- API-plane state backends: `ApiStateStore` trait; SQLite default; PostgreSQL via `API_STATE_PG_URL`
- SLOs: `docs/SLO.md` with per-tenant usage metrics (`crawlkit_crawls_started_by_tenant`, `crawlkit_pages_by_tenant`) and example Prometheus alerts in `monitoring/alerts.yml`
- CI: `test-services` job runs the previously-ignored PostgreSQL and Redis suites against live service containers; dashboard build gate; dashboard component testing (jsdom + @testing-library) with the first component suite — dashboard tests 3 → 44
- OpenAPI documentation: 38 paths annotated via utoipa; OpenAPI JSON at `/api/v1/openapi.json` and Swagger UI at `/api/v1/docs` (gated by `DOCS_PUBLIC=false` → 404)
- Persistent API-plane state: users, tenants, and API keys write-through to SQLite (`API_STATE_DB_PATH`, default `<db>.state`) and are restored on startup — a restart no longer loses accounts. Sessions remain in-memory by design (short-lived JWTs; documented trade-off)
- Router-level API integration test suite (19 tests: auth gates, CSRF, lockout, tenant isolation, RBAC, session revocation, webhook SSRF validation, metrics auth, OpenAPI)
- WASM ABI integration tests (7 tests) incl. full SDK→wasm32→wasmtime conformance run, wall-clock timeout kill, and capability fail-closed checks
- Audit events recorded for login success/failure, session revocation, crawl lifecycle, and tenant mutations
- Client SDK test suites: Go (httptest, table-driven), Python (unittest + httpx.MockTransport), Node (jest with committed toolchain) — 49 tests total; Python client gained an optional `transport` injection point for testing
- Dashboard test suite grew 3 → 41 vitest tests (api_client method/auth/error coverage + use_auth hook store transitions); eslint + tsc clean
- CI: bin-target unit tests (88 previously silently skipped), `property_tests` (21 previously excluded), `wasm_abi_tests`, and `router_tests` now run; fuzz job path fixed (was a guaranteed no-op); honest `cargo-machete` dependency check replaces the fake `^use ` grep; advisory `cargo-semver-checks` job
- Dependabot, CODEOWNERS, PR/issue templates; ADR-001 numbering collision resolved (WASM error detection → ADR-005)
- Version/test-count claims across README/VERSION/ROADMAP reconciled with reality; fabricated claims removed (Lean4 verification, aspirational docs now banner-marked)

### Removed
- `parser::ParseError` (never constructed); `Analyzer` trait's unused config parameter

### Rust API additions
- `crawlkit-engine`: `AuditTrail::open_persistent`, `record_tenant`, `events_for_tenant`; `AuditEvent.tenant_id`; new `AuditEventType` auth/tenant variants; `AuditError`; `CrawlError::Internal`
- `crawlkit-plugin-sdk`: `crawlkit_plugin_sdk::exported::{alloc_raw, free_raw}` (macro-internal allocator, now sound via size headers)

### Housekeeping
- `clients/nodejs/node_modules`, `dashboard/dist` build outputs, and Python `__pycache__` untracked from git (files remain locally; ignored going forward)

## [2.3.0] - 2026-07-27

### Added
- Redis-backed distributed crawl queue (sorted sets, ZPOPMAX)
- PostgreSQL storage backend via the `StorageBackend` trait (sqlx) with migrations (`001_init.sql`)
- Core Web Vitals measurement via Chrome DevTools Protocol (LCP/CLS/INP/FCP/TTFB), wired into the crawl engine when JS rendering is enabled (PerformanceObserver JS injection)
- `cwv_lcp`/`cwv_cls`/`cwv_inp` fields on `PageData`
- Redis/PostgreSQL/browser-dependent tests marked `#[ignore]`

## [2.2.0] - 2026-07-26

### Added
- Streaming HTML parser (`parse_stream` with a `ParserEvent` channel)
- Native plugin loading via libloading (`.so`/`.dylib`/`.dll`)
- `LinkExtractor` helper with deduplication
- Webhook HMAC-SHA256 signing and HTTP delivery with retry; webhook secret generated once, returned at creation, never serialized
- `PATCH /schedules/:id` and `last_run_at` tracking for scheduled crawls
- Session management (list, revoke) and hardened OIDC SSO: state token TTL (10 min), user provisioning, role mapping

### Changed
- Pre-commit unsafe-code check excludes `native_plugin.rs`

## [2.1.0] - 2026-07-26

### Added
- Multi-tenant API isolation: all CRUD endpoints scoped by JWT tenant; tenant derived from JWT (never client-supplied), with admin bypass
- Plugin manifest validation: semver, SPDX license, required fields
- Storage abstraction layer: `StorageBackend` trait with SQLite implementation
- Rustdoc examples on 11 public API items

### Changed
- Connection pool sizing scales with concurrency config
- Rate limiter LRU eviction (10K domain max)
- Export N+1 query elimination (bulk load issues/links)
- Version bump to 2.1.0

## [2.0.0] - 2026-07-24

### Added
- WASM plugin system, JWT authentication, RBAC, enterprise architecture
- Rustdoc coverage, ADRs, runnable examples, security guide
- Workspace renamed `crawlkit-core` to `crawlkit-engine`

### Changed
- Removed `http2_prior_knowledge` (not supported in reqwest 0.12)
- Version bump to 2.0.0

## [0.4.0] - 2026-07-23

### Added
- 28 total analyzers (23 core + 4 AI + 1 WASM)
- AI search optimization analyzers:
  - AI crawler accessibility analyzer (robots.txt AI bot detection)
  - AI content structure analyzer (AI-friendly content patterns)
  - AI citation eligibility analyzer (source authority signals)
  - AI answer box analyzer (FAQ/HowTo/Q&A schema readiness)
  - AI bot registry (GPTBot, Google-Extended, PerplexityBot, ClaudeBot, etc.)
- WASM pattern analyzer (static detection of WASM issues)
- Concurrent DNS cache with background prefetching and TTL eviction
- Criterion benchmarks for parser, analyzers, registry, queue, storage
- 3 runnable examples: basic-crawl, custom-analyzer, export-report
- 3 tutorials: getting-started, custom-analyzers, ci-integration
- Cross-platform release workflow (Linux x86/aarch64, macOS x86/aarch64, Windows x86)
- REST API mode with API key authentication and rate limiting
- Backlink analysis with PageRank scoring
- Crawl comparison engine (diff between snapshots)
- Export formats: CSV, JSON, Markdown, HTML (interactive), SQLite
- Feature flag system for JS rendering, AI analyzers, WASM analyzers
- Circuit breaker pattern for fault tolerance
- Backpressure controller and bounded pipeline
- Resource usage tracking and monitoring
- Audit trail system
- Determinism controller for reproducible crawls
- Encryption manager for sensitive data
- Playwright integration (placeholder for JS rendering)
- JS rendering decision logic
- Real User Monitoring (RUM) integration
- Workspace-level lint configuration (`unsafe_code = "forbid"`, clippy safety lints)
- Pre-commit hook with fmt, clippy `-D warnings`, tests, audit, build checks

### Changed
- HTTP/2 compatibility improved (http2_prior_knowledge disabled for broader compatibility)
- SQLite storage now uses WAL mode with batch inserts
- CLI expanded with crawl, compare, report subcommands
- Refactored `run_crawl` to use `CrawlParams` struct (eliminates 14-argument function)
- CI workflow now enforces `RUSTFLAGS="-D warnings"` and clippy `-D warnings`
- Release workflow uses `cross` for aarch64-linux-gnu cross-compilation
- Docs workflow uses `cargo doc` instead of broken npm-based approach
- HTML export reports now include WCAG accessibility attributes (scope, aria-label, sr-only)
- Renamed `Permission::from_str` to `Permission::parse` (eliminates std::str::FromStr confusion)

### Fixed
- HTTP/2 compatibility issue (removed http2_prior_knowledge)
- Scope filtering and progress reporting issues
- MutexGuard held across await point in ratelimit.rs (async safety)
- 6 field-assignment-outside-initializer patterns in queue.rs tests
- 2 manual RangeInclusive::contains patterns (analyzers.rs, ratelimit.rs)
- map_or to is_some_and conversion in wasm_analyzers.rs
- Unused imports in integration tests and benchmarks
- Redundant pattern matching in basic-crawl.rs example
- Dead code warning for unused OutputConfig::format field
- Cross-sign modulo arithmetic in benchmarks

## [0.1.0] - 2026-07-22

### Added
- Initial release
- Core HTTP client with retry and redirect tracking
- URL queue with priority and deduplication
- Rate limiter (token-bucket per domain)
- HTML parser with link, meta, image, heading extraction
- Meta tag analysis (title, description, OG, Twitter, hreflang)
- SQLite storage layer
- CLI framework with clap
- 263+ unit tests
