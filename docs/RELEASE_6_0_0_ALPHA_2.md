# crawlkit 6.0.0-alpha.2 — "Render Budgets" (consolidated cut)

**Released:** 2026-09-19 · **Tag:** `v6.0.0-alpha.2` (pre-release) ·
**Full change log:** [CHANGELOG.md](../CHANGELOG.md)

The second rolling prerelease of the 6.0.0 "Scale" phase — and one tag
carrying two streams. The alpha.1 exit criteria went green in CI on `main`
(BigQuery emulator round-trip, MinIO round-trip, the ADR-016 schema-contract
drift gate) on the same commit that completed the alpha.2 code, so the
consolidated cut ships both rather than stacking ceremony. The rationale is
recorded in docs/RELEASE_6_0_0_ALPHA_PLAN.md; both exit-criteria sets are
green on the tagged commit per the cadence rule.

> Pre-release: interfaces described here may still shift before 6.0.0
> stable. v5.4.0 "Queue Graduation" remains the latest stable release.

## alpha.1 carries over: warehouse contracts close

The three-destination ladder is contract-tested end to end in CI:

- **BigQuery loads real schemas now.** Load bodies carry
  manifest-derived per-table `fields` schemas (replacing the `_schema`
  placeholder for BigQuery; the sidecar remains for Snowflake/auditing).
  The round-trip test loads all three ADR-016 v1 tables into
  goccy/bigquery-emulator and verifies row counts and a value round-trip
  through `jobs.query` — on every push.
- **Snowflake is pinned at the wire.** A loopback fake of the SQL
  Statements API v2 exercises the real client over real HTTP: statement
  shape, handle extraction, error classification. A real-account smoke is
  an operator-owned pre-stable item, honestly out of CI scope.
- **No schema drift, by construction.** The manifest gate validates all
  three v1 contracts — closed type vocabularies per destination, complete
  bindings, no orphan keys — and mutation tests prove it fails when it
  should.

## Headline: render budgets — cost is a lever, degradation is explicit

Playwright rendering is the engine's most expensive operation, and until
now it had no ceiling. Two independent limits now bracket it, and both
degrade the same way the scanner's daily budget and every other budgeted
surface in crawlkit does: **explicitly, never silently.**

- **Per-crawl render quota** (`RenderBudget::max_renders_per_crawl`, CLI
  `--render-max`): once spent, remaining pages are analyzed statically and
  each emits a `RENDER001` finding naming the lever to raise.
- **Per-page render budget** (`RenderBudget::per_page_timeout`, CLI
  `--render-timeout`): a page that exceeds its slice times out, emits
  `RENDER002`, and is analyzed statically — a slow page is a finding, not
  a hang.
- **Rendering telemetry**: spawn rate, quota-exhaustion, budget-timeout,
  and denied-render counters ride the engine's existing metrics surface.

The exit criterion is pinned by a test: a crawl with a 1-page render
budget produces 1 render, 2 explicit degradations, and completes — not a
hang.

## New findings class: JS errors

The render script now captures uncaught `pageerror` events, and the new
full-gated `JsErrorAnalyzer` turns them into findings:

- **`JSERR001`** — an uncaught exception during render; hydration is often
  aborted, leaving content missing or interactive elements dead even when
  the HTML looks complete.
- **`JSERR002`** — console errors captured during the render window.

The analyzer registry grows 778 → 779; the findings JSON schema and the
Python/Node/Go clients admit the new codes unchanged (open vocabularies
by design).

## Numbers, honestly stated

- Analyzer registry: **779** analyzers (drift-gated)
- Engine lib suite: **4 121** tests green on the tagged commit
- CI: 12/12 jobs green, including the service-backed warehouse contracts
  (PostgreSQL + Redis + MinIO + BigQuery emulator)

## Try it

```
crawlkit crawl https://example.com --javascript \
  --render-max 50 --render-timeout 5000
```

## What's next

The remaining 6.0.0 stream is **alpha.3 "Metering"** (ADR-017: per-tenant
usage accounting + quota surface) and the stable-gate pair: the pinned
8-core/16 GB reference machine and the 10k/100k capacity evidence package
(`docs/REFERENCE_MACHINE_SPEC.md` is open on the sourcing decision).
