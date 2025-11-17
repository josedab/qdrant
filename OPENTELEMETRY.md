# OpenTelemetry Distributed Tracing

This document describes the OpenTelemetry distributed tracing implementation in Qdrant, based on RFC-0004.

## Overview

OpenTelemetry distributed tracing provides end-to-end visibility into request processing across Qdrant's distributed architecture. This helps with:

- Debugging distributed queries
- Understanding time spent in each layer
- Correlating logs with traces
- Performance analysis and optimization

## Features

- Instrumented search operations (search, batch search, search groups, search matrix)
- Instrumented collection operations (create, update, delete)
- Support for multiple exporters:
  - **Jaeger** - for development and testing
  - **OTLP** (OpenTelemetry Protocol) - for production (Grafana Tempo, etc.)
  - **Console** - for local debugging

## Building with OpenTelemetry Support

To enable OpenTelemetry tracing, compile Qdrant with the appropriate feature flags:

### For Jaeger Exporter

```bash
cargo build --features opentelemetry-jaeger
```

### For OTLP Exporter (Grafana Tempo)

```bash
cargo build --features opentelemetry-otlp
```

### For Both Exporters

```bash
cargo build --features opentelemetry-jaeger,opentelemetry-otlp
```

## Configuration

OpenTelemetry is configured through the `logger.opentelemetry` section in your `config.yaml` file.

### Basic Configuration

```yaml
logger:
  opentelemetry:
    enabled: true
    service_name: "qdrant"
    exporter:
      type: jaeger
      endpoint: "localhost:6831"
```

### Configuration Options

- `enabled` (boolean): Enable or disable OpenTelemetry tracing
- `service_name` (string, optional): Service name to use in traces (defaults to "qdrant")
- `exporter.type` (string): Exporter type - `jaeger`, `otlp`, or `console`
- `exporter.endpoint` (string, optional): Exporter endpoint URL

### Exporter Endpoints

#### Jaeger
- UDP Agent: `localhost:6831` (default)
- HTTP Collector: `http://localhost:14268/api/traces`

#### OTLP (for Grafana Tempo, etc.)
- gRPC: `http://localhost:4317` (default)

## Example Configurations

### Development with Jaeger

```yaml
logger:
  opentelemetry:
    enabled: true
    exporter:
      type: jaeger
      endpoint: "localhost:6831"
```

### Production with Grafana Tempo

```yaml
logger:
  opentelemetry:
    enabled: true
    service_name: "qdrant-production"
    exporter:
      type: otlp
      endpoint: "http://tempo:4317"
```

### Local Debugging

```yaml
logger:
  opentelemetry:
    enabled: true
    exporter:
      type: console
```

## Setting Up Jaeger for Development

1. Run Jaeger all-in-one container:

```bash
docker run -d --name jaeger \
  -e COLLECTOR_ZIPKIN_HOST_PORT=:9411 \
  -p 6831:6831/udp \
  -p 6832:6832/udp \
  -p 5778:5778 \
  -p 16686:16686 \
  -p 4317:4317 \
  -p 4318:4318 \
  -p 14250:14250 \
  -p 14268:14268 \
  -p 14269:14269 \
  -p 9411:9411 \
  jaegertracing/all-in-one:latest
```

2. Access Jaeger UI at http://localhost:16686

## Setting Up Grafana Tempo

1. Create a `tempo.yaml` configuration file:

```yaml
server:
  http_listen_port: 3200

distributor:
  receivers:
    otlp:
      protocols:
        grpc:
          endpoint: 0.0.0.0:4317

storage:
  trace:
    backend: local
    local:
      path: /tmp/tempo/traces
```

2. Run Tempo:

```bash
docker run -d --name tempo \
  -p 3200:3200 \
  -p 4317:4317 \
  -v $(pwd)/tempo.yaml:/etc/tempo.yaml \
  grafana/tempo:latest \
  -config.file=/etc/tempo.yaml
```

3. Configure Grafana to use Tempo as a data source

## Instrumented Operations

The following operations are instrumented with tracing spans:

### Search Operations
- `search_points` - Point search
- `batch_search_points` - Batch search
- `search_point_groups` - Search with grouping
- `search_points_matrix_pairs` - Matrix search (pairs format)
- `search_points_matrix_offsets` - Matrix search (offsets format)
- `do_core_search_points` - Core search implementation

### Collection Operations
- `create_collection` - Collection creation
- `update_collection` - Collection updates
- `delete_collection` - Collection deletion

## Trace Context

Each trace includes the following context:
- Collection name
- Operation type
- Timing information
- Request parameters (sanitized)

## Performance Considerations

- OpenTelemetry adds minimal overhead when disabled
- When enabled with sampling, typical overhead is < 5%
- Use OTLP exporter for production (more efficient than Jaeger)
- Consider using tail-based sampling for high-volume deployments

## Troubleshooting

### Traces Not Appearing

1. Verify the feature flag is enabled during compilation
2. Check that `enabled: true` in configuration
3. Verify exporter endpoint is reachable
4. Check logs for OpenTelemetry initialization messages

### High Overhead

1. Ensure you're using the OTLP exporter for production
2. Consider implementing sampling
3. Review instrumented operations and remove unnecessary spans

## Architecture

The OpenTelemetry implementation consists of:

- **Configuration Module** (`src/tracing/opentelemetry.rs`) - Handles OpenTelemetry configuration and initialization
- **Tracing Integration** (`src/tracing/mod.rs`) - Integrates OpenTelemetry layer with existing tracing infrastructure
- **Instrumentation** - Uses `#[tracing::instrument]` macro on key functions

## Development

### Adding New Instrumentation

To add tracing to a new function:

```rust
#[tracing::instrument(skip(large_param), fields(important_field = %field_value))]
async fn my_function(param: Type, large_param: LargeType) -> Result<T> {
    // Function implementation
}
```

### Best Practices

- Skip large parameters in `skip()` to avoid trace bloat
- Include important identifiers in `fields()` for filtering
- Use meaningful span names
- Avoid instrumenting very frequent, low-level operations

## References

- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [Grafana Tempo Documentation](https://grafana.com/docs/tempo/latest/)
- [RFC-0004: Enhanced Observability](analysis-output/rfcs/RFC-0004-enhanced-observability.md)
