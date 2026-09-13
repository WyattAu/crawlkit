# Scanner GA — §7 Sign-off Request

**To:** the named ops/product owner for the hosted scanner decision
**From:** engineering
**Date:** 2026-09-13
**Decision requested by:** end of the current sprint (the engineering gate
has been met since 2026-09-12; the streak requirement cannot "ripen" further
— this is now the only open item)
**Time cost:** ~15 minutes (this page + §2 and §7 of the sign-off package)

---

## The one-line ask

Sign the ownership line in `docs/SCANNER_RUNBOOK.md` §7 to move the hosted
free-scan scanner from `prototype` to `stable-with-configuration`, or reply
with the specific gap that blocks you.

## What you are approving

- The scanner runs as a public, no-auth free scan: bounded read-only crawl of
  the submitted URL plus same-host links (default 500-page cap), results
  behind an unguessable token, auto-expiring. Honest-scope copy ships with
  GA — it is a teaser, not a site audit, and the copy says so.
- You own the ops commitment (runbook §7): cost visibility, escalation path,
  quarterly review. The daily global scan budget is your cost lever — it is
  env-tightenable by you, code-capped against loosening.

## Why it is safe to approve (evidence, in reading order)

1. **`docs/GA_SIGNOFF_PACKAGE.md`** — the 15-minute decision document: scope
   discipline first (§2), then the guarantee-by-guarantee evidence map (§3),
   accepted residual risks (§6), and the exact ask (§7).
2. **Runbook §6.1 drill log** — five live drills against the real binary in
   the Redis posture: abuse burst, daily-budget exhaustion, Redis outage
   (fail-closed), worker `kill -9` mid-scan (lease reclamation, idempotent
   completion), poison → dead-letter → operator redrive. All PASS with
   recorded observations.
3. **GA gate** — the N=5 consecutive-green CI rule (counting rule in the
   runbook) is **met and recorded**: 5/5 per the rule, Service-backed
   PostgreSQL+Redis job verified on each.
4. **Abuse surface** — per-IP fixed window, daily global budget with explicit
   429 + Retry-After degradation, per-target politeness budget, no
   environment-dependent SSRF bypass, DNS-pinned fetch, plugins never load.
5. **Monitoring** — `/metrics` data plane live (success rate, rejection by
   cause, budget, egress bytes, p95 submit→ready latency);
   `docs/SCANNER_DASHBOARDS.md` turns it into the concrete dashboard and
   alert rows for the first post-GA sprint.

## What happens on each answer

- **Signed:** engineering flips `hosted_scanner` → `stable-with-configuration`
  in `capabilities.toml`, lands the honest-scope public copy, deploys with
  `/metrics` proxy-restricted, and schedules the quarterly review. One release,
  checklist already written.
- **Gaps named:** they go into the runbook checklist as new items with owners;
  GA waits, honestly recorded as deferred — no silent slippage.
- **Unsigned:** the scanner stays `prototype`, nothing deploys, and the
  free-scan top-of-funnel every SaaS competitor leads with stays undeployed.
  That is a market cost, not an engineering one — it is worth stating plainly
  in the reply if this is the choice.

## One caution (so the signature is informed)

The per-IP limiter trusts `X-Forwarded-For` first hop — safe only behind a
proxy that overwrites it; a shared proxy rate limit is still recommended for
multi-replica deployment (runbook §6, GA_SIGNOFF_PACKAGE §5). Approving GA
means accepting that deployment constraint, not just the code.
