# ADR-014: Alert Channels (Slack, Teams, Email) over Loop-Retry Webhook Delivery

**Status:** Accepted (implementation gated by 5.3.0)
**Date:** 2026-09-11
**Deciders:** Product strategy (docs/PRODUCT_STRATEGY.md §3, §7), maintainer approval
**Related:** ADR-008 (API backpressure and idempotency), docs/PRODUCT_STRATEGY.md §7 (decision-log trigger: "Alert channel providers — new external services"), docs/ROADMAP.md Phase 3.4 (contract tests), Phase 5.1 (connector-failure metrics)

## Context

The API already delivers webhook notifications with production-grade retry
semantics: `loop_retry::with_backoff` with `IsRetryable` classification that
distinguishes transport errors and non-success statuses (retryable) from
non-retryable failures, configured via `RetryConfig`
(`crates/crawlkit-api/src/handlers/webhooks.rs`).

Alerting to humans, however, currently has no surface. Operators learn of a
failed or degraded crawl by watching dashboards. The strategy's F2 item asks
for Slack, Teams, and email alert channels. The open design question is
whether alerting needs its own delivery pipeline.

It does not. A Slack incoming-webhook POST, a Teams Power Automate connector
POST, and an SMTP submission are all "deliver a payload to an external
endpoint with retry on transient failure" — exactly what the webhook
subsystem already does. Three differences do need decisions:

1. **Payload shape.** Alert consumers are humans and chat platforms, not the
   crawl-results API clients. Payloads must be provider-native (Slack Block
   Kit, Teams MessageCard, plain-text email) and derived from the same
   internal event source as webhooks — one event model, multiple renderers.
2. **Failure visibility.** Phase 5.1 requires connector-failure metrics. A
   silently dead Slack webhook is worse than none: operators believe alerts
   flow while the channel is broken.
3. **Credential surface.** Each channel introduces another secret (webhook
   URL, SMTP password). These fall under the same 5.3.0 secrets-hardening
   contract as connector credentials (ADR-013).

## Decision

Implement alert channels as **payload renderers plus endpoint adapters over
the existing loop-retry delivery pipeline**, not as a new delivery system.

### 1. One event source, three renderers

- Alert events are the same internal events that drive webhooks (crawl
  completed, crawl failed, threshold crossed on findings). A single
  event-emission point fans out to all configured subscribers.
- Each channel type implements a `AlertRenderer` producing provider-native
  payloads from the event: Slack Block Kit JSON, Teams MessageCard JSON,
  RFC 5322 email. Renderers are pure functions — trivially unit-testable,
  no I/O.
- Rendering failures are programming errors (schema drift) and are caught by
  contract tests per Phase 3.4, not by runtime retry.

### 2. Delivery reuses loop-retry semantics

- Endpoint adapters submit the rendered payload through
  `loop_retry::with_backoff` with the same `IsRetryable` classification as
  webhook delivery: transport errors and 5xx/429 retryable; 4xx (except 429)
  fatal and surfaced immediately.
- Email differs in transport only: SMTP submission via a dedicated
  transport module (subprocess-free, TLS-first). SMTP transient failures
  (4xx replies) map to retryable; 5xx permanent failures to fatal.
- Delivery attempts, outcomes, and terminal failures are recorded per
  channel for the Phase 5.1 connector-failure metrics; a channel with
  repeated terminal failures raises a health signal rather than decaying
  silently.

### 3. Credential handling

- Slack/Teams webhook URLs and SMTP credentials are per-tenant secrets,
  stored through the same encrypted credential store as connector
  credentials (ADR-013 §2), never logged (verified by the shared
  no-secret-logging test suite).
- Channel configuration references secrets by ID; API responses never echo
  credential material (redaction pattern reused from API-key handling).

### 4. Explicit non-goals

- No bidirectional integration (no reading Slack threads, no inbox
  processing); channels are outbound-only.
- No custom alert-scripting/DSL; thresholds are configuration, not code.
- No per-finding real-time streaming into chat; alerts summarize at event
  boundaries with the capped summary surface shared with the scanner
  contract.

## Consequences

- No new retry/backoff code paths to maintain; alert delivery inherits the
  tested webhook semantics, and Phase 3.4 contract tests extend to the three
  renderers and adapters.
- The event-fanout point becomes a shared dependency of webhooks and alerts;
  its contract (event ordering, at-least-once delivery) needs the same
  documentation and test discipline as queue semantics (ADR-015), since
  6.0.0's queue graduation will carry alert events too.
- Email introduces the first non-HTTP transport into the delivery pipeline;
  the retryable/fatal classification must be defined for SMTP reply codes at
  implementation time and covered in contract tests.
