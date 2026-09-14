# Heap Attribution — 10k Reference Workload (RSS-gap diagnostic)

**Date:** 2026-09-14
**Method:** `tests/heap_attribution.rs` under the `profiling` feature (dhat
0.3.3, `--release`, `--test-threads=1`): phase snapshots, analyzer-profile
ablation (Core vs Full on the identical 10k loopback workload), and a 10 ms
`curr_bytes` timeline. Raw record: `heap_attribution.json` (same directory).
**Scope limits:** dhat instruments the Rust allocator only — SQLite file
pages, mmapped data, and kernel page cache are invisible to it. `crawl_secs`
values under dhat are inflated ~5× and are not comparable to the capacity
report's throughput numbers; only the byte counters are evidence here.

## The numbers

| Instrument | Core profile (9 analyzers) | Full profile (default set) |
|---|---|---|
| Crawl-phase allocation churn | 21 048 MB | **78 889 MB** |
| Readback allocation | 74 MB | 1 295 MB |
| **Live-heap peak (`max_bytes`)** | **99 MB** | **1 547 MB** |
| Peak timing | during crawl | during crawl |
| Retained at readback | 58 MB | **840 MB** |
| Pages crawled | 10 001 / 10 001 | 10 001 / 10 001 |

Analyzer ablation share of crawl-phase churn: **73.3 %** (Full − Core ÷ Full).

## What this attributes

1. **The live-heap peak is analyzer work, not the page store.** The core
   profile holds the whole 10k-page crawl in ~99 MB of live heap at peak.
   The default (full) profile peaks at 1 547 MB — a ~1.45 GB delta owned by
   the full analyzer set's intermediates (per-page analysis vectors,
   issue-string building, DOM-derived structures held until the page is
   processed).
2. **~780 MB is retained after the crawl under the full profile** (840 MB
   vs the core profile's 58 MB). Something in the full path — post-crawl
   analyzer outputs, caches populated only by full-set analyzers, or issue
   strings — outlives the crawl loop. This retention is what drove the
   earlier in-memory-storage observation (full profile crossing the engine's
   then-hardcoded 512 MB cap mid-crawl; see below).
3. **RSS vs live heap:** the 10k reference report measured 2 115 MB process
   RSS against 1 547 MB live heap — the ~570 MB difference is allocator
   slack/fragmentation after 79 GB of churn through a ~1.5 GB live set
   (glibc malloc retains freed arenas). Lower churn *or* an allocator that
   returns memory more eagerly would shrink it.

## Incident found on the way (fixed)

The first full-profile attempt stopped at 4 507/10 001 pages: the engine's
resource monitor (`ResourceLimits::default()`) enforced a hardcoded 512 MB
memory ceiling with no way to configure it. Two changes landed:

- `CrawlEngineConfig.resource_limits` — limits are now pluggable per crawl
  (embedders can raise/disable limits; default behavior unchanged).
- The capacity harnesses disable only the memory cap when measuring, so a
  run reaches its natural peak and the record shows the gate outcomes.

## Engineering directions this evidence supports (6.0.0 RSS item)

Ranked by measured leverage:

1. **Analyzer intermediates (the 1.45 GB peak delta):** process page
   analysis incrementally — emit issues per page instead of accumulating
   per-page analysis vectors until the page completes; pool/reuse
   issue-string buffers. This is the dominant lever.
2. **Post-crawl retention (780 MB):** audit full-profile-only state that
   outlives the crawl (post-crawl analyzer outputs, caches); make it
   bounded or transient.
3. **Allocator behavior (~570 MB RSS-vs-live):** with churn reduced by (1),
   the slack shrinks proportionally; if not, evaluate jemalloc/mimalloc
   whose decay returns memory faster than glibc.
4. **Not the problem:** the SQLite page store — the core profile proves a
   10k crawl fits in ~100 MB of live heap with the same storage path.

## Reproduction

```bash
cargo build --release -p crawlkit-engine --test heap_attribution --features profiling
scripts/run_heap_attribution.sh   # ~66 min under dhat overhead, single-threaded
```
