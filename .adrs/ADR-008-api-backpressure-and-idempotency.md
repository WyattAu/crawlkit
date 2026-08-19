# ADR-008: API Backpressure, Idempotency Keys, and Bounded Concurrency

## Status

Accepted (2026-08-19)

## Context

The crawlkit API server received an unbounded number of concurrent crawl
tasks from `POST /api/v1/crawls`. Each crawl consumes significant resources
(Tokio tasks, HTTP connections, in-flight fetch semaphore, storage writes,
memory for page data). Without backpressure, bursty traffic or a retry storm
from a slow client could exhaust the host — a classic ECN-inspired concern.

Additionally, HTTP clients naturally retry POST requests on transient failures.
Without idempotency semantics, a client retry would launch a duplicate crawl
against the same target, wasting resources and producing inconsistent results
in the shared `crawl_results` map.

The original audit identified both gaps as ECN-violations (ADR-003 discussion
reference: backpressure was listed as "Planned"; idempotency as "Planned").
This ADR documents the shipped solution.

## Decision

### Backpressure (bounded concurrency)

A `tokio::sync::Semaphore` guards concurrent crawl submissions:

```
POST /api/v1/crawls → try_acquire_owned() → Ok(permit) | Err → 503 + Retry-After: 30
```

- **Default capacity**: 4 concurrent crawls (`MAX_CONCURRENT_CRAWLS` env var).
- **Mechanism**: `try_acquire_owned()` (non-blocking). Returns immediately;
  no queuing. If no permit is available, the handler returns `503 Service
  Unavailable` with a `Retry-After: 30` header — explicit backpressure
  rather than silent degradation.
- **Permit lifetime**: `OwnedSemaphorePermit` is held by `run_crawl_task`
  and released on drop (both success and error paths), guaranteeing the
  semaphore slot is always reclaimed even on panics (should not happen, but
  defensively correct).
- **Scheduler behavior**: scheduled crawl dispatch (`run_scheduler`) also
  acquires a permit via `try_acquire_owned()`. On capacity exhaustion the
  cycle is skipped with a `tracing::warn` and a
  `"skipped_at_capacity"` status on the `CrawlResult`. The next interval
  retries naturally.

Why non-blocking (503) rather than blocking (queue): the crawl pipeline is
long-lived (minutes to hours). A queue would either delay the client
indefinitely or require a bounded queue with opaque queue-depth semantics.
503 + Retry-After is the standard HTTP mechanism for load shedding and is
introspectable by load balancers, clients, and Prometheus alert rules
(`crawlkit-at-capacity` in `monitoring/alerts.yml`).

Why `OwnedSemaphorePermit` rather than `Arc<Semaphore>` in the task: ownership
transfer makes the release unconditional and eliminates the risk of a task
accidentally dropping the semaphore reference early.

### Idempotency Key (POST /api/v1/crawls)

Clients may supply an `Idempotency-Key` header (string, max 256 bytes).

- **Scope**: per crawl-start request. Not a request-level idempotency key.
- **Window**: 24 hours. Stale entries are opportunistically pruned on
  insertion (bounded scan over the `DashMap`; at most 24h × submission rate
  entries in practice).
- **Behavior**:
  - **First submission** with a given key: normal processing; key →
    `(crawl_id, created_at)` stored in `idempotency_keys`.
  - **Replay within window**: returns `200 OK` with the original
    `CrawlResponse` (same `crawl_id`, `"running"` status). No new crawl is
    started.
  - **Replay after window**: stale entry removed, treated as a fresh
    submission with a new `crawl_id`.

Why 24h: covers the common client-retry scenarios (CI timeouts, browser
reloads, webhook delivery retries) without excessive state retention. The
in-memory store is appropriate because idempotency is a transient guarantee;
on restart, clients receive a fresh `crawl_id` (which is acceptable since
the crawl state is also ephemeral in-memory until `storage_crawl_id` is
set on completion).

Why not Redis-backed (ADR-008 future note): multi-node deployments sharing
a Redis instance would persist across processes, but the current single-
process architecture does not justify the added infrastructure. The trait
seam (`ApiStateStore`) exists for that future.

## Consequences

**Positive:**
- Crawl capacity is operator-visible and tunable via env var.
- Retry storms are harmless (idempotency prevents duplicate work;
  backpressure prevents resource exhaustion).
- `Retry-After` header gives clients actionable information (standard HTTP).
- 503 is a signal that load balancers and autoscalers can act on.

**Negative / Trade-offs:**
- `503` is an outage signal to naive health checks. Operators must ensure
  their health probes hit `/health`, not `POST /crawls`.
- Scheduler skipping means scheduled crawls can miss an interval. Acceptable
  because the next interval fires; the alternative (blocking the scheduler
  loop on capacity) would delay ALL schedules.
- In-memory idempotency keys do not survive restart. Documented trade-off.
- The 503 `Retry-After` value (30s) is a heuristic. Under sustained load,
  clients retrying at exactly 30s may perpetually hit 503. Real deployments
  should add jitter.

## Alternatives Considered

1. **Unbounded queue + admission at drain time** — rejected: hides the
   capacity problem; makes monitoring harder (no `503` metric); a full queue
   creates unbounded memory growth before any rejection.

2. **Blocking semaphore (`acquire_owned().await`)** — rejected: ties up a
   Tokio task waiting for a slot; the client gets no response until a crawl
   finishes (minutes-hours). HTTP timeouts would fire first.

3. **Redis-backed idempotency** — deferred: adds infrastructure dependency
   not justified for the current single-process deployment model. The
   `ApiStateStore` trait makes this a drop-in future upgrade.

4. **Exponential backoff in the 503 response** — considered but rejected:
   the `Retry-After` header is the HTTP-standard mechanism; embedding
   backoff state on the server (per-client tracking) violates the stateless
   property of the current architecture.

## References

- `docs/SLO.md` — Objective 3 (capacity) and Objective 2 (completion rate)
- `monitoring/alerts.yml` — `CrawlkitAtCapacity` alert rule
- `crates/crawlkit-api/src/types.rs` — `DEFAULT_MAX_CONCURRENT_CRAWLS`,
  `IDEMPOTENCY_WINDOW`, `crawl_capacity_from_env()`
- `crates/crawlkit-api/src/handlers/crawls.rs` — `start_crawl()`,
  `run_crawl_task()`
- `crates/crawlkit-api/src/handlers/schedules.rs` — scheduler best-effort
  acquisition
- `scripts/verify-release-readiness.sh` — does not gate on this ADR
