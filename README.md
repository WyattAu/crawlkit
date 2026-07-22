# crawlkit

A high-performance Rust-based site crawler and SEO analysis toolkit.

crawlkit crawls websites, extracts SEO signals (meta tags, links, headings, structured data), and produces actionable reports. Built for speed, correctness, and extensibility.

## Features

- **High-performance crawl engine** — async HTTP/2 fetching with configurable concurrency
- **SEO analysis** — meta tags, canonical URLs, hreflang, heading hierarchy, Open Graph, Twitter Cards
- **Link analysis** — internal/external link counting, broken link detection, orphan page identification
- **Security checks** — security headers, SSL certificate validation
- **Structured data** — JSON-LD and Microdata validation
- **Content analysis** — readability scores, keyword density, word count
- **Multiple export formats** — JSON, CSV, Markdown, interactive HTML reports
- **Plugin system** — extend with custom analyzers via native or WASM plugins
- **SQLite storage** — persistent crawl data with concurrent-safe access

## Installation

### From source

```bash
git clone https://github.com/WyattAu/crawlkit.git
cd crawlkit
cargo build --release
```

The binary will be at `target/release/crawlkit`.

### Via cargo install (once published)

```bash
cargo install crawlkit
```

## Quick Start

Crawl a website and save results:

```bash
crawlkit crawl https://example.com --max-pages 50 --output results.json
```

## CLI Reference

```
crawlkit crawl <URL> [OPTIONS]

Options:
  --max-pages <N>       Maximum number of pages to crawl [default: 100]
  --delay <MS>          Delay between requests in milliseconds [default: 500]
  --concurrency <N>     Number of concurrent fetchers [default: 4]
  -o, --output <FILE>   Output file path (JSON) [default: crawl-results.json]
  -h, --help            Print help
  -V, --version         Print version
```

## Configuration

crawlkit can be configured via CLI flags or a `crawlkit.toml` file in the working directory.

### crawlkit.toml

```toml
[general]
max_pages = 200
delay = 300
concurrency = 8

[http]
user_agent = "my-crawler/1.0"
timeout = 30
max_redirects = 20

[scope]
respect_robots_txt = true
allowed_patterns = ["/blog/*", "/products/*"]
disallowed_patterns = ["/admin/*", "/private/*"]
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
  --output crawl-data.json
```

### Limit scope with patterns

```bash
crawlkit crawl https://example.com \
  --allowed "/blog/*" \
  --disallowed "/admin/*"
```

## Architecture

```
crawlkit/
├── crates/
│   ├── crawlkit-core/     # Core types, config, error definitions
│   └── crawlkit/          # CLI binary
├── docs/                  # Documentation and roadmap
├── Cargo.toml             # Workspace root
└── README.md
```

## Development

### Prerequisites

- Rust 1.75+ (MSRV)
- cargo

### Build

```bash
cargo build
```

### Test

```bash
cargo test --workspace
```

### Lint

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

## Roadmap

See [docs/ROADMAP.md](docs/ROADMAP.md) for the full development plan.

| Phase | Goal | Target |
|-------|------|--------|
| 0 | Foundation — crawl loop, CLI | Week 1–2 |
| 1 | Core SEO analyzers | Week 3–4 |
| 2 | Content analysis | Week 5–6 |
| 3 | Security & performance | Week 7–8 |
| 4 | Advanced features & plugins | Week 9–10 |
| 5 | Export & reporting | Week 11–12 |
| 6 | Polish & v1.0 release | Week 13–14 |

## Contributing

Contributions are welcome. Please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Commit your changes (`git commit -m 'feat: add my feature'`)
4. Push to the branch (`git push origin feat/my-feature`)
5. Open a Pull Request

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
