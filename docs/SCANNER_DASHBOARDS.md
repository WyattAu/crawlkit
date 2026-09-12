# Scanner Dashboards — the /metrics data plane, made observable

**Status:** Design (Phase 5.1 alignment for runbook §4; implementation is
dashboard-layer config, not engine code)
**Date:** 2026-09-13
**Data plane:** `GET /metrics` per replica (docs/SCANNER_RUNBOOK.md §4) —
cumulative JSON counters since process start. **There is no Prometheus
endpoint;** the dashboard layer scrapes per replica, aggregates, and computes
rates as deltas between consecutive scrapes. Every query below is written as
that delta math, so it binds to Grafana (JSON API datasource), a hosted
monitoring agent, or a cron + alert script equally.

**Scrape contract:** 15s interval, all scanner replicas, from the monitoring
network only (the runbook §4 exposure rule). One missed scrape is not an
alert (see `up` below); two consecutive misses page.

---

## Dashboard 1 — Service health (the GA §4 row)

**Panel: scan success rate** (5m window, all replicas)

```
rate = Δoutcomes.complete / (Δsubmissions.accepted)   over the window
```

- `outcomes.complete` = scans that finished with an analyzed report.
- Denominator is **accepted submissions**, not raw requests — rejected
  submissions never entered the scan path and are Dashboard 2's concern.
- `Δoutcomes.robots_blocked` counts as *successful service behavior*, not
  failure — a robots.txt block is the politeness contract working. Exclude it
  from the numerator and the denominator (it is a completed scan whose result
  is "blocked"; if your feed does not distinguish, treat it as success).

**Alerts (named per the runbook's "define thresholds pre-GA" metric):**

| Condition | Severity | Rationale |
|---|---|---|
| success rate < 90% for 15m (and accepted volume ≥ 10) | page | Below this, the free-scan surface is lying to users by existing |
| success rate < 97% for 1h | ticket | Slow decay: upstream fetch failures, registry regressions |
| `up == 0` for 2 consecutive scrapes on any replica | page | Replica dead; queue posture means no user-visible failure yet, but capacity is halved |

**Panel: p95 latency, submit → result-ready** (`latency_ms.scan_p95`)

- Queue posture: this includes queue wait and crash recovery — the number
  users actually experience. Inline posture (`inline_*`) is the fetch path
  only and is the baseline for what the queue adds.
- The reservoir is per-replica; aggregate by taking the **max** across
  replicas per scrape (a reservoir quantile is already order-statistic-safe;
  max-of-replicas is the conservative aggregate).

**Alerts:** scan p95 > 30s sustained 15m → ticket; > 120s → page
(the scanner serves a 25-page teaser; two minutes to a result means the
queue is backing up or a target is slow — either way users have left).

## Dashboard 2 — Rejections by cause (the attack-signal row)

All panels: `Δsubmissions.rejected_by_cause[cause] / minute` per cause, 5m
windows, aggregated across replicas. The enum is closed
(`ssrf_denied`, `daily_budget`, `ip_rate_limited`, `target_budget`,
`queue_busy`, `malformed_input`) — a cause appearing that the dashboard does
not know is itself an alert (schema drift).

**Alerts (runbook §4: "an SSRF-denial spike is an attack signal, page it"):**

| Condition | Severity | Rationale |
|---|---|---|
| `ssrf_denied` > 10/min sustained 5m | **page** | Distributed SSRF probing through the scanner; this is the attack signal the runbook names. > 2/min for 15m → ticket (early probe pattern) |
| `daily_budget` > 0 before 12:00 UTC | ticket | The service-wide ceiling is being burned at 2× expected pace; the cost lever may need lowering before midnight exhaustion |
| `daily_budget` exhaustion event (counter hits ceiling) | ticket | Expected degradation, but the runbook says document every exhaustion in the incident log |
| `ip_rate_limited` > 100/min | ticket | Abuse bursts are being absorbed — review top IPs, consider proxy-layer denies |
| `target_budget` > 50/min sustained 15m | ticket | One target is being hammered *through* us (runbook §5 row 1); identify via access logs, consider temporary denylist at the proxy |
| `queue_busy` > 0 sustained 10m | page | Fail-closed admission means users are being refused for capacity, not policy — scale workers or raise worker budget only after capacity review |
| `malformed_input` > 200/min | ticket | Usually a broken client or scanner-in-the-wild; noise, but worth identifying |

## Dashboard 3 — Budget saturation

**Panel: per-target politeness saturation** — the runbook §4 row reads
"observability via `PolitenessBudget::remaining`". The `/metrics` snapshot
deliberately carries **no per-target state** (the exposure rule: no URLs, no
per-actor data). Therefore this panel is served from the aggregate
`target_budget` rejection rate (Dashboard 2) plus Redis-side inspection when
investigating:

```
docker exec redis redis-cli keys "crawlkit:scanner:politeness:*"  # incident use only
```

The dashboard-level panel is the rejection rate trend; the Redis lookup is
the incident drill-down, documented here so the two are not confused.

**Panel: daily budget burn-down** — `Δsubmissions.accepted` cumulative since
00:00 UTC vs. the configured ceiling. Render as a burn-down against the UTC
midnight reset (the same `resets in Ns` window the API returns).

**Alert:** projected exhaustion > 6h before midnight (linear projection over
the trailing hour) → ticket. This is the early-warning version of the
`daily_budget` rule above.

## Dashboard 4 — Egress (the cost driver)

**Panel: egress bytes/day** — `Δegress_bytes_total` per UTC day, cumulative
intraday line. Application-level body bytes only (the counter includes
truncated reads, per the data-plane docs), so it is the honest cost proxy.

**Alerts (runbook §4: "alert on deviation from the established baseline"):**

| Condition | Severity | Rationale |
|---|---|---|
| any 6h window > 3× the trailing-7-day same-window mean | ticket | Anomalous volume: a big site being scanned repeatedly, or abuse |
| projected day-end > 2× trailing-7-day daily mean | ticket | The budget ceiling exists to bound this — if projection breaches it repeatedly, the ceiling is mispriced, not the users |

Baseline honesty: the first two weeks of GA *establish* the baseline; the
deviation alerts are armed only after 7 full days of data exist. Record the
arming date here when GA lands.

## Alert routing

- **Paging** (page severity): ADR-014 webhook channel, high-urgency
  (Slack/Teams webhook with @-mention convention per the API tier's channel
  config). Scanner-to-alert automated wiring is the 6.0.0 integration item —
  until then the dashboard layer's own alerting (Grafana/agent-native) sends
  to the same channels. That is acceptable and documented here as the
  interim routing contract.
- **Tickets:** low-urgency channel; reviewed in the ops cadence the §7 owner
  sets at acceptance.
- Every page must map to a runbook §5 playbook row before it is armed — the
  `ssrf_denied` page routes to the §5 abuse-burst row, `queue_busy` to the
  §5 queue row.

## Implementation checklist (dashboard layer, post-GA sprint)

- [ ] Scrape job: 15s, per-replica, monitoring network, auth header per the exposure rule
- [ ] Dashboard 1 (success rate, p95) + the three Dashboard 1 alerts
- [ ] Dashboard 2 (rejection rates) + the seven cause alerts, including the unknown-cause alert
- [ ] Dashboard 3 (budget burn-down + projection)
- [ ] Dashboard 4 (egress) — deviation alerts armed after 7 days of baseline
- [ ] Routing verified end-to-end with a synthetic fire drill (trigger a test page, confirm the webhook lands) — recorded in the §6.1 drill log
