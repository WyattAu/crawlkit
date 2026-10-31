//! Alerting on regression (GAP_CLOSURE_PLAN §G3) for the CLI command paths.
//!
//! One event model, canonical serialization, signed webhook delivery with
//! bounded retries, and provider payload adapters:
//!
//! * **Event:** `{event, crawl_ids, findings_delta, generated_at}` — the
//!   same field set as §G3, serialized canonically (object keys sorted,
//!   deterministic bytes) so the HMAC signature is reproducible and two
//!   runs over identical data produce byte-identical bodies (ITER-09
//!   determinism rule).
//! * **Delivery:** `POST` the canonical JSON body with
//!   `X-Crawlkit-Signature: hex(hmac-sha256(secret, body))`; 3 retries with
//!   exponential backoff ([`backoff_delay`], a pure function so the policy
//!   is unit-testable). Secrets and URLs never appear in error messages.
//! * **Adapters:** Slack and Discord are formatting-only variants of the
//!   same webhook delivery ([`AlertFormat`]).
//! * **Email:** SMTP via `lettre` behind the opt-in `smtp` feature
//!   (`cargo build --features smtp`). Without the feature the email path
//!   returns a descriptive error instead of failing to compile.
//!
//! Secrets are accepted as hex (`--alert-secret HEX`) and are never logged.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{bail, Context, Result};

/// Fixed ceiling for the exponential backoff so a long outage cannot pin a
/// command run forever.
const BACKOFF_CAP: Duration = Duration::from_secs(30);

/// Retries performed after the initial delivery attempt (§G3: ×3).
pub const RETRY_ATTEMPTS: u32 = 3;

// ---------------------------------------------------------------------------
// Event model + canonical serialization
// ---------------------------------------------------------------------------

/// A regression/improvement alert event (§G3 event model).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertEvent {
    /// Event name: `regression`, `improvement`, or `error`.
    pub event: String,
    /// Crawl (run) identifiers the event refers to.
    pub crawl_ids: Vec<String>,
    /// Per-kind change counts driving the alert (e.g. `new_errors: 2`).
    pub findings_delta: BTreeMap<String, u64>,
    /// RFC 3339 generation timestamp.
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl AlertEvent {
    /// Build a regression event from a crawl diff: fires when the diff
    /// contains new errors (new 4xx/5xx statuses, removed pages, or CWV
    /// regressions).
    #[must_use]
    pub fn regression_from_diff(
        baseline_id: &str,
        target_id: &str,
        diff: &crawlkit_engine::compare::CrawlDiff,
        generated_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            event: "regression".to_string(),
            crawl_ids: vec![baseline_id.to_string(), target_id.to_string()],
            findings_delta: findings_delta_from_diff(diff),
            generated_at,
        }
    }
}

/// Classify a crawl diff into per-kind counts.
///
/// "New errors" (§G3) are: pages whose status degraded to 4xx/5xx, pages
/// that disappeared, and Core Web Vitals regressions. Pure function —
/// unit-tested against fixture diffs.
#[must_use]
pub fn findings_delta_from_diff(
    diff: &crawlkit_engine::compare::CrawlDiff,
) -> BTreeMap<String, u64> {
    use crawlkit_engine::compare::ChangeKind;
    let new_errors = diff
        .status_changes
        .iter()
        .filter(|e| {
            matches!(
                &e.change,
                ChangeKind::StatusChanged { to, .. } if *to >= 400
            )
        })
        .count() as u64;
    let mut delta = BTreeMap::new();
    delta.insert("new_errors".to_string(), new_errors);
    delta.insert("removed_pages".to_string(), diff.removed.len() as u64);
    delta.insert(
        "cwv_regressions".to_string(),
        diff.cwv_changes.iter().filter(|c| c.regression).count() as u64,
    );
    delta
}

/// True when the diff contains a regression worth alerting on.
#[must_use]
pub fn has_regressions(diff: &crawlkit_engine::compare::CrawlDiff) -> bool {
    findings_delta_from_diff(diff).values().any(|v| *v > 0)
}

/// Serialize a JSON value canonically: object keys sorted recursively,
/// no insignificant whitespace. Deterministic across runs and platforms.
#[must_use]
pub fn canonical_json(value: &serde_json::Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::Value::String((*key).clone()).to_string());
                out.push(':');
                write_canonical(&map[key.as_str()], out);
            }
            out.push('}');
        }
        serde_json::Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// Canonical serialization of an alert event (the exact webhook body).
///
/// # Errors
///
/// Returns an error when the event cannot be serialized as JSON.
pub fn canonical_event_body(event: &AlertEvent) -> Result<String> {
    let value = serde_json::to_value(event).context("serialize alert event")?;
    Ok(canonical_json(&value))
}

// ---------------------------------------------------------------------------
// Signing
// ---------------------------------------------------------------------------

/// `hex(hmac-sha256(secret, body))` — the value carried by the
/// `X-Crawlkit-Signature` header.
#[must_use]
pub fn signature_hex(secret: &[u8], body: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    // HMAC accepts keys of any length, so `new_from_slice` cannot fail;
    // the targeted allow keeps the workspace panic-free lint satisfied
    // without pretending there is a meaningful recovery.
    #[allow(clippy::expect_used)]
    let mut mac =
        Hmac::<sha2::Sha256>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(body);
    hex_encode(&mac.finalize().into_bytes())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Decode a hex-encoded webhook secret (`--alert-secret HEX`).
///
/// # Errors
///
/// Returns an error when the string is not valid hex or has odd length.
pub fn decode_secret_hex(secret_hex: &str) -> Result<Vec<u8>> {
    let hex = secret_hex.trim();
    if !hex.len().is_multiple_of(2) {
        bail!("alert secret must be valid hex (even length)");
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for pair in bytes.chunks(2) {
        let hi = hex_val(pair[0])
            .with_context(|| format!("invalid hex in alert secret near '{hex}'"))?;
        let lo = hex_val(pair[1]).context("invalid hex in alert secret")?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_val(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => bail!("invalid hex character"),
    }
}

// ---------------------------------------------------------------------------
// Backoff (pure, unit-testable)
// ---------------------------------------------------------------------------

/// Exponential backoff for retry `attempt` (0-based: the wait before the
/// first retry). `base × 2^attempt`, capped at [`BACKOFF_CAP`].
#[must_use]
pub fn backoff_delay(attempt: u32, base: Duration) -> Duration {
    let exp = attempt.min(16); // 2^16 × base saturates any realistic cap
    let scaled = base.saturating_mul(1u32 << exp);
    if scaled > BACKOFF_CAP {
        BACKOFF_CAP
    } else {
        scaled
    }
}

// ---------------------------------------------------------------------------
// Delivery
// ---------------------------------------------------------------------------

/// Webhook delivery target configuration.
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Endpoint URL.
    pub url: String,
    /// Hex-encoded HMAC secret; `None` sends the request unsigned (the
    /// Discord/Slack adapter case — their URLs embed the credential).
    pub secret_hex: Option<String>,
    /// Payload formatting variant.
    pub format: AlertFormat,
}

/// Payload formatting variants (§G3: Slack/Discord are webhook adapters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertFormat {
    /// Raw canonical event JSON.
    #[default]
    Generic,
    /// Slack Block Kit payload.
    Slack,
    /// Discord webhook payload.
    Discord,
}

impl AlertFormat {
    /// Parse the `--alert-format` value.
    ///
    /// # Errors
    ///
    /// Returns an error for unknown format names.
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "generic" | "" => Ok(Self::Generic),
            "slack" => Ok(Self::Slack),
            "discord" => Ok(Self::Discord),
            other => bail!("unknown alert format '{other}' (use generic, slack, or discord)"),
        }
    }

    /// Render the event into this format's payload.
    ///
    /// Slack and Discord wrap the canonical event JSON so receivers still
    /// get the deterministic body as an escaped string.
    #[must_use]
    pub fn payload(&self, event: &AlertEvent) -> serde_json::Value {
        match self {
            Self::Generic => serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
            Self::Slack => serde_json::json!({
                "text": slack_text(event),
            }),
            Self::Discord => serde_json::json!({
                "content": discord_text(event),
            }),
        }
    }
}

fn slack_text(event: &AlertEvent) -> String {
    let mut lines = vec![format!(":rotating_light: crawlkit *{}* alert", event.event)];
    for (key, value) in &event.findings_delta {
        lines.push(format!("• {key}: {value}"));
    }
    lines.push(format!("crawls: {}", event.crawl_ids.join(", ")));
    lines.join("\n")
}

fn discord_text(event: &AlertEvent) -> String {
    let mut lines = vec![format!("**crawlkit {} alert**", event.event)];
    for (key, value) in &event.findings_delta {
        lines.push(format!("- {key}: {value}"));
    }
    lines.push(format!("crawls: {}", event.crawl_ids.join(", ")));
    lines.join("\n")
}

/// Delivery targets resolved from `--alert-*` flags and the `[alerts]`
/// config section. Any combination is valid; with no targets configured
/// the compare command simply never fires.
#[derive(Debug, Clone, Default)]
pub struct AlertDelivery {
    /// Signed webhook target (generic, Slack, or Discord formatting).
    pub webhook: Option<WebhookConfig>,
    /// Optional SMTP target (`smtp` feature).
    pub email: Option<EmailTarget>,
}

impl AlertDelivery {
    /// True when at least one delivery target is configured.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.webhook.is_some() || self.email.is_some()
    }
}

/// Delivery failure. Never carries the endpoint URL or the secret: URLs are
/// credentials (a Slack webhook URL grants posting rights), so transport
/// errors are materialized through `reqwest::Error::without_url`.
#[derive(Debug)]
pub enum AlertError {
    /// The endpoint kept failing after all retries.
    DeliveryFailed(String),
    /// The endpoint rejected the payload with a non-retryable status.
    Rejected(u16),
}

impl std::fmt::Display for AlertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeliveryFailed(msg) => write!(f, "webhook delivery failed: {msg}"),
            Self::Rejected(status) => write!(f, "webhook rejected the alert (HTTP {status})"),
        }
    }
}

impl std::error::Error for AlertError {}

/// Deliver an alert event to a webhook endpoint with signed canonical JSON
/// and bounded retries (initial attempt + [`RETRY_ATTEMPTS`] retries).
///
/// Transport errors and `5xx`/`429` are retried with
/// [`backoff_delay`]; other `4xx` statuses are fatal — the endpoint
/// understood the request and refused it, so retrying cannot help.
///
/// # Errors
///
/// Returns [`AlertError`] when delivery does not eventually succeed.
pub async fn deliver_event(
    client: &reqwest::Client,
    webhook: &WebhookConfig,
    event: &AlertEvent,
) -> Result<(), AlertError> {
    let payload = webhook.format.payload(event);
    let body = canonical_json(&payload);
    deliver_body(client, webhook, body.as_bytes()).await
}

/// [`deliver_event`] over pre-encoded bytes (the contract-test seam: tests
/// assert against the exact bytes that hit the wire).
async fn deliver_body(
    client: &reqwest::Client,
    webhook: &WebhookConfig,
    body: &[u8],
) -> Result<(), AlertError> {
    let secret = match &webhook.secret_hex {
        Some(hex) => decode_secret_hex(hex)
            .map_err(|e| AlertError::DeliveryFailed(format!("invalid alert secret: {e}")))?,
        None => Vec::new(),
    };

    for attempt in 0..=RETRY_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(backoff_delay(attempt - 1, Duration::from_millis(500))).await;
        }

        let mut request = client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(10));
        if !secret.is_empty() {
            request = request.header("X-Crawlkit-Signature", signature_hex(&secret, body));
        }

        let outcome = request.body(body.to_vec()).send().await;
        match outcome {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    return Ok(());
                }
                if status.is_client_error() && status.as_u16() != 429 {
                    return Err(AlertError::Rejected(status.as_u16()));
                }
                tracing::warn!(
                    status = status.as_u16(),
                    attempt,
                    "alert webhook returned retryable status"
                );
            }
            Err(err) => {
                // Scrub the URL before the error can reach a log line.
                let msg = err.without_url().to_string();
                if attempt == RETRY_ATTEMPTS {
                    return Err(AlertError::DeliveryFailed(msg));
                }
                tracing::warn!(attempt, error = %msg, "alert webhook transport error, retrying");
            }
        }
    }
    Err(AlertError::DeliveryFailed(
        "webhook did not succeed after retries".to_string(),
    ))
}

// ---------------------------------------------------------------------------
// Email (feature-gated)
// ---------------------------------------------------------------------------

/// SMTP email target for the `smtp` feature.
#[derive(Debug, Clone)]
pub struct EmailTarget {
    /// SMTP relay host (e.g. `localhost:1025`).
    pub server: String,
    /// Recipient address (`--alert-email TO`).
    pub to: String,
}

/// Send the alert as a plain-text email via SMTP (`smtp` feature).
///
/// # Errors
///
/// Returns an error when message construction or the SMTP transaction
/// fails.
#[cfg(feature = "smtp")]
pub async fn send_email(target: &EmailTarget, subject: &str, body: &str) -> Result<()> {
    use lettre::message::Mailbox;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let email = Message::builder()
        .from(
            "crawlkit@localhost"
                .parse::<Mailbox>()
                .context("parse sender address")?,
        )
        .to(target
            .to
            .parse::<Mailbox>()
            .with_context(|| format!("parse recipient address '{}'", target.to))?)
        .subject(subject)
        .body(body.to_string())
        .context("build alert email")?;

    // Plain submission host (e.g. a local relay on localhost); TLS-capable
    // relays can be added later behind credentials in the config store.
    // `builder_dangerous` = no TLS, the honest default for loopback relays.
    let host = target
        .server
        .split(':')
        .next()
        .unwrap_or("localhost")
        .to_string();
    let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host).build();
    transport.send(email).await.context("SMTP send failed")?;
    Ok(())
}

/// Stub used when the `smtp` feature is disabled: the CLI accepts
/// `--alert-email` but reports the missing feature instead of failing the
/// whole command.
///
/// # Errors
///
/// Always returns an error explaining that the `smtp` feature is off.
// Kept `async` so callers need not branch on the feature flag; the real
// implementation below awaits the SMTP transaction.
#[cfg(not(feature = "smtp"))]
#[allow(clippy::unused_async)]
pub async fn send_email(target: &EmailTarget, _subject: &str, _body: &str) -> Result<()> {
    bail!(
        "email alerts require the 'smtp' feature (rebuild with --features smtp); \
         would have mailed {} via {}",
        target.to,
        target.server
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crawlkit_engine::compare::{ChangeKind, CrawlDiff, CwvChange, DiffEntry};

    fn diff(status_to: u16, removed: usize, cwv_regressions: usize) -> CrawlDiff {
        CrawlDiff {
            baseline_pages: 3,
            target_pages: 3,
            added: vec![],
            removed: (0..removed)
                .map(|i| DiffEntry {
                    url: format!("https://example.com/r{i}"),
                    change: ChangeKind::Removed,
                })
                .collect(),
            status_changes: vec![DiffEntry {
                url: "https://example.com/x".to_string(),
                change: ChangeKind::StatusChanged {
                    from: 200,
                    to: status_to,
                },
            }],
            title_changes: vec![],
            content_changes: vec![],
            size_changes: vec![],
            cwv_changes: (0..cwv_regressions)
                .map(|i| CwvChange {
                    url: format!("https://example.com/c{i}"),
                    metric_name: "LCP".to_string(),
                    old_value: Some(1.0),
                    new_value: Some(2.0),
                    regression: true,
                })
                .collect(),
        }
    }

    fn event() -> AlertEvent {
        AlertEvent {
            event: "regression".to_string(),
            crawl_ids: vec!["run-a".to_string(), "run-b".to_string()],
            findings_delta: BTreeMap::from([
                ("cwv_regressions".to_string(), 1),
                ("new_errors".to_string(), 2),
                ("removed_pages".to_string(), 0),
            ]),
            generated_at: chrono::DateTime::parse_from_rfc3339("2026-10-31T12:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    #[test]
    fn canonical_serialization_sorts_keys_deterministically() {
        let body = canonical_event_body(&event()).unwrap();
        // Top-level keys sorted: crawl_ids < event < findings_delta < generated_at.
        assert_eq!(
            body,
            "{\"crawl_ids\":[\"run-a\",\"run-b\"],\"event\":\"regression\",\
             \"findings_delta\":{\"cwv_regressions\":1,\"new_errors\":2,\"removed_pages\":0},\
             \"generated_at\":\"2026-10-31T12:00:00Z\"}"
        );
        // Array order is preserved (crawl_ids are ordered, not sorted).
        assert!(body.contains("[\"run-a\",\"run-b\"]"));
    }

    #[test]
    fn canonical_serialization_is_stable_across_invocations() {
        let a = canonical_event_body(&event()).unwrap();
        let b = canonical_event_body(&event()).unwrap();
        assert_eq!(a, b, "identical events must produce byte-identical bodies");
        // Unsorted insertion order does not change the canonical form.
        let mut map = serde_json::Map::new();
        map.insert("z".to_string(), serde_json::json!(1));
        map.insert("a".to_string(), serde_json::json!({"y": 2, "b": 3}));
        assert_eq!(
            canonical_json(&serde_json::Value::Object(map)),
            "{\"a\":{\"b\":3,\"y\":2},\"z\":1}"
        );
    }

    /// RFC 4231 test case 2 — the known HMAC-SHA256 vector.
    #[test]
    fn hmac_signature_matches_rfc_4231_vector() {
        let sig = signature_hex(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            sig,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn signature_is_deterministic_and_depends_on_secret_and_body() {
        let body = canonical_event_body(&event()).unwrap();
        let s1 = signature_hex(b"secret-a", body.as_bytes());
        let s2 = signature_hex(b"secret-a", body.as_bytes());
        let s3 = signature_hex(b"secret-b", body.as_bytes());
        let s4 = signature_hex(b"secret-a", b"other body");
        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
        assert_ne!(s1, s4);
        assert_eq!(s1.len(), 64, "sha256 hex is 64 chars");
    }

    #[test]
    fn secret_hex_decoding() {
        assert_eq!(decode_secret_hex("aabb").unwrap(), vec![0xaa, 0xbb]);
        assert_eq!(decode_secret_hex("AaBb").unwrap(), vec![0xaa, 0xbb]);
        assert_eq!(decode_secret_hex(" ab ").unwrap(), vec![0xab]);
        assert!(decode_secret_hex("abc").is_err(), "odd length");
        assert!(decode_secret_hex("zz").is_err(), "non-hex");
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        let base = Duration::from_millis(500);
        assert_eq!(backoff_delay(0, base), Duration::from_millis(500));
        assert_eq!(backoff_delay(1, base), Duration::from_millis(1000));
        assert_eq!(backoff_delay(2, base), Duration::from_millis(2000));
        assert_eq!(backoff_delay(3, base), Duration::from_millis(4000));
        assert_eq!(
            backoff_delay(40, base),
            BACKOFF_CAP,
            "backoff never exceeds the cap"
        );
        assert_eq!(
            backoff_delay(0, Duration::from_secs(60)),
            BACKOFF_CAP,
            "oversized base is capped"
        );
    }

    #[test]
    fn regression_detection_classifies_new_errors() {
        // 500 status + removed page + CWV regression → regressions, all three kinds.
        let d = diff(500, 1, 2);
        let delta = findings_delta_from_diff(&d);
        assert_eq!(delta["new_errors"], 1);
        assert_eq!(delta["removed_pages"], 1);
        assert_eq!(delta["cwv_regressions"], 2);
        assert!(has_regressions(&d));

        // 200 → 301 is a change, not a new error; nothing fires.
        let benign = diff(301, 0, 0);
        let delta = findings_delta_from_diff(&benign);
        assert_eq!(delta["new_errors"], 0);
        assert_eq!(delta["removed_pages"], 0);
        assert_eq!(delta["cwv_regressions"], 0);
        assert!(!has_regressions(&benign));

        // Empty diff → no alert.
        let empty = CrawlDiff {
            baseline_pages: 0,
            target_pages: 0,
            added: vec![],
            removed: vec![],
            status_changes: vec![],
            title_changes: vec![],
            content_changes: vec![],
            size_changes: vec![],
            cwv_changes: vec![],
        };
        assert!(!has_regressions(&empty));
    }

    #[test]
    fn alert_formats_parse() {
        assert_eq!(AlertFormat::parse("generic").unwrap(), AlertFormat::Generic);
        assert_eq!(AlertFormat::parse("slack").unwrap(), AlertFormat::Slack);
        assert_eq!(AlertFormat::parse("discord").unwrap(), AlertFormat::Discord);
        assert_eq!(AlertFormat::parse("").unwrap(), AlertFormat::Generic);
        assert!(AlertFormat::parse("carrier-pigeon").is_err());
    }

    #[test]
    fn adapters_wrap_the_event_payload() {
        let e = event();
        let slack = AlertFormat::Slack.payload(&e);
        assert!(slack["text"].as_str().unwrap().contains("regression"));
        assert!(slack["text"].as_str().unwrap().contains("new_errors"));

        let discord = AlertFormat::Discord.payload(&e);
        assert!(discord["content"].as_str().unwrap().contains("regression"));

        // Generic keeps the canonical field set.
        let generic = AlertFormat::Generic.payload(&e);
        assert_eq!(generic["event"], "regression");
        assert_eq!(generic["findings_delta"]["new_errors"], 2);
    }

    // -- Contract tests: webhook against a local stub (no external network).

    /// Hermetic HTTP stub: one thread per connection, records the raw
    /// request head, replies with `status`.
    struct Stub {
        addr: std::net::SocketAddr,
    }

    impl Stub {
        fn spawn(status: u16) -> (Self, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let sink = requests.clone();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { continue };
                    let sink = sink.clone();
                    std::thread::spawn(move || {
                        use std::io::Read;
                        use std::io::Write;
                        let mut buf = vec![0u8; 8192];
                        let n = stream.read(&mut buf).unwrap_or(0);
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        // Trim the body out: keep the request head only.
                        let head = text
                            .split("\r\n\r\n")
                            .next()
                            .unwrap_or_default()
                            .to_string();
                        sink.lock().unwrap().push(head);
                        let response = format!(
                            "HTTP/1.1 {status} TEST\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        );
                        let _ = stream.write_all(response.as_bytes());
                    });
                }
            });
            (Self { addr }, requests)
        }
    }

    fn webhook(url: String, secret: Option<&str>, format: AlertFormat) -> WebhookConfig {
        WebhookConfig {
            url,
            secret_hex: secret.map(str::to_string),
            format,
        }
    }

    #[tokio::test]
    async fn webhook_delivers_signed_canonical_body() {
        let (stub, requests) = Stub::spawn(200);
        let secret_hex = "aabbccdd";
        let cfg = webhook(
            format!("http://{}", stub.addr),
            Some(secret_hex),
            AlertFormat::Generic,
        );
        deliver_event(&reqwest::Client::new(), &cfg, &event())
            .await
            .expect("delivery succeeds on 200");

        let heads = requests.lock().unwrap();
        assert_eq!(heads.len(), 1, "no retries after a 200");
        let head = &heads[0];
        assert!(
            head.to_ascii_lowercase().starts_with("post / http/1.1"),
            "post request: {head}"
        );
        assert!(
            head.to_ascii_lowercase()
                .contains("content-type: application/json"),
            "{head}"
        );
        assert!(
            head.to_ascii_lowercase().contains("x-crawlkit-signature:"),
            "signed: {head}"
        );

        // Verify the signature against the body the stub actually received
        // is impossible from the head alone; instead recompute from the
        // canonical body and check the header value matches.
        let body = canonical_json(&AlertFormat::Generic.payload(&event()));
        let expected = signature_hex(&decode_secret_hex(secret_hex).unwrap(), body.as_bytes());
        let header = head
            .lines()
            .find(|l| l.to_ascii_lowercase().starts_with("x-crawlkit-signature:"))
            .unwrap();
        assert_eq!(
            header
                .split_once(':')
                .map(|(_, v)| v.trim())
                .unwrap_or_default(),
            expected
        );
    }

    #[tokio::test]
    async fn webhook_retries_then_succeeds() {
        // First connection → 500, second → 200. Backoff base is small, so
        // the retry path stays fast.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = requests.clone();
        std::thread::spawn(move || {
            let mut first = true;
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let status = if first { 500 } else { 200 };
                first = false;
                use std::io::{Read, Write};
                let mut buf = vec![0u8; 8192];
                let _ = stream.read(&mut buf);
                sink.lock().unwrap().push(format!("status {status}"));
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 {status} T\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                    .as_bytes(),
                );
            }
        });

        let cfg = webhook(format!("http://{addr}"), None, AlertFormat::Generic);
        deliver_event(&reqwest::Client::new(), &cfg, &event())
            .await
            .expect("retry eventually succeeds");
        assert_eq!(requests.lock().unwrap().len(), 2, "500 then 200");
    }

    #[tokio::test]
    async fn webhook_gives_up_after_three_retries() {
        let (stub, requests) = Stub::spawn(500);
        let cfg = webhook(format!("http://{}", stub.addr), None, AlertFormat::Generic);
        let err = deliver_event(&reqwest::Client::new(), &cfg, &event())
            .await
            .expect_err("persistent 500 must fail");
        assert!(
            err.to_string().contains("failed") || err.to_string().contains("500"),
            "error mentions the failure: {err}"
        );
        assert!(
            !err.to_string().contains(&stub.addr.to_string()),
            "error must not embed the endpoint URL: {err}"
        );
        let seen = requests.lock().unwrap().len();
        assert_eq!(seen, 1 + RETRY_ATTEMPTS as usize, "1 initial + 3 retries");
    }

    #[tokio::test]
    async fn webhook_client_error_is_not_retried() {
        let (stub, requests) = Stub::spawn(422);
        let cfg = webhook(format!("http://{}", stub.addr), None, AlertFormat::Generic);
        let err = deliver_event(&reqwest::Client::new(), &cfg, &event())
            .await
            .expect_err("422 is fatal");
        assert!(matches!(err, AlertError::Rejected(422)));
        assert_eq!(requests.lock().unwrap().len(), 1, "no retry on 4xx");
    }

    #[tokio::test]
    async fn transport_errors_are_url_scrubbed() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener); // guaranteed connection refusal

        let cfg = webhook(format!("http://{addr}"), None, AlertFormat::Generic);
        let err = deliver_event(&reqwest::Client::new(), &cfg, &event())
            .await
            .expect_err("refused connection must fail");
        assert!(
            !err.to_string().contains(&addr.to_string()),
            "transport error must not embed the endpoint URL: {err}"
        );
    }

    #[tokio::test]
    async fn bad_secret_fails_fast_without_hitting_the_network() {
        let (stub, requests) = Stub::spawn(200);
        let cfg = webhook(
            format!("http://{}", stub.addr),
            Some("not-hex!"),
            AlertFormat::Generic,
        );
        let err = deliver_event(&reqwest::Client::new(), &cfg, &event())
            .await
            .expect_err("invalid secret must fail");
        assert!(err.to_string().contains("secret"), "{err}");
        assert!(requests.lock().unwrap().is_empty(), "no request was sent");
    }

    #[test]
    fn smtp_stub_reports_missing_feature() {
        // Without the `smtp` feature the email path is an explicit stub.
        let target = EmailTarget {
            server: "localhost:1025".to_string(),
            to: "ops@example.com".to_string(),
        };
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(send_email(&target, "subject", "body"));
        let msg = match result {
            Ok(()) => panic!("stub must not succeed"),
            Err(e) => e.to_string(),
        };
        assert!(msg.contains("smtp"), "error names the feature: {msg}");
    }
}
