# crawlkit 6.0.0-alpha.4 — "Consolidation"

**Released:** 2026-09-29 · **Tag:** `v6.0.0-alpha.4` (pre-release) ·
**Full change log:** [CHANGELOG.md](../CHANGELOG.md)

The fourth rolling prerelease of the 6.0.0 "Scale" phase. Unlike the
first three cuts, this one adds no feature surface: it is a hardening
stream that makes the analyzer registry and its documentation tell the
truth. Four duplicate emitter registrations are gone (each removed only
behind fixture evidence), five co-fire-ambiguous finding codes are
namespaced, the static catalog scanner that enforces all of this is
literal-aware and self-tested, the capacity reports are drift-gated, and
the Wasmtime dependency is patched against three RustSec advisories.

> Pre-release: interfaces described here may still shift before 6.0.0
> stable. v5.4.0 "Queue Graduation" remains the latest stable release.

## Headline: the registry tells the truth

The default registry shipped four analyzers that duplicated finding
emission already owned by earlier generations — same trigger condition,
same code, sometimes a weaker severity. A page with insecure cookies
emitted `COOKIEHTTP001` twice; a robots.txt without a `Sitemap:`
directive emitted `SITEMAPDEEP-V2001` twice. Each removal in this cut is
backed by a fixture in
`crates/crawlkit-engine/src/analyzers/tests/test_generation_dedup.rs`
that pins the exact relationship (exact-duplicate, subset, or
complementary), and every removed type remains exported and
matrix-tested. Default registry: **779 → 775**; no-default-features:
**775 → 771**.

## Finding codes namespaced where analyzers legitimately co-fire

For pairs where both analyzers stay registered — different defects with
legitimate per-defect and aggregate findings on the same page — the
emitted codes are namespaced per the ANALYZER_AUDIT Phase-4 convention.
The bare code stays with the earlier/base generation:

| Old code | New code | Owner of the new code |
|---|---|---|
| `INTLINKQ-V2001` | `INTLINKQ-V2001-DEEP` | `InternalLinkQualityDeepValidator` (nofollow internal-link ratio) |
| `INTLINKQ-V2002` | `INTLINKQ-V2002-DEEP` | `InternalLinkQualityDeepValidator` (internal links without anchor text) |
| `FORMLAB-V2001` | `FORMLAB-V2001-DEEP` | `FormLabelAssociationDeepValidator` (inputs without label association) |
| `HSTSPR001` | `HSTSPR001-DEEP` | `HstsPreloadReadyDeepValidator` (aggregate preload readiness) |
| `HSTSPR-V2001` | `HSTSPR-V2001-DEEP-DEEP` | `HstsPreloadReadyDeepDeepValidator` (aggregate readiness, deep-deep) |
| `EXTLINKAUTH-V2001` | `EXTLINKAUTH-V2001-DEEP-DEEP` | `ExternalLinkAuthorityDeepDeepValidator` (low-authority external-link ratio) |

The `FORMLAB-V2001` deep validator also had a dead-check bug: its
duplicate-input-ID branch was unreachable because IDs were collected
into a `HashSet` first — it now counts occurrences and emits
`FORMLAB-V2001` (the bare code stays with
`FormLabelAssociationAnalyzerV2`).

One retained pair — `XFODEEP-V2001` against the XFO base analyzers — is
complementary, not duplicate (mutually exclusive precondition pairs), so
it stays registered and is pinned by fixture instead.

## The catalog scanner is literal-aware, with a self-test

`scripts/generate_finding_catalog.py` attributes each finding code to
its owning `impl Analyzer for` block. Attribution previously ran on
naive brace counting, so an unbalanced `{` inside a comment (a CSS rule
in `v2/accessibility.rs`) swallowed every following impl block and
mis-attributed ≈14 codes in `docs/FINDING_CODES.md`; char literals such
as `('\"',)` corrupted spans the same way. The scanner now runs on a
sanitized copy of each source — string literals (raw/byte included),
char literals, and line/block comments blanked with offsets preserved —
and a `--self-test` mode (five scoping cases: comment braces, char
quotes, raw strings, byte escapes, doc-comment phantoms) is wired into
`scripts/verify-release-controls.sh` **before** the `--check` gate.
Corrected catalog: **1 150 distinct codes across 737 `impl Analyzer
for` blocks, 6 recorded shared codes** — the shared-code register is now
fully dispositioned (4 exact duplicates unregistered, 2 complementary
pairs pinned by fixture).

## Capacity reports are drift-gated

`scripts/render_capacity_report.py` renders every committed record
layout (inline and distributed run-record sets, paired
inline/queue same-commit sets with median-to-median overhead, and the CI
smoke record), and its `--check` mode — wired into the release-controls
gate — fails when a `docs/capacity/*/REPORT.md` generated-numbers
section drifts from a fresh render. Hand-written narrative is preserved
outside the `<!-- GENERATED-NUMBERS -->` markers. All 7 record-bearing
directories re-rendered. One correction surfaced by the gate: the
2026-09-16 paired-overhead figure is ≈**12.5% median-to-median** (the
previously cited ≈14.5% was mean-to-mean; §5.6 publishes the median).

## Dependencies: Wasmtime 47 → 48 (RustSec), MSRV → 1.95.0

`cargo deny` flagged the Wasmtime 47.0.4 family with
**RUSTSEC-2026-0314/0315/0316** (`call_ref` / exception-`catch` fuel
amplification and related). No patched 47.x exists; the workspace moves
to the patched **48 series (48.0.3)**. Wasmtime remains behind
the engine's non-default `full`/`wasi-preview2` features — the default
and no-default builds never compile it.

Two follow-on consequences are handled in the same stream:

- **MSRV 1.94.0 → 1.95.0** — Wasmtime 48's cranelift requires Rust
  1.95.0; the bump is enforced consistently across `Cargo.toml`, the CI
  MSRV job, pre-commit, the justfile, and the docs.
- **WASI-HTTP hook port** — the 48 series reworked
  `WasiHttpHooks::send_request` (now
  `http::Request<WasiBody>` + `Option<RequestOptions>` → boxed future of
  the response plus the request-error future). `PluginHttpHooks` is
  ported with every host guarantee preserved: network capability gate,
  SSRF target validation, 10 s timeout clamp, and the 1 MiB response
  body cap, all pinned by updated in-module tests.

## Numbers, honestly stated

- Analyzer registry: **775** (full) / **771** (no-default-features) /
  **768** (AI-WASM-off) — down from 779/775/772; drift-gated
- Finding-code catalog: **1 150** distinct codes, **737** impl blocks,
  **6** recorded shared codes (previously mis-attributed: 1 152/738)
- Engine lib suite: **4 140** tests green on the tagged commit,
  including the four new dedup fixtures
- Engine no-default check: green (CI parity: `cargo check`, matching the
  feature-matrix job)
- API semver: **green** against the `v6.0.0-alpha.3` baseline — this cut
  claims no public API surface change (namespaced codes are emitted
  values, not API signatures)

## What's next

The remaining gate between here and 6.0.0 stable is the stable-gate
pair: the pinned 8-core/16 GB reference machine (operator-owned; see
docs/REFERENCE_MACHINE_SPEC.md) and the 10k/100k capacity evidence
package, plus the dead-code `unstable-legacy` gate executed at the
stable cut. Phase 6.2/6.3 release-gate practice continues at each alpha
cut (six consecutive after this one).
