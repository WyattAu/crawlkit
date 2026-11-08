//! §G3 regression alerts for scheduled runs.
//!
//! When a schedule carries an [`ScheduleAlertsConfig`] and its run's
//! monitoring detects a regression, the server POSTs a canonical,
//! HMAC-signed event:
//!
//! ```json
//! {"crawl_ids":["..."],"event":"regression","findings_delta":{...},"generated_at":"..."}
//! ```
//!
//! The same field set and canonical (sorted-key) serialization as the CLI
//! `compare --alert-webhook` path, so one consumer can verify signatures
//! for both. Retries: initial attempt + 3 with exponential backoff;
//! transport errors and 5xx/429 retry, other 4xx statuses are fatal.
//! Endpoint URLs and secrets never appear in log lines or errors.

use std::collections::BTreeMap;
use std::time::Duration;

use hmac::{Hmac, Mac};

/// Retries performed after the initial delivery attempt (§G3: ×3).
pub const RETRY_ATTEMPTS: u32 = 3;

/// Ceiling for the exponential backoff.
const BACKOFF_CAP: Duration = Duration::from_secs(30);

/// The regression event (§G3 event model, CLI-compatible field set).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegressionEvent {
    /// Event name — always `regression` here.
    pub event: String,
    /// Crawl (run) identifiers: baseline + regressed run.
    pub crawl_ids: Vec<String>,
    /// Per-kind change counts driving the alert.
    pub findings_delta: BTreeMap<String, u64>,
    /// RFC 3339 generation timestamp.
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl RegressionEvent {
    /// Build the event for a monitoring result that tripped its threshold.
    #[must_use]
    pub fn from_monitoring(
        baseline_id: Option<&str>,
        current_id: &str,
        monitoring: &crawlkit_engine::monitoring::MonitoringResult,
        generated_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let mut crawl_ids = Vec::new();
        if let Some(baseline) = baseline_id {
            crawl_ids.push(baseline.to_string());
        }
        crawl_ids.push(current_id.to_string());

        let findings_delta = BTreeMap::from([
            ("removed_pages".to_string(), monitoring.removed_pages as u64),
            ("changed_pages".to_string(), monitoring.changed_pages as u64),
            (
                "cwv_regressions".to_string(),
                monitoring.cwv_regressions as u64,
            ),
        ]);

        Self {
            event: "regression".to_string(),
            crawl_ids,
            findings_delta,
            generated_at,
        }
    }
}

/// Serialize a JSON value canonically: recursive sorted object keys, no
/// insignificant whitespace — deterministic bytes for a stable signature.
#[must_use]
pub fn canonical_json(value: &serde_json::Value) -> String {
    fn write(value: &serde_json::Value, out: &mut String) {
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
                    write(&map[key.as_str()], out);
                }
                out.push('}');
            }
            serde_json::Value::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write(item, out);
                }
                out.push(']');
            }
            other => out.push_str(&other.to_string()),
        }
    }
    let mut out = String::new();
    write(value, &mut out);
    out
}

/// Canonical webhook body for the event.
///
/// # Errors
///
/// Returns an error when the event cannot be serialized.
pub fn canonical_event_body(event: &RegressionEvent) -> Result<String, serde_json::Error> {
    Ok(canonical_json(&serde_json::to_value(event)?))
}

/// `hex(hmac-sha256(secret, body))`.
#[must_use]
pub fn signature_hex(secret: &[u8], body: &[u8]) -> String {
    // HMAC accepts keys of any length, so this cannot fail; the targeted
    // allow keeps the workspace panic-free lint satisfied.
    #[allow(clippy::expect_used)]
    let mut mac =
        Hmac::<sha2::Sha256>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

/// Exponential backoff for retry `attempt` (0-based), capped at 30s.
#[must_use]
pub fn backoff_delay(attempt: u32, base: Duration) -> Duration {
    let exp = attempt.min(16);
    let scaled = base.saturating_mul(1u32 << exp);
    if scaled > BACKOFF_CAP {
        BACKOFF_CAP
    } else {
        scaled
    }
}

fn decode_secret(secret_hex: &str) -> Option<Vec<u8>> {
    hex::decode(secret_hex.trim()).ok()
}

/// POST the canonical body with the signature header, retrying transient
/// failures. Returned errors never contain the URL or secret.
///
/// # Errors
///
/// Returns a (URL-scrubbed) error description when delivery fails after
/// all retries or is rejected with a non-retryable status.
pub async fn deliver_signed(
    client: &reqwest::Client,
    url: &str,
    secret_hex: Option<&str>,
    body: &[u8],
) -> Result<(), String> {
    // A configured secret that does not decode as hex must abort delivery,
    // not silently downgrade to an unsigned POST: the receiver would either
    // drop every alert (fail closed) or — worse — accept unsigned bodies an
    // attacker can forge. `None` and an empty/whitespace placeholder keep the
    // previous unsigned behavior.
    let secret = match secret_hex {
        None => Vec::new(),
        Some(hex_str) if hex_str.trim().is_empty() => Vec::new(),
        Some(hex_str) => decode_secret(hex_str)
            .filter(|decoded| !decoded.is_empty())
            .ok_or_else(|| "webhook secret is not valid hex; alert not sent".to_string())?,
    };

    for attempt in 0..=RETRY_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(backoff_delay(attempt - 1, Duration::from_millis(500))).await;
        }

        let mut request = client
            .post(url)
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(10));
        if !secret.is_empty() {
            request = request.header("X-Crawlkit-Signature", signature_hex(&secret, body));
        }

        match request.body(body.to_vec()).send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    return Ok(());
                }
                if status.is_client_error() && status.as_u16() != 429 {
                    return Err(format!(
                        "webhook rejected the alert (HTTP {})",
                        status.as_u16()
                    ));
                }
                tracing::warn!(
                    attempt,
                    status = status.as_u16(),
                    "regression webhook retryable status"
                );
            }
            Err(err) => {
                // Scrub the URL: webhook URLs are credentials.
                let msg = err.without_url().to_string();
                if attempt == RETRY_ATTEMPTS {
                    return Err(format!("regression webhook delivery failed: {msg}"));
                }
                tracing::warn!(attempt, error = %msg, "regression webhook transport error, retrying");
            }
        }
    }
    Err("regression webhook did not succeed after retries".to_string())
}

/// Fire the regression event for a schedule (spawned, best-effort).
pub fn fire_regression_alert(
    state: &crate::types::AppState,
    alerts: &crate::types::ScheduleAlertsConfig,
    event: &RegressionEvent,
) {
    let client = state.http_client.clone();
    let alerts = alerts.clone();
    let body = match canonical_event_body(event) {
        Ok(body) => body,
        Err(e) => {
            tracing::error!(error = %e, "failed to serialize regression event");
            return;
        }
    };
    tokio::spawn(async move {
        if let Err(msg) = deliver_signed(
            &client,
            &alerts.webhook_url,
            alerts.secret_hex.as_deref(),
            body.as_bytes(),
        )
        .await
        {
            tracing::error!(error = %msg, "regression alert delivery failed");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> RegressionEvent {
        RegressionEvent {
            event: "regression".to_string(),
            crawl_ids: vec!["run-1".to_string(), "run-2".to_string()],
            findings_delta: BTreeMap::from([
                ("changed_pages".to_string(), 3),
                ("cwv_regressions".to_string(), 1),
                ("removed_pages".to_string(), 0),
            ]),
            generated_at: chrono::DateTime::parse_from_rfc3339("2026-10-31T12:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    #[test]
    fn canonical_body_sorts_keys_deterministically() {
        let body = canonical_event_body(&event()).unwrap();
        assert_eq!(
            body,
            "{\"crawl_ids\":[\"run-1\",\"run-2\"],\"event\":\"regression\",\
             \"findings_delta\":{\"changed_pages\":3,\"cwv_regressions\":1,\"removed_pages\":0},\
             \"generated_at\":\"2026-10-31T12:00:00Z\"}"
        );
        assert_eq!(body, canonical_event_body(&event()).unwrap());
    }

    /// RFC 4231 test case 2.
    #[test]
    fn signature_matches_rfc_4231_vector() {
        assert_eq!(
            signature_hex(b"Jefe", b"what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        assert_eq!(
            backoff_delay(0, Duration::from_millis(500)),
            Duration::from_millis(500)
        );
        assert_eq!(
            backoff_delay(1, Duration::from_millis(500)),
            Duration::from_millis(1000)
        );
        assert_eq!(
            backoff_delay(2, Duration::from_millis(500)),
            Duration::from_millis(2000)
        );
        assert_eq!(backoff_delay(50, Duration::from_millis(500)), BACKOFF_CAP);
    }

    #[test]
    fn event_from_monitoring_counts_kinds() {
        use crawlkit_engine::monitoring::{AlertSeverity, MonitoringResult};
        let monitoring = MonitoringResult {
            new_pages: 2,
            removed_pages: 1,
            changed_pages: 3,
            cwv_regressions: 1,
            alert_triggered: true,
            overall_severity: AlertSeverity::Critical,
            alerts: vec![],
            changed_urls: vec![],
        };
        let event = RegressionEvent::from_monitoring(
            Some("baseline"),
            "current",
            &monitoring,
            chrono::Utc::now(),
        );
        assert_eq!(event.event, "regression");
        assert_eq!(event.crawl_ids, vec!["baseline", "current"]);
        assert_eq!(event.findings_delta["removed_pages"], 1);
        assert_eq!(event.findings_delta["changed_pages"], 3);
        assert_eq!(event.findings_delta["cwv_regressions"], 1);
    }

    // -- Contract tests: signed webhook against a local stub.

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
                        use std::io::{Read, Write};
                        let mut buf = vec![0u8; 8192];
                        let n = stream.read(&mut buf).unwrap_or(0);
                        sink.lock()
                            .unwrap()
                            .push(String::from_utf8_lossy(&buf[..n]).to_string());
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

    #[tokio::test]
    async fn delivery_carries_matching_signature() {
        let (stub, requests) = Stub::spawn(200);
        let body = canonical_event_body(&event()).unwrap();
        let secret = "aabbccdd"; // test fixture (not a credential)
        deliver_signed(
            &reqwest::Client::new(),
            &format!("http://{}", stub.addr),
            Some(secret),
            body.as_bytes(),
        )
        .await
        .expect("200 succeeds");

        let seen = requests.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let expected = signature_hex(&decode_secret(secret).unwrap(), body.as_bytes());
        assert!(
            seen[0]
                .to_ascii_lowercase()
                .contains(&format!("x-crawlkit-signature: {expected}\r\n")),
            "signature header must match the body: {}",
            seen[0]
        );
    }

    #[tokio::test]
    async fn delivery_retries_then_fails_without_url_leak() {
        let (stub, requests) = Stub::spawn(500);
        let body = canonical_event_body(&event()).unwrap();
        let err = deliver_signed(
            &reqwest::Client::new(),
            &format!("http://{}", stub.addr),
            None,
            body.as_bytes(),
        )
        .await
        .expect_err("persistent 500 fails");
        assert!(
            !err.contains(&stub.addr.to_string()),
            "error must not embed the endpoint: {err}"
        );
        assert_eq!(
            requests.lock().unwrap().len(),
            1 + RETRY_ATTEMPTS as usize,
            "1 initial + 3 retries"
        );
    }

    /// A configured secret that is not valid hex must abort delivery before
    /// any request leaves: sending unsigned would let a receiver accept
    /// forgeable alerts (and silently drop alerts on fail-closed receivers).
    #[tokio::test]
    async fn invalid_secret_hex_aborts_delivery_unsigned() {
        let (stub, requests) = Stub::spawn(200);
        let body = canonical_event_body(&event()).unwrap();
        let err = deliver_signed(
            &reqwest::Client::new(),
            &format!("http://{}", stub.addr),
            Some("not-hex-secret"),
            body.as_bytes(),
        )
        .await
        .expect_err("invalid hex secret must abort");
        assert!(
            !err.contains(&stub.addr.to_string()),
            "error must not embed the endpoint: {err}"
        );
        assert!(
            requests.lock().unwrap().is_empty(),
            "no request may be sent with a broken secret"
        );
    }

    /// An empty-string secret keeps the historical "unsigned placeholder"
    /// behavior (delivery proceeds, no signature header).
    #[tokio::test]
    async fn empty_secret_placeholder_sends_unsigned() {
        let (stub, requests) = Stub::spawn(200);
        let body = canonical_event_body(&event()).unwrap();
        deliver_signed(
            &reqwest::Client::new(),
            &format!("http://{}", stub.addr),
            Some("  "),
            body.as_bytes(),
        )
        .await
        .expect("empty placeholder delivers unsigned");
        let seen = requests.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert!(
            !seen[0]
                .to_ascii_lowercase()
                .contains("x-crawlkit-signature"),
            "placeholder must not produce a signature header: {}",
            seen[0]
        );
    }
}
