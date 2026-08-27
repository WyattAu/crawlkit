# VERSION.md

**Project:** crawlkit
**Current Phase:** Phase 25 (v5.1.0 Depth)
**Version:** 5.1.0
**Status:** In Development
**Last Updated:** 2026-08-27
**MSRV:** 1.94.0

---

## Phase History

| Phase | Name | Status | Date | Artifacts |
|-------|------|--------|------|-----------|
| -1 | Context Discovery | Complete | 2026-07-23 | domain_analysis.md, applicable_standards.md, capability_requirements.md |
| 0 | Requirements Engineering | Complete | 2026-07-23 | requirements.md, acceptance_criteria.md |
| 1 | Deep Testing & Code Quality Audit | Complete | 2026-07-26 | 608 tests passing, 0 clippy warnings, 31 analyzers |
| 2 | CI/CD Pipeline Audit & Debugging | Complete | 2026-07-26 | Pinned actions, cargo-deny fixed, MSRV unified |
| 3 | GUI & UI/UX Evaluation | Complete | 2026-07-26 | Design tokens wired, reduced-motion, unused deps removed |
| 4 | Documentation Overhaul | Complete | 2026-07-26 | README rewritten, zero emojis, technical precision |
| 16 | v2.1.0 Release | Complete | 2026-08-18 | Security hardening, WASM ABI conformance, persistent audit |
| 17 | v3.0.0 Release | Complete | 2026-08-19 | Breaking engine API cleanup; plugin trust chain; determinism rails; API backpressure/idempotency; signed release artifacts |
| 24 | v5.0.0 Ground Truth | In Progress | 2026-08-27 | Type unification (crawlkit-types); DIP fix (dyn StorageBackend); client library completion; doc reconciliation; SSRF dedup; dead code gating |

## Current State

- **Error Level:** None
- **Rollback Checkpoint:** None
- **Capability Matrix:** Updated
- **Traceability:** Initialized
- **Test Count:** 909 passing (unit/lib: 754, integration: 111, doc: 44; 12 ignored)
- **Analyzer Count:** 62
- **Clippy Warnings:** 0
- **Unsafe Code:** Denied (workspace-level); FFI crates override to allow with SAFETY comments
- **Workspace Crates:** 5 (crawlkit, crawlkit-api, crawlkit-engine, crawlkit-plugin-sdk, crawlkit-types)
- **Client Libraries:** Python 92%, Go 100%, Node.js 100%

## Breaking Changes (v5.0.0)

| Change | Migration |
|--------|-----------|
| `Finding.category` type: `String` → `IssueCategory` | Plugin authors: use `IssueCategory::Seo` etc. instead of `"seo".into()`. `From<&str>` provided for easy migration. |
| `CrawlEngine::new()` accepts `impl StorageBackend` | Callers passing concrete `Storage` still work (coercion). Callers using `Arc<Storage>` must coerce to `Arc<dyn StorageBackend>`. |
| `Severity` gains `Copy` | `.clone()` calls on `Severity` are now unnecessary (clippy will flag). |
| `PluginFindingJson` removed | Internal to engine; no external impact. Plugin JSON now deserializes directly to `Finding`. |
| Aspirational docs archived | `ENTERPRISE_ARCHITECTURE.md`, `WIRING_PLAN.md`, `COMPETITIVE_ANALYSIS.md` moved to `docs/archive/`. `PERFORMANCE_BENCHMARKS_FINAL.md` deleted. |

## Version History

| Version | Date | Change |
|---------|------|--------|
| 0.1.0 | 2026-07-22 | Initial release |
| 0.1.1 | 2026-07-23 | Phase -1 complete |
| 0.4.0 | 2026-07-23 | Major release: 28 analyzers, AI/WASM, REST API, cross-platform |
| 2.0.0 | 2026-07-25 | Full rewrite: 31 analyzers, security hardening, quality gates |
| 2.1.0 | 2026-07-26 | Production hardening: rustdoc examples, bug fixes |
| 2.2.0 | 2026-07-26 | Streaming parser, native plugins, webhooks, schedules, SSO |
| 2.3.0 | 2026-07-27 | Distributed queue, PostgreSQL backend, Core Web Vitals |
| 3.0.0 | 2026-08-19 | Breaking: infallible parser, lean Analyzer trait, structured errors; security hardening arc; signed releases |
| 3.0.1 | 2026-08-22 | 5-platform binaries; wasmtime 47.0.4 + sqlx 0.9 (deny ignores 5->3); semver gate enforcing; dogfood protocol; mutants baseline |
| 4.0.0 | 2026-08-22 | WC004 corpus-consistency fix (dogfood-found); ParsedPage.sentence_count (major); ISEO006/AI-AB007 noise removal; IMG004 offenders named; mutants 53.6%->87.7% |
| 4.1.0 | 2026-08-22 | Plugin marketplace (git-based index, install/list/remove, pre-install trust verification); llvm-cov coverage gate (71% floor); seo-wasm crate removed |
| 4.2.0 | 2026-08-23 | First-party index seeded (title-length, viewport-checker; ADR-010); remote-index URL resolution fixed; UN M49 region bug fixed; analyzer fixture coverage |
| 4.3.0 | 2026-08-23 | Structured guest context (B4): crawlkit_host.get_context + analyze_with_context + SDK host module; soft-404 example |
| 4.4.0 | 2026-08-23 | Plugins execute during crawls (plugin_dirs config, --plugins CLI, default ~/.crawlkit/plugins; trap-safe, E2E-tested) |
| 4.4.1 | 2026-08-23 | GPG-signed checksums (first signed release); ADR-011 WASI eval; OSS-Fuzz submission kit |
| 5.0.0 | 2026-08-27 | Breaking: unified types (crawlkit-types crate); DIP fix (dyn StorageBackend); client library completion (Python 92%, Go/Node 100%); SSRF dedup; dead code gated; docs reconciled |

---

*Generated: 2026-08-27 | Version: 5.0.0*
