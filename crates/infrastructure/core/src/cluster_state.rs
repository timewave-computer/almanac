// Cluster state inspection and monitoring for distributed systems
//
// This module provides comprehensive visibility into the distributed
// system state, including node status, work distribution, and health metrics

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, debug, warn};

/// Comprehensive cluster state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterState {
    /// Timestamp of the state snapshot
    pub timestamp: SystemTime,
    /// Cluster metadata
    pub metadata: ClusterMetadata,
    /// Node states indexed by node ID
    pub nodes: HashMap<String, NodeState>,
    /// Work distribution across nodes
    pub work_distribution: WorkDistribution,
    /// Cluster-wide metrics
    pub metrics: ClusterMetrics,
    /// Active connections between nodes
    pub connections: Vec<NodeConnection>,
    /// Cluster health summary
    pub health: ClusterHealth,
}

/// Cluster metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetadata {
    pub cluster_id: String,
    pub version: String,
    pub environment: String,
    pub start_time: SystemTime,
    pub leader_node: Option<String>,
}

/// Individual node state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    pub node_id: String,
    pub status: NodeStatus,
    pub capabilities: NodeCapabilities,
    pub resources: NodeResources,
    pub workload: NodeWorkload,
    pub last_heartbeat: SystemTime,
    pub uptime: Duration,
    pub errors: Vec<NodeError>,
}

/// Node status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Active,
    Idle,
    Busy,
    Stopping,
    Offline,
    Failed,
}

/// Node capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub supported_chains: Vec<String>,
    pub max_concurrent_tasks: usize,
    pub features: Vec<String>,
    pub version: String,
}

/// Node resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResources {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub memory_total_mb: u64,
    pub disk_usage_gb: f64,
    pub disk_total_gb: f64,
    pub network_bandwidth_mbps: f64,
    pub open_connections: u32,
}

/// Node workload information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeWorkload {
    pub active_tasks: Vec<TaskInfo>,
    pub queued_tasks: u32,
    pub completed_tasks_24h: u64,
    pub failed_tasks_24h: u64,
    pub average_task_duration: Duration,
    pub current_throughput: f64,
}

/// Task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub task_id: String,
    pub task_type: String,
    pub chain_id: String,
    pub start_time: SystemTime,
    pub progress_percent: f64,
    pub estimated_completion: Option<SystemTime>,
}

/// Node error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeError {
    pub timestamp: SystemTime,
    pub error_type: String,
    pub message: String,
    pub recoverable: bool,
}

/// Work distribution across the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkDistribution {
    pub total_tasks: u64,
    pub active_tasks: u64,
    pub distribution_strategy: String,
    pub partition_map: HashMap<String, PartitionInfo>,
    pub load_balance_score: f64,
}

/// Partition information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub partition_id: String,
    pub assigned_node: String,
    pub chain_id: String,
    pub block_range: Option<(u64, u64)>,
    pub event_count: u64,
    pub last_updated: SystemTime,
}

/// Cluster-wide metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    pub total_events_processed: u64,
    pub events_per_second: f64,
    pub total_blocks_indexed: u64,
    pub blocks_per_second: f64,
    pub average_latency_ms: f64,
    pub error_rate_percent: f64,
    pub storage_size_gb: f64,
    pub api_requests_per_second: f64,
}

/// Node connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnection {
    pub from_node: String,
    pub to_node: String,
    pub connection_type: ConnectionType,
    pub status: ConnectionStatus,
    pub latency_ms: f64,
    pub bandwidth_usage_mbps: f64,
    pub established_at: SystemTime,
}

/// Connection types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Gossip,
    DataReplication,
    Coordination,
    ClientRequest,
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Established,
    Connecting,
    Disconnected,
    Failed,
}

/// Cluster health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealth {
    pub overall_status: HealthStatus,
    pub healthy_nodes: u32,
    pub unhealthy_nodes: u32,
    pub warnings: Vec<HealthWarning>,
    pub critical_issues: Vec<CriticalIssue>,
    pub recommendations: Vec<String>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
    Unknown,
}

/// Health warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthWarning {
    pub severity: WarningSeverity,
    pub component: String,
    pub message: String,
    pub affected_nodes: Vec<String>,
}

/// Warning severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningSeverity {
    Low,
    Medium,
    High,
}

/// Critical issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalIssue {
    pub issue_type: String,
    pub description: String,
    pub impact: String,
    pub resolution: String,
    pub affected_nodes: Vec<String>,
}

/// Cluster state inspector
pub struct ClusterStateInspector {
    node_id: String,
    cluster_state: Arc<RwLock<ClusterState>>,
    update_interval: Duration,
}

impl ClusterStateInspector {
    /// Create a new cluster state inspector
    pub fn new(node_id: String, cluster_id: String) -> Self {
        let initial_state = ClusterState {
            timestamp: SystemTime::now(),
            metadata: ClusterMetadata {
                cluster_id,
                version: env!("CARGO_PKG_VERSION").to_string(),
                environment: "production".to_string(),
                start_time: SystemTime::now(),
                leader_node: None,
            },
            nodes: HashMap::new(),
            work_distribution: WorkDistribution {
                total_tasks: 0,
                active_tasks: 0,
                distribution_strategy: "RoundRobin".to_string(),
                partition_map: HashMap::new(),
                load_balance_score: 1.0,
            },
            metrics: ClusterMetrics {
                total_events_processed: 0,
                events_per_second: 0.0,
                total_blocks_indexed: 0,
                blocks_per_second: 0.0,
                average_latency_ms: 0.0,
                error_rate_percent: 0.0,
                storage_size_gb: 0.0,
                api_requests_per_second: 0.0,
            },
            connections: Vec::new(),
            health: ClusterHealth {
                overall_status: HealthStatus::Unknown,
                healthy_nodes: 0,
                unhealthy_nodes: 0,
                warnings: Vec::new(),
                critical_issues: Vec::new(),
                recommendations: Vec::new(),
            },
        };

        Self {
            node_id,
            cluster_state: Arc::new(RwLock::new(initial_state)),
            update_interval: Duration::from_secs(30),
        }
    }

    /// Start periodic state updates
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting cluster state inspector for node: {}", self.node_id);

        let cluster_state = Arc::clone(&self.cluster_state);
        let update_interval = self.update_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_interval);
            
            loop {
                interval.tick().await;
                
                // Update cluster state
                if let Err(e) = Self::update_cluster_state(&cluster_state).await {
                    warn!("Failed to update cluster state: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Update cluster state
    async fn update_cluster_state(
        cluster_state: &Arc<RwLock<ClusterState>>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut state = cluster_state.write().await;
        
        // Update timestamp
        state.timestamp = SystemTime::now();
        
        // Calculate health status
        state.health = Self::calculate_cluster_health(&state);
        
        // Update metrics
        state.metrics = Self::calculate_cluster_metrics(&state);
        
        debug!("Cluster state updated");
        Ok(())
    }

    /// Calculate cluster health
    fn calculate_cluster_health(state: &ClusterState) -> ClusterHealth {
        let mut health = ClusterHealth {
            overall_status: HealthStatus::Healthy,
            healthy_nodes: 0,
            unhealthy_nodes: 0,
            warnings: Vec::new(),
            critical_issues: Vec::new(),
            recommendations: Vec::new(),
        };

        // Count healthy/unhealthy nodes
        for node in state.nodes.values() {
            match node.status {
                NodeStatus::Active | NodeStatus::Idle => health.healthy_nodes += 1,
                NodeStatus::Failed | NodeStatus::Offline => health.unhealthy_nodes += 1,
                _ => {}
            }
        }

        // Check for critical issues
        if health.unhealthy_nodes > 0 {
            health.critical_issues.push(CriticalIssue {
                issue_type: "NodeFailure".to_string(),
                description: format!("{} nodes are offline or failed", health.unhealthy_nodes),
                impact: "Reduced cluster capacity and potential data loss".to_string(),
                resolution: "Investigate and restart failed nodes".to_string(),
                affected_nodes: state.nodes.values()
                    .filter(|n| matches!(n.status, NodeStatus::Failed | NodeStatus::Offline))
                    .map(|n| n.node_id.clone())
                    .collect(),
            });
        }

        // Check for warnings
        for node in state.nodes.values() {
            if node.resources.cpu_usage_percent > 80.0 {
                health.warnings.push(HealthWarning {
                    severity: WarningSeverity::High,
                    component: "CPU".to_string(),
                    message: format!("Node {} has high CPU usage: {:.1}%", 
                        node.node_id, node.resources.cpu_usage_percent),
                    affected_nodes: vec![node.node_id.clone()],
                });
            }

            if node.resources.memory_usage_mb as f64 / node.resources.memory_total_mb as f64 > 0.9 {
                health.warnings.push(HealthWarning {
                    severity: WarningSeverity::High,
                    component: "Memory".to_string(),
                    message: format!("Node {} has high memory usage", node.node_id),
                    affected_nodes: vec![node.node_id.clone()],
                });
            }
        }

        // Determine overall status
        if !health.critical_issues.is_empty() {
            health.overall_status = HealthStatus::Critical;
        } else if !health.warnings.is_empty() {
            health.overall_status = HealthStatus::Degraded;
        } else if health.healthy_nodes > 0 {
            health.overall_status = HealthStatus::Healthy;
        }

        // Add recommendations
        if state.work_distribution.load_balance_score < 0.7 {
            health.recommendations.push(
                "Consider rebalancing work distribution for better performance".to_string()
            );
        }

        if state.metrics.error_rate_percent > 5.0 {
            health.recommendations.push(
                "High error rate detected, investigate error logs".to_string()
            );
        }

        health
    }

    /// Calculate cluster metrics
    fn calculate_cluster_metrics(state: &ClusterState) -> ClusterMetrics {
        let mut metrics = ClusterMetrics {
            total_events_processed: 0,
            events_per_second: 0.0,
            total_blocks_indexed: 0,
            blocks_per_second: 0.0,
            average_latency_ms: 0.0,
            error_rate_percent: 0.0,
            storage_size_gb: 0.0,
            api_requests_per_second: 0.0,
        };

        // Aggregate metrics from all nodes
        let total_latency = 0.0;
        let latency_count = 0;

        for node in state.nodes.values() {
            metrics.total_events_processed += node.workload.completed_tasks_24h;
            
            if let Some(duration) = state.timestamp.duration_since(SystemTime::now() - Duration::from_secs(86400)).ok() {
                metrics.events_per_second = metrics.total_events_processed as f64 / duration.as_secs_f64();
            }
        }

        if latency_count > 0 {
            metrics.average_latency_ms = total_latency / latency_count as f64;
        }

        metrics
    }

    /// Get current cluster state
    pub async fn get_state(&self) -> ClusterState {
        self.cluster_state.read().await.clone()
    }

    /// Get node state by ID
    pub async fn get_node_state(&self, node_id: &str) -> Option<NodeState> {
        let state = self.cluster_state.read().await;
        state.nodes.get(node_id).cloned()
    }

    /// Update node state
    pub async fn update_node_state(&self, node_state: NodeState) {
        let mut state = self.cluster_state.write().await;
        state.nodes.insert(node_state.node_id.clone(), node_state);
    }

    /// Get cluster health
    pub async fn get_health(&self) -> ClusterHealth {
        let state = self.cluster_state.read().await;
        state.health.clone()
    }

    /// Get work distribution
    pub async fn get_work_distribution(&self) -> WorkDistribution {
        let state = self.cluster_state.read().await;
        state.work_distribution.clone()
    }

    /// Generate state report
    pub async fn generate_report(&self) -> ClusterStateReport {
        let state = self.cluster_state.read().await;
        
        ClusterStateReport {
            generated_at: SystemTime::now(),
            cluster_id: state.metadata.cluster_id.clone(),
            summary: ClusterSummary {
                total_nodes: state.nodes.len(),
                active_nodes: state.nodes.values().filter(|n| n.status == NodeStatus::Active).count(),
                total_tasks: state.work_distribution.total_tasks,
                active_tasks: state.work_distribution.active_tasks,
                health_status: state.health.overall_status.clone(),
                events_per_second: state.metrics.events_per_second,
                error_rate: state.metrics.error_rate_percent,
            },
            node_details: state.nodes.clone(),
            work_distribution: state.work_distribution.clone(),
            health_details: state.health.clone(),
            recommendations: state.health.recommendations.clone(),
        }
    }
}

/// Cluster state report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStateReport {
    pub generated_at: SystemTime,
    pub cluster_id: String,
    pub summary: ClusterSummary,
    pub node_details: HashMap<String, NodeState>,
    pub work_distribution: WorkDistribution,
    pub health_details: ClusterHealth,
    pub recommendations: Vec<String>,
}

/// Cluster summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub total_tasks: u64,
    pub active_tasks: u64,
    pub health_status: HealthStatus,
    pub events_per_second: f64,
    pub error_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_state_inspector() {
        let inspector = ClusterStateInspector::new(
            "test-node".to_string(),
            "test-cluster".to_string(),
        );

        let state = inspector.get_state().await;
        assert_eq!(state.metadata.cluster_id, "test-cluster");
        assert_eq!(state.nodes.len(), 0);
    }

    #[tokio::test]
    async fn test_node_state_update() {
        let inspector = ClusterStateInspector::new(
            "test-node".to_string(),
            "test-cluster".to_string(),
        );

        let node_state = NodeState {
            node_id: "node1".to_string(),
            status: NodeStatus::Active,
            capabilities: NodeCapabilities {
                supported_chains: vec!["ethereum".to_string()],
                max_concurrent_tasks: 10,
                features: vec!["indexing".to_string()],
                version: "1.0.0".to_string(),
            },
            resources: NodeResources {
                cpu_usage_percent: 45.0,
                memory_usage_mb: 512,
                memory_total_mb: 1024,
                disk_usage_gb: 10.0,
                disk_total_gb: 100.0,
                network_bandwidth_mbps: 100.0,
                open_connections: 50,
            },
            workload: NodeWorkload {
                active_tasks: vec![],
                queued_tasks: 0,
                completed_tasks_24h: 1000,
                failed_tasks_24h: 5,
                average_task_duration: Duration::from_secs(30),
                current_throughput: 10.0,
            },
            last_heartbeat: SystemTime::now(),
            uptime: Duration::from_secs(3600),
            errors: vec![],
        };

        inspector.update_node_state(node_state).await;

        let retrieved = inspector.get_node_state("node1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().node_id, "node1");
    }

    #[tokio::test]
    async fn test_health_calculation() {
        let inspector = ClusterStateInspector::new(
            "test-node".to_string(),
            "test-cluster".to_string(),
        );

        // Add a healthy node
        let healthy_node = NodeState {
            node_id: "healthy".to_string(),
            status: NodeStatus::Active,
            capabilities: NodeCapabilities {
                supported_chains: vec!["ethereum".to_string()],
                max_concurrent_tasks: 10,
                features: vec![],
                version: "1.0.0".to_string(),
            },
            resources: NodeResources {
                cpu_usage_percent: 30.0,
                memory_usage_mb: 512,
                memory_total_mb: 2048,
                disk_usage_gb: 20.0,
                disk_total_gb: 100.0,
                network_bandwidth_mbps: 50.0,
                open_connections: 25,
            },
            workload: NodeWorkload {
                active_tasks: vec![],
                queued_tasks: 0,
                completed_tasks_24h: 500,
                failed_tasks_24h: 0,
                average_task_duration: Duration::from_secs(20),
                current_throughput: 5.0,
            },
            last_heartbeat: SystemTime::now(),
            uptime: Duration::from_secs(7200),
            errors: vec![],
        };

        inspector.update_node_state(healthy_node).await;

        let health = inspector.get_health().await;
        assert_eq!(health.healthy_nodes, 1);
        assert_eq!(health.unhealthy_nodes, 0);
    }
}