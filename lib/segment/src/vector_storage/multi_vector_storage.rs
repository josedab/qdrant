//! Multi-vector support for advanced search patterns
//!
//! Enables storing and searching multiple vectors per point

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VectorStorage {
    Single(Vec<f32>),
    Multi(HashMap<String, Vec<f32>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiVectorPoint {
    pub id: u64,
    pub vectors: VectorStorage,
    pub payload: serde_json::Value,
}

pub struct MultiVectorIndex {
    vectors: HashMap<u64, HashMap<String, Vec<f32>>>,
}

impl MultiVectorIndex {
    pub fn new() -> Self {
        Self {
            vectors: HashMap::new(),
        }
    }

    /// Insert a multi-vector point
    pub fn insert(
        &mut self,
        point_id: u64,
        vectors: HashMap<String, Vec<f32>>,
    ) -> Result<(), String> {
        // Validate all vectors have same dimension within each named vector
        for (name, vector) in &vectors {
            if vector.is_empty() {
                return Err(format!("Empty vector for name: {}", name));
            }
        }

        self.vectors.insert(point_id, vectors);
        Ok(())
    }

    /// Get specific named vector
    pub fn get_vector(&self, point_id: u64, vector_name: &str) -> Option<&Vec<f32>> {
        self.vectors
            .get(&point_id)
            .and_then(|vecs| vecs.get(vector_name))
    }

    /// Search using multi-vector fusion
    pub fn search_multi(
        &self,
        queries: HashMap<String, Vec<f32>>,
        limit: usize,
    ) -> Vec<(u64, f32)> {
        let mut scores: HashMap<u64, f32> = HashMap::new();

        for (point_id, point_vectors) in &self.vectors {
            let mut total_score = 0.0;
            let mut count = 0;

            for (query_name, query_vector) in &queries {
                if let Some(point_vector) = point_vectors.get(query_name) {
                    let score = Self::cosine_similarity(query_vector, point_vector);
                    total_score += score;
                    count += 1;
                }
            }

            if count > 0 {
                scores.insert(*point_id, total_score / count as f32);
            }
        }

        let mut results: Vec<_> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(limit);
        results
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_vector_insert_and_search() {
        let mut index = MultiVectorIndex::new();

        // Insert point with multiple vectors
        let mut vectors = HashMap::new();
        vectors.insert("image".to_string(), vec![0.1, 0.2, 0.3]);
        vectors.insert("text".to_string(), vec![0.4, 0.5, 0.6]);

        index.insert(1, vectors).unwrap();

        // Search
        let mut queries = HashMap::new();
        queries.insert("image".to_string(), vec![0.1, 0.2, 0.3]);

        let results = index.search_multi(queries, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }
}
