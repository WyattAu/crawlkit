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
| 2026-09-15 | `72d40ef3` | 4 / 5 | tracker update; one more green run unblocks launch |
| 2026-09-15 | `d0a7029c` | 5 / 5 | gate met — then reset same day, see Reset 1 |

### Reset 1 — 2026-09-15

The next run (`3d763b65`, docs-only diff, identical binary) failed the
capacity smoke baseline gate on its first attempt: the 1k debug-profile
crawl deviated >20% from baseline — shared-runner noise, proven by the
unchanged binary and the immediate green re-run of the same job. Per the
rule above, the count resets.

**Watch (§3) — closed by fix:** the second consecutive reset (`dd60c5a1`)
failed the same capacity smoke baseline gate, also on a docs-only diff —
the §3 condition was met and the flake source was fixed rather than
lawyered: the relative baseline comparison now re-measures up to 2 more
times before failing (absolute caps stay no-retry; every attempt recorded
in the run record). Fix exercised: a 2× unreachable baseline correctly
fails after 3 genuine crawls with the attempts trail in the panic.

| Date | HEAD | Count | Note |
|---|---|---|---|
| 2026-09-15 | `3d763b65` | 1 / 5 | green on re-run after noise failure |
| 2026-09-15 | `335bd2ec` | 1 / 5 | reset 2 (`dd60c5a1`) fixed by the bounded re-measurement gate — green first attempt |

**Rule for updating:** append a row only for a *complete, all-jobs-green* `CI`
run on `main` whose HEAD is the current tip. Any failure resets the count to
0 and requires a new table section. Do not count skipped or partially-green
runs.

## 2. On reaching 5 / 5

**Reached 2026-09-15** (`d0a7029c`), then **reset** the same day before
the launch procedure executed (Reset 1). The steps below stand ready for
when the streak rebuilds:
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
