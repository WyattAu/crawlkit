# GAP CLOSURE PLAN — crawlkit vs. competitors (all 10 gaps)

**Document ID:** REQ-GAPCLOSE-001
**Version:** 1.0.0
**Date:** 2026-10-31
**Source:** `docs/COMPETITOR_COMPARISON.md` (siteone-crawler, open-seo ×2; Ahrefs closed-source)
**Policy:** every gap gets a spec, an owner module, acceptance criteria, and a proof/verification
story consistent with the research program (`ITERATION_LEDGER.md`). No feature ships without
either a machine-checked invariant or an explicit property-test suite.

---

## G1. MCP server / agent integration  🔴 P0 (all 3 competitors)

**Design:** new crate `crawlkit-mcp`. Expose crawlkit over the Model Context Protocol
(JSON-RPC over stdio + Streamable HTTP), tools only (no resources/prompts in v1):
`crawl` (start + poll), `compare`, `report`, `schedule_list/add/toggle`, `export`,
`findings_search` (SQL over the SQLite store), `doc` (get extracted text of a URL from the
last crawl). Auth: optional API key (reuse `crawlkit-api` key store). Transport framing via
`rmcp` (official Rust SDK) — do not hand-roll JSON-RPC.
**Files:** `crates/crawlkit-mcp/{Cargo.toml,src/{lib,tools,convert}.rs}`, workspace member,
README section. Reuses `crawlkit-engine` + `crawlkit-api` storage — no new analysis code.
**Acceptance:** `crawlkit mcp` runs stdio server; smoke-tested against an MCP inspector
client; tools round-trip on the example crawl fixture. Property test: every tool schema
validates against the MCP meta-schema.
**Effort:** M (2–3 days).

## G2. Weighted scoring + prioritized fixes  🔴 P0 (2 competitors)

**Design:** scoring module in `crawlkit-engine::scoring`. Each finding type gets
`(category, severity weight)` from a TOML table `scoring.toml` (versioned — scoring changes
are breaking for report diffs and MUST be recorded; determinism rule from ITER-09 applies).
Page score = `100 × Π (1 − wᵢ)` over finding weights (multiplicative so one catastrophic
issue dominates); site score = traffic-weighted? No — deterministic: page-count weighted
mean, 2 fixed decimals. Category scores reported per competitor convention
(seo/perf/a11y/security/best-practice). Prioritized fixes: findings sorted by
`(weight × pages_affected)` desc — deterministic tie-break by rule ID.
**Verification:** property tests — score ∈ [0,100]; removing a finding never lowers a page
score (monotonicity); identical crawl ⟹ identical scores (determinism rails). Lean follow-up:
scoring monotonicity is a 5-line proof candidate for `CrawlkitProofs`.
**Files:** `crawlkit-engine/src/scoring/{mod,weights}.rs`, `scoring.toml`, report renderers.
**Effort:** M.

## G3. Alerting on regression (email/Slack/Discord/webhook)  🟡 P1

**Design:** `crawlkit alert` + `--alert-webhook URL --alert-email TO` on `compare` and
`scheduled` runs. Event model: `{event: "regression"|"improvement"|"error", crawl_ids,
findings_delta, scores}` (JSON, RFC 8785-canonical — reuse ITER-09). Delivery: webhook =
signed POST (`X-Crawlkit-Signature: hmac-sha256(key, body)`), retries ×3 exponential;
email via SMTP (lettre crate) — feature-gated `smtp`. Slack/Discord = webhook adapters
(formatting only). Config in `[alerts]` TOML. Secrets never logged.
**Acceptance:** contract tests against a local HTTP stub; signature verification test;
scheduled-run integration test fires exactly once per regression (deterministic).
**Effort:** M.

## G4. CI quality gate  🟡 P1 (2 competitors)

**Design:** `crawlkit report --ci [--max-errors N] [--max-warnings M] [--min-score S]` —
exit `0` pass, `10` errors exceeded, `11` warnings exceeded, `12` score below floor,
`13` score regression vs baseline crawl (`--baseline <dir>` reuses `compare`).
Plus a publishable GitHub Action wrapper repo `crawlkit-action` (Docker-based, inputs =
flags). Exit codes documented in `--help` and docs site.
**Acceptance:** integration tests asserting each exit code on fixture crawls; docs page.
**Effort:** S (2–3 days).

## G5. Lighthouse / PageSpeed CWV integration  🟡 P1 (both open-seos)

**Design:** external-process integration, NOT a Rust Lighthouse port: `crawlkit lighthouse
<url|crawl>` shells out to `lighthouse`/`pageview` CLI if installed (feature `lighthouse`),
ingests JSON output, stores CWV (LCP/INP/CLS/TTFB) per page as findings; PSI API mode
(feature `psi`, needs API key, rate-limited via GCRA — reuse ProofGcra!). Graceful
"not installed" finding instead of hard failure. CWV thresholds from web.dev (LCP ≤2.5s etc).
**Acceptance:** fixture JSON ingestion tests (no network); GCRA-bounded PSI calls property
test; findings render in reports.
**Effort:** M.

## G6. Monitoring with score history  🟡 P1

**Design:** extends existing scheduler: every scheduled run appends a score snapshot
`(run_id, ts, page, category, score)` to SQLite; new command `crawlkit history <target>`
prints/stores the trend; `--alert-on-drop X` fires G3 when score drops > X points.
Shareable links: `crawlkit-api` gains `GET /reports/{run_id}` returning the HTML report
(auth-gated) — the "share link" primitive.
**Acceptance:** history table migration test; two-run fixture produces correct delta;
API route integration test.
**Effort:** M (G3 dependency).

## G7. Technology (stack) detection  🟢 P2

**Design:** passive-only analyzer `tech-detection` (no extra requests): match on response
headers (`x-powered-by`, `server`), HTML fingerprints (generator meta, known asset paths,
script srcs), cookie names. Signatures compiled into a static TOML (curated subset of
Wappalyzer's, license-checked — wappalyzer licenses are GPL; write our own signatures from
public knowledge to keep Apache-2.0 clean). Output: findings `tech:{name}` with confidence.
**Acceptance:** fixture-HTML unit tests per signature; no additional network requests
(testable); false-positive rate check on the top-100 fixture corpus.
**Effort:** M.

## G8. Offline cloning + site→Markdown  🟢 P2 (siteone differentiator)

**Design:** `crawlkit clone <url> --out DIR` — rewrites absolute URLs to relative paths,
mirrors assets under `assets/`, honors politeness (GCRA) and robots; `crawlkit md <url>
--out DIR` — content extraction (reuse existing parser) → one Markdown per page + merged
`site.md` with dedup mode for AI ingestion. Rewriting uses the same canonicalization as
dedup (BP-URL-QUEUE-001).
**Acceptance:** fixture-site clone test (local HTTP server): all links resolve offline;
Markdown round-trip contains extracted headings/text; politeness invariant holds during
clone (existing ProofQueue spacing theorem's runtime assertion).
**Effort:** L.

## G9. Share-preview rendering  🟢 P3

**Design:** low value for a CLI-first tool; implement as report section, not a service:
`crawlkit report --section previews` renders per-platform OG/Twitter card mock frames
(HTML/CSS only, image dims measured from stored responses). No new infrastructure.
**Effort:** S.

## G10. White-label/PDF + keyword/rank/backlink data  🟢 P3 (conditional)

**PDF:** HTML report already print-ready → add `@media print` CSS + `crawlkit report
--pdf` shelling to `weasyprint`/`chromium --print-to-pdf` if present. S effort.
**SEO data (keywords/rank/backlinks):** deliberately OUT of scope for v1 — requires a paid
third-party (DataForSEO) contradicting crawlkit's zero-API-dependency and determinism
stances. Revisit as a plugin (G-WASM) so the dependency stays optional and sandboxed.

---

## Sequencing

| Wave | Gaps | Rationale |
|---|---|---|
| 1 | G4, G2 | smallest, unblock CI adoption + everything score-related |
| 2 | G1 | biggest competitive gap; API/storage already exist |
| 3 | G3, G6 | alerting + history build on scheduler |
| 4 | G5, G7 | analyzer-layer additions |
| 5 | G8, G9, G10 | largest surface / lowest frequency |

## Cross-cutting rules (from the research program)

- Determinism: every new feature must preserve byte-identical reports (ITER-09 rules).
- Politeness: every new network touch goes through the GCRA layer (ProofGcra invariants).
- Proofs: new state machines (alert dedup, score monotonicity) get Lean theorems or an
  explicit property-test justification in this ledger.
- No new dependency without a supply-chain audit pass (cargo-deny).
