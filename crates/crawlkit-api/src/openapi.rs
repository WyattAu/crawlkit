//! Auto-generated OpenAPI documentation for the crawlkit API.
//!
//! Every route handler under `src/handlers` is annotated with
//! `#[utoipa::path]`; this module aggregates them into a single OpenAPI
//! document, registers the response schemas, and declares the two security
//! schemes (`X-API-Key` and Bearer JWT) enforced by the middleware stack.
//!
//! The document is served at `GET /api/v1/openapi.json` with a Swagger UI
//! at `GET /api/v1/docs` (both disableable via `DOCS_PUBLIC=false`).

use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// Aggregate OpenAPI document for all annotated handlers.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "crawlkit API",
        version = env!("CARGO_PKG_VERSION"),
        description = "REST API for the crawlkit web crawler and SEO analysis toolkit",
        license(name = "Apache-2.0")
    ),
    paths(
        crate::handlers::api_keys::create_api_key,
        crate::handlers::api_keys::delete_api_key,
        crate::handlers::api_keys::list_api_keys,
        crate::handlers::audit::get_audit_events,
        crate::handlers::auth_handlers::get_me,
        crate::handlers::auth_handlers::list_sessions,
        crate::handlers::auth_handlers::login,
        crate::handlers::auth_handlers::oidc_authorize,
        crate::handlers::auth_handlers::oidc_callback,
        crate::handlers::auth_handlers::refresh_token,
        crate::handlers::auth_handlers::revoke_session,
        crate::handlers::crawls::get_crawl_backlinks,
        crate::handlers::crawls::get_crawl_findings,
        crate::handlers::crawls::get_crawl_stats,
        crate::handlers::crawls::get_crawl_status,
        crate::handlers::crawls::list_crawls,
        crate::handlers::crawls::start_crawl,
        crate::handlers::health::health,
        crate::handlers::health::metrics_endpoint,
        crate::handlers::marketplace::delete_marketplace_plugin,
        crate::handlers::marketplace::get_marketplace_plugin,
        crate::handlers::marketplace::list_marketplace_plugins,
        crate::handlers::marketplace::submit_plugin,
        crate::handlers::marketplace::test_plugin,
        crate::handlers::schedules::create_schedule,
        crate::handlers::schedules::delete_schedule,
        crate::handlers::schedules::list_schedules,
        crate::handlers::schedules::update_schedule,
        crate::handlers::tenants::create_tenant,
        crate::handlers::tenants::delete_tenant,
        crate::handlers::tenants::get_tenant,
        crate::handlers::tenants::list_tenants,
        crate::handlers::users::create_user,
        crate::handlers::users::delete_user,
        crate::handlers::users::list_users,
        crate::handlers::webhooks::create_webhook,
        crate::handlers::webhooks::delete_webhook,
        crate::handlers::webhooks::list_webhooks
    ),
    components(
        schemas(
            crate::types::ApiAuditEvent,
            crate::types::ApiErrorBody,
            crate::types::ApiKeyCreateRequest,
            crate::types::ApiKeyResponse,
            crate::handlers::auth_handlers::LoginResponse,
            crate::handlers::crawls::BacklinksResponse,
            crate::types::CreateCrawlRequest,
            crate::types::CreateScheduleRequest,
            crate::types::CreateTenantRequest,
            crate::types::CreateUserRequest,
            crate::types::CreateWebhookRequest,
            crate::types::CrawlFinding,
            crate::types::CrawlResponse,
            crate::types::CrawlResult,
            crate::types::CrawlStatsResponse,
            crate::types::HealthResponse,
            crate::types::LoginRequest,
            crate::types::MarketplacePlugin,
            crate::types::RevokeSessionRequest,
            crate::types::ScheduleConfig,
            crate::types::ScheduleResponse,
            crate::types::SessionResponse,
            crate::types::SubmitPluginRequest,
            crate::types::Tenant,
            crate::types::UpdateScheduleRequest,
            crate::types::UserResponse,
            crate::types::WebhookConfig,
            crate::types::WebhookCreatedResponse
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Liveness and metrics endpoints"),
        (name = "auth", description = "Login, token refresh, OIDC, and session management"),
        (name = "crawls", description = "Crawl lifecycle, statistics, findings, and backlinks"),
        (name = "api-keys", description = "API key management (admin)"),
        (name = "webhooks", description = "Crawl lifecycle webhook delivery configuration"),
        (name = "schedules", description = "Recurring crawl schedules"),
        (name = "audit", description = "Tamper-evident audit trail (admin)"),
        (name = "tenants", description = "Tenant management (admin)"),
        (name = "users", description = "User management (admin)"),
        (name = "marketplace", description = "Plugin marketplace")
    )
)]
pub struct ApiDoc;

/// Registers the security schemes referenced by path annotations.
///
/// Protected `/api/v1/*` routes require both schemes simultaneously:
/// an `X-API-Key` header (API key + rate limiting) and a Bearer JWT.
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
        );
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

/// Build the full OpenAPI document for this API.
#[must_use]
pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
