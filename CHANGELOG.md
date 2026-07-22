# Changelog

All notable changes to crawlkit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Full crawl loop with HTML parsing, link extraction, and analyzer pipeline
- 18 analyzers: HTTP, SEO, Content, Links, Images, Schema, Security, Performance, Mobile, Accessibility, Social
- SQLite storage with WAL mode and batch inserts
- Export formats: CSV, JSON, Markdown, HTML (interactive), SQLite
- Crawl comparison engine (diff between snapshots)
- REST API mode (crawlkit-api crate)
- Backlink analysis with PageRank scoring
- CLI with crawl, compare, report subcommands
- Progress bars and real-time stats

### Fixed
- HTTP/2 compatibility issue (removed http2_prior_knowledge)

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
