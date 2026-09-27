# Capacity Report — 2026-09-13-smoke-1k

<!-- GENERATED-NUMBERS:BEGIN -->

**Workload:** `1000` pages · storage `sqlite-in-memory` · concurrency 8 · delay 0 ms · mode `inline` · build profile `debug (CI smoke)`
**Engine:** crawlkit 5.4.0
**Machine:** 11th Gen Intel(R) Core(TM) i9-11980HK @ 2.60GHz · Linux 7.2.2-1-cachyos

| Metric | Value | Target | Verdict |
|---|---|---|---|
| Throughput | **90.0 pages/s** | ≥ 20 (CI smoke floor) | ✅ met |
| Peak RSS | **298 MB** | < 500 MB | ✅ met |
| Pages | 1001 | exact budget | ✅ |
| fds returned to baseline | 11 (baseline 11) | +10 after settle | ✅ |

Single-shot CI smoke record (relative-gate baseline candidate); absolute numbers from CI are never published (plan §6).

Honest notes:

- Loopback serving inflates throughput vs the real internet by design; this is an engine-capacity number (plan §9).
- The RSS target is the plan §2 row sized for the 1k CI class; at this workload class it is **met**.
- Every number above is derived from the committed records; nothing is hand-copied.

<!-- GENERATED-NUMBERS:END -->
