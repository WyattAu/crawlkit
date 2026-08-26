//! OpenTelemetry OTLP trace exporter initialization.
//!
//! Provides [`init_tracing_pipeline`] to set up a gRPC-based OTLP exporter
//! for distributed tracing. The returned [`SdkTracerProvider`] guard must be
//! held alive until process shutdown so that pending spans are flushed.

use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::time::Duration;

/// Error returned when the OTLP tracing pipeline fails to initialize.
#[derive(Debug, thiserror::Error)]
pub enum OtelError {
    /// The OTLP gRPC exporter could not be built.
    #[error("failed to create OTLP exporter: {0}")]
    Exporter(#[from] opentelemetry_otlp::ExporterBuildError),

    /// The tracer provider could not be built.
    #[error("failed to build tracer provider: {0}")]
    Provider(String),
}

/// Initialize a global OTLP tracing pipeline.
///
/// Creates an OTLP gRPC exporter pointing at `endpoint` (e.g.
/// `"http://localhost:4317"`), wraps it in a [`SdkTracerProvider`], and
/// installs it as the process-global default via
/// [`opentelemetry::global::set_tracer_provider`].
///
/// Returns a [`SdkTracerProvider`]. Dropping it shuts down the exporter;
/// keep it alive until the process exits.
///
/// # Examples
///
/// ```rust,no_run
/// # fn example() -> Result<(), crawlkit_engine::observability::otel::OtelError> {
/// use crawlkit_engine::observability::otel;
///
/// let provider = otel::init_tracing_pipeline("http://localhost:4317")?;
/// // … run application …
/// provider.shutdown()?;
/// # Ok(())
/// # }
/// ```
pub fn init_tracing_pipeline(endpoint: &str) -> Result<SdkTracerProvider, OtelError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_secs(10))
        .build()?;

    let resource = opentelemetry_sdk::Resource::builder()
        .with_attributes([opentelemetry::KeyValue::new(
            "service.name",
            "crawlkit-engine",
        )])
        .build();

    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    opentelemetry::global::set_tracer_provider(provider.clone());

    Ok(provider)
}

/// Initialize a tracing pipeline using the `OTEL_ENDPOINT` environment
/// variable. If the variable is not set or empty, this is a no-op and
/// returns `Ok(None)`.
///
/// This is the main entry point used by the crawl engine and API server.
pub fn init_from_env() -> Result<Option<SdkTracerProvider>, OtelError> {
    match std::env::var("OTEL_ENDPOINT") {
        Ok(endpoint) if !endpoint.is_empty() => {
            tracing::info!(endpoint = %endpoint, "Initializing OTLP tracing pipeline");
            Ok(Some(init_tracing_pipeline(&endpoint)?))
        }
        _ => Ok(None),
    }
}
