# Core/Full Feature Migration

Status: design groundwork only (2026-09-02)

## Goal

Provide a small `crawlkit-core` build without Wasmtime and optional
integration runtimes, while preserving the existing `full` feature and its
public API for compatibility.

## Current baseline

`cargo check -p crawlkit-engine --no-default-features` currently fails because
core-visible modules still reference full-mode concerns. The observed failure
surface includes storage, post-crawl analyzers, plugin index/runtime, the
Playwright rendered-page type, LLM configuration, and full-only dependencies
such as reqwest, TOML, tracing, and parking_lot.

This is an architectural boundary issue, not a dependency-size issue. The
stable default feature path must remain unchanged until each boundary has a
compile and behavior test.

## Proposed feature layers

1. **core** — types, parser, analyzers, deterministic in-memory analysis, and
   no Wasmtime/plugin execution.
2. **full** — current default behavior: HTTP crawling, SQLite storage,
   post-crawl systems, plugin runtime, Playwright, and integrations.
3. **wasi-preview2**, `postgres`, `observability`, and `profiling` — remain
   additive optional features.

## Migration sequence

1. Split `CrawlConfig` from full-only LLM configuration using a core-safe
   configuration type.
2. Move plugin index/runtime and plugin CLI commands behind `full`.
3. Introduce core-safe storage traits and a minimal in-memory implementation.
4. Split `AnalysisContext`'s rendered-page type behind a feature-neutral
   abstraction.
5. Gate post-crawl, insights, and integration modules consistently.
6. Add CI checks for `cargo check -p crawlkit-engine --no-default-features`,
   `cargo check -p crawlkit-engine --features full`, and CLI full builds.
7. Measure separate release artifacts and only then revise the 10 MB target.

## Acceptance criteria

- Core engine compiles with no full-only dependencies.
- Full workspace behavior remains unchanged.
- Public full-feature exports remain available.
- Core and full tests run independently in CI.
- Core release artifact size is measured reproducibly.
- Plugin functionality is unavailable in core with a clear compile/runtime
  explanation, rather than silently degraded.

No feature split should be merged until the above criteria are met.