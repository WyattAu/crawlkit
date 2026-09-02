# Analyzer Audit — Finding-Code Duplicates & Standards Claims

Date: 2026-09-01
Scope: `crates/crawlkit-engine/src/analyzers/`

## Executive summary

The registry now enforces unique finding codes for findings emitted by the
representative default fixture. Phase 4 has resolved the principal runtime
collisions through explicit namespaces, while preserving analyzer behavior.
Static duplicate scans still report repeated literals that are either multiple
branches within one analyzer, test expectations, or generation-specific
implementations not exercised by the minimal fixture. Those are not treated as
resolved until a registry fixture demonstrates the collision or ownership is
explicitly documented.

## Phase 4 remediation completed

| Original collision | Resolution |
|---|---|
| `XCTO-V2001` | Deep implementation receives `XCTO-V2001-DEEP` |
| `PERMP-V2001` | Deep-deep implementation receives `PERMP-V2001-DEEP-DEEP` |
| `COISO-V2001` | Deep-deep implementation receives `COISO-V2001-DEEP-DEEP` |
| `LANGATTR-V2001` | Deep-deep implementation receives `LANGATTR-V2001-DEEP-DEEP` |
| `HHIER-V2001` | Deep-deep-deep implementation receives `HHIER-V2001-DEEP-DEEP-DEEP` |
| Form/table/link-quality generation collisions | Generation-specific `DEEP`, `DEEP-DEEP`, and `DEEP-DEEP-DEEP` namespaces |
| Legacy CORS vs policy analyzer | `CORS001/002-MISCONFIG` |
| Legacy cookie secure validator | `COOKIESEC001-VALIDATOR` |
| Accessibility vs SEO link collision | `A11Y-LINK-V2001` |
| Accessibility focus analyzers | `A11Y-FOCUS001/002` |
| Policy validator collisions | `COEP001-POLICY`, `COOP002-POLICY`, `CSPDIR001-VALIDATOR` |
| SEO metadata and anchor diversity | `META-V3001-LENGTH`, `ANCH-DIV001-INTERNAL` |

The namespace suffix identifies ownership or analyzer generation. It is not a
replacement for behavioral consolidation; redundant analyzers remain a Phase
4 follow-up.

## Current duplicate-code guard

`test_registry_finding_codes_unique_on_fixture` executes every analyzer in the
default registry against a deterministic parsed-page fixture and fails if two
registered analyzers emit the same code. This guard is intentionally runtime
based: a repeated string literal in source does not necessarily mean two
registered analyzers can emit it for the same input.

Validation at the latest Phase 4 batch:

- `cargo fmt --all -- --check` passed.
- `cargo check -p crawlkit-engine` passed.
- `cargo clippy -p crawlkit-engine --all-targets -- -D warnings` passed.
- Registry duplicate-code test passed.
- Full library suite previously passed with 3,824 tests.

## Remaining static candidates

**Status update (2026-09-02):** every family below has now been fixtured
with a dedicated behavior matrix and resolved. The previously listed
collisions are closed:

| Family | Resolution | Matrix |
|---|---|---|
| `COLRCL-V2001` | Deep link heuristic namespaced; V2 and deep have distinct triggers | `test_color_contrast_matrix.rs` |
| `FOCUS-V2001` | Deep-deep/deep-deep-deep generations namespaced; V2 canonical | `test_focus_matrix.rs` |
| `HHIER-V2002/3` | Semantic collision fixed (V2: empty headings; V8: missing/multiple H1); V8 generations namespaced; deep-deep-deep registration removed as strict subset | `test_hhier_matrix.rs` |
| `IMGALT-V2001` | Deep-deep namespaced (distinct empty-alt semantics); deep-deep-deep registration removed as strict subset | `test_imgalt_matrix.rs` |
| `ANCHGEN-V2001` | Deep generation namespaced; trigger lists overlap but neither contains the other, so both retained | `test_anchorgen_matrix.rs` |
| `LINKTQ-V2002` | Already namespaced (`-V2`, `-DEEP`) in Phase 4 with matrix coverage | `test_behavior_matrix.rs` |

The registry now ships 785 analyzers with unique, ownership-documented
finding codes guarded by runtime fixtures. Aggregate generation analyzers
emit at most one finding per code; per-link analyzers legitimately emit
one finding per link, and the matrices assert the difference.

Earlier guidance (still applicable to any future duplicate scan hit):
many repeated literals are separate findings within one analyzer or
inline test fixtures. Each candidate requires a page fixture that
activates the relevant registered analyzers before a code change is
justified. The next correct action is never a global rename. Build
focused fixtures, compare finding semantics, and either:

1. retain one canonical analyzer and remove redundant registrations;
2. assign a stable generation/ownership namespace when both are intentional;
3. merge analyzers when they are the same check with different wording.

## Standards-claim audit

### Accurate or appropriately scoped

- HSTS behavior aligns with RFC 6797 concepts such as `max-age`,
  `includeSubDomains`, and preload readiness.
- `X-Content-Type-Options: nosniff` validation is correctly treated as a
  security-header check.
- Referrer-Policy and Permissions-Policy checks reflect current browser header
  semantics and are presented as recommendations where appropriate.
- Skip links, main landmarks, and form labels have defensible WCAG mappings.

### Heuristic checks

The following are best-practice or SEO heuristics, not automatic WCAG failures:

- Heading-level skipping and multiple H1 headings.
- Missing navigation or banner landmarks.
- Metadata length recommendations.
- Generic or repetitive anchor text.

These should remain advisory and should not claim that the corresponding
heuristic alone violates WCAG or OWASP.

## Next roadmap phase: behavioral consolidation

1. ~~Create targeted fixtures for the remaining static candidates.~~
   **Complete** — all known collision families are matrixed (see table
   above).
2. ~~Produce behavior matrices for form labels, tables, links, headings,
   cookies, CSP, and metadata.~~ **Complete** — plus color-contrast,
   focus, heading-hierarchy, image-alt, and anchor-text families.
3. **In progress** — select canonical implementations and remove
   redundant default registrations. Two strict-subset registrations are
   removed so far (`HeadingHierarchyDeepDeepDeepValidator`,
   `ImageAltTextDeepDeepDeepValidator`); continue candidate-by-candidate
   with fixture evidence only.
4. Keep compatibility exports for public analyzer types where required
   (unregistered types remain exported and tested).
5. **Pending** — re-measure default-registry size (currently 785),
   findings per page, latency, and memory against the kingstonpeptides.com
   baseline (954 findings / 10 pages before consolidation).
6. ~~Add registry profiles (`core`, `standard`, `deep`, and custom
   selection).~~ **Complete** — implemented and selectable via CLI; see
   `docs/ANALYZER_PROFILES.md`.
