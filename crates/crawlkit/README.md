# crawlkit

High-performance Rust web crawler and SEO analysis toolkit.

## Installation

```bash
cargo install crawlkit
```

## Usage

```bash
# Crawl a website
crawlkit crawl https://example.com --max-pages 50

# Reproducible crawl
crawlkit crawl https://example.com --seed 42

# Encrypted storage
crawlkit crawl https://example.com --encrypt

# Export metrics
crawlkit crawl https://example.com --metrics-json metrics.json
```

## Documentation

- [User Guide](https://wyattau.github.io/crawlkit)
- [CLI Reference](https://wyattau.github.io/crawlkit/cli-reference/)
- [GitHub Repository](https://github.com/WyattAu/crawlkit)
