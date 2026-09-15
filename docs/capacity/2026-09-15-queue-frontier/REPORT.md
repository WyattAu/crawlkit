# Capacity Report — 2026-09-15-queue-frontier

**Workload:** `distributed-crawl-2x5000-queue` · 2 worker processes × 5 000-page
loopback hosts · one shared PostgreSQL 16 · **Redis 7 lease queue in the crawl
frontier** · build profile `release`
**Engine:** crawlkit 5.4.0 · record schema
`crawlkit.capacity.distributed_run_record/v1`, `topology.queue` names the
lease path — never conflated with the inline posture numbers
**Environment:** 11th Gen Intel(R) Core(TM) i9-11980HK @ 2.60GHz · 31 GB RAM ·
Linux 7.2.4-1-cachyos · quiet host (verified before and after the set)
**Runs:** 3 committed raw records (`run_record_run{1,2,3}.json`), gates named
in the harness before measurement, all PASS in every run.

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

## 100k headline run: not yet published

Two attempts: (1) truncated by the engine's 10k default page cap (defect 2,
now fixed), (2) stalled by stale-namespace pollution (defect 3, now fixed).
The next attempt is deferred until the host is quiet — a concurrent build/
test session on this machine currently violates the quiet-host requirement,
and publishing numbers measured under that contention would break the
evidence plan's discipline. The harness is ready; the run is mechanical.

## What this does not claim

- Not the 6.0.0 published class of numbers (no pinned reference machine —
  plan §8 open question).
- Not a multi-worker *shared-queue* topology: each worker owns its
  namespace; cross-worker lease stealing is exercised by the scanner worker
  suite, not here.
- Not a Postgres/Redis tier-sizing result: service-side resources were not
  sampled (Postgres checkpoints ran hot during the 100k attempt — a sizing
  observation for the runbook, not a gate).
