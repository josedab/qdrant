//! Enhanced error messages with context and hints
//!
//! Provides helpful error messages with documentation links

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Vec<String>,
    pub hints: Vec<String>,
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ErrorCode {
    InvalidVectorDimension,
    CollectionNotFound,
    InvalidFilter,
    MemoryExhausted,
    SegmentOptimizationFailed,
}

impl EnhancedError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        let message = message.into();
        let (hints, docs_url) = Self::generate_hints_and_docs(&code);

        Self {
            code,
            message,
            context: Vec::new(),
            hints,
            docs_url,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context.push(context.into());
        self
    }

    fn generate_hints_and_docs(code: &ErrorCode) -> (Vec<String>, Option<String>) {
        match code {
            ErrorCode::InvalidVectorDimension => (
                vec![
                    "Check that your vectors match the collection's configured dimension".to_string(),
                    "Use the collection info endpoint to verify dimension settings".to_string(),
                ],
                Some("https://qdrant.tech/documentation/concepts/collections/#vector-size".to_string()),
            ),
            ErrorCode::CollectionNotFound => (
                vec![
                    "List available collections with GET /collections".to_string(),
                    "Create a collection with PUT /collections/{name}".to_string(),
                ],
                Some("https://qdrant.tech/documentation/concepts/collections/".to_string()),
            ),
            ErrorCode::InvalidFilter => (
                vec![
                    "Check filter syntax in the documentation".to_string(),
                    "Ensure field names match your payload schema".to_string(),
                    "Create field indexes for better performance".to_string(),
                ],
                Some("https://qdrant.tech/documentation/concepts/filtering/".to_string()),
            ),
            ErrorCode::MemoryExhausted => (
                vec![
                    "Consider enabling quantization to reduce memory usage".to_string(),
                    "Use on-disk storage for vectors and payloads".to_string(),
                    "Increase available RAM or use distributed deployment".to_string(),
                ],
                Some("https://qdrant.tech/documentation/guides/quantization/".to_string()),
            ),
            ErrorCode::SegmentOptimizationFailed => (
                vec![
                    "Check disk space availability".to_string(),
                    "Review logs for detailed error messages".to_string(),
                    "Consider manual segment optimization with POST /collections/{name}/optimize".to_string(),
                ],
                Some("https://qdrant.tech/documentation/concepts/optimizer/".to_string()),
            ),
        }
    }
}

impl fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Error [{:?}]: {}", self.code, self.message)?;

        if !self.context.is_empty() {
            writeln!(f, "
Context:")?;
            for ctx in &self.context {
                writeln!(f, "  - {}", ctx)?;
            }
        }

        if !self.hints.is_empty() {
            writeln!(f, "
Hints:")?;
            for hint in &self.hints {
                writeln!(f, "  - {}", hint)?;
            }
        }

        if let Some(url) = &self.docs_url {
            writeln!(f, "
Documentation: {}", url)?;
        }

        Ok(())
    }
}

impl std::error::Error for EnhancedError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_error() {
        let error = EnhancedError::new(
            ErrorCode::InvalidVectorDimension,
            "Vector has 128 dimensions, expected 384",
        )
        .with_context("Collection: my_collection")
        .with_context("Point ID: 12345");

        let error_str = format!("{}", error);
        assert!(error_str.contains("InvalidVectorDimension"));
        assert!(error_str.contains("Hints:"));
        assert!(error_str.contains("Documentation:"));
    }
}
