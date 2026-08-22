# ADR-009: Dogfood Validation Protocol

## Status

Accepted (2026-08-22)

## Context

crawlkit's analyzers are heuristics about real-world web pages, but until
this decision the project had no mechanism for validating those heuristics
against production websites. Unit tests use synthetic fixtures written by
the same author as the heuristic — they encode the author's assumptions,
not reality.

Two prior incidents motivated this:

1. **The kingstonpeptides.com a11y bug**: crawlkit flagged WCAG-H67-correct
   decorative badges (`alt=""` + `aria-hidden="true"`) as missing-alt
   Errors. The check even contradicted its own recommendation text. Real
   usage found a spec inversion that 500+ unit tests did not.
2. **The analyzer noise problem**: "Multi-language content detected" and
   "Missing speakable schema" fired on 100% of pages of a correctly
   configured site — findings indistinguishable from defects but carrying
   zero actionable signal.

## Decision

Adopt a standing dogfood protocol: before any release, crawl a real
production site with the freshly-built binary and triage **every finding**
as one of:

- **TRUE positive** — the site has a genuine issue; fix the site.
- **FALSE positive** — the analyzer is wrong; fix crawlkit (this is an
   engine bug).
- **Noise** — accurate but non-actionable by design; redesign or remove
   the finding.

The protocol is codified in `scripts/dogfood.sh` (default target:
kingstonpeptides.com, a site we operate and can therefore fix both sides
of). The script builds the release binary, crawls 100 pages, and emits a
severity/category/code summary plus an a11y canary line.

## Evidence from first formal use (2026-08-22)

| Finding | Triage | Outcome |
|---|---|---|
| CQ004 thin content (98 pages) | TRUE — production outage | Blog SSG conversion shipped empty article bodies on 756 pages (deployed the prior day, still live); source API held the real 2,000+ word articles. July crawl of same pages: 1,378-3,194 words — proving the measurement pipeline correct |
| META005 short descriptions (9 pages) | TRUE — fallback strings | 98-117 char template fallbacks where the DB had proper 121-154 char descriptions the build never fetched |
| WC004 "long sentences" (100 pages) | FALSE — engine bug | Full-page words ÷ headings-only sentence count reported 147-190 "words/sentence"; fixed in 4.0.0 with a corpus-consistent counter |
| ISEO006 (100 pages) | Noise | Removed in 4.0.0 |
| AI-AB007 (100 pages) | Noise | Removed in 4.0.0 |
| A11Y canary | Clean | 0 findings — the H67 fix from the original report confirmed in production |

Yield: one caught production regression, one engine bug fix, two noise
removals, one confirmed fix — from a single 100-page crawl.

## Consequences

**Positive:**
- Every analyzer heuristic is eventually confronted with reality; false
  positives surface as engine bugs rather than user trust erosion.
- Sites we operate give a two-sided feedback loop (crawlkit findings
  drive real site fixes, which drive new crawl validations).
- The protocol is cheap: one script run (~15 s of crawl time per 100
  pages) plus triage proportional to new findings.

**Negative / trade-offs:**
- Single-site bias: kingstonpeptides.com is a modern Astro/React
  e-commerce site; heuristics validated only against it may still
  misbehave on legacy stacks (table layouts, frames, government portals).
  Mitigation: additional dogfood targets with different profiles should
  rotate in over time.
- Triage is manual judgment; a maintainer can wrongly classify an engine
  bug as a site issue. Mitigation: findings against sites we don't
  operate should be source-verified (as done for KP via its repository)
  before acting.
- The crawl is unauthenticated/unchanged; findings on the dogfood target
  leak nothing about third-party sites but do reflect one specific
  deployment's config (headers, hreflang) — severity expectations must be
  interpreted per-site.

## Alternatives Considered

1. **Golden-file corpus testing** — a curated set of real HTML snapshots
   with expected findings. Rejected as the *primary* mechanism: the
   corpus encodes the same author bias as unit tests, and it ages (sites
   change). Retained as a possible complement for regression pinning.
2. **Lighthouse/axe-core parity checks** — diff crawlkit findings against
   established tools on the same pages. Rejected: parity would import
   their heuristics' bugs as ground truth; crawlkit's value includes
   findings they don't make.
3. **Community false-positive reports** — wait for users to report bad
   findings. Rejected as sole mechanism: trust erodes before reports
   arrive; the a11y bug shipped exactly this way.

## References

- `scripts/dogfood.sh` — the protocol implementation
- `docs/MIGRATION.md` (4.0.0 section) — the WC004 fix this protocol forced
- kingstonpeptides.com validation session (2026-08-22) — evidence table
  source
