# Crawlkit Examples

This directory contains example programs demonstrating how to use crawlkit.

## Examples

### basic_crawl.rs

A minimal example showing how to crawl a single URL with default settings.

```bash
cargo run --example basic_crawl
```

### custom_analyzer.rs

Demonstrates how to implement a custom analyzer by creating an `ExternalResourceAnalyzer` that checks for excessive external links.

```bash
cargo run --example custom_analyzer
```

### api_usage.rs

Shows various programmatic use cases:
- Basic crawl with default settings
- Crawl with custom configuration (timeouts, delays, depth limits)
- Crawl with progress callbacks

```bash
cargo run --example api_usage
```

## Running Examples

From the workspace root:

```bash
# Run a specific example
cargo run --example basic_crawl

# Run all examples
cargo run --example basic_crawl && cargo run --example api_usage
```

## Example Features Demonstrated

- **Basic Crawling**: Simple URL fetching and storage
- **Custom Analyzers**: Implementing the `Analyzer` trait
- **Configuration**: Customizing crawl behavior
- **Progress Tracking**: Using callbacks for real-time updates
- **Incremental Crawling**: Using ETag/If-Modified-Since for efficient re-crawls
