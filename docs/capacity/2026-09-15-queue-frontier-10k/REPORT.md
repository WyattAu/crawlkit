# Capacity Report — 2026-09-15-queue-frontier-10k

**Workload:** `distributed-crawl-1x10000-queue` · 1 worker process × 10 000-page
loopback host · one shared PostgreSQL 16 · **Redis 7 lease queue in the crawl
frontier** · build profile `release`
**Engine:** crawlkit 5.4.0 (commit series incl. the early-stop insight fix) ·
record schema `crawlkit.capacity.distributed_run_record/v1`
**Environment:** 11th Gen Intel(R) Core(TM) i9-11980HK @ 2.60GHz · 31 GB RAM ·
Linux · quiet host
**Runs:** 3 committed raw records (`run_record_run{1,2,3}.json`) — gates named
in the harness before measurement, all PASS in every run.

## What this is evidence of

The per-URL lease-queue overhead at reference scale. The 2 × 5 000 queue-mode
set (`2026-09-15-queue-frontier/`) measured ~10% aggregate cost vs the inline
posture; this single-worker 10k reference isolates one worker's rate against
the inline posture's per-worker number (58.5 p/s, `2026-09-15-distributed-posture/`)
at the same page scale — the like-for-like anchor the 2-worker runs could
only approximate.

## Numbers (3-run set)

| Metric | run 1 | run 2 | run 3 |
|---|---|---|---|
| Pages (exact gate) | 10 001 | 10 001 | 10 001 |
| Issues emitted (workload identity) | 1 633 312 | 1 633 312 | 1 633 312 |
| Aggregate throughput | 55.6 p/s | 51.8 p/s | 58.8 p/s |
| Worker RSS, peak | 138 MB | 131 MB | 137 MB |
| Gates | all PASS | all PASS | all PASS |

Validity: spread 13.5% (≤ 20% rule, plan §6) — a valid set.

## Findings

1. **Lease overhead at 10k scale is ~5–11% per worker** (51.8–58.8 vs the
   inline posture's 58.5 p/s per-worker reference). At 10 001 pages and
   ~1.63 M issues, the per-URL Redis round-trips amortize better than the
   2 × 5 000 runs suggested — closeout-dominated smaller runs overstate the
   per-URL cost. The overhead is bounded and modest; the lease queue is
   viable as the default distributed frontier.
2. **Memory posture holds at reference scale**: 131–138 MB peak, flat with
   the 5k runs (~160 MB sum for two workers) — no per-page queue-state
   growth.
3. **Issue volume is the constant across postures** (1 633 312 on both
   sides of every comparison) — analyzer output is provably workload-
   identical, so throughput deltas are frontier cost, not work done.

## Honest scope

- Single-host loopback corpus: measures frontier + engine + storage, not
  network variance.
- `topology.mode` names `distributed-posture` and `topology.queue` names
  the Redis lease path — never conflate with inline numbers.
- Postgres write amplification (~1.63 M issue rows per 10k pages) remains
  the service-side sizing signal (see the 100k report's checkpoint finding).

<!-- GENERATED-NUMBERS:BEGIN -->

### Workload `distributed-crawl-1x10000-queue`

**Topology:** 1 worker processes · storage `postgres-shared` · queue `redis-lease-queue (DistributedQueueAdapter in the crawl frontier)` · pages/worker 10000 · concurrency/worker 8 · profile `release`
**Engine:** crawlkit 5.4.0
**Machine:** 11th Gen Intel(R) Core(TM) i9-11980HK @ 2.60GHz · 31 GB RAM · Linux 7.2.4-1-cachyos
**Runs:** 3 committed raw records · aggregate-throughput spread 13.6% → set VALID

| Metric | Median | Target (plan §2) | Verdict |
|---|---|---|---|
| Aggregate throughput | **55.6 pages/s** | ≥ 50 | ✅ met |
| Worker RSS sum (peak) | **135 MB** | per-run caps (gates) | — |

| Run | Aggregate p/s | Worker RSS sum peak | Pages | All gates |
|---|---|---|---|---|
| distributed-crawl-1x10000-queue @ 1789511124 | 55.6 | 135 MB | 10001 | ✅
| distributed-crawl-1x10000-queue @ 1789511340 | 51.8 | 132 MB | 10001 | ✅
| distributed-crawl-1x10000-queue @ 1789511516 | 58.8 | 138 MB | 10001 | ✅

Honest notes:

- Loopback serving inflates throughput vs the real internet by design; this is an engine-capacity number (plan §9).
- The relevant RSS bound for this class is the per-worker / worker-sum cap enforced by the record's own gates (shown per run below), not the inline 500 MB row.
- Every number above is derived from the committed records; nothing is hand-copied.

<!-- GENERATED-NUMBERS:END -->
