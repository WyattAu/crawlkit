use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::auth;
use crate::types::*;
use crawlkit_engine::AuditEventType;

/// Register a webhook for crawl lifecycle events.
///
/// The signing secret is returned exactly once, in the `201` response.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks",
    tag = "webhooks",
    request_body = CreateWebhookRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 201, description = "Webhook created; secret returned once", body = WebhookCreatedResponse),
        (status = 400, description = "URL failed SSRF validation or unknown event type", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "CSRF origin validation failed", body = ApiErrorBody)
    )
)]
pub async fn create_webhook(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookCreatedResponse>), ApiError> {
    validate_public_url(&req.url)?;

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

    let id = Uuid::new_v4().to_string();
    let secret = generate_webhook_secret();
    let tenant_id = extract_tenant(&claims).to_string();
    let created_at = Utc::now();
    let url = req.url.clone();
    let events = req.events.clone();

    let config = WebhookConfig {
        id: id.clone(),
        tenant_id: tenant_id.clone(),
        url: req.url,
        events: req.events,
        secret: secret.clone(),
        created_at,
    };

    state.webhooks.insert(id.clone(), config);

    state.audit_trail.record_tenant(
        AuditEventType::WebhookCreated,
        &claims.sub,
        Some(extract_tenant(&claims)),
        &format!("webhook created: {id} -> {url}"),
    );

    Ok((
        StatusCode::CREATED,
        Json(WebhookCreatedResponse {
            id,
            tenant_id,
            url,
            events,
            secret,
            created_at,
        }),
    ))
}

/// List webhooks visible to the caller (own tenant, or all for admins).
/// Secrets are never included in listings.
#[utoipa::path(
    get,
    path = "/api/v1/webhooks",
    tag = "webhooks",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Webhooks visible to the caller", body = [WebhookConfig]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody)
    )
)]
pub async fn list_webhooks(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Json<Vec<WebhookConfig>> {
    let tenant = extract_tenant(&claims);
    let admin = is_admin(&claims);
    Json(
        state
            .webhooks
            .iter()
            .filter(|entry| admin || entry.value().tenant_id == tenant)
            .map(|e| e.value().clone())
            .collect(),
    )
}

/// Delete a webhook. Cross-tenant access returns `404` by design.
#[utoipa::path(
    delete,
    path = "/api/v1/webhooks/{id}",
    tag = "webhooks",
    params(
        ("id" = String, Path, description = "Webhook identifier")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 204, description = "Webhook deleted"),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "CSRF origin validation failed", body = ApiErrorBody),
        (status = 404, description = "Webhook not found or owned by another tenant", body = ApiErrorBody)
    )
)]
pub async fn delete_webhook(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let entry = state
        .webhooks
        .get(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Webhook {id} not found")))?;

    if !can_access_tenant(&claims, &entry.tenant_id) {
        return Err(ApiError::NotFound(format!("Webhook {id} not found")));
    }
    drop(entry);

    state
        .webhooks
        .remove(&id)
        .map(|_| {
            state.audit_trail.record_tenant(
                AuditEventType::WebhookDeleted,
                &claims.sub,
                Some(extract_tenant(&claims)),
                &format!("webhook deleted: {id}"),
            );
            StatusCode::NO_CONTENT
        })
        .ok_or_else(|| ApiError::NotFound(format!("Webhook {id} not found")))
}

pub fn generate_webhook_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
}

#[allow(clippy::expect_used)]
pub fn sign_webhook_payload(secret: &str, payload: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC key creation cannot fail for non-empty keys");
    mac.update(payload);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Error type for webhook delivery that supports retry classification.
#[derive(Debug)]
enum WebhookError {
    /// Network or transport error (retryable).
    Network(String),
    /// Non-success HTTP status code (retryable).
    Http(axum::http::StatusCode),
}

impl std::fmt::Display for WebhookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "network error: {msg}"),
            Self::Http(status) => write!(f, "HTTP {status}"),
        }
    }
}

impl retry_backoff::IsRetryable for WebhookError {
    fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_) | Self::Http(_))
    }
}

pub async fn deliver_webhook(
    client: &reqwest::Client,
    webhook: &WebhookConfig,
    payload: &WebhookPayload,
) -> Result<(), String> {
    // Defense in depth: re-validate the destination at delivery time. The
    // HTTP client also re-validates every redirect hop.
    if let Err(e) = validate_public_url(&webhook.url) {
        return Err(format!(
            "Webhook URL failed SSRF validation: {}",
            e.message()
        ));
    }

    let body = serde_json::to_vec(payload)
        .map_err(|e| format!("Failed to serialize webhook payload: {e}"))?;
    let signature = sign_webhook_payload(&webhook.secret, &body);

    let config = retry_backoff::RetryConfig {
        max_retries: 3,
        initial_delay: std::time::Duration::from_secs(1),
        ..Default::default()
    };

    let url = webhook.url.clone();
    let event = payload.event.clone();

    retry_backoff::with_backoff(&config, || {
        let body = body.clone();
        let url = url.clone();
        let event = event.clone();
        let signature = signature.clone();
        let client = client.clone();
        async move {
            tracing::debug!("Delivering webhook to {url} (event={event})");

            match client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-Webhook-Event", &event)
                .header("X-Webhook-Signature", format!("sha256={signature}"))
                .body(body)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        tracing::info!(
                            "Webhook delivered successfully to {url} (status={})",
                            resp.status()
                        );
                        Ok(())
                    } else {
                        let status = resp.status();
                        tracing::warn!("Webhook to {url} returned non-success status {status}");
                        Err(WebhookError::Http(status))
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to send webhook to {url}: {e}");
                    Err(WebhookError::Network(e.to_string()))
                }
            }
        }
    })
    .await
    .map_err(|e| {
        let msg = e.into_inner();
        tracing::error!("Webhook delivery to {} failed: {msg}", webhook.url);
        format!("Webhook delivery failed: {msg}")
    })?;

    Ok(())
}

pub fn fire_webhooks(
    state: &AppState,
    event: &str,
    crawl_id: &str,
    tenant_id: &str,
    pages_crawled: usize,
    issues_found: usize,
) {
    let payload = WebhookPayload {
        event: event.to_string(),
        crawl_id: crawl_id.to_string(),
        pages_crawled,
        issues_found,
        timestamp: Utc::now(),
    };

    let matching: Vec<WebhookConfig> = state
        .webhooks
        .iter()
        .filter(|entry| entry.value().tenant_id == tenant_id)
        .filter(|entry| entry.value().events.iter().any(|e| e == event))
        .map(|entry| entry.value().clone())
        .collect();

    if matching.is_empty() {
        return;
    }

    let client = state.http_client.clone();
    tokio::spawn(async move {
        for webhook in matching {
            let client = client.clone();
            let payload = payload.clone();
            tokio::spawn(async move {
                if let Err(e) = deliver_webhook(&client, &webhook, &payload).await {
                    tracing::error!("Webhook delivery failed: {e}");
                }
            });
        }
    });
}

/// Fire `monitoring.alert_triggered` webhooks with monitoring delta details.
pub fn fire_monitoring_webhooks(
    state: &AppState,
    crawl_id: &str,
    tenant_id: &str,
    monitoring: &crawlkit_engine::monitoring::MonitoringResult,
) {
    let matching: Vec<WebhookConfig> = state
        .webhooks
        .iter()
        .filter(|entry| entry.value().tenant_id == tenant_id)
        .filter(|entry| {
            entry
                .value()
                .events
                .iter()
                .any(|e| e == "monitoring.alert_triggered")
        })
        .map(|entry| entry.value().clone())
        .collect();

    if matching.is_empty() {
        return;
    }

    let alert = crawlkit_engine::monitoring::Alert::from_result(monitoring, 20);

    let critical_count = monitoring
        .alerts
        .iter()
        .filter(|a| a.severity == crawlkit_engine::monitoring::AlertSeverity::Critical)
        .count();
    let warning_count = monitoring
        .alerts
        .iter()
        .filter(|a| a.severity == crawlkit_engine::monitoring::AlertSeverity::Warning)
        .count();
    let info_count = monitoring
        .alerts
        .iter()
        .filter(|a| a.severity == crawlkit_engine::monitoring::AlertSeverity::Info)
        .count();

    let body = serde_json::json!({
        "event": "monitoring.alert_triggered",
        "crawl_id": crawl_id,
        "monitoring": {
            "severity": monitoring.overall_severity.to_string(),
            "alert_triggered": monitoring.alert_triggered,
            "summary": {
                "new_pages": monitoring.new_pages,
                "removed_pages": monitoring.removed_pages,
                "changed_pages": monitoring.changed_pages,
                "cwv_regressions": monitoring.cwv_regressions,
                "total_affected_urls": monitoring.changed_urls.len(),
                "total_alerts": monitoring.alerts.len(),
                "critical_count": critical_count,
                "warning_count": warning_count,
                "info_count": info_count,
            },
            "changed_urls": monitoring.changed_urls.iter().take(20).collect::<Vec<_>>(),
            "alerts": monitoring.alerts.iter().map(|a| serde_json::json!({
                "url": a.url,
                "severity": a.severity.to_string(),
                "message": a.message,
            })).collect::<Vec<_>>(),
            "alert": {
                "title": alert.title,
                "description": alert.description,
                "affected_urls": alert.affected_urls,
                "timestamp": alert.timestamp.to_rfc3339(),
            },
            "trend_report_url": format!("/api/v1/monitoring/trends/{crawl_id}"),
        },
        "timestamp": Utc::now().to_rfc3339(),
    });

    let client = state.http_client.clone();
    tokio::spawn(async move {
        for webhook in matching {
            let client = client.clone();
            let body = body.clone();
            let webhook_url = webhook.url.clone();
            let webhook_secret = webhook.secret.clone();
            tokio::spawn(async move {
                let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
                let signature = sign_webhook_payload(&webhook_secret, &body_bytes);
                if let Err(e) = client
                    .post(&webhook_url)
                    .header("Content-Type", "application/json")
                    .header("X-Webhook-Event", "monitoring.alert_triggered")
                    .header("X-Webhook-Signature", format!("sha256={signature}"))
                    .body(body_bytes)
                    .timeout(std::time::Duration::from_secs(10))
                    .send()
                    .await
                {
                    tracing::error!("Monitoring webhook delivery failed: {e}");
                }
            });
        }
    });
}
