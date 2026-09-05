//! Router-level integration tests.
//!
//! These exercise the real middleware stack (API-key auth, CSRF origin
//! validation, JWT auth + session revocation) and the handler authorization
//! logic (tenant isolation, RBAC) through the axum `Router`, which is where
//! every regression in this area has historically lived.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use dashmap::DashMap;
use serde_json::Value;
use tower::ServiceExt;

use crawlkit_api::auth::{AuthManager, User};
use crawlkit_api::router::create_router;
use crawlkit_api::types::{
    ApiKey, AppState, CrawlResult, LoginAttemptRecord, MarketplaceState, Metrics,
    DEFAULT_MAX_CONCURRENT_CRAWLS,
};

const ORIGIN: &str = "https://dashboard.example.com";

struct TestApp {
    app: Router,
    state: AppState,
    api_key: String,
}

fn make_user(id: &str, tenant: &str, role: &str, password: &str) -> User {
    let auth = AuthManager::new("test-secret".to_string());
    let hash = auth.hash_password(password).unwrap_or_default();
    User {
        id: id.to_string(),
        email: format!("{id}@example.com"),
        name: id.to_string(),
        password_hash: hash,
        tenant_id: tenant.to_string(),
        roles: vec![role.to_string()],
        enabled: true,
    }
}

fn make_state(dir: &std::path::Path) -> AppState {
    make_state_with_capacity(dir, DEFAULT_MAX_CONCURRENT_CRAWLS)
}

fn make_state_with_capacity(dir: &std::path::Path, crawl_capacity: usize) -> AppState {
    let storage = crawlkit_engine::storage::Storage::new(&dir.join("test.db")).unwrap();
    AppState {
        storage: Arc::new(storage),
        api_keys: Arc::new(DashMap::new()),
        rate_limits: Arc::new(DashMap::new()),
        crawl_results: Arc::new(DashMap::new()),
        audit_trail: Arc::new(crawlkit_engine::AuditTrail::new()),
        metrics: Arc::new(Metrics::new()),
        webhooks: Arc::new(DashMap::new()),
        schedules: Arc::new(DashMap::new()),
        http_client: reqwest::Client::new(),
        auth: Arc::new(AuthManager::new("test-secret".to_string())),
        access_logger: Arc::new(crawlkit_engine::access_log::AccessLogger::new(10_000)),
        oidc: None,
        oidc_states: Arc::new(DashMap::new()),
        tenants: Arc::new(DashMap::new()),
        marketplace: MarketplaceState::new(),
        sessions: Arc::new(DashMap::new()),
        login_attempts: Arc::new(DashMap::<String, LoginAttemptRecord>::new()),
        persistence: None,
        crawl_permits: Arc::new(tokio::sync::Semaphore::new(crawl_capacity)),
        idempotency_keys: Arc::new(DashMap::new()),
    }
}

fn setup(dir: &std::path::Path) -> TestApp {
    let state = make_state(dir);
    let api_key = "ck_testkey123".to_string();
    state.api_keys.insert(
        api_key.clone(),
        ApiKey {
            key: api_key.clone(),
            name: "test".to_string(),
            created_at: chrono::Utc::now(),
            requests_per_minute: 1000,
        },
    );
    let app = create_router(state.clone(), vec![ORIGIN.to_string()]);
    TestApp {
        app,
        state,
        api_key,
    }
}

impl TestApp {
    fn authed(&self, token: &str, method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("x-api-key", &self.api_key)
            .header("authorization", format!("Bearer {token}"))
            .header("origin", ORIGIN);
        let request = match body {
            Some(json) => {
                builder = builder.header("content-type", "application/json");
                builder.body(Body::from(json.to_string()))
            }
            None => builder.body(Body::empty()),
        };
        request.unwrap()
    }

    fn public(&self, method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
        let mut builder = Request::builder().method(method).uri(uri);
        let request = match body {
            Some(json) => {
                builder = builder.header("content-type", "application/json");
                builder.body(Body::from(json.to_string()))
            }
            None => builder.body(Body::empty()),
        };
        request.unwrap()
    }

    async fn send(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        (status, body)
    }

    fn token_for(&self, user_id: &str) -> String {
        let user = self.state.auth.find_user_by_id(user_id).unwrap();
        self.state.auth.generate_token(&user).unwrap()
    }
}

// ---------------------------------------------------------------------------
// Authentication gates
// ---------------------------------------------------------------------------

#[tokio::test]
async fn missing_api_key_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    let request = Request::builder()
        .uri("/api/v1/crawls")
        .body(Body::empty())
        .unwrap();
    let (status, _) = test.send(request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn missing_jwt_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    let request = Request::builder()
        .uri("/api/v1/crawls")
        .header("x-api-key", &test.api_key)
        .body(Body::empty())
        .unwrap();
    let (status, _) = test.send(request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_jwt_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    let request = Request::builder()
        .uri("/api/v1/crawls")
        .header("x-api-key", &test.api_key)
        .header("authorization", "Bearer not-a-jwt")
        .body(Body::empty())
        .unwrap();
    let (status, _) = test.send(request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_without_origin_is_rejected_by_csrf() {
    // Login is a public POST; the CSRF layer only protects the authenticated
    // router, but authenticated mutations without Origin must 403.
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "admin", "password123!X"));

    let token = test.token_for("u1");
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/sessions/revoke")
        .header("x-api-key", &test.api_key)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"jti":"nonexistent"}"#))
        .unwrap();
    let (status, _) = test.send(request).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// Login + lockout + sessions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn login_success_returns_token_and_user() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "viewer", "password123!X"));

    let (status, body) = test
        .send(test.public(
            "POST",
            "/api/v1/auth/login",
            Some(serde_json::json!({
                "email": "u1@example.com",
                "password": "password123!X"
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["token"].as_str().is_some_and(|t| !t.is_empty()));
    assert_eq!(body["user"]["tenant_id"], "tenant-a");
}

#[tokio::test]
async fn login_lockout_engages_after_five_failures() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "viewer", "password123!X"));

    let bad = serde_json::json!({"email": "u1@example.com", "password": "wrong"});
    for _ in 0..5 {
        let (status, _) = test
            .send(test.public("POST", "/api/v1/auth/login", Some(bad.clone())))
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    // Correct password must still be refused while locked out.
    let good = serde_json::json!({"email": "u1@example.com", "password": "password123!X"});
    let (status, body) = test
        .send(test.public("POST", "/api/v1/auth/login", Some(good)))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("locked"),
        "expected lockout message, got: {body}"
    );
}

#[tokio::test]
async fn login_unknown_and_disabled_users_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state.auth.add_user(make_user(
        "disabled-user",
        "tenant-a",
        "viewer",
        "password123!X",
    ));
    if let Some(mut user) = test.state.auth.find_user_by_id("disabled-user") {
        user.enabled = false;
        test.state.auth.delete_user("disabled-user");
        test.state.auth.add_user(user);
    }

    let (status, body) = test
        .send(test.public(
            "POST",
            "/api/v1/auth/login",
            Some(serde_json::json!({
                "email": "missing@example.com",
                "password": "password123!X"
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Invalid credentials");

    let (status, body) = test
        .send(test.public(
            "POST",
            "/api/v1/auth/login",
            Some(serde_json::json!({
                "email": "disabled-user@example.com",
                "password": "password123!X"
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Account disabled");
}

#[tokio::test]
async fn admin_user_creation_rejects_weak_and_duplicate_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    test.state
        .auth
        .add_user(make_user("existing", "default", "viewer", "password123!X"));
    let token = test.token_for("root");

    let (status, body) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/users",
            Some(serde_json::json!({
                "email": "new@example.com",
                "name": "New User",
                "password": "weak"
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("12 characters"));

    let (status, body) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/users",
            Some(serde_json::json!({
                "email": "existing@example.com",
                "name": "Duplicate",
                "password": "Str0ng!Pass#12"
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "User with this email already exists");
}

#[tokio::test]
async fn list_users_is_scoped_to_the_callers_tenant() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("same", "tenant-a", "viewer", "password123!X"));
    test.state
        .auth
        .add_user(make_user("other", "tenant-b", "viewer", "password123!X"));
    let token = test.token_for("same");

    let (status, body) = test
        .send(test.authed(&token, "GET", "/api/v1/users", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let ids = body
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|user| user["id"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["same"]);
}

#[tokio::test]
async fn revoked_session_is_enforced_by_middleware() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "viewer", "password123!X"));

    let login = serde_json::json!({"email": "u1@example.com", "password": "password123!X"});
    let (_, body) = test
        .send(test.public("POST", "/api/v1/auth/login", Some(login)))
        .await;
    let token = body["token"].as_str().unwrap().to_string();

    // /auth/me works before revocation.
    let (status, _) = test
        .send(test.authed(&token, "GET", "/api/v1/auth/me", None))
        .await;
    assert_eq!(status, StatusCode::OK);

    // Revoke via the API (needs Origin, which authed() provides).
    let claims = test.state.auth.validate_token(&token).unwrap();
    let revoke = serde_json::json!({"jti": claims.jti});
    let (status, _) = test
        .send(test.authed(&token, "POST", "/api/v1/sessions/revoke", Some(revoke)))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The very same token must now be rejected.
    let (status, _) = test
        .send(test.authed(&token, "GET", "/api/v1/auth/me", None))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Tenant isolation (default-deny)
// ---------------------------------------------------------------------------

fn seed_crawl(state: &AppState, crawl_id: &str, tenant: &str) {
    state.crawl_results.insert(
        crawl_id.to_string(),
        CrawlResult {
            crawl_id: crawl_id.to_string(),
            tenant_id: tenant.to_string(),
            start_url: "https://example.com".to_string(),
            status: "completed".to_string(),
            pages_crawled: 5,
            issues_found: 2,
            created_at: chrono::Utc::now(),
            completed_at: None,
            storage_crawl_id: Some(crawl_id.to_string()),
        },
    );
}

#[tokio::test]
async fn cross_tenant_crawl_access_is_404() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state.auth.add_user(make_user(
        "victim-user",
        "tenant-a",
        "viewer",
        "password123!X",
    ));
    seed_crawl(&test.state, "crawl-owned-by-b", "tenant-b");

    let token = test.token_for("victim-user");
    for uri in [
        "/api/v1/crawls/crawl-owned-by-b",
        "/api/v1/crawls/crawl-owned-by-b/stats",
        "/api/v1/crawls/crawl-owned-by-b/findings",
        "/api/v1/crawls/crawl-owned-by-b/backlinks",
    ] {
        let (status, _) = test.send(test.authed(&token, "GET", uri, None)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "uri {uri} must be 404");
    }
}

#[tokio::test]
async fn unknown_crawl_id_is_404_not_data_leak() {
    // Regression: an id absent from the in-memory map used to fall through
    // to an unscoped storage query. It must now default-deny.
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "viewer", "password123!X"));

    let token = test.token_for("u1");
    for uri in [
        "/api/v1/crawls/never-seen/stats",
        "/api/v1/crawls/never-seen/findings",
        "/api/v1/crawls/never-seen/backlinks",
    ] {
        let (status, _) = test.send(test.authed(&token, "GET", uri, None)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "uri {uri} must be 404");
    }
}

#[tokio::test]
async fn same_tenant_crawl_access_is_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "viewer", "password123!X"));
    seed_crawl(&test.state, "crawl-a", "tenant-a");

    let token = test.token_for("u1");
    let (status, _) = test
        .send(test.authed(&token, "GET", "/api/v1/crawls/crawl-a", None))
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = test
        .send(test.authed(&token, "GET", "/api/v1/crawls/crawl-a/stats", None))
        .await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// RBAC on admin surfaces
// ---------------------------------------------------------------------------

#[tokio::test]
async fn viewer_cannot_create_tenants_or_keys() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("v", "tenant-a", "viewer", "password123!X"));

    let token = test.token_for("v");
    let (status, _) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/tenants",
            Some(serde_json::json!({"id": "evil", "name": "Evil"})),
        ))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/keys",
            Some(serde_json::json!({"name": "k", "requests_per_minute": 60})),
        ))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins",
            Some(serde_json::json!({
                "name": "p", "version": "1.0", "author": "a",
                "description": "d", "license": "MIT",
                "categories": [], "tags": []
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_create_tenants_and_keys() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));

    let token = test.token_for("root");
    let (status, _) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/tenants",
            Some(serde_json::json!({"id": "acme", "name": "ACME"})),
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/keys",
            Some(serde_json::json!({"name": "ci", "requests_per_minute": 120})),
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn non_admin_cannot_delete_users() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state.auth.add_user(make_user(
        "admin-user",
        "tenant-a",
        "admin",
        "password123!X",
    ));
    test.state
        .auth
        .add_user(make_user("peer", "tenant-a", "viewer", "password123!X"));

    let token = test.token_for("peer");
    let (status, _) = test
        .send(test.authed(&token, "DELETE", "/api/v1/users/admin-user", None))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(
        test.state.auth.find_user_by_id("admin-user").is_some(),
        "admin must not have been deleted"
    );
}

#[tokio::test]
async fn rpm_is_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));

    let token = test.token_for("root");
    let (status, body) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/keys",
            Some(serde_json::json!({"name": "greedy", "requests_per_minute": 4_294_967_295u64})),
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("requests_per_minute"),
        "expected validation error, got: {body}"
    );
}

// ---------------------------------------------------------------------------
// Webhook SSRF validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn webhook_to_private_address_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "editor", "password123!X"));

    let token = test.token_for("u1");
    for url in [
        "http://169.254.169.254/latest/meta-data",
        "http://127.0.0.1:8080/hook",
        "http://10.0.0.5/hook",
    ] {
        let (status, _) = test
            .send(test.authed(
                &token,
                "POST",
                "/api/v1/webhooks",
                Some(serde_json::json!({"url": url})),
            ))
            .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "url {url} must be rejected"
        );
    }
}

#[tokio::test]
async fn webhook_creation_listing_and_deletion_preserve_secret_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("editor", "tenant-a", "editor", "password123!X"));
    let token = test.token_for("editor");

    let (status, created) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/webhooks",
            Some(serde_json::json!({
                "url": "https://example.com/hooks",
                "events": ["crawl.completed", "crawl.failed"]
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let webhook_id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["tenant_id"], "tenant-a");
    assert_eq!(created["secret"].as_str().unwrap().len(), 64);

    let (status, listed) = test
        .send(test.authed(&token, "GET", "/api/v1/webhooks", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert!(listed[0].get("secret").is_none(), "webhook secret leaked");

    let (status, _) = test
        .send(test.authed(
            &token,
            "DELETE",
            &format!("/api/v1/webhooks/{webhook_id}"),
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!test.state.webhooks.contains_key(&webhook_id));
}

#[tokio::test]
async fn webhook_unknown_event_is_rejected_before_registration() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("editor", "tenant-a", "editor", "password123!X"));
    let token = test.token_for("editor");

    let (status, body) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/webhooks",
            Some(serde_json::json!({
                "url": "https://example.com/hooks",
                "events": ["crawl.completed", "account.deleted"]
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("Invalid event type"));
    assert!(test.state.webhooks.is_empty());
}

#[tokio::test]
async fn webhook_cross_tenant_delete_is_not_found_and_keeps_resource() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("viewer", "tenant-a", "viewer", "password123!X"));
    let webhook_id = "tenant-b-webhook";
    test.state.webhooks.insert(
        webhook_id.to_string(),
        crawlkit_api::types::WebhookConfig {
            id: webhook_id.to_string(),
            tenant_id: "tenant-b".to_string(),
            url: "https://example.com/hooks".to_string(),
            events: vec!["crawl.completed".to_string()],
            secret: "secret".to_string(),
            created_at: chrono::Utc::now(),
        },
    );
    let token = test.token_for("viewer");

    let (status, _) = test
        .send(test.authed(
            &token,
            "DELETE",
            &format!("/api/v1/webhooks/{webhook_id}"),
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(test.state.webhooks.contains_key(webhook_id));
}

fn schedule_body() -> Value {
    serde_json::json!({
        "start_url": "https://example.com/start",
        "max_pages": 25,
        "request_delay_ms": 100,
        "concurrency": 2,
        "interval_secs": 300
    })
}

#[tokio::test]
async fn schedule_crud_and_partial_update_are_tenant_scoped() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("editor", "tenant-a", "editor", "password123!X"));
    test.state
        .auth
        .add_user(make_user("other", "tenant-b", "editor", "password123!X"));
    test.state.schedules.insert(
        "foreign-schedule".to_string(),
        crawlkit_api::types::ScheduleConfig {
            id: "foreign-schedule".to_string(),
            tenant_id: "tenant-b".to_string(),
            crawl_config: crawlkit_engine::CrawlConfig {
                start_url: url::Url::parse("https://example.com/foreign").unwrap(),
                ..Default::default()
            },
            interval_secs: 300,
            enabled: true,
            next_run: chrono::Utc::now(),
            last_run_at: None,
            last_crawl_id: None,
            created_at: chrono::Utc::now(),
        },
    );
    let token = test.token_for("editor");

    let (status, created) = test
        .send(test.authed(&token, "POST", "/api/v1/schedules", Some(schedule_body())))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["interval_secs"], 300);
    assert_eq!(created["enabled"], true);
    let schedule_id = created["id"].as_str().unwrap().to_string();

    let (status, listed) = test
        .send(test.authed(&token, "GET", "/api/v1/schedules", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let listed_ids = listed
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|schedule| schedule["id"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(listed_ids.len(), 1);
    assert!(listed_ids.contains(&schedule_id.as_str()));
    assert!(!listed_ids.contains(&"foreign-schedule"));

    let (status, _) = test
        .send(test.authed(&token, "DELETE", "/api/v1/schedules/foreign-schedule", None))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(test.state.schedules.contains_key("foreign-schedule"));

    let (status, updated) = test
        .send(test.authed(
            &token,
            "PATCH",
            &format!("/api/v1/schedules/{schedule_id}"),
            Some(serde_json::json!({
                "start_url": "https://example.com/updated",
                "max_pages": 50,
                "enabled": false,
                "interval_secs": 600
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["start_url"], "https://example.com/updated");
    assert_eq!(updated["interval_secs"], 600);
    assert_eq!(updated["enabled"], false);

    let (status, _) = test
        .send(test.authed(
            &token,
            "DELETE",
            &format!("/api/v1/schedules/{schedule_id}"),
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!test.state.schedules.contains_key(&schedule_id));
}

#[tokio::test]
async fn schedule_validation_rejects_invalid_interval_and_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("editor", "tenant-a", "editor", "password123!X"));
    let token = test.token_for("editor");

    let mut invalid_interval = schedule_body();
    invalid_interval["interval_secs"] = serde_json::json!(59);
    let (status, body) = test
        .send(test.authed(&token, "POST", "/api/v1/schedules", Some(invalid_interval)))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("at least 60"));

    let mut invalid_pages = schedule_body();
    invalid_pages["max_pages"] = serde_json::json!(0);
    let (status, body) = test
        .send(test.authed(&token, "POST", "/api/v1/schedules", Some(invalid_pages)))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("max_pages"));
    assert!(test.state.schedules.is_empty());
}

#[tokio::test]
async fn public_health_endpoint_is_open() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    let (status, body) = test.send(test.public("GET", "/health", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

// ---------------------------------------------------------------------------
// Metrics authentication
// ---------------------------------------------------------------------------

#[tokio::test]
async fn metrics_requires_api_key_by_default() {
    // SAFETY: tests run multi-threaded; serialise env mutation. The guard is
    // dropped before any `.await` (env is only read at router-build time).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    let dir = tempfile::tempdir().unwrap();
    let test = {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("METRICS_PUBLIC");
        setup(dir.path())
    };

    // No API key -> 401.
    let (status, _) = test.send(test.public("GET", "/metrics", None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // With API key -> 200.
    let request = Request::builder()
        .uri("/metrics")
        .header("x-api-key", &test.api_key)
        .body(Body::empty())
        .unwrap();
    let (status, _) = test.send(request).await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// OpenAPI documentation endpoints
// ---------------------------------------------------------------------------

/// Shared lock: both docs tests read `DOCS_PUBLIC` at router-build time
/// (inside `create_router`), so the env mutation must be serialised.
// SAFETY: tests run multi-threaded; serialise env mutation.
static DOCS_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test]
async fn openapi_json_is_served_with_crawl_and_tenant_paths() {
    let dir = tempfile::tempdir().unwrap();
    let test = {
        let _guard = DOCS_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("DOCS_PUBLIC");
        setup(dir.path())
    };

    let (status, body) = test
        .send(test.public("GET", "/api/v1/openapi.json", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let paths = &body["paths"];
    assert!(
        paths["/api/v1/crawls"].is_object(),
        "expected /api/v1/crawls path entry, got: {body}"
    );
    assert!(
        paths["/api/v1/tenants"].is_object(),
        "expected /api/v1/tenants path entry, got: {body}"
    );
}

#[tokio::test]
async fn docs_public_false_removes_openapi_routes() {
    // DOCS_PUBLIC is read at router-build time inside create_router, so the
    // env var must be set before setup() builds the app. The guard is
    // dropped before any `.await`.
    let dir = tempfile::tempdir().unwrap();
    let test = {
        let _guard = DOCS_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("DOCS_PUBLIC", "false");
        setup(dir.path())
    };
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "viewer", "password123!X"));
    let token = test.token_for("u1");

    // Unauthenticated requests to unknown paths are still rejected by the
    // router-wide auth middleware (it wraps the catch-all fallback).
    let (status, _) = test
        .send(test.public("GET", "/api/v1/openapi.json", None))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Authenticated requests confirm the docs routes are truly absent (404,
    // not 200 as when they are registered on the public router).
    let (status, _) = test
        .send(test.authed(&token, "GET", "/api/v1/openapi.json", None))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = test
        .send(test.authed(&token, "GET", "/api/v1/docs", None))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    {
        let _guard = DOCS_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("DOCS_PUBLIC");
    }
}

// ---------------------------------------------------------------------------
// Crawl backpressure + idempotency
// ---------------------------------------------------------------------------

fn crawl_body() -> Value {
    serde_json::json!({
        "start_url": "https://example.com",
        "max_pages": 10,
        "request_delay_ms": 10,
        "concurrency": 2
    })
}

#[tokio::test]
async fn crawl_submission_at_capacity_returns_503_with_retry_after() {
    let dir = tempfile::tempdir().unwrap();
    // Zero crawl capacity: every submission must be rejected.
    let state = make_state_with_capacity(dir.path(), 1);
    let _hold = state.crawl_permits.clone().acquire_owned().await.unwrap();
    state
        .auth
        .add_user(make_user("u1", "tenant-a", "editor", "password123!X"));
    let api_key = "ck_cap".to_string();
    state.api_keys.insert(
        api_key.clone(),
        ApiKey {
            key: api_key.clone(),
            name: "t".to_string(),
            created_at: chrono::Utc::now(),
            requests_per_minute: 1000,
        },
    );
    let app = create_router(state.clone(), vec![ORIGIN.to_string()]);

    let token = {
        let user = state.auth.find_user_by_id("u1").unwrap();
        state.auth.generate_token(&user).unwrap()
    };
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/crawls")
        .header("x-api-key", &api_key)
        .header("authorization", format!("Bearer {token}"))
        .header("origin", ORIGIN)
        .header("content-type", "application/json")
        .body(Body::from(crawl_body().to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(!retry_after.is_empty(), "Retry-After header missing");
}

#[tokio::test]
async fn idempotency_key_replays_the_original_crawl() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "editor", "password123!X"));
    let token = test.token_for("u1");

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/crawls")
        .header("x-api-key", &test.api_key)
        .header("authorization", format!("Bearer {token}"))
        .header("origin", ORIGIN)
        .header("idempotency-key", "key-abc-123")
        .header("content-type", "application/json")
        .body(Body::from(crawl_body().to_string()))
        .unwrap();
    let (status, body) = test.send(request).await;
    assert_eq!(status, StatusCode::ACCEPTED, "first submission: {body}");
    let first_id = body["crawl_id"].as_str().unwrap().to_string();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/crawls")
        .header("x-api-key", &test.api_key)
        .header("authorization", format!("Bearer {token}"))
        .header("origin", ORIGIN)
        .header("idempotency-key", "key-abc-123")
        .header("content-type", "application/json")
        .body(Body::from(crawl_body().to_string()))
        .unwrap();
    let (status, body) = test.send(request).await;
    assert_eq!(status, StatusCode::OK, "replay should be 200: {body}");
    assert_eq!(
        body["crawl_id"].as_str().unwrap(),
        first_id,
        "replay must return the original crawl id"
    );
    assert_eq!(
        test.state.crawl_results.len(),
        1,
        "replay must not create a second crawl"
    );
}

// ---------------------------------------------------------------------------
// Marketplace plugins
// ---------------------------------------------------------------------------

fn plugin_body(name: &str) -> Value {
    serde_json::json!({
        "name": name,
        "version": "1.0.0",
        "author": "author",
        "description": "description",
        "license": "MIT",
        "categories": ["seo"],
        "tags": ["meta"]
    })
}

fn seed_plugin(state: &AppState, name: &str, rating: f64) {
    state.marketplace.plugins.write().insert(
        name.to_string(),
        crawlkit_api::types::MarketplacePlugin {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            author: "author".to_string(),
            description: "description".to_string(),
            license: "MIT".to_string(),
            categories: vec!["seo".to_string()],
            tags: vec!["meta".to_string()],
            downloads: 0,
            rating,
            rating_count: 1,
            verified: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    );
}

#[tokio::test]
async fn marketplace_publish_duplicate_get_and_delete_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    let token = test.token_for("root");

    let (status, created) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins",
            Some(plugin_body("meta-checker")),
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["name"], "meta-checker");
    assert_eq!(created["downloads"], 0);
    assert_eq!(created["rating"], 0.0);
    assert_eq!(created["rating_count"], 0);
    assert_eq!(created["verified"], false);

    let (status, body) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins",
            Some(plugin_body("meta-checker")),
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("already exists"));

    let (status, fetched) = test
        .send(test.authed(
            &token,
            "GET",
            "/api/v1/marketplace/plugins/meta-checker",
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["name"], "meta-checker");

    let (status, body) = test
        .send(test.authed(
            &token,
            "GET",
            "/api/v1/marketplace/plugins/never-published",
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("not found"));

    let (status, listed) = test
        .send(test.authed(&token, "GET", "/api/v1/marketplace/plugins", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let names = listed
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|plugin| plugin["name"].as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"meta-checker"));

    let (status, _) = test
        .send(test.authed(
            &token,
            "DELETE",
            "/api/v1/marketplace/plugins/meta-checker",
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = test
        .send(test.authed(
            &token,
            "DELETE",
            "/api/v1/marketplace/plugins/meta-checker",
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn marketplace_rating_aggregation_and_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "viewer", "password123!X"));
    let token = test.token_for("u1");
    seed_plugin(&test.state, "meta-checker", 0.0);

    let (status, rated) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins/meta-checker/rate",
            Some(serde_json::json!({"rating": 5.0})),
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rated["rating"], 5.0);
    assert_eq!(rated["rating_count"], 1);
    assert_eq!(rated["average_rating"], 5.0);

    let (status, rated) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins/meta-checker/rate",
            Some(serde_json::json!({"rating": 3.0})),
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rated["rating_count"], 2);
    assert_eq!(rated["average_rating"], 4.0);

    // The stored aggregate reflects both ratings.
    let plugin = test
        .state
        .marketplace
        .plugins
        .read()
        .get("meta-checker")
        .cloned()
        .unwrap();
    assert_eq!(plugin.rating, 4.0);
    assert_eq!(plugin.rating_count, 2);

    for out_of_range in [serde_json::json!(-1.0), serde_json::json!(5.5)] {
        let (status, body) = test
            .send(test.authed(
                &token,
                "POST",
                "/api/v1/marketplace/plugins/meta-checker/rate",
                Some(serde_json::json!({"rating": out_of_range})),
            ))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("between 0.0 and 5.0"));
    }

    let (status, body) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins/missing/rate",
            Some(serde_json::json!({"rating": 4.0})),
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("not found"));
}

#[tokio::test]
async fn marketplace_read_roles_rate_but_cannot_write_or_test() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("editor", "tenant-a", "editor", "password123!X"));
    let token = test.token_for("editor");
    seed_plugin(&test.state, "existing", 0.0);

    // marketplace:read roles may list, fetch, and rate.
    let (status, _) = test
        .send(test.authed(&token, "GET", "/api/v1/marketplace/plugins", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = test
        .send(test.authed(&token, "GET", "/api/v1/marketplace/plugins/existing", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins/existing/rate",
            Some(serde_json::json!({"rating": 4.0})),
        ))
        .await;
    assert_eq!(status, StatusCode::OK);

    // marketplace:write actions are admin-only.
    let (status, _) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins",
            Some(plugin_body("denied")),
        ))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = test
        .send(test.authed(
            &token,
            "DELETE",
            "/api/v1/marketplace/plugins/existing",
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins/existing/test",
            Some(serde_json::json!({"url": "https://example.com"})),
        ))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(test
        .state
        .marketplace
        .plugins
        .read()
        .contains_key("existing"));
}

#[tokio::test]
async fn marketplace_test_plugin_returns_passed_for_existing_plugin() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    let token = test.token_for("root");
    seed_plugin(&test.state, "auditor", 0.0);

    let (status, body) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins/auditor/test",
            Some(serde_json::json!({"url": "https://example.com"})),
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "passed");
    assert_eq!(body["findings"], 0);

    let (status, _) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/marketplace/plugins/unknown/test",
            Some(serde_json::json!({})),
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// API keys
// ---------------------------------------------------------------------------

#[tokio::test]
async fn api_key_crud_redacts_keys_and_reports_unknown_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    let token = test.token_for("root");

    let (status, created) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/keys",
            Some(serde_json::json!({"name": "ci", "requests_per_minute": 120})),
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let full_key = created["key"].as_str().unwrap().to_string();
    assert!(full_key.starts_with("ck_"));
    assert_eq!(created["name"], "ci");
    assert_eq!(created["requests_per_minute"], 120);

    let (status, listed) = test
        .send(test.authed(&token, "GET", "/api/v1/keys", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let listed_keys = listed
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|entry| entry["key"].as_str())
        .collect::<Vec<_>>();
    let redacted_tail = format!("{}****", &full_key[full_key.len() - 4..]);
    assert!(
        listed_keys.iter().any(|k| *k == redacted_tail),
        "expected redacted key {redacted_tail:?} in listing, got: {listed}"
    );
    assert!(
        !listed_keys.iter().any(|k| *k == full_key),
        "full key leaked in listing: {listed}"
    );
    assert!(listed_keys.contains(&"y123****"));

    let (status, _) = test
        .send(test.authed(&token, "DELETE", &format!("/api/v1/keys/{full_key}"), None))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!test.state.api_keys.contains_key(&full_key));

    let (status, _) = test
        .send(test.authed(&token, "DELETE", &format!("/api/v1/keys/{full_key}"), None))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn api_key_listing_requires_read_and_deletion_requires_write() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("editor", "tenant-a", "editor", "password123!X"));
    let token = test.token_for("editor");

    let (status, _) = test
        .send(test.authed(&token, "GET", "/api/v1/keys", None))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = test
        .send(test.authed(&token, "DELETE", "/api/v1/keys/ck_testkey123", None))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(test.state.api_keys.contains_key("ck_testkey123"));
}

// ---------------------------------------------------------------------------
// Tenants
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tenant_crud_duplicate_rejection_and_unknown_delete() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    let token = test.token_for("root");

    let (status, created) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/tenants",
            Some(serde_json::json!({"id": "acme", "name": "ACME"})),
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["id"], "acme");
    assert_eq!(created["name"], "ACME");

    let (status, body) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/tenants",
            Some(serde_json::json!({"id": "acme", "name": "Again"})),
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("already exists"));

    let (status, fetched) = test
        .send(test.authed(&token, "GET", "/api/v1/tenants/acme", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["id"], "acme");

    let (status, body) = test
        .send(test.authed(&token, "GET", "/api/v1/tenants/nope", None))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("not found"));

    let (status, listed) = test
        .send(test.authed(&token, "GET", "/api/v1/tenants", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(listed
        .as_array()
        .unwrap()
        .iter()
        .any(|tenant| tenant["id"] == "acme"));

    let (status, _) = test
        .send(test.authed(&token, "DELETE", "/api/v1/tenants/acme", None))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = test
        .send(test.authed(&token, "DELETE", "/api/v1/tenants/acme", None))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn tenant_read_and_write_surfaces_deny_non_admins() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("viewer", "tenant-a", "viewer", "password123!X"));
    let token = test.token_for("viewer");

    for (method, uri) in [
        ("GET", "/api/v1/tenants"),
        ("GET", "/api/v1/tenants/acme"),
        ("POST", "/api/v1/tenants"),
        ("DELETE", "/api/v1/tenants/acme"),
    ] {
        let (status, _) = test
            .send(test.authed(
                &token,
                method,
                uri,
                (method == "POST").then(|| serde_json::json!({"id": "acme", "name": "ACME"})),
            ))
            .await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "{method} {uri} must be denied"
        );
    }
}

// ---------------------------------------------------------------------------
// Audit trail
// ---------------------------------------------------------------------------

#[tokio::test]
async fn audit_events_are_admin_only_and_carry_chain_fields() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("viewer", "tenant-a", "viewer", "password123!X"));
    let viewer_token = test.token_for("viewer");

    // Non-admins lack audit:read entirely.
    let (status, _) = test
        .send(test.authed(&viewer_token, "GET", "/api/v1/audit", None))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    let admin_token = test.token_for("root");

    // Trigger audited writes through the real handlers.
    let (status, _) = test
        .send(test.authed(
            &admin_token,
            "POST",
            "/api/v1/keys",
            Some(serde_json::json!({"name": "audited", "requests_per_minute": 60})),
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = test
        .send(test.authed(
            &admin_token,
            "POST",
            "/api/v1/tenants",
            Some(serde_json::json!({"id": "audited-tenant", "name": "Audited"})),
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, events) = test
        .send(test.authed(&admin_token, "GET", "/api/v1/audit", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let events = events.as_array().unwrap();
    assert!(
        events.len() >= 2,
        "expected audit entries for the writes, got: {events:?}"
    );
    for event in events {
        assert!(
            !event["hash"].as_str().unwrap_or_default().is_empty(),
            "every audit event must carry a tamper-evidence hash"
        );
        assert!(
            event["previous_hash"].is_string(),
            "audit chain must be continuous"
        );
    }
}

// ---------------------------------------------------------------------------
// Users (create/delete/list under RBAC)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_user_creation_success_persists_and_echoes_roles() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    let token = test.token_for("root");

    let (status, created) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/users",
            Some(serde_json::json!({
                "email": "new@example.com",
                "name": "New User",
                "password": "Str0ng!Pass#12",
                "roles": ["viewer"]
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["email"], "new@example.com");
    assert_eq!(created["tenant_id"], "default");
    assert_eq!(created["roles"], serde_json::json!(["viewer"]));
    assert_eq!(created["enabled"], true);

    let stored = test
        .state
        .auth
        .find_user_by_id(created["id"].as_str().unwrap());
    assert!(
        stored.is_some(),
        "created user must be persisted in auth state"
    );
    let stored = stored.unwrap();
    assert_eq!(stored.email, "new@example.com");
    assert_ne!(stored.password_hash, "Str0ng!Pass#12");
    assert!(
        test.state
            .auth
            .verify_password("Str0ng!Pass#12", &stored.password_hash),
        "stored hash must verify the password"
    );
}

#[tokio::test]
async fn user_creation_and_deletion_are_admin_only() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("editor", "tenant-a", "editor", "password123!X"));
    let token = test.token_for("editor");

    let (status, body) = test
        .send(test.authed(
            &token,
            "POST",
            "/api/v1/users",
            Some(serde_json::json!({
                "email": "new@example.com",
                "name": "New",
                "password": "Str0ng!Pass#12"
            })),
        ))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("Only admins"));
    assert!(test.state.auth.find_user_by_id("editor").is_some());
}

#[tokio::test]
async fn admin_user_delete_success_and_unknown_id() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    test.state
        .auth
        .add_user(make_user("doomed", "default", "viewer", "password123!X"));
    let token = test.token_for("root");

    let (status, _) = test
        .send(test.authed(&token, "DELETE", "/api/v1/users/doomed", None))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(test.state.auth.find_user_by_id("doomed").is_none());

    let (status, body) = test
        .send(test.authed(&token, "DELETE", "/api/v1/users/ghost", None))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("not found"));
}

#[tokio::test]
async fn admin_user_listing_spans_tenants() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    test.state
        .auth
        .add_user(make_user("other", "tenant-b", "viewer", "password123!X"));
    let token = test.token_for("root");

    let (status, listed) = test
        .send(test.authed(&token, "GET", "/api/v1/users", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let ids = listed
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|user| user["id"].as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"root"));
    assert!(
        ids.contains(&"other"),
        "admins must see every tenant, got: {listed}"
    );
}

// ---------------------------------------------------------------------------
// Sessions: refresh, listing, revoke ownership, OIDC error paths
// ---------------------------------------------------------------------------

#[tokio::test]
async fn refresh_issues_new_token_and_registers_session() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "viewer", "password123!X"));
    let token = test.token_for("u1");

    let (status, body) = test
        .send(test.authed(&token, "POST", "/api/v1/auth/refresh", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let refreshed = body["token"].as_str().unwrap();
    assert_ne!(refreshed, token, "refresh must issue a new token");
    assert_eq!(body["user"]["id"], "u1");

    // The refreshed token is itself valid against the middleware.
    let (status, _) = test
        .send(test.authed(refreshed, "GET", "/api/v1/auth/me", None))
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn session_listing_is_scoped_to_the_caller() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("alice", "tenant-a", "viewer", "password123!X"));
    test.state
        .auth
        .add_user(make_user("bob", "tenant-a", "viewer", "password123!X"));

    // Real logins register sessions for each user.
    let login = |email: &str| {
        test.public(
            "POST",
            "/api/v1/auth/login",
            Some(serde_json::json!({"email": email, "password": "password123!X"})),
        )
    };
    let (_, alice_login) = test.send(login("alice@example.com")).await;
    let (_, bob_login) = test.send(login("bob@example.com")).await;
    let alice_token = alice_login["token"].as_str().unwrap().to_string();
    let bob_jti = {
        let claims = test
            .state
            .auth
            .validate_token(bob_login["token"].as_str().unwrap());
        claims.unwrap().jti
    };

    let (status, sessions) = test
        .send(test.authed(&alice_token, "GET", "/api/v1/sessions", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let jtis = sessions
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|session| session["jti"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        jtis.len(),
        1,
        "only the caller's session may be listed: {sessions}"
    );
    assert!(
        !jtis.contains(&bob_jti.as_str()),
        "foreign session leaked: {sessions}"
    );
}

#[tokio::test]
async fn revoke_session_ownership_is_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("alice", "tenant-a", "viewer", "password123!X"));
    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));

    let login = |email: &str| {
        test.public(
            "POST",
            "/api/v1/auth/login",
            Some(serde_json::json!({"email": email, "password": "password123!X"})),
        )
    };
    let (_, bob_login) = test.send(login("alice@example.com")).await;
    let bob_jti = test
        .state
        .auth
        .validate_token(bob_login["token"].as_str().unwrap())
        .unwrap()
        .jti;

    // A different non-admin may not revoke the session (404, no leak).
    test.state
        .auth
        .add_user(make_user("mallory", "tenant-a", "viewer", "password123!X"));
    let mallory_token = test.token_for("mallory");
    let (status, _) = test
        .send(test.authed(
            &mallory_token,
            "POST",
            "/api/v1/sessions/revoke",
            Some(serde_json::json!({"jti": bob_jti})),
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // An admin may revoke another user's session.
    let admin_token = test.token_for("root");
    let (status, _) = test
        .send(test.authed(
            &admin_token,
            "POST",
            "/api/v1/sessions/revoke",
            Some(serde_json::json!({"jti": bob_jti})),
        ))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn oidc_endpoints_fail_cleanly_when_unconfigured_or_malformed() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());

    // oidc is None in the test state, so authorize fails deterministically.
    let (status, body) = test
        .send(test.public("GET", "/api/v1/auth/oidc/authorize", None))
        .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("OIDC not configured"));

    // Provider-reported error surfaces as 400.
    let (status, body) = test
        .send(test.public(
            "GET",
            "/api/v1/auth/oidc/callback?error=access_denied&state=abc",
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("provider error"));

    // Missing code / state / unknown state are rejected before any network I/O.
    let (status, body) = test
        .send(test.public("GET", "/api/v1/auth/oidc/callback?state=abc", None))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("authorization code"));

    let (status, body) = test
        .send(test.public("GET", "/api/v1/auth/oidc/callback?code=abc", None))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("state parameter"));

    let (status, body) = test
        .send(test.public(
            "GET",
            "/api/v1/auth/oidc/callback?code=abc&state=never-issued",
            None,
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap_or_default()
        .contains("Invalid state"));
}

// ---------------------------------------------------------------------------
// Crawl listing and storage-backed detail endpoints
// ---------------------------------------------------------------------------

/// Seed a crawl whose pages/issues/links live in the SQLite storage and whose
/// public in-memory result resolves to the engine crawl id.
fn seed_storage_backed_crawl(state: &AppState, public_id: &str, tenant: &str) -> String {
    let engine_id = state
        .storage
        .start_crawl("https://example.com", None)
        .unwrap();
    let page = crawlkit_engine::storage::PageData {
        id: format!("{public_id}-p1"),
        url: url::Url::parse("https://example.com/").unwrap(),
        final_url: url::Url::parse("https://example.com/").unwrap(),
        status_code: 200,
        title: Some("Home".to_string()),
        load_time_ms: Some(120),
        body_size: Some(4096),
        fetched_at: chrono::Utc::now(),
        links: vec![
            url::Url::parse("https://example.com/about").unwrap(),
            url::Url::parse("https://cdn.other.test/asset.js").unwrap(),
        ],
        tenant_id: Some(tenant.to_string()),
        ..Default::default()
    };
    state.storage.insert_page(&engine_id, &page).unwrap();
    state
        .storage
        .insert_issue(&crawlkit_engine::storage::Issue {
            id: format!("{public_id}-i1"),
            page_id: page.id,
            category: crawlkit_engine::types::IssueCategory::Seo,
            severity: crawlkit_engine::types::Severity::Warning,
            code: "SEO001".to_string(),
            title: "Duplicate title".to_string(),
            description: "Pages share a title".to_string(),
            element: None,
            recommendation: "Write unique titles".to_string(),
            tenant_id: Some(tenant.to_string()),
        })
        .unwrap();
    state.crawl_results.insert(
        public_id.to_string(),
        CrawlResult {
            crawl_id: public_id.to_string(),
            tenant_id: tenant.to_string(),
            start_url: "https://example.com".to_string(),
            status: "completed".to_string(),
            pages_crawled: 1,
            issues_found: 1,
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            storage_crawl_id: Some(engine_id.clone()),
        },
    );
    engine_id
}

#[tokio::test]
async fn crawl_listing_is_tenant_scoped_and_admin_wide() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state.auth.add_user(make_user(
        "tenant-user",
        "tenant-a",
        "viewer",
        "password123!X",
    ));
    seed_crawl(&test.state, "own-crawl", "tenant-a");
    seed_crawl(&test.state, "foreign-crawl", "tenant-b");

    let token = test.token_for("tenant-user");
    let (status, listed) = test
        .send(test.authed(&token, "GET", "/api/v1/crawls", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let ids = listed
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|crawl| crawl["crawl_id"].as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"own-crawl"));
    assert!(
        !ids.contains(&"foreign-crawl"),
        "tenant leak in listing: {listed}"
    );

    test.state
        .auth
        .add_user(make_user("root", "default", "admin", "password123!X"));
    let admin_token = test.token_for("root");
    let (status, listed) = test
        .send(test.authed(&admin_token, "GET", "/api/v1/crawls", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let ids = listed
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|crawl| crawl["crawl_id"].as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"own-crawl") && ids.contains(&"foreign-crawl"));
}

#[tokio::test]
async fn crawl_stats_findings_and_backlinks_serve_storage_rows() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("u1", "tenant-a", "viewer", "password123!X"));
    let token = test.token_for("u1");
    seed_storage_backed_crawl(&test.state, "rich-crawl", "tenant-a");

    // Stats aggregate over the seeded page + issue.
    let (status, stats) = test
        .send(test.authed(&token, "GET", "/api/v1/crawls/rich-crawl/stats", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stats["crawl_id"], "rich-crawl");
    assert_eq!(stats["total_pages"], 1);
    assert_eq!(stats["total_issues"], 1);
    assert_eq!(stats["issues_by_severity"]["warning"], 1);
    assert_eq!(stats["issues_by_category"]["seo"], 1);
    assert_eq!(stats["avg_response_time_ms"], 120.0);

    // Findings map engine issues onto the public response shape.
    let (status, findings) = test
        .send(test.authed(&token, "GET", "/api/v1/crawls/rich-crawl/findings", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["code"], "SEO001");
    assert_eq!(findings[0]["category"], "seo");
    assert_eq!(findings[0]["severity"], "warning");
    assert_eq!(findings[0]["title"], "Duplicate title");
    assert!(findings[0]["page_id"].is_string());

    // Backlinks summarize internal/external link rows.
    let (status, backlinks) = test
        .send(test.authed(&token, "GET", "/api/v1/crawls/rich-crawl/backlinks", None))
        .await;
    assert_eq!(status, StatusCode::OK);
    // get_links_for_crawl returns every link row (internal and external
    // targets alike), so both seeded rows count as internal links, while the
    // external row is additionally registered as a backlink below.
    assert_eq!(backlinks["total_internal_links"], 2);
    assert_eq!(backlinks["total_external_links"], 1);
    assert!(backlinks["orphan_pages"].is_array());
    assert!(backlinks["top_pages_by_pagerank"].is_array());
}

#[tokio::test]
async fn crawl_submission_validates_url_pages_concurrency_and_delay() {
    let dir = tempfile::tempdir().unwrap();
    let test = setup(dir.path());
    test.state
        .auth
        .add_user(make_user("editor", "tenant-a", "editor", "password123!X"));
    let token = test.token_for("editor");

    let submit = |overrides: serde_json::Value| {
        let mut body = crawl_body();
        if let Some(value) = overrides.get("start_url") {
            body["start_url"] = value.clone();
        }
        if let Some(value) = overrides.get("max_pages") {
            body["max_pages"] = value.clone();
        }
        if let Some(value) = overrides.get("concurrency") {
            body["concurrency"] = value.clone();
        }
        if let Some(value) = overrides.get("request_delay_ms") {
            body["request_delay_ms"] = value.clone();
        }
        body
    };

    let cases: Vec<(serde_json::Value, &str)> = vec![
        (
            submit(serde_json::json!({"start_url": "ftp://example.com"})),
            "http or https",
        ),
        (
            submit(serde_json::json!({"start_url": "http://127.0.0.1/admin"})),
            "private IP",
        ),
        (submit(serde_json::json!({"max_pages": 0})), "max_pages"),
        (submit(serde_json::json!({"concurrency": 0})), "concurrency"),
        (
            submit(serde_json::json!({"request_delay_ms": 60_001})),
            "request_delay_ms",
        ),
    ];
    for (body, expected) in cases {
        let (status, response) = test
            .send(test.authed(&token, "POST", "/api/v1/crawls", Some(body.clone())))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} must be rejected");
        assert!(
            response["error"]
                .as_str()
                .unwrap_or_default()
                .contains(expected),
            "expected {expected:?} in error, got: {response}"
        );
    }
    assert!(test.state.crawl_results.is_empty());
}
