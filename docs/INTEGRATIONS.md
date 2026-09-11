# Integrations

Status of every external-service integration, kept aligned with the `status` fields in `docs/capabilities.toml`. A capability is called **stable** here only if `capabilities.toml` records it as `stable`, and each section lists the evidence gates that promotion satisfied.

## Google Search Console (stable)

Search-analytics and URL-level performance data fused into crawl workflows.

- **Engine client:** `crates/crawlkit-engine/src/gsc.rs` (`GscClient`)
- **CLI:** `crawlkit gsc --start-date YYYY-MM-DD --end-date YYYY-MM-DD [--dimension query|page|all] [--limit N] [--format json|md]`
- **API base:** `https://searchconsole.googleapis.com` (overridable via `GscClient::with_base_url` for testing only)

### Configuration

| Variable | Required | Purpose |
|---|---|---|
| `GSC_ACCESS_TOKEN` | yes | OAuth2 access token with Search Console scope. Sent as `Authorization: Bearer ...` on every request. |
| `GSC_SITE_URL` | yes | Property URL exactly as registered in GSC (e.g. `https://example.com/` or `sc-domain:example.com`). |

Example:

```bash
export GSC_ACCESS_TOKEN=ya29.xxxxxxxxxxxx
export GSC_SITE_URL=https://example.com/
crawlkit gsc --start-date 2026-08-01 --end-date 2026-08-31 --dimension query --limit 20 --format md
```

### Error handling contract

- `GscError::EnvMissing` — required variables absent; message names the variable, never its value.
- `GscError::RequestFailed` — transport-level failure (DNS, connect, timeout).
- `GscError::ApiError { status, body }` — non-2xx upstream response; status and upstream body surfaced verbatim (token never included).
- `GscError::ParseError` — 2xx response that is not valid JSON or has an unexpected shape; empty `rows` is *not* an error (returns empty analytics).
- Connection failures and malformed responses are covered by tests against a local TCP stub (`gsc_http_error_surfaces_status_and_body`, `gsc_connection_refused_maps_to_request_failed`, `gsc_malformed_json_is_parse_error_not_panic`).

### Token hygiene (tested)

The access token is transmitted exactly once per request, in the `Authorization` header, and never appears in error messages, `Debug` output, or logs. Enforced by `gsc_token_sent_as_bearer_and_absent_from_errors`.

### Documented limits

- GSC data is aggregated, delayed ~2 days, and capped at 25,000 rows per request (the client sends `rowLimit: 25000`, `startRow: 0` — larger result sets are truncated, not paginated).
- "URL inspection" uses a Search Analytics dimension filter; `indexed: true` means the page recorded impressions/clicks in the window — it is not the Index Status API.

## Core Web Vitals / CrUX (optional — with configuration)

Field CWV data from the Chrome UX Report. **Status: stable (2026-09-11).** Two paths:

- `CruxClient` (`crates/crawlkit-engine/src/crux.rs`) calls the CrUX `records:queryRecord` endpoint directly; reads `CRUX_API_KEY` or takes a key explicitly. Returns the five p75 metrics (LCP, CLS, INP, FCP, TTFB); 404 → `Ok(None)` (origin genuinely has no data).
- `CruxAdapter` (`crates/crawlkit-engine/src/rum.rs`, via `crawlkit inspect` under the RUM feature flag) reads `PAGESPEED_API_KEY` and resolves field data through PageSpeed Insights.

Error contract (both paths, tested against hermetic HTTP stubs): HTTP error statuses surface as typed errors with status + body; malformed JSON is a distinct error; connection failures map to `RequestFailed` with no panics.

Token hygiene (pinned by test on both paths): the API key is sent in the `x-goog-api-key` header, never in the URL query string — reqwest error messages embed request URLs, so a query-string key would leak into logs and error output. Keys never appear in error `Display` output.

## Alert channels (experimental — Slack, Teams)

Crawl-lifecycle alerts to human channels per ADR-014 (`.adrs/ADR-014-alert-channels.md`). Slack (Block Kit) and Teams (MessageCard) renderers deliver over the same loop-retry pipeline as webhooks — transport errors and 5xx/429 retry, other 4xx fatal.

- **Endpoints are credentials.** The Slack/Teams webhook URL is stored in the encrypted per-tenant credential store (requires `CRAWLKIT_ENCRYPTION_KEY`) and referenced by connector name in channel config. It is never echoed in API responses, and delivery failures are URL-scrubbed before logging (reqwest error strings embed request URLs).
- **Delivery health.** Each channel tracks consecutive failures, last success/failure, and a sanitized last error, surfaced in `GET /api/v1/alert-channels`; repeated failures escalate to an error-level log signal so a dead webhook is visible, not silent.
- **Events.** `crawl.completed`, `crawl.failed`, `monitoring.alert_triggered` — the same event source as webhooks, fanned out independently.
- **Email/SMTP** is deferred to the 6.0.0 transport decision (new dependency, ADR-014 §4).

## LLM analysis (optional — with configuration)

Bring-your-own-key semantic analysis via `LLM_*` environment variables; see `crates/crawlkit-engine/src/llm_analyzer.rs`. Conditional: behavior depends on provider choice, cost controls, and network egress policy, so it stays `conditional` until those are contractual.

## Hosted scanner (prototype)

Single-URL free-scan service per ADR-012 (`.adrs/ADR-012-hosted-scanner-trust-boundary.md`). Crate `crawlkit-scanner`; not part of self-hosted deployments; GA gated on the ADR §4 checklist (shared-state politeness budget, queue-backed execution, runbook).

---

*Claims policy: statuses here mirror `docs/capabilities.toml`. To change a status, change both, with evidence paths and tests.*
