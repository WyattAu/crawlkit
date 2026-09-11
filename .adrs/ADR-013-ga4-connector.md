# ADR-013: GA4 Connector (Read-Only, Per-Tenant OAuth)

**Status:** Accepted (implementation gated by 5.3.0; release per ROADMAP Phase 1.4)
**Date:** 2026-09-11
**Deciders:** Product strategy (docs/PRODUCT_STRATEGY.md §3, §7), maintainer approval
**Related:** ADR-002 (enterprise authentication), ADR-008 (API backpressure), docs/PRODUCT_STRATEGY.md §7 (decision-log trigger: "GA4 connector — new external service and credential"), docs/INTEGRATIONS.md (GSC precedent)

## Context

The 5.3.0 "Connectors" release pairs crawl findings with traffic data. GSC
integration (promoted to stable in 5.2.0) covers search-console queries; the
remaining gap is behavioral analytics. GA4 is the largest source and is
explicitly scheduled as "F1 part 2" in the strategy.

Three constraints shape this decision:

1. **A new external credential class is introduced.** Unlike GSC today — whose
   access token is operator-supplied via `GSC_ACCESS_TOKEN` and never stored —
   OAuth2 requires a long-lived **refresh token** per tenant that must be
   persisted. Persistence changes the threat model: a database leak now
   exposes third-party data access, not just crawl results.
2. **Secrets hardening is the blocking gate.** The strategy sequences
   per-tenant credential storage (rotation, no-secret logging) ahead of every
   connector. This ADR consumes that work; it does not duplicate it.
3. **Read-only by construction.** crawlkit only ever reads analytics data.
   Scopes must be narrowed to the read-only set so a compromised credential
   cannot mutate customer analytics.

Existing primitives this ADR builds on:

- `EncryptionManager` (engine `encryption.rs`): AES-GCM at rest with
  two-phase `rotate_key` / `complete_rotation`.
- `GscClient` (engine `gsc.rs`): the error-contract and token-hygiene
  patterns promoted in 5.2.0 — injectable base URL, hermetic TCP-stub tests,
  token never present in error output.

## Decision

Add a **GA4 Data API v1beta connector**, read-only, with per-tenant OAuth2
credentials stored encrypted at rest, under the following contract:

### 1. OAuth2 authorization-code flow, minimal scopes

- Requested scope: `https://www.googleapis.com/auth/analytics.readonly` and
  nothing else. No write, no admin, no user scopes.
- The client exchanges the authorization code once to obtain the refresh
  token; thereafter it mints short-lived access tokens via
  `refresh_token` grants. Access tokens live in memory only and are never
  persisted or logged (same hygiene contract as GSC, enforced by test).
- Token exchange and refresh happen against a base URL that is injectable in
  the same style as `GscClient::with_base_url`, so contract tests run against
  a hermetic HTTP stub with no Google dependency in CI.

### 2. Per-tenant credential storage

- Refresh tokens are stored **only** through the per-tenant credential store
  delivered by the 5.3.0 secrets-hardening item, encrypted with
  `EncryptionManager` (AES-GCM). The store, not the connector, owns
  encryption, key rotation, and deletion on disconnect.
- Key identifiers in configuration (property ID, tenant ID) are
  non-secret and may appear in logs; token material, client secrets, and
  authorization headers must never appear in logs, errors, or exports. This
  is verified by the secrets-hardening suite (log-capture assertion), not by
  convention.

### 3. Connector surface

- Query surface mirrors GSC's: a `Ga4Client` with typed requests over the
  `runReport` endpoint (dimensions/metrics/date ranges), returning
  strongly-typed rows. The first consumer is the same fusion path GSC feeds
  (findings joined with engagement/traffic data for report generation).
- Error contract follows the GSC promotion precedent: typed
  `ApiError { status, body }`, `RequestFailed`, `ParseError` — connection
  failures, HTTP error statuses, and malformed responses are each a distinct,
  tested path with no panics.
- Rate limiting is respected via the documented GA4 Data API quotas
  (tokens-per-property and concurrent-requests); the client surfaces
  retryable-vs-fatal classification compatible with `loop_retry::IsRetryable`
  so callers get backoff semantics for free.

### 4. Explicit non-goals

- No write APIs, no user-property access, no GA4 Admin API.
- No cross-tenant credential sharing; each tenant's grant is isolated.
- No client-side analytics ingestion; crawlkit reads GA4, it is not gauged by it.

## Consequences

- The database becomes a holder of third-party credentials; the 5.3.0
  secrets-hardening acceptance criteria (never in logs/errors/exports —
  tested) become release-blocking for this connector.
- Key rotation must account for encrypted refresh tokens: rotation via
  `EncryptionManager::rotate_key` re-encrypts stored credentials; the
  two-phase flow prevents decrypt failures during rollover.
- Operational dependency on Google's token endpoints is added; connector
  failures must surface through the Phase 5.1 connector-failure metrics
  (shared with alert channels, ADR-014) rather than failing silently.
