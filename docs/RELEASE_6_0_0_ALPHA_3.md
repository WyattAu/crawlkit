# crawlkit 6.0.0-alpha.3 — "Metering"

**Released:** 2026-09-26 · **Tag:** `v6.0.0-alpha.3` (pre-release) ·
**Full change log:** [CHANGELOG.md](../CHANGELOG.md)

The third rolling prerelease of the 6.0.0 "Scale" phase. Usage metering
and quotas — ADR-017, accepted 2026-09-25 — land end to end: per-tenant
accounting at the acceptance points, an explicit quota surface, and a
refusal posture that mirrors every other budgeted surface in crawlkit.
The exit criteria (ADR accepted before merge, semver check green against
`v6.0.0-alpha.2`, quota exhaustion pinned by tests) are green on the
tagged commit per the cadence rule.

> Pre-release: interfaces described here may still shift before 6.0.0
> stable. v5.4.0 "Queue Graduation" remains the latest stable release.

## Headline: usage is accounted, quotas are explicit, refusal is machine-readable

Until now, multi-tenant operators had no way to answer "what did tenant X
cost today?" or to cap an individual tenant without capping the fleet.
Both arrive in 6.0.0-alpha.3, with the same design law the renderer and
the scanner already obey: **degrade explicitly, never silently.**

### Units of measure (ADR-017 §1)

Five metered units, each with a deterministic recording point so retries
re-record idempotently (dedupe by logical key) — an over-count is
possible, an under-count is not:

| Unit | Recorded when | Dedupe key |
|---|---|---|
| `pages` | a page is fetched and analyzed | tenant + UTC day rollup |
| `crawl_started` | a crawl is accepted | crawl ID |
| `findings` | analyzer output is persisted | tenant + UTC day rollup |
| `export_bytes` | warehouse export emits bytes | crawl ID |
| `scan_submitted` | a hosted-scanner submission is accepted | — (quota-surface-ready; recording lands with hosted multi-tenant deployment — the public scanner has no tenant attribution by design, ADR-012 §5) |

### Default posture: unmetered

No quota row means no limits. Self-hosted users — the OSS default —
never meet a quota they did not set, and un-tenanted work records
nothing (zero overhead on the default path). Operators opt tenants in
per unit:

```console
$ curl -X PUT https://api.example.com/api/v1/tenants/acme/quotas \
    -H 'Authorization: Bearer …' -H 'Content-Type: application/json' \
    -d '{"crawl_started": 100, "pages": 10000}'
```

### Exhaustion is a 402 with a cause, not a truncation

When a tenant's daily `crawl_started` quota is spent, crawl submission
refuses up front:

```json
HTTP/1.1 402 Payment Required

{
  "cause": {
    "cause": "quota_exhausted",
    "tenant_id": "acme",
    "unit": "crawl_started",
    "limit": 100,
    "resets": "next UTC day"
  }
}
```

In-flight work finishes; overage records as telemetry. Quota exhaustion
never silently truncates a running crawl — the same explicit-degradation
posture as `RENDER001` and the scanner's daily budget.

### Reading usage back

```console
$ curl https://api.example.com/api/v1/tenants/acme/usage?from=2026-09-20
{"entries":[{"unit":"pages","day_utc":"2026-09-25T00:00:00Z","total":8417}, …]}

$ crawlkit usage --db crawl.db --tenant acme --from 2026-09-20
```

Usage rows roll up per tenant per UTC day per unit
(`usage_counters` table); quotas persist in `usage_quotas`. Retention:
a 400-day default horizon via `purge_usage_before`. Both tables are
storage-adjacent — no new external dependency.

## Storage and API surface (all additive)

- Engine: `crawlkit_engine::metering` (`MeteredUnit`, `MeterEvent`,
  `Quota`, `QuotaVerdict::{Allowed, Partial, Exhausted}`) — full-gated.
- Storage: `record_usage`, `get_usage`, `get_usage_today`, `set_quota`,
  `get_quota`, `purge_usage_before` on the `StorageBackend` trait
  (Unsupported defaults, matching the rank-tracking pattern).
- API: `GET/PUT /api/v1/tenants/{id}/quotas`, `GET
  /api/v1/tenants/{id}/usage?from=&to=`; unmetered units read as absent
  fields, not `null`. Refusals carry `ApiError::QuotaExhausted` → 402.
- CLI: `crawlkit usage` (read-only mirror, full-gated).
- Audit: `TenantUpdated` events cover quota changes.

## Numbers, honestly stated

- Analyzer registry: **779** analyzers (unchanged — metering adds no
  analyzers; drift-gated)
- Engine lib suite: **4 133** tests green on the tagged commit, including
  11 metering unit tests, 5 storage metering tests, and 4 router
  integration tests pinning refusal shape, unmetered default, usage read
  path, and quota validation
- API semver: **green** against the `v6.0.0-alpha.2` baseline (additive
  endpoints, one `ApiError` variant, one audit variant)

## docs/BILLING.md is superseded

The aspirational Stripe guide predates a working system; billing
integration remains a non-goal for this codebase (PRODUCT_STRATEGY §5).
ADR-017 is the source of truth for usage accounting; the old guide is
retained for historical context with a supersession banner.

## What's next

The remaining gate between here and 6.0.0 stable is the stable-gate
pair: the pinned 8-core/16 GB reference machine (operator-owned; see
docs/REFERENCE_MACHINE_SPEC.md) and the 10k/100k capacity evidence
package. Phase 6.2/6.3 release-gate practice continues at each alpha
cut.
