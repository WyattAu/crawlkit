# ADR-010: First-Party Plugin Index Seeded in the Repository

## Status

Accepted (2026-08-23)

## Context

The marketplace mechanics shipped in v4.1.0 (ADR-006 trust chain +
`plugin install/list/remove` over a git/file-based index) have no content:
an empty distribution channel has the same cold-start problem as no
channel at all. The original PLUGIN_MARKETPLACE.md envisioned a hosted
registry (ratings, CDN, gateway) — infrastructure that a small project
cannot operate responsibly before demand exists.

Additionally, all first-party artifacts are currently signable only by
the development key whose seed lives in the test fixtures
(`TRUSTED_SEED_HEX`) — acceptable for tests and seeding, not for a
long-lived production signing identity.

## Decision

1. **The first-party index lives in this repository** at
   `plugins/index/` — a versioned `plugin-index.toml` plus signed `.wasm`
   artifacts under `artifacts/`. Distribution = `git push`; consumers
   point `crawlkit plugin install --index` at the repo path or its
   raw GitHub URL. No server, no hosting cost, no new attack surface.
2. **Index updates are deliberate release events**: `scripts/build-plugin-index.sh`
   rebuilds and re-signs; the change is committed with intent. Entries
   pin exact artifact hashes — there is no mutable "latest" pointer.
3. **Seeded with two plugins** (`title-length`, `viewport-checker`) built
   from SDK examples, each functionally verified through the real host
   ABI in the CI conformance suite.
4. **Key rotation before third-party submissions**: the dev key signs the
   seed index; opening the index to external contributors requires (a) a
   release-environment-held signing key added to `TRUSTED_PLUGIN_KEYS`,
   (b) re-signing all first-party artifacts with it, and (c) removing the
   dev seed from test fixtures in favor of per-test generated keys where
   practical.

## Consequences

**Positive:**
- The marketplace is usable on day one; the docs point at a working URL.
- Index history is git history — auditable, revertable, blame-able.
- Artifact sizes (31-37 KB each) are trivially committable; no LFS.
- The hosted-registry evolution remains additive on top of the same
  index format.

**Negative / trade-offs:**
- Installing requires network access to GitHub (or a local checkout).
- Every artifact byte lands in git history; rotating a plugin binary
  permanently grows the repo (~35 KB per version — bounded and
  acceptable at this scale, revisited if the catalog grows large).
- Two-file atomicity (index + artifacts) is maintained by convention
  (the build script writes both); a torn state is caught by hash
  verification at install time, not prevented at commit time.
- The dev-key signing of published artifacts is explicitly transitional
  (point 4).

## Alternatives Considered

1. **Hosted registry now** — rejected: infrastructure and trust burden
   before any demand signal; the git index deferers nothing that blocks
   it later.
2. **Release-attached artifacts (GitHub Releases)** — rejected for the
   index itself: releases are per-tag snapshots; the index wants
   main-branch liveness, and raw-URL install from `main` is simpler.
3. **Unsigned index, signature-in-manifest only** — rejected: the index
   entry's trust fields are exactly what makes tampering with the
   catalog detectable; both are required together.

## References

- `plugins/index/` — the seeded index + README
- `scripts/build-plugin-index.sh` — rebuild procedure
- `crates/crawlkit-engine/tests/plugin_index_tests.rs` — conformance and
  functional coverage incl. `dump_first_party_index`
- ADR-006 (trust chain), ADR-009 (dogfood protocol that motivated
  shipping with real content)
