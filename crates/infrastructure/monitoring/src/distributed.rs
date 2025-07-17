use almanac_infrastructure_core::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Metrics for distributed nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub node_id: String,
    pub timestamp: u64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub events_processed: u64,
    pub blocks_indexed: u64,
    pub correlations_found: u64,
    pub storage_operations: u64,
    pub api_requests: u64,
    pub errors: u64,
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub latency_p99: f64,
}

/// Health status of a distributed node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealth {
    pub node_id: String,
    pub status: HealthStatus,
    pub last_check: u64,
    pub uptime: Duration,
    pub issues: Vec<HealthIssue>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    pub severity: IssueSeverity,
    pub component: String,
    pub message: String,
    pub detected_at: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Distributed monitoring service
#[async_trait]
pub trait DistributedMonitoring: Send + Sync {
    /// Report metrics from a node
    async fn report_metrics(&self, metrics: NodeMetrics) -> Result<()>;
    
    /// Get metrics for all nodes
    async fn get_all_metrics(&self) -> Result<HashMap<String, NodeMetrics>>;
    
    /// Get metrics for a specific node
    async fn get_node_metrics(&self, node_id: &str) -> Result<Option<NodeMetrics>>;
    
    /// Report health status
    async fn report_health(&self, health: NodeHealth) -> Result<()>;
    
    /// Get health status for all nodes
    async fn get_all_health(&self) -> Result<HashMap<String, NodeHealth>>;
    
    /// Get health status for a specific node
    async fn get_node_health(&self, node_id: &str) -> Result<Option<NodeHealth>>;
    
    /// Check if cluster is healthy
    async fn is_cluster_healthy(&self) -> Result<bool>;
    
    /// Get cluster summary
    async fn get_cluster_summary(&self) -> Result<ClusterSummary>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub unhealthy_nodes: usize,
    pub total_events_processed: u64,
    pub total_blocks_indexed: u64,
    pub total_correlations_found: u64,
    pub average_cpu_usage: f64,
    pub average_memory_usage: f64,
    pub cluster_status: HealthStatus,
}

/// In-memory implementation of distributed monitoring
pub struct InMemoryMonitoring {
    metrics: Arc<RwLock<HashMap<String, NodeMetrics>>>,
    health: Arc<RwLock<HashMap<String, NodeHealth>>>,
}

impl Default for InMemoryMonitoring {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMonitoring {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            health: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl DistributedMonitoring for InMemoryMonitoring {
    async fn report_metrics(&self, metrics: NodeMetrics) -> Result<()> {
        let mut metrics_map = self.metrics.write().await;
        metrics_map.insert(metrics.node_id.clone(), metrics);
        Ok(())
    }
    
    async fn get_all_metrics(&self) -> Result<HashMap<String, NodeMetrics>> {
        let metrics_map = self.metrics.read().await;
        Ok(metrics_map.clone())
    }
    
    async fn get_node_metrics(&self, node_id: &str) -> Result<Option<NodeMetrics>> {
        let metrics_map = self.metrics.read().await;
        Ok(metrics_map.get(node_id).cloned())
    }
    
    async fn report_health(&self, health: NodeHealth) -> Result<()> {
        let mut health_map = self.health.write().await;
        health_map.insert(health.node_id.clone(), health);
        Ok(())
    }
    
    async fn get_all_health(&self) -> Result<HashMap<String, NodeHealth>> {
        let health_map = self.health.read().await;
        Ok(health_map.clone())
    }
    
    async fn get_node_health(&self, node_id: &str) -> Result<Option<NodeHealth>> {
        let health_map = self.health.read().await;
        Ok(health_map.get(node_id).cloned())
    }
    
    async fn is_cluster_healthy(&self) -> Result<bool> {
        let health_map = self.health.read().await;
        
        if health_map.is_empty() {
            return Ok(false);
        }
        
        let unhealthy_count = health_map
            .values()
            .filter(|h| h.status == HealthStatus::Unhealthy)
            .count();
            
        Ok(unhealthy_count == 0)
    }
    
    async fn get_cluster_summary(&self) -> Result<ClusterSummary> {
        let metrics_map = self.metrics.read().await;
        let health_map = self.health.read().await;
        
        let total_nodes = health_map.len();
        let healthy_nodes = health_map.values().filter(|h| h.status == HealthStatus::Healthy).count();
        let degraded_nodes = health_map.values().filter(|h| h.status == HealthStatus::Degraded).count();
        let unhealthy_nodes = health_map.values().filter(|h| h.status == HealthStatus::Unhealthy).count();
        
        let total_events_processed: u64 = metrics_map.values().map(|m| m.events_processed).sum();
        let total_blocks_indexed: u64 = metrics_map.values().map(|m| m.blocks_indexed).sum();
        let total_correlations_found: u64 = metrics_map.values().map(|m| m.correlations_found).sum();
        
        let average_cpu_usage = if !metrics_map.is_empty() {
            metrics_map.values().map(|m| m.cpu_usage).sum::<f64>() / metrics_map.len() as f64
        } else {
            0.0
        };
        
        let average_memory_usage = if !metrics_map.is_empty() {
            metrics_map.values().map(|m| m.memory_usage).sum::<f64>() / metrics_map.len() as f64
        } else {
            0.0
        };
        
        let cluster_status = if unhealthy_nodes > 0 {
            HealthStatus::Unhealthy
        } else if degraded_nodes > 0 {
            HealthStatus::Degraded
        } else if healthy_nodes > 0 {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        };
        
        Ok(ClusterSummary {
            total_nodes,
            healthy_nodes,
            degraded_nodes,
            unhealthy_nodes,
            total_events_processed,
            total_blocks_indexed,
            total_correlations_found,
            average_cpu_usage,
            average_memory_usage,
            cluster_status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_metrics_reporting() {
        let monitor = InMemoryMonitoring::new();
        
        let metrics = NodeMetrics {
            node_id: "node1".to_string(),
            timestamp: 1234567890,
            cpu_usage: 45.5,
            memory_usage: 60.2,
            events_processed: 1000,
            blocks_indexed: 100,
            correlations_found: 50,
            storage_operations: 200,
            api_requests: 500,
            errors: 5,
            latency_p50: 10.5,
            latency_p95: 25.0,
            latency_p99: 50.0,
        };
        
        monitor.report_metrics(metrics.clone()).await.unwrap();
        
        let retrieved = monitor.get_node_metrics("node1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().cpu_usage, 45.5);
    }
    
    #[tokio::test]
    async fn test_health_reporting() {
        let monitor = InMemoryMonitoring::new();
        
        let health = NodeHealth {
            node_id: "node1".to_string(),
            status: HealthStatus::Healthy,
            last_check: 1234567890,
            uptime: Duration::from_secs(3600),
            issues: vec![],
        };
        
        monitor.report_health(health).await.unwrap();
        
        assert!(monitor.is_cluster_healthy().await.unwrap());
    }
}