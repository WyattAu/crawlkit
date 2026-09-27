# Queue-Overhead Refresh — 2026-09-16 (post-early-stop-insight HEAD)

**Purpose:** re-anchor the Redis lease-queue overhead against the inline
frontier **on the same commit** (the early-stop insight + 10k reference
work landed after the 2026-09-15 `distributed-posture` report). Paired
3-run sets, same machine, same session, ~30 minutes apart.

**Workload:** 2 worker processes × 5 000-page loopback hosts, one shared
PostgreSQL 16 instance, release profile, crawlkit 5.4.0 (HEAD at
`a23ef600`).
**Environment:** i9-11980HK (as prior reports); local containers
`cap-postgres` (127.0.0.1:5499) + `cap-redis` (127.0.0.1:6379).
**Harness:** `capacity_distributed.rs`; queue mode = `CAPACITY_DIST_QUEUE=1`
(verified in each record's `topology.queue` — the first attempt of this
session accidentally ran with the wrong env name and produced an *inline*
record, which was kept as comparator data rather than mislabeled).

## Results (gates: all pass in all six runs; pages exactly 10 002 each)

| Set | Run | Aggregate throughput | RSS peak / worker |
|---|---|---|---|
| inline (engine `UrlQueue`) | 1 | 117.4 p/s | 81.3 / 86.8 MB |
| inline | 2 | 112.6 p/s | 86.6 / 88.6 MB |
| inline | 3 | 112.6 p/s | 86.4 / 87.0 MB |
| queue (`DistributedQueueAdapter`) | 1 | 98.7 p/s | 78.7 / 79.0 MB |
| queue | 2 | 95.9 p/s | 87.9 / 81.2 MB |
| queue | 3 | 98.6 p/s | 82.9 / 79.8 MB |

## Findings

1. **Lease overhead is ≈ 12.5% median-to-median at 2×5000 on current HEAD**
   (plan §5.6 publishes the median: inline median 112.6 p/s vs queue median
   98.6 p/s; mean-to-mean ≈ 14.4%. Set spreads 4.3% and 2.9% (max−min over
   min) — inside the §5.6 ≤15% set-validity rule). This is a larger delta than the 2026-09-15 report's
   ~10% at the same topology and the ~5–11% inferred at 10k; the honest
   reading is that the overhead scales with per-page Redis round-trips and
   the earlier estimates were taken in faster runner windows.
2. **RSS is unchanged by queue mode** (~78–89 MB/worker both postures) —
   the adapter adds no measurable memory cost.
3. **Absolute posture numbers stay with the 2026-09-15 reports** (never
   conflated across sessions/machines); this report contributes only the
   *paired ratio* measured in one session, which is the number the
   scaling argument needs.

## Raw records

`run_record_inline_{1,2,3}.json`, `run_record_queue_{1,2,3}.json`
(schema `crawlkit.capacity.distributed_run_record/v1`; gates and
`topology.queue` distinguish the sets — cross-check `topology.queue`
when citing).

<!-- GENERATED-NUMBERS:BEGIN -->

**Engine:** crawlkit 5.4.0
**Machine:** 11th Gen Intel(R) Core(TM) i9-11980HK @ 2.60GHz · 31 GB RAM · Linux 7.2.4-1-cachyos

### `inline` set — 3 runs (topology: 2 workers · inline-per-worker (engine's in-process UrlQueue))

| Run | Aggregate p/s | Worker RSS sum peak | Pages | All gates |
|---|---|---|---|---|
| run 1789580642 | 117.4 | 164 MB | 10002 | ✅
| run 1789581237 | 112.6 | 171 MB | 10002 | ✅
| run 1789581327 | 112.6 | 169 MB | 10002 | ✅

Median aggregate throughput: **112.6 pages/s** (range 112.6–117.4, spread 4.3% → set VALID); median worker RSS sum peak 169 MB.

### `queue` set — 3 runs (topology: 2 workers · redis-lease-queue (DistributedQueueAdapter in the crawl frontier))

| Run | Aggregate p/s | Worker RSS sum peak | Pages | All gates |
|---|---|---|---|---|
| run 1789580898 | 98.7 | 154 MB | 10002 | ✅
| run 1789581022 | 95.9 | 165 MB | 10002 | ✅
| run 1789581125 | 98.6 | 159 MB | 10002 | ✅

Median aggregate throughput: **98.6 pages/s** (range 95.9–98.7, spread 2.9% → set VALID); median worker RSS sum peak 159 MB.

### Paired overhead (computed from the medians above)

Lease-queue overhead ≈ **12.5%** median-to-median (plan §5.6: publish the median; inline median 112.6 p/s vs queue median 98.6 p/s). The paired ratio is the citable number; cross-session absolute comparisons remain invalid by the plan's rules.


Honest notes:

- Loopback serving inflates throughput vs the real internet by design; this is an engine-capacity number (plan §9).
- The relevant RSS bound for this class is the per-worker / worker-sum cap enforced by the record's own gates (shown per run below), not the inline 500 MB row.
- Every number above is derived from the committed records; nothing is hand-copied.

<!-- GENERATED-NUMBERS:END -->
