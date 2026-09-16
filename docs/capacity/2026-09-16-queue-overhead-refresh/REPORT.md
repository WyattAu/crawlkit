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

1. **Lease overhead is ≈ 14.5% at 2×5000 on current HEAD** (mean 97.7 vs
   114.2 p/s; spreads 2.8% and 2.1% respectively — well inside the ≤20%
   validity rule). This is a larger delta than the 2026-09-15 report's
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
