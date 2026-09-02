# Release assurance

This is the maintainer checklist for producing a trustworthy release. It is
an evidence gate, not a certification.

## Phase D verification (2026-09-02)

| Gate | Result | Status |
|---|---:|---|
| Workspace build | All crates build | Pass |
| Workspace tests | 4,263 passed, 0 failed, 5 ignored | Pass |
| Documentation | `cargo doc --workspace --no-deps`, 0 warnings | Pass |
| Engine line coverage | 79.07% (79.08% after focused tests) | Exception — below the 90% target |
| Stripped release binary | 25 MB | Exception — above the 10 MB target |
| CLI smoke surface | All documented subcommands expose help | Pass |

These exceptions are explicit. Coverage is limited primarily by large
storage, sitemap, RUM, type, and WASM feature surfaces; integration coverage
remains planned. The binary already uses LTO, one codegen unit, `opt-level=3`,
stripping, and aborting panics. Reaching 10 MB requires a separate packaging
decision: make the Wasmtime/plugin runtime opt-in, or ship a small core binary
with plugin support as a separate artifact. A no-default-features prototype
was attempted during remediation and exposed 97 compile-time feature
assumptions across engine modules; it was reverted without changing the
stable default path. This feature/API migration must not be made implicitly
at release time.

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

6. Build release artifacts and record checksums, dependency/SBOM output, and
   the exact toolchain.
7. Run CLI help/API smoke tests against the release artifact.
8. Review migration, rollback, security, and plugin compatibility notes.

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
