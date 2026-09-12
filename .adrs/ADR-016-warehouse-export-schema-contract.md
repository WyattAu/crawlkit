# ADR-016: Warehouse Exporters — Versioned Schema Contract

**Status:** Proposed (6.0.0 "Scale" design; implementation follows acceptance)
**Date:** 2026-09-12
**Deciders:** Maintainers
**Related:** docs/PRODUCT_STRATEGY.md §3 (6.0.0 row: warehouse exporters, AR2);
ADR-013 §2 (per-tenant credential store — the tenancy precedent); ADR-015
(infrastructure-graduation pattern this ADR imitates: evidence first, stable
status later); `docs/capabilities.toml` claims policy (no public claim
without `stable` / `stable-with-configuration`).

---

## 1. Context

6.0.0 adds warehouse exporters — BigQuery, Snowflake, and S3 (Parquet) —
on top of the existing SQLite/PostgreSQL storage paths. The hard part is
not the transports; it is that a warehouse becomes a **public contract**:
once customers load crawl data into BigQuery, the column names, types,
and semantics are load-bearing for their SQL, dbt models, and dashboards.
Breaking them is a semver-major event for *their* pipelines, discovered
in *their* production.

The existing in-process report paths (`crawlkit report json|csv`) are
not a contract in this sense — they are versioned with the binary. The
warehouse path is different: data written today must remain queryable
and correct against binaries from years later. That demands an explicit,
versioned schema contract before any transport code is written.

## 2. Decision

### 2.1 One logical schema, three physical bindings

A single versioned logical schema (the "crawl export schema") binds to
each warehouse's native types. The logical schema — not the BigQuery or
Iceberg DDL — is the source of truth, lives in-repo as both a
human-readable table and a machine-checkable definition (TOML tables
under `schemas/export/v1/`), and is conformance-tested like the findings
JSON schema (`docs/schema/findings.schema.json` precedent: tests run
over real emit paths, not fixtures).

### 2.2 Schema v1 scope: three tables, no more

| Table | Grain | Source (today) |
|---|---|---|
| `crawl_runs` | one row per crawl | `CrawlMeta` + `CrawlStats` |
| `pages` | one row per fetched page | `PageData` (id, url, final_url, status_code, title, description, canonical_url, word_count, load_time_ms, body_size, fetched_at, tenant_id, ETag/Last-Modified, CWV columns) |
| `findings` | one row per issue | `Issue` (id, page_id, category, severity, code, title, …) + `crawl_id` denormalized for warehouse-style joins |

Deliberately excluded from v1: page links (row explosion; the graph is
not a warehouse concern before 6.x), plugin findings beyond the `Issue`
shape, CrUX time series (join via `pages.crawl_id` → `crawl_runs`).

Rules:

1. **Additive evolution inside a version.** v1.x may add nullable
   columns or new tables; never widen, rename, repurpose, or tighten a
   type. Removals and renames require v2 and a documented migration.
2. **Every column carries a deprecation discipline:** `added_in`,
   `deprecated_in` (optional), and a stability note. The conformance
   test fails CI when a struct field feeding the schema changes without
   a matching manifest diff — the same drift-gate philosophy as the
   capabilities manifest (Phase 0.1).
3. **Types bind at the edges, once.** Each binding (BigQuery, Snowflake,
   Parquet) has a table mapping logical → physical types in the schema
   manifest, reviewed in this ADR, so per-transport improvisation is
   impossible. Bindings are tested round-trip: write a fixture crawl via
   the real writer, read it back with the warehouse's own reader
   (testcontainers where available, recorded fixtures otherwise).
4. **Identity and idempotency.** `(crawl_id, page_id)` /
   `(crawl_id, page_id, issue_id)` are the logical keys; writers must be
   idempotent per key (re-running an export overwrites, never
   duplicates). MERGE semantics on warehouses that have them; key-sorted
   Parquet with deterministic row groups elsewhere (the deterministic
   artifact philosophy from the release pipeline extends to data
   files: same crawl → byte-identical Parquet).
5. **Tenancy rides on existing rails.** `tenant_id` is a first-class
   column (ADR-013 §2 precedent) and the *only* multi-tenant isolation
   the export provides; warehouse-side ACLs are the customer's layer,
   documented, not enforced by us.
6. **Credentials never enter the data path.** Warehouse credentials come
   from the deployment environment (env/secret store), like the GA4
   OAuth app config; nothing tenant-credential-shaped is added to the
   store's scope.

### 2.3 Versioning and compatibility promise

- Schema version is embedded in every artifact: a `_schema` table (BQ/
  SF) and a per-file footer key (Parquet), so any reader can reject
  data it cannot interpret.
- Compatibility is **backward only**: a v1 reader reads v1.x data. A
  writer never emits a mixed-version dataset.
- The contract's canonical home is `docs/schema/export/v1/README.md`
  (human) + `schemas/export/v1/*.toml` (machine), rendered into release
  notes at each schema change, with a "what changed" section per
  version — the changelog-for-schemas pattern.

### 2.4 Status discipline

`warehouse_exporters` enters `capabilities.toml` as `experimental` on
first landing (one binding behind a feature flag), moves to
`stable-with-configuration` only when: all three bindings round-trip in
CI, the conformance gate runs on every push, and one real warehouse
deployment has run a documented migration drill (export → load → query
→ verify) with the artifacts committed. This mirrors ADR-015's
evidence-before-stable pattern.

## 3. Consequences

**Positive:** customers' warehouse investments survive crawlkit
upgrades; per-transport type improvisation becomes impossible; the
drift gate extends to data contracts; Parquet determinism composes with
the release pipeline's reproducibility guarantees.

**Costs / risks:**

- Schema rigidity is real: an early mistake in v1 keys or types is
  expensive to fix. Mitigation: the three-table scope is deliberately
  minimal, nullable-heavy where uncertain (CWV columns were nullable in
  storage first for the same reason).
- Parquet byte-determinism constrains writer implementation (no
  parallel row-group writing without sorting discipline); accepted as a
  performance trade for verifiability, revisit only in a v2.
- Warehouse egress/cost is the customer's; crawlkit's obligation is
  bounded exports (crawl-scoped, not account-scoped), stated in the
  contract.

## 4. Alternatives considered

- **Per-transport schemas** (BQ-first design): fastest to ship, but the
  contract fragments; rejected — one logical schema is the point.
- **Defer schema versioning until v2** ("we'll rename later"): the
  findings JSON schema already proved in-repo conformance testing is
  cheap; deferring creates exactly the silent-break risk this ADR
  exists to prevent.
- **Export the storage rows verbatim** (no logical layer): couples the
  warehouse to internal storage structs (`PageData` etc. are not
  stability-promised), which would make every internal refactor a
  customer-facing event; rejected.

## 5. Open questions (resolve before implementation, not after)

1. Parquet compression/encoding defaults (zstd level; dictionary
   encoding thresholds) — pick by measured size/speed on a 100k-page
   fixture, committed with the capacity-evidence work (PRODUCT_STRATEGY
   6.0.0 row 1 shares the fixture).
2. Partitioning guidance for BQ/SF (`fetched_at` day partitions vs.
   `crawl_id` clustering) — document a recommendation, do not enforce.
3. Incremental exports (crawl deltas) — out of scope for v1; the
   idempotency rule makes a later addition non-breaking if `crawl_runs`
   carries a monotonic sequence from day one (accepted into v1: yes, as
   a nullable column).
