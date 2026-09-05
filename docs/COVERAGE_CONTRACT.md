# Coverage contract

This document defines what "coverage" means in this repository: what is
measured, what is excluded and why, which numbers are blocking gates, and
what must happen before the 90% roadmap target can be claimed.

The contract exists so coverage claims are reproducible and honest. Every
figure referenced here is produced by the same commands the CI coverage job
runs (`cargo llvm-cov`, region coverage — the `Cover` column of the summary
`TOTAL` row). See `.github/workflows/ci.yml`.

## Scope

| Target | Command | What it measures |
|---|---|---|
| Engine library | `cargo llvm-cov -p crawlkit-engine --lib` | All `crawlkit-engine` library code under default features |
| API surface | `cargo llvm-cov -p crawlkit-api --all-targets` | All `crawlkit-api` library and test-target code (router integration tests included) |

Both are measured on supported native targets only. A workspace-wide
invocation is not used because it discovers the wasm32 test target and can
fail during instrumentation.

## Exclusions

1. **Crate binary entrypoints** (`src/main.rs`). These are process
   bootstrap — environment parsing, tokio runtime construction, and the
   call into the library — not product logic. They are excluded with
   `--ignore-filename-regex 'src/main\.rs'` in both the summary gate and the
   LCOV upload so the two never diverge.
2. **Feature-gated service adapters.** PostgreSQL, WASM, and other
   non-default engine features are not compiled by the `--lib` default
   run, so they are outside the measured surface until their feature is
   enabled in the coverage invocation.
3. **Live third-party services** (Google Analytics/CrUX, search console,
   OIDC token exchange) are exercised by the manual, service-backed suite
   in `scripts/verify-release-controls.sh`, not by the unit coverage gate.

Coverage is not gamed by excluding difficult code: webhook delivery, the
OIDC flow, crawl runtime glue, and all handlers remain inside the measured
surface even where they require test doubles to reach.

## Gates

| Gate | Value | Role |
|---|---|---|
| Engine library region coverage floor | 70% | Blocking in CI |
| API all-target region coverage floor | 50% | Blocking in CI |
| 90% region coverage target | 90% | Roadmap success metric, not yet met |

## Measured state (2026-09-05)

| Surface | Region coverage |
|---|---:|
| Engine library | 86.64% |
| API all-target (main.rs excluded) | 73.94% |
| API all-target (raw, bootstrap included) | 68.67% |

## Path to the 90% target

The remaining API gap is concentrated in surfaces that need test doubles
rather than more fixtures: the OIDC success flow, webhook delivery
(SSRF-gated by design, so it needs an injectable client), and crawl
runtime glue. The engine gap is spread across storage edge branches, RUM
aggregation, and the schema-analyzer family.

Claiming the 90% target requires all three of:

1. Engine library and API covered-surface region coverage both at or above
   90% in a CI run;
2. The numeric floors in `.github/workflows/ci.yml` raised to 90% so the
   gate is machine-checked;
3. This document updated with the measured evidence.
