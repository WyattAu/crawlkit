use axum::extract::State;
use axum::http::header::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;

use crawlkit_engine::access_log::AccessLogEntry;

use crate::auth;
use crate::types::AppState;

/// Derive the action string from the HTTP method and URI path.
fn derive_action(method: &str, path: &str) -> String {
    let resource_type = if path.contains("/crawls") {
        "crawl"
    } else if path.contains("/keys") {
        "key"
    } else if path.contains("/webhooks") {
        "webhook"
    } else if path.contains("/schedules") {
        "schedule"
    } else if path.contains("/users") {
        "user"
    } else if path.contains("/tenants") {
        "tenant"
    } else if path.contains("/sessions") {
        "session"
    } else if path.contains("/audit") {
        "audit"
    } else {
        "unknown"
    };

    let verb = match method {
        "GET" => "read",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "other",
    };

    format!("{resource_type}.{verb}")
}

/// Middleware that logs every API access to the engine's `AccessLogger`
/// for SOC 2 compliance. Extracts the authenticated user (if present) and records
/// the action, resource, IP, and outcome.
pub async fn access_log_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // Extract client IP from X-Forwarded-For or socket.
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .map(str::to_string);

    // Extract user identity from request extensions (set by auth middleware).
    let claims = request.extensions().get::<auth::Claims>().cloned();
    let user_id = claims.as_ref().map(|c| c.sub.clone());
    let api_key_id = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let action = derive_action(&method, &path);
    // Use the last path segment as the resource hint (e.g. crawl_id).
    let resource = path
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty() && *s != "v1")
        .unwrap_or("global")
        .to_string();

    let response = next.run(request).await;
    let success = response.status().is_success() || response.status().is_informational();

    let entry = AccessLogEntry {
        timestamp: Utc::now(),
        user_id,
        api_key_id,
        action,
        resource,
        ip_address: ip,
        success,
    };

    // Non-blocking log: clone the logger Arc and push.
    let logger = state.access_logger.clone();
    tokio::task::spawn_blocking(move || {
        logger.log(entry);
    });

    response
}
