# Core/Full Feature Migration

Status: first implementation slice complete; follow-up boundaries remain (2026-09-02)

## Goal

Provide a small `crawlkit-core` build without Wasmtime and optional
integration runtimes, while preserving the existing `full` feature and its
public API for compatibility.

## Current baseline

The first migration slice now passes both library and CLI compilation:

```bash
cargo check -p crawlkit-engine --no-default-features
cargo check -p crawlkit --no-default-features
cargo check -p crawlkit-engine --features full
cargo check -p crawlkit --features full
```

Core mode excludes the full-only plugin, post-crawl, crawl-map, and integration
modules. The `CrawlConfig` LLM field remains source-compatible through a small
core-safe placeholder, and `AnalysisContext` uses a unit rendered-page
placeholder in core mode. Full mode retains the concrete Playwright type and
all existing behavior.

Full-only integration tests, benchmarks, and examples are declared with
`required-features = ["full"]`, so core lint/check jobs do not attempt to
compile targets whose public APIs are intentionally unavailable. Full-mode
workspace tests continue to exercise those targets.

This slice also produced reproducible release measurements:

| Artifact | Build flags | Stripped size |
|---|---|---:|
| Core CLI | `--no-default-features` | 2,575,640 bytes (2.46 MiB) |
| Full CLI | `--features full` | 25,579,176 bytes (24.39 MiB) |

The core artifact meets the sub-10 MB target. The full artifact does not, by
design, because it includes the Wasmtime/plugin and integration runtime. The
full/core distinction must remain explicit in packaging and public claims.

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
6. Add CI checks for core engine/CLI compilation and full workspace behavior.
7. Measure separate release artifacts and only then revise the 10 MB target.

Completed in the first migration slice:

- plugin index/runtime, crawl-map, insights, and post-crawl analyzer exports are
  gated behind `full`;
- core-safe `LlmConfig` and `AnalysisContext::rendered` boundaries compile;
- `RenderedPageSummary` provides an owned, serializable contract and
  `RenderedPage::summary()` adapts full browser output;
- full-only examples, benches, and integration tests are explicitly gated;
- core and full library/CLI compile checks pass;
- core and full stripped release sizes are measured reproducibly.

## Acceptance criteria

- Core engine compiles with no full-only dependencies.
- Full workspace behavior remains unchanged.
- Public full-feature exports remain available.
- Core and full tests run independently in CI.
- Core release artifact size is measured reproducibly.
- Plugin functionality is unavailable in core with a clear compile/runtime
  explanation, rather than silently degraded.

No feature split should be merged until the above criteria are met.