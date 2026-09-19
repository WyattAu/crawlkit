# crawlkit 6.0.0-alpha — Rolling Prerelease Plan

**Status:** Proposed (maintainer decision to adopt) · **Date:** 2026-09-16
**Precedent:** the 5.3.0 cadence decision — ship the signed-release pipeline
as rolling `alpha` cuts on small increments rather than one big-bang stable
gate; each cut practices signing, SBOM, and rollback on real artifacts.
**Scope source:** PRODUCT_STRATEGY §3 "6.0.0 — Scale" ladder. This document
sequences it; it does not weaken any ROADMAP gate.

## Why rolling alphas for 6.0.0

6.0.0 is the enterprise unlock: distributed stability + capacity evidence,
warehouse exporters, render budgets, usage metering. These are four
independent risk streams; coupling them into one stable cut repeats the
release-order mistake the 5.3/5.4 split already fixed. Each alpha below is
independently useful, independently revertible, and increments the evidence
base the 6.0.0 stable gate requires.

## The cuts

### 6.0.0-alpha.1 — "Warehouse Contracts" (first cut)

| Item | Gate | Notes |
|---|---|---|
| BigQuery + Snowflake + S3 (Parquet) exporters over the ADR-016 v1 schema contract | Phase 3.4 contract tests | The engine already writes Parquet/JSONL (`export/warehouse.rs`); this cut adds the load manifests, destination clients, and CI contract tests against local emulators (BigQuery emulator / local Parquet assertions). No new schema changes. |
| ADR-016 open questions closed (compression zstd kept; partition guidance; `sequence` reserved) | Done 2026-09-16 — docs/capacity/2026-09-16-adr016-open-questions/ | Carried as completed prerequisite. |
| Exporter failure metrics wired into the 5.3.0 connector-failure metrics surface | Phase 5.1 | Exports are connectors; same retry + failure-metric contract. |

**Exit criteria:** round-trip contract test (crawl → export → load → assert)
green in CI for all three destinations; no schema drift against
`schemas/export/v1` (drift gate extended to warehouse manifests).

**Progress 2026-09-16:** ADR-016 open questions closed by measurement;
destination layout plans landed (`crawlkit export --layout
s3|bigquery|snowflake` — deterministic NDJSON plans rendered from the
manifest bindings, pinned by unit tests); ADR-017 metering spec drafted
for the alpha.3 stream. **Progress 2026-09-17:** destination clients
landed and CLI-wired (`crawl export --upload s3://|bigquery://|
snowflake://` — SigV4 S3 single PUT + multipart, BQ multipart/related
load, Snowflake statement COPY; plan-driven, injectable transport seam);
S3 round-trip contract test green locally against MinIO and wired into
CI (MinIO service in the service-backed job); exporter failure metrics
landed (`crawlkit_connector_deliveries_total{channel_type,outcome}` on
the 5.3.0 surface). **Remaining for this cut:** BigQuery emulator
contract test (**done 2026-09-19** — `bigquery_roundtrip_load_and_query`
loads all three tables with manifest-derived fields schemas and verifies
row counts + a value round-trip through `jobs.query` against
goccy/bigquery-emulator; wired into the CI service-backed job), Snowflake
stage decision (**decided 2026-09-19: emulator-based for alpha.1** — a
loopback fake of the SQL Statements API v2 pins the wire shape, handle
extraction, and error classification over real HTTP
(`snowflake_copy_over_real_http_shape_and_handle`); a real-account smoke
is an operator-owned pre-stable item, not a CI dependency), and the
no-schema-drift extension of the manifest gate (**done 2026-09-19** —
`validate_schema_contract` runs over all three v1 manifests: closed
type vocabularies per destination, complete bindings, no orphan keys;
pinned by mutation tests). **Remaining for this cut:** cut the tag once
the full CI battery is green on the tagged commit — the exit criteria
(round-trip contract in CI for all three destinations, no schema drift)
are now met. **The tag is NOT cut until the exit criteria are green on
the tagged commit** — per the cadence rule above.

### 6.0.0-alpha.2 — "Render Budgets"

| Item | Gate | Notes |
|---|---|---|
| Per-crawl render quota + per-page render budget | Phase 4.1 benchmark class 5 | Playwright path exists; budgets are enforcement on top. |
| JS-error findings class | Phase 2 analyzer gates | New finding codes require the findings-schema + client-parity ripple. |
| Rendering telemetry (spawn rate, budget exhaustion events) | Phase 5.1 metrics | Budget exhaustion degrades explicitly (like the scanner's daily budget), never silently. |

**Exit criteria:** a crawl with a 1-page render budget produces an explicit
degradation finding, not a hang; benchmark class 5 recorded.

### 6.0.0-alpha.3 — "Metering"

| Item | Gate | Notes |
|---|---|---|
| Usage metering (per-tenant crawl/export/render accounting) | ADR required first | New ADR: units of measure, recording point, storage, retention. Replaces aspirational docs/BILLING.md. |
| Quota surface (API + dashboard read path) | ADR for API changes | Quota exhaustion mirrors the scanner posture: explicit refusals with machine-readable cause. |

**Exit criteria:** metering ADR accepted before merge; API semver check
green; quota exhaustion pinned by tests.

### 6.0.0 stable — "Scale"

| Item | Gate | Notes |
|---|---|---|
| Distributed mode stable + published capacity evidence (10k and 100k with memory/fd/task bounds) | Phase 2.4/4.3 acceptance | Queue semantics + frontier wiring already graduated in 5.4.0; the paired-overhead and 100k evidence exist. **Open blocker: pinned 8-core/16 GB reference machine** (the plan's published class cannot come from CI). |
| Phase 6.2/6.3 release gate in full | checksums, SBOM, migration/rollback evidence | Practiced four times by the alpha cuts. |

## Cadence and rules

- **One stream per alpha; no alphas stack unreviewed work.** A cut ships
  only with its exit criteria green on the tagged commit.
- **Semver discipline:** alphas may add API surface; nothing renames or
  removes. The CI semver gate (now with its corrected 30-min budget)
  enforces against the last stable tag.
- **Every alpha updates:** CHANGELOG (Unreleased → alpha entry), release
  notes doc (`RELEASE_6_0_0_ALPHA_N.md`), and the capabilities manifest if
  any capability's status moved.
- **Stable cut conditions:** all four streams complete + the capacity
  report committed with raw artifacts + no experimental capability
  described as production-supported.

## Immediate next actions

1. Source the pinned 8-core/16 GB reference machine (long pole; start now).
2. Draft the metering ADR (needs product input on units + quota defaults).
3. alpha.1 exporter work can start immediately — the schema contract and
   its open questions are closed as of 2026-09-16.
