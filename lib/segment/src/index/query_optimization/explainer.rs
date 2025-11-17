//! Query execution plan explainer
//!
//! Provides detailed insights into how queries are executed,
//! helping users understand performance characteristics.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExplanation {
    /// Total estimated cost
    pub total_cost: f64,
    /// Strategy chosen
    pub strategy: QueryStrategy,
    /// Breakdown of operations
    pub operations: Vec<Operation>,
    /// Cardinality estimates
    pub cardinality: CardinalityInfo,
    /// Index usage
    pub indexes_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryStrategy {
    PlainSearch,
    PreFilter,
    PostFilter,
    FilteredHNSW,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub name: String,
    pub cost: f64,
    pub estimated_rows: usize,
    pub actual_rows: Option<usize>,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardinalityInfo {
    pub min: usize,
    pub expected: usize,
    pub max: usize,
}

pub struct QueryExplainer {
    total_points: usize,
}

impl QueryExplainer {
    pub fn new(total_points: usize) -> Self {
        Self { total_points }
    }

    pub fn explain(
        &self,
        has_filter: bool,
        filter_selectivity: f64,
        limit: usize,
    ) -> QueryExplanation {
        let cardinality = self.estimate_cardinality(filter_selectivity);

        let strategy = self.choose_strategy(filter_selectivity, limit);

        let operations = self.explain_operations(&strategy, &cardinality, limit);

        let total_cost = operations.iter().map(|op| op.cost).sum();

        let indexes_used = if has_filter {
            vec!["field_index".to_string(), "hnsw".to_string()]
        } else {
            vec!["hnsw".to_string()]
        };

        QueryExplanation {
            total_cost,
            strategy,
            operations,
            cardinality,
            indexes_used,
        }
    }

    fn estimate_cardinality(&self, selectivity: f64) -> CardinalityInfo {
        let expected = (self.total_points as f64 * selectivity) as usize;

        CardinalityInfo {
            min: expected / 2,
            expected,
            max: expected * 2,
        }
    }

    fn choose_strategy(&self, selectivity: f64, _limit: usize) -> QueryStrategy {
        if selectivity > 0.9 {
            QueryStrategy::PostFilter
        } else if selectivity < 0.1 {
            QueryStrategy::PreFilter
        } else if selectivity < 0.5 {
            QueryStrategy::FilteredHNSW
        } else {
            QueryStrategy::PlainSearch
        }
    }

    fn explain_operations(
        &self,
        strategy: &QueryStrategy,
        cardinality: &CardinalityInfo,
        limit: usize,
    ) -> Vec<Operation> {
        match strategy {
            QueryStrategy::PlainSearch => vec![Operation {
                name: "HNSW Search".to_string(),
                cost: (self.total_points as f64).log2() * 100.0,
                estimated_rows: limit,
                actual_rows: None,
                duration: None,
            }],
            QueryStrategy::PreFilter => vec![
                Operation {
                    name: "Apply Filter".to_string(),
                    cost: self.total_points as f64 * 5.0,
                    estimated_rows: cardinality.expected,
                    actual_rows: None,
                    duration: None,
                },
                Operation {
                    name: "HNSW Search on Filtered".to_string(),
                    cost: (cardinality.expected as f64).log2() * 100.0,
                    estimated_rows: limit,
                    actual_rows: None,
                    duration: None,
                },
            ],
            QueryStrategy::PostFilter => vec![
                Operation {
                    name: "HNSW Search".to_string(),
                    cost: (self.total_points as f64).log2() * 100.0,
                    estimated_rows: limit * 3,
                    actual_rows: None,
                    duration: None,
                },
                Operation {
                    name: "Apply Filter".to_string(),
                    cost: (limit * 3) as f64 * 5.0,
                    estimated_rows: limit,
                    actual_rows: None,
                    duration: None,
                },
            ],
            QueryStrategy::FilteredHNSW => vec![Operation {
                name: "Filtered HNSW Search".to_string(),
                cost: (self.total_points as f64).log2() * 150.0,
                estimated_rows: limit,
                actual_rows: None,
                duration: None,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_plain_search() {
        let explainer = QueryExplainer::new(1_000_000);
        let explanation = explainer.explain(false, 1.0, 10);

        assert!(matches!(explanation.strategy, QueryStrategy::PostFilter));
        assert!(explanation.total_cost > 0.0);
    }

    #[test]
    fn test_explain_pre_filter() {
        let explainer = QueryExplainer::new(1_000_000);
        let explanation = explainer.explain(true, 0.05, 10);

        assert!(matches!(explanation.strategy, QueryStrategy::PreFilter));
        assert_eq!(explanation.operations.len(), 2);
    }
}
