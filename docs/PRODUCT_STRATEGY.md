# crawlkit Product Strategy and Path Forward

**Status:** Adopted
**Date:** 2026-09-10
**Inputs:** docs/COMPETITIVE_MATRIX.md (20-competitor analysis), ROADMAP.md (engineering contract), docs/capabilities.toml (verified capability states)
**Decision owners:** Maintainers
**Relationship to ROADMAP.md:** This document sequences and scopes product work. It does not modify the engineering contract: every item below remains subject to the ROADMAP gates (code, test, security, documentation, release) and the Phase 7 candidacy requirements.

---

## 1. Positioning decision

**Adopted: hybrid — engine-led growth now, platform unlock at 6.0.**

The competitive matrix (§3, §4) shows the market buys four things crawlkit is weakest at: data fusion, workflow, hosted convenience, and scale evidence. It also shows crawlkit holds eight advantages no competitor in the set has (matrix §3.2). The hybrid path exploits the second while closing the first in dependency order:

- **5.2.x and 5.3.x optimize for the CI-native OSS audience** (developers, agencies, platform teams running audits in pipelines). Everything in these releases is surface work on internals that already exist — no new trust boundaries except where noted.
- **6.0.0 is the enterprise unlock** (distributed crawl, metering, warehouse export). It is sequenced after the connector work because enterprise buyers evaluate data fusion first; shipping scale before connectors repeats the incumbents' demo-order mistake in reverse.

**Explicitly not chased in 5.x:** brand-suite parity (Semrush's 25-tool breadth), web-wide link index (Ahrefs/Majestic — prohibited by ROADMAP §1 non-goals), agency white-label at scale (WebCEO's wedge — deferred to 6.x report builder).

## 2. The wedge

Positioning language for all public surfaces (README, docs site, launch posts). Three claims, each backed by shipped, tested behavior — not aspiration:

1. **CI-native reproducible audits.** Deterministic crawl mode (`--seed`), stable finding codes (778), JSON-schema-stable output, exit codes designed for pipelines. No competitor in the 20 offers determinism as a feature (matrix §2.4, last row).
2. **Security and AI visibility in one crawl.** 20 security-header analyzers plus 4 AI-crawler/GEO analyzers plus citation-eligibility scoring, in the same pass as SEO. Incumbents are partial here (matrix §2.2); it is the 2026 buying trigger.
3. **The only extensible crawler.** Signed WASM plugin marketplace with content-addressed verification, capability-gated WASI, and fuel/memory isolation. No competitor has any plugin ecosystem (matrix §2.4).

Rule: every public claim in this wedge must trace to `docs/capabilities.toml` status `stable` or `stable-with-configuration`. Anything conditional is marketed as conditional or not at all.

## 3. Release ladder

### 5.2.0 — "Truth and Surface" (CI-native positioning)

| Item | Closes | Roadmap gate | Notes |
|---|---|---|---|
| Registry-generated capability manifest in CI; README counts derived, not hand-copied | Matrix §5 counting caveat (P0-4) | Phase 0.1 exit criterion | Hard prerequisite for all marketing claims in this document |
| `crawlkit schedule add/list/remove/enable/disable` CLI over existing API `ScheduleConfig` | F3 | Phase 3.4 CLI contract tests | No new persistence; API surface already exists |
| GSC integration conditional → stable (docs, error-path tests, token handling review) | F1 part 1 | Phase 1.4, capabilities.toml status change | `gsc.rs` + `cli/gsc.rs` exist |
| Python client 92% → 100% parity | — | Phase 7 candidacy (client-library parity) | VERSION.md records 92% |
| Findings JSON schema published and CI-checked | A1 trust story | Phase 3.4 | Stable output contract for CI consumers |
| Hosted scanner: prototype behind feature flag (see §4) | F9 | Phase 2.4 resource limits | Prototype only; GA gated on 5.3 |

**Exit criteria:** Phase 0.1 acceptance (CI fails on README/manifest drift); CLI help output tested against docs; no conditional capability described as stable anywhere public.

### 5.3.0 — "Connectors" (data fusion + workflow)

| Item | Closes | Roadmap gate | Notes |
|---|---|---|---|
| Secrets and credential storage hardening (per-tenant connector credentials, rotation, no-secret logging verification) | F1/F2 prerequisite | Phase 1.4 acceptance | Blocks everything below |
| GA4 connector (read-only, per-tenant OAuth) | F1 part 2 | Phase 1.4; ADR required (§7) | New external service/credential |
| Alert channels: Slack, Teams, email — built on existing loop-retry webhook delivery | F2 | Phase 3.4 contract tests; Phase 5.1 connector-failure metrics | Webhook payload schema exists (api types.rs) |
| CrUX optional → stable; field CWV in HTML/MD reports | F4 | capabilities.toml status change; docs gate | `crux.rs` exists |
| Per-tenant data-retention configuration (API + docs) | AR9 | Phase 1.4, soc2_features requirements | Purge policy exists; needs tenant-level config surface |
| Hosted scanner: general availability | F9 | Full §4 checklist | See §4 |

**Exit criteria:** connector credentials never appear in logs/errors/exports (tested); alert delivery has retry + failure metrics; CrUX data path proven by test against fixture collector (mirroring the OTel test pattern in Phase 5.2 acceptance).

### 6.0.0 — "Scale" (enterprise unlock; major version)

| Item | Closes | Roadmap gate | Notes |
|---|---|---|---|
| Redis queue graduation: documented delivery semantics, leases, retries, poison handling, crash-recovery tests | AR4 | Phase 2.3 full acceptance | ADR required (§7) |
| Distributed mode stable; published capacity evidence (10k and 100k URL crawls with memory/fd/task bounds) | AR1 | Phase 2.4, 4.3 acceptance | `--distributed-mode` exists; graduation is the work |
| Warehouse exporters: BigQuery, Snowflake, S3 (Parquet), versioned schemas | AR2 | Phase 3.4; ADR for schema contract | Adds to SQLite/PG export paths |
| Render budgets: per-crawl render quota, per-page budget, JS-error findings, rendering telemetry | F5 | Phase 4.1 benchmark class 5; Phase 5.1 metrics | Playwright path exists |
| Usage metering + quota surface (API + dashboard) | AR8 | Phase 3.4; ADR for API changes | Replaces aspirational docs/BILLING.md |

**Exit criteria:** Phase 6.2/6.3 release gate in full (checksums, SBOM, migration/rollback evidence); capacity report committed with raw artifacts per Phase 4.2; no experimental capability described as production-supported.

### 6.x — "Product surface" (post-unlock, Phase 7 candidacy in full)

Trends dashboard UI (F6) · issue triage workflow — assign/ignore/resolve/verify, requires storage schema ADR (F8) · white-label report builder with scheduled email (F7) · second-party plugin marketplace with provenance attestations (A1) · localized report templates (F10).

Each item enters development only with the full Phase 7 candidacy record (user problem, lifecycle, security model, operational cost, API/CLI contract, test plan, maintenance owner, deprecation plan).

## 4. Candidate: hosted free scanner (approved 2026-09-10)

Phase 7 candidacy record:

- **User problem.** Every SaaS competitor's top-of-funnel is a free URL scan (matrix F9). crawlkit has no hosted surface; OSS distribution alone does not reach the buyers who evaluate via "paste a URL."
- **Supported lifecycle.** MVP behind feature flag in 5.2.0 (prototype, internal feedback); GA in 5.3.0. Deprecation plan: the scanner is a stateless facade over `inspect` + capped crawl; decommission = remove facade, users keep CLI capability.
- **Security model.**
  - SSRF: shared engine policy (Phase 1.2) with private/link-local/metadata ranges denied; `allow_private` unavailable on the hosted path by construction, not by flag discipline.
  - No authentication → per-IP and per-target rate limits; global per-target politeness cap across all users (a scanner is a crawler against third-party sites; per-IP limits alone permit distributed abuse of one victim).
  - Caps: ≤25 pages per scan, render disabled by default, body-bytes and findings caps per Phase 2.4 budgets; results cached per URL for a fixed TTL.
  - Output redaction: no headers/secrets in responses beyond what the finding model exposes; API-key redaction pattern reused.
- **Operational cost.** Egress and compute scale with adoption; mitigations are the caps above plus a daily global scan budget with graceful degradation (queue-full response, not silent truncation). Owner: maintainers; runbook required per Phase 5.3 before GA.
- **API/CLI contract.** Public endpoint wraps `inspect` semantics; documented as a separate hosted-service contract — it does not extend the self-hosted API surface, so hosted changes cannot break API consumers.
- **Test plan.** Contract tests for the facade; abuse-path tests (rate limits, caps, SSRF denial on private targets); load test at the documented daily budget.
- **Maintenance owner.** Maintainers, with the ops runbook and monitoring dashboards (Phase 5.3, 5.1) as GA prerequisites.
- **Honest scope note.** A 25-page scan is a teaser, not an audit. Conversion path: scan → full report via email capture → self-host or API. The scanner must not be described as a site audit.

## 5. Non-goals (retained and extended)

Retained from ROADMAP §1: not a SIEM, vulnerability scanner, search engine, or marketing platform; no formal-verification, HFT, or certification claims.

Extended for this strategy: no web-wide link index; no acquisition-led suite breadth; no hosted multi-tenant crawl farm before 6.0.0 gates pass; no scanner GA without the §4 security checklist; no public claim without a capabilities.toml `stable` (or `stable-with-configuration`) status.

## 6. Success metrics

| Horizon | Metric | Target |
|---|---|---|
| 5.2.0 | Manifest CI check enforced; README counts generated | Zero drift possible |
| 5.2.0 | Public claims traceable to capabilities.toml | 100% |
| 5.3.0 | Connector adoption (GSC/GA4/alert channels in telemetry, opt-in) | Baseline established |
| 5.3.0 | Scanner: scan success rate, p95 latency, abuse-block rate | Runbook thresholds defined pre-GA |
| 6.0.0 | Capacity evidence published (10k/100k) with raw artifacts | Done/not-done |
| 6.0.0 | Distributed-mode crash-recovery and duplicate-delivery tests | Green in CI |

## 7. Decision log (ADRs required)

| Trigger (ROADMAP §7) | When |
|---|---|
| GA4 connector — new external service and credential | 5.3.0 design |
| Alert channel providers — new external services | 5.3.0 design |
| Redis queue semantics — new persistence/queue semantics | 6.0.0 design |
| Warehouse exporter schemas — new persistence semantics | 6.0.0 design |
| Metering/quota API — public API change | 6.0.0 design |
| Triage workflow — storage schema change | 6.x design |
| Hosted scanner — new trust boundary (public unauthenticated surface) | 5.2.0 prototype — discharged by ADR-012 (`.adrs/ADR-012-hosted-scanner-trust-boundary.md`) |

## 8. Review cadence

This document is reviewed at each release gate listed in §3 and re-baselined against a refreshed docs/COMPETITIVE_MATRIX.md every six months (next: 2027-03-10) or on any material competitor move (pricing-model change, AI-visibility feature parity, plugin-system announcement).

---

*Per the claims policy: this document proposes; nothing here is claimed as shipped until the corresponding release gate passes and capabilities.toml records the new status.*
