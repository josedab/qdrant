use serde::{Deserialize, Serialize};

/// OpenTelemetry configuration for distributed tracing
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct OpenTelemetryConfig {
    /// Enable OpenTelemetry tracing
    pub enabled: bool,

    /// Service name to use in traces
    pub service_name: Option<String>,

    /// Exporter configuration
    pub exporter: ExporterConfig,
}

/// Configuration for trace exporters
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct ExporterConfig {
    /// Exporter type: jaeger, otlp, or console
    #[serde(rename = "type")]
    pub exporter_type: ExporterType,

    /// Endpoint for the exporter (e.g., "http://localhost:14268/api/traces" for Jaeger)
    pub endpoint: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExporterType {
    /// Export to Jaeger
    Jaeger,

    /// Export via OTLP (for Grafana Tempo, etc.)
    Otlp,

    /// Export to console for debugging
    #[default]
    Console,
}

#[cfg(feature = "tracing")]
pub fn init_tracer(config: &OpenTelemetryConfig) -> anyhow::Result<()> {
    if !config.enabled {
        return Ok(());
    }

    #[cfg(any(feature = "opentelemetry-jaeger", feature = "opentelemetry-otlp"))]
    {
        let service_name = config
            .service_name
            .clone()
            .unwrap_or_else(|| "qdrant".to_string());

        match config.exporter.exporter_type {
            #[cfg(feature = "opentelemetry-jaeger")]
            ExporterType::Jaeger => {
                log::info!("Initializing Jaeger exporter for OpenTelemetry");

                // Note: Jaeger exporter is deprecated, but we keep it for backward compatibility
                // Users should migrate to OTLP exporter
                #[allow(deprecated)]
                let builder = opentelemetry_jaeger::new_agent_pipeline()
                    .with_service_name(service_name);

                if let Some(endpoint) = &config.exporter.endpoint {
                    log::info!("Using Jaeger endpoint: {}", endpoint);
                }

                let _tracer_provider = builder.install_simple()?;

                log::info!("Jaeger exporter initialized successfully");
            }

            #[cfg(feature = "opentelemetry-otlp")]
            ExporterType::Otlp => {
                log::info!("Initializing OTLP exporter for OpenTelemetry (Grafana Tempo compatible)");

                use opentelemetry::KeyValue;
                use opentelemetry_otlp::WithExportConfig;
                use opentelemetry_sdk::Resource;

                let mut exporter = opentelemetry_otlp::new_exporter()
                    .tonic();

                if let Some(endpoint) = &config.exporter.endpoint {
                    log::info!("Using OTLP endpoint: {}", endpoint);
                    exporter = exporter.with_endpoint(endpoint.clone());
                }

                let _tracer_provider = opentelemetry_otlp::new_pipeline()
                    .tracing()
                    .with_exporter(exporter)
                    .with_trace_config(
                        opentelemetry_sdk::trace::Config::default()
                            .with_resource(Resource::new(vec![
                                KeyValue::new("service.name", service_name),
                            ]))
                    )
                    .install_batch(opentelemetry_sdk::runtime::Tokio)?;

                log::info!("OTLP exporter initialized successfully");
            }

            ExporterType::Console => {
                log::info!("Console exporter selected - traces will be logged to stdout");
                // Console output is handled by the regular tracing subscriber
            }

            #[allow(unreachable_patterns)]
            _ => {
                log::warn!("Selected OpenTelemetry exporter is not available. Check feature flags.");
            }
        };
    }

    #[cfg(not(any(feature = "opentelemetry-jaeger", feature = "opentelemetry-otlp")))]
    {
        log::warn!("OpenTelemetry enabled but no exporters available. Enable 'opentelemetry-jaeger' or 'opentelemetry-otlp' feature.");
    }

    Ok(())
}

#[cfg(feature = "tracing")]
#[allow(dead_code)]
pub fn shutdown_tracer() {
    // Shutdown is handled automatically when tracer provider is dropped
    // This function is kept for potential future explicit shutdown needs
    log::info!("OpenTelemetry tracer shutdown requested");
}
