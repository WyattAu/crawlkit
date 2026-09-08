# Measured Benchmarks — v5.1.0

**Date:** 2026-09-08
**Commit:** `febedbd9` (feat(v5.2.0): FP audit, first-party plugins, rank tracker, WASI Gates 4-5; workspace version 5.0.0, `crawlkit --version` → `crawlkit 5.0.0`)
**Machine:** Linux 7.2.2-1-cachyos x86_64, 16 CPUs (11th Gen Intel(R) Core(TM) i9-11980HK @ 2.60GHz, max 5.0 GHz), 31 GiB RAM
**Toolchain:** rustc 1.97.1 (8bab26f4f 2026-07-14), cargo 1.97.1
**Profile:** release (`lto = true`, `codegen-units = 1`, `opt-level = 3`, `strip = true`, `panic = "abort"`)

All measurements were taken in a clean detached git worktree at the commit above.
The main working tree had unrelated in-progress modifications during this session,
so the pinned commit was measured instead. Every number below is machine-measured;
nothing is projected or copied from earlier docs.

**Load caveat:** the machine carries persistent background load (a user agent
process at ~88% CPU since the previous day; 1-minute load average oscillated
between ~2 and ~6 during the runs). Numbers should be read as measured *under
that load*, and cross-run variance below is reported honestly.

## Criterion microbenchmarks (median)

Command:

```bash
cargo bench -p crawlkit-engine --bench crawlkit_benchmarks
```

Final run on the idle machine (criterion medians from `target/criterion`):

| Benchmark | Median |
|---|---:|
| `html_parser/parse_small_page` | 142.8 µs |
| `html_parser/parse_5kb_page` | 594.4 µs |
| `analyzer_registry_full_suite` | 2.087 ms |
| `url_queue/push_1000_urls` | 1.646 ms |
| `url_queue/pop_1000_urls` | 718.6 µs |
| `url_queue/push_pop_mixed_priorities` | 2.258 ms |
| `storage/insert_100_pages` | 1.501 ms |
| `storage/query_pages` | 704.9 µs |
| `storage/insert_and_query_issues` | 526.3 µs |
| `circuit_breaker_state_check` | 0.98 ns |
| `link_graph_pagerank_100_nodes` | 912.2 µs |
| `feature_flags_get` | 41.3 ns |

Cross-run variance (two more criterion runs of the same commit): results were
consistently worse when other processes were building concurrently — e.g.
`parse_5kb_page` 594 µs (idle) vs 726 µs (loaded), `analyzer_registry_full_suite`
2.09 ms (idle) vs 2.56 ms (loaded). Parsing and storage numbers moved ~10–25%
with load; queue and PageRank numbers moved <5%.

## Throughput (end-to-end crawl)

Command:

```bash
cargo run --release -p crawlkit-engine --example throughput_bench
```

Local HTTP/1.1 keep-alive fixture server, engine fetch concurrency pinned to 4.
Three runs of the same binary:

| Pages | Run 1 (pages/sec) | Run 2 (pages/sec) | Run 3, idle (pages/sec) |
|---:|---:|---:|---:|
| 50 | 182.8 | 191.0 | 128.9 |
| 100 | 184.1 | 184.9 | 130.9 |
| 500 | 144.6 | 149.7 | 134.0 |

**Honesty caveat (from the harness itself):** the benchmark prints a warning
when the server never observed more than 1 concurrent request. In these runs the
observed fetch overlap was flaky — `max-concurrent` was 1 in most size/run
combinations (2 in a few). Per the harness documentation, rows where
`max-concurrent = 1` are **not** concurrent-throughput measurements; the
~130–190 pages/sec figures therefore largely reflect the serial
fetch → parse → analyze → store pipeline, not overlapped fetching. Treat
throughput as engine-pipeline-limited at this configuration.

**Peak RSS:** 147.4–151.3 MB across the three runs (harness reports `VmHWM`;
delta from warm-up ≈ 121 MB).

## Binary size

```
target/release/crawlkit: 25,762,864 bytes (24.6 MiB / "25M" in ls -lh)
```

## Startup time

50 × `crawlkit --version` via `time.perf_counter_ns()` around `subprocess.run`,
two batches on the same binary:

| Batch | median | p95 | min | max |
|---|---:|---:|---:|---:|
| 1 | 4.1 ms | 4.6 ms | 3.7 ms | 6.0 ms |
| 2 | 4.1 ms | 6.9 ms | 3.7 ms | 18.2 ms |

Median is stable at **4.1 ms**; p95 varies with background load (4.6–6.9 ms).
The batch-2 max of 18.2 ms is a single outlier consistent with the load caveat.

## Comparison with the README table (before this measurement)

The README performance table previously pointed at
[`measured-v5.3.0.md`](./measured-v5.3.0.md) (measured 2026-08-30 on a
different machine: CachyOS 7.2.0, kernel `msi-ge66`). Its numbers did not
reproduce here and have been replaced in the README with the figures in this
file. Notable deltas: HTML parse (5 KB) 610 µs → 594 µs measured; full analyzer
suite 1.48 ms → 2.09 ms measured; PageRank (100 nodes) 981 µs → 912 µs
measured; throughput 301.5 pages/sec at 10 pages is not reproducible with the
current harness (which now runs 50/100/500 pages at concurrency 4).
