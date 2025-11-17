//! Integration test framework for clustered testing
//!
//! Provides utilities for creating and managing test clusters

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

pub struct TestCluster {
    nodes: Vec<TestNode>,
    temp_dirs: Vec<TempDir>,
}

pub struct TestNode {
    pub id: u64,
    pub http_port: u16,
    pub grpc_port: u16,
    pub p2p_port: u16,
    pub process: Option<Child>,
    pub data_dir: PathBuf,
}

impl TestCluster {
    /// Create a new test cluster with N nodes
    pub async fn new(node_count: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let mut nodes = Vec::new();
        let mut temp_dirs = Vec::new();

        let base_http_port = 16333;
        let base_grpc_port = 16334;
        let base_p2p_port = 16335;

        for i in 0..node_count {
            let temp_dir = TempDir::new()?;
            let data_dir = temp_dir.path().to_path_buf();

            let node = TestNode {
                id: i as u64,
                http_port: base_http_port + (i * 3) as u16,
                grpc_port: base_grpc_port + (i * 3) as u16,
                p2p_port: base_p2p_port + (i * 3) as u16,
                process: None,
                data_dir: data_dir.clone(),
            };

            nodes.push(node);
            temp_dirs.push(temp_dir);
        }

        Ok(Self { nodes, temp_dirs })
    }

    /// Start all nodes in the cluster
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for i in 0..self.nodes.len() {
            self.start_node(i).await?;
        }

        // Wait for cluster to form
        sleep(Duration::from_secs(5)).await;

        Ok(())
    }

    /// Start a specific node
    async fn start_node(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let node = &mut self.nodes[index];

        // In a real implementation, this would start the Qdrant binary
        // For now, we just simulate it

        println!(
            "Starting node {} on http://localhost:{}",
            node.id, node.http_port
        );

        Ok(())
    }

    /// Stop all nodes
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for node in &mut self.nodes {
            if let Some(mut process) = node.process.take() {
                process.kill()?;
            }
        }

        Ok(())
    }

    /// Get HTTP client for a specific node
    pub fn client(&self, node_index: usize) -> reqwest::Client {
        reqwest::Client::new()
    }

    /// Get base URL for a node
    pub fn node_url(&self, node_index: usize) -> String {
        format!("http://localhost:{}", self.nodes[node_index].http_port)
    }

    /// Wait for node to be ready
    pub async fn wait_for_node(&self, node_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/health", self.node_url(node_index));
        let client = self.client(node_index);

        for _ in 0..30 {
            if let Ok(response) = client.get(&url).send().await {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            sleep(Duration::from_millis(100)).await;
        }

        Err("Node did not become ready".into())
    }
}

impl Drop for TestCluster {
    fn drop(&mut self) {
        let _ = futures::executor::block_on(self.stop());
    }
}

/// Helper macro for running clustered tests
#[macro_export]
macro_rules! cluster_test {
    ($nodes:expr, $body:expr) => {{
        let mut cluster = TestCluster::new($nodes).await.unwrap();
        cluster.start().await.unwrap();

        let result = $body(&cluster).await;

        cluster.stop().await.unwrap();

        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_cluster() {
        let cluster = TestCluster::new(3).await.unwrap();
        assert_eq!(cluster.nodes.len(), 3);
    }

    #[tokio::test]
    async fn test_node_urls() {
        let cluster = TestCluster::new(2).await.unwrap();
        assert!(cluster.node_url(0).contains("16333"));
        assert!(cluster.node_url(1).contains("16336"));
    }
}
