//! Configuration validation tool
//!
//! Validates Qdrant configuration files for conflicts and best practices

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
    pub suggestion: String,
}

pub struct ConfigValidator {}

impl ConfigValidator {
    pub fn new() -> Self {
        Self {}
    }

    /// Validate a configuration file
    pub fn validate_file(&self, path: &Path) -> Result<ValidationResult, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        self.validate_yaml(&content)
    }

    /// Validate YAML configuration
    pub fn validate_yaml(&self, yaml: &str) -> Result<ValidationResult, Box<dyn std::error::Error>> {
        let config: serde_yaml::Value = serde_yaml::from_str(yaml)?;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate storage configuration
        if let Some(storage) = config.get("storage") {
            self.validate_storage(storage, &mut errors, &mut warnings);
        }

        // Validate cluster configuration
        if let Some(cluster) = config.get("cluster") {
            self.validate_cluster(cluster, &mut errors, &mut warnings);
        }

        // Validate service configuration
        if let Some(service) = config.get("service") {
            self.validate_service(service, &mut errors, &mut warnings);
        }

        let valid = errors.is_empty();

        Ok(ValidationResult {
            valid,
            errors,
            warnings,
        })
    }

    fn validate_storage(
        &self,
        storage: &serde_yaml::Value,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        // Check replication factor
        if let Some(rf) = storage.get("replication_factor").and_then(|v| v.as_u64()) {
            if rf == 0 {
                errors.push(ValidationError {
                    field: "storage.replication_factor".to_string(),
                    message: "Replication factor cannot be 0".to_string(),
                });
            } else if rf == 1 {
                warnings.push(ValidationWarning {
                    field: "storage.replication_factor".to_string(),
                    message: "Replication factor is 1, no redundancy".to_string(),
                    suggestion: "Set to 2 or 3 for production".to_string(),
                });
            }
        }

        // Check write consistency factor
        if let Some(wcf) = storage.get("write_consistency_factor").and_then(|v| v.as_u64()) {
            if let Some(rf) = storage.get("replication_factor").and_then(|v| v.as_u64()) {
                if wcf > rf {
                    errors.push(ValidationError {
                        field: "storage.write_consistency_factor".to_string(),
                        message: format!(
                            "Write consistency factor ({}) cannot exceed replication factor ({})",
                            wcf, rf
                        ),
                    });
                }
            }
        }

        // Check on_disk_payload with quantization
        if let Some(on_disk) = storage.get("on_disk_payload").and_then(|v| v.as_bool()) {
            if !on_disk {
                if let Some(_quant) = storage.get("quantization") {
                    warnings.push(ValidationWarning {
                        field: "storage.on_disk_payload".to_string(),
                        message: "Quantization enabled but payloads in RAM".to_string(),
                        suggestion: "Enable on_disk_payload to maximize memory savings".to_string(),
                    });
                }
            }
        }
    }

    fn validate_cluster(
        &self,
        cluster: &serde_yaml::Value,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        // Check if enabled
        if let Some(enabled) = cluster.get("enabled").and_then(|v| v.as_bool()) {
            if enabled {
                // Validate P2P port is set
                if cluster.get("p2p").and_then(|p| p.get("port")).is_none() {
                    errors.push(ValidationError {
                        field: "cluster.p2p.port".to_string(),
                        message: "P2P port must be set when cluster is enabled".to_string(),
                    });
                }

                // Warn about consensus timeout
                if let Some(consensus) = cluster.get("consensus") {
                    if let Some(tick_ms) = consensus.get("tick_period_ms").and_then(|v| v.as_u64()) {
                        if tick_ms < 50 {
                            warnings.push(ValidationWarning {
                                field: "cluster.consensus.tick_period_ms".to_string(),
                                message: format!("Tick period ({} ms) is very low", tick_ms),
                                suggestion: "Use at least 100ms for stability".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    fn validate_service(
        &self,
        service: &serde_yaml::Value,
        _errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        // Check max request size
        if let Some(max_size) = service.get("max_request_size_mb").and_then(|v| v.as_u64()) {
            if max_size < 10 {
                warnings.push(ValidationWarning {
                    field: "service.max_request_size_mb".to_string(),
                    message: format!("Max request size ({} MB) is low", max_size),
                    suggestion: "Consider increasing to 32MB or more for batch operations".to_string(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_config() {
        let validator = ConfigValidator::new();
        let yaml = r#"
storage:
  replication_factor: 2
  write_consistency_factor: 1
cluster:
  enabled: true
  p2p:
    port: 6335
"#;

        let result = validator.validate_yaml(yaml).unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_invalid_config() {
        let validator = ConfigValidator::new();
        let yaml = r#"
storage:
  replication_factor: 2
  write_consistency_factor: 3
"#;

        let result = validator.validate_yaml(yaml).unwrap();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_warnings() {
        let validator = ConfigValidator::new();
        let yaml = r#"
storage:
  replication_factor: 1
"#;

        let result = validator.validate_yaml(yaml).unwrap();
        assert!(result.valid);
        assert!(!result.warnings.is_empty());
    }
}
