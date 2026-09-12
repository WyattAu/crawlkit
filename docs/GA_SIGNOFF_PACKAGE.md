# Hosted Scanner — GA Sign-off Package

**Date:** 2026-09-12
**Prepared for:** the §7 ownership decision (docs/SCANNER_RUNBOOK.md §7)
**Asks of the reader:** sign the ownership line in §7, or name the gaps.

---

## 1. What is being proposed

That the hosted free-scan scanner move from `prototype` to
`stable-with-configuration` (capabilities.toml) under a named owner, per
docs/PRODUCT_STRATEGY.md §4 and the runbook GA checklist (§6).

Every code obligation in the GA checklist is implemented and CI-proven.
What remains are two human actions: observing N consecutive green CI runs
(they accumulate automatically from today) and the signature itself.

## 2. Scope discipline (read this first)

The scanner is **not** a competitor to commercial site-audit products, and
GA must not oversell it. Honest scope per PRODUCT_STRATEGY §1:

- What it is: a bounded, read-only scan of the submitted URL plus its
  same-host links (default 500 pages), analyzed with the engine's default
  analyzer registry, delivered via an unguessable token. No accounts, no
  auth, no JS rendering.
- What it is not: no link-graph database, no rank tracking, no keyword
  data, no scheduled/automated scans, no crawl of third-party hosts.
- Public copy must state the page cap and that it analyzes what it can
  fetch, not what a browser would render.

## 3. Evidence map: checklist item → where the proof lives

| Checklist item (runbook §6) | Status | Evidence |
|---|---|---|
| CI Redis integration suites green for N consecutive runs | In progress — green today | ADR-015 §7 suites (crash-recovery, concurrent sweep, retry exhaustion, budget windows) run on every push against Redis 7 + PostgreSQL services; latest run all 14 jobs green |
| Daily global scan budget + queue-full degradation | ✅ Implemented | `GlobalDailyBudget` (`abuse.rs`); per-UTC-day service-wide ceiling; shared Redis `INCR` in multi-replica, fail-closed on outage; explicit 429 + Retry-After |
| Per-URL results cache with TTL | ✅ Implemented | Token-addressable done-keys with the retention TTL; in-process store single-replica |
| Per-IP rate limit | ✅ Implemented | `IpRateLimiter`, fixed-window, bounded memory (10k IPs, stale-first eviction), X-Forwarded-For first hop; pinned by tests |
| Runbook drills | ✅ Completed 2026-09-12 | Runbook §6.1 drill log — five live drills against the real binary in the `redis` posture (abuse burst, daily-budget exhaustion, Redis outage, worker crash mid-scan, poison → dead-letter → operator redrive), all PASS |
| §7 ownership line | ⏳ Pending | The ask of this document |
| Public copy review (honest-scope) | ⏳ Pending | This document §2 is the draft; copy lands only after signature |

## 4. What the multi-replica posture guarantees (and how it's proven)

- **Fail-closed availability:** Redis outage → submissions error rather
  than being acknowledged; no silent in-process fallback. (Drill 3.)
- **At-least-once delivery:** leases with TTL reclamation; a `kill -9`'d
  worker's job is redelivered and completed idempotently. (Drill 4; the
  CI worker suite exercises this automatically.)
- **Poison quarantine:** jobs failing ×3 dead-letter with reason,
  attempts, ISO-8601 timestamps; the operator CLI lists and redrives
  them with attempts reset. (Drill 5.)
- **Bounded abuse surface:** per-IP fixed window and a daily global
  ceiling sit in front of the per-target politeness budget; exceeding a
  limit is an explicit 429 with a retry window, never a queue that eats
  requests. (Drills 1–2.)
- **Un-earnable trust boundary:** no environment-dependent private-IP
  bypass exists in the scanner (the engine's dev escape hatch is not
  compiled in); DNS-pinned fetch; plugins never load.

Each guarantee has an automated suite in CI plus the live drill; the
drill log records what was observed, not what was expected.

## 5. Known limits (stated, not hidden)

- Per-IP limiting is in-service; a shared proxy limit is still
  recommended for multi-replica deployments (runbook §6).
- Results are retained for the retention window and then deleted; there
  is no history, by design.
- The per-IP limiter trusts `X-Forwarded-For` first hop — safe only
  behind a proxy that overwrites it.
- Budget/rate constants are env-lowerable, code-capped — an owner can
  tighten, not loosen.

## 6. Residual risks accepted at signature

1. **Cost:** egress is the primary driver; the daily budget is the lever.
   Monitoring dashboards (runbook §4) are listed as GA-adjacent work, not
   a blocker, but should land in the first post-GA sprint.
2. **No auth:** token entropy is the only access control. Reviewed and
   accepted in ADR-012; nothing in the drills suggests revisiting.
3. **Reputational:** a scanner is a crawler against third-party sites;
   the politeness budget and honest public copy are the mitigations.

## 7. The ask

1. Read §2 (scope) and §4 (guarantees). If any guarantee reads as
   oversold, this package fails and GA waits.
2. Confirm the ops escalation path and cost visibility per runbook §7.
3. Sign the ownership line in docs/SCANNER_RUNBOOK.md §7 with name and
   date; quarterly review is scheduled automatically.
4. Only after signature: flip `hosted_scanner` to
   `stable-with-configuration` in docs/capabilities.toml and land the
   public copy.

Unsigned, the scanner stays `prototype` — the fallback is explicit and
costs nothing but the GA label.
