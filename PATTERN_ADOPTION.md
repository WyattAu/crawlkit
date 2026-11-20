# PATTERN_ADOPTION: crawlkit

Relationship to the WyattAu pattern canon
(`github.com/WyattAu/pattern-standards`), validated by
`scripts/adoption-check.py` per `references/adoption-format.md`.

| Pattern | Status | Evidence |
|---|---|---|
| T13 | partial | `crates/crawlkit-engine` (fetch/analyzer core) vs `crates/crawlkit-api` (HTTP handlers); structural split visible, purity not yet asserted in CI |
| P118 | partial | `crates/crawlkit-budget-check` (usage-budget gate crate) |
| T8 | partial | `crates/crawlkit-api/src/auth_mw.rs`, `crates/crawlkit-api/src/handlers/api_keys.rs`, `crates/crawlkit-api/src/handlers/audit.rs` (capability-checked API surface with audit trail) |
| P31 | n/a | canon baseline line names this repo; code shows no implementation of P31: no LLM, planner, or privileged-prompt surface in any crate (`crates/crawlkit-plugin-sdk/src/host.rs:52` ParsedSummary is a parsed-page data struct, not an inference boundary) |
