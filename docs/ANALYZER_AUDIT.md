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

A source scan still finds repeated literals in several large generation files,
including `ANCHGEN-V2001`, `COLRCL-V2001`, `FOCUS-V2001`, `HHIER-V2002/3`,
`IMGALT-V2001`, `LINKTQ-V2002`, and several CSP/robots/canonical families.
Many occurrences are separate findings within one analyzer or inline test
fixtures. Each candidate requires a page fixture that activates the relevant
registered analyzers before a code change is justified.

The next correct action is not a global rename. Build focused fixtures for the
remaining families, compare finding semantics, and either:

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

1. Create targeted fixtures for the remaining static candidates.
2. Produce behavior matrices for form labels, tables, links, headings, cookies,
   CSP, and metadata.
3. Select canonical implementations and remove redundant default registrations.
4. Keep compatibility exports for public analyzer types where required.
5. Re-measure default-registry size, findings per page, latency, and memory.
6. Add registry profiles (`core`, `standard`, `deep`, and custom selection) only
   after the measurements establish useful boundaries. Core, standard, and deep
   are now implemented; see `docs/ANALYZER_PROFILES.md`. Full-registry reduction
   remains gated by behavior and performance evidence.
