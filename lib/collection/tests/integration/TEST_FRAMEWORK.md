# Integration Test Framework

This document describes the integration test framework for testing distributed Qdrant scenarios.

## Overview

The test framework provides a reusable `TestCluster` abstraction that simplifies creating and managing multi-node clusters in integration tests. This reduces boilerplate code and makes it easier to test distributed scenarios such as replication, node failures, and cluster operations.

## Features

- **Multi-node cluster simulation**: Create clusters with multiple nodes
- **Vector collection configuration**: Configure collections with custom vector sizes and distance metrics
- **Replication support**: Set replication factors for testing distributed data
- **Node failure simulation**: Kill and restart nodes to test resilience
- **Data insertion helpers**: Insert random or specific test data
- **Search functionality**: Query the cluster and verify results

## Quick Start

### Basic Example

```rust
use crate::test_cluster::{TestCluster, VectorConfig};
use segment::types::Distance;

#[tokio::test]
async fn test_simple_search() {
    // Create a single-node cluster
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

    // Insert test data
    let vectors = vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0],
    ];
    cluster.insert_points(vectors).await.unwrap();

    // Search
    let query = vec![1.0, 0.0, 0.0, 0.0];
    let results = cluster.search(&query, 2).await.unwrap();

    assert_eq!(results.len(), 2);
}
```

### Distributed Scenario with Node Failure

```rust
#[tokio::test]
async fn test_distributed_search_with_failure() {
    // Create a 3-node cluster with replication
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

    // Insert random test data
    cluster.insert_random_points(1000).await.unwrap();

    // Wait for replication
    cluster.wait_for_sync().await;

    // Simulate node failure
    cluster.kill_node(1);
    assert_eq!(cluster.alive_node_count(), 2);

    // Verify search still works
    let query_vector: Vec<f32> = (0..128).map(|i| (i as f32) / 128.0).collect();
    let results = cluster.search(&query_vector, 10).await.unwrap();
    assert_eq!(results.len(), 10);

    // Restart the node
    cluster.restart_node(1);
    assert_eq!(cluster.alive_node_count(), 3);
}
```

## API Reference

### TestCluster

The main struct for managing test clusters.

#### Constructor

```rust
TestCluster::new(node_count: u32) -> Self
```

Creates a new test cluster builder with the specified number of nodes.

#### Builder Methods

- `with_collection(name: &str, config: VectorConfig) -> Self`
  - Configure the collection with a specific name and vector configuration

- `with_replication(replication_factor: u32) -> Self`
  - Set the replication factor for the cluster

- `with_shards(shard_number: u32) -> Self`
  - Set the number of shards for the collection

- `build() -> CollectionResult<Self>`
  - Initialize the cluster and create all nodes

#### Data Operations

- `insert_random_points(count: usize) -> CollectionResult<()>`
  - Insert the specified number of randomly generated points

- `insert_points(vectors: Vec<Vec<f32>>) -> CollectionResult<()>`
  - Insert specific vectors into the collection

#### Node Operations

- `kill_node(node_id: u32)`
  - Simulate killing a node (marks it as unavailable)

- `restart_node(node_id: u32)`
  - Simulate restarting a killed node

- `alive_node_count() -> usize`
  - Get the number of currently alive nodes

#### Query Operations

- `search(query_vector: &[f32], limit: usize) -> CollectionResult<Vec<ScoredPoint>>`
  - Search for similar vectors

- `wait_for_sync()`
  - Wait for nodes to synchronize (useful after data insertion)

#### Node Access

- `get_node(node_id: u32) -> Option<&TestNode>`
  - Get a reference to a specific node

- `get_node_mut(node_id: u32) -> Option<&mut TestNode>`
  - Get a mutable reference to a specific node

- `alive_nodes() -> impl Iterator<Item = &TestNode>`
  - Get an iterator over all alive nodes

### VectorConfig

Configuration for vector collections.

```rust
pub struct VectorConfig {
    pub size: u64,          // Vector dimension
    pub distance: Distance, // Distance metric
}
```

Default configuration:
- size: 128
- distance: Distance::Cosine

## Implementation Notes

### Current Limitations

1. **Single-process simulation**: The current implementation simulates multiple nodes within a single process. While this is sufficient for testing basic distributed logic, it doesn't test true inter-process communication or network failures.

2. **Node failure simulation**: The `kill_node` method marks a node as unavailable but doesn't actually terminate any processes or close connections. This is by design for the single-process simulation.

3. **Synchronization**: The `wait_for_sync()` method is currently a simple sleep. In a true distributed setup, this would wait for actual replication to complete.

### Future Enhancements

Potential improvements for the test framework:

1. **True multi-process testing**: Support for spawning actual separate processes to test real distributed scenarios
2. **Network failure simulation**: Simulate network partitions and latency
3. **More advanced failure scenarios**: Simulate partial failures, slow nodes, etc.
4. **Performance testing utilities**: Built-in support for measuring throughput and latency
5. **Snapshot and recovery testing**: Utilities for testing backup and restore scenarios

## Best Practices

1. **Use appropriate cluster sizes**: For unit tests, prefer small clusters (1-3 nodes) to keep tests fast
2. **Clean up resources**: The framework automatically cleans up temporary directories when the cluster is dropped
3. **Use random data for scale tests**: Use `insert_random_points()` for testing with large datasets
4. **Use specific data for correctness tests**: Use `insert_points()` when you need predictable test data
5. **Wait for sync after insertion**: Call `wait_for_sync()` after inserting data before querying

## Examples

See `distributed_search_test.rs` for comprehensive examples of using the test framework.

## Related Files

- `lib/collection/tests/integration/test_cluster.rs` - Framework implementation
- `lib/collection/tests/integration/distributed_search_test.rs` - Example tests
- `lib/collection/tests/integration/common/mod.rs` - Common test utilities

## Contributing

When adding new functionality to the test framework:

1. Keep the API simple and intuitive
2. Add inline documentation for all public methods
3. Add example usage in the tests
4. Update this documentation
5. Consider backward compatibility when making changes
