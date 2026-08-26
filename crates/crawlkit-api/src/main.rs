//! REST API server for crawlkit — exposes crawl, compare, and report operations over HTTP.
//!
//! Built on Axum with API-key authentication and per-key rate limiting.
//! Start a crawl via `POST /api/v1/crawls` and poll status at
//! `GET /api/v1/crawls/{crawl_id}`.

use chrono::Utc;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crawlkit_api::handlers::schedules::run_scheduler;
use crawlkit_api::router;
use crawlkit_api::types::{ssrf_safe_redirect_policy, ApiKey, AppState, MarketplaceState, Metrics};
use crawlkit_api::{auth, auth::AuthManager, oidc};
use crawlkit_engine::storage_trait::StorageBackend;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Sentry error tracking
    let _sentry_guard = std::env::var("SENTRY_DSN").ok().map(|dsn| {
        sentry::init((dsn, {
            let mut opts = sentry::ClientOptions::default();
            opts.release = sentry::release_name!();
            opts.environment = Some(
                std::env::var("ENVIRONMENT")
                    .unwrap_or_else(|_| "development".into())
                    .into(),
            );
            opts.traces_sampling_strategy = sentry::TracesSamplingStrategy::FixedRate(0.1);
            opts
        }))
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
            key: default_key.clone(),
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

    // Server-side outbound HTTP client. All redirects are re-validated
    // against the SSRF blocklist on every hop (webhooks, OIDC endpoints).
    let http_client = reqwest::Client::builder()
        .redirect(ssrf_safe_redirect_policy())
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))?;

    // Audit trail: persistent + tamper-evident when AUDIT_LOG_PATH is set
    // (JSONL with SHA-256 chaining, fsync per event, truncation anchor).
    let audit_trail = match std::env::var("AUDIT_LOG_PATH") {
        Ok(path) if !path.is_empty() => {
            let trail = crawlkit_engine::AuditTrail::open_persistent(std::path::Path::new(&path))
                .map_err(|e| anyhow::anyhow!("Failed to open audit log at {path}: {e}"))?;
            tracing::info!("Audit trail persisted to {path} (tamper-evident, chained)");
            Arc::new(trail)
        }
        _ => Arc::new(crawlkit_engine::AuditTrail::new()),
    };

    // API-plane state persistence (users, tenants, API keys) so a restart
    // does not lose accounts. Postgres via API_STATE_PG_URL; otherwise
    // SQLite (API_STATE_DB_PATH, default `<db>.state`). Disabled with
    // API_STATE_DB_PATH="" (and no PG URL).
    let persistence: Option<Arc<dyn crawlkit_api::persistence::ApiStateStore>> =
        if let Ok(pg_url) = std::env::var("API_STATE_PG_URL") {
            if pg_url.is_empty() {
                None
            } else {
                let store = crawlkit_api::persistence::PgStateStore::open(&pg_url)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to open API state PG store: {e}"))?;
                tracing::info!("API state persisted to PostgreSQL");
                Some(Arc::new(store))
            }
        } else {
            match std::env::var("API_STATE_DB_PATH") {
                Ok(p) if !p.is_empty() => Some(Arc::new(
                    crawlkit_api::persistence::SqliteStateStore::open(std::path::Path::new(&p))
                        .map_err(|e| anyhow::anyhow!("Failed to open API state DB at {p}: {e}"))?,
                )),
                _ => {
                    let default_path = format!("{db_path}.state");
                    Some(Arc::new(
                        crawlkit_api::persistence::SqliteStateStore::open(std::path::Path::new(
                            &default_path,
                        ))
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to open API state DB at {default_path}: {e}")
                        })?,
                    ))
                }
            }
        };

    // Build application state
    let state = AppState {
        storage: Arc::new(storage) as Arc<dyn StorageBackend>,
        api_keys,
        rate_limits: Arc::new(DashMap::new()),
        crawl_results: Arc::new(DashMap::new()),
        audit_trail,
        metrics: Arc::new(Metrics::new()),
        webhooks: Arc::new(DashMap::new()),
        schedules: Arc::new(DashMap::new()),
        http_client,
        auth,
        oidc,
        oidc_states: Arc::new(DashMap::new()),
        tenants: Arc::new(DashMap::new()),
        marketplace: MarketplaceState::new(),
        sessions: Arc::new(DashMap::new()),
        login_attempts: Arc::new(DashMap::new()),
        persistence,
        crawl_permits: Arc::new(tokio::sync::Semaphore::new(
            crawlkit_api::types::crawl_capacity_from_env(),
        )),
        idempotency_keys: Arc::new(DashMap::new()),
    };

    // Restore persisted API-plane state into memory.
    if let Some(persistence) = &state.persistence {
        let users = persistence
            .load_users()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load users: {e}"))?;
        for user in users {
            state.auth.add_user(user);
        }

        let tenants = persistence
            .load_tenants()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load tenants: {e}"))?;
        for tenant in tenants {
            state.tenants.insert(tenant.id.clone(), tenant);
        }

        // Persist the bootstrap key so it survives restarts too.
        if let Some(persistence) = &state.persistence {
            if let Some(key) = state.api_keys.get(&default_key) {
                let _ = persistence.save_api_key(&key).await;
            }
        }

        let keys = persistence
            .load_api_keys()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load API keys: {e}"))?;
        for key in keys {
            state.api_keys.insert(key.key.clone(), key);
        }

        tracing::info!(
            "API state restored: {} users, {} tenants, {} API keys",
            state.auth.list_users().len(),
            state.tenants.len(),
            state.api_keys.len()
        );
    }

    // Build router
    let cors = router::build_cors();
    let csrf_origins = router::csrf_origins();
    let app = router::create_router(state.clone(), csrf_origins)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    if std::env::var("METRICS_PUBLIC")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true")
    {
        tracing::warn!(
            "METRICS_PUBLIC=true: /metrics is unauthenticated. Endpoint labels \
             and request rates are exposed without an API key."
        );
    }
    if router::docs_enabled() {
        tracing::info!(
            "API docs enabled: GET /api/v1/docs and GET /api/v1/openapi.json are public \
             (set DOCS_PUBLIC=false to disable)"
        );
    } else {
        tracing::info!("API docs disabled (DOCS_PUBLIC=false): docs routes return 404");
    }

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

    let use_otel_stdout = std::env::var("OTEL_EXPORTER").ok().as_deref() == Some("stdout");
    let otel_endpoint = std::env::var("OTEL_ENDPOINT").ok().filter(|e| !e.is_empty());

    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("crawlkit_api=info")),
    );

    if use_otel_stdout {
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
    } else if let Some(endpoint) = otel_endpoint {
        use opentelemetry::trace::TracerProvider;
        use opentelemetry_otlp::WithExportConfig;
        use tracing_opentelemetry::OpenTelemetryLayer;

        let exporter = match opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build()
        {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Failed to create OTLP span exporter: {e}");
                return;
            }
        };

        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_resource(
                opentelemetry_sdk::Resource::builder()
                    .with_attributes([opentelemetry::KeyValue::new(
                        "service.name",
                        "crawlkit-api",
                    )])
                    .build(),
            )
            .with_batch_exporter(exporter)
            .build();

        let tracer = provider.tracer("crawlkit-api");
        let otel_layer = OpenTelemetryLayer::new(tracer);
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(otel_layer)
            .init();

        // Keep provider alive for the process lifetime.
        // Store in a static so it is never dropped.
        use std::sync::OnceLock;
        static OTEL_PROVIDER: OnceLock<opentelemetry_sdk::trace::SdkTracerProvider> =
            OnceLock::new();
        let _ = OTEL_PROVIDER.set(provider);
    } else {
        tracing_subscriber::registry().with(fmt_layer).init();
    }
}

fn create_admin(auth: &AuthManager) -> anyhow::Result<()> {
    // Prefer an operator-supplied password; only generate (and print) one
    // when ADMIN_PASSWORD is not provided.
    let admin_password = match std::env::var("ADMIN_PASSWORD") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            use rand::seq::SliceRandom;
            const ALPHABET: &[u8] =
                b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
            let mut rng = rand::thread_rng();
            let password: String = (0..24)
                .map(|_| {
                    let idx = ALPHABET.choose(&mut rng).copied().unwrap_or(b'x');
                    idx as char
                })
                .collect();
            tracing::warn!(
                "Admin account created. Email: admin@crawlkit.local. \
                 Generated password printed once below — set ADMIN_PASSWORD to control it. \
                 CHANGE THIS PASSWORD IMMEDIATELY. Password: {password}"
            );
            password
        }
    };
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
