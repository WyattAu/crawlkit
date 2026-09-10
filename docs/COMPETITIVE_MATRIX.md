# Competitive Matrix: crawlkit vs. 20 Competitors

**Last updated:** 2026-09-10
**crawlkit version analyzed:** 5.1.0 (workspace) / 5.0.0 (capabilities.toml)
**Methodology:** Feature-by-feature comparison based on vendor documentation and public product pages (accessed 2026-09-10) for competitors; source code, `docs/capabilities.toml`, and `docs/ANALYZER_CATALOG.md` for crawlkit. A source URL demonstrates that a capability is *documented*; it does not independently verify performance, completeness, or the absence of a feature. No competitor performance/resource figures are used unless independently reproducible. crawlkit cells cite repo evidence; competitor cells cite vendor URLs in the sources register (§7).

**Legend:** ✅ documented & shipped · ⚠️ partial/conditional/experimental (see note) · ❌ not offered · "—" not documented by vendor (absence cannot be independently confirmed)

---

## 1. Competitor Register (20)

| # | Competitor | Type | Entry price (documented) | Primary positioning |
|---|-----------|------|--------------------------|---------------------|
| 1 | Screaming Frog SEO Spider | Desktop crawler | Free ≤500 URLs; £199–259/yr | De-facto standard technical crawler; custom extraction, log analyser, 300+ issues |
| 2 | Ahrefs Site Audit | Cloud SaaS module | $129/mo (Lite) | 100+ technical checks inside backlink/keyword suite; scheduled cloud crawls |
| 3 | Semrush Site Audit | Cloud SaaS module | $130/mo (Pro) | 140+ checks, thematic reports, scheduling, CWV, log-file analyzer (higher tiers) |
| 4 | Sitebulb | Desktop + Cloud crawler | $13.50–42/mo | Audit hints (100–300+), visualizations, crawl maps, prioritization |
| 5 | Lumar (ex-DeepCrawl) | Enterprise cloud platform | Custom | Enterprise-scale crawling (450 URLs/s claim), monitoring, rendering service, API |
| 6 | Botify | Enterprise cloud platform | Custom | Crawl + log + analytics triad; JS rendering; SpeedWorkers rendering service |
| 7 | OnCrawl | Enterprise cloud platform | Custom | Crawl-to-log cross-analysis, data warehouse, GSC/GA4/BI integrations |
| 8 | seoClarity (Clarity Audits) | Enterprise suite | Custom | 100+ checks, Bot Clarity log analysis, Content Guard page monitoring |
| 9 | ContentKing (Conductor Monitoring) | Real-time monitoring | Custom | Continuous page-level change tracking and instant alerts |
| 10 | Conductor | Enterprise intelligence platform | Custom | 24/7 monitoring (via ContentKing), AI/AEO reporting, content workflow |
| 11 | JetOctopus | Cloud crawler + log analyzer | $30–237/mo | Fast cloud crawls, log analysis, GSC/GA4 inclusion, no-domain-limit model |
| 12 | Ryte | Quality assurance platform | Custom | Crawl-based QA, monitoring, content success modules; part of Semrush ecosystem |
| 13 | SE Ranking Website Audit | Cloud SaaS module | $65–103/mo | 115–120+ checks, up to 2M pages/project, on-page checker |
| 14 | Moz Pro Site Crawl | Cloud SaaS module | $49/mo (Starter) | Campaign-bound site crawl, issue prioritization, recrawl scheduling |
| 15 | Serpstat Site Audit | Cloud SaaS module | $69–129/mo | Audit up to 1.5M pages/mo, JS crawling (credit-based), audit API |
| 16 | WebCEO | Cloud SaaS suite | ~$31–99/mo | Agency-oriented audit + white-label reporting + technical auditor |
| 17 | SEO PowerSuite (WebSite Auditor) | Desktop suite | Free; $299–599/yr | Desktop audit + link + rank tools; unlimited crawl on paid tiers |
| 18 | Netpeak Spider | Desktop crawler | $7–19/mo | Lightweight desktop audit, custom search/extraction, GA/GSC integration |
| 19 | Siteimprove | Enterprise DX platform | Custom | SEO + WCAG accessibility + QA governance combined; enterprise compliance angle |
| 20 | SEORadar | Change-monitoring point tool | Paid SaaS | SEO-diff monitoring of page changes with deployment-integration alerts |

Related open-source, non-commercial comparators (not counted in the 20): Xenu's Link Sleuth, Beam Us Up, Katana, Firecrawl, Open SEO Crawler. These compete for the "free crawler" slot; crawlkit's OSS license makes them the closest substitutes in that segment.

---

## 2. Capability Matrix

### 2.1 Crawling

| Capability | crawlkit | SF | Ahrefs | Semrush | Sitebulb | Lumar | Botify | OnCrawl | seoClarity | ContentKing | Conductor | JetOctopus | Ryte | SE Rank | Moz | Serpstat | WebCEO | SEO PS | Netpeak | Siteimp | SEORadar |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| HTTP crawler (robots/sitemap-aware) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ a |
| JS rendering | ⚠️ b | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ❌ | ⚠️ | ⚠️ | ✅ | ⚠️ | ❌ | ✅ | ⚠️ | ❌ | ⚠️ | ✅ | ❌ |
| Custom extraction (CSS/XPath/regex) | ✅ c | ✅ | ⚠️ | ❌ | ⚠️ | ⚠️ | ✅ | ⚠️ | ⚠️ | ❌ | ❌ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ✅ | ⚠️ | ❌ |
| Config-file-driven crawl (TOML/YAML) | ✅ d | ⚠️ e | ❌ | ❌ | ❌ | ⚠️ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Incremental crawling (conditional GET) | ✅ f | ✅ | ⚠️ | ⚠️ | ⚠️ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| Distributed / multi-instance crawl | ✅ g | ❌ | n/a | n/a | ❌ | ✅ | ✅ | ✅ | ✅ | n/a | ✅ | ⚠️ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Allowlist/blocklist URL patterns | ✅ h | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |

Notes: a — SEORadar monitors rendered pages, not a general crawler; b — requires Playwright runtime (`capabilities.javascript_rendering: optional`); c — `extraction.rs`, config-declared rules; d — `crawlkit.toml` envstack config; e — SF has config file on paid tiers; f — `--incremental` (ETag/If-Modified-Since); g — `--distributed-mode` with instance partitioning (Redis queue experimental); h — `--include`/`--exclude` globs.

### 2.2 Analysis coverage

| Capability | crawlkit | SF | Ahrefs | Semrush | Sitebulb | Lumar | Botify | OnCrawl | seoClarity | ContentKing | Conductor | JetOctopus | Ryte | SE Rank | Moz | Serpstat | WebCEO | SEO PS | Netpeak | Siteimp | SEORadar |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| On-page SEO checks | ✅ (100+) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| Structured-data validation (deep, per-type) | ✅ (58 types) | ⚠️ | ⚠️ | ⚠️ | ✅ | ⚠️ | ⚠️ | ❌ | ⚠️ | ❌ | ❌ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ❌ | ⚠️ | ⚠️ | ⚠️ | ❌ |
| Security header audit (CSP/HSTS/SRI/COEP…) | ✅ (20 analyzers) | ❌ | ⚠️ | ⚠️ | ❌ | ⚠️ | ❌ | ❌ | ⚠️ | ❌ | ❌ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ | ⚠️ | ❌ |
| Accessibility (WCAG) checks | ✅ (13 analyzers) | ❌ | ❌ | ❌ | ⚠️ | ✅ | ❌ | ❌ | ⚠️ | ❌ | ⚠️ | ⚠️ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| AI-crawler / GEO (GPTBot, llms.txt, citation signals) | ✅ (4 analyzers + LLM hook) | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | ⚠️ | ⚠️ | ❌ | ❌ | ⚠️ | ❌ |
| Core Web Vitals (lab) | ✅ (built-in perf analyzers) | ✅ (PSI API) | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ❌ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ⚠️ | ⚠️ | ✅ | ❌ |
| Core Web Vitals (CrUX field) | ⚠️ (crux.rs, optional) | ❌ | ❌ | ⚠️ | ⚠️ | ✅ | ✅ | ❌ | ⚠️ | ❌ | ⚠️ | ❌ | ⚠️ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Readability/content quality scoring | ✅ (5 formulae) | ⚠️ | ⚠️ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ✅ | ⚠️ | ✅ | ❌ |
| Duplicate content detection (cross-page) | ✅ (cosine sim) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ❌ |
| Keyword cannibalization detection | ✅ | ❌ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ⚠️ | ✅ | ❌ | ⚠️ | ❌ | ⚠️ | ❌ | ⚠️ | ❌ |
| Entity extraction / E-E-A-T signals | ✅ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ❌ | ⚠️ | ❌ | ⚠️ | ❌ | ⚠️ | ⚠️ | ✅ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ❌ |
| Internal link graph + orphan pages | ✅ (graph + cross-page analyzers) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Health/opportunity scoring | ✅ (OverallHealthScore) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |

### 2.3 Data & integrations

| Capability | crawlkit | SF | Ahrefs | Semrush | Sitebulb | Lumar | Botify | OnCrawl | seoClarity | ContentKing | Conductor | JetOctopus | Ryte | SE Rank | Moz | Serpstat | WebCEO | SEO PS | Netpeak | Siteimp | SEORadar |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Log file analysis | ✅ (nginx/Apache/JSON) | ✅ | ❌ | ⚠️ (tier) | ❌ | ⚠️ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Google Search Console integration | ✅ (conditional) | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ❌ |
| GA4 / analytics integration | ❌ | ⚠️ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ | ✅ | ❌ |
| Keyword rank tracking | ✅ (rank cmd) | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ | ❌ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ❌ |
| Backlink data (own index) | ⚠️ (crawl-derived + sources) | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ | ❌ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ⚠️ | ❌ |
| Bi-directional API / data warehouse | ✅ (REST + SQLite/PG export) | ⚠️ (CLI only) | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ⚠️ | ❌ | ✅ | ❌ |
| BI/webhook push (Slack, Teams, etc.) | ⚠️ (webhooks; no managed connectors) | ❌ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Scheduled crawls | ⚠️ (API ScheduleConfig) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | n/a | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | n/a |

### 2.4 Platform & operations

| Capability | crawlkit | SF | Ahrefs | Semrush | Sitebulb | Lumar | Botify | OnCrawl | seoClarity | ContentKing | Conductor | JetOctopus | Ryte | SE Rank | Moz | Serpstat | WebCEO | SEO PS | Netpeak | Siteimp | SEORadar |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Self-hosted / on-prem | ✅ (binary, Docker) | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ |
| Open source | ✅ Apache-2.0 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| CLI / headless operation | ✅ (11 commands) | ✅ (headless) | ❌ | ❌ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ |
| REST API | ✅ (axum, OpenAPI) | ❌ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ |
| Client libraries (Go/Node/Python) | ⚠️ (Go 100%, Node 100%, Python 92%) | ❌ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | ⚠️ | ✅ | ⚠️ | ❌ | ❌ | ⚠️ | ❌ |
| Multi-tenancy + RBAC | ✅ (enterprise.rs, API) | ❌ | n/a | n/a | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | n/a | ✅ | ❌ | ❌ | ✅ | ❌ |
| SSO / OIDC | ✅ (oidc.rs) | ❌ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ❌ | ⚠️ | ⚠️ | ❌ | ❌ | ✅ | ❌ |
| Encryption at rest (optional) | ✅ (feature) | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| Audit trail / access logs | ✅ (audit.rs, access_log.rs) | ❌ | n/a | n/a | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ❌ | n/a | ⚠️ | ❌ | ❌ | ✅ | ❌ |
| Extensibility: plugin marketplace | ✅ (signed WASM + WASI) | ⚠️ (custom JS snippets) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Observability (Prometheus + OTel) | ✅ (conditional OTel) | ❌ | n/a | n/a | ❌ | ⚠️ | ✅ | ⚠️ | ⚠️ | n/a | n/a | ⚠️ | ⚠️ | ⚠️ | ❌ | n/a | ⚠️ | ❌ | ❌ | ⚠️ | ❌ |
| Real-time change monitoring | ⚠️ (stable via `--monitor`; crawl-diff based, not continuous polling) | ❌ | ✅ | ✅ | ❌ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ❌ | ❌ | ✅ | ✅ |
| Historical trend comparison | ✅ (compare + trend + regression) | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Visual crawl map | ✅ (SVG force-directed) | ⚠️ (limited) | ❌ | ❌ | ✅ (rich) | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ❌ |
| Dashboard UI | ⚠️ (React dashboard, JWT/RBAC) | ✅ (GUI) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| LLM-assisted analysis (BYO key) | ✅ (--llm) | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ✅ | ⚠️ | ⚠️ | ✅ | ⚠️ | ⚠️ | ⚠️ | ❌ | ❌ | ⚠️ | ❌ |
| Reproducible/deterministic crawl mode | ✅ (--seed, deterministic export) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

---

## 3. Feature discrepancies (crawlkit vs. market leaders)

### 3.1 Where crawlkit is behind (closed first for market-leading standard)

| ID | Gap | Market standard set by | crawlkit today | Impact |
|----|-----|------------------------|----------------|--------|
| F1 | Managed GA4/GSC connectors in the UI | Lumar/Botify/SE Ranking: connect-in-UI data sources | GSC via CLI + access token only (conditional); GA4 absent | High — data-fusion reporting is the #1 enterprise buying criterion |
| F2 | Managed alert connectors (Slack/Teams/email/Jira) | ContentKing/Conductor/Lumar | Generic webhook only | High — alerts that don't arrive where teams work are ignored |
| F3 | Scheduling UX (cron in CLI/UI, not API-only) | Every cloud tool | `ScheduleConfig` exists via API; no CLI/UI surface | Medium — API-only scheduling is undiscoverable |
| F4 | CrUX field data promoted from optional | Lumar/SE Ranking/Siteimprove | `crux.rs` behind feature flag | Medium — lab-only CWV is now table stakes |
| F5 | Rendering parity with commercial crawlers | SF/Sitebulb/Lumar: headless Chrome fleet, rendering queues, per-page render budgets | Playwright-based, optional runtime | Medium — fine for CLI, weak for enterprise-scale JS sites |
| F6 | Historical dashboards (time-series UI) | All cloud tools | `trend` command + compare; no UI charts over time | Medium |
| F7 | Report builder (white-label PDF/scheduled email) | WebCEO/Siteimprove/SE Ranking | HTML/MD report only | Medium — agencies are the largest buyer segment |
| F8 | Issue triage workflow (assign/ignore/resolve/verify) | Siteimprove/Moz/Semrush | Findings list only | Medium — "fix workflow" differentiates leaders |
| F9 | Free-tier marketing surface (hosted scanner) | Every SaaS has a free URL scan | None (self-host only) | Low-Medium — top-of-funnel gap |
| F10 | Localization of reports/i18n content checks depth | Dragon Metrics-class tools | International analyzers exist; no localized output | Low |

### 3.2 Where crawlkit is ahead (defend and market these)

| ID | Advantage | Nobody else in the 20 has |
|----|-----------|---------------------------|
| A1 | Signed WASM plugin marketplace with content-addressed verification and WASI component-model support | None of the 20 offers any plugin ecosystem |
| A2 | Deterministic crawl mode (`--seed`) → reproducible audits for CI | None |
| A3 | Full self-host OSS (Apache-2.0) with client libraries in 3 languages | None (SEORadar self-host ≠ OSS) |
| A4 | Security-header suite (20 analyzers incl. SRI/COEP/CT) inside an SEO crawler | SF/Sitebulb/Ahrefs: none |
| A5 | AI-crawler accessibility + citation eligibility analyzers (GEO) built-in | Only partial at Ahrefs/Semrush |
| A6 | 58 schema.org-type validators (deepest structured-data coverage) | None at this depth |
| A7 | Cryptographic crawl evidence: encryption at rest, audit trail, signed releases, SBOM, OSS-Fuzz kit | Unique in segment |
| A8 | Analyzer profiles (core/standard/deep/full) and per-check finding codes (778) | Not exposed this way anywhere |

---

## 4. Architecture discrepancies

| ID | Dimension | crawlkit | Commercial leaders | Discrepancy & implication |
|----|-----------|----------|--------------------|--------------------------|
| AR1 | Execution model | Single Rust binary; tokio semaphore-bounded fetch loop; in-process analyzers | Cloud vendors: queued multi-tenant crawl farms with render pools and per-tenant rate quotas | crawlkit scales vertically + optional distributed mode; no managed horizontal autoscaling story. Fine for OSS; blocks competing for "450 URLs/s" enterprise deals until distributed mode exits experimental |
| AR2 | Storage | SQLite default; PostgreSQL conditional behind feature + service tests | Cloud: columnar/Warehouse storage (BigQuery/Snowflake/Redshift class) with retention policies | No warehouse export connector (BigQuery/Snowflake/S3+Iceberg). Enterprise data-fusion workflows expect it |
| AR3 | Rendering | Playwright subprocess, optional | Vendor render fleets with caching, queue prioritization, JS-error capture | Functional parity locally; no render budget management at fleet scale |
| AR4 | Queue | In-memory + experimental Redis | Managed durable queues with exactly/at-least-once semantics | Redis path must graduate (Phase 2.3 of ROADMAP) before distributed claims are defensible |
| AR5 | Extension points | WASM plugins (fuel/memory/capability sandbox), native experimental | Vendors: closed; SF offers custom JS/DB access; APIs elsewhere | Structural advantage; no competitor has a sandboxed extension model |
| AR6 | UI vs API parity | API-first, dashboard secondary; some features CLI-only (F3) | UI-first SaaS with API parity | crawlkit's monitor/schedule/GSC lack UI; buyers demo UIs first |
| AR7 | Determinism/testability | Deterministic mode, mutation-tested analyzers (87.7% mutant kill), corpus fixtures | Not published by any vendor | Unique engineering evidence; should be surfaced as a trust differentiator |
| AR8 | Multi-tenancy | Tenant ID across API/engine, RBAC, quotas in API layer | Deep per-tenant billing, usage metering, self-serve onboarding | No usage metering/billing hooks (docs/BILLING.md is aspirational) |
| AR9 | Data retention/lifecycle | Retention purge policy exists (soc2_features) | Vendors document retention SLAs | Retention must be configurable per tenant and surfaced in UI/API |
| AR10 | Observability | Prometheus + conditional OTel with redaction | Managed dashboards | Parity for self-host; no hosted observability story |

---

## 5. Counting caveat (per claims policy)

- Analyzer count differs by source: README says **31 analyzers**, `docs/ANALYZER_CATALOG.md` says **200 (181 single-page + 19 cross-page)** with a footnote stating the canonical test-asserted count is 181 + 19 = 200 and that the registry registers 180 `Box::new` entries including 4 duplicates. `capabilities.toml` deliberately reports no numeric count. **Action: generate README/catalog numbers from the registry in CI (ROADMAP Phase 0.1) — this matrix uses "100+ on-page checks / 58 schema-type validators" conservatively.**
- "Checks" reported by competitors (~100–300+) are vendor-claimed counts of issue types, not verified. They are listed only to size the market's expectation.

---

## 6. Market-leading gap-closure plan

Ordered by (buying-criterion weight × effort). Each item carries the ROADMAP gate it must pass.

### P0 — close before claiming market-leading parity
1. **F1/F2 — Managed data & alert connectors.** Ship GSC (stable), GA4, and Slack/Teams/email alert channels as first-party modules with per-tenant credentials, secret storage, and contract tests against sandbox tenants. *Gates: Phase 1.4 (secrets), Phase 3.4 (API contract tests).*
2. **AR1/AR4 — Graduate distributed mode.** Promote Redis queue from experimental: documented delivery semantics, lease/retry/poison handling, crash-recovery tests; publish capacity results (10k/100k URL crawls). *Gates: Phase 2.3, 2.4, 4.3.*
3. **F3/F8 — Scheduling + triage surfaces.** CLI subcommands (`crawlkit schedule add/list/remove`), dashboard pages for schedules and issue triage (assign/ignore/resolve with verification crawl). *Gates: Phase 3.4, documentation gate.*
4. **Counting fix (§5).** Generate README/analyzer numbers from the registry manifest in CI. *Gate: Phase 0.1 — hard exit criterion for any marketing claim.*

### P1 — differentiators to lead the market
5. **F4 — CrUX field data stable + surfaced in reports** (builds on existing `crux.rs`).
6. **F7 — Report builder**: templated, white-label PDF/scheduled email export; reusable report templates per tenant.
7. **AR2 — Warehouse export**: BigQuery, Snowflake, and S3 (Parquet/Iceberg) exporters alongside SQLite/PG; schema-documented, versioned.
8. **F5 — Rendering at scale**: render-budget config (per-crawl render quota, per-page budget), JS-error capture as findings, rendering telemetry (metrics contract Phase 5.1).
9. **A1 — Marketplace growth**: second-party signed plugins, plugin categories, dependency-free install, provenance attestations for plugin artifacts (extends existing trust chain).
10. **F6 — Trends UI**: time-series charts in the dashboard over stored crawl snapshots (data already exists via `trend`).

### P2 — market coverage
11. **F9 — Hosted free scan page** (single-URL public scan with rate limits and no auth; feeds signups for the self-hosted product).
12. **F10 — i18n reports** (localized report templates; UA/EU markets).
13. **AR8 — Usage metering + billing hooks** (per-tenant crawl/analysis quotas surfaced in API + dashboard; replaces aspirational BILLING.md).
14. **AR9 — Per-tenant retention config UI/API.**

### Explicit non-goals (retain from ROADMAP §1)
No SIEM replacement, no link-index building (crawlkit has no web-wide index — backlink features remain source-aggregated), no formal-verification claims, no HFT/enterprise certification language.

---

## 7. Competitor source register

URLs accessed 2026-09-10 for competitor capability cells. Absence of a feature at a URL is a documentation observation, not a verified product absence.

1. Screaming Frog: https://www.screamingfrog.co.uk/seo-spider/ (+ /pricing/, /seo-spider/tutorials/web-scraping/, log file analyser docs)
2. Ahrefs Site Audit: https://ahrefs.com/site-audit (+ https://help.ahrefs.com/en/articles/9082329)
3. Semrush Site Audit: https://www.semrush.com/siteaudit/ (+ https://www.semrush.com/kb/31-site-audit)
4. Sitebulb: https://sitebulb.com/ (+ /features/, /subscriptions/pricing/)
5. Lumar: https://www.lumar.io/ (+ /product-guides/how-to-crawl/javascript-rendering/, crawler-speed announcement)
6. Botify: https://www.botify.com/ (+ JS crawling and platform guides)
7. OnCrawl: https://www.oncrawl.com/ (+ /platform/scale-your-seo-impact/, /platform/integrations/)
8. seoClarity: https://www.seoclarity.net/technology/site-audits/ (+ /log-file-analyzer/, Content Guard announcement)
9. ContentKing: https://www.contentkingapp.com/ (via Conductor Monitoring pages)
10. Conductor: https://www.conductor.com/platform/monitoring/
11. JetOctopus: https://jetoctopus.com/ (+ /pricing/)
12. Ryte: https://en.ryte.com/platform/quality-assurance/
13. SE Ranking: https://seranking.com/website-audit.html
14. Moz Pro: https://moz.com/products/pro/site-crawl (+ Help Hub site crawl overview)
15. Serpstat: https://serpstat.com/page/pricing-plans/ (+ audit API docs)
16. WebCEO: https://www.webceo.com/online-seo-tools.htm (+ pricing page)
17. SEO PowerSuite: https://www.link-assistant.com/website-auditor/ (+ pricing)
18. Netpeak Spider: https://netpeaksoftware.com/spider (+ custom-extraction blog post)
19. Siteimprove: https://www.siteimprove.com/ (+ enterprise SEO pages)
20. SEORadar: https://seoradar.com/ (via curated tool directories; product page thin — cells marked accordingly)

---

*This document follows the repository claims policy (ROADMAP §0.3): every competitor cell is a documentation observation with a listed source; every crawlkit cell cites repo evidence; no performance claims are made against competitors; counts that cannot be generated from code are labelled vendor-claimed.*
