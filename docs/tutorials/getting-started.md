# Getting Started with crawlkit

A step-by-step guide for new users.

## Prerequisites

- Rust 1.70+ (installed via [rustup](https://rustup.rs/))
- Git

## Installation

### From source

```bash
git clone https://github.com/WyattAu/crawlkit.git
cd crawlkit
cargo build --release
```

The binary is at `target/release/crawlkit`.

### Via cargo install (once published)

```bash
cargo install crawlkit
```

## Quick start

Crawl a website and save results:

```bash
crawlkit crawl https://example.com --max-pages 50 --output results/
```

This produces:

| File | Format | Contents |
|------|--------|----------|
| `results/crawlkit.db` | SQLite | All crawl data, pages, links, findings |
| `results/crawl-results.json` | JSON | Structured crawl summary |
| `results/report.html` | HTML | Interactive single-file report |
| `results/report.md` | Markdown | Text summary |

## Step 1: Your first crawl

```bash
crawlkit crawl https://example.com
```

Default settings: 100 pages, 4 concurrent fetchers, 500ms delay between requests, robots.txt respected.

## Step 2: Customize crawl parameters

```bash
crawlkit crawl https://example.com \
  --max-pages 200 \
  --delay 200 \
  --concurrency 8 \
  --depth 5 \
  --output my-crawl/
```

| Flag | Default | Description |
|------|---------|-------------|
| `--max-pages` | 100 | Stop after this many pages |
| `--delay` | 500 | Milliseconds between requests |
| `--concurrency` | 4 | Parallel fetchers |
| `--depth` | unlimited | Max link depth from seed URL |
| `--timeout` | 30 | Request timeout in seconds |
| `--user-agent` | `crawlkit/0.1.0` | Custom User-Agent header |
| `--respect-robots` | true | Obey robots.txt |
| `--allow-external` | false | Follow off-domain links |

## Step 3: Use a configuration file

Create `crawlkit.toml` in your working directory:

```toml
[crawl]
max_pages = 200
delay_ms = 300
concurrency = 8
timeout_secs = 15
user_agent = "my-company-bot/1.0"
respect_robots_txt = true

[output]
dir = "./crawl-results"
format = "all"
```

Then run without flags:

```bash
crawlkit crawl https://example.com --config crawlkit.toml
```

## Step 4: Export data

### JSON

```bash
# Already produced by crawl — read it:
cat crawl-results/crawl-results.json | jq .
```

### SQLite (ad-hoc queries)

```bash
sqlite3 crawl-results/crawlkit.db \
  "SELECT url, status_code, title FROM pages WHERE status_code != 200;"
```

### Generate a report from existing crawl data

```bash
crawlkit report crawl-results/ --format html --output report.html
```

## Step 5: Compare two crawls

Run two crawls at different times, then diff them:

```bash
crawlkit crawl https://example.com --output before/
# ... make changes to your site ...
crawlkit crawl https://example.com --output after/
crawlkit compare before/ after/ --output diff.json
```

The diff shows:

- **Added pages** — new in the target crawl
- **Removed pages** — existed in baseline but gone now
- **Status changes** — HTTP code differences
- **Title changes** — `<title>` tag differences
- **Content changes** — word count shifts >10% or >100 words
- **Size changes** — body size differences >10%

## Step 6: Programmatic usage (Rust)

Add `crawlkit-core` to your `Cargo.toml`:

```toml
[dependencies]
crawlkit-core = { git = "https://github.com/WyattAu/crawlkit" }
tokio = { version = "1", features = ["full"] }
```

Minimal example:

```rust
use crawlkit_core::{CrawlConfig, HtmlParser, FetchResult};
use std::time::Duration;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse HTML directly (no network needed)
    let html = r#"<html><head><title>My Page</title></head>
    <body><h1>Hello</h1><p>World</p></body></html>"#;

    let url = Url::parse("https://example.com/page")?;
    let parsed = HtmlParser::parse(html, &url)?;

    println!("Title: {:?}", parsed.meta.title);
    println!("Word count: {}", parsed.word_count);
    println!("Links: {}", parsed.links.len());

    Ok(())
}
```

For a full crawl loop, see `examples/basic-crawl.rs`.

## Step 7: Run analyzers

```rust
use crawlkit_core::analyzers::{AnalysisContext, AnalyzerRegistry};
use crawlkit_core::CrawlConfig;

let config = CrawlConfig::default();
let registry = AnalyzerRegistry::new(&config);

// After fetching and parsing a page:
let findings = registry.analyze(&ctx, &config);

for finding in &findings {
    println!("[{:?}] {}: {}", finding.severity, finding.code, finding.title);
}
```

Built-in analyzers cover: HTTP status, redirects, canonical URLs, hreflang, sitemaps, robots.txt, meta tags, heading hierarchy, link quality, images, structured data, security headers, SSL, mobile-friendliness, content quality, accessibility, and social metadata.

## Next steps

- Read the [custom analyzers tutorial](./custom-analyzers.md) to extend crawlkit with your own checks
- See the [CI integration guide](./ci-integration.md) to automate crawls in your pipeline
- Browse the [architecture docs](../ARCHITECTURE.md) for internals
