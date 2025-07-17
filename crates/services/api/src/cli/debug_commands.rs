// Debug command interface for inspecting distributed system state
//
// This module provides a comprehensive set of debug commands for
// system inspection, troubleshooting, and diagnostics

use almanac_infrastructure_core::cluster_state::{ClusterStateInspector, NodeStatus, HealthStatus};
use almanac_infrastructure_monitoring::logging::{CentralizedLogger, LogQueryFilters, LogLevel};
use almanac_infrastructure_monitoring::observability::{ObservabilitySystem, SystemMetrics, ApplicationMetrics, ChainMetrics};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::info;

/// Debug command interface
pub struct DebugInterface {
    cluster_inspector: Arc<ClusterStateInspector>,
    logger: Arc<CentralizedLogger>,
    observability: Arc<ObservabilitySystem>,
    node_id: String,
}

impl DebugInterface {
    /// Create a new debug interface
    pub fn new(
        cluster_inspector: Arc<ClusterStateInspector>,
        logger: Arc<CentralizedLogger>,
        observability: Arc<ObservabilitySystem>,
        node_id: String,
    ) -> Self {
        Self {
            cluster_inspector,
            logger,
            observability,
            node_id,
        }
    }

    /// Execute a debug command
    pub async fn execute_command(&self, command: DebugCommand) -> Result<DebugResponse> {
        match command {
            DebugCommand::ClusterStatus => self.get_cluster_status().await,
            DebugCommand::NodeStatus { node_id } => self.get_node_status(node_id).await,
            DebugCommand::WorkDistribution => self.get_work_distribution().await,
            DebugCommand::HealthCheck => self.perform_health_check().await,
            DebugCommand::LogQuery { filters } => self.query_logs(filters).await,
            DebugCommand::TraceRequest { correlation_id } => self.trace_request(correlation_id).await,
            DebugCommand::Metrics { metric_type } => self.get_metrics(metric_type).await,
            DebugCommand::ConnectionMap => self.get_connection_map().await,
            DebugCommand::ErrorAnalysis { time_range } => self.analyze_errors(time_range).await,
            DebugCommand::PerformanceProfile => self.get_performance_profile().await,
            DebugCommand::ConfigDump => self.dump_configuration().await,
            DebugCommand::TaskInspect { task_id } => self.inspect_task(task_id).await,
            DebugCommand::PartitionInfo { partition_id } => self.get_partition_info(partition_id).await,
            DebugCommand::DiagnosticReport => self.generate_diagnostic_report().await,
        }
    }

    /// Get cluster status
    async fn get_cluster_status(&self) -> Result<DebugResponse> {
        let state = self.cluster_inspector.get_state().await;
        
        let summary = ClusterStatusSummary {
            cluster_id: state.metadata.cluster_id,
            environment: state.metadata.environment,
            leader_node: state.metadata.leader_node,
            total_nodes: state.nodes.len(),
            active_nodes: state.nodes.values().filter(|n| n.status == NodeStatus::Active).count(),
            health_status: state.health.overall_status,
            events_per_second: state.metrics.events_per_second,
            error_rate: state.metrics.error_rate_percent,
            uptime: SystemTime::now().duration_since(state.metadata.start_time)
                .unwrap_or(Duration::from_secs(0)),
        };

        Ok(DebugResponse::ClusterStatus(summary))
    }

    /// Get node status
    async fn get_node_status(&self, node_id: Option<String>) -> Result<DebugResponse> {
        let target_node = node_id.unwrap_or_else(|| self.node_id.clone());
        
        if let Some(node_state) = self.cluster_inspector.get_node_state(&target_node).await {
            let details = NodeStatusDetails {
                node_id: node_state.node_id,
                status: node_state.status,
                uptime: node_state.uptime,
                cpu_usage: node_state.resources.cpu_usage_percent,
                memory_usage_mb: node_state.resources.memory_usage_mb,
                memory_total_mb: node_state.resources.memory_total_mb,
                active_tasks: node_state.workload.active_tasks.len(),
                queued_tasks: node_state.workload.queued_tasks,
                completed_tasks_24h: node_state.workload.completed_tasks_24h,
                failed_tasks_24h: node_state.workload.failed_tasks_24h,
                error_count: node_state.errors.len(),
                last_heartbeat: node_state.last_heartbeat,
            };
            
            Ok(DebugResponse::NodeStatus(details))
        } else {
            Err(anyhow::anyhow!("Node {} not found", target_node))
        }
    }

    /// Get work distribution
    async fn get_work_distribution(&self) -> Result<DebugResponse> {
        let distribution = self.cluster_inspector.get_work_distribution().await;
        
        let summary = WorkDistributionSummary {
            total_tasks: distribution.total_tasks,
            active_tasks: distribution.active_tasks,
            distribution_strategy: distribution.distribution_strategy,
            load_balance_score: distribution.load_balance_score,
            partitions: distribution.partition_map.len(),
            partition_details: distribution.partition_map.into_iter()
                .map(|(id, info)| (id, PartitionSummary {
                    assigned_node: info.assigned_node,
                    chain_id: info.chain_id,
                    event_count: info.event_count,
                    last_updated: info.last_updated,
                }))
                .collect(),
        };

        Ok(DebugResponse::WorkDistribution(summary))
    }

    /// Perform health check
    async fn perform_health_check(&self) -> Result<DebugResponse> {
        let health = self.cluster_inspector.get_health().await;
        
        let check_result = HealthCheckResult {
            overall_status: health.overall_status,
            healthy_nodes: health.healthy_nodes,
            unhealthy_nodes: health.unhealthy_nodes,
            warnings: health.warnings.into_iter()
                .map(|w| format!("[{}] {}: {}", 
                    format!("{:?}", w.severity), w.component, w.message))
                .collect(),
            critical_issues: health.critical_issues.into_iter()
                .map(|i| format!("{}: {} (Impact: {})", 
                    i.issue_type, i.description, i.impact))
                .collect(),
            recommendations: health.recommendations,
        };

        Ok(DebugResponse::HealthCheck(check_result))
    }

    /// Query logs
    async fn query_logs(&self, filters: LogQueryFilters) -> Result<DebugResponse> {
        let logs = self.logger.query_logs(filters).await;
        
        let log_entries = logs.into_iter()
            .map(|log| LogEntry {
                timestamp: log.timestamp,
                level: format!("{:?}", log.level),
                node_id: log.node_id,
                message: log.message,
                correlation_id: log.correlation_id.as_ref().map(|c| c.request_id.clone()),
                fields: log.fields,
            })
            .collect();

        Ok(DebugResponse::Logs(log_entries))
    }

    /// Trace a request by correlation ID
    async fn trace_request(&self, correlation_id: String) -> Result<DebugResponse> {
        let filters = LogQueryFilters {
            min_level: None,
            node_id: Some(self.node_id.clone()),
            correlation_id: Some(correlation_id.clone()),
            start_time: None,
            end_time: None,
            limit: Some(1000),
        };

        let logs = self.logger.query_logs(filters).await;
        
        // Build request trace
        let mut trace = RequestTrace {
            correlation_id,
            start_time: None,
            end_time: None,
            duration: None,
            nodes_visited: vec![],
            log_entries: vec![],
            error_occurred: false,
        };

        for log in logs {
            if trace.start_time.is_none() || log.timestamp < trace.start_time.unwrap() {
                trace.start_time = Some(log.timestamp);
            }
            if trace.end_time.is_none() || log.timestamp > trace.end_time.unwrap() {
                trace.end_time = Some(log.timestamp);
            }

            if !trace.nodes_visited.contains(&log.node_id) {
                trace.nodes_visited.push(log.node_id.clone());
            }

            if matches!(log.level, LogLevel::Error) {
                trace.error_occurred = true;
            }

            trace.log_entries.push(LogEntry {
                timestamp: log.timestamp,
                level: format!("{:?}", log.level),
                node_id: log.node_id,
                message: log.message,
                correlation_id: log.correlation_id.as_ref().map(|c| c.request_id.clone()),
                fields: log.fields,
            });
        }

        if let (Some(start), Some(end)) = (trace.start_time, trace.end_time) {
            trace.duration = end.duration_since(start).ok();
        }

        Ok(DebugResponse::RequestTrace(trace))
    }

    /// Get metrics
    async fn get_metrics(&self, metric_type: MetricType) -> Result<DebugResponse> {
        match metric_type {
            MetricType::System => {
                let metrics = self.observability.get_system_metrics().await;
                Ok(DebugResponse::Metrics(MetricsData::System {
                    cpu_usage: metrics.cpu_usage_percent,
                    memory_usage_mb: metrics.memory_usage_bytes / (1024 * 1024),
                    disk_usage_gb: metrics.disk_usage_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                    network_rx_mb: metrics.network_bytes_received / (1024 * 1024),
                    network_tx_mb: metrics.network_bytes_sent / (1024 * 1024),
                    connections: metrics.active_connections,
                }))
            }
            MetricType::Application => {
                let metrics = self.observability.get_application_metrics().await;
                Ok(DebugResponse::Metrics(MetricsData::Application {
                    events_per_second: metrics.events_per_second,
                    blocks_per_second: metrics.blocks_per_second,
                    api_requests_per_second: metrics.api_requests_per_second,
                    avg_response_time_ms: metrics.avg_response_time_ms,
                    error_rate: metrics.error_rate_percent,
                    cache_hit_rate: metrics.cache_hit_rate_percent,
                }))
            }
            MetricType::Chain(chain_id) => {
                if let Some(metrics) = self.observability.get_chain_metrics(&chain_id).await {
                    Ok(DebugResponse::Metrics(MetricsData::Chain {
                        chain_id,
                        current_block: metrics.current_block_height,
                        blocks_behind: metrics.blocks_behind_head,
                        sync_progress: metrics.sync_progress_percent,
                        block_latency_ms: metrics.block_processing_latency_ms,
                        event_latency_ms: metrics.event_processing_latency_ms,
                        reorg_count: metrics.reorg_count,
                    }))
                } else {
                    Err(anyhow::anyhow!("Chain {} not found", chain_id))
                }
            }
        }
    }

    /// Get connection map
    async fn get_connection_map(&self) -> Result<DebugResponse> {
        let state = self.cluster_inspector.get_state().await;
        
        let connections = state.connections.into_iter()
            .map(|conn| ConnectionInfo {
                from: conn.from_node,
                to: conn.to_node,
                connection_type: format!("{:?}", conn.connection_type),
                status: format!("{:?}", conn.status),
                latency_ms: conn.latency_ms,
                bandwidth_mbps: conn.bandwidth_usage_mbps,
            })
            .collect();

        Ok(DebugResponse::ConnectionMap(connections))
    }

    /// Analyze errors
    async fn analyze_errors(&self, time_range: Duration) -> Result<DebugResponse> {
        let start_time = SystemTime::now() - time_range;
        
        let filters = LogQueryFilters {
            min_level: Some(LogLevel::Error),
            node_id: None,
            correlation_id: None,
            start_time: Some(start_time),
            end_time: None,
            limit: Some(1000),
        };

        let error_logs = self.logger.query_logs(filters).await;
        
        // Group errors by type
        let mut error_groups: HashMap<String, Vec<String>> = HashMap::new();
        
        for log in error_logs {
            let error_type = log.fields.get("error_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            
            error_groups.entry(error_type).or_default().push(log.message);
        }

        let analysis = ErrorAnalysis {
            time_range,
            total_errors: error_groups.values().map(|v| v.len()).sum(),
            error_types: error_groups.into_iter()
                .map(|(error_type, messages)| ErrorTypeSummary {
                    error_type,
                    count: messages.len(),
                    sample_messages: messages.into_iter().take(5).collect(),
                })
                .collect(),
        };

        Ok(DebugResponse::ErrorAnalysis(analysis))
    }

    /// Get performance profile
    async fn get_performance_profile(&self) -> Result<DebugResponse> {
        let system_metrics = self.observability.get_system_metrics().await;
        let app_metrics = self.observability.get_application_metrics().await;
        let _state = self.cluster_inspector.get_state().await;
        
        let profile = PerformanceProfile {
            timestamp: SystemTime::now(),
            cpu_usage: system_metrics.cpu_usage_percent,
            memory_usage_percent: system_metrics.memory_usage_percent,
            throughput: ThroughputMetrics {
                events_per_second: app_metrics.events_per_second,
                blocks_per_second: app_metrics.blocks_per_second,
                api_requests_per_second: app_metrics.api_requests_per_second,
            },
            latency: LatencyMetrics {
                avg_response_time_ms: app_metrics.avg_response_time_ms,
                p50_ms: 0.0, // Would be calculated from histogram
                p95_ms: 0.0,
                p99_ms: 0.0,
            },
            resource_utilization: ResourceUtilization {
                active_connections: system_metrics.active_connections,
                thread_count: system_metrics.thread_count,
                file_descriptors: system_metrics.file_descriptor_count,
                queue_depth: app_metrics.queue_depth,
            },
            bottlenecks: self.identify_bottlenecks(&system_metrics, &app_metrics),
        };

        Ok(DebugResponse::PerformanceProfile(profile))
    }

    /// Identify performance bottlenecks
    fn identify_bottlenecks(
        &self,
        system_metrics: &SystemMetrics,
        app_metrics: &ApplicationMetrics,
    ) -> Vec<String> {
        let mut bottlenecks = Vec::new();

        if system_metrics.cpu_usage_percent > 80.0 {
            bottlenecks.push("High CPU usage detected".to_string());
        }

        if system_metrics.memory_usage_percent > 85.0 {
            bottlenecks.push("High memory usage detected".to_string());
        }

        if app_metrics.queue_depth > 1000 {
            bottlenecks.push("Large queue depth indicates processing backlog".to_string());
        }

        if app_metrics.error_rate_percent > 5.0 {
            bottlenecks.push("High error rate impacting performance".to_string());
        }

        if app_metrics.cache_hit_rate_percent < 50.0 {
            bottlenecks.push("Low cache hit rate causing extra database load".to_string());
        }

        bottlenecks
    }

    /// Dump configuration
    async fn dump_configuration(&self) -> Result<DebugResponse> {
        // In a real implementation, this would gather all configuration
        let config = ConfigurationDump {
            node_id: self.node_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: "production".to_string(),
            settings: HashMap::new(), // Would be populated with actual config
        };

        Ok(DebugResponse::Configuration(config))
    }

    /// Inspect a specific task
    async fn inspect_task(&self, task_id: String) -> Result<DebugResponse> {
        let state = self.cluster_inspector.get_state().await;
        
        // Find task across all nodes
        for node in state.nodes.values() {
            if let Some(task) = node.workload.active_tasks.iter().find(|t| t.task_id == task_id) {
                let details = TaskDetails {
                    task_id: task.task_id.clone(),
                    task_type: task.task_type.clone(),
                    chain_id: task.chain_id.clone(),
                    assigned_node: node.node_id.clone(),
                    start_time: task.start_time,
                    progress: task.progress_percent,
                    estimated_completion: task.estimated_completion,
                    status: "Active".to_string(),
                };
                
                return Ok(DebugResponse::TaskDetails(details));
            }
        }

        Err(anyhow::anyhow!("Task {} not found", task_id))
    }

    /// Get partition information
    async fn get_partition_info(&self, partition_id: String) -> Result<DebugResponse> {
        let distribution = self.cluster_inspector.get_work_distribution().await;
        
        if let Some(info) = distribution.partition_map.get(&partition_id) {
            let details = PartitionDetails {
                partition_id,
                assigned_node: info.assigned_node.clone(),
                chain_id: info.chain_id.clone(),
                block_range: info.block_range,
                event_count: info.event_count,
                last_updated: info.last_updated,
            };
            
            Ok(DebugResponse::PartitionInfo(details))
        } else {
            Err(anyhow::anyhow!("Partition {} not found", partition_id))
        }
    }

    /// Generate comprehensive diagnostic report
    async fn generate_diagnostic_report(&self) -> Result<DebugResponse> {
        info!("Generating comprehensive diagnostic report");

        let cluster_state = self.cluster_inspector.get_state().await;
        let system_metrics = self.observability.get_system_metrics().await;
        let app_metrics = self.observability.get_application_metrics().await;
        let health = self.cluster_inspector.get_health().await;

        let report = DiagnosticReport {
            generated_at: SystemTime::now(),
            cluster_id: cluster_state.metadata.cluster_id,
            node_id: self.node_id.clone(),
            cluster_summary: ClusterDiagnostics {
                total_nodes: cluster_state.nodes.len(),
                active_nodes: cluster_state.nodes.values()
                    .filter(|n| n.status == NodeStatus::Active).count(),
                health_status: health.overall_status,
                uptime: SystemTime::now().duration_since(cluster_state.metadata.start_time)
                    .unwrap_or(Duration::from_secs(0)),
            },
            performance_summary: PerformanceDiagnostics {
                cpu_usage: system_metrics.cpu_usage_percent,
                memory_usage: system_metrics.memory_usage_percent,
                events_per_second: app_metrics.events_per_second,
                error_rate: app_metrics.error_rate_percent,
                avg_latency_ms: app_metrics.avg_response_time_ms,
            },
            issues: health.critical_issues.into_iter()
                .map(|i| format!("{}: {}", i.issue_type, i.description))
                .collect(),
            warnings: health.warnings.into_iter()
                .map(|w| format!("{}: {}", w.component, w.message))
                .collect(),
            recommendations: health.recommendations,
        };

        Ok(DebugResponse::DiagnosticReport(report))
    }
}

/// Debug command enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugCommand {
    ClusterStatus,
    NodeStatus { node_id: Option<String> },
    WorkDistribution,
    HealthCheck,
    LogQuery { filters: LogQueryFilters },
    TraceRequest { correlation_id: String },
    Metrics { metric_type: MetricType },
    ConnectionMap,
    ErrorAnalysis { time_range: Duration },
    PerformanceProfile,
    ConfigDump,
    TaskInspect { task_id: String },
    PartitionInfo { partition_id: String },
    DiagnosticReport,
}

/// Metric type for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    System,
    Application,
    Chain(String),
}

/// Debug command response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugResponse {
    ClusterStatus(ClusterStatusSummary),
    NodeStatus(NodeStatusDetails),
    WorkDistribution(WorkDistributionSummary),
    HealthCheck(HealthCheckResult),
    Logs(Vec<LogEntry>),
    RequestTrace(RequestTrace),
    Metrics(MetricsData),
    ConnectionMap(Vec<ConnectionInfo>),
    ErrorAnalysis(ErrorAnalysis),
    PerformanceProfile(PerformanceProfile),
    Configuration(ConfigurationDump),
    TaskDetails(TaskDetails),
    PartitionInfo(PartitionDetails),
    DiagnosticReport(DiagnosticReport),
}

/// Response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatusSummary {
    pub cluster_id: String,
    pub environment: String,
    pub leader_node: Option<String>,
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub health_status: HealthStatus,
    pub events_per_second: f64,
    pub error_rate: f64,
    pub uptime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatusDetails {
    pub node_id: String,
    pub status: NodeStatus,
    pub uptime: Duration,
    pub cpu_usage: f64,
    pub memory_usage_mb: u64,
    pub memory_total_mb: u64,
    pub active_tasks: usize,
    pub queued_tasks: u32,
    pub completed_tasks_24h: u64,
    pub failed_tasks_24h: u64,
    pub error_count: usize,
    pub last_heartbeat: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkDistributionSummary {
    pub total_tasks: u64,
    pub active_tasks: u64,
    pub distribution_strategy: String,
    pub load_balance_score: f64,
    pub partitions: usize,
    pub partition_details: HashMap<String, PartitionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionSummary {
    pub assigned_node: String,
    pub chain_id: String,
    pub event_count: u64,
    pub last_updated: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub overall_status: HealthStatus,
    pub healthy_nodes: u32,
    pub unhealthy_nodes: u32,
    pub warnings: Vec<String>,
    pub critical_issues: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: String,
    pub node_id: String,
    pub message: String,
    pub correlation_id: Option<String>,
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTrace {
    pub correlation_id: String,
    pub start_time: Option<SystemTime>,
    pub end_time: Option<SystemTime>,
    pub duration: Option<Duration>,
    pub nodes_visited: Vec<String>,
    pub log_entries: Vec<LogEntry>,
    pub error_occurred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricsData {
    System {
        cpu_usage: f64,
        memory_usage_mb: u64,
        disk_usage_gb: f64,
        network_rx_mb: u64,
        network_tx_mb: u64,
        connections: u32,
    },
    Application {
        events_per_second: f64,
        blocks_per_second: f64,
        api_requests_per_second: f64,
        avg_response_time_ms: f64,
        error_rate: f64,
        cache_hit_rate: f64,
    },
    Chain {
        chain_id: String,
        current_block: u64,
        blocks_behind: u64,
        sync_progress: f64,
        block_latency_ms: f64,
        event_latency_ms: f64,
        reorg_count: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub from: String,
    pub to: String,
    pub connection_type: String,
    pub status: String,
    pub latency_ms: f64,
    pub bandwidth_mbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAnalysis {
    pub time_range: Duration,
    pub total_errors: usize,
    pub error_types: Vec<ErrorTypeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorTypeSummary {
    pub error_type: String,
    pub count: usize,
    pub sample_messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    pub timestamp: SystemTime,
    pub cpu_usage: f64,
    pub memory_usage_percent: f64,
    pub throughput: ThroughputMetrics,
    pub latency: LatencyMetrics,
    pub resource_utilization: ResourceUtilization,
    pub bottlenecks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub events_per_second: f64,
    pub blocks_per_second: f64,
    pub api_requests_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub avg_response_time_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    pub active_connections: u32,
    pub thread_count: u32,
    pub file_descriptors: u32,
    pub queue_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationDump {
    pub node_id: String,
    pub version: String,
    pub environment: String,
    pub settings: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetails {
    pub task_id: String,
    pub task_type: String,
    pub chain_id: String,
    pub assigned_node: String,
    pub start_time: SystemTime,
    pub progress: f64,
    pub estimated_completion: Option<SystemTime>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionDetails {
    pub partition_id: String,
    pub assigned_node: String,
    pub chain_id: String,
    pub block_range: Option<(u64, u64)>,
    pub event_count: u64,
    pub last_updated: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub generated_at: SystemTime,
    pub cluster_id: String,
    pub node_id: String,
    pub cluster_summary: ClusterDiagnostics,
    pub performance_summary: PerformanceDiagnostics,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterDiagnostics {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub health_status: HealthStatus,
    pub uptime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDiagnostics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub events_per_second: f64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
}

// MetricValue and ClusterMetrics are used but not defined in observability crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    pub cluster_throughput: MetricValue,
    pub total_events_processed: u64,
    pub active_chains: usize,
    pub error_rate: f64,
    pub average_latency_ms: f64,
    pub chain_metrics: HashMap<String, ChainMetrics>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use almanac_infrastructure_monitoring::logging::LoggingConfig;
    use almanac_infrastructure_monitoring::observability::{MetricsConfig, TracingConfig, HealthConfig, AlertingConfig};

    #[tokio::test]
    async fn test_debug_interface() {
        let cluster_inspector = Arc::new(ClusterStateInspector::new(
            "test-node".to_string(),
            "test-cluster".to_string(),
        ));
        
        let logger = Arc::new(CentralizedLogger::new(
            LoggingConfig::default(),
            "test-node".to_string(),
        ));
        
        let observability = Arc::new(ObservabilitySystem::new(
            MetricsConfig::default(),
            TracingConfig::default(),
            HealthConfig::default(),
            AlertingConfig::default(),
        ));
        
        let debug_interface = DebugInterface::new(
            cluster_inspector,
            logger,
            observability,
            "test-node".to_string(),
        );

        // Test cluster status command
        let response = debug_interface
            .execute_command(DebugCommand::ClusterStatus)
            .await
            .unwrap();
        
        match response {
            DebugResponse::ClusterStatus(summary) => {
                assert_eq!(summary.cluster_id, "test-cluster");
            }
            _ => panic!("Unexpected response type"),
        }
    }

    #[tokio::test]
    async fn test_health_check_command() {
        let cluster_inspector = Arc::new(ClusterStateInspector::new(
            "test-node".to_string(),
            "test-cluster".to_string(),
        ));
        
        let logger = Arc::new(CentralizedLogger::new(
            LoggingConfig::default(),
            "test-node".to_string(),
        ));
        
        let observability = Arc::new(ObservabilitySystem::new(
            MetricsConfig::default(),
            TracingConfig::default(),
            HealthConfig::default(),
            AlertingConfig::default(),
        ));
        
        let debug_interface = DebugInterface::new(
            cluster_inspector,
            logger,
            observability,
            "test-node".to_string(),
        );

        let response = debug_interface
            .execute_command(DebugCommand::HealthCheck)
            .await
            .unwrap();
        
        match response {
            DebugResponse::HealthCheck(result) => {
                assert_eq!(result.overall_status, HealthStatus::Unknown);
            }
            _ => panic!("Unexpected response type"),
        }
    }
}