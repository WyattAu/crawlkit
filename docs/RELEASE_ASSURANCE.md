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
| `crawlkit` (core) | `--no-default-features` | 2,575,648 | `b7d9858efcbb44652f0c2d9fc021b80e973ba9fc3f2495e1d94462384e53d443` |
| `crawlkit` (full) | `--features full` | 25,574,192 | `70ac19f44b69d52292e08bd9fb32fc21d7798ada46e64f0d38cae17cd03aa189` |

Build commands: `cargo build --release -p crawlkit --no-default-features` and
`cargo build --release -p crawlkit --features full`, then `strip`.

Both binaries rebuilt 2026-09-06 at the **v5.0.0** relabel state (workspace
version bumped from 4.4.1, which changes embedded package metadata): core
`d153272b…` (2,575,648 B, 4.4.1) → `b7d9858e…` (2,575,648 B), full
`da5688f1…` (25,574,184 B, 4.4.1) → `70ac19f4…` (25,574,192 B). The
5.0.0 relabel corrected the earlier mislabeling: v4.4.1 was already tagged
at `fe9258d7` (2026-08-23), and the 156 unreleased commits since then —
including the `5cb5e7b8` robots.txt RFC 9309 fix, the PostgreSQL storage
fixes, and the breaking finding-code changes — now ship as 5.0.0.
Earlier provenance: the `5cb5e7b8` robots fix changed 4.4.1-era records
(core `e52ccca4…` → `d153272b…`, full `2055dda5…` → `da5688f1…`), and the
dependency supply-chain fix changed the full artifact from `76f5f6b1…`
(25,579,176 B, `888d14dd`) to `2055dda5…` (25,576,616 B, `394b18d7`)
with core byte-for-byte identical.

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
