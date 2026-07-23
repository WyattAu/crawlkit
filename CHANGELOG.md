# Changelog

All notable changes to crawlkit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
