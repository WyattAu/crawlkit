# Hosted Scanner Runbook (ADR-012 GA Decision Document)

**Status:** Draft — GA decision pending ownership acceptance (see §7)
**Date:** 2026-09-11
**Applies to:** `crawlkit-scanner` crate, ADR-012 (trust boundary), ADR-015 (queue/budget §6, posture §A1)
**Purpose:** This document is simultaneously the Phase 5.3 runbook prerequisite
for scanner GA and the instrument for the GA decision itself. It does not
assume GA; it makes the operational cost explicit so ownership can be
accepted or declined with full information. If no owner signs §7, scanner GA
does not happen, and no code written for it is wasted (the scanner remains a
self-hostable library and the budget work feeds distributed crawling).

---

## 1. Service summary

| Property | Value |
|---|---|
| Surface | `POST /scan` (submit URL), `GET /scan/{token}` (fetch result), `GET /healthz` |
| Auth | None (public, unauthenticated) |
| Trust boundary | Public internet → scanner service (ADR-012 invariants) |
| Caps per scan | ≤ 25 pages, depth 4, 30 s wall clock, 2 MB bodies, ≤ 30 requests/target/10 min (global) |
| Result retention | 24 h, then deleted; unguessable 128-bit tokens |
| Degradation | Queue-full / budget-exhausted → explicit 429-style response, never silent truncation |

The scanner is a **teaser, not an audit** (ADR-012 honest-scope note). All
public copy must say so. Conversion path: scan → email capture → self-host
or API. The scanner must never be described as a site audit.

## 2. Deployment postures (ADR-015 §A1)

### Single replica (prototype → initial GA)

- Budget: in-process store (the prototype's current behavior). Budget resets
  on process restart; no cross-replica accounting. Acceptable because one
  replica cannot multiply its own budget.
- Selection: default, or explicit `CRAWLKIT_BUDGET_BACKEND=in-memory`.
- Scan execution: inline (in-process). No queue dependency; Redis is **not
  required** in this posture.
- Failure mode: process restart clears budget state; acceptable window is
  bounded by the 30 req/10 min/target limit's short window.

### Multi-replica (post-adoption)

- Budget: `PolitenessBudget` (Redis-backed, ADR-015 §6) — **required**.
  Fail-closed: no budget record → no scan. A Redis outage takes the scanner
  down by design; do not add a bypass.
- Selection: `CRAWLKIT_BUDGET_BACKEND=redis` plus `CRAWLKIT_REDIS_URL`,
  built with the `shared-budget` feature. Startup fails hard when the
  backend value is unknown, the URL is missing, or the feature is absent —
  a typo can never silently degrade the anti-abuse guarantee (pinned by
  tests in `crates/crawlkit-scanner/src/api.rs`). The selected posture is
  logged at startup (`budget_backend` field) for runbook verification.
- Scan execution: requires the lease queue for fan-out; ADR-015 §5 surface
  exists, worker wiring does not yet (tracked below).
- Recommended: Redis with Sentinel/failover; persistence config documented
  in the deployment manifest before enabling this posture.
- Dead-letter operations (ADR-015 §3): `crawlkit queue dead-letter list`
  and `crawlkit queue dead-letter redrive <INDEX>` against the same Redis
  (`CRAWLKIT_REDIS_URL`).

## 3. Anti-abuse architecture (what is already enforced)

Denial by construction, not blocklist discipline (ADR-012):

1. **DNS-pinned SSRF denial** — resolved addresses are filtered inside the
   resolver hook; no connection (any hop, any retry) can receive a
   non-approved address. Redirects re-validated per hop.
2. **Global per-target budget** — 30 req/10 min/target across all users;
   per-IP limits alone are insufficient (distributed abuse of one victim).
3. **Hard bounds** — page count, depth, wall clock, body bytes; no plugins
   ever load (the registry is analysis-only, outside the ADR-003 threat
   model).
4. **No environment bypass** — the scanner's guard does not honor
   `CRAWLKIT_ALLOW_PRIVATE` (unlike the engine's dev escape hatch).

Known gaps before GA (all pre-GA, tracked):

- [ ] Daily global scan budget with queue-full degradation (strategy §4) —
      not yet implemented; single-replica posture needs a process-level cap.
- [ ] Results cache per URL with TTL (strategy §4) — not yet implemented;
      repeat scans of one URL currently cost full budget each time.
- [ ] Per-IP rate limiting in front of `POST /scan` (reverse-proxy layer is
      acceptable; must exist before public exposure).

## 4. Monitoring and alerting

Minimum dashboards before GA (Phase 5.1 alignment):

- Scan success rate and p95 latency (submit → result-ready)
- Rejection rates by cause: SSRF-denied, budget-exhausted, queue-full,
  malformed input — an SSRF-denial spike is an attack signal, page it
- Budget saturation per top target (observability via
  `PolitenessBudget::remaining`)
- Egress bytes/day — the primary cost driver; alert on deviation from the
  established baseline

Alert channels route through the same infra as everything else (ADR-014
once implemented; webhook delivery exists today).

## 5. Incident playbook (abridged)

| Event | Immediate action |
|---|---|
| Abuse burst (one target hammered) | Verify budget rejection rate ↑; if a target is being attacked *through* us, consider temporary target allowlist/denylist at the proxy; document in incident log |
| SSRF-denial spike | Treat as attack recon; review logs for novel bypass shapes; file security advisory if any request reached a private address (this must be zero, always) |
| Redis outage (multi-replica posture) | Scanner fails closed — this is correct. Restore Redis; do not deploy the in-process fallback to "keep it up" |
| Cost overrun (egress) | Lower `MAX_PAGES`/budget constants (single config change), announce in changelog; constants exist precisely so this is one line |

Full incident process: docs/SECURITY.md and the maintainer security policy.

## 6. GA checklist (all boxes required before public launch)

- [ ] CI Redis integration suites green for N consecutive runs (ADR-015 §7
      evidence: crash-recovery, concurrent-sweep, retry-exhaustion, budget
      windows) — currently pending first real execution
- [ ] Daily global scan budget + queue-full degradation implemented
- [ ] Per-URL results cache with TTL implemented
- [ ] Per-IP rate limit at the proxy (or in-service) implemented and tested
- [ ] Runbook drills: abuse burst + Redis outage tabletop completed once
- [ ] §7 ownership line signed (name + date); ops escalation path confirmed
- [ ] Public copy reviewed against the honest-scope rule (§1) and the
      claims policy (capabilities.toml status must move `prototype` →
      `stable-with-configuration` only after the above)

## 7. Ownership acceptance

Scanner GA is an ops commitment, not a code milestone. The owner accepts:

- Egress and compute cost visibility and a monthly review
- Being in the escalation path for scanner incidents
- The daily-budget cost lever (may lower limits at their discretion)

```
Owner:     ______________________
Accepted:  ______/______/______
Review:    quarterly, first review 3 months after acceptance
```

**If this section remains unsigned, the scanner stays at `prototype` status
and public exposure does not happen.** The engineering work retains full
value: the guard, budget, and queue modules serve self-hosted deployments
and the 6.0.0 distributed-crawl work regardless.

---

*Per the claims policy: nothing in this document claims GA; the
capabilities.toml `hosted_scanner` status changes only when the checklist
above is complete and the ownership line is signed.*
