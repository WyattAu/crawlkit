# VERSION.md

**Project:** crawlkit
**Current Phase:** Phase 17 (v3.0.0 Release)
**Version:** 3.0.0
**Status:** Release
**Last Updated:** 2026-08-19
**MSRV:** 1.85.0

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
| 17 | v3.0.0 Release | Complete | 2026-08-19 | Breaking engine API cleanup; signed release artifacts |

## Current State

- **Error Level:** None
- **Rollback Checkpoint:** None
- **Capability Matrix:** Updated
- **Traceability:** Initialized
- **Test Count:** 736 passing (unit+bins: 614, integration: 81 incl. 19 router + 7 WASM-ABI tests, doc: 43; 13 ignored: 7 PostgreSQL, 5 Redis, 1 Playwright — run with service containers) + 2 postgres-feature tests + 49 client-SDK tests (Go 27, Python 12, Node 10) + 41 dashboard tests
- **Analyzer Count:** 31
- **Clippy Warnings:** 0
- **Unsafe Code:** Forbidden (workspace-level)

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

---

*Generated: 2026-08-19 | Version: 3.0.0*
