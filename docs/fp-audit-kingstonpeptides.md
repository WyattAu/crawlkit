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
| `METAKEY`/`OPDESC`-style near-synonym codes | 3 codes/1 defect | Titles differ by synonyms ("No" vs "Missing"), which signature normalization cannot safely unify. Needs analyzer consolidation, not heuristics. |

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

## Open findings not addressed

| Item | Why |
|---|---|
| Cross-page hreflang reciprocity | Needs `hreflang` in `PageData` + migration. Currently `PageData` cannot express it, so the check is absent rather than wrong. |
| `ELINK001` "entity needs a Wikipedia link" (60) | Now correctly deduplicated to 1/page, but the *rule* is questionable: most commercial brands have no Wikipedia article, so the recommendation is rarely actionable. `info` severity. Worth redesigning. |
| `COLRCL-V2001-UNDERLINE` (59) | "Links without underline" fires on essentially every link. WCAG 1.4.1 can be met by other means. Pure noise at current volume. |
| `CORSMISS` for XHR assets | Correctly removed for documents, but the check is legitimate for a JSON/font target type. Gating on content type would restore it. |
| Plugin distribution | `plugins/index/` ships a flat `artifacts/*.wasm` layout, but `--plugins` expects `<dir>/<name>/crawlkit-plugin.toml`. The five first-party plugins are not directly loadable. |

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