# Competitive Analysis: crawlkit vs. Open-Source SEO Crawlers

**Date:** 2026-10-31
**Method:** full shallow clones of each competitor codebase, analyzed component-by-component
(clones in /tmp/opencode/competitors). Ahrefs, Semrush, Screaming Frog, Sitebulb are closed-source —
compared via published docs only.

## Competitors analyzed

| Project | Language / size | What it is |
|---|---|---|
| [siteone-crawler](https://github.com/janreges/siteone-crawler) | Rust, ~122k LOC, single binary | Screaming-Frog-style CLI crawler + 20 analyzers + scored HTML reports |
| [open-seo (openwarelabs)](https://github.com/openwarelabs/open-seo) | TypeScript/Next.js, ~17k LOC | self-hosted graded site audits (A+–F), monitors, alerts, MCP |
| [open-seo (every-app)](https://github.com/every-app/open-seo) — openseo.so | TypeScript/TanStack, ~192k LOC | commercial-grade Ahrefs/Semrush alternative (DataForSEO-powered), $10/mo hosted |

*(Ahrefs: closed-source SaaS — crawler details unpublished; its moat is the backlink index, which no
self-hosted tool replicates without a paid data provider.)*

## Capability matrix

| Capability | crawlkit | siteone | open-seo (ow) | open-seo (ea) |
|---|---|---|---|---|
| Async concurrent crawling | ✅ | ✅ (workers) | ⚠️ light | ⚠️ shallow |
| Per-domain rate limiting | ✅ GCRA, proven | ❌ global RPS only | ❌ | ⚠️ throttle |
| robots.txt + sitemap input | ✅ | ✅ | ✅ | ✅ |
| JS rendering | ❌ | ✅ Chromium opt-in | ❌ (via PSI) | ✅ (billed) |
| Analyzer count | ~20 modules | 20 analyzers | 72 checks | dozens |
| Weighted scoring / graded report | ❌ | ✅ 0–10 | ✅ A+–F | ✅ |
| WASM plugin system | ✅ | ❌ | ❌ | ❌ |
| Storage | SQLite | report files | JSON files | D1/Postgres |
| Exports | CSV/JSON/HTML/MD/JSONL/Parquet | HTML/JSON/sitemap/md | PDF/JSON/CSV/share links | UI/MCP |
| REST API server | ✅ | ❌ (report server only) | ⚠️ thin | ✅ app API |
| Scheduled crawls | ✅ | ❌ | ✅ monitors | ✅ |
| Alerting (email/Slack/Discord/webhook) | ❌ | ✅ email | ✅ | ✅ |
| CI quality gate (--ci, GitHub Action) | ❌ | ✅ | ✅ | ⚠️ |
| Deterministic reproducible crawls | ✅ (spec + proofs) | ❌ | ❌ | ❌ |
| Formal verification (Lean 4) | ✅ 26 theorems | ❌ | ❌ | ❌ |
| MCP / AI-agent integration | ❌ | ⚠️ AI assistant | ✅ | ✅ + OAuth |
| Lighthouse / PageSpeed CWV | ❌ | ❌ | ✅ | ✅ |
| Offline cloning / site→Markdown | ❌ | ✅ | ❌ | ❌ |
| Technology (stack) detection | ❌ | ✅ | ❌ | ❌ |
| Keyword research / rank tracking / backlinks | ❌ | ❌ | ❌ | ✅ (DataForSEO, paid) |
| Supply-chain auditing of itself | ✅ | ❌ | ❌ | ❌ |

## crawlkit gaps (ranked by cross-competitor frequency)

1. **MCP server / agent integration** — all three competitors have it. Biggest single gap.
2. **Weighted scoring + prioritized-fix graded reports** — siteone + open-seo(ow).
3. **Alerting on regression** (email/Slack/Discord/webhook) — open-seo(ow) + siteone (email).
4. **CI quality gate** (`--ci` exit codes, GitHub Action) — siteone + open-seo(ow).
5. **Lighthouse/PageSpeed Core Web Vitals integration** — both open-seos.
6. **Monitoring with score history + shareable report links** — open-seo(ow) (crawlkit's scheduler covers half).
7. **Technology (stack) detection** (passive, Wappalyzer-style) — siteone.
8. **Offline cloning + site→Markdown conversion** — siteone (natural fit for crawlkit's AI-search analyzer).
9. **Share-preview rendering** (per-platform OG cards) — open-seo(ow).
10. **White-label/PDF reports; keyword/rank/backlink data** (everyapp only — requires paid DataForSEO, conflicts with crawlkit's zero-API-dependency stance).

## crawlkit unique strengths (none of the competitors have)

1. Lean 4 machine-verified core invariants (26 theorems: GCRA rate limiter, circuit breaker, politeness, PageRank bound, audit chain, DNS freshness, idempotency replay).
2. WASM plugin sandbox for third-party analyzers.
3. Per-domain GCRA rate limiting with proven arrival-curve guarantees (competitors: global throttle at best).
4. Deterministic, reproducible crawls (byte-identical report goal; event-log replay design).
5. 7-format export matrix incl. Parquet/JSONL warehouse targets; SQLite storage layer.
6. Self-auditing supply chain.

## Recommended roadmap (from gaps)

1. MCP server exposing crawl/compare/report/schedule (agents are the new primary users).
2. Weighted scoring engine + prioritized fixes + `--ci` gate mode.
3. Alerting webhooks on compare-diff regressions (reuses existing compare command).
4. Optional Chromium rendering feature flag (parity with siteone/everyapp).
5. Passive technology detection analyzer.
