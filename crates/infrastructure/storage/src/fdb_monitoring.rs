use async_trait::async_trait;
use foundationdb::{Database, FdbResult, KeySelector, RangeOption, Transaction};
use futures::TryStreamExt;
use almanac_infrastructure_core::distributed_monitoring::{
    Alert, ClusterMetrics, DistributedMonitoringService, NodeHealth, NodeMetrics,
};
use almanac_infrastructure_core::Result;
use serde_json;
use std::time::Duration;
use tracing::{debug, info};

const METRICS_PREFIX: &[u8] = b"/monitoring/metrics/";
const HEALTH_PREFIX: &[u8] = b"/monitoring/health/";
const ALERTS_PREFIX: &[u8] = b"/monitoring/alerts/";

pub struct FdbMonitoringService {
    db: Database,
    namespace: String,
}

impl FdbMonitoringService {
    pub fn new(db: Database, namespace: String) -> Self {
        Self { db, namespace }
    }

    fn namespace_key(&self, prefix: &[u8], key: &[u8]) -> Vec<u8> {
        let mut full_key = Vec::with_capacity(self.namespace.len() + prefix.len() + key.len() + 2);
        full_key.push(b'/');
        full_key.extend_from_slice(self.namespace.as_bytes());
        full_key.extend_from_slice(prefix);
        full_key.extend_from_slice(key);
        full_key
    }

    fn metrics_key(&self, node_id: &str, timestamp: u64) -> Vec<u8> {
        let key = format!("{}/{:016x}", node_id, timestamp);
        self.namespace_key(METRICS_PREFIX, key.as_bytes())
    }

    fn health_key(&self, node_id: &str) -> Vec<u8> {
        self.namespace_key(HEALTH_PREFIX, node_id.as_bytes())
    }

    fn alert_key(&self, alert_id: &str) -> Vec<u8> {
        self.namespace_key(ALERTS_PREFIX, alert_id.as_bytes())
    }

    async fn transact<F, R>(&self, f: F) -> FdbResult<R>
    where
        F: Fn(Transaction) -> FdbResult<R>,
    {
        self.db.run(f).await
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[async_trait]
impl DistributedMonitoringService for FdbMonitoringService {
    async fn report_metrics(&self, metrics: NodeMetrics) -> Result<()> {
        let key = self.metrics_key(&metrics.node_id, metrics.timestamp);
        let value = serde_json::to_vec(&metrics)
            .map_err(|e| almanac_infrastructure_core::Error::Serialization(e.to_string()))?;

        self.transact(|mut trx| {
            trx.set(&key, &value);
            Ok(())
        })
        .await
        .map_err(|e| almanac_infrastructure_core::Error::Generic(format!("FDB error: {}", e)))?;

        debug!(
            "Reported metrics for node {} at timestamp {}",
            metrics.node_id, metrics.timestamp
        );

        Ok(())
    }

    async fn get_node_metrics(&self, node_id: &str, duration: Duration) -> Result<Vec<NodeMetrics>> {
        let now = Self::current_timestamp();
        let start_time = now - duration.as_secs();
        
        let start_key = self.metrics_key(node_id, start_time);
        let end_key = self.metrics_key(node_id, now + 1);

        let range = RangeOption {
            begin: KeySelector::first_greater_or_equal(&start_key),
            end: KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Iterator,
            limit: None,
            reverse: false,
        };

        let results = self
            .transact(|trx| {
                let mut results = Vec::new();
                let iter = trx.get_range(&range, 1, false);
                
                futures::executor::block_on(async {
                    let mut stream = iter;
                    while let Ok(Some(kv)) = stream.try_next().await {
                        results.push(kv);
                    }
                });
                
                Ok(results)
            })
            .await
            .map_err(|e| almanac_infrastructure_core::Error::Generic(format!("FDB error: {}", e)))?;

        let mut metrics = Vec::new();
        for kv in results {
            if let Ok(metric) = serde_json::from_slice::<NodeMetrics>(kv.value()) {
                metrics.push(metric);
            }
        }

        Ok(metrics)
    }

    async fn get_cluster_metrics(&self, duration: Duration) -> Result<ClusterMetrics> {
        let now = Self::current_timestamp();
        let start_time = now - duration.as_secs();
        
        // Get all metrics in the time range
        let start_key = self.namespace_key(METRICS_PREFIX, b"");
        let end_key = self.namespace_key(METRICS_PREFIX, b"\xff");

        let range = RangeOption {
            begin: KeySelector::first_greater_or_equal(&start_key),
            end: KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Iterator,
            limit: None,
            reverse: false,
        };

        let results = self
            .transact(|trx| {
                let mut results = Vec::new();
                let iter = trx.get_range(&range, 1, false);
                
                futures::executor::block_on(async {
                    let mut stream = iter;
                    while let Ok(Some(kv)) = stream.try_next().await {
                        results.push(kv);
                    }
                });
                
                Ok(results)
            })
            .await
            .map_err(|e| almanac_infrastructure_core::Error::Generic(format!("FDB error: {}", e)))?;

        let mut all_metrics = Vec::new();
        for kv in results {
            if let Ok(metric) = serde_json::from_slice::<NodeMetrics>(kv.value()) {
                if metric.timestamp >= start_time {
                    all_metrics.push(metric);
                }
            }
        }

        // Aggregate metrics
        let mut node_set = std::collections::HashSet::new();
        let mut total_events = 0u64;
        let mut total_blocks = 0u64;
        let mut total_cpu = 0.0;
        let mut total_memory = 0.0;
        let mut total_errors = 0u64;
        let mut latencies_p50 = Vec::new();
        let mut latencies_p95 = Vec::new();
        let mut latencies_p99 = Vec::new();

        for metric in &all_metrics {
            node_set.insert(metric.node_id.clone());
            total_events += metric.events_processed;
            total_blocks += metric.blocks_indexed;
            total_cpu += metric.cpu_usage;
            total_memory += metric.memory_usage;
            total_errors += metric.errors;
            latencies_p50.push(metric.latency_p50);
            latencies_p95.push(metric.latency_p95);
            latencies_p99.push(metric.latency_p99);
        }

        let total_nodes = node_set.len();
        let metric_count = all_metrics.len() as f64;

        Ok(ClusterMetrics {
            total_nodes,
            healthy_nodes: total_nodes, // Would need health data
            total_events_processed: total_events,
            total_blocks_indexed: total_blocks,
            avg_cpu_usage: if metric_count > 0.0 { total_cpu / metric_count } else { 0.0 },
            avg_memory_usage: if metric_count > 0.0 { total_memory / metric_count } else { 0.0 },
            total_errors,
            cluster_latency_p50: calculate_percentile(&mut latencies_p50, 50),
            cluster_latency_p95: calculate_percentile(&mut latencies_p95, 50),
            cluster_latency_p99: calculate_percentile(&mut latencies_p99, 50),
        })
    }

    async fn report_health(&self, health: NodeHealth) -> Result<()> {
        let key = self.health_key(&health.node_id);
        let value = serde_json::to_vec(&health)
            .map_err(|e| almanac_infrastructure_core::Error::Serialization(e.to_string()))?;

        self.transact(|mut trx| {
            trx.set(&key, &value);
            Ok(())
        })
        .await
        .map_err(|e| almanac_infrastructure_core::Error::Generic(format!("FDB error: {}", e)))?;

        info!(
            "Reported health for node {}: {:?}",
            health.node_id, health.status
        );

        Ok(())
    }

    async fn get_cluster_health(&self) -> Result<Vec<NodeHealth>> {
        let start_key = self.namespace_key(HEALTH_PREFIX, b"");
        let end_key = self.namespace_key(HEALTH_PREFIX, b"\xff");

        let range = RangeOption {
            begin: KeySelector::first_greater_or_equal(&start_key),
            end: KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Iterator,
            limit: None,
            reverse: false,
        };

        let results = self
            .transact(|trx| {
                let mut results = Vec::new();
                let iter = trx.get_range(&range, 1, false);
                
                futures::executor::block_on(async {
                    let mut stream = iter;
                    while let Ok(Some(kv)) = stream.try_next().await {
                        results.push(kv);
                    }
                });
                
                Ok(results)
            })
            .await
            .map_err(|e| almanac_infrastructure_core::Error::Generic(format!("FDB error: {}", e)))?;

        let mut health_statuses = Vec::new();
        for kv in results {
            if let Ok(health) = serde_json::from_slice::<NodeHealth>(kv.value()) {
                health_statuses.push(health);
            }
        }

        Ok(health_statuses)
    }

    async fn set_alert(&self, alert: Alert) -> Result<()> {
        let key = self.alert_key(&alert.alert_id);
        let value = serde_json::to_vec(&alert)
            .map_err(|e| almanac_infrastructure_core::Error::Serialization(e.to_string()))?;

        self.transact(|mut trx| {
            trx.set(&key, &value);
            Ok(())
        })
        .await
        .map_err(|e| almanac_infrastructure_core::Error::Generic(format!("FDB error: {}", e)))?;

        info!(
            "Set alert {}: {:?} - {}",
            alert.alert_id, alert.alert_type, alert.message
        );

        Ok(())
    }

    async fn get_active_alerts(&self) -> Result<Vec<Alert>> {
        let start_key = self.namespace_key(ALERTS_PREFIX, b"");
        let end_key = self.namespace_key(ALERTS_PREFIX, b"\xff");

        let range = RangeOption {
            begin: KeySelector::first_greater_or_equal(&start_key),
            end: KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Iterator,
            limit: None,
            reverse: false,
        };

        let results = self
            .transact(|trx| {
                let mut results = Vec::new();
                let iter = trx.get_range(&range, 1, false);
                
                futures::executor::block_on(async {
                    let mut stream = iter;
                    while let Ok(Some(kv)) = stream.try_next().await {
                        results.push(kv);
                    }
                });
                
                Ok(results)
            })
            .await
            .map_err(|e| almanac_infrastructure_core::Error::Generic(format!("FDB error: {}", e)))?;

        let mut alerts = Vec::new();
        for kv in results {
            if let Ok(alert) = serde_json::from_slice::<Alert>(kv.value()) {
                // Only include unresolved alerts
                if alert.resolved_at.is_none() {
                    alerts.push(alert);
                }
            }
        }

        Ok(alerts)
    }
}

fn calculate_percentile(values: &mut [f64], percentile: u8) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let index = ((percentile as f64 / 100.0) * (values.len() - 1) as f64) as usize;
    values[index]
}

#[cfg(test)]
mod tests {
    use super::*;
    use almanac_infrastructure_core::distributed_monitoring::{HealthStatus, AlertType, IssueSeverity};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_monitoring_basic_operations() {
        // Skip if FDB not available
        let db = match Database::default() {
            Ok(db) => db,
            Err(_) => {
                eprintln!("FoundationDB not available, skipping test");
                return;
            }
        };

        let service = FdbMonitoringService::new(db, "test".to_string());

        // Report metrics
        let metrics = NodeMetrics {
            node_id: "test-node-1".to_string(),
            timestamp: FdbMonitoringService::current_timestamp(),
            cpu_usage: 45.5,
            memory_usage: 60.2,
            events_processed: 1000,
            blocks_indexed: 100,
            correlations_found: 50,
            storage_operations: 500,
            api_requests: 200,
            errors: 5,
            latency_p50: 10.5,
            latency_p95: 25.0,
            latency_p99: 50.0,
        };

        service.report_metrics(metrics.clone()).await.unwrap();

        // Get node metrics
        let retrieved = service
            .get_node_metrics("test-node-1", Duration::from_secs(3600))
            .await
            .unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].node_id, "test-node-1");

        // Report health
        let health = NodeHealth {
            node_id: "test-node-1".to_string(),
            status: HealthStatus::Healthy,
            last_check: FdbMonitoringService::current_timestamp(),
            uptime: Duration::from_secs(3600),
            issues: Vec::new(),
        };

        service.report_health(health).await.unwrap();

        // Get cluster health
        let cluster_health = service.get_cluster_health().await.unwrap();
        assert_eq!(cluster_health.len(), 1);

        // Set an alert
        let alert = Alert {
            alert_id: Uuid::new_v4().to_string(),
            node_id: Some("test-node-1".to_string()),
            alert_type: AlertType::HighCpuUsage,
            severity: IssueSeverity::Warning,
            message: "CPU usage above threshold".to_string(),
            triggered_at: FdbMonitoringService::current_timestamp(),
            resolved_at: None,
        };

        service.set_alert(alert).await.unwrap();

        // Get active alerts
        let active_alerts = service.get_active_alerts().await.unwrap();
        assert_eq!(active_alerts.len(), 1);
    }
}