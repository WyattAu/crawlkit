# Capacity Report — 2026-09-15-distributed-posture

**Workload:** `distributed-crawl-2x5000` · 2 worker processes × 5 000-page
loopback hosts · **one shared PostgreSQL 16 instance** · build profile
`release`
**Engine:** crawlkit 5.4.0 · mode `distributed-posture` (named in the record;
never conflate with inline numbers)
**Environment:** 11th Gen Intel(R) Core(TM) i9-11980HK @ 2.60GHz · 31 GB RAM ·
Linux 7.2.4-1-cachyos
**Runs:** 3 committed raw records (`run_record_run*.json`) · throughput spread
3.0% → set VALID

## Results (across runs)

| Metric | Value | Gate (named before measurement) | Verdict |
|---|---|---|---|
| Aggregate throughput | **112.9–116.4 pages/s** | ≥ 40 p/s (2 × per-worker floor) | ✅ met |
| Per-worker throughput | 58.5 p/s each | ≥ 20 p/s | ✅ met |
| Worker RSS sum (peak) | **156–169 MB** | < 1 000 MB (2 × 500 MB) | ✅ met |
| Per-worker RSS peak | 83–85 MB | < 500 MB each | ✅ met |
| Pages exact | 10 002 = 2 × (5 000 + 1 index) | exact budget per worker | ✅ met |
| fds returned to baseline | 12 → 17 per worker (pg pool) | baseline + 20 | ✅ met |
| All gates | PASS on every run | — | ✅ |

## Topology (named in every record)

- **Workers:** 2 independent OS processes, each running the real
  [`CrawlEngine`] inline path against its own loopback host
  (127.0.0.1 / 127.0.0.2 — distinct hosts, the multi-domain shape).
- **Shared state:** one PostgreSQL 16 instance, schema reset before each run;
  both workers write concurrently through their own pools.
- **Queue:** inline-per-worker. The Redis lease queue is **not wired into the
  crawl frontier** — that integration is the remaining 6.0.0 engineering item
  (plan §8.4 harness note). This run measures the shared-*state* posture:
  multi-process footprint, aggregate throughput, concurrent write load.

## Findings

1. **Per-worker RSS drops ~6× vs the inline SQLite posture** (83–85 MB vs
   ~490 MB peak): with storage externalized to Postgres, the page cache and
   WAL no longer sit in the crawl process. Sizing implication: worker
   processes are cheap; the sizing problem moves to the database tier.
2. **Aggregate throughput ≈ 2 × a Postgres-backed single worker**, i.e. the
   shared instance is not the bottleneck at this scale — no throughput
   ceiling from concurrent writers was observable at 2 × 8 concurrency.
3. **fd behavior is clean**: each worker's pool adds ~5 fds and returns to
   baseline; no leak across the run.

## Honest notes

- Loopback serving inflates throughput vs the real internet by design; this
  is an engine-capacity number (plan §9).
- The queue claim is deliberately scoped: numbers here must not be cited as
  "distributed queue" numbers — the record's `topology.queue` field names the
  inline-per-worker path explicitly.
- Every number above is derived from the committed records; nothing is
  hand-copied.
