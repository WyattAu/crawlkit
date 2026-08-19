# Migrating to crawlkit 3.0

Version 3.0.0 contains three breaking changes to the `crawlkit-engine`
public API. All three remove APIs that were dead, unsound, or speculative;
each migration is mechanical. Everything else in this release (security
hardening, OpenAPI docs, persistent audit/state) is additive.

## 1. `HtmlParser::parse` is now infallible

The underlying HTML5 parser is error-tolerant by design — malformed input
yields a best-effort DOM rather than an error, so the `Result` wrapper
could never carry `Err`. The `parser::ParseError` type never had a
constructor path and has been removed.

**Before (2.x)**

```rust
use crawlkit_engine::{HtmlParser, parser::ParseError};

match HtmlParser::parse(&body, &url) {
    Ok(page) => analyze(page),
    Err(e) => log::warn!("parse failed: {e}"),
}
```

**After (3.0)**

```rust
use crawlkit_engine::HtmlParser;

let page = HtmlParser::parse(&body, &url);
analyze(page);
```

`StreamingHtmlParser::parse` (the incremental variant) changed identically.
If you referenced `ParseError` in a signature, remove it — there is no
replacement because no failure mode existed.

## 2. `Analyzer::analyze` no longer takes `&CrawlConfig`

The second parameter was accepted and ignored by every one of the 33
built-in analyzers. Analyzer configuration now flows exclusively through
`AnalyzerRegistry` construction (`AnalyzerRegistry::new(&config)` /
`with_feature_flags`), where it selects *which* analyzers are registered.

**Before (2.x)**

```rust
impl Analyzer for MyAnalyzer {
    fn name(&self) -> &str { "my-analyzer" }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        // ...
    }
}

let findings = registry.analyze(&ctx, &config);
```

**After (3.0)**

```rust
impl Analyzer for MyAnalyzer {
    fn name(&self) -> &str { "my-analyzer" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        // ...
    }
}

let findings = registry.analyze(&ctx);
```

## 3. `CrawlError::Storage` is a structured type

`CrawlError::Storage(String)` flattened `StorageError` via `to_string()`,
destroying the error structure. It is now
`CrawlError::Storage(StorageError)` with a generated `From` conversion, and
a new `CrawlError::Internal(String)` covers non-storage subsystem failures
(I/O, environment) that previously masqueraded as storage errors.

**Before (2.x)**

```rust
// Constructing:
return Err(CrawlError::Storage(format!("failed: {e}")));

// Matching:
match err {
    CrawlError::Storage(msg) => /* string */,
    _ => (),
}
```

**After (3.0)**

```rust
// Constructing — use ? on StorageError (From is derived):
storage.start_crawl(&url, None)?;

// Or explicitly for non-storage failures:
return Err(CrawlError::Internal(format!("statm unreadable: {e}")));

// Matching:
match err {
    CrawlError::Storage(e) => /* e: StorageError (Database/PgDatabase/InvalidUrl/...) */,
    CrawlError::Internal(msg) => /* string */,
    _ => (),
}
```

## Unchanged in this release

- All 31 built-in analyzers, their finding codes, and severities
- `Storage`/`PgStorage` method signatures (the `PgStorage` runtime fix is
  internal: the sync-trait bridge no longer panics outside Tokio)
- CLI flags and `crawlkit.toml` configuration format
- REST API routes, request/response shapes, and OpenAPI paths

## Versioning policy going forward

From 3.0.0 onward, `cargo-semver-checks` runs against the latest tagged
release in CI; breaking API changes require a new major version.
