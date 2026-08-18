use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::types::*;

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub async fn metrics_endpoint(State(state): State<AppState>) -> Response {
    let registry = state.metrics.registry.read().await;
    let mut buffer = String::new();
    if let Err(e) = prometheus_client::encoding::text::encode(&mut buffer, &registry) {
        tracing::error!("Failed to encode metrics: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to encode metrics",
        )
            .into_response();
    }
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        buffer,
    )
        .into_response()
}
