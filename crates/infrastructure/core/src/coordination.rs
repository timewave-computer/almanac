use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Node information in the distributed cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub address: String,
    pub capabilities: NodeCapabilities,
    pub status: NodeStatus,
    pub last_heartbeat: u64,
    pub metadata: HashMap<String, String>,
}

/// Node capabilities for work assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub supported_chains: Vec<String>,
    pub max_throughput: u64,
    pub storage_backends: Vec<String>,
    pub api_endpoints: Vec<String>,
}

/// Current status of a node
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Starting,
    Active,
    Degraded,
    Stopping,
    Offline,
}

/// Work assignment for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkAssignment {
    pub assignment_id: String,
    pub node_id: String,
    pub chain_id: String,
    pub work_type: WorkType,
    pub parameters: HashMap<String, String>,
    pub assigned_at: u64,
}

/// Type of work to be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkType {
    BlockIndexing { from_block: u64, to_block: Option<u64> },
    EventProcessing { contract_address: String },
    ProofGeneration { block_range: (u64, u64) },
    CrossChainCorrelation { source_chain: String, target_chain: String },
}

/// Leader election result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderInfo {
    pub node_id: String,
    pub term: u64,
    pub elected_at: u64,
    pub lease_duration: u64,
}

/// Coordination service for distributed nodes
#[async_trait]
pub trait CoordinationService: Send + Sync {
    /// Register a node in the cluster
    async fn register_node(&self, info: NodeInfo) -> Result<()>;
    
    /// Update node status and heartbeat
    async fn update_node_status(&self, node_id: &str, status: NodeStatus) -> Result<()>;
    
    /// Get all active nodes
    async fn get_active_nodes(&self) -> Result<Vec<NodeInfo>>;
    
    /// Assign work to a node
    async fn assign_work(&self, assignment: WorkAssignment) -> Result<()>;
    
    /// Get work assignments for a node
    async fn get_node_assignments(&self, node_id: &str) -> Result<Vec<WorkAssignment>>;
    
    /// Complete a work assignment
    async fn complete_assignment(&self, assignment_id: &str) -> Result<()>;
    
    /// Elect a leader for coordination tasks
    async fn elect_leader(&self, candidate_id: &str) -> Result<bool>;
    
    /// Get current leader information
    async fn get_leader(&self) -> Result<Option<LeaderInfo>>;
    
    /// Release work assignment (for failure handling)
    async fn release_assignment(&self, assignment_id: &str) -> Result<()>;
}

/// Configuration for the coordination service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationConfig {
    pub heartbeat_interval: Duration,
    pub node_timeout: Duration,
    pub leader_lease_duration: Duration,
    pub work_steal_threshold: Duration,
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(30),
            node_timeout: Duration::from_secs(120),
            leader_lease_duration: Duration::from_secs(300),
            work_steal_threshold: Duration::from_secs(600),
        }
    }
}

/// Work distribution strategy
pub struct WorkDistributor {
    coordination: Box<dyn CoordinationService>,
    config: CoordinationConfig,
}

impl WorkDistributor {
    pub fn new(coordination: Box<dyn CoordinationService>, config: CoordinationConfig) -> Self {
        Self { coordination, config }
    }
    
    /// Distribute work across available nodes
    pub async fn distribute_work(&self, work_items: Vec<WorkType>) -> Result<Vec<WorkAssignment>> {
        let nodes = self.coordination.get_active_nodes().await?;
        let mut assignments = Vec::new();
        
        // Simple round-robin distribution
        for (i, work) in work_items.into_iter().enumerate() {
            let node = &nodes[i % nodes.len()];
            
            let assignment = WorkAssignment {
                assignment_id: Uuid::new_v4().to_string(),
                node_id: node.node_id.clone(),
                chain_id: self.extract_chain_id(&work),
                work_type: work,
                parameters: HashMap::new(),
                assigned_at: current_timestamp(),
            };
            
            self.coordination.assign_work(assignment.clone()).await?;
            assignments.push(assignment);
        }
        
        Ok(assignments)
    }
    
    /// Rebalance work if nodes are overloaded or have failed
    pub async fn rebalance_work(&self) -> Result<()> {
        let nodes = self.coordination.get_active_nodes().await?;
        let now = current_timestamp();
        
        for node in nodes {
            let assignments = self.coordination.get_node_assignments(&node.node_id).await?;
            
            // Check for stale assignments
            for assignment in assignments {
                let age = Duration::from_secs(now - assignment.assigned_at);
                if age > self.config.work_steal_threshold {
                    // Release and reassign
                    self.coordination.release_assignment(&assignment.assignment_id).await?;
                    
                    // Find a healthy node to reassign to
                    if let Some(target_node) = self.find_least_loaded_node().await? {
                        let new_assignment = WorkAssignment {
                            assignment_id: Uuid::new_v4().to_string(),
                            node_id: target_node.node_id,
                            ..assignment
                        };
                        self.coordination.assign_work(new_assignment).await?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn find_least_loaded_node(&self) -> Result<Option<NodeInfo>> {
        let nodes = self.coordination.get_active_nodes().await?;
        let mut min_load = usize::MAX;
        let mut best_node = None;
        
        for node in nodes {
            let assignments = self.coordination.get_node_assignments(&node.node_id).await?;
            if assignments.len() < min_load {
                min_load = assignments.len();
                best_node = Some(node);
            }
        }
        
        Ok(best_node)
    }
    
    fn extract_chain_id(&self, work: &WorkType) -> String {
        match work {
            WorkType::BlockIndexing { .. } => "ethereum".to_string(), // Would be determined from context
            WorkType::EventProcessing { .. } => "ethereum".to_string(),
            WorkType::ProofGeneration { .. } => "ethereum".to_string(),
            WorkType::CrossChainCorrelation { source_chain, .. } => source_chain.clone(),
        }
    }
}

/// Health monitor for distributed nodes
pub struct HealthMonitor {
    coordination: Box<dyn CoordinationService>,
    config: CoordinationConfig,
}

impl HealthMonitor {
    pub fn new(coordination: Box<dyn CoordinationService>, config: CoordinationConfig) -> Self {
        Self { coordination, config }
    }
    
    /// Check node health and mark unhealthy nodes
    pub async fn check_health(&self) -> Result<()> {
        let nodes = self.coordination.get_active_nodes().await?;
        let now = current_timestamp();
        
        for node in nodes {
            let time_since_heartbeat = Duration::from_secs(now - node.last_heartbeat);
            
            if time_since_heartbeat > self.config.node_timeout {
                // Mark node as offline
                self.coordination.update_node_status(&node.node_id, NodeStatus::Offline).await?;
                
                // Release all assignments for this node
                let assignments = self.coordination.get_node_assignments(&node.node_id).await?;
                for assignment in assignments {
                    self.coordination.release_assignment(&assignment.assignment_id).await?;
                }
            } else if time_since_heartbeat > self.config.heartbeat_interval * 2 {
                // Mark as degraded if missing heartbeats
                self.coordination.update_node_status(&node.node_id, NodeStatus::Degraded).await?;
            }
        }
        
        Ok(())
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}