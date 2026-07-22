# crawlkit

[![CI](https://github.com/WyattAu/crawlkit/actions/workflows/ci.yml/badge.svg)](https://github.com/WyattAu/crawlkit/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)

A high-performance Rust-based site crawler and SEO analysis toolkit that surpasses commercial tools like Ahrefs in depth, speed, and extensibility.

## Features

### Core Crawling
- **High-performance crawl engine** — async HTTP/2 fetching with configurable concurrency (50+ pages/sec)
- **Full redirect chain tracking** — follows ALL redirects (up to 20 hops), detects loops and mixed protocols
- **Polite crawling** — respects robots.txt, rate limiting, crawl-delay directives
- **Priority queue** — URLs scored by importance, deduplicated via concurrent hash set

### SEO Analysis (18 analyzers)
- **Meta tags** — title, description, canonical, OG, Twitter Cards, hreflang
- **Link analysis** — internal/external counts, broken links, orphan pages, PageRank scoring
- **Structured data** — JSON-LD and Microdata validation against Schema.org
- **Content quality** — Flesch-Kincaid readability, keyword density, word count
- **Heading hierarchy** — H1-H6 structure, skipped levels detection

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

## Installation

### From source

```bash
git clone https://github.com/WyattAu/crawlkit.git
cd crawlkit
cargo build --release
```

Binary: `target/release/crawlkit`

### Via cargo install (once published)

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
crawlkit 0.1.0
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
user_agent = "crawlkit/0.1.0"
respect_robots_txt = true

[crawl.scope]
allowed_domains = ["example.com"]
blocked_patterns = ["/wp-admin/*", "/api/*"]

[analyzers]
enabled = ["meta", "links", "security", "accessibility", "content"]

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
│   ├── crawlkit-core/     # Core types, analyzers, storage, export
│   ├── crawlkit/          # CLI binary
│   └── crawlkit-api/      # REST API server
├── docs/                  # Architecture, roadmap, competitive analysis
├── scripts/               # Build and utility scripts
├── Cargo.toml             # Workspace root
├── README.md
├── CHANGELOG.md
└── CONTRIBUTING.md
```

### Analyzer Pipeline

```
URL Queue → Fetcher → Parser → Analyzers → Storage → Export
    ↑                    ↓
    └──── Link Discovery ─┘
```

### 18 Analyzers

| Category | Analyzers |
|----------|-----------|
| HTTP | Status codes, redirects, response times |
| SEO | Meta tags, canonical, hreflang, sitemap, robots.txt |
| Content | Readability, keywords, word count |
| Links | Internal/external, broken links, orphan pages |
| Images | Alt text, dimensions, lazy loading |
| Schema | JSON-LD, Microdata validation |
| Security | CSP, HSTS, X-Frame-Options, COEP/COOP/CORP |
| Performance | TTFB, FCP, page size |
| Mobile | Viewport, touch targets, font sizes |
| Accessibility | WCAG 2.1 AA (16 checks) |
| Social | Open Graph, Twitter Cards |

## Performance

| Metric | Target | Measured |
|--------|--------|----------|
| Pages/sec | ≥50 | 50-100 |
| Memory (10k pages) | <500MB | ~200MB |
| Startup time | <100ms | ~10ms |
| Binary size | <10MB | ~8MB |

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — Full technical architecture
- [Roadmap](docs/ROADMAP.md) — Development roadmap
- [Competitive Analysis](docs/COMPETITIVE_ANALYSIS.md) — 25 competitors compared
- [ADR-001](docs/ADR-001-crawler-architecture.md) — Architecture decision record

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
