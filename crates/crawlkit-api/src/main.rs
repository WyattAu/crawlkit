//! REST API server for crawlkit — exposes crawl, compare, and report operations over HTTP.
//!
//! Built on Axum with API-key authentication and per-key rate limiting.
//! Start a crawl via `POST /api/v1/crawls` and poll status at
//! `GET /api/v1/crawls/{crawl_id}`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod auth;
mod auth_mw;
mod handlers;
mod middleware;
mod oidc;
mod router;
mod types;

use chrono::Utc;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use auth::AuthManager;
use handlers::schedules::run_scheduler;
use types::{ApiKey, AppState, MarketplaceState, Metrics};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Sentry error tracking
    let _sentry_guard = std::env::var("SENTRY_DSN").ok().map(|dsn| {
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                environment: Some(
                    std::env::var("ENVIRONMENT")
                        .unwrap_or_else(|_| "development".into())
                        .into(),
                ),
                traces_sample_rate: 0.1,
                ..Default::default()
            },
        ))
    });

    // Initialize tracing
    init_tracing();

    // Open storage
    let db_path =
        std::env::var("CRAWLKIT_DB_PATH").unwrap_or_else(|_| "crawlkit-api.db".to_string());
    let storage = crawlkit_engine::storage::Storage::new(std::path::Path::new(&db_path))
        .map_err(|e| anyhow::anyhow!("Failed to open storage: {e}"))?;

    // Generate a random API key for this session
    let default_key = format!("ck_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    eprintln!("Generated API key: {default_key}");
    let api_keys = Arc::new(DashMap::new());
    api_keys.insert(
        default_key.clone(),
        ApiKey {
            key: default_key,
            name: "development".to_string(),
            created_at: Utc::now(),
            requests_per_minute: 300,
        },
    );

    // JWT auth
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let secret: String = (0..32)
            .map(|_| format!("{:02x}", rng.gen::<u8>()))
            .collect();
        tracing::warn!(
            "JWT_SECRET not set. Generated random secret. Set JWT_SECRET env var for production."
        );
        secret
    });
    let auth = Arc::new(AuthManager::new(jwt_secret));

    // Admin account creation
    if std::env::var("CREATE_ADMIN").unwrap_or_default() == "true" {
        create_admin(&auth)?;
    }

    // OIDC setup
    let oidc = setup_oidc().await;

    // Build application state
    let state = AppState {
        storage: Arc::new(storage),
        api_keys,
        rate_limits: Arc::new(DashMap::new()),
        crawl_results: Arc::new(DashMap::new()),
        audit_trail: Arc::new(crawlkit_engine::AuditTrail::new()),
        metrics: Arc::new(Metrics::new()),
        webhooks: Arc::new(DashMap::new()),
        schedules: Arc::new(DashMap::new()),
        http_client: reqwest::Client::new(),
        auth,
        oidc,
        oidc_states: Arc::new(DashMap::new()),
        tenants: Arc::new(DashMap::new()),
        marketplace: MarketplaceState::new(),
        sessions: Arc::new(DashMap::new()),
    };

    // Build router
    let cors = router::build_cors();
    let csrf_origins = router::csrf_origins();
    let app = router::create_router(state.clone(), csrf_origins)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    tracing::info!("crawlkit API listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Spawn background scheduler
    tokio::spawn(async move {
        run_scheduler(state).await;
    });

    axum::serve(listener, app).await?;

    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    let use_otel = std::env::var("OTEL_EXPORTER").ok().as_deref() == Some("stdout");

    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("crawlkit_api=info")),
    );

    if use_otel {
        use opentelemetry::trace::TracerProvider;
        use tracing_opentelemetry::OpenTelemetryLayer;
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
            .build();
        let tracer = provider.tracer("crawlkit-api");
        let otel_layer = OpenTelemetryLayer::new(tracer);
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(otel_layer)
            .init();
    } else {
        tracing_subscriber::registry().with(fmt_layer).init();
    }
}

fn create_admin(auth: &AuthManager) -> anyhow::Result<()> {
    let admin_password: String = (0..24)
        .map(|_| {
            let idx = rand::random::<u8>() % 62;
            match idx {
                0..=9 => (b'0' + idx) as char,
                10..=35 => (b'a' + idx - 10) as char,
                36..=61 => (b'A' + idx - 36) as char,
                _ => unreachable!(),
            }
        })
        .collect();
    let admin_password_hash = auth
        .hash_password(&admin_password)
        .map_err(|e| anyhow::anyhow!("Failed to hash admin password: {e}"))?;
    auth.add_user(auth::User {
        id: uuid::Uuid::new_v4().to_string(),
        email: "admin@crawlkit.local".to_string(),
        name: "Admin".to_string(),
        password_hash: admin_password_hash,
        tenant_id: "default".to_string(),
        roles: vec!["admin".to_string()],
        enabled: true,
    });
    tracing::warn!(
        "Admin account created. Email: admin@crawlkit.local, Password: {}. CHANGE THIS PASSWORD IMMEDIATELY.",
        admin_password
    );
    Ok(())
}

async fn setup_oidc() -> Option<Arc<oidc::OidcManager>> {
    match (
        std::env::var("OIDC_PROVIDER"),
        std::env::var("OIDC_CLIENT_ID"),
        std::env::var("OIDC_DISCOVERY_URL"),
    ) {
        (Ok(provider), Ok(client_id), Ok(discovery_url)) => {
            let scopes: Vec<String> = std::env::var("OIDC_SCOPES")
                .unwrap_or_else(|_| "openid email profile".to_string())
                .split_whitespace()
                .map(String::from)
                .collect();
            let redirect_uri = std::env::var("OIDC_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:4000/api/v1/auth/oidc/callback".to_string());
            let client_secret_env = std::env::var("OIDC_CLIENT_SECRET_ENV")
                .unwrap_or_else(|_| "OIDC_CLIENT_SECRET".to_string());

            let config = oidc::OidcConfig {
                provider,
                client_id,
                client_secret_env,
                discovery_url,
                scopes,
                redirect_uri,
            };
            let manager = Arc::new(oidc::OidcManager::new(config));
            if let Err(e) = manager.discover().await {
                tracing::warn!("OIDC discovery failed: {e}. OIDC auth will not be available.");
                None
            } else {
                tracing::info!("OIDC authentication enabled");
                Some(manager)
            }
        }
        _ => {
            tracing::info!("OIDC not configured, using local auth only");
            None
        }
    }
}
