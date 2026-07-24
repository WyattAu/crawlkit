# ADR-001: WASM Error Detection & AI Result Optimization Analyzers

| Field | Value |
|-------|-------|
| **ADR ID** | ADR-001 |
| **Title** | Add WASM Error Detection and AI Result Optimization Analyzers |
| **Status** | Proposed |
| **Date** | 2026-07-23 |
| **Author** | Nexus (Principal Systems Architect) |
| **Deciders** | Project Owner, Architect |
| **Related Standards** | IEEE 1016 (Software Design Description), SEO industry practices |
| **Related ADRs** | None (first ADR) |
| **Related Docs** | `.adrs/coding-standards.md` (Engineering Standards v1.0.0) |

---

## Context

### Problem Statement

crawlkit currently detects 23 categories of SEO/technical issues. Two emerging gaps exist:

1. **WASM Error Detection:** Sites increasingly ship WebAssembly modules for core functionality (image processing, cryptography, SPA rendering). Standard crawlers detect broken HTML/JS but miss WASM-specific failures that break page functionality and hurt Core Web Vitals.

2. **AI Result Optimization:** AI-generated search results (Google SGE/AI Overviews, Bing Copilot, Perplexity, ChatGPT Browse) now influence 15-30% of search traffic. Sites that don't optimize for AI crawlers lose visibility. No existing crawler in the market provides AI-specific optimization signals.

### Constraints

- Must fit existing `Analyzer` trait architecture (zero changes to core pipeline)
- WASM runtime detection requires Playwright integration (Phase 7.1 dependency)
- AI analyzer signals must be actionable (not just informational)
- No new external dependencies for static analysis analyzers
- Performance budget: new analyzers must not increase per-page analysis time >50ms

### Assumptions

- AI search traffic will continue growing (conservative: 20% CAGR)
- WASM adoption in web apps will increase (already at ~5% of top-1M sites)
- Playwright integration (Phase 7.1) will ship before these analyzers
- `robots.txt` AI bot directives will become standardized (Google-Extended already draft)

---

## Decision

**We will implement two new analyzer groups:**

### Group 1: WASM Error Detection (3 analyzers)

| Analyzer | Type | Dependency |
|----------|------|------------|
| `WasmPatternAnalyzer` | Static analysis | None (ships with v1.0) |
| `WasmRuntimeAnalyzer` | Dynamic analysis | Playwright (Phase 7.1) |
| `WasmPerformanceAnalyzer` | Static + Dynamic | None (static), Playwright (dynamic) |

### Group 2: AI Result Optimization (4 analyzers)

| Analyzer | Type | Dependency |
|----------|------|------------|
| `AiCrawlerAccessibilityAnalyzer` | Static analysis | None |
| `AiContentStructureAnalyzer` | Static analysis | None |
| `AiCitationEligibilityAnalyzer` | Static analysis | None |
| `AiAnswerBoxAnalyzer` | Static analysis | None |

### Implementation Order

1. **Phase 1 (immediate):** AI analyzers (static, no dependencies)
2. **Phase 2 (post-Phase 7.1):** WASM static pattern analyzer
3. **Phase 3 (post-Phase 7.1):** WASM runtime + performance analyzers

---

## Alternatives Considered

### Alternative 1: External Tool Integration (e.g., Lighthouse WASM audit)

| Pros | Cons |
|------|------|
| No custom code needed | Adds heavy dependency (Chrome/Chromium) |
| Battle-tested audit logic | Slow (10-30s per page) |
| | Can't customize for crawlkit-specific needs |
| | Doesn't cover AI optimization signals |

**Rejected:** Too heavy for high-throughput crawling. crawlkit targets ≥50 pages/sec.

### Alternative 2: JavaScript-Based Analyzers (Node.js plugins)

| Pros | Cons |
|------|------|
| Easy WASM runtime detection | Cross-language plugin complexity |
| Rich JS ecosystem for web analysis | Performance penalty (IPC overhead) |
| | Breaks Rust-only architecture |

**Rejected:** Violates single-language architecture. Performance unacceptable.

### Alternative 3: Do Nothing (Defer to External Tools)

| Pros | Cons |
|------|------|
| No development cost | Loses competitive advantage |
| | Users must use separate tools |
| | Missing critical SEO signals |

**Rejected:** AI optimization is a first-mover opportunity. WASM errors are a real user pain point.

---

## Consequences

### Positive

- **First-mover advantage:** No other crawler provides AI-specific SEO signals
- **Comprehensive coverage:** WASM errors detected that HTML/JS analysis misses
- **Revenue opportunity:** AI optimization signals could be premium feature
- **Ecosystem growth:** WASM plugin system (already planned) benefits from better WASM understanding

### Negative

- **Maintenance burden:** 7 new analyzers to maintain and test
- **Playwright dependency:** WASM runtime analyzer blocked until Phase 7.1
- **AI signal volatility:** AI search algorithms change frequently; signals may become stale
- **False positives:** WASM static analysis may flag benign patterns

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| AI search algorithms change, invalidating signals | High | Medium | Abstract signal definitions; configurable rules |
| WASM static analysis produces false positives | Medium | Low | Conservative thresholds; user-configurable sensitivity |
| Playwright integration delayed | Medium | High | Ship static WASM analyzer first; runtime analyzer gated |
| New AI bots appear, not covered by robots.txt parser | Medium | Medium | Extensible bot list; configurable via TOML |

---

## Proof of Correctness

- All new analyzers implement existing `Analyzer` trait (compile-time verification)
- Static analyzers can be validated against known-bad pages (test vectors)
- AI analyzer signals validated against top-100 sites (empirical validation)
- WASM analyzer validated against sites known to use WASM (GitHub Pages, Figma, etc.)

---

## Evidence

| Evidence Type | Description | Confidence |
|---------------|-------------|------------|
| Industry data | AI search usage growing 30%+ QoQ (SparkToro, 2026) | 0.85 |
| Technical analysis | WASM sites show console errors invisible to HTML parsers | 0.90 |
| Competitive analysis | No existing crawler provides AI-specific signals | 0.95 |
| Architecture review | Existing `Analyzer` trait supports new analyzers without modification | 0.99 |

---

## Verification Protocol

1. **Unit tests:** Each analyzer tested with synthetic HTML inputs
2. **Integration tests:** Analyzers run against real crawled pages
3. **Regression tests:** New findings don't duplicate existing analyzer output
4. **Performance tests:** Per-page analysis time stays under 50ms budget
5. **Empirical validation:** Run against top-100 sites; review findings for accuracy

---

## Quality Requirements

All new analyzers must comply with `.adrs/coding-standards.md` (Engineering Standards v1.0.0).

### FAANG Requirements

| Requirement | Threshold | Measurement |
|-------------|-----------|-------------|
| Cyclomatic complexity | ≤ 10 per function | `cargo clippy` |
| Cognitive complexity | ≤ 15 per function | `cargo clippy` |
| Function length | ≤ 30 lines | Manual review |
| Test coverage | ≥ 90% branch | `cargo tarpaulin` |
| Documentation | 100% public items | `cargo doc` |
| Code review | ≥ 1 approval | GitHub PR |

### HFT/ECN Requirements

| Requirement | Threshold | Measurement |
|-------------|-----------|-------------|
| Zero-allocation hot path | 0 allocs in steady state | `dhat` profiling |
| Per-page latency | < 50ms analysis | Criterion benchmark |
| Backpressure | Bounded channels only | Code review |
| Determinism | Same input → same output | Test vectors |
| Circuit breaker | Per-domain isolation | Integration test |

### Defence Requirements

| Requirement | Threshold | Measurement |
|-------------|-----------|-------------|
| Audit trail | Every state-change event | Audit log query |
| Input validation | 100% boundary validation | Unit tests |
| SQL injection | 0 string interpolation | `cargo audit` + review |
| Formal verification | Critical algorithms proven | Lean4/Coq proofs |
| Tamper evidence | SHA-256 chaining | Audit log verification |

### Analyzer-Specific Quality Gates

| Analyzer | Additional Gate |
|----------|-----------------|
| `WasmPatternAnalyzer` | ≤ 5% false positive rate on test vectors |
| `WasmRuntimeAnalyzer` | Requires Playwright integration test |
| `AiCrawlerAccessibilityAnalyzer` | Bot registry validated against real robots.txt |
| `AiContentStructureAnalyzer` | Content signals validated against top-100 sites |
| `AiCitationEligibilityAnalyzer` | Citation signals correlated with actual AI citations |
| `AiAnswerBoxAnalyzer` | Schema detection validated against Google Rich Results |

---

## Related Documentation

- `docs/ROADMAP.md` — Phase 4.4 (WASM plugins), Phase 7.1 (Playwright)
- `crates/crawlkit-engine/src/analyzers.rs` — Existing analyzer implementations
- `crates/crawlkit-engine/src/parser.rs` — HTML parsing (script tag extraction)

---

*Generated: 2026-07-23 | Version: 1.0.0 | Status: Proposed*
