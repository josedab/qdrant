//! Distributed search test using the TestCluster framework
//!
//! This test demonstrates the usage of the new TestCluster framework
//! for testing distributed scenarios with node failures.

use segment::types::Distance;

use crate::test_cluster::{TestCluster, VectorConfig};

#[tokio::test]
async fn test_distributed_search() {
    // Create a 3-node cluster with replication factor of 2
    let cluster = TestCluster::new(3)
        .with_collection(
            "test",
            VectorConfig {
                size: 128,
                distance: Distance::Cosine,
            },
        )
        .with_replication(2)
        .with_shards(3)
        .build()
        .await
        .unwrap();

    // Insert 1000 random points
    cluster.insert_random_points(1000).await.unwrap();

    // Wait for replication to complete
    cluster.wait_for_sync().await;

    // Create a query vector
    let query_vector: Vec<f32> = (0..128).map(|i| (i as f32) / 128.0).collect();

    // Verify search works with all nodes alive
    let results = cluster.search(&query_vector, 10).await.unwrap();
    assert_eq!(results.len(), 10);

    // Note: In the current implementation, node failure simulation
    // marks nodes as unavailable but doesn't test true distributed replication.
    // This is because we're using a single-process simulation.
    // For true distributed testing, you would need multiple processes
    // or integration with the actual cluster management code.
}

#[tokio::test]
async fn test_search_with_node_failure() {
    let mut cluster = TestCluster::new(3)
        .with_collection(
            "test",
            VectorConfig {
                size: 128,
                distance: Distance::Cosine,
            },
        )
        .with_replication(2)
        .with_shards(3)
        .build()
        .await
        .unwrap();

    // Insert test data
    cluster.insert_random_points(1000).await.unwrap();

    // Wait for sync
    cluster.wait_for_sync().await;

    // Create a query vector
    let query_vector: Vec<f32> = (0..128).map(|i| (i as f32) / 128.0).collect();

    // Search before node failure
    let results_before = cluster.search(&query_vector, 10).await.unwrap();
    assert_eq!(results_before.len(), 10);

    // Simulate node failure
    cluster.kill_node(1);
    assert_eq!(cluster.alive_node_count(), 2);

    // Verify search still works with remaining nodes
    // Note: This works because we still have node 0 and 2 alive
    let results_after = cluster.search(&query_vector, 10).await.unwrap();
    assert_eq!(results_after.len(), 10);

    // Restart the node
    cluster.restart_node(1);
    assert_eq!(cluster.alive_node_count(), 3);
}

#[tokio::test]
async fn test_simple_search() {
    let cluster = TestCluster::new(1)
        .with_collection(
            "test",
            VectorConfig {
                size: 4,
                distance: Distance::Dot,
            },
        )
        .build()
        .await
        .unwrap();

    // Insert known test vectors
    let vectors = vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0],
        vec![0.0, 0.0, 0.0, 1.0],
        vec![1.0, 1.0, 0.0, 0.0],
    ];

    cluster.insert_points(vectors).await.unwrap();

    // Search for vector closest to [1.0, 0.0, 0.0, 0.0]
    let query = vec![1.0, 0.0, 0.0, 0.0];
    let results = cluster.search(&query, 3).await.unwrap();

    assert_eq!(results.len(), 3);
    // First result should be the exact match
    assert_eq!(results[0].id, 0.into());
}

#[tokio::test]
async fn test_multiple_shards() {
    let cluster = TestCluster::new(1)
        .with_collection(
            "test",
            VectorConfig {
                size: 8,
                distance: Distance::Cosine,
            },
        )
        .with_shards(4)
        .build()
        .await
        .unwrap();

    // Insert data
    cluster.insert_random_points(500).await.unwrap();

    // Query
    let query_vector: Vec<f32> = (0..8).map(|i| (i as f32) / 8.0).collect();
    let results = cluster.search(&query_vector, 10).await.unwrap();

    assert_eq!(results.len(), 10);
}

#[tokio::test]
async fn test_cluster_initialization() {
    // Test with different configurations
    let cluster1 = TestCluster::new(1)
        .with_collection("test1", VectorConfig::default())
        .build()
        .await
        .unwrap();

    assert_eq!(cluster1.alive_node_count(), 1);

    let cluster2 = TestCluster::new(5)
        .with_collection(
            "test2",
            VectorConfig {
                size: 256,
                distance: Distance::Euclid,
            },
        )
        .with_replication(3)
        .with_shards(10)
        .build()
        .await
        .unwrap();

    assert_eq!(cluster2.alive_node_count(), 5);
}
