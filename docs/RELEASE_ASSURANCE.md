# Release assurance

This is the maintainer checklist for producing a trustworthy release. It is
an evidence gate, not a certification.

## Phase D verification (2026-09-02)

| Gate | Result | Status |
|---|---:|---|
| Workspace build | All crates build | Pass |
| Workspace tests | 4,300 passed, 0 failed, 5 ignored | Pass |
| Documentation | `cargo doc --workspace --no-deps`, 0 warnings | Pass |
| Engine library region coverage | 86.64% | Exception — below the 90% target |
| API all-target region coverage | 73.94% (main.rs excluded; see `docs/COVERAGE_CONTRACT.md`) | Exception — below the 90% target |
| API raw region coverage (bootstrap included) | 68.67% | Informational |
| Stripped core CLI (`--no-default-features`) | 2,575,640 bytes (2.46 MiB) | Pass — below the 10 MB target |
| Stripped full CLI (`--features full`) | 25,579,176 bytes (24.39 MiB) | Exception — full runtime remains above the 10 MB target |
| CLI feature matrix | Core and full all-target checks; strict clippy | Pass |
| CLI smoke surface | Full command surface exposes help | Pass |
| Bounded release gate | `verify-release-controls.sh` run 2026-09-05, every stage exit 0; log in `docs/release-evidence/release-gate-2026-09-05.txt` | Pass |
| Dependency gate | `cargo deny check` (advisories, bans, licenses, sources) exit 0 after fixing yanked `chacha20` and `event-listener` RUSTSEC-2026-0221; `cargo audit` 0 vulnerabilities, 2 documented warnings | Pass |
| Service-backed suite | Run 2026-09-05 against ephemeral PostgreSQL 16 + Redis 7 (Docker): all 20 engine + 1 API ignored service tests pass | Pass — 3 `pg_storage` decode/aggregation bugs found and fixed |
| Dogfood crawl (ADR-009) | kingstonpeptides.com 2026-09-05: 100 pages, 0 failures | Pass — found and fixed a 400-finding robots.txt "blocks all" false-positive cluster (RFC 9309 group scoping) |

These exceptions are explicit. Coverage is limited primarily by large
storage, sitemap, RUM, type, and WASM feature surfaces. API all-target
coverage is measured separately with router integration tests included and
currently stands at 73.94% on the covered surface (68.67% raw including
the excluded binary bootstrap); the scope, exclusions, and path to the 90%
target are defined in `docs/COVERAGE_CONTRACT.md`. The binary
already uses LTO, one codegen unit, `opt-level=3`,
stripping, and aborting panics. The feature migration now provides a measured
core artifact at 2,575,640 bytes stripped, while the full artifact remains
25,579,176 bytes because it includes the Wasmtime/plugin and integration
runtime. The core/full distinction must remain explicit in packaging and
release claims; the full artifact must not be represented as sub-10 MB.

## Release artifact evidence (2026-09-06; tag-state build)

Artifact checksums are point-in-time records: regenerate both binaries and
re-record the table below at the tag commit (the `release.yml` workflow
computes and signs its own `checksums.txt` from the tagged sources).

Workspace version **5.0.0**; toolchain **rustc 1.97.1 (8bab26f4f 2026-07-14)**;
release profile (LTO, one codegen unit, `opt-level=3`, stripped).

| Artifact | Feature flags | Stripped bytes | SHA-256 |
|---|---:|---:|---|
| `crawlkit` (core) | `--no-default-features` | 2,575,648 | `f4b016c1ffcdadb72728941ee3ebb8bcb94142286d1b9479d05bc403acd122dd` |
| `crawlkit` (full) | `--features full` | 25,621,264 | `8466be3da9717eaefa28fe516ba17328dbffb0b3ed10db922fca8a7289e437ae` |

Build commands: `cargo build --release -p crawlkit --no-default-features` and
`cargo build --release -p crawlkit --features full`, then `strip`.

Both binaries rebuilt 2026-09-06 at the final **v5.0.0** tag-state tree
(`1c3b944a`): the merged reconciliation (`6b1f835c`, origin/main's 9 owner
commits: `--allow-private`, envstack config, auth salting policy,
loop-retry webhook client) plus the wasmtime 47.0.4 / h2 0.4.19 advisory
patches (`1c3b944a`). Full: `1a683b77…` (25,631,312 B, merged tree) →
`8466be3d…` (25,621,264 B, patched lock); core is byte-identical across
the advisory patches (`f4b016c1…`, 2,575,648 B — wasmtime/h2 are
full-only deps). The workspace version is 5.0.0 throughout; the 5.0.0
relabel corrected the original mislabeling: v4.4.1 was already tagged at
`fe9258d7` (2026-08-23), and the 156 unreleased commits since then — the
robots.txt RFC 9309 fix, the PostgreSQL storage fixes, the breaking
finding-code changes — ship as 5.0.0 alongside the owner's main work.

## v5.0.0 shipped (2026-09-06)

GitHub Release **v5.0.0** published from tag `554fe85e` (run 34055829118):
all preflight/audit/build jobs green; `Create Release` succeeded with the
download-artifact v8 checksum fix. Assets verified: the linux x86_64
archive hash matches the GPG-signed `checksums.txt`
(`71b6268c…`); SBOMs (`crawlkit.cdx.json` + per-crate) included. CI on the
final merged main (`554fe85e`) is green. The version-tag guard now
requires a workspace version bump before the next release.

## Before release

1. Review `ROADMAP.md`, `docs/capabilities.toml`, and public claims.
2. Confirm the version and MSRV agree across Cargo metadata, CI, and docs.
3. Run the bounded release gate:

   ```bash
   CARGO_BUILD_JOBS=1 CRAWLKIT_TEST_THREADS=1 bash scripts/verify-release-controls.sh
   ```

4. Run service-backed tests when PostgreSQL/Redis support is part of the
   release scope. Executed 2026-09-05 against ephemeral PostgreSQL 16 +
   Redis 7 containers; the full ignored suite (20 engine + 1 API) passes.
   The run surfaced and fixed three `pg_storage` bugs (TIMESTAMPTZ->text
   and INTEGER->i64 decode mismatches in `get_crawl_meta`, and an
   impossible `test_pg_top_issues` aggregation assertion) — see CHANGELOG.
5. Capture benchmark evidence when performance claims change:

   ```bash
   bash scripts/capture-benchmark-metadata.sh
   ```

   The 2026-09-05 run is committed (raw output + summary in
   `docs/benchmarks/2026-09-05/` and `measured-2026-09-05.md`).

6. Coverage is measured by the CI workflow with native targets separated:
   engine library coverage uses `--lib`, and API coverage uses `--all-targets`
   so router integration tests are included without instrumenting the WASM
   target. Binary entrypoints are excluded per `docs/COVERAGE_CONTRACT.md`.

7. Build both release artifacts and record checksums, dependency/SBOM output,
   exact toolchain, feature flags, and stripped byte counts:

   ```bash
   cargo build --release -p crawlkit --no-default-features
   strip target/release/crawlkit
   stat -c '%s' target/release/crawlkit
   cargo build --release -p crawlkit --features full
   strip target/release/crawlkit
   stat -c '%s' target/release/crawlkit
   ```
8. Run CLI help/API smoke tests against the release artifact. Executed
   2026-09-05 against the stripped full artifact: root and all 10
   subcommand help surfaces resolve, `--version` reports the workspace
   version; recorded 2026-09-05 at 4.4.1 in
   `docs/release-evidence/cli-smoke-2026-09-05.txt` and re-recorded
   2026-09-06 at 5.0.0 in `docs/release-evidence/cli-smoke-2026-09-06.txt`.
9. Review migration, rollback, security, and plugin compatibility notes.
   Reviewed 2026-09-05: `MIGRATION.md` (4.0/3.0 sections + semver policy),
   `SECURITY.md`, `SECURITY_BOUNDARIES.md`, and `PLUGIN_DEVELOPMENT.md`
   compatibility notes are present and carry no stale claims; v5.0.0 is a
   breaking release (finding-code renames/removals, engine construction,
   feature boundary), so a `[5.0]` migration section was added 2026-09-06.

## Evidence retention

The 2026-09-05 run commits its records under `docs/release-evidence/`:
`release-gate-2026-09-05.txt` (bounded gate, per-stage exit codes),
`cargo-deny-2026-09-05.txt`, `cargo-audit-2026-09-05.txt`,
`service-backed-2026-09-05.txt` (PostgreSQL 16 + Redis 7 suite), and
`cli-smoke-2026-09-05.txt` (release-artifact CLI surface).

The 2026-09-06 v5.0.0 records add `release-gate-2026-09-06.txt` (bounded
gate at `0042ae15`, every stage exit 0), `readiness-2026-09-06.txt`
(pre-tag gate at `a61379a5`, READY), and `cli-smoke-2026-09-06.txt`
(release-artifact CLI surface reporting 5.0.0).

Retain the following with the release or CI run:

- validation logs;
- benchmark metadata and raw output;
- artifact checksums;
- dependency/advisory results;
- test summary and ignored-test rationale;
- migration and smoke-test results;
- claim-review sign-off.

Do not publish a numeric performance or capability claim unless its raw
workload and environment evidence are retained and reproducible.

## Release blockers

Block a release for:

- unresolved critical/high security issues;
- failed format, lint, contract, or required integration tests;
- undocumented schema/API/ABI breaks;
- stale or unsupported public claims;
- missing artifact checksums or rollback guidance;
- unreviewed unsafe code or trust-boundary changes.

Native plugins are trusted process code and must never be represented as
sandboxed. WASM sandbox controls do not imply formal verification or defence,
HFT, FAANG, or compliance certification.
