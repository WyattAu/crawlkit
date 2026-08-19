# ADR-007: Deterministic Crawl Output

## Status
Accepted (2026-08-19)

## Context

crawlkit's engineering standards (`.adrs/coding-standards.md`, HFT-inspired —
see the HFT/ECN requirements in ADR-005 and `docs/ARCHITECTURE.md` §HFT)
include "Determinism: same input → same output, measured by test vectors."
Before v3.0.0 that standard was only partially real:

- **Already deterministic:** the crawl queue's tie-breaks were URL-lexicographic
  (ADR-002-era scheduler behavior), so *which* URLs get crawled in *what*
  order was stable given the same frontier.
- **Not deterministic — user agents:** `UserAgentRotator` was a round-robin
  counter shared across concurrent fetch tasks. Which page got which UA
  depended on Tokio task interleaving, i.e. on scheduler timing, i.e. on
  machine load. Re-running the same crawl produced different UA
  distributions, and different UAs can produce different server responses.
- **Not deterministic — a real bug:** `CrawlStats` accumulated counters in
  `HashMap`s and serialized them without sorting, so the *same crawl* could
  emit JSON with keys in a different order between runs. This is not merely
  cosmetic: it breaks byte-level report diffing and makes cache/incremental
  comparisons (which hash prior results) unreliable.
- **Not deterministic — findings:** analyzers ran under Rayon
  (ADR-004, parallel analyzer execution), and their per-page findings were
  concatenated in completion order, so finding lists varied run to run and
  page to page.

Why this matters to the product: determinism is a brand claim, and it is the
cheapest possible regression test — if two runs of the same input produce
byte-identical reports, whole classes of "works on my machine" defects become
impossible to ship. But a claim like that is only credible if the bytes are
the contract, not "semantically equivalent modulo ordering."

## Decision

Make **byte-identical exports** the testable contract: same crawl input +
same seed ⇒ identical report bytes. Four rails, each closing one
nondeterminism source above:

1. **Seeded per-URL user-agent assignment.** When a seed is configured
   (`HttpClientConfig::with_seed`, wired through `CrawlEngineConfig.seed`),
   the UA for a request is chosen by a stable hash of `(seed, url)` —
   `UserAgentRotator::ua_for_url` — instead of the round-robin counter.
   Unseeded operation keeps round-robin (for UA-balancing users). The
   mapping is pure: re-crawling the same URL picks the same UA regardless of
   task interleaving.
2. **Canonical finding ordering at the registry choke point.**
   `AnalyzerRegistry::analyze` stable-sorts the merged findings by
   `(code, url)` before returning (crates/crawlkit-engine/src/analyzers/mod.rs).
   Individual analyzers cannot leak ordering instability past the registry
   even though they execute in parallel; there is exactly one place where
   order is defined, and one test asserting it.
3. **Sorted exports.** Report/export paths (HTML/Markdown/JSON) emit from the
   canonically-ordered findings and sorted key collections, so the
   `CrawlStats` HashMap serialization bug class is closed at every sink, not
   just one.
4. **Pure seed derivation.** `DeterminismController::derive_seed` is a pure
   hash of `(seed, context)` — same inputs, same output, no hidden state —
   and the order-sensitive cases use the explicit `derive_seed_stream`
   counter. Nothing that needs reproducibility ever draws from a
   process-global PRNG.

A regression test pins the contract: identical input + seed must produce
identical exported bytes (CHANGELOG 3.0.0, "Determinism rails").

## Consequences

### Positive
- Determinism became a falsifiable property instead of an aspiration:
  the byte-identity test fails the moment any future change reintroduces an
  unordered collection or a timing-dependent branch into the output path.
- Report diffing, result caching, and incremental-crawl comparisons get a
  stable basis (compare hashes of bytes, not normalized structures).
- Debugging: two runs that differ pinpoint a real defect rather than
  scheduler noise.
- The seed threads through `CrawlEngineConfig`, so the API/CLI surface
  controls it without new plumbing per rail.

### Negative
- Strictly more work per crawl: sorts at the registry and at each export are
  O(n log n) in findings per page and per report. With findings measured in
  thousands and the sort keys being two string comparisons, this is noise
  against parse + analyze + IO costs — but it is nonzero and now permanent.
- Seeded UA assignment trades round-robin *balance* for *stability*: a crawl
  where one heavily-hashed UA bucket dominates is possible (though with a
  reasonable UA list and many URLs the distribution evens out). Users who
  need strict rotation simply do not set a seed.
- Every future output-affecting component must be built deterministically
  from day one (no wall-clock timestamps inside report bodies, no iteration
  over unsorted hashmaps into serialized output). The byte-identity test
  enforces this only where coverage exists.

### Risks
- What is **not** deterministic must stay documented, or users will
  reasonably feel the brand claim oversold: wall-clock timings embedded in
  crawl *metadata* (durations, timestamps), network variance (a page's
  content can change between runs — determinism is over the *same* fetched
  input), storage row ids (SQLite/Postgres autoincrement ids appear in
  stored rows, not in exports), and JS-rendered DOM (best-effort within the
  render timeout window per `docs/ARCHITECTURE.md`).

## Alternatives Considered

- **Post-hoc normalization at export time only (sort in the export layer):**
  rejected — storage order would remain unstable, so anything reading the
  database rather than the export (API endpoints, incremental comparisons)
  would still see run-varying order, and the normalization logic would be
  duplicated per sink instead of living at one registry choke point.
- **Full event-sourcing replay (record all crawl events, derive reports by
  deterministic replay):** the strongest form of the guarantee — enables
  time-travel debugging and exact diffing of crawl behavior — at the cost of
  an event store, replay tooling, and serialization stability for every
  event. Deferred as a future direction; the current rails deliver the
  product-visible contract (byte-identical reports) without the machinery.
- **Serialization-level key ordering only (e.g. a preserving-map type in
  stats):** would have fixed the `CrawlStats` bug but not finding order or
  UA assignment; too narrow to support the brand claim.

## References

- `.adrs/coding-standards.md` — HFT determinism requirement
- ADR-004 (Parallel Analyzer Execution) — the source of completion-order
  variance that rail 2 absorbs
- `crates/crawlkit-engine/src/determinism.rs` — `DeterminismController`
- `crates/crawlkit-engine/src/http.rs` — `ua_for_url`, `with_seed`
