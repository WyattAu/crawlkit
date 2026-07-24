# crawlkit-engine

Core library for [crawlkit](https://github.com/WyattAu/crawlkit) — a high-performance Rust web crawler for SEO analysis.

## Features

- 28 SEO analyzers (meta tags, content quality, security, accessibility, AI)
- Async HTTP/2 fetching with retry, redirect tracking, rate limiting
- HTML parsing with link, heading, image, and structured data extraction
- SQLite storage with WAL mode and batch operations
- WASM plugin system for third-party extensions
- AES-256-GCM encryption at rest
- Deterministic crawls with seed-based reproducibility

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
crawlkit-engine = "1.0.0"
```

## Documentation

- [API Documentation](https://docs.rs/crawlkit-engine)
- [GitHub Repository](https://github.com/WyattAu/crawlkit)
- [Security Guide](https://github.com/WyattAu/crawlkit/blob/main/docs/SECURITY.md)
