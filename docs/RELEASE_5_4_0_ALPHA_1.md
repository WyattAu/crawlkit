# crawlkit 5.4.0-alpha.1 — "Queue Graduation"

**Released:** 2026-09-12 · **Tag:** `v5.4.0-alpha.1` (pre-release) ·
**Full change log:** [CHANGELOG.md](../CHANGELOG.md)

This is the first rolling prerelease of the 5.4.0 "Queue and Scanner GA"
phase. It completes the code obligations of ADR-015: the Redis lease
queue graduates from experimental substrate to the scanner's execution
path, with an operator surface for the failure modes that used to be
silent.

> Pre-release: interfaces described here may still shift before 5.4.0
> stable. v5.3.0 "Connectors" remains the Latest stable release.

## The queue graduates

The engine's Redis queue now has the full at-least-once machinery behind
it, and every guarantee is pinned by CI suites that run against a real
Redis 7 service on every push:

- **Leases, not blind pops.** A worker holds a lease with a TTL; if the
  worker dies mid-job, a sweeper reclaims the abandoned lease and the
  job is redelivered. Crash-recovery is tested, not aspirational.
- **Bounded retries with classification.** Transient failures retry;
  poison entries are quarantined after 3 attempts instead of cycling
  forever.
- **O(1) membership** for queue bookkeeping, keeping operations cheap at
  depth.

## Operators can finally see the dead letters

Failure modes you can't inspect are failure modes you can't trust. Two
new commands (behind the `queue-ops` feature):

```
crawlkit queue dead-letter list            # NDJSON: reason, attempts, timestamp
crawlkit queue dead-letter redrive <INDEX> # re-queue with attempts reset
```

Each row carries the failure reason, `attempt_count`, and an ISO-8601
`dead_lettered_at` timestamp. Redrive restores the entry to the pending
queue with attempts reset to zero.

## The scanner fans out

In the multi-replica posture (`CRAWLKIT_BUDGET_BACKEND=redis`), scan
submissions now enqueue jobs instead of running inline, and any
replica's worker pool can execute them:

- The worker runs the **identical trust path** as inline execution —
  same DNS-pinned fetcher, same politeness budget, same analyzer
  registry, no plugin loading.
- Outcomes land in shared, TTL-bounded keys, so **any replica can serve
  any token**.
- Enqueue failures **fail closed**: a job that cannot be persisted is
  never acknowledged to the submitter (503, nothing lost silently).
- Duplicate delivery is idempotent — an at-least-once system, honestly
  labeled.

Also in this posture: a **daily global scan budget** (per-UTC-day
service-wide ceiling, explicit 429s with the reset time) and a
**per-IP rate limit** (fixed window over the proxy's first
`X-Forwarded-For` hop) sit in front of the per-target politeness budget.

## Five live drills, all passing

Before this prerelease, the runbook GA checklist was exercised live
against the real binary in the multi-replica posture (runbook §6.1):
abuse burst, daily-budget exhaustion, Redis outage (fail-closed),
worker crash mid-scan (lease reclaimed, job redelivered, completed
exactly once), and poison → dead-letter → operator redrive.

## Trying it

Binaries and checksums are attached below (18 assets; `checksums.txt`
is signed — verify before use). The queue features need the `queue-ops`
build feature; the scanner's multi-replica posture additionally needs a
Redis endpoint and `CRAWLKIT_BUDGET_BACKEND=redis`. Every setting that
is wrong fails loudly at startup, naming the accepted values.

## What's next

5.4.0 stable closes when: the GA checklist's human items complete
(consecutive green CI runs accumulate on their own), the scanner
runbook's §7 ownership line is signed, and the capabilities manifest
flips `hosted_scanner` to `stable-with-configuration`. If ownership is
not accepted, the scanner stays `prototype` and everything else in this
release ships regardless — see docs/GA_SIGNOFF_PACKAGE.md for the
decision material.
