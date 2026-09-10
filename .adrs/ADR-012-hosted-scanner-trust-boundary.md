# ADR-012: Hosted Free-Scan Scanner Service

**Status:** Accepted (prototype gated by 5.2.0, GA gated by 5.3.0)
**Date:** 2026-09-10
**Deciders:** Product strategy (docs/PRODUCT_STRATEGY.md §4), maintainer approval
**Related:** ADR-003 (WASM plugin sandboxing), ADR-007 (deterministic crawl output), ADR-008 (API backpressure and idempotency), docs/PRODUCT_STRATEGY.md §4 (hosted scanner candidacy record)

## Context

The product strategy adopts a hosted single-URL free scan as a growth channel
(decision recorded in docs/PRODUCT_STRATEGY.md §4). The scanner lets anyone
submit one URL and receive a bounded, read-only audit of that URL and a small
sample of its site — no account, no API key, no install.

This service is publicly reachable from the internet, which makes it
categorically different from every other crawlkit deployment: the operator does
not control who submits requests, and submitted URLs can point at internal
network addresses. Every prior crawlkit component assumed a trusted operator.
This ADR defines the trust boundary that assumption break requires.

## Decision

Ship a hosted scanner as a **separate service** (new crate `crawlkit-scanner`),
not part of crawlkit-api. It wraps the engine in a hardened configuration with
four invariant properties:

### 1. SSRF denial by construction

The scanner MUST resolve and validate every URL **before** any network request
is issued, and MUST re-validate after every DNS resolution and every redirect
hop. Validation is by construction, not by blocklist:

- **DNS resolution is pinned.** The scanner resolves the host itself, checks
  every resolved address against the private-range rules below, and issues the
  request against the validated IP (SNI/Host header preserved). This closes
  the DNS-rebinding window between check and use: an attacker-controlled DNS
  answer cannot swap in a private IP after validation passes, because the
  connection is made to the address that was validated, not re-resolved.
- **Private, loopback, link-local, and unique-local ranges are denied.**
  IPv4: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `127.0.0.0/8`,
  `169.254.0.0/16`, `0.0.0.0/8`, `100.64.0.0/10`, `192.0.0.0/24`,
  `198.18.0.0/15`, `240.0.0.0/4`. IPv6: `::1/128`, `fc00::/7`, `fe80::/10`,
  `::ffff:0:0/96` (IPv4-mapped), `64:ff9b:1::/48` (NAT64 local), `100::/64`
  (discard-only). IPv4-mapped IPv6 is unwrapped before evaluation so
  `::ffff:10.0.0.1` is evaluated as IPv4.
- **Non-HTTP(S) schemes are denied** at parse time (`file:`, `data:`, `ftp:`,
  etc.). Credentials in URL (`user:pass@host`) are denied.
- **Redirect handling.** Each hop in a redirect chain is resolved and validated
  independently; a redirect to a private address is denied and the crawl stops.
  Redirect chains are capped at the engine limit (8 hops).
- **Re-validation on DNS events.** Because connections are pinned to
  pre-validated addresses, TTL expiry cannot introduce a new address
  mid-request. If the engine's HTTP stack re-resolves for any reason (retry,
  connection pool churn), the resolver hook re-runs validation before
  returning the new address; a failing address aborts the request.

Rationale: blocklists rot and path-based filters (`http://host/../../`) have
historically been bypassed. Deny-by-construction at the resolver layer is the
only approach that survives new bypass techniques. This matches the strategy
document's requirement that SSRF protection be "denial by construction, not
flag discipline."

### 2. Global per-target politeness cap

Per-source rate limits alone permit distributed abuse of one victim: N attack
subnets each within their own limit still DDoS a small site. Therefore:

- A **global, persistent, per-target-host budget** is enforced before any
  request to that host: a sliding-window cap of **20 pages / 10 minutes per
  target host** (configurable downward per deployment, never upward).
- The budget is enforced in **shared state** (Redis or the API's existing
  queue infrastructure), not in per-process memory, so it holds across
  scanner replicas.
- When the budget for a target is exhausted, submissions for that target
  queue behind the budget (HTTP 429 with `Retry-After`) rather than silently
  scanning a truncated sample. Concurrent distinct-submitter requests for the
  same host share the budget and queue.
- Per-source limits (IP-based token bucket) remain in force **in addition**,
  sized so one source cannot monopolize a target's budget.

Rationale: this is the strategy document's explicit requirement ("per-IP
limits alone permit distributed abuse of one victim"). The cost — a popular
target serializes across all submitters — is acceptable for a teaser surface.

### 3. Hard crawl bounds

- **≤ 25 pages** per scan (hard ceiling enforced in the engine config, not by
  politeness). Depth ≤ 4. One URL per submission. No cross-domain links are
  followed beyond the submitted host.
- **≤ 30 s wall-clock** per scan and ≤ 512 MB memory per scan worker; the
  worker is killed at the bound and the partial result is returned with an
  explicit truncation notice.
- **No plugin execution.** WASM plugins are disabled in the scanner
  configuration; first-party analyzers only. This keeps the scanner's attack
  surface out of the plugin sandbox threat model (ADR-003) and keeps scan
  latency bounded.
- **Read-only semantics.** No form submission, no cookie jar persistence
  across scans, no state kept about the target between scans beyond the
  politeness budget.

### 4. Data handling and abuse response

- Scan results are retained for **24 hours** and then deleted; no account
  linkage is possible because there are no accounts. Results are addressable
  by an unguessable result token (≥ 128 bits of entropy) presented to the
  submitter.
- No target content is persisted beyond the retention window; aggregated
  counters (scans/day, top targets) may be kept without URL-level detail.
- An **abuse channel** (email + automated takedown endpoint) is published in
  the scanner's robots-visible metadata and service headers so target
  operators can request exclusion. A per-target opt-out list (verified via
  the standard webmaster-verification methods crawlkit already supports) is
  honored before any request is issued.
- Submissions are rate-limited per source IP and behind the existing
  infrastructure edge protections (bot detection, ASN throttling) where the
  deployment provides them.

## Consequences

**Positive:**
- The scanner can be marketed as a teaser without misrepresentation: every
  claim ("25-page sample scan") is mechanically enforced, so the claims
  discipline in ROADMAP Phase 0.1 extends to marketing copy.
- The SSRF resolver-pin design is reusable: it becomes the reference
  implementation for any future crawlkit feature that crawls user-supplied
  URLs (e.g., the trends UI's per-URL drilldown).
- The global per-target budget is a precondition for ever exposing
  scheduling of user-submitted crawls in the hosted product.

**Negative:**
- A separate service and shared budget store add deployment surface that
  must be run and monitored (new on-call scope).
- The 24-hour retention means users cannot revisit old scans; this is a
  deliberate conversion lever, not a defect.
- Resolver-pinned requests bypass the OS resolver's caching; under load this
  shifts DNS load onto the scanner's own resolver infrastructure (mitigation:
  cache validated address sets for the lesser of TTL or 60 s, re-validating
  each cached entry before reuse).

**Neutral:**
- The scanner is out of scope for the engine's determinism guarantees
  (ADR-007): live-web scans are inherently non-reproducible, and the scanner
  does not claim `--seed` semantics.

## Compliance

- **ROADMAP Phase 0 (claims):** All scanner-facing numbers (25 pages, 10 min
  window, 30 s budget, 24 h retention) are constants in the scanner crate,
  surfaced in `docs/capabilities.toml` when the scanner reaches prototype
  status in 5.2.0.
- **ROADMAP Phase 1 (security):** The scanner inherits the threat model
  review; the resolver-pin and budget-store designs must be reviewed under
  the Phase 1 security audit before any public deployment.
- **ADR triggers scheduled in PRODUCT_STRATEGY §7:** This ADR discharges the
  "hosted scanner trust boundary" trigger. The remaining trigger for scanner
  GA (5.3.0) is the secrets/rotation hardening (Phase 1.4), which the
  scanner's abuse-channel endpoint will consume.
- **Truth baseline:** `docs/capabilities.toml` gains a
  `[capabilities.hosted_scanner]` entry with status `prototype` only when the
  5.2.0 prototype exists; it must not be added speculatively.
