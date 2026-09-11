# ADR-015: Redis Queue Graduation — Leases, Retries, and Poison Handling

**Status:** Accepted (implementation re-sequenced from 6.0.0 to its own 5.4.0 release by maintainer decision, 2026-09-11; ROADMAP Phase 2.3 full acceptance remains the release gate)
**Date:** 2026-09-11
**Deciders:** Maintainer decision ("pull Redis forward"), per docs/PRODUCT_STRATEGY.md §3
**Related:** ADR-008 (API backpressure and idempotency), ADR-012 (hosted scanner — requires shared-state politeness budget), docs/ROADMAP.md Phase 2.3 (distributed queue acceptance), docs/PRODUCT_STRATEGY.md §3 (6.0.0 "Redis queue graduation" pulled forward and given its own 5.4.0 release in the 5.3/5.4 split)

## Context

The experimental distributed queue
(`crates/crawlkit-engine/src/distributed_queue.rs`, gated behind
`full` + `unstable`) is a Redis sorted set with a fundamentally unsound
delivery contract for a system that must not lose or duplicate work:

1. **Destructive pop.** `pop()` issues `ZPOPMIN` — the entry is removed
   atomically at receive time. If the worker crashes after pop and before
   persisting the crawl result, that URL is silently lost. There is no
   recovery path.
2. **No failure classification.** A pop that leads to a permanent fetch
   failure (robots-denied, 410 Gone, SSRF denial) and one that leads to a
   transient failure (timeout, 5xx) are indistinguishable; neither is
   retried, neither is recorded.
3. **No poison handling.** An entry whose deserialization or processing
   always fails would be lost (today) or — worse — would retry forever
   (in any naive retry addition).
4. **O(N) membership.** `contains()` does `ZRANGE 0 -1` and linear-scans
   JSON — unusable as a visited-set at distributed-crawl scale.
5. **Sync client.** `redis::Commands` blocking calls inside async workers
   stall the executor under connection latency.

The 6.0.0 plan scheduled fixing all of this as "Redis queue graduation." The
maintainer decision to pull Redis forward into 5.3.0 exists for one driving
reason: **the hosted scanner's GA gate (ADR-012) requires a global
per-target politeness budget in shared state**, and a single Redis instance
is already the natural shared-state substrate for deployments that need
distributed crawling at all. Graduating the queue in the same release means
one shared-infrastructure work stream instead of two, and the scanner's
budget store rides the same connection, ordering, and crash-recovery
discipline as crawl entries.

Phase 2.3's full acceptance (documented delivery semantics, leases, retries,
poison handling, crash-recovery tests) remains the gate — the resequencing
changes *when*, not *how well*.

## Decision

Graduate the queue to a **lease-based, at-least-once delivery queue** with
poison-quarantine, on the existing Redis dependency (`redis` 0.25,
`tokio-comp` enabled in the workspace):

### 1. Leases replace destructive pops

- Popping moves to an atomic `ZPOPMIN`-into-lease pattern: the entry is
  moved from the pending sorted set into a per-worker **processing hash**
  keyed by lease ID, with a lease TTL (default: 120s, configurable) — all in
  a Lua script so the move is atomic and a crash between the two steps is
  impossible.
- A worker completes work by `HDEL`-ing its lease key. A lease that expires
  (worker crash, network partition) makes the entry eligible for **lease
  reclamation**: a periodic sweep scans the processing hash for expired
  leases and re-inserts those entries into the pending set.
- Consequence: delivery is **at-least-once**. Workers must tolerate
  duplicate delivery of the same URL. The engine's existing
  idempotency guarantees (ADR-008) and result upserts make re-crawling a
  URL safe; the documented contract states this explicitly.

### 2. Retry classification and bounded attempts

- Each entry carries an `attempt_count`. Terminal failure handlers classify
  outcomes using the same retryable/fatal taxonomy as webhook delivery
  (`loop_retry::IsRetryable` semantics): transient (timeout, 5xx, lease
  expiry) re-queues with the attempt count incremented; fatal (410, SSRF
  denial by ADR-012 policy, permanent DNS failure) does not.
- Retry backoff is encoded in the sorted-set score (next-eligible
  timestamp), reusing the existing score-means-priority convention:
  scheduled retries naturally surface later.
- `attempt_count > MAX_ATTEMPTS` (default: 5, configurable) routes the entry
  to the dead-letter set instead of re-queueing.

### 3. Poison quarantine and dead-letter surface

- Entries that fail deserialization, violate invariants, or exhaust
  attempts move to a **dead-letter zset** with the failure reason and
  timestamp, capped in size (default: 10,000 entries, oldest evicted).
- Dead-letter contents are inspectable via the API (operator surface) and
  re-drivable (operator can re-queue an entry after fixing a defect), so a
  poison message is a visible, actionable event — never a silent loss or an
  infinite retry loop.

### 4. Membership becomes O(1)

- A separate **visited set** (Redis `SET` with `SADD` returning
  membership, namespaced per crawl) replaces the linear-scan `contains`.
  Visited membership and queue push are decoupled: the visited set records
  "seen," the pending zset records "owed." The O(N) `contains`
  implementation is deleted.

### 5. Async client

- All queue operations move to `redis::aio::ConnectionManager`
  (multiplexed, auto-reconnect) behind the existing `queue_trait::Queue`
  abstraction. The sync `Commands` implementation is removed; no caller is
  left on blocking calls inside async contexts.

### 6. Shared-state primitives for the scanner budget

- The global per-target politeness budget (ADR-012: 30 requests / 10 min per
  target across all users) is implemented on the same Redis instance using
  a sorted-set sliding window keyed by target host, mutated via Lua for
  atomic check-and-consume. This is a primitive of this ADR, consumed by
  the scanner; the scanner's in-process `DashMap` budget store is replaced
  when GA configuration runs multi-replica.

### 7. Crash-recovery evidence (Phase 2.3 acceptance)

The graduation is not complete without tests proving the contract:

- Worker crash between pop and result-write → entry reclaimed after lease
  TTL and re-delivered (crash-recovery test).
- Duplicate delivery → no duplicate findings/results (idempotency test).
- Always-failing entry → dead-lettered after MAX_ATTEMPTS, never re-queued
  (poison test).
- Redis connection loss mid-operation → reconnect and continue; no
  entry loss beyond the lease-expiry window (partition test).
- Documented delivery semantics (at-least-once, ordering caveats: no
  cross-entry ordering guarantee) published alongside the code.

## Consequences

- 5.3.0 carries infrastructure work previously budgeted for 6.0.0; the
  6.0.0 "Scale" release keeps distributed-mode *stability graduation* and
  capacity evidence (10k/100k URL crawls) but drops its queue-semantics
  work item, which this ADR closes.
- At-least-once delivery is a behavioral change for any consumer of the
  `unstable` queue surface; the change is acceptable because that surface
  is explicitly unstable, and the new contract is strictly safer than
  at-most-once-with-loss.
- Redis becomes a load-bearing operational dependency for scanner GA and
  distributed crawls; deployment documentation must state Redis
  availability requirements (persistence configuration, failover
  expectations) and the scanner runbook (ADR-012 GA prerequisite) must
  cover Redis-outage degradation (fail closed: no budget record → no scan).
- The in-process scanner budget store and inline execution remain the
  single-replica path until this ADR ships; ADR-012's GA gate is
  satisfied by the combination of both ADRs.
