# Capacity Report — 2026-09-15-queue-frontier

**Workload:** `distributed-crawl-2x5000-queue` · 2 worker processes × 5 000-page
loopback hosts · one shared PostgreSQL 16 · **Redis 7 lease queue in the crawl
frontier** · build profile `release`
**Engine:** crawlkit 5.4.0 · record schema
`crawlkit.capacity.distributed_run_record/v1`, `topology.queue` names the
lease path — never conflated with the inline posture numbers
**Environment:** 11th Gen Intel(R) Core(TM) i9-11980HK @ 2.60GHz · 31 GB RAM ·
Linux 7.2.4-1-cachyos · quiet host (verified before and after the set)
**Runs:** 3-run valid set (`run_record_run{1,2,3}.json`, summarized in full
in the table below; raw records retained with the harness artifacts per the
evidence plan) — gates named in the harness before measurement, all PASS in
every run — plus the 100k headline run (`run_record_100k.json`, §100k).

## What this is evidence of

The Redis lease queue (ADR-015) is wired into the engine's crawl frontier via
`DistributedQueueAdapter` (`queue` config slot): every URL the engine
dispatches is a queue pop holding a lease, acknowledged on success and
reported into the retry/backoff/dead-letter path on failure. This is the
first capacity evidence of ADR-015 in the crawl path — until now the
distributed evidence named an inline-per-worker frontier.

Failure-path semantics exercised by construction: leases, idempotent
processing, and the failure-kind taxonomy (`failure_kind` /
`failure_is_fatal`) are pinned by 13 Redis-backed queue tests + 4 adapter
tests run serially in CI (`REDIS_URL` service suite).

## Numbers (3-run valid set)

| Metric | run 1 | run 2 | run 3 |
|---|---|---|---|
| Aggregate throughput | 103.9 p/s | 93.9 p/s | 90.7 p/s |
| Per-worker throughput | 52.0 / 52.0 | 47.0 / 47.0 | 45.4 / 45.3 |
| Pages (exact-match gate) | 10 002 | 10 002 | 10 002 |
| Issues emitted (workload identity) | 1 633 444 | 1 633 444 | 1 633 444 |
| Worker RSS sum, peak | 161 MB | 158 MB | 162 MB |
| Worker RSS sum, end | 153 MB | 150 MB | 154 MB |
| FDs per worker (end, baseline 12) | 18 | 18 | 18 |
| Tasks peak (sum) | 54 | 54 | 54 |

Spread 14% (rule: ≤ 20% → valid set). Identical issue counts across runs
confirm workload identity.

**Queue cost:** the same topology without the queue in the frontier
(`docs/capacity/2026-09-15-distributed-posture/`) ran 112.9–116.4 p/s.
Median-to-median, the Redis frontier costs **~18% aggregate throughput**
(93.9 vs 114.7) — one Redis round-trip pair per URL (pop + ack), amortized
across ~20 s of fetch-and-analyze work per worker-batch. Memory is
indistinguishable between postures (~160 MB sum peak). That is the honest
price of at-least-once delivery, crash reclamation, and poison quarantine in
the crawl path.

## Harness defects found by this round (each fixed in the harness)

1. **Empty-env fallback trap.** The parent forwarded `CAP_DIST_REDIS` as an
   empty string when unset; the worker's `unwrap_or_else` never fired on
   empty, so `DistributedQueue::new("")` failed. Fixed: empty is treated as
   unset.
2. **Engine resource limits truncate large runs.** The engine's default
   `ResourceLimits` cap `max_pages` at 10 000 and `max_memory_bytes` at
   512 MB; when tripped, the crawl ends early — with a `tracing::warn!` and
   an internal metric only. The first 100k attempt ended at 20 214 pages
   with both workers at exactly 10 000 pages and the frontier still holding
   39 894 ready entries. The harness now clears `max_pages` /
   `max_memory_bytes` via `set_default_limits` (the harness owns gating), and
   the 10k-class numbers already published this week are unaffected (they
   ran at or under the cap).
   **Product finding:** the resource-limit stop is invisible in
   `CrawlOutput` — a user crawl can end early with nothing user-facing
   saying so. Fix is an output-shape change (semver care) and is filed for
   the next minor.
3. **Stale queue namespaces reference dead ports.** Queue keys
   (`crawlkit:cap-dist-{worker}:*`) survive between runs, but the loopback
   test server binds a new ephemeral port each run. Queue pops are
   score-ordered oldest-first, so a second 100k attempt ground ~20k stale
   entries from earlier runs through fetch failures, circuit-breaker
   openings, and backoff requeues before touching fresh URLs — one worker
   effectively stalled (2 088 backoff-scored entries, 8 leases awaiting
   TTL reclamation) while its partner finished all 50 002 pages. Fixed: the
   worker clears its namespace before the crawl (`dq.clear()`), and the
   finding is recorded for operator runbooks: **rolling deployments must
   namespace or drain stale queue entries when URL lifetimes are tied to
   deployment-scoped services.**

## 100k headline run: PASS

After both harness defects were fixed, the sharded headline run completed on
a quiet host (loadavg ≈ 1.2 on 8 cores/16 threads throughout — sidecar
sampler attached):

| Metric | Value |
|---|---|
| Pages | **100 002 (exact-match gate PASS)** — 2 × 50 001 |
| Aggregate throughput | 57.0 p/s (1 755 s) |
| Per-worker | 28.5 / 28.5 p/s — perfectly balanced |
| Issues emitted | 16 332 480 |
| Worker RSS sum, peak | 729 MB (345 + 385); end 587 MB |
| FDs | 12 → 18 per worker, returned to baseline |
| Gates | all 7 PASS |

Two honest observations against the 10k set (90.7–103.9 p/s, ~80 MB/worker):

1. **Per-worker throughput halves at 50k pages** (28.5 vs ~46–52 p/s) and
   per-worker RSS roughly quadruples (345–385 vs ~80 MB peak). The driver is
   `finish_and_report`'s per-crawl closeout: Postgres-side issue storage
   and the per-crawl index/aggregate work scale with the crawl's issue
   count (8.2M per worker here), not with instantaneous memory. This is a
   real sizing curve — workers are cheap per 10k pages, and the per-crawl
   closeout is the dominant cost at scale — and it is now *measured*, not
   assumed.
2. **Postgres is the scaling surface**, confirming the distributed-posture
   finding: during the 100k run the DB tier logged `checkpoints are
   occurring too frequently (24 seconds apart)` — the first service-side
   sizing signal captured by this harness. A production deployment at this
   class tunes `max_wal_size` / checkpoint spacing before worker count.

The resource-limit finding from the first truncated attempt is directly
relevant: with the engine's 10k-page default still in place, this run would
have silently stopped at 20 214 pages. Large-crawl operators must raise
`ResourceLimits` explicitly — and the engine telling them so in
`CrawlOutput` remains the open product fix.

## What this does not claim

- Not the 6.0.0 published class of numbers (no pinned reference machine —
  plan §8 open question).
- Not a multi-worker *shared-queue* topology: each worker owns its
  namespace; cross-worker lease stealing is exercised by the scanner worker
  suite, not here.
- Not a Postgres/Redis tier-sizing result: service-side resources were not
  sampled (Postgres checkpoints ran hot during the 100k attempt — a sizing
  observation for the runbook, not a gate).
