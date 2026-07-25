# crawlkit

[![CI](https://github.com/WyattAu/crawlkit/actions/workflows/ci.yml/badge.svg)](https://github.com/WyattAu/crawlkit/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-422--passing-green.svg)](https://github.com/WyattAu/crawlkit)
[![Analyzers](https://img.shields.io/badge/analyzers-28-blue.svg)](https://github.com/WyattAu/crawlkit)
[![Clippy](https://img.shields.io/badge/clippy-0--warnings-blue.svg)](https://github.com/WyattAu/crawlkit)

High-performance Rust web crawler and SEO analysis toolkit. Async HTTP/2 fetching, 28 analyzers, supply chain security auditing, zero clippy warnings.

## Features

### Core Crawling
- **High-performance crawl engine** — async HTTP/2 fetching with configurable concurrency (50+ pages/sec)
- **Full redirect chain tracking** — follows ALL redirects (up to 20 hops), detects loops and mixed protocols
- **Polite crawling** — respects robots.txt, rate limiting, crawl-delay directives
- **Priority queue** — URLs scored by importance, deduplicated via concurrent hash set
- **Concurrent DNS cache** — background prefetching with TTL eviction for faster resolution

### SEO Analysis
- **Meta tags** — title, description, canonical, OG, Twitter Cards, hreflang
- **Link analysis** — internal/external counts, broken links, orphan pages, PageRank scoring
- **Structured data** — JSON-LD and Microdata validation against Schema.org
- **Content quality** — Flesch-Kincaid readability, keyword density, word count
- **Heading hierarchy** — H1-H6 structure, skipped levels detection
- **Canonical URL validation** — canonical tag correctness and consistency
- **Hreflang validation** — international SEO hreflang tag verification
- **Sitemap analysis** — XML sitemap parsing and validation
- **Robots.txt analysis** — robots.txt parsing and compliance checking
- **Image analysis** — alt text, dimensions, lazy loading detection
- **Ecommerce signals** — product schema, pricing, availability detection
- **International SEO** — language targeting, regional content analysis

### AI Search Optimization
- **AI crawler accessibility** — detects AI bot blocking in robots.txt
- **AI content structure** — evaluates AI-friendly content patterns
- **AI citation eligibility** — identifies source authority signals
- **AI answer box readiness** — checks FAQ/HowTo/Q&A schema readiness
- **AI bot registry** — tracks GPTBot, Google-Extended, PerplexityBot, ClaudeBot, and 6 more

### WASM Analysis
- **WASM pattern detection** — identifies missing modulepreload, legacy instantiation, missing crossorigin

### Security & Performance
- **Security headers** — CSP, HSTS, X-Frame-Options, COEP/COOP/CORP scoring (0-100)
- **SSL validation** — certificate chain, expiry, SAN matching
- **Mobile-friendliness** — viewport, touch targets, font sizes
- **Core Web Vitals** — LCP, FID/INP, CLS, TTFB, FCP (via Chromium integration)

### Accessibility
- **WCAG 2.1 AA** — alt text, heading hierarchy, ARIA, keyboard navigation, color contrast

### Export & Reporting
- **CSV** — configurable columns, nested data
- **JSON** — schema-versioned, full structure
- **Markdown** — auto-generated summary
- **HTML** — interactive single-file report with charts
- **SQLite** — normalized schema for ad-hoc queries

### Advanced Features
- **Crawl comparison** — diff between snapshots (new/removed/changed pages)
- **Backlink analysis** — PageRank scoring, internal link graphs
- **REST API** — programmatic access with API key authentication
- **Plugin system** — extend with custom analyzers
- **Feature flags** — toggle JS rendering, AI analyzers, WASM analyzers
- **Circuit breaker** — fault tolerance for external dependencies
- **Backpressure** — bounded pipeline for memory-efficient crawling
- **Resource monitoring** — track memory, CPU, and network usage
- **Audit trail** — complete logging of all crawl operations
- **Determinism** — reproducible crawl results with seed control
- **Encryption** — sensitive data protection
- **Real User Monitoring** — RUM integration for performance data

## Installation

### From source

```bash
git clone https://github.com/WyattAu/crawlkit.git
cd crawlkit
cargo build --release
```

Binary: `target/release/crawlkit`

### Via cargo install

```bash
cargo install crawlkit
```

### Binary releases

Download pre-built binaries from [GitHub Releases](https://github.com/WyattAu/crawlkit/releases).

Available for:
- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)

## Quick Start

```bash
# Crawl a website
crawlkit crawl https://example.com --max-pages 50

# Compare two crawls
crawlkit compare crawl1/ crawl2/ --output diff.json

# Generate HTML report
crawlkit report crawl1/ --format html --output report.html
```

## CLI Reference

```
crawlkit 0.4.0
Wyatt Au
A high-performance Rust-based site crawler for SEO analysis

USAGE:
    crawlkit <COMMAND>

COMMANDS:
    crawl      Crawl a website and analyze it
    compare    Compare two crawl snapshots
    report     Generate a report from crawl data
    help       Print help

CRAWL OPTIONS:
    -u, --url <URL>              Target URL to crawl
    -m, --max-pages <N>          Maximum pages to crawl [default: 100]
    -d, --delay <MS>             Delay between requests (ms) [default: 500]
    -c, --concurrency <N>        Concurrent fetchers [default: 4]
    -o, --output <DIR>           Output directory [default: .]
    -f, --format <FORMAT>        Output format: json, csv, sqlite, html, all [default: all]
    --max-depth <N>              Maximum crawl depth
    --user-agent <STRING>        Custom user agent
    --timeout <SECONDS>          Request timeout [default: 30]
    --no-robots                  Ignore robots.txt
    --javascript                 Enable JavaScript rendering
```

## Configuration

### CLI flags

All options can be set via CLI flags (see above).

### Configuration file

Create `crawlkit.toml` in the working directory:

```toml
[crawl]
seed_urls = ["https://example.com"]
max_pages = 200
max_depth = 10
max_redirect_hops = 20
max_concurrent_requests = 64
request_timeout_secs = 30
crawl_delay_default_ms = 1000
user_agent = "crawlkit/0.4.0"
respect_robots_txt = true

[crawl.scope]
allowed_domains = ["example.com"]
blocked_patterns = ["/wp-admin/*", "/api/*"]

[analyzers]
enabled = ["meta", "links", "security", "accessibility", "content", "ai", "wasm"]

[output]
formats = ["json", "sqlite", "html"]
output_dir = "./crawl-results"
```

## Examples

### Basic crawl

```bash
crawlkit crawl https://example.com
```

### Crawl with custom settings

```bash
crawlkit crawl https://example.com \
  --max-pages 500 \
  --delay 200 \
  --concurrency 8 \
  --output crawl-data
```

### Compare two crawls

```bash
crawlkit compare before/ after/ --output diff.json
```

### Generate HTML report

```bash
crawlkit report crawl-data/ --format html --output report.html
```

### REST API mode

```bash
crawlkit-api --port 8080 --api-key my-secret-key
```

## Architecture

```
crawlkit/
├── crates/
│   ├── crawlkit-engine/     # Core types, analyzers, storage, export
│   ├── crawlkit/          # CLI binary
│   └── crawlkit-api/      # REST API server
├── docs/                  # Architecture, roadmap, competitive analysis
├── .github/workflows/     # CI/CD pipelines (format, clippy, test, audit, release)
├── Cargo.toml             # Workspace root
├── README.md
├── CHANGELOG.md
└── CONTRIBUTING.md
```

### Analyzer Pipeline

```
URL Queue → DNS Prefetch → Fetcher → Parser → Analyzers → Storage → Export
    ↑                              ↓
    └──── Link Discovery ──────────┘
```

### 28 Analyzers

| Category | Analyzers | Count |
|----------|-----------|-------|
| HTTP | Status codes, redirects, response times | 2 |
| SEO | Meta tags, canonical, hreflang, sitemap, robots.txt | 5 |
| Content | Readability, keywords, word count, ecommerce, international | 5 |
| Links | Internal/external, broken links, orphan pages | 1 |
| Images | Alt text, dimensions, lazy loading | 1 |
| Schema | JSON-LD, Microdata validation | 1 |
| Security | CSP, HSTS, X-Frame-Options, COEP/COOP/CORP, SSL | 2 |
| Performance | TTFB, FCP, page size | 1 |
| Mobile | Viewport, touch targets, font sizes | 1 |
| Accessibility | WCAG 2.1 AA (16 checks) | 1 |
| Social | Open Graph, Twitter Cards | 1 |
| Entity | Named entity extraction | 1 |
| AI | Crawler accessibility, content structure, citation eligibility, answer box | 4 |
| WASM | Pattern detection | 1 |

## Performance

| Metric | Target | Measured |
|--------|--------|----------|
| Pages/sec | >=50 | 50-100 |
| Memory (10k pages) | <500MB | ~200MB |
| Startup time | <100ms | ~10ms |
| Binary size | <10MB | ~8MB |
| Full analyzer suite | — | ~25 us/page |
| HTML parse (5 KB) | — | ~45 us |
| PageRank (1K nodes) | — | ~4 ms |

## Benchmarks

Run benchmarks with:

```bash
cargo bench
```

Benchmarks cover:
- HTML parser performance
- Analyzer execution time
- Registry lookup speed
- Queue operations
- Storage insert/query performance

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — Full technical architecture
- [Roadmap](docs/ROADMAP.md) — Development roadmap
- [Competitive Analysis](docs/COMPETITIVE_ANALYSIS.md) — 25 competitors compared
- [ADR-001](docs/ADR-001-crawler-architecture.md) — Architecture decision record
- [Benchmarks](docs/benchmarks.md) — Performance benchmarks
- [Getting Started](docs/tutorials/getting-started.md) — Step-by-step user guide
- [Custom Analyzers](docs/tutorials/custom-analyzers.md) — Writing custom analyzers
- [CI Integration](docs/tutorials/ci-integration.md) — CI/CD integration guide

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
