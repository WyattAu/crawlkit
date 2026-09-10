//! crawlkit-scanner binary: serves the ADR-012 hosted free-scan HTTP API.

use std::sync::Arc;

use crawlkit_scanner::api::{create_router, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let state = match AppState::new() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("fatal: scanner state initialization failed: {e}");
            std::process::exit(1);
        }
    };

    let port: u16 = std::env::var("CRAWLKIT_SCANNER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8090);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    let app = create_router(state);
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("fatal: failed to bind {addr}: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!(
        "crawlkit-scanner listening on {addr} — free-scan prototype (ADR-012)"
    );
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("fatal: server error: {e}");
        std::process::exit(1);
    }
}
