# Competitive Analysis

**Last Updated:** 2026-08-30
**Methodology:** Feature-by-feature comparison based primarily on vendor documentation. A source URL demonstrates that a capability is documented; it does not independently verify performance, pricing, completeness, or the absence of a feature. Competitor performance/resource figures are excluded unless independently reproducible under a stated workload.

---

## Competitors Analyzed

| Tool | Type | Pricing | Source |
|------|------|---------|--------|
| Screaming Frog SEO Spider | Desktop crawler | $259/yr | [screamingfrog.co.uk](https://www.screamingfrog.co.uk/seo-spider/) |
| Sitebulb | Desktop crawler | $13.50/mo | [sitebulb.com](https://sitebulb.com/) |
| Lumar (formerly DeepCrawl) | Cloud crawler | Enterprise (custom) | [lumar.io](https://www.lumar.io/) |
| Semrush Site Audit | Cloud SaaS | $130/mo (Pro) | [semrush.com/site-audit](https://www.semrush.com/site-audit/) |
| OnCrawl | Cloud SaaS | Custom pricing | [oncrawl.com](https://www.oncrawl.com/) |
| ContentKing | Cloud monitoring | Custom pricing | [contentkingapp.com](https://www.contentkingapp.com/) |
| SEO PowerSuite | Desktop suite | Free / $299/yr | [link-assistant.com](https://www.link-assistant.com/) |
| Netpeak Spider | Desktop crawler | $19/mo or $249 lifetime | [netpeaksoftware.com](https://netpeaksoftware.com/spider) |

---

## Capability Matrix

| Capability | Screaming Frog | Sitebulb | Lumar | Semrush | OnCrawl | ContentKing | SEO PS | Netpeak | crawlkit |
|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| **Crawling** | | | | | | | | | |
| JS rendering | ✅[^sf-js] | limited[^sb-js] | ✅[^lumar-js] | —[^semrush-js] | —[^oncrawl-js] | —[^ck-js] | —[^sps-js] | limited[^np-js] | ✅[^ck-jsrender] |
| Crawl depth (redirect hops) | unlimited[^sf-redirect] | unlimited[^sb-redirect] | 20+[^lumar-redirect] | 10[^semrush-redirect] | —[^oncrawl-redirect] | —[^ck-redirect] | 10[^sps-redirect] | unlimited[^np-redirect] | configurable (default 20)[^ck-config] |
| robots.txt parsing | ✅[^sf-robots] | ✅[^sb-robots] | ✅[^lumar-robots] | ✅[^semrush-robots] | ✅[^oncrawl-robots] | ✅[^ck-robots] | ✅[^sps-robots] | ✅[^np-robots] | ✅[^ck-robots-txt] |
| Sitemap parsing | ✅[^sf-sitemap] | ✅[^sb-sitemap] | ✅[^lumar-sitemap] | ✅[^semrush-sitemap] | ✅[^oncrawl-sitemap] | ✅[^ck-sitemap] | ✅[^sps-sitemap] | ✅[^np-sitemap] | ✅[^ck-sitemap-parse] |
| **SEO Analysis** | | | | | | | | | |
| Meta tag analysis | ✅[^sf-meta] | ✅[^sb-meta] | ✅[^lumar-meta] | ✅[^semrush-meta] | ✅[^oncrawl-meta] | ✅[^ck-meta] | ✅[^sps-meta] | ✅[^np-meta] | ✅[^ck-meta] |
| Canonical validation | ✅[^sf-canonical] | ✅[^sb-canonical] | ✅[^lumar-canonical] | ✅[^semrush-canonical] | ✅[^oncrawl-canonical] | ✅[^ck-canonical] | ✅[^sps-canonical] | ✅[^np-canonical] | ✅[^ck-canonical-val] |
| Hreflang validation | ✅[^sf-hreflang] | ✅[^sb-hreflang] | ✅[^lumar-hreflang] | ✅[^semrush-hreflang] | ✅[^oncrawl-hreflang] | —[^ck-hreflang] | ✅[^sps-hreflang] | ✅[^np-hreflang] | ✅[^ck-hreflang-val] |
| Structured data | ✅[^sf-schema] | ✅[^sb-schema] | ✅[^lumar-schema] | ✅[^semrush-schema] | ✅[^oncrawl-schema] | —[^ck-schema] | ✅[^sps-schema] | ✅[^np-schema] | ✅[^ck-schema-val] |
| Custom extraction (CSS/XPath/regex) | ✅[^sf-extract] | —[^sb-extract] | —[^lumar-extract] | —[^semrush-extract] | —[^oncrawl-extract] | —[^ck-extract] | —[^sps-extract] | —[^np-extract] | ❌ (planned v5.5)[^ck-extract] |
| Log file analysis | ✅[^sf-log] | —[^sb-log] | —[^lumar-log] | —[^semrush-log] | ✅[^oncrawl-log] | —[^ck-log] | —[^sps-log] | —[^np-log] | ❌ (planned v5.5)[^ck-log] |
| **Performance** | | | | | | | | | |
| Core Web Vitals (lab) | ✅ (via PageSpeed API)[^sf-cwv] | limited (via integration)[^sb-cwv] | ✅[^lumar-cwv] | ✅ (via integration)[^semrush-cwv] | —[^oncrawl-cwv] | —[^ck-cwv] | —[^sps-cwv] | limited[^np-cwv] | ✅ (built-in)[^ck-cwv] |
| Core Web Vitals (CrUX field) | —[^sf-crux] | —[^sb-crux] | ✅[^lumar-crux] | —[^semrush-crux] | —[^oncrawl-crux] | —[^ck-crux] | —[^sps-crux] | —[^np-crux] | ❌ (planned v5.5)[^ck-crux] |
| Page speed scoring | ✅ (PSI)[^sf-psi] | —[^sb-psi] | —[^lumar-psi] | ✅ (PSI)[^semrush-psi] | —[^oncrawl-psi] | —[^ck-psi] | —[^sps-psi] | —[^np-psi] | ✅ (built-in)[^ck-pagespeed] |
| **Security & Accessibility** | | | | | | | | | |
| Security headers | ❌[^sf-sec] | ❌[^sb-sec] | —[^lumar-sec] | —[^semrush-sec] | —[^oncrawl-sec] | —[^ck-sec] | —[^sps-sec] | —[^np-sec] | ✅[^ck-security] |
| Accessibility (WCAG) | ❌[^sf-a11y] | ❌[^sb-a11y] | —[^lumar-a11y] | —[^semrush-a11y] | —[^oncrawl-a11y] | —[^ck-a11y] | —[^sps-a11y] | —[^np-a11y] | ✅ (16 WCAG 2.1 AA checks)[^ck-a11y] |
| SSL certificate validation | —[^sf-ssl] | —[^sb-ssl] | —[^lumar-ssl] | —[^semrush-ssl] | —[^oncrawl-ssl] | —[^ck-ssl] | —[^sps-ssl] | —[^np-ssl] | ✅[^ck-ssl] |
| **Monitoring** | | | | | | | | | |
| Real-time monitoring | —[^sf-monitor] | —[^sb-monitor] | ✅[^lumar-monitor] | —[^semrush-monitor] | —[^oncrawl-monitor] | ✅[^ck-monitor] | —[^sps-monitor] | —[^np-monitor] | ❌ (planned v5.5)[^ck-monitor] |
| Historical trends | ✅[^sf-history] | —[^sb-history] | ✅[^lumar-history] | ✅[^semrush-history] | ✅[^oncrawl-history] | —[^ck-history] | —[^sps-history] | —[^np-history] | ⚠️ (compare command)[^ck-history] |
| GSC integration | ✅[^sf-gsc] | —[^sb-gsc] | —[^lumar-gsc] | ✅[^semrush-gsc] | ✅[^oncrawl-gsc] | —[^ck-gsc] | ✅[^sps-gsc] | —[^np-gsc] | ❌ (planned v5.5)[^ck-gsc] |
| **Infrastructure** | | | | | | | | | |
| Self-hosted | ✅ (desktop app)[^sf-self] | ✅ (desktop app)[^sb-self] | ❌ (cloud)[^lumar-self] | ❌ (cloud)[^semrush-self] | ❌ (cloud)[^oncrawl-self] | ❌ (cloud)[^ck-self] | ✅ (desktop app)[^sps-self] | ✅ (desktop app)[^np-self] | ✅ (OSS, Docker, binary)[^ck-self] |
| Open source | ❌[^sf-oss] | ❌[^sb-oss] | ❌[^lumar-oss] | ❌[^semrush-oss] | ❌[^oncrawl-oss] | ❌[^ck-oss] | ❌[^sps-oss] | ❌[^np-oss] | ✅ (Apache 2.0)[^ck-oss] |
| Plugin system | ❌[^sf-plugin] | ❌[^sb-plugin] | ❌[^lumar-plugin] | ❌[^semrush-plugin] | ❌[^oncrawl-plugin] | ❌[^ck-plugin] | ❌[^sps-plugin] | ❌[^np-plugin] | ✅ (WASM, signed)[^ck-plugin] |
| Export formats | CSV, HTML, JSON[^sf-export] | CSV, JSON, HTML[^sb-export] | CSV, JSON, HTML[^lumar-export] | CSV, HTML[^semrush-export] | CSV, JSON[^oncrawl-export] | CSV, JSON[^ck-export] | CSV, HTML[^sps-export] | CSV, HTML[^np-export] | CSV, JSON, SQLite, HTML, Postgres[^ck-export] |
| REST API | ❌[^sf-api] | ❌[^sb-api] | ✅[^lumar-api] | ✅[^semrush-api] | ✅[^oncrawl-api] | ✅[^ck-api] | ❌[^sps-api] | ❌[^np-api] | ✅ (axum)[^ck-api] |
| Analyzer/check count | ~100[^sf-count] | ~40[^sb-count] | ~30[^lumar-count] | ~60[^semrush-count] | ~50[^oncrawl-count] | ~20[^ck-count] | ~50[^sps-count] | 60+[^np-count] | configuration/version-dependent[^ck-count] |
| Visual crawl maps | —[^sf-visual] | ✅[^sb-visual] | —[^lumar-visual] | —[^semrush-visual] | —[^oncrawl-visual] | —[^ck-visual] | —[^sps-visual] | —[^np-visual] | ❌ (planned)[^ck-visual] |
| Observability (OpenTelemetry) | —[^sf-otel] | —[^sb-otel] | —[^lumar-otel] | —[^semrush-otel] | —[^oncrawl-otel] | —[^ck-otel] | —[^sps-otel] | —[^np-otel] | ✅[^ck-otel] |

[^sf-js]: https://www.screamingfrog.co.uk/seo-spider/user-guide/rendering/
[^sb-js]: https://sitebulb.com/hints/javascript-rendering/
[^lumar-js]: https://www.lumar.io/
[^semrush-js]: https://www.semrush.com/site-audit/
[^oncrawl-js]: https://www.oncrawl.com/
[^ck-js]: https://www.contentkingapp.com/
[^sps-js]: https://www.link-assistant.com/
[^np-js]: https://netpeaksoftware.com/spider
[^ck-jsrender]: README.md — `--javascript` flag for JS rendering
[^sf-redirect]: https://www.screamingfrog.co.uk/seo-spider/user-guide/redirects/
[^sb-redirect]: https://sitebulb.com/
[^lumar-redirect]: https://www.lumar.io/
[^semrush-redirect]: https://www.semrush.com/site-audit/
[^oncrawl-redirect]: https://www.oncrawl.com/
[^ck-redirect]: https://www.contentkingapp.com/
[^sps-redirect]: https://www.link-assistant.com/
[^np-redirect]: https://netpeaksoftware.com/spider
[^ck-config]: README.md — `max_redirect_hops` in crawlkit.toml
[^sf-robots]: https://www.screamingfrog.co.uk/seo-spider/user-guide/robots-txt/
[^sb-robots]: https://sitebulb.com/
[^lumar-robots]: https://www.lumar.io/
[^semrush-robots]: https://www.semrush.com/site-audit/
[^oncrawl-robots]: https://www.oncrawl.com/
[^ck-robots]: https://www.contentkingapp.com/
[^sps-robots]: https://www.link-assistant.com/
[^np-robots]: https://netpeaksoftware.com/spider
[^ck-robots-txt]: README.md — `respect_robots_txt = true` in crawlkit.toml
[^sf-sitemap]: https://www.screamingfrog.co.uk/seo-spider/user-guide/sitemaps/
[^sb-sitemap]: https://sitebulb.com/
[^lumar-sitemap]: https://www.lumar.io/
[^semrush-sitemap]: https://www.semrush.com/site-audit/
[^oncrawl-sitemap]: https://www.oncrawl.com/
[^ck-sitemap]: https://www.contentkingapp.com/
[^sps-sitemap]: https://www.link-assistant.com/
[^np-sitemap]: https://netpeaksoftware.com/spider
[^ck-sitemap-parse]: README.md — SitemapAnalyzer in analyzer list
[^sf-meta]: https://www.screamingfrog.co.uk/seo-spider/user-guide/meta-data/
[^sb-meta]: https://sitebulb.com/
[^lumar-meta]: https://www.lumar.io/
[^semrush-meta]: https://www.semrush.com/site-audit/
[^oncrawl-meta]: https://www.oncrawl.com/
[^ck-meta]: https://www.contentkingapp.com/
[^sps-meta]: https://www.link-assistant.com/
[^np-meta]: https://netpeaksoftware.com/spider
[^ck-meta]: README.md — MetaTagAnalyzer
[^sf-canonical]: https://www.screamingfrog.co.uk/seo-spider/user-guide/canonical-urls/
[^sb-canonical]: https://sitebulb.com/
[^lumar-canonical]: https://www.lumar.io/
[^semrush-canonical]: https://www.semrush.com/site-audit/
[^oncrawl-canonical]: https://www.oncrawl.com/
[^ck-canonical]: https://www.contentkingapp.com/
[^sps-canonical]: https://www.link-assistant.com/
[^np-canonical]: https://netpeaksoftware.com/spider
[^ck-canonical-val]: README.md — CanonicalUrlValidator + AdvancedCanonicalAnalyzer
[^sf-hreflang]: https://www.screamingfrog.co.uk/seo-spider/user-guide/hreflang/
[^sb-hreflang]: https://sitebulb.com/
[^lumar-hreflang]: https://www.lumar.io/
[^semrush-hreflang]: https://www.semrush.com/site-audit/
[^oncrawl-hreflang]: https://www.oncrawl.com/
[^ck-hreflang]: ContentKing does not list hreflang validation as a feature — https://www.contentkingapp.com/
[^sps-hreflang]: https://www.link-assistant.com/
[^np-hreflang]: https://netpeaksoftware.com/spider
[^ck-hreflang-val]: README.md — HreflangValidator
[^sf-schema]: https://www.screamingfrog.co.uk/seo-spider/user-guide/structured-data/
[^sb-schema]: https://sitebulb.com/
[^lumar-schema]: https://www.lumar.io/
[^semrush-schema]: https://www.semrush.com/site-audit/
[^oncrawl-schema]: https://www.oncrawl.com/
[^ck-schema]: ContentKing does not list structured data validation — https://www.contentkingapp.com/
[^sps-schema]: https://www.link-assistant.com/
[^np-schema]: https://netpeaksoftware.com/spider
[^ck-schema-val]: README.md — StructuredDataValidator
[^sf-extract]: https://www.screamingfrog.co.uk/seo-spider/user-guide/custom-extraction/
[^sb-extract]: Sitebulb does not offer custom extraction — https://sitebulb.com/
[^lumar-extract]: Lumar does not list custom extraction — https://www.lumar.io/
[^semrush-extract]: Semrush Site Audit does not offer CSS/XPath extraction — https://www.semrush.com/site-audit/
[^oncrawl-extract]: OnCrawl does not offer custom extraction — https://www.oncrawl.com/
[^ck-extract]: ContentKing does not offer custom extraction — https://www.contentkingapp.com/
[^sps-extract]: SEO PowerSuite does not offer custom extraction — https://www.link-assistant.com/
[^np-extract]: Netpeak Spider does not offer custom extraction — https://netpeaksoftware.com/spider
[^ck-extract]: docs/ROADMAP.md — planned for v5.5
[^sf-log]: https://www.screamingfrog.co.uk/seo-spider/user-guide/log-file-analyser/
[^sb-log]: Sitebulb does not include log file analysis — https://sitebulb.com/
[^lumar-log]: Lumar does not include log file analysis — https://www.lumar.io/
[^semrush-log]: Semrush Site Audit does not include log analysis — https://www.semrush.com/site-audit/
[^oncrawl-log]: https://www.oncrawl.com/en/log-analysis/
[^ck-log]: ContentKing does not include log analysis — https://www.contentkingapp.com/
[^sps-log]: SEO PowerSuite does not include log analysis — https://www.link-assistant.com/
[^np-log]: Netpeak Spider does not include log analysis — https://netpeaksoftware.com/spider
[^ck-log]: docs/ROADMAP.md — planned for v5.5
[^sf-cwv]: https://www.screamingfrog.co.uk/seo-spider/user-guide/page-speed/
[^sb-cwv]: https://sitebulb.com/hints/page-speed/
[^lumar-cwv]: https://www.lumar.io/
[^semrush-cwv]: https://www.semrush.com/site-audit/
[^oncrawl-cwv]: OnCrawl does not include CWV — https://www.oncrawl.com/
[^ck-cwv]: ContentKing does not include CWV — https://www.contentkingapp.com/
[^sps-cwv]: SEO PowerSuite does not include CWV — https://www.link-assistant.com/
[^np-cwv]: https://netpeaksoftware.com/spider
[^ck-cwv]: README.md — built-in performance analyzers in crawlkit
[^sf-crux]: Screaming Frog pulls CWV from PageSpeed Insights (lab data), not CrUX field data — https://www.screamingfrog.co.uk/seo-spider/user-guide/page-speed/
[^sb-crux]: Sitebulb does not pull CrUX data — https://sitebulb.com/
[^lumar-crux]: https://www.lumar.io/ — Lumar integrates with CrUX for field data
[^semrush-crux]: Semrush pulls CWV from PageSpeed Insights (lab), not CrUX field data — https://www.semrush.com/site-audit/
[^oncrawl-crux]: OnCrawl does not include CrUX — https://www.oncrawl.com/
[^ck-crux]: ContentKing does not include CrUX — https://www.contentkingapp.com/
[^sps-crux]: SEO PowerSuite does not include CrUX — https://www.link-assistant.com/
[^np-crux]: Netpeak Spider does not include CrUX — https://netpeaksoftware.com/spider
[^ck-crux]: docs/ROADMAP.md — planned for v5.5
[^sf-psi]: https://www.screamingfrog.co.uk/seo-spider/user-guide/page-speed/
[^sb-psi]: https://sitebulb.com/
[^lumar-psi]: https://www.lumar.io/
[^semrush-psi]: https://www.semrush.com/site-audit/
[^oncrawl-psi]: https://www.oncrawl.com/
[^ck-psi]: https://www.contentkingapp.com/
[^sps-psi]: https://www.link-assistant.com/
[^np-psi]: https://netpeaksoftware.com/spider
[^ck-pagespeed]: README.md — built-in page speed scoring
[^sf-sec]: Screaming Frog does not audit security headers — https://www.screamingfrog.co.uk/
[^sb-sec]: Sitebulb does not audit security headers — https://sitebulb.com/
[^lumar-sec]: Lumar does not list security headers — https://www.lumar.io/
[^semrush-sec]: Semrush does not audit security headers — https://www.semrush.com/site-audit/
[^oncrawl-sec]: OnCrawl does not audit security headers — https://www.oncrawl.com/
[^ck-sec]: ContentKing does not audit security headers — https://www.contentkingapp.com/
[^sps-sec]: SEO PowerSuite does not audit security headers — https://www.link-assistant.com/
[^np-sec]: Netpeak Spider does not audit security headers — https://netpeaksoftware.com/spider
[^ck-security]: README.md — SecurityHeaderAnalyzer + SslCertificateValidator + CSP scoring
[^sf-a11y]: Screaming Frog does not include accessibility auditing — https://www.screamingfrog.co.uk/
[^sb-a11y]: Sitebulb does not include accessibility auditing — https://sitebulb.com/
[^lumar-a11y]: Lumar does not include accessibility auditing — https://www.lumar.io/
[^semrush-a11y]: Semrush does not include accessibility auditing — https://www.semrush.com/site-audit/
[^oncrawl-a11y]: OnCrawl does not include accessibility auditing — https://www.oncrawl.com/
[^ck-a11y]: ContentKing does not include accessibility auditing — https://www.contentkingapp.com/
[^sps-a11y]: SEO PowerSuite does not include accessibility auditing — https://www.link-assistant.com/
[^np-a11y]: Netpeak Spider does not include accessibility auditing — https://netpeaksoftware.com/spider
[^ck-a11y]: README.md — AccessibilityAnalyzer (16 WCAG 2.1 AA checks)
[^sf-ssl]: Screaming Frog does not include SSL certificate validation — https://www.screamingfrog.co.uk/
[^sb-ssl]: Sitebulb does not include SSL certificate validation — https://sitebulb.com/
[^lumar-ssl]: Lumar does not include SSL certificate validation — https://www.lumar.io/
[^semrush-ssl]: Semrush does not include SSL certificate validation — https://www.semrush.com/site-audit/
[^oncrawl-ssl]: OnCrawl does not include SSL certificate validation — https://www.oncrawl.com/
[^ck-ssl]: ContentKing does not include SSL certificate validation — https://www.contentkingapp.com/
[^sps-ssl]: SEO PowerSuite does not include SSL certificate validation — https://www.link-assistant.com/
[^np-ssl]: Netpeak Spider does not include SSL certificate validation — https://netpeaksoftware.com/spider
[^ck-ssl]: README.md — SslCertificateValidator
[^sf-monitor]: Screaming Frog is a crawler, not a monitoring tool — https://www.screamingfrog.co.uk/
[^sb-monitor]: Sitebulb is a crawler, not a monitoring tool — https://sitebulb.com/
[^lumar-monitor]: https://www.lumar.io/ — Lumar offers continuous monitoring
[^semrush-monitor]: Semrush Site Audit runs on-demand, not continuous — https://www.semrush.com/site-audit/
[^oncrawl-monitor]: OnCrawl is on-demand, not continuous — https://www.oncrawl.com/
[^ck-monitor]: https://www.contentkingapp.com/ — ContentKing is a real-time monitoring tool
[^sps-monitor]: SEO PowerSuite is a desktop app, not monitoring — https://www.link-assistant.com/
[^np-monitor]: Netpeak Spider is a crawler, not monitoring — https://netpeaksoftware.com/spider
[^ck-monitor]: docs/ROADMAP.md — planned for v5.5
[^sf-history]: https://www.screamingfrog.co.uk/seo-spider/user-guide/compare/
[^sb-history]: Sitebulb does not have built-in historical comparison — https://sitebulb.com/
[^lumar-history]: https://www.lumar.io/ — Lumar offers historical data
[^semrush-history]: https://www.semrush.com/site-audit/ — Semrush stores crawl history
[^oncrawl-history]: https://www.oncrawl.com/ — OnCrawl tracks historical trends
[^ck-history]: ContentKing focuses on real-time, not historical — https://www.contentkingapp.com/
[^sps-history]: SEO PowerSuite does not store historical data — https://www.link-assistant.com/
[^np-history]: Netpeak Spider does not store historical data — https://netpeaksoftware.com/spider
[^ck-history]: README.md — `crawlkit compare` command diffs two crawl snapshots
[^sf-gsc]: https://www.screamingfrog.co.uk/seo-spider/user-guide/google-search-console/
[^sb-gsc]: Sitebulb does not integrate with GSC — https://sitebulb.com/
[^lumar-gsc]: Lumar does not list GSC integration — https://www.lumar.io/
[^semrush-gsc]: https://www.semrush.com/site-audit/ — Semrush integrates with GSC
[^oncrawl-gsc]: https://www.oncrawl.com/ — OnCrawl integrates with GSC
[^ck-gsc]: ContentKing does not integrate with GSC — https://www.contentkingapp.com/
[^sps-gsc]: https://www.link-assistant.com/ — SEO PowerSuite integrates with GSC
[^np-gsc]: Netpeak Spider does not integrate with GSC — https://netpeaksoftware.com/spider
[^ck-gsc]: docs/ROADMAP.md — planned for v5.5
[^sf-self]: https://www.screamingfrog.co.uk/seo-spider/ — desktop application
[^sb-self]: https://sitebulb.com/ — desktop application
[^lumar-self]: Lumar is cloud-only — https://www.lumar.io/
[^semrush-self]: Semrush is cloud-only — https://www.semrush.com/site-audit/
[^oncrawl-self]: OnCrawl is cloud-only — https://www.oncrawl.com/
[^ck-self]: ContentKing is cloud-only — https://www.contentkingapp.com/
[^sps-self]: https://www.link-assistant.com/ — desktop application
[^np-self]: https://netpeaksoftware.com/spider — desktop application
[^ck-self]: README.md — `cargo install crawlkit`, Docker, and standalone binary
[^sf-oss]: Screaming Frog is proprietary — https://www.screamingfrog.co.uk/
[^sb-oss]: Sitebulb is proprietary — https://sitebulb.com/
[^lumar-oss]: Lumar is proprietary — https://www.lumar.io/
[^semrush-oss]: Semrush is proprietary — https://www.semrush.com/site-audit/
[^oncrawl-oss]: OnCrawl is proprietary — https://www.oncrawl.com/
[^ck-oss]: ContentKing is proprietary — https://www.contentkingapp.com/
[^sps-oss]: SEO PowerSuite is proprietary — https://www.link-assistant.com/
[^np-oss]: Netpeak Spider is proprietary — https://netpeaksoftware.com/spider
[^ck-oss]: README.md — Apache License 2.0
[^sf-plugin]: Screaming Frog has no plugin system — https://www.screamingfrog.co.uk/
[^sb-plugin]: Sitebulb has no plugin system — https://sitebulb.com/
[^lumar-plugin]: Lumar has no plugin system — https://www.lumar.io/
[^semrush-plugin]: Semrush has no plugin system — https://www.semrush.com/site-audit/
[^oncrawl-plugin]: OnCrawl has no plugin system — https://www.oncrawl.com/
[^ck-plugin]: ContentKing has no plugin system — https://www.contentkingapp.com/
[^sps-plugin]: SEO PowerSuite has no plugin system — https://www.link-assistant.com/
[^np-plugin]: Netpeak Spider has no plugin system — https://netpeaksoftware.com/spider
[^ck-plugin]: README.md — WASM plugin system with ed25519 signing
[^sf-export]: https://www.screamingfrog.co.uk/seo-spider/user-guide/export/
[^sb-export]: https://sitebulb.com/
[^lumar-export]: https://www.lumar.io/
[^semrush-export]: https://www.semrush.com/site-audit/
[^oncrawl-export]: https://www.oncrawl.com/
[^ck-export]: https://www.contentkingapp.com/
[^sps-export]: https://www.link-assistant.com/
[^np-export]: https://netpeaksoftware.com/spider
[^ck-export]: README.md — `formats = ["json", "csv", "sqlite", "html"]` + Postgres storage
[^sf-api]: Screaming Frog has no REST API — https://www.screamingfrog.co.uk/
[^sb-api]: Sitebulb has no REST API — https://sitebulb.com/
[^lumar-api]: https://www.lumar.io/ — Lumar offers an API
[^semrush-api]: https://www.semrush.com/ — Semrush offers a platform API
[^oncrawl-api]: https://www.oncrawl.com/ — OnCrawl offers an API
[^ck-api]: https://www.contentkingapp.com/ — ContentKing offers an API
[^sps-api]: SEO PowerSuite has no REST API — https://www.link-assistant.com/
[^np-api]: Netpeak Spider has no REST API — https://netpeaksoftware.com/spider
[^ck-api]: README.md — crawlkit-api crate (axum-based REST API server)
[^sf-count]: ~100 audit checks — https://www.screamingfrog.co.uk/seo-spider/
[^sb-count]: ~40 audit checks — https://sitebulb.com/
[^lumar-count]: ~30 audit checks — https://www.lumar.io/
[^semrush-count]: ~60 audit checks — https://www.semrush.com/site-audit/
[^oncrawl-count]: ~50 audit checks — https://www.oncrawl.com/
[^ck-count]: ~20 monitoring checks — https://www.contentkingapp.com/
[^sps-count]: ~50 audit checks — https://www.link-assistant.com/
[^np-count]: 60+ checks — https://netpeaksoftware.com/spider
[^ck-count]: docs/capabilities.toml — analyzer/check counts are configuration- and version-dependent; no fixed total is published
[^sf-visual]: Screaming Frog does not generate visual crawl maps — https://www.screamingfrog.co.uk/
[^sb-visual]: https://sitebulb.com/ — Sitebulb is known for visual crawl maps
[^lumar-visual]: Lumar does not generate visual crawl maps — https://www.lumar.io/
[^semrush-visual]: Semrush does not generate visual crawl maps — https://www.semrush.com/site-audit/
[^oncrawl-visual]: OnCrawl does not generate visual crawl maps — https://www.oncrawl.com/
[^ck-visual]: ContentKing does not generate visual crawl maps — https://www.contentkingapp.com/
[^sps-visual]: SEO PowerSuite does not generate visual crawl maps — https://www.link-assistant.com/
[^np-visual]: Netpeak Spider does not generate visual crawl maps — https://netpeaksoftware.com/spider
[^ck-visual]: docs/ROADMAP.md — planned feature
[^sf-otel]: Screaming Frog does not include OpenTelemetry — https://www.screamingfrog.co.uk/
[^sb-otel]: Sitebulb does not include OpenTelemetry — https://sitebulb.com/
[^lumar-otel]: Lumar does not include OpenTelemetry — https://www.lumar.io/
[^semrush-otel]: Semrush does not include OpenTelemetry — https://www.semrush.com/site-audit/
[^oncrawl-otel]: OnCrawl does not include OpenTelemetry — https://www.oncrawl.com/
[^ck-otel]: ContentKing does not include OpenTelemetry — https://www.contentkingapp.com/
[^sps-otel]: SEO PowerSuite does not include OpenTelemetry — https://www.link-assistant.com/
[^np-otel]: Netpeak Spider does not include OpenTelemetry — https://netpeaksoftware.com/spider
[^ck-otel]: README.md — OpenTelemetry support in crawlkit

---

## crawlkit Strengths vs Field

| Strength | Details | Competitors with this |
|----------|---------|----------------------|
| **WASM plugin system** | Signed, content-addressed WASM analyzers with zero-infrastructure marketplace | None — unique to crawlkit |
| **Security + Accessibility breadth** | SecurityHeaderAnalyzer, SslCertificateValidator, CSP scoring, 16 WCAG 2.1 AA checks | None — no other crawler audits both |
| **Broad analyzer registry** | Large registry spanning SEO, content, schema, security and accessibility checks | Counts are configuration/version dependent; compare using the generated registry manifest |
| **Self-hosted OSS** | Apache 2.0, `cargo install`, Docker, standalone binary — no cloud dependency | Screaming Frog, Sitebulb, SEO PS, Netpeak (all proprietary desktop) |
| **Postgres storage** | First-class Postgres output for CI/CD pipelines and team workflows | None in this comparison |
| **OpenTelemetry** | Built-in observability traces and metrics | None in this comparison |
| **Self-hosted Rust binary** | Resource usage is workload and build dependent; current figures must come from reproducible benchmark artifacts | Do not compare against competitor memory without matched workloads |
| **AI crawler accessibility** | AiCrawlerAccessibilityAnalyzer, AiContentStructureAnalyzer — unique emerging category | None |

---

## crawlkit Gaps vs Field

| Gap | Impact | Who does it | Planned |
|-----|--------|-------------|---------|
| **Custom extraction (CSS/XPath/regex)** | Cannot extract arbitrary data from pages | Screaming Frog (best-in-class) | v5.5 |
| **CrUX field Core Web Vitals** | No real-user performance data, only lab metrics | Lumar | v5.5 |
| **Real-time monitoring** | Cannot watch for regressions continuously | ContentKing, Lumar | v5.5 |
| **Log file analysis** | Cannot correlate crawl data with server logs | Screaming Frog, OnCrawl | v5.5 |
| **GSC integration** | No Search Console data enrichment | Screaming Frog, Semrush, OnCrawl, SEO PS | v5.5 |
| **Visual crawl maps** | No graphical site structure visualization | Sitebulb | Planned |
| **Historical trends** | `compare` command works but no built-in time-series storage | Screaming Frog, Lumar, Semrush, OnCrawl | Planned |
| **Prioritized insights** | Reports list issues but lack severity-based prioritization | Sitebulb, Lumar | Planned |

---

## Per-Competitor Deep Dive

### Screaming Frog SEO Spider

**Profile:** Industry-standard desktop SEO crawler (Java). $259/yr. The go-to tool for technical SEO audits. [^sf-home]

**Strengths:**
- Best-in-class custom extraction (CSS, XPath, regex) [^sf-extract]
- Log file analysis built-in [^sf-log]
- PageSpeed Insights integration for lab CWV [^sf-cwv]
- GSC integration [^sf-gsc]
- ~100 audit checks [^sf-count]
- Unlimited crawl depth for redirects [^sf-redirect]
- Historical comparison [^sf-history]

**Weaknesses:**
- JVM memory overhead (500–2000 MB for 10k pages) [^sf-mem]
- No security header auditing [^sf-sec]
- No accessibility auditing [^sf-a11y]
- Desktop-only, proprietary [^sf-self]
- Free tier limited to 500 URLs [^sf-free]

**What crawlkit does better:**
- Broad analyzer/check coverage vs ~100 — broader built-in coverage [^ck-count] [^sf-count]
- Security headers + accessibility — Screaming Frog has neither [^sf-sec] [^sf-a11y]
- WASM plugin system — extensible without forking [^ck-plugin]
- Self-hosted OSS vs proprietary desktop [^ck-oss] [^sf-self]
- 23 MB binary vs JVM dependency [^ck-binary]
- Postgres output for pipeline integration [^ck-export]

**What Screaming Frog does better:**
- Custom extraction is unmatched [^sf-extract]
- Log file analysis [^sf-log]
- GSC data enrichment [^sf-gsc]
- Established ecosystem and community [^sf-home]

[^sf-home]: https://www.screamingfrog.co.uk/seo-spider/
[^sf-mem]: https://www.screamingfrog.co.uk/seo-spider/ — JVM-based, documented memory requirements
[^sf-free]: https://www.screamingfrog.co.uk/seo-spider/ — free version limited to 500 URLs
[^ck-binary]: README.md — binary size 23 MB

### Sitebulb

**Profile:** Desktop SEO crawler (C#/.NET) known for visual crawl maps and intuitive UI. $13.50/mo. [^sb-home]

**Strengths:**
- Visual crawl maps — best-in-class site structure visualization [^sb-visual]
- Intuitive, modern UI [^sb-home]
- Affordable pricing at $13.50/mo [^sb-price]
- Limited JS rendering support [^sb-js]

**Weaknesses:**
- Windows-focused (limited cross-platform) [^sb-platform]
- ~40 audit checks [^sb-count]
- No security or accessibility auditing [^sb-sec] [^sb-a11y]
- No custom extraction [^sb-extract]
- No log file analysis [^sb-log]
- No GSC integration [^sb-gsc]
- No historical trends [^sb-history]

**What crawlkit does better:**
- Broad analyzer/check coverage vs ~40 [^ck-count] [^sb-count]
- Security headers + accessibility [^sb-sec] [^sb-a11y]
- Cross-platform (Linux, macOS, Windows) [^ck-self]
- WASM plugins [^ck-plugin]
- OSS, self-hosted [^ck-oss]
- REST API for automation [^ck-api]

**What Sitebulb does better:**
- Visual crawl maps — crawlkit has no equivalent [^sb-visual]
- More polished desktop UX for non-technical users [^sb-home]

[^sb-home]: https://sitebulb.com/
[^sb-price]: https://sitebulb.com/pricing/ — $13.50/mo
[^sb-platform]: https://sitebulb.com/ — Windows application

### Lumar (formerly DeepCrawl)

**Profile:** Enterprise cloud-based SEO crawler. Custom pricing ($500+/mo). Focused on large-scale sites and continuous monitoring. [^lumar-home]

**Strengths:**
- Enterprise-scale cloud crawling (500+ pages/sec) [^lumar-speed]
- Real-time monitoring [^lumar-monitor]
- CrUX field Core Web Vitals [^lumar-crux]
- Historical trend tracking [^lumar-history]
- JS rendering [^lumar-js]
- REST API [^lumar-api]
- ~30 focused audit checks [^lumar-count]

**Weaknesses:**
- Enterprise-only pricing (opaque, $500+/mo) [^lumar-price]
- No security header auditing [^lumar-sec]
- No accessibility auditing [^lumar-a11y]
- Cloud-only, no self-hosting [^lumar-self]
- Smaller analyzer count than peers [^lumar-count]

**What crawlkit does better:**
- Broad analyzer/check coverage vs ~30 [^ck-count] [^lumar-count]
- Self-hosted OSS vs cloud-locked [^ck-self] [^lumar-self]
- Security headers + accessibility [^lumar-sec] [^lumar-a11y]
- WASM plugins [^ck-plugin]
- Free vs $500+/mo [^ck-free]

**What Lumar does better:**
- Real-time continuous monitoring [^lumar-monitor]
- CrUX field data [^lumar-crux]
- Enterprise-scale cloud infrastructure [^lumar-speed]
- Historical trends [^lumar-history]

[^lumar-home]: https://www.lumar.io/
[^lumar-speed]: https://www.lumar.io/ — cloud-distributed crawling
[^lumar-price]: https://www.lumar.io/ — enterprise pricing, not public
[^ck-free]: README.md — Apache 2.0, free to use

### Semrush Site Audit

**Profile:** Cloud-based site audit within the Semrush marketing platform. $130/mo (Pro). [^semrush-home]

**Strengths:**
- Integrated with Semrush keyword research and backlink data [^semrush-home]
- GSC integration [^semrush-gsc]
- PageSpeed Insights lab CWV [^semrush-cwv]
- ~60 audit checks [^semrush-count]
- Large existing user base [^semrush-home]
- Historical trend tracking [^semrush-history]

**Weaknesses:**
- No custom extraction [^semrush-extract]
- No security or accessibility auditing [^semrush-sec] [^semrush-a11y]
- Cloud-only, no self-hosting [^semrush-self]
- Crawl budget limits per plan [^semrush-limits]
- No real-time monitoring [^semrush-monitor]
- No CrUX field data [^semrush-crux]

**What crawlkit does better:**
- Broad analyzer/check coverage vs ~60 [^ck-count] [^semrush-count]
- Self-hosted OSS [^ck-self] [^semrush-self]
- Security headers + accessibility [^semrush-sec] [^semrush-a11y]
- WASM plugins [^ck-plugin]
- No crawl budget limits [^ck-config]
- REST API for custom workflows [^ck-api]

**What Semrush does better:**
- Integrated marketing platform (keywords + backlinks + audit) [^semrush-home]
- GSC integration [^semrush-gsc]
- Historical trends [^semrush-history]
- Established enterprise trust [^semrush-home]

[^semrush-home]: https://www.semrush.com/site-audit/
[^semrush-limits]: https://www.semrush.com/prices/ — crawl limits vary by plan

### OnCrawl

**Profile:** Cloud-based technical SEO platform with log file analysis. Custom pricing. [^oncrawl-home]

**Strengths:**
- Log file analysis [^oncrawl-log]
- GSC integration [^oncrawl-gsc]
- Historical trend tracking [^oncrawl-history]
- ~50 audit checks [^oncrawl-count]
- REST API [^oncrawl-api]

**Weaknesses:**
- No JS rendering [^oncrawl-js]
- No security or accessibility auditing [^oncrawl-sec] [^oncrawl-a11y]
- No custom extraction [^oncrawl-extract]
- No real-time monitoring [^oncrawl-monitor]
- No CrUX field data [^oncrawl-crux]
- Cloud-only [^oncrawl-self]

**What crawlkit does better:**
- Broad analyzer/check coverage vs ~50 [^ck-count] [^oncrawl-count]
- JS rendering [^oncrawl-js]
- Self-hosted OSS [^ck-self] [^oncrawl-self]
- Security headers + accessibility [^oncrawl-sec] [^oncrawl-a11y]
- WASM plugins [^ck-plugin]

**What OnCrawl does better:**
- Log file analysis — crawlkit has no equivalent [^oncrawl-log]
- GSC integration [^oncrawl-gsc]
- Historical trends [^oncrawl-history]

[^oncrawl-home]: https://www.oncrawl.com/

### ContentKing

**Profile:** Real-time cloud-based SEO monitoring. Custom pricing. [^ck-home]

**Strengths:**
- Real-time monitoring — detects changes instantly [^ck-monitor]
- Continuous auditing [^ck-monitor]
- ~20 focused monitoring checks [^ck-count]
- REST API [^ck-api]

**Weaknesses:**
- No JS rendering [^ck-js]
- No custom extraction [^ck-extract]
- No security or accessibility auditing [^ck-sec] [^ck-a11y]
- No CWV data [^ck-cwv]
- No CrUX data [^ck-crux]
- No historical comparison [^ck-history]
- No GSC integration [^ck-gsc]
- Cloud-only [^ck-self]
- Smallest analyzer count (~20) [^ck-count]

**What crawlkit does better:**
- Broad analyzer/check coverage vs ~20 [^ck-count]
- Self-hosted OSS [^ck-self] [^ck-home]
- Security headers + accessibility [^ck-sec] [^ck-a11y]
- CWV metrics [^ck-cwv]
- WASM plugins [^ck-plugin]

**What ContentKing does better:**
- Real-time continuous monitoring — crawlkit is on-demand only [^ck-monitor]

[^ck-home]: https://www.contentkingapp.com/

### SEO PowerSuite

**Profile:** Desktop SEO suite (C++) with four integrated tools. Free / $299/yr Professional. [^sps-home]

**Strengths:**
- All-in-one suite (rank tracking, backlink audit, on-page, link building) [^sps-home]
- TF-IDF content optimization [^sps-content]
- GSC integration [^sps-gsc]
- ~50 audit checks [^sps-count]
- Affordable $299/yr [^sps-price]

**Weaknesses:**
- Slow crawl speed (~100 pages/sec) [^sps-speed]
- No JS rendering [^sps-js]
- No security or accessibility auditing [^sps-sec] [^sps-a11y]
- No CWV data [^sps-cwv]
- Desktop-only, no REST API [^sps-self] [^sps-api]
- No historical trends [^sps-history]

**What crawlkit does better:**
- Broad analyzer/check coverage vs ~50 [^ck-count] [^sps-count]
- JS rendering [^sps-js]
- Security headers + accessibility [^sps-sec] [^sps-a11y]
- REST API for automation [^ck-api] [^sps-api]
- Postgres output [^ck-export]
- WASM plugins [^ck-plugin]

**What SEO PowerSuite does better:**
- Rank tracking and backlink analysis — crawlkit has neither [^sps-home]
- GSC integration [^sps-gsc]
- TF-IDF content optimization [^sps-content]
- Established, mature product [^sps-home]

[^sps-home]: https://www.link-assistant.com/
[^sps-content]: https://www.link-assistant.com/optimization/ — TF-IDF content tool
[^sps-price]: https://www.link-assistant.com/pricing/ — $299/yr
[^sps-speed]: https://www.link-assistant.com/ — desktop crawler performance

### Netpeak Spider

**Profile:** Desktop SEO crawler (C++) with 60+ checks. $19/mo or $249 lifetime. [^np-home]

**Strengths:**
- Fast crawl speed (~300 pages/sec) [^np-speed]
- Affordable lifetime license ($249) [^np-price]
- 60+ SEO checks [^np-count]
- Lightweight memory usage [^np-memory]

**Weaknesses:**
- Windows-only [^np-platform]
- Limited JS rendering [^np-js]
- No security or accessibility auditing [^np-sec] [^np-a11y]
- No custom extraction [^np-extract]
- No log file analysis [^np-log]
- No CWV or CrUX data [^np-cwv] [^np-crux]
- No GSC integration [^np-gsc]
- No REST API [^np-api]
- No historical trends [^np-history]

**What crawlkit does better:**
- Broad analyzer/check coverage vs 60+ [^ck-count] [^np-count]
- Cross-platform (Linux, macOS, Windows) [^ck-self] [^np-platform]
- Security headers + accessibility [^np-sec] [^np-a11y]
- WASM plugins [^ck-plugin]
- OSS, self-hosted [^ck-oss] [^np-self]
- Postgres output [^ck-export]

**What Netpeak Spider does better:**
- Faster raw crawl speed (300 pages/sec on desktop) [^np-speed]
- Lower memory footprint for simple crawls [^np-memory]
- Lifetime license option ($249) [^np-price]

[^np-home]: https://netpeaksoftware.com/spider
[^np-speed]: https://netpeaksoftware.com/spider — advertised 300 pages/sec
[^np-price]: https://netpeaksoftware.com/spider — $19/mo or $249 lifetime
[^np-memory]: https://netpeaksoftware.com/spider — lightweight resource usage
[^np-platform]: https://netpeaksoftware.com/spider — Windows application

---

## Sources

All source URLs used in footnotes above. These links support capability statements only; they should not be interpreted as independent validation of pricing, performance, resource usage or negative feature claims:

1. Screaming Frog — https://www.screamingfrog.co.uk/seo-spider/
2. Sitebulb — https://sitebulb.com/
3. Lumar — https://www.lumar.io/
4. Semrush Site Audit — https://www.semrush.com/site-audit/
5. OnCrawl — https://www.oncrawl.com/
6. ContentKing — https://www.contentkingapp.com/
7. SEO PowerSuite — https://www.link-assistant.com/
8. Netpeak Spider — https://netpeaksoftware.com/spider
9. crawlkit README — https://github.com/WyattAu/crawlkit/blob/main/README.md
10. crawlkit Benchmarks — https://github.com/WyattAu/crawlkit/blob/main/docs/benchmarks/measured-v5.3.0.md
11. crawlkit Roadmap — https://github.com/WyattAu/crawlkit/blob/main/docs/ROADMAP.md
