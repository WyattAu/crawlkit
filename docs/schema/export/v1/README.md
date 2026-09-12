# Crawl Export Schema v1 (ADR-016) — human-readable contract

**Status:** Canonical contract for `warehouse_exporters` schema v1
**Date:** 2026-09-13
**Authority:** This document renders `schemas/export/v1/*.toml`; the TOML
files are the machine-checkable source of truth. The conformance tests
(`crates/crawlkit-engine/src/export/warehouse.rs` tests + the manifest
drift gate) fail CI when code and contract diverge.

---

## What this contract is

Once crawl data is loaded into a customer's warehouse (BigQuery, Snowflake,
or S3 Parquet), these column names, types, and semantics become load-bearing
for their SQL, dbt models, and dashboards. Breaking them is a semver-major
event **for the customer's pipelines**. This contract exists so that never
happens silently.

## The three tables

| Table | Grain | Logical keys | Source |
|---|---|---|---|
| `crawl_runs` | one row per crawl | `(crawl_id)` | `CrawlMeta` + tenancy + export sequence |
| `pages` | one row per fetched page | `(crawl_id, page_id)` | `PageData` |
| `findings` | one row per issue | `(crawl_id, page_id, issue_id)` | `Issue` + denormalized `crawl_id` |

Deliberately excluded from v1 (ADR-016 §2.2): page links (row explosion;
the graph is not a warehouse concern before 6.x), plugin findings beyond the
`Issue` shape, CrUX time series (join via `pages.crawl_id` → `crawl_runs`).

## Evolution rules (the promise)

1. **Additive only, inside v1.x.** New *nullable* columns or new tables are
   permitted. Never a rename, repurpose, type tightening, narrowing of
   nullability, or removal — those require v2 and a documented migration.
2. **Every column carries `added_in`** (and `deprecated_in` once anything is
   deprecated) in the TOML manifest. The conformance test fails CI when a
   struct field feeding the schema changes without a matching manifest diff —
   the same drift-gate philosophy as the capabilities manifest.
3. **Version embedded in every artifact.** A `_schema` table (BQ/SF) and a
   per-file footer key (Parquet) let any reader reject data it cannot
   interpret. Writers never emit mixed-version datasets.
4. **Backward compatibility only:** a v1 reader reads v1.x data.

## Types

Logical types are small and closed: `string`, `int64`, `float64`, `bool`,
`timestamp`. The per-binding physical mapping is declared per column in the
TOML (`bigquery`, `snowflake`, `parquet`) and is the only permitted mapping —
per-transport improvisation is a conformance failure.

Notable mappings:

- timestamps: BQ `TIMESTAMP` / SF `TIMESTAMP_NTZ` / Parquet
  `TIMESTAMP_MILLIS`, always UTC.
- `status_code` is `int64` (Parquet `INT32`) — HTTP codes fit, and the
  logical type stays stable if the storage type ever changes.
- `category` in `findings` is a **string, not an enum**: the plugin surface
  (`custom:<name>`) must never break the contract.

## Identity, idempotency, tenancy

- **Idempotency:** writers are idempotent per logical key — re-running an
  export overwrites, never duplicates. MERGE on warehouses that have it;
  key-sorted deterministic output elsewhere (same crawl → byte-identical
  Parquet).
- **Tenancy:** `tenant_id` is a first-class nullable column on all three
  tables and is the *only* isolation the export provides; warehouse-side
  ACLs are the customer's layer.
- **Credentials** come from the deployment environment (env/secret store);
  nothing tenant-credential-shaped enters the data path.
- **Bounded exports:** exports are crawl-scoped, never account-scoped.

## Binding implementation status

| Binding | Status | Notes |
|---|---|---|
| S3 (Parquet) | implemented (`warehouse` feature) | deterministic key-sorted rows, zstd; footer carries schema version |
| BigQuery | contract defined, transport pending | MERGE on `(keys)` |
| Snowflake | contract defined, transport pending | MERGE on `(keys)` |

`warehouse_exporters` enters `capabilities.toml` as `experimental`; it moves
to `stable-with-configuration` only per ADR-016 §2.4 (all three bindings
round-trip in CI, conformance gate on every push, one documented migration
drill).

## Open items owned elsewhere (ADR-016 §5)

- Compression/encoding defaults to be finalized against the 100k-page
  capacity-evidence fixture (shared with PRODUCT_STRATEGY 6.0.0 row 1).
- Partitioning guidance (BQ/SF): `fetched_at` day partitions recommended,
  documented not enforced.
- Incremental exports: out of scope for v1; `crawl_runs.sequence` (nullable)
  is reserved from day one so the later addition is non-breaking.
