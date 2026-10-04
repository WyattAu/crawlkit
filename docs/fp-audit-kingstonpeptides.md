# False-positive audit — kingstonpeptides.com

**Date:** 2026-10-03
**Engine:** crawlkit 6.0.0-alpha.4 @ `feebe2d3` (pre-fix)
**Target:** <https://kingstonpeptides.com> — Astro SSR storefront, 665 source
pages across 10 locales, Cloudflare + Render, behind GSC (232 impressions,
position ~54).
**Method:** 60-page crawl (`--concurrency 2 --delay 150`, robots.txt respected),
every finding classified against the live HTML and the kp-ecom source tree, then
re-crawled after each fix. Crawl-kit's own audit database was the evidence base;
every claim below is reproducible with the commands in *Reproducing*.

---

## Headline

| Metric | Before | After | Change |
|---|---:|---:|---:|
| Findings on 60 pages | 6,788 | 4,123 | **−39.3%** |
| Distinct finding codes | 201 | 146 | −27% |
| **Warnings** (actionable) | 2,415 | 891 | **−63%** |
| Info | 4,357 | 3,216 | −26% |
| Error | 16 | 16 | unchanged |
| Exact duplicate rows | 179 | 0 | eliminated |
| Cross-code redundant findings | 881 (13.0%) | 22 (0.5%) | −97.5% |
| Distinct messages reported under >1 code | 23 | 4 | −83% |

The error count is deliberately unchanged: no finding was downgraded across the
error boundary to make a number look better. Every change removed a finding that
was factually wrong, or merged two reports of one fact.

**Warning count is the metric that matters.** Warnings are what a site owner
works through. Cutting them by 63% while leaving errors untouched means the audit
got *more* accurate about what it demands, not quieter.

---

## Confirmed false positives, by root cause

### 1. Trailing slash treated as a different URL — 774 findings

The site serves `/en` and canonicalises to `/en/`. Because `Url::path()`
preserves the trailing slash and every canonical/hreflang analyzer compared it
with `!=`, the two spellings of one resource compared unequal. Every
internationalised page was reported as broken.

| Code | Findings | What it claimed |
|---|---:|---|
| `CANDEEP001` | 59 | "Canonical path mismatch" |
| `CANSELF-V5001` | 59 | "Canonical points to different URL" |
| `CANCHAIN-V5001` | 59 | "Canonical points off-page" |
| `CANSELFRF001` | 59 | "Canonical does not self-reference" |
| `CANSELFRF-V6089` | 59 | "Canonical does not self-reference" |
| `HREFSELF-V2001` | 60 | "Missing hreflang self-reference" |
| `HREFRECIP-V5001` | 60 | "Missing self-referencing hreflang" |
| `SITEMAP006` | 118 | "Non-canonical page…" (single-page + post-crawl) |
| `CANON005` | 58 | "Canonical URL has no incoming links" (post-crawl) |

One bug, thirteen analyzers, 774 findings. There were **three** independent
`normalize_url` implementations in the tree, each with slightly different rules.

**Fix:** new `analyzers::url_norm` module with `paths_equivalent` and
`urls_equivalent`, applied at every comparison site. `urls_equivalent` folds
trailing slash, fragment, scheme/host case, and default port — and deliberately
*does not* fold the query string, because a canonical that drops query parameters
is a real difference. All 774 findings now 0.

**Retained:** 9 hreflang findings survive, and they are **true positives**. The
root page declares `hreflang` alternates to `/en/`, `/sv/`, `/de/`… but has no
self-referencing `hreflang`, which Google requires. That is a genuine defect in
the site's markup.

### 2. Hreflang reciprocity asserted from a single page — 594 findings (8.8%)

`HREFR001` claimed a page's hreflang set was "not reciprocal" by checking
whether each `hreflang` target also appeared in that page's **outbound `<a
href>` links**.

This is not what reciprocity means, and a single-page analyzer cannot evaluate
it:

- Google requires the **referenced page** to carry a `hreflang` pointing back.
  Whether it does is a property of the other page.
- `<link rel="alternate" hreflang>` in `<head>` is the normal, correct way to
  publish alternates. Those URLs are essentially never duplicated as visible
  links — so the check fired on *every alternate of every internationalised
  page* while describing correct markup as broken.

Nine locale alternates × 66 pages = 594 findings, 8.8% of the entire audit.

**Fix:** removed from single-page analysis, with the reasoning recorded in place.
The parts of hreflang that *are* checkable from one page (self-reference,
duplicate language codes) remain covered. Cross-page reciprocity needs hreflang
persisted in the crawl graph (`PageData` has no `hreflang` field) — recorded as
follow-up work, not silently guessed at.

### 3. Same defect reported under many codes — 881 redundant findings (13%)

Analyzers were added additively and several clusters implemented the *same*
check independently:

| Defect | Codes emitting it |
|---|---|
| CSP `script-src 'unsafe-inline'` | `CSP001`, `CSPDIR002`, `CSPSS-V2002`, `CSPSSRC-V5001` |
| Multiple `<h1>` | **8 codes** (`A11Y004`, `CDEPTH003`, `HEAD003`, `H1MULTI-V6108`, `HEADH1-V5002`, `HEADSC003`, `HHIERDEEP003`, `HHIER-V2003-DEEP-DEEP`) |
| Meta description too short | **7 codes** |
| Missing COEP | **7 codes** |
| Table missing caption | **9 codes** |
| Missing COOP | 6 codes |

One real issue was counted three times as loudly as three distinct ones, which
makes severity roll-ups meaningless and trains users to ignore the output.

**Fix:** `analyzers::dedupe` collapses findings that share page, category, and a
normalised title (version/depth suffixes stripped) *across different codes*.
Nothing is discarded silently — the surviving finding records the suppressed
codes in its description, so filtering on any of them still surfaces the issue.

Deliberately **not** collapsed: two findings from the *same* code. Several
analyzers legitimately report once per offending element under one code (generic
anchor text, one finding per link), and merging those would hide real defects.
This was caught by a test (`ANCHGEN001`) rather than assumed.

**Proof no defect was lost.** Comparing the corpus with dedupe enabled and
disabled:

```text
distinct normalized defects: no-dedupe=1953  dedupe=1953
```

Identical. 736 duplicate *codes* removed, zero distinct defects removed.

### 4. robots.txt `Disallow` read without user-agent scoping — 60 findings

`ROBOTS-V2001` reported "Robots.txt disallows `/`" on all 60 pages. It collected
every `Disallow:` line in the file regardless of its `User-agent:` block:

```text
User-agent: *
Allow: /                    ← the crawlkit group: allowed everywhere

User-agent: CCBot
Disallow: /                 ← training crawlers only, deliberately blocked
User-agent: ByteSpider
Disallow: /
```

The crawler's own group says `Allow: /`. Flattening the file attributed the
training-crawler block to the audit, making the whole site look blocked.

**Fix:** new `robots_group` module implementing real group resolution — select
the single most specific matching `User-agent` token, fall back to `*`, apply
longest-match-wins between `Allow` and `Disallow`. `AnalysisContext` now carries
`user_agent` so the analyzer knows the crawler's identity. All 60 findings gone;
`Disallow: /admin` and `/api/` are still correctly reported (they are in the
`*` group).

### 5. crawlkit never asked for compression, then blamed the site — 60 findings

`COMP001` reported "large response not compressed" on all 60 pages. Verified
against the live site:

```text
curl -H 'Accept-Encoding: gzip, deflate, br' -I  →  content-encoding: br   (27,996 B)
crawlkit's own client                            →  no header,             (70,714 B)
```

Two compounding causes, both confirmed empirically:

1. `reqwest` is configured with `default-features = false` and no compression
   features, so it sends **no** `Accept-Encoding` at all. A server has no reason
   to compress a response that never advertised support.
2. Turning those features on does **not** fix it: reqwest then strips
   `Content-Encoding` while decoding, making a `br`-compressed response
   indistinguishable from an uncompressed one. Measured directly —
   `hdrs_before_body=[]` with the features on, `["content-encoding=br"]` with
   them off.

**Fix:** crawlkit now negotiates encoding explicitly and decodes itself
(`compression` module: `gzip`/`deflate` via `flate2`, `br` via `brotli`), so the
wire header stays observable. `FetchResult` gained `content_encoding` and
`transfer_size`; `AnalysisContext` carries the encoding forward. Decoding is
conservative — an unsupported encoding or a corrupt body degrades to the raw
bytes rather than yielding an empty page.

This is also a **correctness improvement beyond the finding**: crawlkit now
transfers what a browser transfers, so fetch timings reflect real user cost.

### 6. CDN product name read as a version leak — 60 findings

`Server: cloudflare` names the edge product and discloses no version, but
`"Cloudflare"` sat in `VERSION_PATTERNS` alongside entries like `"Apache/"` that
carry a version separator. Every page of every Cloudflare-fronted site was
flagged. All 60 gone; `Cloudflare/` with an actual version is still caught.

### 7. Decorative and logo imagery — 124 findings

- `IMGAR001` flagged `logo-withtext.svg` at 4.5:1 and `janoshik-logo.svg` at
  3.82:1 as "unusual aspect ratio". A logo is *supposed* to be wide. Also
  double-counted, because the header and footer logos are the same `src`.
  **Fix:** skip logo/badge/icon/divider assets and `alt=""` imagery, dedupe by
  `src`, and raise the threshold to a genuinely anomalous 10:1.
- `IMGALTDEEP001` reported 3 of 7 images on every page as "missing or empty alt
  text" — those three carried `alt=""`, which is the **WCAG-sanctioned** way to
  mark an image decorative. **Fix:** only a genuinely absent attribute counts.

### 8. Accessible name ignored for icon links — 59 findings

`INTLINK-V2001` reported the logo link as having "empty anchor text":

```html
<a href="/en" aria-label="Kingston Peptides"><img src="…" alt="Kingston Peptides"></a>
```

It has **two** accessible names. The analyzer read only the text between `<a>`
and `</a>`, stripped the `<img>`, and found nothing — despite `ExtractedLink`
already carrying `aria_label` and `img_alt`. All 59 gone.

### 9. Duplicate entity names — 120 redundant findings

`ELINK001` emitted three identical findings per page. The site names itself
"Kingston Peptides" in `Organization`, `Store`, and `WebSite`; `get_entity_names`
pushed each occurrence without deduplication. Now deduplicated (180 → 60).

### 10. Advice that no longer protects anyone — 196 findings

| Code | Findings | Problem |
|---|---:|---|
| `ECT001` | 59 | `Expect-CT` removed from Chrome in 2021; ignored everywhere |
| `CT001` | 59 | Same header, duplicate analyzer. CT is a CA duty, not an origin duty |
| `XSS001` | 38 | `X-XSS-Protection` deprecated by WHATWG and removed from all current browsers; `1; mode=block` *introduced* XSS vectors in several versions |
| `XPCDP001` | 38 | `X-Permitted-Cross-Domain-Policies` only ever governed Flash/Acrobat-era policy files |

Recommending these is not a false positive in the narrow sense — the headers
really are absent — but it is worse than a false positive, because it directs
site owners to spend effort on headers that protect nothing. All removed.

### 11. Checks that cannot fire on a normal page — 98 findings

- `CORSMISS-V6070` (60): "No CORS headers". CORS governs whether *scripted code on
  another origin* may read a response. A document fetched by navigating to it is
  not subject to it; `Access-Control-Allow-Origin` on a document response affects
  no navigation. The condition cannot occur, yet it fired on every page.
- `SIZE003` (38): "Missing Content-Length". HTTP/2 and HTTP/3 carry length in the
  frame layer and **forbid** the header (RFC 9113 §8.1); chunked transfer has no
  length by design; a compressed response's length is only known after encoding.
  The recommendation was to add a header the specification prohibits.
- `TITLE003` (40): "Title contains separator characters", justified by "search
  engines may truncate these at the separator". Truncation is driven by rendered
  pixel width; no current engine truncates at a separator character.
  `Brand - Page` is an ordinary title convention. Title *length* is already
  measured separately.

---

## What was verified as a TRUE positive

Recording these matters as much as the false positives — it shows the audit is
not simply being suppressed.

| Finding | Verdict |
|---|---|
| `NAP001` LocalBusiness schema missing `telephone` (60) | **True.** `Organization` and `Store` schema both lack `telephone`; a `ContactPoint` is present but empty. Genuine local-SEO gap. |
| `HREFSELF-V2001` on 9 pages | **True.** Root page declares hreflang alternates but has no self-reference. |
| `CACHE003` HTML marked non-cacheable (60) | **True.** Site sends `cache-control: no-store, must-revalidate`. |
| `CHARSET002` charset missing from header (60) | **True.** Verified: `content-type: text/html`, no `charset`. |
| `CSP001` / `CSPDIR002` `unsafe-inline` | **True.** CSP genuinely contains `'unsafe-inline'` in both `script-src` and `style-src`. |
| `AI-ACC006` AI bot blocked | **True.** `ByteSpider` is disallowed by policy, deliberately. |
| `SRI-V5001`, `SCRIPT002`, `ASYNC002`, `CRIT002` | **True.** Confirmed in the HTML. |
| `AI-AB001` no FAQ schema | **True.** Matches the site's own diagnostic: FAQ component exists but data is unpopulated. |

---

---

## Round 2 — verification at scale, and a second site

The fixes above were then re-verified rather than assumed, on a 200-page crawl
and on a second architecture (gov.uk). That surfaced five further classes,
three of which were invisible on a 60-page crawl of one site.

| # | Defect | Scale | Root cause |
|---|---|---|---|
| 13 | **CSP wildcard false positive at Critical** | 40 findings | `script-src` tested with `contains("*")`, so `*.youtube.com` read as "any host". gov.uk pins a long specific list using scoped wildcards plus a per-response nonce. |
| 14 | **Non-HTML resources audited as HTML** | 62 findings | `/llms.txt` (`text/plain`) got a **Critical "missing title tag"**. Every "missing X" analyzer fired on a plain-text file. |
| 15 | **`inspect` bypassed the crawl pipeline** | 64 findings | The content-type gate existed in `crawl` only; `inspect` ran the registry unconditionally. |
| 16 | **Short anchors flagged on indexes** | ~100 findings | Glossary pages link `A`→`#a`; a single character *is* the label. |
| 17 | **`http://schema.org` rejected** | 160 findings | Three analyzers compared `@context` against the `https` literal only. |

Two non-FP correctness bugs were also fixed:

- **Non-deterministic report text.** Codes were reproducible; the text was not.
  Six analyzers summarize a `HashMap` frequency table and sorted on the count
  alone, leaving ties in randomized order. `crawl` reports, `log-analyze` JSON,
  and cross-code deduplication were all affected.
- **`bytes fetched` reported decoded size**, understating real transfer ~4×.

Plus two CLI defects: `crawlkit report` defaulted to `html`, which was
advertised but never implemented, so a bare invocation always failed; and
cross-page findings were written only to a JSON file, invisible to `report`,
`compare`, `insights`, `trend`, `--monitor`, the dashboard and the API.

### Verified true positives (round 2)

| Finding | Verdict |
|---|---|
| `NAP001` missing `telephone` | **True.** `Organization` and `Store` both lack it. |
| `CSP001`/`CSPDIR002` `unsafe-inline` | **True.** Present in both directives. |
| gov.uk Article missing `headline` | **True.** Verified in the live JSON-LD: the block carries `name`, not `headline`. |
| gov.uk hreflang/canonical hygiene | **True** where reported. |

### Round 2 results

| Metric | kingstonpeptides (200pp) | gov.uk (40pp) |
|---|---:|---:|
| Critical findings | 0 | 0 |
| Cross-code redundancy | 0 | 0 |
| Error tier | 26 | 39 (was 117) |
| Warning tier | 3,039 | 871 (was 1,110) |

Every remaining same-`(page, title)` group is a same-code per-element finding —
12 short anchors, 8 links on one broken page, 2 JSON-LD blocks per page — which
are legitimately distinct defects, not duplicates.

**Losslessness.** Each change to the deduplication rule was proven not to lose
defects by running the corpus with dedupe enabled and disabled and comparing the
distinct normalized defect set: **1,831 = 1,831**. Each new regression test was
also verified to *fail* without its fix.

### Known remaining, with measurements

| Item | Impact | Why not fixed here |
|---|---:|---|
| `ELINK001` "entity needs a Wikipedia link" | 1/page | The rule is questionable for brands with no article; deduplicated but the advice is still weak. |
| `COLRCL-V2001-UNDERLINE` | ~1/page | "Links without underline" fires on nearly every link; WCAG 1.4.1 admits other means. Pure noise. |
| `CANDEP-V2003` "canonical has trailing slash" | 1/page | Unactionable: there is no way to know a "preferred" slash format. |
| Informational metric findings | ~60% of volume | Readability indices, TF-IDF, sentiment, scores. Metrics, not defects — but they dominate raw counts. |

---

## Round 3 — the defect-family registry

Deduplication by normalized title cannot see defects whose analyzers word the
message differently. A static scan of the analyzer sources (1,365
`code`/`title` pairs extracted from source, rather than from a crawl that only
exercises the analyzers its pages happen to trigger) found **195 such groups**.

Fuzzy clustering was measured first and **rejected**: at 60% token overlap it
proposed 46 clusters, several of which are different defects that must not be
merged — location vs organization entities, description-too-long vs
title-too-long, fairly vs very difficult readability, script-SRI vs
stylesheet-SRI, and the two opposite directions of the title/description
keyword comparison.

So the grouping is explicit. `analyzers::CANONICAL_TITLES` holds **201
families over 300+ codes**, built by union-find over:

1. codes whose normalized titles are *identical* — safe by construction;
2. reviewed synonym families.

Every component was then inspected for transitive over-merges. Five were found
and rejected by correcting the synonym table:

| Rejected merge | Why it is wrong |
|---|---|
| `TABACC-V2001` into table *captions* | That family is about missing table **headers** |
| `SITEMAPMISS001` into "no robots.txt" | Missing *sitemap* ≠ missing *robots.txt* |
| `LAZYIMG001` into image dimensions | It is a compound lazy-loading finding |
| `ARIALAND-V2006` into banner landmark | "No ARIA landmarks found" is broader |
| `XFOMISS-V6072` into X-Frame-Options | "No clickjacking protection" is an aggregate check |

### A bug the audit caught in the audit

Keys first canonicalized to a *family id*. That broke for codes whose title is
built at runtime: `SD006` produces `"Article missing headline"` dynamically, so
the static scan cannot place it in a family, and it fell back to a title key
while its static sibling `ART-HL001` resolved to a family id. The two stopped
merging and **gov.uk's error tier doubled from 40 to 80**.

Keys now canonicalize through the **title**, so the family mechanism and the
title mechanism cannot disagree. Regression test added and exercised through
the real registry.

### Results

| | Findings | Codes | Cross-code dupes |
|---|---:|---:|---:|
| corpus (20 fixtures) | 2,597 → **1,495** (−42%) | — | — |
| gov.uk (40pp) | 2,785 → **2,688** | 97 → 94 | **0** |
| kingstonpeptides (100pp) | 6,226 → **6,207** | 127 → 127 | **0** |

Losslessness proven by A/B on the corpus: **distinct defect keys 1,483 with
dedupe disabled and 1,483 enabled.**

Two existing dedupe tests were corrected rather than deleted — they paired
`CSPDIR002` (which reports `style-src`) with a `script-src` title, an artifact
of the old title-only keying that describes no real page. A new test now asserts
that `script-src` and `style-src` never merge.

---

## Engineering changes

### Round 1

| Change | File(s) |
|---|---|
| Shared URL equivalence | `analyzers/url_norm.rs` (new) — applied at 13 comparison sites |
| robots.txt group resolution | `robots_group.rs` (new) — `AnalysisContext.user_agent` added, plumbed through `FetchedPage` |
| Explicit transfer-encoding negotiation + decode | `compression.rs` (new) — `FetchResult.{content_encoding, transfer_size}` |
| Cross-code finding collapse | `analyzers/dedupe.rs` (new) — applied in `AnalyzerRegistry::analyze` |

### Round 2

| Change | File(s) |
|---|---|
| Deterministic top-N ordering | `content_analyzers.rs`, `seo_analyzers.rs`, `log_analyzer.rs` — tiebreak on the key |
| Ordered map serialization | `log_analyzer.rs` — `HashMap` → `BTreeMap` |
| CSP wildcard parsing | `analyzers/csp_wildcard.rs` (new) — bare vs scoped subdomain wildcard |
| Document content-type gate | `analyzers::is_auditable_as_document`, `analyzers::non_html_finding` — shared by `crawl` and `inspect` |
| Index/jump-link detection | `seo_analyzers::is_index_or_jump_link` |
| Wider dedupe key + schema.org context | `analyzers/dedupe.rs`, `content_analyzers::is_schema_org_context` |
| Self-identification in logs | `log_analyzer::classify_user_agent` |
| Transferred-byte metric | `crawl_engine/pipeline.rs`, `observability.rs` |
| Cross-page finding persistence | `cli/crawl.rs` |
| Report format contract | `cli/report.rs`, `cli/mod.rs`, `main.rs` |

New tests: `tests/determinism_of_text.rs` (6), `tests/non_html_resources.rs`
(6), plus `analyzers/url_norm.rs` (7), `robots_group.rs` (11),
`analyzers/dedupe.rs` (14), `analyzers/csp_wildcard.rs` (6), `compression.rs`
(23), and analyzer-level cases for each fix.

New dependencies: `flate2` (already in the lock transitively) and `brotli`.
Both added because reqwest's built-in decoders destroy the signal the
compression analyzer needs.

`FetchOutcome::Fetched` now boxes its `FetchResult` — the enum was sized by its
largest variant, making every outcome allocation-sized.

### Test posture

- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo test --workspace --lib --bins` — **4,426 passed**, 0 failed
- `cargo test --workspace --tests` — 34 suites, 0 failed (incl. 4 corpus tests)
- New unit tests: 23 (compression), 11 (robots group), 7 (url_norm), 9 (dedupe)
- `expected.json` updated for 14 corpus assertions whose *codes* were
  consolidated; each replacement was verified to carry the **identical title**,
  confirming the defect is still reported

Tests that had encoded buggy behaviour as expected were corrected rather than
deleted — e.g. `test_server_cloudflare_no_version` asserted that bare
`cloudflare` *should* be flagged. It now asserts the opposite, with a companion
test proving `cloudflare/1.2.3` is still caught.

---

## Round 4 — the measurement/metric split, and cross-page findings that were never stored

### Measurements were 39% of all findings

Readability indices, keyword extraction, entity detection and composite scores
were emitted as `info` findings alongside real problems. On a 100-page crawl that
was **2,401 of 4,696 info findings — 39% of every finding crawlkit reported**.

The cost is not the row count. A page with three real problems and twenty scores
is indistinguishable, in aggregate, from a page with twenty-three problems, so
every severity roll-up, "issues per page" average and trend line was dominated by
rows nobody can act on.

The split is by measurement, not by severity: a code is a metric when its output
is a quantity about the page and no markup change would "fix" it. Advisory content
analysis stayed a finding (`KWPRO*`, `TITLEKDEN*`, `CQ-V2001` — all actionable by
rewriting), and so did everything describing wrong markup (`CHARSET002`,
`CACHE003`, `COEP-V2001`, `AI-ACC006`, `IMGALT*`).

Nothing was discarded. Measurements are stored with an `is_metric` column,
retrievable via `get_page_metrics`, and written to `page-metrics.json`.

| | before | after |
|---|---|---|
| 40-page crawl, defect findings | 1,599 | 1,454 |
| 40-page crawl, measurements | — | 962 |

### Three noise analyzers, removed

All three fired on **100% of crawled pages** (40/40). Each is removed outright
rather than suppressed, with the rationale recorded at its registration site.

| Code(s) | Volume | Why removed |
|---|---|---|
| `ELINK001` | 40 | Asserted that a schema entity needs an outbound *body* link "to strengthen entity signals". No engine states that requirement, and the mechanism that does exist — `sameAs` on the entity — is not what the rule checked: it read `ctx.page.links`. |
| `ELINK002` | 0 here | Same fabricated "topical authority" requirement as `ELINK001`. Removed with its analyzer. |
| `COLRCL-V2001-UNDERLINE` | 40 | Fired whenever *any* CSS rule paired `text-decoration:none` with a `color:` declaration anywhere in the document — not on links, not on the link in question. That is the default styling of most navigation. WCAG 1.4.1 asks whether a link has *any* non-colour indicator, which is a property of one element's computed style and needs a rendering engine, not a stylesheet regex. |
| `CANDEP-V2003` | 40 | A trailing slash on a canonical is a valid URL form, and the recommendation was unactionable without knowing the site's chosen form. It also contradicted this product's own model: `analyzers::url_norm` deliberately folds trailing slashes when deciding URL equivalence. |

Registry size 775 → 772. A tripwire test (`removed_codes_do_not_reappear`)
scans the analyzer sources for `code: "…"` literals and fails if any of the four
returns; it was verified to fail on a deliberately reintroduced code.

The remaining contrast analyzers are untouched and remain canonical:
`ColorContrastAnalyzer` (`CONTR001/002`) and `ColorContrastLinkAnalyzer`
(`COLRCL001`) compute real WCAG ratios.

### 20 cross-page analyzers ran, and every finding was discarded

`PostCrawlAnalyzerRegistry` runs orphan-page detection, keyword cannibalization,
link equity distribution, redirect-chain optimization, internal link balance,
crawl quality, schema coverage, heading structure, canonical consistency and an
overall health score. The engine used only `.len()` on the result and dropped the
vector. Orphan pages and cannibalization are among the highest-value cross-page
results, so each crawl silently threw them away.

Four defects surfaced while fixing that, all verified against live HTML:

1. **`LinkVelocityAnalyzer` measured nothing.** `avg_links` summed a literal `0`
   per page and `zero_link_pages` counted every page via `filter(|p| true)`, so
   `LINK-V001` always reported *"Average links per page is 0.0"* and `LINK-V002`
   always reported *"100% of pages have no outgoing links"*.
2. **`InternalLinkBalanceAnalyzer` had the same shape, and the worse variant.**
   Both totals summed a literal `0`, so `total_external > 0` was never true and
   `LINK-BAL001` had **never fired on any crawl in the product's history** — a
   silently dead rule. A rule that always fires and a rule that never fires look
   equally like working analyzers from the outside; only the second is invisible
   in the output.
3. **Site-wide findings were attributed to `https://example.com`.** The seed came
   from `cfg.crawl_config.start_url`, which keeps its `Default` value because
   callers pass the target to `run_with_callback` rather than through the config.
4. **`LINK-V002` and `LINK-BAL002` tested the same condition** with identical
   description text, differing only in code and threshold. `LINK-BAL002` survives.

The natural data source, `PageData::links`, is always empty on read —
`row_to_page_data` returns `Vec::new()` because links live in a separate table.
The original hardcoded `0` was matching that emptiness rather than being
arbitrary. Both analyzers now read `CrawlData::links`, the populated
`(source_url, [target_url])` graph, and count only pages present in it: a page
absent from the graph cannot be distinguished from one whose links were never
recorded.

Verified on kingstonpeptides.com: the `links` table holds 272 rows across 10 pages
(avg 27.2 links/page), and both analyzers are now correctly silent. Cross-page
codes persist (`SITEMAP006`, `CANON005`, `DUP-CROSS001`, `DUP-CROSS002`,
`CANNIB001`), zero duplicate `(page_id, code)` pairs, and console counts agree
with storage.

### Two count-drift defects the split exposed

* The engine counted cross-page findings it never persisted, so the console
  summary and `crawlkit report` disagreed by exactly that many.
* `write_output` wrote `crawl-results.json` before post-crawl persistence ran, so
  the file's totals predated the findings the crawl had just added.

## Round 5 — severity semantics: the representative and the severity were chosen independently

`collapse_duplicates` kept the **first** finding positionally and then copied a
later, more severe family member's **severity** onto it. Code, title, severity and
description therefore came from different findings, and the output was a chimera:
a finding whose surviving code declared `Info` shipped at `warning`, because the
severity came from a sibling that had been discarded.

Measured on a live 40-page crawl, **2 codes shipped a severity their own source
never declares**:

| Code | Shipped as | Actually declares | Why |
|---|---|---|---|
| `MDESC-PX002` | `warning` | `info` | family also contains `META005` (`warning`) |
| `IMG004` | `warning` | `info` | family sibling declares `warning` |

This is not a corner case. **77 of 202** curated families contain members that
disagree on severity, so the decoupling applied to more than a third of them.

### The resolution

Keep severity at the **family maximum**, but make the most severe member the
finding that survives, so code, title, description and severity all come from one
analyzer.

Family-max is the right call on its own merits: when several analyzers
independently assess one defect and disagree, the most severe credible assessment
should stand. It is also the conservative direction — it never understates. What
was wrong was decoupling it from the code that earned it, not the max itself.

The alternative — hand-declaring a severity for all 202 families — was rejected.
It replaces a defensible computed rule with 202 judgements that can each be
wrong, and would *lower* severity for many families, which is a regression in a
tool whose value is not understating problems.

The replacement is also order-independent (strictly-more-severe wins), so it does
not depend on the caller's `(code, url)` sort. Ties fall to the first in sorted
order, which is deterministic. Suppressed codes remain listed in the description,
so filtering on any member's code still surfaces the defect.

Verified on the same crawl: **0 code/severity mismatches**, from 2. Six corpus
expectations named the previous representative and were updated to the code that
now reports the same defect — confirmed by title, not inferred from the code:

| Fixture | was | now | Same defect |
|---|---|---|---|
| `empty_page.html`, `spa_vue.html` | `TITLE-V4001` "Missing title tag" (`error`) | `TITLEMISS-V2001` (`critical`) | yes |
| `minimal_landing.html`, `spa_react.html` | `AI-CIT001` "Missing canonical URL" (`info`) | `CAN-V3001` (`warning`) | yes |
| `minimal_landing.html` | `META009` "Missing viewport meta tag" | `MOB001` (`error`) | yes |
| `spa_react.html` | `A11Y006` "Missing main landmark" (`error`) | `LAND001` (`error`) | yes |

`accessibility_focused.html` asserts `must_not_have` for that fixture, so both the
old and new codes are listed — dropping the old one would have weakened it.

## Round 6 — first-party plugins reused built-in codes, one causing a false negative

Chasing the `plugins/index` item from round 3 turned up something worse.

### The `plugins/index` "layout mismatch" was a misdiagnosis

Round 3 recorded: *"`plugins/index/` ships a flat `artifacts/*.wasm` layout, but
`--plugins` expects `<dir>/<name>/crawlkit-plugin.toml`. The five first-party
plugins are not directly loadable."*

That is not a defect. `plugins/index/` is a **marketplace index** — a catalogue
plus signed artifacts — consumed by `crawlkit plugin install`, which produces the
`<name>/crawlkit-plugin.toml` layout `--plugins` loads. Verified end to end: all
five install, verify hash and signature, and load during a crawl.

### Both first-party plugins emitted codes already owned by other defects

| Plugin code | Plugin title | Registry family for that code | |
|---|---|---|---|
| `HEAD001` | Multiple H1 headings | no headings found | ✗ |
| `HEAD002` | Heading level skipped | missing h1 heading | ✗ |
| `HEAD003` | No headings found | multiple h1 headings | ✗ |
| `META002` | Meta description too short | **title** too short | ✗ |
| `META003` | Meta description too long | **title** too long | ✗ |

`defect_key` resolves by **code**, so a plugin's code overrides its title. The
meta-description case is a **silent false negative**: a "Meta description too
short" finding was keyed into the *title* family and merged into the built-in
`TITLE001`, so on any page with both a short title and a short description the
description problem disappeared from the report.

Confirmed on live data — with the plugin loaded, `META002` is stored titled "Meta
description too short"; without it, no such finding exists at all.

Codes now say what they mean: `HEADING-MULTIH1`, `HEADING-SKIPLEVEL`,
`HEADING-NONE`, `METADESC-MISSING`, `METADESC-SHORT`, `METADESC-LONG`.

### Plugin findings never participated in deduplication

`AnalyzerRegistry::analyze` deduplicates; the pipeline appended plugin findings
afterwards. With `heading-structure` loaded, every multiple-H1 page was reported
twice — built-in `A11Y004` *and* the plugin's code — and no aggregate merged
them. The combined vector is now collapsed.

### Four near-identical heading-skip families

`"heading level skipped"`, `"heading levels skipped"`, `"heading level skip
detected"` and `"skipped heading level"` were four families for one defect,
differing by singular/plural and word order. `defect_signature` strips version
and depth decorations but does not normalize word order, so the same defect
reported up to four times under four families. Merged into one family of 12
codes.

### Outstanding: the shipped artifacts are stale

`plugins/index/artifacts/*.wasm` still contain the old codes (verified with
`strings`). They hash-match the index, so the index is self-consistent, but the
content is wrong until re-signed. Re-signing needs the private key for
`signed_by = 12a7a8db5aabb20b`, which is not available in the audit environment.

Until re-signed, `crawlkit plugin install meta-description-checker` reintroduces
the false negative above. Commands, once the key is available:

```bash
export CRAWLKIT_SIGNING_KEY=<hex seed for 12a7a8db5aabb20b>
cargo run --bin crawlkit -- plugin publish plugins/heading-structure
cargo run --bin crawlkit -- plugin publish plugins/meta-description-checker
```

## Open findings not addressed

| Item | Why |
|---|---|
| Cross-page hreflang reciprocity | Needs `hreflang` in `PageData` + migration. Currently `PageData` cannot express it, so the check is absent rather than wrong. |
| `CORSMISS` for XHR assets | Correctly removed for documents, but the check is legitimate for a JSON/font target type. Gating on content type would restore it. |
| Re-signing the two fixed plugins | Blocked on the private key for `12a7a8db5aabb20b`. Not self-executable here. |

## Reproducing

```bash
cargo build --release --bin crawlkit
./target/release/crawlkit crawl https://kingstonpeptides.com \
  --max-pages 60 --concurrency 2 --delay 150 \
  --user-agent "crawlkit/6.0.0-alpha.4 (SEO audit)" \
  --output /tmp/kp --format json
sqlite3 /tmp/kp/crawlkit.db \
  'select severity, count(*) from findings group by severity;'
```

The site's `sitemap.xml` returned HTTP 503 throughout the audit window, so the
crawl was link-driven. That is a **site** issue, not a crawlkit one, and it is
worth reporting to kp-ecom: 792 URLs are reportedly unindexed in GSC and the
sitemap that should submit them is failing.