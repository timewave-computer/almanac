use anyhow::Result;
use async_trait::async_trait;
use indexer_core::coordination::{NodeInfo, NodeCapabilities, NodeStatus};
use indexer_storage::fdb::{FdbConfig, FdbStorage};
use indexer_storage::fdb_coordination::FdbCoordinationService;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{info, warn};
use uuid::Uuid;

/// Distributed test cluster for testing multi-node scenarios
pub struct DistributedTestCluster {
    nodes: Arc<RwLock<HashMap<String, TestNode>>>,
    coordination: Arc<FdbCoordinationService>,
    fdb_storage: Arc<FdbStorage>,
}

/// A test node in the distributed cluster
pub struct TestNode {
    pub node_id: String,
    pub info: NodeInfo,
    pub handle: Option<JoinHandle<()>>,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl DistributedTestCluster {
    /// Create a new test cluster
    pub async fn new(namespace: &str) -> Result<Self> {
        let fdb_config = FdbConfig {
            cluster_file: None,
            namespace: namespace.to_string(),
        };
        
        let fdb_storage = Arc::new(FdbStorage::new(fdb_config).await?);
        let db = foundationdb::Database::default()?;
        let coordination = Arc::new(FdbCoordinationService::new(db, namespace.to_string()));
        
        Ok(Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            coordination,
            fdb_storage,
        })
    }
    
    /// Add a node to the cluster
    pub async fn add_node(&self, capabilities: NodeCapabilities) -> Result<String> {
        let node_id = format!("test-node-{}", Uuid::new_v4());
        
        let node_info = NodeInfo {
            node_id: node_id.clone(),
            address: format!("127.0.0.1:{}", 8000 + self.nodes.read().await.len()),
            capabilities,
            status: NodeStatus::Active,
            last_heartbeat: 0,
            metadata: HashMap::new(),
        };
        
        // Register node with coordination service
        self.coordination.register_node(node_info.clone()).await?;
        
        let test_node = TestNode {
            node_id: node_id.clone(),
            info: node_info,
            handle: None,
            shutdown_tx: None,
        };
        
        self.nodes.write().await.insert(node_id.clone(), test_node);
        
        info!("Added test node: {}", node_id);
        Ok(node_id)
    }
    
    /// Start a node with simulated processing
    pub async fn start_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.get_mut(node_id) {
            let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
            let node_id_clone = node_id.to_string();
            let coordination = Arc::clone(&self.coordination);
            
            let handle = tokio::spawn(async move {
                info!("Starting node: {}", node_id_clone);
                
                loop {
                    tokio::select! {
                        _ = &mut shutdown_rx => {
                            info!("Node {} shutting down", node_id_clone);
                            break;
                        }
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                            // Update heartbeat
                            if let Err(e) = coordination.update_node_status(&node_id_clone, NodeStatus::Active).await {
                                warn!("Failed to update heartbeat for {}: {}", node_id_clone, e);
                            }
                        }
                    }
                }
            });
            
            node.handle = Some(handle);
            node.shutdown_tx = Some(shutdown_tx);
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Node {} not found", node_id))
        }
    }
    
    /// Stop a node
    pub async fn stop_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.get_mut(node_id) {
            // Update status
            self.coordination.update_node_status(node_id, NodeStatus::Stopping).await?;
            
            // Send shutdown signal
            if let Some(tx) = node.shutdown_tx.take() {
                let _ = tx.send(());
            }
            
            // Wait for handle to complete
            if let Some(handle) = node.handle.take() {
                handle.await?;
            }
            
            // Mark as offline
            self.coordination.update_node_status(node_id, NodeStatus::Offline).await?;
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Node {} not found", node_id))
        }
    }
    
    /// Simulate a node failure
    pub async fn fail_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.get_mut(node_id) {
            // Abruptly stop the node without cleanup
            if let Some(handle) = node.handle.take() {
                handle.abort();
            }
            
            // Don't update status - simulating sudden failure
            info!("Node {} failed", node_id);
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Node {} not found", node_id))
        }
    }
    
    /// Get active nodes in the cluster
    pub async fn get_active_nodes(&self) -> Result<Vec<NodeInfo>> {
        self.coordination.get_active_nodes().await
    }
    
    /// Shutdown the entire cluster
    pub async fn shutdown(&self) -> Result<()> {
        let node_ids: Vec<String> = self.nodes.read().await.keys().cloned().collect();
        
        for node_id in node_ids {
            let _ = self.stop_node(&node_id).await;
        }
        
        // Clear FDB data
        self.fdb_storage.clear().await?;
        
        Ok(())
    }
}

/// Test scenarios for distributed systems
pub struct DistributedTestScenarios;

impl DistributedTestScenarios {
    /// Test leader election under various conditions
    pub async fn test_leader_election(cluster: &DistributedTestCluster) -> Result<()> {
        info!("Testing leader election...");
        
        // Add multiple nodes
        let node1 = cluster.add_node(NodeCapabilities::default()).await?;
        let node2 = cluster.add_node(NodeCapabilities::default()).await?;
        let node3 = cluster.add_node(NodeCapabilities::default()).await?;
        
        // Start nodes
        cluster.start_node(&node1).await?;
        cluster.start_node(&node2).await?;
        cluster.start_node(&node3).await?;
        
        // Attempt leader election from each node
        let elected1 = cluster.coordination.elect_leader(&node1).await?;
        let elected2 = cluster.coordination.elect_leader(&node2).await?;
        let elected3 = cluster.coordination.elect_leader(&node3).await?;
        
        // Only one should succeed
        assert!(elected1 || elected2 || elected3);
        assert!(!(elected1 && elected2) && !(elected2 && elected3) && !(elected1 && elected3));
        
        // Get current leader
        let leader = cluster.coordination.get_leader().await?;
        assert!(leader.is_some());
        
        info!("Leader election test passed");
        Ok(())
    }
    
    /// Test work distribution and rebalancing
    pub async fn test_work_distribution(cluster: &DistributedTestCluster) -> Result<()> {
        use indexer_core::coordination::{WorkAssignment, WorkType, WorkDistributor};
        
        info!("Testing work distribution...");
        
        // Add nodes with different capabilities
        let mut caps1 = NodeCapabilities::default();
        caps1.supported_chains = vec!["ethereum".to_string()];
        let node1 = cluster.add_node(caps1).await?;
        
        let mut caps2 = NodeCapabilities::default();
        caps2.supported_chains = vec!["ethereum".to_string(), "polygon".to_string()];
        let node2 = cluster.add_node(caps2).await?;
        
        cluster.start_node(&node1).await?;
        cluster.start_node(&node2).await?;
        
        // Create work items
        let work_items = vec![
            WorkType::BlockIndexing { from_block: 100, to_block: Some(200) },
            WorkType::BlockIndexing { from_block: 200, to_block: Some(300) },
            WorkType::EventProcessing { contract_address: "0x123".to_string() },
        ];
        
        // Distribute work
        let distributor = WorkDistributor::new(
            Box::new(cluster.coordination.clone()),
            Default::default(),
        );
        
        let assignments = distributor.distribute_work(work_items).await?;
        assert_eq!(assignments.len(), 3);
        
        // Verify assignments
        for assignment in &assignments {
            let node_assignments = cluster.coordination
                .get_node_assignments(&assignment.node_id)
                .await?;
            assert!(node_assignments.iter().any(|a| a.assignment_id == assignment.assignment_id));
        }
        
        info!("Work distribution test passed");
        Ok(())
    }
    
    /// Test node failure and recovery
    pub async fn test_node_failure_recovery(cluster: &DistributedTestCluster) -> Result<()> {
        use indexer_core::coordination::{HealthMonitor, WorkType, WorkAssignment};
        
        info!("Testing node failure and recovery...");
        
        // Add nodes
        let node1 = cluster.add_node(NodeCapabilities::default()).await?;
        let node2 = cluster.add_node(NodeCapabilities::default()).await?;
        
        cluster.start_node(&node1).await?;
        cluster.start_node(&node2).await?;
        
        // Assign work to node1
        let assignment = WorkAssignment {
            assignment_id: Uuid::new_v4().to_string(),
            node_id: node1.clone(),
            chain_id: "ethereum".to_string(),
            work_type: WorkType::BlockIndexing { from_block: 100, to_block: Some(200) },
            parameters: HashMap::new(),
            assigned_at: 0,
        };
        
        cluster.coordination.assign_work(assignment.clone()).await?;
        
        // Fail node1
        cluster.fail_node(&node1).await?;
        
        // Run health monitor
        let monitor = HealthMonitor::new(
            Box::new(cluster.coordination.clone()),
            Default::default(),
        );
        
        // Wait for timeout
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        monitor.check_health().await?;
        
        // Verify node1 is marked offline
        let nodes = cluster.get_active_nodes().await?;
        assert!(!nodes.iter().any(|n| n.node_id == node1));
        
        // Verify work was released
        let node1_assignments = cluster.coordination.get_node_assignments(&node1).await?;
        assert_eq!(node1_assignments.len(), 0);
        
        info!("Node failure recovery test passed");
        Ok(())
    }
}

/// Integration test helpers
pub mod helpers {
    use super::*;
    use indexer_core::{Event, EventType, UnifiedEvent};
    
    /// Generate test events
    pub fn generate_test_events(count: usize, chain_id: &str) -> Vec<Box<dyn Event>> {
        (0..count)
            .map(|i| {
                let event = UnifiedEvent {
                    id: Uuid::new_v4().to_string(),
                    chain_id: chain_id.to_string(),
                    block_number: (i + 100) as u64,
                    block_hash: format!("0x{:064x}", i),
                    transaction_hash: Some(format!("0x{:064x}", i * 2)),
                    transaction_index: Some(i as u32),
                    log_index: Some(i as u32),
                    event_type: EventType::Generic,
                    contract_address: Some("0x1234567890123456789012345678901234567890".to_string()),
                    timestamp: Some(chrono::Utc::now()),
                    data: serde_json::json!({
                        "value": i,
                        "message": format!("Test event {}", i)
                    }),
                };
                Box::new(event) as Box<dyn Event>
            })
            .collect()
    }
    
    /// Verify event correlation across nodes
    pub async fn verify_cross_node_correlation(
        cluster: &DistributedTestCluster,
        source_chain: &str,
        target_chain: &str,
    ) -> Result<()> {
        use indexer_core::distributed_correlation::{
            DistributedEventCorrelator, EventCorrelation, CorrelationType,
        };
        
        let correlator = DistributedEventCorrelator::new("test-correlator".to_string());
        
        // Generate correlated events
        let source_events = generate_test_events(5, source_chain);
        let target_events = generate_test_events(5, target_chain);
        
        // Store events
        for event in &source_events {
            cluster.fdb_storage.store_event(event.clone()).await?;
        }
        
        for event in &target_events {
            cluster.fdb_storage.store_event(event.clone()).await?;
        }
        
        // Correlate events
        for (i, source_event) in source_events.iter().enumerate() {
            if let Some(target_event) = target_events.get(i) {
                let correlations = correlator
                    .correlate_cross_chain(source_event.as_ref(), vec![target_chain.to_string()])
                    .await?;
                
                // Verify correlation exists
                assert!(!correlations.is_empty(), "No correlations found for event {}", i);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_distributed_cluster_lifecycle() {
        let cluster = match DistributedTestCluster::new("test-cluster").await {
            Ok(c) => c,
            Err(_) => {
                eprintln!("FoundationDB not available, skipping test");
                return;
            }
        };
        
        // Add and start nodes
        let node1 = cluster.add_node(NodeCapabilities::default()).await.unwrap();
        cluster.start_node(&node1).await.unwrap();
        
        // Verify node is active
        let active_nodes = cluster.get_active_nodes().await.unwrap();
        assert_eq!(active_nodes.len(), 1);
        
        // Stop node
        cluster.stop_node(&node1).await.unwrap();
        
        // Cleanup
        cluster.shutdown().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_leader_election_scenario() {
        let cluster = match DistributedTestCluster::new("test-leader").await {
            Ok(c) => c,
            Err(_) => {
                eprintln!("FoundationDB not available, skipping test");
                return;
            }
        };
        
        DistributedTestScenarios::test_leader_election(&cluster).await.unwrap();
        
        cluster.shutdown().await.unwrap();
    }
}