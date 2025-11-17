//! Dynamic shard rebalancing
//!
//! Automatically rebalances shards based on load and storage metrics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMetrics {
    pub shard_id: u32,
    pub node_id: u64,
    pub point_count: usize,
    pub disk_usage_bytes: u64,
    pub query_rate: f64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone)]
pub struct RebalancePlan {
    pub moves: Vec<ShardMove>,
    pub estimated_duration: Duration,
    pub estimated_traffic_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMove {
    pub shard_id: u32,
    pub from_node: u64,
    pub to_node: u64,
    pub reason: RebalanceReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RebalanceReason {
    LoadImbalance,
    DiskSpaceImbalance,
    NodeAdded,
    NodeRemoved,
}

pub struct ShardRebalancer {
    load_threshold: f64,
    disk_threshold: f64,
}

impl ShardRebalancer {
    pub fn new(load_threshold: f64, disk_threshold: f64) -> Self {
        Self {
            load_threshold,
            disk_threshold,
        }
    }

    /// Analyze cluster and create rebalancing plan
    pub fn create_rebalance_plan(
        &self,
        metrics: Vec<ShardMetrics>,
    ) -> Option<RebalancePlan> {
        let node_stats = self.aggregate_by_node(&metrics);

        if !self.needs_rebalancing(&node_stats) {
            return None;
        }

        let moves = self.calculate_moves(&metrics, &node_stats);

        if moves.is_empty() {
            return None;
        }

        let estimated_duration = self.estimate_duration(&moves, &metrics);
        let estimated_traffic_gb = self.estimate_traffic(&moves, &metrics);

        Some(RebalancePlan {
            moves,
            estimated_duration,
            estimated_traffic_gb,
        })
    }

    fn aggregate_by_node(&self, metrics: &[ShardMetrics]) -> HashMap<u64, NodeStats> {
        let mut node_stats: HashMap<u64, NodeStats> = HashMap::new();

        for metric in metrics {
            let stats = node_stats.entry(metric.node_id).or_insert(NodeStats {
                node_id: metric.node_id,
                total_points: 0,
                total_disk_bytes: 0,
                total_query_rate: 0.0,
                shard_count: 0,
            });

            stats.total_points += metric.point_count;
            stats.total_disk_bytes += metric.disk_usage_bytes;
            stats.total_query_rate += metric.query_rate;
            stats.shard_count += 1;
        }

        node_stats
    }

    fn needs_rebalancing(&self, node_stats: &HashMap<u64, NodeStats>) -> bool {
        if node_stats.len() < 2 {
            return false;
        }

        let avg_load: f64 = node_stats.values().map(|s| s.total_query_rate).sum::<f64>()
            / node_stats.len() as f64;

        let avg_disk: f64 = node_stats.values().map(|s| s.total_disk_bytes as f64).sum::<f64>()
            / node_stats.len() as f64;

        for stats in node_stats.values() {
            let load_deviation = (stats.total_query_rate - avg_load).abs() / avg_load;
            let disk_deviation = ((stats.total_disk_bytes as f64) - avg_disk).abs() / avg_disk;

            if load_deviation > self.load_threshold || disk_deviation > self.disk_threshold {
                return true;
            }
        }

        false
    }

    fn calculate_moves(
        &self,
        metrics: &[ShardMetrics],
        node_stats: &HashMap<u64, NodeStats>,
    ) -> Vec<ShardMove> {
        let mut moves = Vec::new();

        // Find overloaded and underloaded nodes
        let avg_load: f64 = node_stats.values().map(|s| s.total_query_rate).sum::<f64>()
            / node_stats.len() as f64;

        let mut overloaded: Vec<_> = node_stats
            .values()
            .filter(|s| s.total_query_rate > avg_load * (1.0 + self.load_threshold))
            .collect();

        let mut underloaded: Vec<_> = node_stats
            .values()
            .filter(|s| s.total_query_rate < avg_load * (1.0 - self.load_threshold))
            .collect();

        overloaded.sort_by(|a, b| b.total_query_rate.partial_cmp(&a.total_query_rate).unwrap());
        underloaded.sort_by(|a, b| a.total_query_rate.partial_cmp(&b.total_query_rate).unwrap());

        // Create moves
        for overloaded_node in &overloaded {
            if let Some(underloaded_node) = underloaded.first() {
                // Find shard to move
                if let Some(shard) = metrics
                    .iter()
                    .find(|m| m.node_id == overloaded_node.node_id)
                {
                    moves.push(ShardMove {
                        shard_id: shard.shard_id,
                        from_node: overloaded_node.node_id,
                        to_node: underloaded_node.node_id,
                        reason: RebalanceReason::LoadImbalance,
                    });

                    if moves.len() >= 5 {
                        // Limit moves per rebalance
                        break;
                    }
                }
            }
        }

        moves
    }

    fn estimate_duration(&self, moves: &[ShardMove], metrics: &[ShardMetrics]) -> Duration {
        let avg_move_time = Duration::from_secs(300); // 5 minutes per shard
        Duration::from_secs((moves.len() as u64) * avg_move_time.as_secs())
    }

    fn estimate_traffic(&self, moves: &[ShardMove], metrics: &[ShardMetrics]) -> f64 {
        let mut total_gb = 0.0;

        for mv in moves {
            if let Some(metric) = metrics.iter().find(|m| m.shard_id == mv.shard_id) {
                total_gb += metric.disk_usage_bytes as f64 / 1_000_000_000.0;
            }
        }

        total_gb
    }
}

#[derive(Debug, Clone)]
struct NodeStats {
    node_id: u64,
    total_points: usize,
    total_disk_bytes: u64,
    total_query_rate: f64,
    shard_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rebalancer() {
        let rebalancer = ShardRebalancer::new(0.3, 0.3);

        let metrics = vec![
            ShardMetrics {
                shard_id: 0,
                node_id: 1,
                point_count: 1000,
                disk_usage_bytes: 1_000_000,
                query_rate: 100.0,
                avg_latency_ms: 10.0,
            },
            ShardMetrics {
                shard_id: 1,
                node_id: 2,
                point_count: 100,
                disk_usage_bytes: 100_000,
                query_rate: 10.0,
                avg_latency_ms: 5.0,
            },
        ];

        let plan = rebalancer.create_rebalance_plan(metrics);
        assert!(plan.is_some());

        if let Some(plan) = plan {
            assert!(!plan.moves.is_empty());
        }
    }
}
