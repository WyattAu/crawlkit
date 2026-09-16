# ADR-016 Open Questions — Resolution Evidence (2026-09-16)

**Context:** ADR-016 §5 left three questions "resolve before
implementation." This document resolves all three with measurement on a
real corpus — the 5 001-page / 816 711-findings Postgres corpus from the
2026-09-16 queue-overhead capacity runs (raw record:
`adr016_open_questions.json`, schema
`crawlkit.adr016.open_questions/v1`, emitted by
`crates/crawlkit-engine/tests/adr016_open_questions.rs`).

## Q1: Parquet compression/encoding defaults — RESOLVED: keep zstd default

| Binding | Size |
|---|---|
| Parquet findings (`Compression::ZSTD(Default::default())`) | **21.4 MB** |
| Parquet pages | 0.2 MB |
| JSONL findings (same rows) | 351.9 MB |

**Compression ratio vs the JSONL binding: ≈ 16.5×.** Export time 14.8 s
for 816 711 finding rows on the reference workstation — comfortably inside
the capacity plan's 120 s 100k-class gate (scaled: this corpus is 1/12 of
the 100k fixture and used 12% of the gate). The shipped default needs no
tuning for v1; re-measure only if the 100k export gate is ever approached.

**Latent bug fixed while measuring:** `PgStorage::get_pages` /
`get_pages_for_tenant` bound `usize::MAX` (the "everything" sentinel the
warehouse exporter passes) as a negative `LIMIT` — `LIMIT must not be
negative` on Postgres. Fixed by clamping to `i64::MAX` in the bridge; the
SQLite path was already correct.

## Q2: Partitioning guidance for BQ/SF — RESOLVED: recommended, not enforced

**Day-partition on `fetched_at`, cluster by `crawl_id`.** Rationale from
the measured workload shape: findings queries in the export contract are
crawl-scoped (`WHERE p.crawl_id = $1` in every access path), so `crawl_id`
clustering prunes per-crawl scans; day partitioning on `fetched_at` bounds
retention purges (per-tenant retention configuration from 5.3.0) and keeps
partition counts proportional to crawl cadence, not crawl size. Written
into the record's `guidance` field; ADR-016 §2.4 status discipline means
this is a recommendation in docs, never enforced by the writer.

## Q3: Incremental exports — already closed in schema v1

The nullable monotonic `sequence` column shipped in `schemas/export/v1/`
`crawl_runs` (ADR-016 §5.3 accepted into v1), so a later incremental-export
addition is non-breaking. No further action for v1; revisit only when a
consumer needs deltas.

## Reproduction

```bash
docker run -d --name cap-postgres -e POSTGRES_USER=crawlkit \
  -e POSTGRES_PASSWORD=crawlkit -e POSTGRES_DB=crawlkit \
  -p 127.0.0.1:5499:5432 postgres:16
# …run the queue-overhead capacity harness to seed the corpus…
DATABASE_URL=postgres://crawlkit:crawlkit@127.0.0.1:5499/crawlkit \
  cargo test -p crawlkit-engine --features postgres,warehouse \
  --test adr016_open_questions -- --ignored --nocapture
```
