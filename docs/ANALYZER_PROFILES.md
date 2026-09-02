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
forms, tables, and links. Nine redundant generation registrations have
been removed from the full registry (eight exact duplicates or strict
subsets plus the reverse-subset `TableAccessibilityDeepDeepValidator`);
all removed types remain exported for API compatibility with behavior
pinned by `tests/test_generation_dedup.rs`. Remaining consolidation is
gated on fixture evidence per candidate, following the policy above.

## Live measurement (kingstonpeptides.com, 10 pages, 500 ms delay)

| Profile | Findings | Distinct codes | Severity spread | Categories hit |
|---|---:|---:|---|---|
| Full (778 analyzers) | 954 | 100 | 40 critical, 20 error, 309 warning, 585 info | 11 |
| Standard (17 analyzers) | **80** | **8** | 10 warning, 70 info | 6 |
| Core (9 analyzers) | **30** | **3** | 30 info | 3 |

The full profile is unchanged from the pre-consolidation baseline on
this site: the removed duplicates only emitted on pages carrying the
matching defects (insecure cookies, positive tabindex, missing H1s,
missing alt, generic anchors), which this site does not have. The
consolidation is therefore a correctness and maintenance win rather
than an output-noise win here; the noise lever is profile selection —
standard already reduces findings 12× with the canonical checks intact.
