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
        oidc: None,
        oidc_states: Arc::new(DashMap::new()),
        tenants: Arc::new(DashMap::new()),
        marketplace: MarketplaceState::new(),
        sessions: Arc::new(DashMap::new()),
        login_attempts: Arc::new(DashMap::<String, LoginAttemptRecord>::new()),
        persistence: None,
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
