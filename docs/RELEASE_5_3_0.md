# crawlkit 5.3.0 "Connectors" — Release Highlights

**Released:** 2026-09-12 · **Full change log:** [CHANGELOG.md](../CHANGELOG.md)

5.3.0 turns crawlkit from a crawler with APIs into a crawler that plugs into
the services SEO teams already use — with a security posture that treats
every credential as if the release notes will be read by an attacker
(because they might be).

## Headline: the connectors ladder, completed

All three 5.3.0 ladder items (docs/PRODUCT_STRATEGY.md §3) shipped in this
release cycle:

1. **GA4 Data API connector** (ADR-013) — read-only engagement reporting
   over per-tenant OAuth2. The `analytics.readonly` scope is pinned by
   test; refresh tokens live only in the encrypted credential store;
   access tokens are memory-only and redacted in `Debug` so an accidental
   `{:?}` log line cannot leak secrets.

   ```
   POST /api/v1/integrations/ga4/exchange      # one-time auth code → tokens
   POST /api/v1/integrations/ga4/credentials   # store refresh token
   GET  /api/v1/integrations/ga4/status        # connection state
   POST /api/v1/integrations/ga4/report        # runReport summaries
   ```

2. **Slack + Teams alert channels** (ADR-014) — provider-native renderers
   (Slack Block Kit, Teams MessageCard) delivered over the same
   retry/backoff pipeline as webhooks. Crawl completion and failure fan
   out to channels and webhooks alike.

3. **Per-tenant credential store** (ADR-013 §2) — AES-256-GCM at rest,
   tenant-scoped lookups, audited disconnect, two-phase key rotation.
   Alert-channel webhook URLs and GA4 refresh tokens share it.

## The credential rule

One design rule repeated across every connector: **secrets are stored
exactly once, in the encrypted credential store, and are never echoed,
logged, or embedded in errors.** Slack/Teams webhook URLs grant posting
rights and are treated as credentials; `reqwest` error strings embed
request URLs, so transport errors are scrubbed before materialization —
pinned by tests. A dead webhook is never silent either: per-channel
health tracking escalates repeated failures to an error-level signal.

## Ops-facing security hardening

- **Deterministic release SBOMs** — `cargo-cyclonedx` runs under
  `SOURCE_DATE_EPOCH`, so SBOM bytes are reproducible and a re-uploaded
  asset cannot silently mismatch the signed checksum manifest.
- **Self-healing release publish** — the asset upload tolerates the
  `action-gh-release` metadata race and the verify step re-uploads
  anything missing before publishing; a transient CI failure can no
  longer strand an incomplete release.

## Also in 5.3.0

- **CrUX integration stabilized** (graduated from experimental).
- **Per-tenant data retention** — deletion windows are tenant-scoped.
- Full semver-checked upgrade from 5.2.0; 18 release assets with a
  signed checksum manifest and per-crate CycloneDX SBOMs.

## Upgrading

- No breaking API changes; the release is semver-verified against 5.2.0.
- Set `CRAWLKIT_ENCRYPTION_KEY` to enable the credential store; without
  a key, connectors are explicitly unavailable rather than storing
  plaintext.
- Alert channels and GA4 are configured per tenant through the API; see
  [INTEGRATIONS.md](INTEGRATIONS.md) for the full contract.

## What's next

The 5.4.0 line graduates the Redis queue (ADR-015): lease-based
at-least-once delivery, bounded retries, poison quarantine with an
operator dead-letter surface, and the shared-state politeness budget the
hosted scanner's GA requires. The implementation is underway on main.
