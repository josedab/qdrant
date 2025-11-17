//! Test cluster framework for integration tests
//!
//! This module provides a reusable test harness for creating and managing
//! multi-node clusters in integration tests. It reduces boilerplate and
//! makes it easier to test distributed scenarios.

use std::collections::HashMap;
use std::num::NonZeroU32;

use collection::collection::Collection;
use collection::config::{CollectionConfigInternal, CollectionParams, WalConfig};
use collection::operations::point_ops::{
    BatchPersisted, BatchVectorStructPersisted, PointInsertOperationsInternal, PointOperations,
    WriteOrdering,
};
use collection::operations::shard_selector_internal::ShardSelectorInternal;
use collection::operations::types::{CollectionResult, VectorsConfig};
use collection::operations::vector_params_builder::VectorParamsBuilder;
use collection::operations::CollectionUpdateOperations;
use collection::shards::channel_service::ChannelService;
use collection::shards::collection_shard_distribution::CollectionShardDistribution;
use collection::shards::replica_set::ReplicaState;
use common::counter::hardware_accumulator::HwMeasurementAcc;
use rand::Rng;
use segment::types::{Distance, ScoredPoint};
use tempfile::TempDir;

use crate::common::{
    dummy_abort_shard_transfer, dummy_on_replica_failure, dummy_request_shard_transfer,
    REST_PORT, TEST_OPTIMIZERS_CONFIG,
};

/// Configuration for creating a vector collection
#[derive(Clone)]
pub struct VectorConfig {
    pub size: u64,
    pub distance: Distance,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            size: 128,
            distance: Distance::Cosine,
        }
    }
}

/// Represents a node in the test cluster
#[allow(dead_code)]
pub struct TestNode {
    pub id: u64,
    pub collection: Collection,
    pub data_dir: TempDir,
    pub is_alive: bool,
}

/// Test cluster that manages multiple nodes for integration testing
pub struct TestCluster {
    nodes: HashMap<u32, TestNode>,
    node_count: u32,
    collection_name: String,
    vector_config: Option<VectorConfig>,
    replication_factor: Option<u32>,
    shard_number: u32,
}

impl TestCluster {
    /// Create a new test cluster with the specified number of nodes
    pub fn new(node_count: u32) -> Self {
        Self {
            nodes: HashMap::new(),
            node_count,
            collection_name: "test".to_string(),
            vector_config: None,
            replication_factor: None,
            shard_number: 1,
        }
    }

    /// Configure the collection with a specific vector configuration
    pub fn with_collection(mut self, name: &str, config: VectorConfig) -> Self {
        self.collection_name = name.to_string();
        self.vector_config = Some(config);
        self
    }

    /// Set the replication factor for the cluster
    pub fn with_replication(mut self, replication_factor: u32) -> Self {
        self.replication_factor = Some(replication_factor);
        self
    }

    /// Set the number of shards for the collection
    pub fn with_shards(mut self, shard_number: u32) -> Self {
        self.shard_number = shard_number;
        self
    }

    /// Initialize the cluster and create all nodes
    pub async fn build(mut self) -> CollectionResult<Self> {
        let vector_config = self.vector_config.clone().unwrap_or_default();

        for node_id in 0..self.node_count {
            let node = self.create_node(node_id, &vector_config).await?;
            self.nodes.insert(node_id, node);
        }

        Ok(self)
    }

    /// Create a single node in the cluster
    async fn create_node(
        &self,
        node_id: u32,
        vector_config: &VectorConfig,
    ) -> CollectionResult<TestNode> {
        let data_dir = tempfile::Builder::new()
            .prefix(&format!("test_node_{}_", node_id))
            .tempdir()
            .expect("Failed to create temp dir");

        let collection_path = data_dir.path().join("collection");
        let snapshots_path = data_dir.path().join("snapshots");

        std::fs::create_dir_all(&collection_path).expect("Failed to create collection dir");
        std::fs::create_dir_all(&snapshots_path).expect("Failed to create snapshots dir");

        let collection_config = self.create_collection_config(vector_config);
        let node_id_u64 = node_id as u64;

        let collection = Collection::new(
            self.collection_name.clone(),
            node_id_u64,
            &collection_path,
            &snapshots_path,
            &collection_config,
            Default::default(),
            CollectionShardDistribution::all_local(Some(self.shard_number), node_id_u64),
            None,
            ChannelService::new(REST_PORT + node_id as u16, None),
            dummy_on_replica_failure(),
            dummy_request_shard_transfer(),
            dummy_abort_shard_transfer(),
            None,
            None,
            Default::default(),
            None,
        )
        .await?;

        // Activate all local shards
        let local_shards = collection.get_local_shards().await;
        for shard_id in local_shards {
            collection
                .set_shard_replica_state(shard_id, node_id_u64, ReplicaState::Active, None)
                .await?;
        }

        Ok(TestNode {
            id: node_id_u64,
            collection,
            data_dir,
            is_alive: true,
        })
    }

    /// Create collection configuration
    fn create_collection_config(&self, vector_config: &VectorConfig) -> CollectionConfigInternal {
        let wal_config = WalConfig {
            wal_capacity_mb: 1,
            wal_segments_ahead: 0,
            wal_retain_closed: 1,
        };

        let collection_params = CollectionParams {
            vectors: VectorsConfig::Single(
                VectorParamsBuilder::new(vector_config.size, vector_config.distance).build(),
            ),
            shard_number: NonZeroU32::new(self.shard_number)
                .expect("Shard number cannot be zero"),
            replication_factor: NonZeroU32::new(self.replication_factor.unwrap_or(1))
                .expect("Replication factor cannot be zero"),
            ..CollectionParams::empty()
        };

        CollectionConfigInternal {
            params: collection_params,
            optimizer_config: TEST_OPTIMIZERS_CONFIG.clone(),
            wal_config,
            hnsw_config: Default::default(),
            quantization_config: Default::default(),
            strict_mode_config: Default::default(),
            uuid: None,
            metadata: None,
        }
    }

    /// Insert random points into the collection
    pub async fn insert_random_points(&self, count: usize) -> CollectionResult<()> {
        let default_config = VectorConfig::default();
        let vector_config = self.vector_config.as_ref().unwrap_or(&default_config);
        let dim = vector_config.size as usize;

        let mut rng = rand::rng();

        // Generate random vectors
        let vectors: Vec<Vec<f32>> = (0..count)
            .map(|_| (0..dim).map(|_| rng.random_range(-1.0..1.0)).collect())
            .collect();

        // Generate point IDs
        let ids: Vec<_> = (0..count as u64).map(|i| i.into()).collect();

        let batch = BatchPersisted {
            ids,
            vectors: BatchVectorStructPersisted::Single(vectors),
            payloads: None,
        };

        let insert_points = CollectionUpdateOperations::PointOperation(
            PointOperations::UpsertPoints(PointInsertOperationsInternal::from(batch)),
        );

        // Insert on the first alive node
        let node = self
            .get_first_alive_node()
            .expect("No alive nodes in cluster");

        let hw_counter = HwMeasurementAcc::new();
        node.collection
            .update_from_client_simple(insert_points, true, WriteOrdering::default(), hw_counter)
            .await?;

        Ok(())
    }

    /// Insert specific points into the collection
    pub async fn insert_points(&self, vectors: Vec<Vec<f32>>) -> CollectionResult<()> {
        let ids: Vec<_> = (0..vectors.len() as u64).map(|i| i.into()).collect();

        let batch = BatchPersisted {
            ids,
            vectors: BatchVectorStructPersisted::Single(vectors),
            payloads: None,
        };

        let insert_points = CollectionUpdateOperations::PointOperation(
            PointOperations::UpsertPoints(PointInsertOperationsInternal::from(batch)),
        );

        let node = self
            .get_first_alive_node()
            .expect("No alive nodes in cluster");

        let hw_counter = HwMeasurementAcc::new();
        node.collection
            .update_from_client_simple(insert_points, true, WriteOrdering::default(), hw_counter)
            .await?;

        Ok(())
    }

    /// Simulate killing a node
    pub fn kill_node(&mut self, node_id: u32) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.is_alive = false;
        }
    }

    /// Simulate restarting a killed node
    pub fn restart_node(&mut self, node_id: u32) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.is_alive = true;
        }
    }

    /// Search for similar vectors
    pub async fn search(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> CollectionResult<Vec<ScoredPoint>> {
        use api::rest::SearchRequestInternal;

        let search_request = SearchRequestInternal {
            vector: query_vector.to_vec().into(),
            with_payload: None,
            with_vector: None,
            filter: None,
            params: None,
            limit,
            offset: None,
            score_threshold: None,
        };

        let node = self
            .get_first_alive_node()
            .expect("No alive nodes in cluster");

        let hw_acc = HwMeasurementAcc::new();
        let results = node
            .collection
            .search(
                search_request.into(),
                None,
                &ShardSelectorInternal::All,
                None,
                hw_acc,
            )
            .await?;

        Ok(results)
    }

    /// Get the first alive node
    fn get_first_alive_node(&self) -> Option<&TestNode> {
        self.nodes.values().find(|node| node.is_alive)
    }

    /// Get a specific node by ID
    #[allow(dead_code)]
    pub fn get_node(&self, node_id: u32) -> Option<&TestNode> {
        self.nodes.get(&node_id)
    }

    /// Get a mutable reference to a specific node by ID
    #[allow(dead_code)]
    pub fn get_node_mut(&mut self, node_id: u32) -> Option<&mut TestNode> {
        self.nodes.get_mut(&node_id)
    }

    /// Get all alive nodes
    #[allow(dead_code)]
    pub fn alive_nodes(&self) -> impl Iterator<Item = &TestNode> {
        self.nodes.values().filter(|node| node.is_alive)
    }

    /// Get the number of alive nodes
    pub fn alive_node_count(&self) -> usize {
        self.nodes.values().filter(|node| node.is_alive).count()
    }

    /// Wait for all nodes to be synchronized (in a real cluster)
    /// For now, this is a no-op as we're using a single-process simulation
    pub async fn wait_for_sync(&self) {
        // In a real distributed cluster, this would wait for replication
        // For single-process testing, operations are already synchronous
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_creation() {
        let cluster = TestCluster::new(3)
            .with_collection("test", VectorConfig::default())
            .build()
            .await
            .unwrap();

        assert_eq!(cluster.alive_node_count(), 3);
    }

    #[tokio::test]
    async fn test_node_failure() {
        let mut cluster = TestCluster::new(3)
            .with_collection("test", VectorConfig::default())
            .build()
            .await
            .unwrap();

        cluster.kill_node(1);
        assert_eq!(cluster.alive_node_count(), 2);

        cluster.restart_node(1);
        assert_eq!(cluster.alive_node_count(), 3);
    }

    #[tokio::test]
    async fn test_insert_and_search() {
        let cluster = TestCluster::new(1)
            .with_collection(
                "test",
                VectorConfig {
                    size: 4,
                    distance: Distance::Cosine,
                },
            )
            .build()
            .await
            .unwrap();

        // Insert some test points
        let vectors = vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 1.0],
        ];

        cluster.insert_points(vectors).await.unwrap();

        // Search for similar vector
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let results = cluster.search(&query, 2).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 0.into());
    }
}
