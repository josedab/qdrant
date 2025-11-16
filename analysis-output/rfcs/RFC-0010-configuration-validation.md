# RFC-0010: Configuration Validation Tool

**Status:** Draft
**Category:** Quick Win
**Estimated Effort:** 3-4 dev-days

## Summary
CLI tool to validate Qdrant configuration files before deployment.

## Usage
```bash
qdrant validate-config config/production.yaml

# Output
✅ Configuration is valid
⚠️  Warnings:
  - hnsw_index.m=64 is unusually high (recommended: 16-32)
  - storage.on_disk_payload=false may use significant RAM

❌ Errors:
  - service.grpc_port=6333 conflicts with http_port
```

## Implementation
```rust
fn validate_config(config: &Config) -> ValidationResult {
    // Check conflicts
    // Validate ranges
    // Performance recommendations
}
```

**Effort:** 3-4 dev-days
