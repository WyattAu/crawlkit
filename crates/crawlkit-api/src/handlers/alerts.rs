//! Alert channels (ADR-014) — Slack and Teams delivery over the existing
//! loop-retry webhook pipeline.
//!
//! Per ADR-014, alerting is *not* a new delivery system: a Slack
//! incoming-webhook POST and a Teams Power Automate connector POST are both
//! "deliver a payload to an external endpoint with retry on transient
//! failure". This module provides:
//!
//! 1. **One event source, provider-native renderers.** [`AlertEvent`] is the
//!    same event model that drives webhooks; [`render_slack`] and
//!    [`render_teams`] are pure functions producing Slack Block Kit and Teams
//!    MessageCard payloads respectively. Rendering failures are programming
//!    errors, caught by contract tests here — never by runtime retry.
//! 2. **Delivery reuses loop-retry semantics** with the same retryable
//!    classification as webhook delivery (transport errors and 5xx/429
//!    retryable; other 4xx fatal).
//! 3. **Credentials live in the encrypted credential store.** A channel
//!    references its webhook URL by connector name
//!    ([`AlertChannelConfig::credential_connector`]); the URL itself is
//!    never stored in channel config, echoed in responses, or logged.
//!    Delivery failures are recorded with the URL scrubbed — `reqwest`
//!    error strings embed request URLs, so error materialization must go
//!    through [`scrub_url`].
//! 4. **Failure visibility.** Consecutive failures, last outcome, and a
//!    sanitized last error are tracked per channel and surfaced in listings,
//!    so a silently dead Slack webhook is detectable.

use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::auth;
use crate::types::*;
use crawlkit_engine::AuditEventType;

/// Number of consecutive failures after which delivery degradation is
/// escalated from a warning to an error-level health signal.
const FAILURE_ESCALATION_THRESHOLD: u32 = 3;

// ---------------------------------------------------------------------------
// Event model
// ---------------------------------------------------------------------------

/// The internal alert event — the same event model that drives webhooks.
///
/// Renderers are pure functions of this type; adding a channel type means
/// adding a renderer, nothing else.
#[derive(Debug, Clone)]
pub struct AlertEvent {
    /// Event name, e.g. `crawl.completed`, `crawl.failed`.
    pub event: String,
    /// Crawl the event refers to.
    pub crawl_id: String,
    /// Pages crawled at event time (0 for `crawl.failed`).
    pub pages_crawled: usize,
    /// Findings at event time (0 for `crawl.failed`).
    pub issues_found: usize,
    /// Event time.
    pub timestamp: DateTime<Utc>,
}

impl AlertEvent {
    /// Human-readable summary line used as the notification title.
    #[must_use]
    pub fn title(&self) -> String {
        match self.event.as_str() {
            "crawl.failed" => format!("Crawl failed: {}", self.crawl_id),
            "crawl.completed" => format!(
                "Crawl completed: {} pages, {} issues",
                self.pages_crawled, self.issues_found
            ),
            other => format!("Alert: {other}"),
        }
    }

    /// Severity color used by renderers (Teams `themeColor`, Slack bar).
    #[must_use]
    pub fn theme_color(&self) -> &'static str {
        match self.event.as_str() {
            "crawl.failed" => "CC0000",
            _ => "0076D7",
        }
    }
}

// ---------------------------------------------------------------------------
// Renderers (pure functions)
// ---------------------------------------------------------------------------

/// Render an event as a Slack Block Kit payload.
///
/// Contract-tested shape: a `header` block with the title, a `section` with
/// the detail line, and a `context` block with the timestamp.
#[must_use]
pub fn render_slack(event: &AlertEvent) -> serde_json::Value {
    serde_json::json!({
        "blocks": [
            {
                "type": "header",
                "text": { "type": "plain_text", "text": event.title() }
            },
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!(
                        "*{}* · crawl `{}`\nPages: {} · Issues: {}",
                        event.event, event.crawl_id, event.pages_crawled, event.issues_found
                    )
                }
            },
            {
                "type": "context",
                "elements": [
                    { "type": "mrkdwn", "text": format!("crawlkit · {}", event.timestamp.to_rfc3339()) }
                ]
            }
        ]
    })
}

/// Render an event as a Teams MessageCard payload.
#[must_use]
pub fn render_teams(event: &AlertEvent) -> serde_json::Value {
    serde_json::json!({
        "@type": "MessageCard",
        "@context": "http://schema.org/extensions",
        "summary": event.title(),
        "themeColor": event.theme_color(),
        "title": event.title(),
        "sections": [
            {
                "activityTitle": event.title(),
                "facts": [
                    { "name": "Event", "value": event.event },
                    { "name": "Crawl ID", "value": event.crawl_id },
                    { "name": "Pages", "value": event.pages_crawled.to_string() },
                    { "name": "Issues", "value": event.issues_found.to_string() }
                ],
                "markdown": true
            }
        ]
    })
}

/// Renderer for a channel type. Unknown channel types are rejected at
/// creation time, so this only needs to cover the shipped set.
#[must_use]
pub fn render(channel_type: &str, event: &AlertEvent) -> Option<serde_json::Value> {
    match channel_type {
        "slack" => Some(render_slack(event)),
        "teams" => Some(render_teams(event)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Delivery — loop-retry over the existing pipeline
// ---------------------------------------------------------------------------

/// Delivery error with retry classification matching webhook delivery:
/// transport errors and non-success statuses are retryable; the caller
/// distinguishes only through [`loop_retry::IsRetryable`].
#[derive(Debug)]
enum AlertDeliveryError {
    /// Transport-level failure (connection, timeout, TLS).
    Network(String),
    /// Non-success HTTP status.
    Http(StatusCode),
}

impl std::fmt::Display for AlertDeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // `msg` is already URL-scrubbed by the caller.
            Self::Network(msg) => write!(f, "network error: {msg}"),
            Self::Http(status) => write!(f, "HTTP {status}"),
        }
    }
}

impl loop_retry::IsRetryable for AlertDeliveryError {
    fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_) | Self::Http(_))
    }
}

/// Strip URL material from a transport error before it can reach a string
/// that outlives the request.
///
/// `reqwest::Error`'s `Display` embeds the request URL, and channel webhook
/// URLs are credentials (a Slack URL grants posting rights). Every
/// materialization of a transport error in this module goes through here.
/// Takes ownership because `without_url` is a consuming builder method.
#[must_use]
pub fn scrub_url(err: reqwest::Error) -> String {
    err.without_url().to_string()
}

/// Deliver a rendered payload to a channel endpoint with loop-retry.
///
/// The endpoint URL never appears in returned errors or log lines; callers
/// identify deliveries by `channel_id` + `event`.
pub async fn deliver_alert(
    client: &reqwest::Client,
    url: &str,
    channel_id: &str,
    event: &str,
    payload: &serde_json::Value,
) -> Result<(), String> {
    deliver_alert_checked(
        client,
        url,
        channel_id,
        event,
        payload,
        &validate_public_url,
    )
    .await
}

/// [`deliver_alert`] with an injectable URL validator. Production callers
/// go through [`deliver_alert`]; tests use a permissive validator so the
/// delivery pipeline can be exercised against loopback stubs.
pub(crate) async fn deliver_alert_checked(
    client: &reqwest::Client,
    url: &str,
    channel_id: &str,
    event: &str,
    payload: &serde_json::Value,
    validate: &(dyn Fn(&str) -> Result<(), ApiError> + Send + Sync),
) -> Result<(), String> {
    // Defense in depth: the credential materialized from the store is still
    // an outbound URL and must pass SSRF validation at delivery time.
    if let Err(e) = validate(url) {
        return Err(format!(
            "alert endpoint failed SSRF validation: {}",
            e.message()
        ));
    }

    let body = match serde_json::to_vec(payload) {
        Ok(b) => b,
        Err(e) => return Err(format!("failed to render alert payload: {e}")),
    };

    let config = loop_retry::RetryConfig {
        max_retries: 3,
        initial_delay: std::time::Duration::from_secs(1),
        ..Default::default()
    };

    let url = url.to_string();
    let event = event.to_string();
    let channel_id = channel_id.to_string();

    loop_retry::with_backoff(&config, || {
        let body = body.clone();
        let url = url.clone();
        let client = client.clone();
        let channel_id = channel_id.clone();
        let event = event.clone();
        async move {
            tracing::debug!(channel = %channel_id, event = %event, "delivering alert");
            match client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(body)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        Ok(())
                    } else {
                        tracing::warn!(
                            channel = %channel_id,
                            event = %event,
                            status = %status,
                            "alert endpoint returned non-success status"
                        );
                        Err(AlertDeliveryError::Http(status))
                    }
                }
                Err(e) => {
                    let msg = scrub_url(e);
                    tracing::warn!(
                        channel = %channel_id,
                        event = %event,
                        error = %msg,
                        "alert delivery transport error"
                    );
                    Err(AlertDeliveryError::Network(msg))
                }
            }
        }
    })
    .await
    .map_err(|e| {
        let msg = e.into_inner().to_string();
        // Already scrubbed: AlertDeliveryError never carries the URL.
        format!("alert delivery failed: {msg}")
    })
}

// ---------------------------------------------------------------------------
// Fan-out — one event source, all configured subscribers
// ---------------------------------------------------------------------------

/// Fire an alert event to every matching channel of the tenant.
///
/// Mirrors [`fire_webhooks`]: spawns delivery tasks so crawl completion is
/// never blocked on alert delivery, resolves the endpoint from the
/// credential store at delivery time, and updates per-channel health after
/// every attempt (ADR-014 §2 — a dead channel must be visible, not silent).
pub fn fire_alerts(
    state: &AppState,
    event: &str,
    crawl_id: &str,
    tenant_id: &str,
    pages_crawled: usize,
    issues_found: usize,
) {
    fire_alerts_with_validator(
        state,
        event,
        crawl_id,
        tenant_id,
        pages_crawled,
        issues_found,
        Arc::new(validate_public_url),
    );
}

/// [`fire_alerts`] with an injectable URL validator (test seam, same
/// rationale as [`deliver_alert_checked`]).
pub(crate) fn fire_alerts_with_validator(
    state: &AppState,
    event: &str,
    crawl_id: &str,
    tenant_id: &str,
    pages_crawled: usize,
    issues_found: usize,
    validate: Arc<dyn Fn(&str) -> Result<(), ApiError> + Send + Sync>,
) {
    let matching: Vec<AlertChannelConfig> = state
        .alert_channels
        .iter()
        .filter(|entry| entry.value().tenant_id == tenant_id)
        .filter(|entry| entry.value().events.iter().any(|e| e == event))
        .map(|entry| entry.value().clone())
        .collect();

    if matching.is_empty() {
        return;
    }

    let alert_event = AlertEvent {
        event: event.to_string(),
        crawl_id: crawl_id.to_string(),
        pages_crawled,
        issues_found,
        timestamp: Utc::now(),
    };

    let client = state.http_client.clone();
    let store = state.credential_store.clone();
    let channels = state.alert_channels.clone();

    tokio::spawn(async move {
        for channel in matching {
            let client = client.clone();
            let store = store.clone();
            let channels = channels.clone();
            let validate = validate.clone();
            let event = alert_event.clone();
            tokio::spawn(async move {
                let outcome = match store.as_ref() {
                    None => Err("credential store unavailable".to_string()),
                    Some(store) => match store
                        .get(&channel.tenant_id, &channel.credential_connector)
                    {
                        Ok(credential) => {
                            let payload = match render(&channel.channel_type, &event) {
                                Some(p) => p,
                                None => {
                                    record_failure(&channels, &channel.id, "unknown channel type");
                                    return;
                                }
                            };
                            deliver_alert_checked(
                                &client,
                                &credential.secret,
                                &channel.id,
                                &event.event,
                                &payload,
                                &*validate,
                            )
                            .await
                        }
                        Err(e) => Err(format!("credential unavailable: {e}")),
                    },
                };

                match outcome {
                    Ok(()) => record_success(&channels, &channel.id),
                    Err(msg) => record_failure(&channels, &channel.id, &msg),
                }
            });
        }
    });
}

/// Record a successful delivery in the channel's health fields.
fn record_success(channels: &AlertChannelMap, id: &str) {
    if let Some(mut entry) = channels.get_mut(id) {
        entry.consecutive_failures = 0;
        entry.last_success_at = Some(Utc::now());
        entry.last_error = None;
    }
}

/// Record a failed delivery, escalating to an error-level signal once
/// failures exceed [`FAILURE_ESCALATION_THRESHOLD`].
fn record_failure(channels: &AlertChannelMap, id: &str, error: &str) {
    if let Some(mut entry) = channels.get_mut(id) {
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        entry.last_failure_at = Some(Utc::now());
        entry.last_error = Some(error.to_string());

        if entry.consecutive_failures >= FAILURE_ESCALATION_THRESHOLD {
            tracing::error!(
                channel = %id,
                failures = entry.consecutive_failures,
                last_error = %error,
                "alert channel is failing repeatedly — alerts are NOT being delivered"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// HTTP API
// ---------------------------------------------------------------------------

/// Register an alert channel.
///
/// The referenced credential must already exist in the tenant's credential
/// store; the webhook URL is provided out-of-band to the store, never to
/// this endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/alert-channels",
    tag = "alerts",
    request_body = CreateAlertChannelRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 201, description = "Alert channel created", body = AlertChannelConfig),
        (status = 400, description = "Unknown channel type, invalid event type, missing credential store, or unknown credential", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn create_alert_channel(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<CreateAlertChannelRequest>,
) -> Result<(StatusCode, Json<AlertChannelConfig>), ApiError> {
    if req.channel_type != "slack" && req.channel_type != "teams" {
        return Err(ApiError::BadRequest(format!(
            "Invalid channel type: {}. Must be 'slack' or 'teams'",
            req.channel_type
        )));
    }

    for event in &req.events {
        if event != "crawl.completed"
            && event != "crawl.failed"
            && event != "monitoring.alert_triggered"
        {
            return Err(ApiError::BadRequest(format!(
                "Invalid event type: {event}. Must be 'crawl.completed', 'crawl.failed', or 'monitoring.alert_triggered'"
            )));
        }
    }

    let tenant_id = extract_tenant(&claims);

    // The credential must exist — fail fast rather than discovering the gap
    // at first delivery. The store is required: a channel without a
    // credential backend cannot ever deliver.
    let store = state.credential_store.as_ref().ok_or_else(|| {
        ApiError::BadRequest("credential store is not configured on this deployment".to_string())
    })?;
    if !store.contains(tenant_id, &req.credential_connector) {
        return Err(ApiError::BadRequest(format!(
            "no credential '{}' for this tenant — store the webhook URL credential first",
            req.credential_connector
        )));
    }

    let id = Uuid::new_v4().to_string();
    let config = AlertChannelConfig {
        id: id.clone(),
        tenant_id: tenant_id.to_string(),
        channel_type: req.channel_type,
        credential_connector: req.credential_connector,
        events: req.events,
        created_at: Utc::now(),
        consecutive_failures: 0,
        last_success_at: None,
        last_failure_at: None,
        last_error: None,
    };

    state.alert_channels.insert(id, config.clone());

    state.audit_trail.record_tenant(
        AuditEventType::ConfigChanged,
        &claims.sub,
        Some(tenant_id),
        &format!(
            "alert channel created: {} ({})",
            config.id, config.channel_type
        ),
    );

    Ok((StatusCode::CREATED, Json(config)))
}

/// List alert channels visible to the caller (own tenant, or all for
/// admins), including delivery health.
///
/// Responses never contain endpoint URLs or any credential material.
#[utoipa::path(
    get,
    path = "/api/v1/alert-channels",
    tag = "alerts",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Alert channels visible to the caller", body = [AlertChannelConfig]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn list_alert_channels(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Json<Vec<AlertChannelConfig>> {
    let tenant = extract_tenant(&claims);
    let admin = is_admin(&claims);
    Json(
        state
            .alert_channels
            .iter()
            .filter(|entry| admin || entry.value().tenant_id == tenant)
            .map(|e| e.value().clone())
            .collect(),
    )
}

/// Delete an alert channel. Cross-tenant access returns `404` by design.
#[utoipa::path(
    delete,
    path = "/api/v1/alert-channels/{id}",
    tag = "alerts",
    params(
        ("id" = String, Path, description = "Alert channel identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 204, description = "Alert channel deleted"),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "CSRF origin validation failed", body = ApiErrorBody),
        (status = 404, description = "Alert channel not found or owned by another tenant", body = ApiErrorBody)
    )
)]
pub async fn delete_alert_channel(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let entry = state
        .alert_channels
        .get(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Alert channel {id} not found")))?;

    if !can_access_tenant(&claims, &entry.tenant_id) {
        return Err(ApiError::NotFound(format!("Alert channel {id} not found")));
    }
    let channel_type = entry.channel_type.clone();
    drop(entry);

    state
        .alert_channels
        .remove(&id)
        .map(|_| {
            state.audit_trail.record_tenant(
                AuditEventType::ConfigChanged,
                &claims.sub,
                Some(extract_tenant(&claims)),
                &format!("alert channel deleted: {id} ({channel_type})"),
            );
            StatusCode::NO_CONTENT
        })
        .ok_or_else(|| ApiError::NotFound(format!("Alert channel {id} not found")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential_store::{Credential, CredentialMetadata};
    use dashmap::DashMap;
    use std::io::{Read, Write};
    use std::sync::Arc;

    fn event(event: &str) -> AlertEvent {
        AlertEvent {
            event: event.to_string(),
            crawl_id: "crawl-123".to_string(),
            pages_crawled: 120,
            issues_found: 4,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn slack_renderer_produces_block_kit_shape() {
        let payload = render_slack(&event("crawl.completed"));
        let blocks = payload["blocks"].as_array().expect("blocks array");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0]["type"], "header");
        assert_eq!(blocks[0]["text"]["type"], "plain_text");
        let title = blocks[0]["text"]["text"].as_str().expect("title");
        assert!(title.contains("120 pages"), "title mentions pages: {title}");
        assert!(title.contains("4 issues"), "title mentions issues: {title}");
        assert_eq!(blocks[1]["type"], "section");
        assert_eq!(blocks[2]["type"], "context");
    }

    #[test]
    fn slack_renderer_flags_failures() {
        let payload = render_slack(&event("crawl.failed"));
        let title = payload["blocks"][0]["text"]["text"].as_str().unwrap();
        assert!(title.contains("failed"), "failure title: {title}");
    }

    #[test]
    fn teams_renderer_produces_message_card_shape() {
        let payload = render_teams(&event("crawl.completed"));
        assert_eq!(payload["@type"], "MessageCard");
        assert_eq!(payload["@context"], "http://schema.org/extensions");
        assert!(payload["summary"].as_str().is_some());
        let facts = payload["sections"][0]["facts"].as_array().expect("facts");
        assert_eq!(facts.len(), 4);
        assert_eq!(facts[2]["name"], "Pages");
        assert_eq!(facts[2]["value"], "120");
    }

    #[test]
    fn teams_renderer_uses_severity_color() {
        let ok = render_teams(&event("crawl.completed"));
        let bad = render_teams(&event("crawl.failed"));
        assert_ne!(ok["themeColor"], bad["themeColor"]);
    }

    #[test]
    fn render_dispatches_by_channel_type() {
        let e = event("crawl.completed");
        assert!(render("slack", &e).is_some());
        assert!(render("teams", &e).is_some());
        assert!(
            render("email", &e).is_none(),
            "email transport not shipped yet"
        );
        assert!(render("carrier-pigeon", &e).is_none());
    }

    /// Hermetic HTTP stub: accepts one connection per request on a loopback
    /// port, records the request line, replies with `status`.
    struct Stub {
        addr: std::net::SocketAddr,
    }

    impl Stub {
        fn spawn(status: u16) -> (Self, Arc<std::sync::Mutex<Vec<String>>>) {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
            let addr = listener.local_addr().expect("addr");
            let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
            let sink = requests.clone();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { continue };
                    let sink = sink.clone();
                    std::thread::spawn(move || {
                        let mut buf = vec![0u8; 4096];
                        let _ = stream.read(&mut buf);
                        let text = String::from_utf8_lossy(&buf).to_string();
                        let request_line = text.lines().next().unwrap_or_default().to_string();
                        sink.lock().expect("lock").push(request_line);
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

    /// Permissive validator standing in for `validate_public_url` in tests:
    /// loopback stubs are legitimate delivery targets there.
    fn allow_all(url: &str) -> Result<(), ApiError> {
        let _ = url;
        Ok(())
    }

    #[tokio::test]
    async fn delivery_succeeds_on_2xx_and_hits_the_endpoint() {
        let (stub, requests) = Stub::spawn(200);
        let payload = render_slack(&event("crawl.completed"));
        deliver_alert_checked(
            &reqwest::Client::new(),
            &format!("http://{}", stub.addr),
            "ch-1",
            "crawl.completed",
            &payload,
            &allow_all,
        )
        .await
        .expect("delivery succeeds on 200");

        let seen = requests.lock().expect("lock");
        assert!(
            seen.iter().any(|line| line.starts_with("POST / ")),
            "expected POST request line, got: {seen:?}"
        );
    }

    #[tokio::test]
    async fn delivery_fails_on_500() {
        let (stub, _requests) = Stub::spawn(500);
        let result = deliver_alert_checked(
            &reqwest::Client::new(),
            &format!("http://{}", stub.addr),
            "ch-1",
            "crawl.completed",
            &render_slack(&event("crawl.completed")),
            &allow_all,
        )
        .await;
        let msg = result.expect_err("500 must fail delivery");
        assert!(msg.contains("HTTP 500"), "error mentions status: {msg}");
        assert!(
            !msg.contains(&stub.addr.to_string()),
            "error must not embed the endpoint URL: {msg}"
        );
    }

    #[tokio::test]
    async fn transport_errors_are_url_scrubbed() {
        // Nothing listens on this port — guaranteed connection refusal.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let result = deliver_alert_checked(
            &reqwest::Client::new(),
            &format!("http://{addr}"),
            "ch-1",
            "crawl.completed",
            &render_slack(&event("crawl.completed")),
            &allow_all,
        )
        .await;
        let msg = result.expect_err("refused connection must fail");
        assert!(
            !msg.contains(&addr.to_string()),
            "transport error must not embed the endpoint URL: {msg}"
        );
    }

    #[tokio::test]
    async fn scrub_url_removes_url_from_transport_errors() {
        // Direct pin on the hygiene contract: a reqwest error carrying a URL
        // materializes without it. Produced via a guaranteed connection
        // refusal (bind a port, drop the listener, connect).
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let err = reqwest::Client::new()
            .get(format!("http://{addr}/credential-material"))
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .expect_err("refused connection yields a transport error");
        let raw = err.to_string();
        let scrubbed = scrub_url(err);
        assert!(
            raw.contains(&addr.to_string()),
            "precondition: raw error embeds URL: {raw}"
        );
        assert!(
            !scrubbed.contains(&addr.to_string()),
            "scrubbed error must not: {scrubbed}"
        );
    }

    #[tokio::test]
    async fn fire_alerts_updates_health_and_never_logs_the_url() {
        use crate::credential_store::CredentialStore;
        use crawlkit_engine::EncryptionManager;

        let (stub, requests) = Stub::spawn(200);
        let url = format!("http://{}", stub.addr);

        let store = Arc::new(CredentialStore::new(Arc::new(EncryptionManager::default())));
        store
            .put(
                "tenant-1",
                Credential {
                    secret: url.clone(),
                    metadata: CredentialMetadata {
                        connector: "slack".to_string(),
                        identifiers: std::collections::HashMap::new(),
                        created_at: "2026-09-11T00:00:00Z".to_string(),
                    },
                },
            )
            .expect("store credential");

        let channels: AlertChannelMap = Arc::new(DashMap::new());
        channels.insert(
            "ch-1".to_string(),
            AlertChannelConfig {
                id: "ch-1".to_string(),
                tenant_id: "tenant-1".to_string(),
                channel_type: "slack".to_string(),
                credential_connector: "slack".to_string(),
                events: vec!["crawl.completed".to_string()],
                created_at: Utc::now(),
                consecutive_failures: 0,
                last_success_at: None,
                last_failure_at: None,
                last_error: None,
            },
        );

        // Minimal AppState stand-in: fire_alerts touches only these fields.
        let app_state = make_state(store, channels.clone());
        fire_alerts_with_validator(
            &app_state,
            "crawl.completed",
            "crawl-9",
            "tenant-1",
            5,
            1,
            Arc::new(allow_all),
        );

        // Delivery is spawned — wait for the request to arrive.
        for _ in 0..50 {
            if !requests.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let entry = channels.get("ch-1").unwrap();
        assert_eq!(entry.consecutive_failures, 0);
        assert!(entry.last_success_at.is_some(), "success recorded");
        assert!(entry.last_error.is_none());
        drop(entry);

        // Health fields never expose the endpoint.
        let entry = channels.get("ch-1").unwrap();
        let serialized = serde_json::to_string(&*entry).unwrap();
        assert!(
            !serialized.contains(&stub.addr.to_string()),
            "channel representation must not contain the endpoint URL: {serialized}"
        );
        drop(entry);
    }

    #[tokio::test]
    async fn fire_alerts_records_failure_when_credential_missing() {
        use crate::credential_store::CredentialStore;
        use crawlkit_engine::EncryptionManager;

        let store = Arc::new(CredentialStore::new(Arc::new(EncryptionManager::default())));
        let channels: AlertChannelMap = Arc::new(DashMap::new());
        channels.insert(
            "ch-2".to_string(),
            AlertChannelConfig {
                id: "ch-2".to_string(),
                tenant_id: "tenant-1".to_string(),
                channel_type: "teams".to_string(),
                credential_connector: "teams".to_string(),
                events: vec!["crawl.failed".to_string()],
                created_at: Utc::now(),
                consecutive_failures: 0,
                last_success_at: None,
                last_failure_at: None,
                last_error: None,
            },
        );

        let app_state = make_state(store, channels.clone());
        fire_alerts_with_validator(
            &app_state,
            "crawl.failed",
            "crawl-9",
            "tenant-1",
            0,
            0,
            Arc::new(allow_all),
        );
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let entry = channels.get("ch-2").unwrap();
        assert_eq!(entry.consecutive_failures, 1, "failure recorded");
        assert!(entry
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("credential unavailable"));
    }

    /// Build a full AppState for fan-out tests. Only the fields `fire_alerts`
    /// reads are given meaningful values; the rest are inert defaults.
    fn make_state(
        store: Arc<crate::credential_store::CredentialStore>,
        channels: AlertChannelMap,
    ) -> AppState {
        AppState {
            storage: Arc::new(
                crawlkit_engine::storage::Storage::new(
                    &std::env::temp_dir().join(format!("alerts-test-{}", Uuid::new_v4())),
                )
                .expect("storage"),
            ),
            api_keys: Arc::new(DashMap::new()),
            rate_limits: Arc::new(DashMap::new()),
            crawl_results: Arc::new(DashMap::new()),
            audit_trail: Arc::new(crawlkit_engine::AuditTrail::new()),
            metrics: Arc::new(Metrics::new()),
            webhooks: Arc::new(DashMap::new()),
            alert_channels: channels,
            schedules: Arc::new(DashMap::new()),
            http_client: reqwest::Client::new(),
            auth: Arc::new(crate::auth::AuthManager::new("test-secret".to_string())),
            oidc: None,
            oidc_states: Arc::new(DashMap::new()),
            tenants: Arc::new(DashMap::new()),
            marketplace: MarketplaceState::new(),
            sessions: Arc::new(DashMap::new()),
            login_attempts: Arc::new(DashMap::new()),
            persistence: None,
            crawl_permits: Arc::new(tokio::sync::Semaphore::new(1)),
            idempotency_keys: Arc::new(DashMap::new()),
            access_logger: Arc::new(crawlkit_engine::access_log::AccessLogger::new(100)),
            credential_store: Some(store),
        }
    }
}
