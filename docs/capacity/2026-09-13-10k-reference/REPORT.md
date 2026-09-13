# Capacity Report — 2026-09-13-10k-reference

**Workload:** `10000` pages · storage `sqlite-file` · concurrency 8 · delay 0 ms · build profile `release`
**Engine:** crawlkit 5.4.0 · mode `inline`
**Environment:** 11th Gen Intel(R) Core(TM) i9-11980HK @ 2.60GHz · 31 GB RAM · Linux 7.2.2-1-cachyos
**Runs:** 3 committed raw records (`run_record_run*.json`) · throughput spread 8.9% → set VALID

## Results (medians across runs)

| Metric | Median | Target (plan §2) | Verdict |
|---|---|---|---|
| Throughput | **64.2 pages/s** | ≥ 50 | ✅ met |
| Peak RSS | **2115 MB** | < 500 MB | ❌ missed |
| Crawl wall time | 155.7 s | — | — |
| Peak tasks | 26 | bounded, returns to baseline | ✅ (fd gate passed on every run) |

## Per-run records

| Run | Throughput | Peak RSS | Pages | fds start→end | All absolute gates |
|---|---|---|---|---|---|
| crawl-10000 @ 1789307469 | 64.2 p/s | 2115 MB | 10001 | 14→14 | ❌ (see gates in raw record)
| crawl-10000 @ 1789307647 | 59.5 p/s | 2077 MB | 10001 | 14→14 | ❌ (see gates in raw record)
| crawl-10000 @ 1789307812 | 64.8 p/s | 2116 MB | 10001 | 14→14 | ❌ (see gates in raw record)

## Honest notes

- Loopback serving inflates throughput vs the real internet by design; this is an engine-capacity number (plan §9).
- The RSS target is measured against the plan §2 row sized for the 1k CI class; at this workload class it is **missed** — the raw records above are the evidence, and the gap is a 6.0.0 engineering item, not a reporting artifact.
- Machine RAM exceeds the reference spec (32 GB vs 16 GB); CPU class matches. The record embeds both, so a reader can weigh it.
- Every number above is derived from the committed records; nothing is hand-copied.

