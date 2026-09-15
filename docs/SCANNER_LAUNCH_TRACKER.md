# Scanner Public-Launch Tracker (post-GA)

**Created:** 2026-09-15 · **Owner:** see SCANNER_RUNBOOK §7 (signed)
**Purpose:** track the single remaining gate between GA (signed) and public
launch — the §6 CI-streak rule — and hold the launch-day checklist so the
launch itself is a recorded, boring procedure.

## 1. The gate

Per runbook §6: public launch waits for **5 consecutive green CI runs on
`main`** (the `CI` workflow, which includes the Redis + PostgreSQL service
suites), rebuilding after the 2026-09-14/15 fmt/semver fix cycle that reset
the previous streak.

| Date | HEAD | Count | Note |
|---|---|---|---|
| 2026-09-15 | `8072352b` | 1 / 5 | all jobs green; first after the fix cycle |
| 2026-09-15 | `47b04cac` | 2 / 5 | queue-frontier wiring landed |
| 2026-09-15 | `a05f71e6` | 3 / 5 | 100k queue run evidence landed |

**Rule for updating:** append a row only for a *complete, all-jobs-green* `CI`
run on `main` whose HEAD is the current tip. Any failure resets the count to
0 and requires a new table section. Do not count skipped or partially-green
runs.

## 2. On reaching 5 / 5

1. Deploy the scanner in the `redis` posture (runbook §2): behind a proxy
   that overwrites `X-Forwarded-For`, with the shared proxy rate limit —
   the constraint accepted with the §7 signature.
2. Verify: `GET /healthz` live, one real self-scan end-to-end, `/metrics`
   scraped by the dashboard (docs/SCANNER_DASHBOARDS.md).
3. Publish the GA_SIGNOFF_PACKAGE §2 copy **verbatim** — it is the only
   approved description of the free scan.
4. Record the deployment in runbook §6.1 (drill log) with date + revision.
5. First quarterly posture review: **2026-12-15** (runbook §7).

## 3. If the streak keeps resetting

Two consecutive reset cycles (5 failures preventing a 5/5 build) mean the
gate is masking a real stability problem: stop pushing non-essential changes,
fix the flake source, and re-plan. The gate is a floor, not a stopwatch.
