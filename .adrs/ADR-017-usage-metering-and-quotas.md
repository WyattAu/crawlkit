# ADR-017: Usage Metering and Quotas — Per-Tenant Accounting for the Scale Phase

**Status:** Accepted (2026-09-25; implemented for 6.0.0-alpha.3 — open questions resolved per the proposals below: refusals are telemetry, usage aggregate rows follow a 400-day default horizon via `purge_usage_before`)
**Date:** 2026-09-16
**Deciders:** Maintainers
**Related:** ADR-008 (API backpressure and idempotency), ADR-012 (hosted scanner — the refusals posture precedent), ADR-015 (Redis lease queue), ADR-016 (warehouse export schema contract), docs/PRODUCT_STRATEGY.md §3 (6.0.0 "Usage metering + quota surface" row; replaces aspirational docs/BILLING.md), docs/ROADMAP.md Phase 3.4

## Context

6.0.0 is the enterprise unlock, and its ladder names "usage metering +
quota surface (API + dashboard)" as a gate. Metering cannot be bolted on
after the fact: the recording point must exist where work is *accepted*,
not where it is billed, or the numbers will drift from reality under
retries, fan-out, and at-least-once delivery. The scanner (ADR-012)
already established the exhaustion posture this ADR generalizes:
explicit, machine-readable refusals — never silent truncation, never
best-effort accounting.

docs/BILLING.md is aspirational and predates a working system; this ADR
supersedes it as the source of truth and it should be removed or rewritten
against this decision at implementation time.

## Decision

### 1. Units of measure (what is counted, exactly)

| Unit | Definition | Recording point |
|---|---|---|
| `pages` | One page fetched *and analyzed* (ack'd in the crawl frontier sense) | Page write to storage, at `finish_crawl` rollup and incrementally at the storage insert |
| `crawl_started` | One crawl accepted (the single most billable event) | `StorageBackend::start_crawl` |
| `findings` | Findings persisted (the analyzer output volume, drives downstream warehouse cost) | Batched at `insert_issues_batch` |
| `export_bytes` | Bytes emitted by a warehouse export | `export_crawl_parquet` / `export_crawl_jsonl` return paths |
| `scan_submitted` (hosted scanner) | Scanner submissions accepted | Scanner `POST /scan` handler after budget checks |

Everything else (crawl wall-time, egress bytes, render count) is
*telemetry*, not metering: it feeds dashboards (Phase 5.1) but is not
billable, because it is not attributable per-tenant with the same
guarantees. Honest-scope rule: no number is "metered" unless its recording
point is deterministic under retry/failure.

### 2. Recording point and delivery guarantees

- Recording happens **inline with acceptance/persistence**, on the same
  storage transaction or immediately adjacent to it. A metered unit that
  was recorded but not performed is acceptable (rare, over-count); the
  reverse is not. This matches the at-least-once queue posture: dedupe is
  by logical key (`crawl_id`, `page_id`), so retries re-record idempotently.
- The metering record is an **append-only counter event**, written to the
  same storage backend as the work (SQLite/PG), schema:

  `(tenant_id, day_utc, unit, delta)` — one row per tenant-day-unit with
  `UPDATE ... delta = delta + n` upserts, plus a per-crawl detail row in
  `crawl_runs` (already tenant-scoped since 5.3.0 per-tenant retention).

- **No new external dependency.** Metering rides the existing storage
  backends; Redis is *not* required (the scanner's daily budget is the
  only Redis-counter case, already shipped in 5.4.0).

### 3. Quota surface (the enforcement half)

- Quotas are **per-tenant, per-UTC-day, per-unit**, configured via the API
  (`PUT /tenants/{id}/quotas`, an additive endpoint), stored in the
  tenants table (new nullable columns; migration is additive — no
  backfill).
- **Default posture: unmetered.** A tenant with no quota row has no limits
  — self-hosted single-tenant users must never meet a quota they did not
  set. Limits exist only where an operator (hosted deployment, enterprise
  tenant) creates them. This is the inverse of the scanner's posture
  (hard-capped by design) and is deliberate: the scanner is a public
  trust boundary; tenant quotas are a commercial control.
- **Exhaustion semantics — copied verbatim from the ADR-012 posture:**
  the API returns HTTP 402-class (`quota_exhausted`) with a
  machine-readable cause in the existing rejection-shape, in-flight work
  finishes, and new work is refused until the UTC boundary. Never
  truncate mid-crawl silently; a crawl that exhausts its quota mid-run
  *completes what it holds and records overage as telemetry* (the early-
  stop insight pattern from the resource monitor).

### 4. API and CLI surface (additive only)

- `GET /tenants/{id}/usage?from=&to=` — the metering read path (drives
  the dashboard chart).
- `PUT /tenants/{id}/quotas` / `GET /tenants/{id}/quotas` — set/read
  limits.
- Response shapes live beside the existing API types; semver gate must
  stay green (additive fields only).
- CLI: `crawlkit usage <tenant>` read-only mirror for operators.

### 5. Explicitly out of scope

- Billing, invoicing, payment integration (never in this codebase — see
  PRODUCT_STRATEGY §5 non-goals).
- Real-time quota enforcement across replicas via Redis (the storage-
  adjacent counter is the enforcement point; multi-replica enforcement
  arrives with the distributed-stability stream and follows the same
  fail-closed review as ADR-015 §6 if ever needed).
- Metering the self-hosted OSS default install (no tenant configured →
  no rows written; zero overhead for the primary OSS audience).

## Consequences

- Every acceptance point gains a cheap upsert (one indexed row per
  tenant-day-unit); the write amplification is bounded and measured in
  the capacity harness before alpha.3's stable gate.
- The dashboard Trends surface (6.x) gains its data source for free.
- Migrating the scanner's daily budget onto this system is *not* planned —
  the scanner's Redis counter is fail-closed and public-facing; tenant
  quotas are commercial. Unifying them would weaken the scanner's
  guarantee (fail-open when the metering backend lags).

## Alternatives considered

1. **Redis-first metering** (reject): adds a hard dependency to the
   core path for a commercial feature; contradicts the OSS-default
   no-overhead rule.
2. **Post-hoc aggregation from `crawl_runs`** (reject): cannot count
   `findings`/`export_bytes` without replay; aggregates are the read
   path, not the record.
3. **Event-log + downstream billing system** (reject for 6.0.0; revisit
   only with a hosted platform decision): correct at hyperscale, wrong
   at this phase's complexity budget.

## Open questions (resolved 2026-09-25 at implementation)

1. Does `pages` count robots-denied or SSRF-denied fetch attempts?
   **Resolved: no** (proposal adopted) — only analyzed pages count;
   refusals are telemetry. The unit vocabulary has no refusal units, and
   a unit test pins that so one cannot silently appear.
2. Retention of the usage rows: same as per-tenant retention config
   (5.3.0) or a fixed 400 days? **Resolved: fixed 400-day default for
   aggregate rows** (proposal adopted) — `purge_usage_before` sweeps
   counters; wiring it to the per-tenant retention scheduler is an
   alpha.4 follow-up if operators need per-tenant divergence.
