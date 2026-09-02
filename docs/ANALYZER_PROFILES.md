# Analyzer Registry Profiles

`crawlkit-engine` exposes four registry levels so callers can choose an explicit
trade-off between coverage, cost, and output noise.

| Profile | Constructor | Purpose | Current size |
|---|---|---|---:|
| Core | `AnalyzerRegistry::core(&config)` | Foundational HTTP, SEO, and structured-data checks | 9 |
| Standard | `AnalyzerRegistry::standard(&config)` | Canonical checks suitable for routine crawls | 17 |
| Deep | `AnalyzerRegistry::deep(&config)` | Focused deep security, accessibility, and SEO checks | 20 |
| Full | `AnalyzerRegistry::new(&config)` | Complete backward-compatible built-in registry | 785 |

Profiles are selectable at runtime with `crawlkit crawl --analyzer-profile
core|standard|deep|full` (default `full`).

## Compatibility

`new()` remains the full registry and is unchanged by profile consolidation.
`core`, `standard`, and `deep` are explicit opt-in profiles. They do not
implicitly combine with one another; use `with_analyzers` or `register` when a
custom combination is required.

The profile sizes are guarded by tests because they are part of the current
behavioral baseline. A future change should update the tests and explain the
compatibility impact in the changelog.

## Selection guidance

- Use **core** for fast baseline checks and low-noise reports.
- Use **standard** for normal SEO/security/accessibility crawls.
- Use **deep** when advanced generation-specific checks are required.
- Use **full** when maximum coverage or compatibility with existing output is
  required.

Profiles are not standards-compliance levels. A profile only controls which
analyzers run; individual findings retain their own severity and standards
scope.

## Consolidation policy

Analyzer generations are removed from a profile only after:

1. a deterministic behavior matrix compares the candidate implementations;
2. finding-code ownership is unique in the target profile;
3. focused and full tests pass; and
4. the output and compatibility impact are documented.

The full registry is retained as the rollback and compatibility path while
this consolidation work proceeds.

## Consolidation progress (2026-09-02)

All known duplicate finding-code families are resolved and guarded by
dedicated behavior matrices (`tests/test_*_matrix.rs`): color-contrast,
focus, heading-hierarchy, image-alt, anchor-text, LINKTQ, CSP, cookies,
forms, tables, and links. Two strict-subset registrations have been
removed from the full registry (`HeadingHierarchyDeepDeepDeepValidator`,
`ImageAltTextDeepDeepDeepValidator`); both types remain exported for API
compatibility. Remaining consolidation is gated on fixture evidence per
candidate, following the policy above.
