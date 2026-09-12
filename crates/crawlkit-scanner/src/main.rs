//! crawlkit-scanner binary: serves the ADR-012 hosted free-scan HTTP API.
//!
//! Execution posture (ADR-015 §A1, selected via `CRAWLKIT_BUDGET_BACKEND`):
//!
//! - `in-memory` (default): scans run inline in the HTTP handler; no Redis
//!   required.
//! - `redis`: submissions enqueue jobs and the worker pool in this process
//!   executes them against the same trust path; results land in shared
//!   Redis keys so any replica can serve any token. The reclaim sweeper
//!   recovers jobs from crashed workers after the lease TTL.

use std::sync::Arc;
#[cfg(feature = "shared-budget")]
use std::time::Duration;

use crawlkit_scanner::api::{create_router, AppState};
#[cfg(feature = "shared-budget")]
use crawlkit_scanner::worker;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let state = match AppState::from_env() {
        Ok(s) => {
            tracing::info!(
                budget_backend = ?s.budget_backend,
                queue_mode = s.queue_mode(),
                "budget posture selected (ADR-015 §A1)"
            );
            Arc::new(s)
        }
        Err(e) => {
            eprintln!("fatal: scanner state initialization failed: {e}");
            std::process::exit(1);
        }
    };

    // Multi-replica posture: spawn the scan worker pool and the lease
    // reclaim sweeper alongside the HTTP server.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    #[cfg(not(feature = "shared-budget"))]
    let _ = shutdown_rx;
    #[cfg(feature = "shared-budget")]
    if state.queue_mode() {
        let Some(redis_url) = state.redis_url.clone() else {
            eprintln!("fatal: queue mode without a Redis URL is unreachable");
            std::process::exit(1);
        };
        let concurrency: usize = std::env::var("CRAWLKIT_SCANNER_WORKERS")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(2);
        let poll_ms: u64 = std::env::var("CRAWLKIT_SCANNER_POLL_MS")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(500);

        let sweeper_url = redis_url.clone();
        let sweeper_shutdown = shutdown_rx.clone();
        tokio::spawn(async move {
            let reclaimed =
                worker::run_scanner_sweeper(&sweeper_url, Duration::from_secs(5), sweeper_shutdown)
                    .await;
            tracing::info!(reclaimed, "scanner sweeper stopped");
        });

        let worker_url = redis_url;
        let worker_shutdown = shutdown_rx.clone();
        let worker_state = state.clone();
        tokio::spawn(async move {
            worker::run_worker_pool(
                &worker_url,
                move || worker_state.deps(),
                concurrency,
                Duration::from_millis(poll_ms),
                worker_shutdown,
            )
            .await;
        });
        tracing::info!(
            workers = concurrency,
            poll_ms,
            "scanner worker pool started (multi-replica posture)"
        );
    }

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

    tracing::info!("crawlkit-scanner listening on {addr} — free-scan (ADR-012)");
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("fatal: server error: {e}");
        let _ = shutdown_tx.send(true);
        std::process::exit(1);
    }
}
