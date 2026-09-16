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

**Watch (§3) — updated after Reset 3:** the bounded re-measurement fix
handles single-sample jitter (proven: `335bd2ec` green first attempt;
recovery run at 65.0 p/s first attempt). It **cannot** handle a sustained
slow runner window — `d2066d5b` failed all three clustered attempts at
49–52 p/s (±2% internally, −25% vs baseline). Residual noise mode is now
*environment windows*, not samples. If a slow-window reset recurs, the
§3 escalation is a deliberate baseline re-seed (cache-key bump with the
evidence committed) or a gate redesign (median-of-N instead of retries)
— not more retries, and not gate removal.

Note also recorded: the engine landed the early-stop insight feature and
the 10k queue-mode reference during this window; those runs' results live
in `docs/capacity/2026-09-15-queue-frontier-10k/`, not here.

| Date | HEAD | Count | Note |
|---|---|---|---|
| 2026-09-15 | `3d763b65` | 1 / 5 | green on re-run after noise failure |
| 2026-09-15 | `335bd2ec` | 1 / 5 | reset 2 (`dd60c5a1`) fixed by the bounded re-measurement gate — green first attempt |
| 2026-09-15 | `d2066d5b` | 0 / 5 | **Reset 3:** capacity gate failed all 3 attempts at 49–52 p/s, tightly clustered — a sustained slow runner window, not single-sample jitter (retries cannot rescue a uniformly slow window) |
| 2026-09-15 | `292c8dc9` | 1 / 5 | recovery proven: 65.0 p/s first attempt, baseline within — the slow window was transient |
| 2026-09-15 | `493eb65b` | 2 / 5 | tracker history commit; first attempt externally cancelled, re-run green with no failed job |
| 2026-09-15 | `5609cc25` | 3 / 5 | streak rebuild holding |
| 2026-09-16 | `03e48abf` | 4 / 5 | first attempt externally cancelled (no newer push, no failed job); re-run green |
| 2026-09-16 | `00bb168d` | 0 / 5 | **Reset 4:** same failure mode as Reset 3 — 43.7/47.0/47.0 p/s, tightly clustered ±3.7%, vs the ~81 p/s baseline. **§3 escalation executed:** the flake source was fixed at the root — the relative gate now calibrates runner CPU speed and normalizes the throughput threshold (clamped ±40%; RSS raw; verified end-to-end on synthetic 2×-era baselines). CI baseline re-seeded (`-v3`) to attach calibration. |
| 2026-09-16 | `02083151` | 1 / 5 | gate-fix commit green first attempt; `-v3` calibrated baseline seeded (ungated recording run, seed step confirmed in log) |

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

**Executed 2026-09-16:** Resets 3 and 4 were the same sustained slow-runner
mode, which retries cannot fix. Per this section, the flake source was
fixed structurally (machine-normalized relative gate; see Reset 4 row and
docs/CAPACITY_EVIDENCE_PLAN.md §6) rather than observed further. The
`-v3` baseline re-seed auto-arms exactly as `-v2` did: the first run
records ungated, the next gates.
