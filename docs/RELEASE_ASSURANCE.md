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
| API all-target region coverage | 68.67% (router integration included) | Exception — below the 90% target |
| Stripped core CLI (`--no-default-features`) | 2,575,640 bytes (2.46 MiB) | Pass — below the 10 MB target |
| Stripped full CLI (`--features full`) | 25,579,176 bytes (24.39 MiB) | Exception — full runtime remains above the 10 MB target |
| CLI feature matrix | Core and full all-target checks; strict clippy | Pass |
| CLI smoke surface | Full command surface exposes help | Pass |

These exceptions are explicit. Coverage is limited primarily by large
storage, sitemap, RUM, type, and WASM feature surfaces. API all-target
coverage is measured separately with router integration tests included and
currently stands at 68.67%; the overall 90% target remains open. The binary
already uses LTO, one codegen unit, `opt-level=3`,
stripping, and aborting panics. The feature migration now provides a measured
core artifact at 2,575,640 bytes stripped, while the full artifact remains
25,579,176 bytes because it includes the Wasmtime/plugin and integration
runtime. The core/full distinction must remain explicit in packaging and
release claims; the full artifact must not be represented as sub-10 MB.

## Before release

1. Review `ROADMAP.md`, `docs/capabilities.toml`, and public claims.
2. Confirm the version and MSRV agree across Cargo metadata, CI, and docs.
3. Run the bounded release gate:

   ```bash
   CARGO_BUILD_JOBS=1 CRAWLKIT_TEST_THREADS=1 bash scripts/verify-release-controls.sh
   ```

4. Run service-backed tests when PostgreSQL/Redis support is part of the
   release scope.
5. Capture benchmark evidence when performance claims change:

   ```bash
   bash scripts/capture-benchmark-metadata.sh
   ```

6. Coverage is measured by the CI workflow with native targets separated:
   engine library coverage uses `--lib`, and API coverage uses `--all-targets`
   so router integration tests are included without instrumenting the WASM
   target.

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
8. Run CLI help/API smoke tests against the release artifact.
9. Review migration, rollback, security, and plugin compatibility notes.

## Evidence retention

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
