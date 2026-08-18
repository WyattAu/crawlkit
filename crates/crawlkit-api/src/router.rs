use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{AllowHeaders, Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::auth_mw::auth_middleware as jwt_auth_middleware;
use crate::handlers::api_keys::*;
use crate::handlers::audit::*;
use crate::handlers::auth_handlers::*;
use crate::handlers::crawls::*;
use crate::handlers::health::*;
use crate::handlers::marketplace::*;
use crate::handlers::schedules::*;
use crate::handlers::tenants::*;
use crate::handlers::users::*;
use crate::handlers::webhooks::*;
use crate::middleware::*;
use crate::types::*;

/// Build the full application router with protected and public routes.
pub fn create_router(state: AppState, csrf_allowed_origins: Vec<String>) -> Router {
    let protected = Router::new()
        .route("/api/v1/auth/refresh", post(refresh_token))
        .route("/api/v1/auth/me", get(get_me))
        .route("/api/v1/crawls", post(start_crawl).get(list_crawls))
        .route("/api/v1/crawls/{crawl_id}", get(get_crawl_status))
        .route("/api/v1/crawls/{crawl_id}/stats", get(get_crawl_stats))
        .route(
            "/api/v1/crawls/{crawl_id}/backlinks",
            get(get_crawl_backlinks),
        )
        .route(
            "/api/v1/crawls/{crawl_id}/findings",
            get(get_crawl_findings),
        )
        .route("/api/v1/keys", post(create_api_key).get(list_api_keys))
        .route("/api/v1/keys/{key}", axum::routing::delete(delete_api_key))
        .route("/api/v1/webhooks", post(create_webhook).get(list_webhooks))
        .route(
            "/api/v1/webhooks/{id}",
            axum::routing::delete(delete_webhook),
        )
        .route(
            "/api/v1/schedules",
            post(create_schedule).get(list_schedules),
        )
        .route(
            "/api/v1/schedules/{id}",
            axum::routing::delete(delete_schedule).patch(update_schedule),
        )
        .route("/api/v1/audit", get(get_audit_events))
        .route("/api/v1/tenants", post(create_tenant).get(list_tenants))
        .route(
            "/api/v1/tenants/{id}",
            get(get_tenant).delete(delete_tenant),
        )
        .route("/api/v1/users", post(create_user).get(list_users))
        .route("/api/v1/users/{id}", axum::routing::delete(delete_user))
        .route(
            "/api/v1/marketplace/plugins",
            post(submit_plugin).get(list_marketplace_plugins),
        )
        .route(
            "/api/v1/marketplace/plugins/{name}",
            get(get_marketplace_plugin).delete(delete_marketplace_plugin),
        )
        .route(
            "/api/v1/marketplace/plugins/{name}/test",
            post(test_plugin),
        )
        .route("/api/v1/sessions", get(list_sessions))
        .route("/api/v1/sessions/revoke", post(revoke_session))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            csrf_allowed_origins,
            csrf_origin_validation,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            api_key_auth_middleware,
        ));

    let public = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_endpoint))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/oidc/authorize", get(oidc_authorize))
        .route("/api/v1/auth/oidc/callback", get(oidc_callback));

    Router::new()
        .merge(public)
        .merge(protected)
        .layer(axum::middleware::from_fn(csp_headers))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            request_metrics_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Build CORS layer from environment configuration.
pub fn build_cors() -> CorsLayer {
    let allowed_origins: Vec<String> = std::env::var("CORS_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let cors_methods: Vec<http::Method> = std::env::var("CORS_METHODS")
        .unwrap_or_else(|_| "GET,POST,PUT,DELETE,PATCH,OPTIONS".to_string())
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter_map(|m| match m.as_str() {
            "GET" => Some(http::Method::GET),
            "POST" => Some(http::Method::POST),
            "PUT" => Some(http::Method::PUT),
            "DELETE" => Some(http::Method::DELETE),
            "PATCH" => Some(http::Method::PATCH),
            "OPTIONS" => Some(http::Method::OPTIONS),
            "HEAD" => Some(http::Method::HEAD),
            _ => None,
        })
        .collect();

    let cors_headers: Vec<http::header::HeaderName> = std::env::var("CORS_HEADERS")
        .unwrap_or_else(|_| "Authorization,Content-Type,X-API-Key".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .filter_map(|h| http::header::HeaderName::from_bytes(h.as_bytes()).ok())
        .collect();

    if allowed_origins.is_empty() || allowed_origins.iter().all(|o| o == "*") {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(AllowHeaders::any())
    } else {
        let origins: Vec<http::HeaderValue> = allowed_origins
            .iter()
            .filter_map(|o| http::HeaderValue::from_str(o).ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(cors_methods)
            .allow_headers(cors_headers)
            .max_age(std::time::Duration::from_secs(86400))
    }
}

/// Get CSRF allowed origins from environment.
pub fn csrf_origins() -> Vec<String> {
    let allowed_origins: Vec<String> = std::env::var("CORS_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| allowed_origins.join(","))
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
