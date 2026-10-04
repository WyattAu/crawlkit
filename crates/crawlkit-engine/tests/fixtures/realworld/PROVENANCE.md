# Real-world fixture provenance

These are **captured live pages**, not hand-written HTML. The synthetic
fixtures in `../corpus/` prove analyzers behave on markup we constructed; these
prove they behave on markup nobody designed for the test.

Every false-positive class fixed in the audit rounds was found on real pages. A
regression harness built only from synthetic HTML would not have caught any of
them.

## Why captured rather than fetched

`realworld_tests.rs` runs entirely offline. Fetching live sites from a test would
make the suite depend on the network, on those sites staying up, and on their
markup not changing — so a test failure could never distinguish "crawlkit
regressed" from "gov.uk deployed". Capturing freezes the evidence.

## Sites

Chosen for structural variety, not popularity: government, health, news,
academic, documentation, standards-body, blog, small commercial, and a
deliberately minimal page.

| Fixture | Source | Why included |
|---|---|---|
| `govuk_homepage.html` | `https://www.gov.uk/` | Large accessible public-sector site. High-quality markup is the best test for false positives: a rule that fires here is almost certainly noise. |
| `govuk_check_mot_history.html` | `https://www.gov.uk/check-mot-history` | GOV.UK transaction start page — forms, breadcrumbs, multi-step navigation. |
| `nhs.html` | `https://www.nhs.uk/` | Second large public sector. Different design system from GOV.UK. |
| `kp_homepage.html` | `https://kingstonpeptides.com/` | The audit target. Astro SSR storefront, Cloudflare + Render, 10 locales, schema-heavy commerce markup. |
| `mozilla_blog.html` | `https://blog.mozilla.org/en/` | Editorial/blog. Different heading and link patterns from a corporate site. |
| `arxiv_cs.html` | `https://arxiv.org/list/cs.AI/recent` | Dense academic listing. Extreme link density — good for link-analysis false positives. |
| `rust_docs.html` | `https://doc.rust-lang.org/std/` | Generated documentation. Machine-produced markup with unusual class/ID patterns. |
| `xkcd.html` | `https://xkcd.com/353/` | Tiny, near-valid HTML5, image-led, minimal text. |
| `python_org.html` | `https://www.python.org/` | Small nonprofit marketing site. |
| `example_org.html` | `https://example.com/` | Deliberately minimal: one paragraph, one link. Any finding here is worth a look. |

## What each capture consists of

Three files per site, because a body alone produces a misleading corpus:

| File | Fed to the harness as |
|---|---|
| `<name>.html` | `AnalysisContext::body` |
| `<name>.headers.json` | `headers`, `server`, `content_type`, `content_encoding`, `status_code` |
| `<name>.robots.txt` | `robots_txt` |

**Headers are not optional.** Roughly twenty security-header analyzers read
`ctx.headers`. The first version of this corpus passed `headers: &[]`, and the
result was that all twenty fired on all ten captures — `SEC001` "Missing CSP" on
gov.uk, which sends a strict CSP. That buried the HTML findings the corpus exists
to test *and* would have made a header-analyzer regression invisible, because the
rules already looked like they always fire.

**robots.txt is not optional either**, for the same reason: passing `None` made
`AI-ACC009` "No robots.txt found" fire on all ten captures, every one of which
serves a perfectly good `robots.txt`.

## Refreshing a capture

```bash
scripts/capture-realworld-fixture.sh <name> <url>
```

The script captures all three files from the same request, so body and headers
cannot drift apart. Use it rather than a bare `curl`: it is the thing that gets
the two mistakes below right.

`--compressed` is not optional. `https://www.python.org/` serves
`content-encoding: br` with no `Content-Length`, so a plain `curl` writes the
**compressed bytes** — the result looks like a valid 11 KB file and parses as
nothing. This is the same signal crawlkit's own `compression` module has to
negotiate explicitly for the same reason.

The script also warns above a 150 KB body budget. BBC News (934 KB), Wikipedia
(253 KB), Stripe (678 KB) and GitHub (325 KB) were all dropped for exceeding it;
the suite reads every fixture on every run.

After refreshing, regenerate nothing by hand: `realworld_tests.rs` will fail
with the exact expectation that moved, and the diff is the record of what
changed on the site.

## Adding a site

1. Capture it with the command above.
2. Keep it under ~150 KB — the suite reads every file on every run.
3. Add a row to the table above explaining *why* this site earns its place.
   A duplicate design system adds cost without adding coverage.
4. Run `cargo test -p crawlkit-engine --test realworld_tests` and record the
   expectations it produces **after checking each one by hand**. The value of
   this corpus is that the expectations were verified; an unreviewed snapshot
   of current output is just a change detector.