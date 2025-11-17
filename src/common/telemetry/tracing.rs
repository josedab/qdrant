//! OpenTelemetry tracing integration
//!
//! Provides distributed tracing for debugging and performance analysis

use opentelemetry::global;
use opentelemetry::sdk::export::trace::stdout;
use opentelemetry::sdk::trace::TracerProvider;
use opentelemetry::trace::{Span, SpanKind, Tracer};
use std::time::Instant;
use tracing::{info, instrument};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

/// Initialize OpenTelemetry tracing
pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    let exporter = stdout::new_pipeline().install_simple();

    let provider = TracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();

    global::set_tracer_provider(provider);

    let telemetry_layer = OpenTelemetryLayer::new(global::tracer("qdrant"));

    let subscriber = Registry::default().with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

/// Shutdown telemetry gracefully
pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}

/// Example instrumented function
#[instrument(skip(data))]
pub async fn example_traced_operation(name: &str, data: &[u8]) -> Result<usize, String> {
    let start = Instant::now();

    info!(name, data_len = data.len(), "Starting operation");

    // Simulate work
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let duration = start.elapsed();
    info!(name, ?duration, "Operation completed");

    Ok(data.len())
}

/// Trace a search operation
#[instrument(skip(query_vector))]
pub async fn trace_search(
    collection: &str,
    query_vector: &[f32],
    limit: usize,
) -> Result<Vec<u64>, String> {
    info!(collection, vector_dim = query_vector.len(), limit, "Search request");

    // Simulate search
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

    Ok(vec![1, 2, 3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tracing() {
        // init_telemetry().unwrap();

        let result = example_traced_operation("test", b"hello").await;
        assert!(result.is_ok());

        // shutdown_telemetry();
    }
}
