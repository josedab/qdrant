# RFC-0004: Enhanced Observability with Distributed Tracing

**Status:** Draft
**Category:** Quick Win
**Estimated Effort:** 5-7 dev-days

## Summary
Add OpenTelemetry tracing for end-to-end request visibility.

## Motivation
- Hard to debug distributed queries
- Can't see time spent in each layer
- Missing context in logs

## Design
```rust
#[tracing::instrument]
async fn search_collection(query: SearchRequest) -> Result<SearchResponse> {
    let span = tracing::info_span!("search_collection");
    // ...
}
```

### Export to
- Jaeger (development)
- Grafana Tempo (production)
- Console (local debugging)

## Implementation
1. Add `tracing-opentelemetry` dependency
2. Instrument key functions
3. Configure exporter

**Effort:** 5-7 dev-days
