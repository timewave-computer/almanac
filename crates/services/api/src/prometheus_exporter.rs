// Prometheus metrics exporter for Almanac monitoring
//
// This module provides HTTP endpoints for exporting metrics in Prometheus format,
// including system metrics, application metrics, and custom metrics from all components.

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use tokio::sync::RwLock;

use indexer_core::metrics::{AlmanacMetrics, MetricsRegistry};
use indexer_core::health::HealthMonitor;
use indexer_core::observability::ObservabilitySystem;
use indexer_core::{Error, Result};

/// Prometheus exporter state
#[derive(Clone)]
pub struct PrometheusExporterState {
    /// Almanac metrics instance
    pub almanac_metrics: Arc<AlmanacMetrics>,
    
    /// Health monitor instance
    pub health_monitor: Arc<HealthMonitor>,
    
    /// Observability system instance
    pub observability_system: Arc<ObservabilitySystem>,
    
    /// Custom metrics registries
    pub custom_registries: Arc<RwLock<HashMap<String, Arc<MetricsRegistry>>>>,
    
    /// Exporter configuration
    pub config: PrometheusExporterConfig,
}

/// Prometheus exporter configuration
#[derive(Debug, Clone)]
pub struct PrometheusExporterConfig {
    /// Enable metric aggregation
    pub enable_aggregation: bool,
    
    /// Metric namespace prefix
    pub namespace: String,
    
    /// Labels to add to all metrics
    pub global_labels: HashMap<String, String>,
    
    /// Metric collection timeout
    pub collection_timeout: Duration,
    
    /// Enable debug metrics
    pub enable_debug_metrics: bool,
}

impl Default for PrometheusExporterConfig {
    fn default() -> Self {
        let mut global_labels = HashMap::new();
        global_labels.insert("service".to_string(), "almanac".to_string());
        global_labels.insert("environment".to_string(), "production".to_string());
        
        Self {
            enable_aggregation: true,
            namespace: "almanac".to_string(),
            global_labels,
            collection_timeout: Duration::from_secs(5),
            enable_debug_metrics: false,
        }
    }
}

/// Query parameters for metrics endpoint
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    /// Include debug metrics
    #[serde(default)]
    pub debug: bool,
    
    /// Filter by metric name pattern
    pub filter: Option<String>,
    
    /// Include help text
    #[serde(default = "default_true")]
    pub include_help: bool,
    
    /// Include metric type
    #[serde(default = "default_true")]
    pub include_type: bool,
}

fn default_true() -> bool {
    true
}

/// Create Prometheus exporter router
pub fn create_prometheus_router(state: PrometheusExporterState) -> Router {
    Router::new()
        .route("/metrics", get(get_metrics))
        .route("/metrics/fdb", get(get_fdb_metrics))
        .route("/metrics/chains", get(get_chain_metrics))
        .route("/metrics/health", get(get_health_metrics))
        .route("/metrics/custom/:registry", get(get_custom_metrics))
        .with_state(state)
}

/// Main metrics endpoint handler
pub async fn get_metrics(
    State(state): State<PrometheusExporterState>,
    Query(query): Query<MetricsQuery>,
) -> Response {
    let mut output = String::new();
    
    // Add header
    output.push_str("# Almanac Indexer Metrics\n");
    output.push_str(&format!("# Generated at: {}\n\n", chrono::Utc::now()));
    
    // Collect core metrics
    let core_metrics = state.almanac_metrics.export_prometheus().await;
    output.push_str(&core_metrics);
    output.push_str("\n");
    
    // Collect health metrics
    let health_metrics = collect_health_metrics(&state).await;
    output.push_str(&health_metrics);
    output.push_str("\n");
    
    // Collect observability metrics
    let obs_metrics = collect_observability_metrics(&state).await;
    output.push_str(&obs_metrics);
    output.push_str("\n");
    
    // Collect FDB metrics
    let fdb_metrics = collect_fdb_metrics(&state).await;
    output.push_str(&fdb_metrics);
    output.push_str("\n");
    
    // Collect chain-specific metrics
    let chain_metrics = collect_chain_metrics(&state).await;
    output.push_str(&chain_metrics);
    output.push_str("\n");
    
    // Include debug metrics if requested
    if query.debug || state.config.enable_debug_metrics {
        let debug_metrics = collect_debug_metrics(&state).await;
        output.push_str(&debug_metrics);
        output.push_str("\n");
    }
    
    // Apply filter if provided
    if let Some(filter) = query.filter {
        output = filter_metrics(&output, &filter);
    }
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(output.into())
        .unwrap()
}

/// Get FDB-specific metrics
pub async fn get_fdb_metrics(
    State(state): State<PrometheusExporterState>,
    Query(_query): Query<MetricsQuery>,
) -> Response {
    let metrics = collect_fdb_metrics(&state).await;
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics.into())
        .unwrap()
}

/// Get chain-specific metrics
pub async fn get_chain_metrics(
    State(state): State<PrometheusExporterState>,
    Query(_query): Query<MetricsQuery>,
) -> Response {
    let metrics = collect_chain_metrics(&state).await;
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics.into())
        .unwrap()
}

/// Get health metrics
pub async fn get_health_metrics(
    State(state): State<PrometheusExporterState>,
    Query(_query): Query<MetricsQuery>,
) -> Response {
    let metrics = collect_health_metrics(&state).await;
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics.into())
        .unwrap()
}

/// Get custom metrics from a specific registry
pub async fn get_custom_metrics(
    State(state): State<PrometheusExporterState>,
    axum::extract::Path(registry_name): axum::extract::Path<String>,
    Query(_query): Query<MetricsQuery>,
) -> Response {
    let registries = state.custom_registries.read().await;
    
    if let Some(registry) = registries.get(&registry_name) {
        let metrics = registry.export_prometheus().await;
        
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(metrics.into())
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Registry not found".into())
            .unwrap()
    }
}

/// Collect health-related metrics
async fn collect_health_metrics(state: &PrometheusExporterState) -> String {
    let mut output = String::new();
    
    // Component health status
    output.push_str("# HELP almanac_component_health_status Health status of components (1=healthy, 0=unhealthy)\n");
    output.push_str("# TYPE almanac_component_health_status gauge\n");
    
    let all_health = state.health_monitor.get_all_health().await;
    for (component, health_result) in all_health {
        let value = match health_result.status {
            indexer_core::health::HealthStatus::Healthy => 1.0,
            _ => 0.0,
        };
        
        output.push_str(&format!(
            "almanac_component_health_status{{component=\"{}\",status=\"{}\"}} {}\n",
            component,
            health_result.status,
            value
        ));
    }
    
    // Health check statistics
    let stats = state.health_monitor.get_stats().await;
    
    output.push_str("\n# HELP almanac_health_checks_total Total health checks performed\n");
    output.push_str("# TYPE almanac_health_checks_total counter\n");
    output.push_str(&format!("almanac_health_checks_total {}\n", stats.total_checks));
    
    output.push_str("\n# HELP almanac_health_checks_failed_total Total failed health checks\n");
    output.push_str("# TYPE almanac_health_checks_failed_total counter\n");
    output.push_str(&format!("almanac_health_checks_failed_total {}\n", stats.failed_checks));
    
    output.push_str("\n# HELP almanac_component_restarts_total Total component restarts\n");
    output.push_str("# TYPE almanac_component_restarts_total counter\n");
    output.push_str(&format!("almanac_component_restarts_total {}\n", stats.restart_attempts));
    
    output
}

/// Collect observability system metrics
async fn collect_observability_metrics(state: &PrometheusExporterState) -> String {
    let mut output = String::new();
    
    // System metrics
    let sys_metrics = state.observability_system.get_system_metrics().await;
    
    output.push_str("# HELP almanac_cpu_usage_percent Current CPU usage percentage\n");
    output.push_str("# TYPE almanac_cpu_usage_percent gauge\n");
    output.push_str(&format!("almanac_cpu_usage_percent {}\n", sys_metrics.cpu_usage_percent));
    
    output.push_str("\n# HELP almanac_memory_usage_bytes Current memory usage in bytes\n");
    output.push_str("# TYPE almanac_memory_usage_bytes gauge\n");
    output.push_str(&format!("almanac_memory_usage_bytes {}\n", sys_metrics.memory_usage_bytes));
    
    output.push_str("\n# HELP almanac_memory_usage_percent Current memory usage percentage\n");
    output.push_str("# TYPE almanac_memory_usage_percent gauge\n");
    output.push_str(&format!("almanac_memory_usage_percent {}\n", sys_metrics.memory_usage_percent));
    
    output.push_str("\n# HELP almanac_active_connections Number of active connections\n");
    output.push_str("# TYPE almanac_active_connections gauge\n");
    output.push_str(&format!("almanac_active_connections {}\n", sys_metrics.active_connections));
    
    output.push_str("\n# HELP almanac_goroutines_count Number of active goroutines\n");
    output.push_str("# TYPE almanac_goroutines_count gauge\n");
    output.push_str(&format!("almanac_goroutines_count {}\n", sys_metrics.thread_count));
    
    // Application metrics
    let app_metrics = state.observability_system.get_application_metrics().await;
    
    output.push_str("\n# HELP almanac_events_per_second Events processed per second\n");
    output.push_str("# TYPE almanac_events_per_second gauge\n");
    output.push_str(&format!("almanac_events_per_second {}\n", app_metrics.events_per_second));
    
    output.push_str("\n# HELP almanac_error_rate_percent Error rate percentage\n");
    output.push_str("# TYPE almanac_error_rate_percent gauge\n");
    output.push_str(&format!("almanac_error_rate_percent {}\n", app_metrics.error_rate_percent));
    
    output.push_str("\n# HELP almanac_cache_hit_rate_percent Cache hit rate percentage\n");
    output.push_str("# TYPE almanac_cache_hit_rate_percent gauge\n");
    output.push_str(&format!("almanac_cache_hit_rate_percent {}\n", app_metrics.cache_hit_rate_percent));
    
    output
}

/// Collect FDB-specific metrics
async fn collect_fdb_metrics(_state: &PrometheusExporterState) -> String {
    let mut output = String::new();
    
    // FDB operation metrics
    output.push_str("# HELP fdb_read_operations_total Total FDB read operations\n");
    output.push_str("# TYPE fdb_read_operations_total counter\n");
    output.push_str("fdb_read_operations_total 0\n"); // Placeholder
    
    output.push_str("\n# HELP fdb_write_operations_total Total FDB write operations\n");
    output.push_str("# TYPE fdb_write_operations_total counter\n");
    output.push_str("fdb_write_operations_total 0\n"); // Placeholder
    
    output.push_str("\n# HELP fdb_operation_latency_seconds FDB operation latency\n");
    output.push_str("# TYPE fdb_operation_latency_seconds histogram\n");
    output.push_str("fdb_operation_latency_seconds_bucket{le=\"0.001\"} 0\n");
    output.push_str("fdb_operation_latency_seconds_bucket{le=\"0.01\"} 0\n");
    output.push_str("fdb_operation_latency_seconds_bucket{le=\"0.1\"} 0\n");
    output.push_str("fdb_operation_latency_seconds_bucket{le=\"1\"} 0\n");
    output.push_str("fdb_operation_latency_seconds_bucket{le=\"+Inf\"} 0\n");
    output.push_str("fdb_operation_latency_seconds_count 0\n");
    output.push_str("fdb_operation_latency_seconds_sum 0\n");
    
    output.push_str("\n# HELP fdb_storage_used_bytes FDB storage used in bytes\n");
    output.push_str("# TYPE fdb_storage_used_bytes gauge\n");
    output.push_str("fdb_storage_used_bytes 0\n"); // Placeholder
    
    output.push_str("\n# HELP fdb_conflict_retries_total Total FDB conflict retries\n");
    output.push_str("# TYPE fdb_conflict_retries_total counter\n");
    output.push_str("fdb_conflict_retries_total 0\n"); // Placeholder
    
    output.push_str("\n# HELP fdb_cluster_machines Number of machines in FDB cluster\n");
    output.push_str("# TYPE fdb_cluster_machines gauge\n");
    output.push_str("fdb_cluster_machines 1\n"); // Placeholder
    
    output.push_str("\n# HELP fdb_network_bytes_received_total Total bytes received by FDB\n");
    output.push_str("# TYPE fdb_network_bytes_received_total counter\n");
    output.push_str("fdb_network_bytes_received_total 0\n"); // Placeholder
    
    output.push_str("\n# HELP fdb_network_bytes_sent_total Total bytes sent by FDB\n");
    output.push_str("# TYPE fdb_network_bytes_sent_total counter\n");
    output.push_str("fdb_network_bytes_sent_total 0\n"); // Placeholder
    
    output.push_str("\n# HELP fdb_process_status Status of FDB processes\n");
    output.push_str("# TYPE fdb_process_status gauge\n");
    output.push_str("fdb_process_status{process=\"coordinator\",role=\"coordinator\"} 1\n");
    
    output
}

/// Collect chain-specific metrics
async fn collect_chain_metrics(state: &PrometheusExporterState) -> String {
    let mut output = String::new();
    
    let all_chain_metrics = state.observability_system.get_all_chain_metrics().await;
    
    for (chain_id, metrics) in all_chain_metrics {
        // Current block height
        output.push_str(&format!(
            "\n# HELP almanac_chain_latest_block Latest processed block per chain\n"
        ));
        output.push_str("# TYPE almanac_chain_latest_block gauge\n");
        output.push_str(&format!(
            "almanac_chain_latest_block{{chain=\"{}\"}} {}\n",
            chain_id, metrics.current_block_height
        ));
        
        // Blocks behind head
        output.push_str(&format!(
            "\n# HELP almanac_chain_blocks_behind_head Blocks behind chain head\n"
        ));
        output.push_str("# TYPE almanac_chain_blocks_behind_head gauge\n");
        output.push_str(&format!(
            "almanac_chain_blocks_behind_head{{chain=\"{}\"}} {}\n",
            chain_id, metrics.blocks_behind_head
        ));
        
        // Event processing metrics
        output.push_str(&format!(
            "\n# HELP almanac_chain_events_processed_total Events processed per chain\n"
        ));
        output.push_str("# TYPE almanac_chain_events_processed_total counter\n");
        output.push_str(&format!(
            "almanac_chain_events_processed_total{{chain=\"{}\"}} 0\n", // Placeholder
            chain_id
        ));
        
        // Reorg events
        output.push_str(&format!(
            "\n# HELP almanac_chain_reorg_events_total Reorg events per chain\n"
        ));
        output.push_str("# TYPE almanac_chain_reorg_events_total counter\n");
        output.push_str(&format!(
            "almanac_chain_reorg_events_total{{chain=\"{}\"}} {}\n",
            chain_id, metrics.reorg_count
        ));
        
        // Connection status
        let connection_value = match metrics.connection_status {
            indexer_core::observability::ConnectionStatus::Connected => 1,
            _ => 0,
        };
        output.push_str(&format!(
            "\n# HELP almanac_chain_connection_status Chain connection status (1=connected, 0=disconnected)\n"
        ));
        output.push_str("# TYPE almanac_chain_connection_status gauge\n");
        output.push_str(&format!(
            "almanac_chain_connection_status{{chain=\"{}\",status=\"{:?}\"}} {}\n",
            chain_id, metrics.connection_status, connection_value
        ));
    }
    
    output
}

/// Collect debug metrics
async fn collect_debug_metrics(_state: &PrometheusExporterState) -> String {
    let mut output = String::new();
    
    output.push_str("# DEBUG METRICS\n");
    output.push_str("# These metrics are for debugging purposes only\n\n");
    
    // Rust runtime metrics
    output.push_str("# HELP debug_rust_threads_count Number of Rust threads\n");
    output.push_str("# TYPE debug_rust_threads_count gauge\n");
    output.push_str(&format!("debug_rust_threads_count {}\n", 
        std::thread::available_parallelism().map(|p| p.get()).unwrap_or(0)
    ));
    
    // Build info
    output.push_str("\n# HELP almanac_build_info Build information\n");
    output.push_str("# TYPE almanac_build_info gauge\n");
    output.push_str(&format!(
        "almanac_build_info{{version=\"{}\",rust_version=\"{}\"}} 1\n",
        env!("CARGO_PKG_VERSION"),
        env!("RUSTC_VERSION", "unknown")
    ));
    
    output
}

/// Filter metrics by pattern
fn filter_metrics(metrics: &str, pattern: &str) -> String {
    let lines: Vec<&str> = metrics.lines().collect();
    let mut filtered = Vec::new();
    let mut include_next = false;
    
    for line in lines {
        if line.starts_with('#') {
            // Check if this is a HELP or TYPE line for a metric matching the pattern
            if line.contains("HELP") || line.contains("TYPE") {
                let metric_name = line.split_whitespace().nth(2);
                if let Some(name) = metric_name {
                    include_next = name.contains(pattern);
                    if include_next {
                        filtered.push(line);
                    }
                }
            } else if include_next {
                filtered.push(line);
            }
        } else if !line.is_empty() && include_next {
            // This is a metric line
            if line.contains(pattern) || include_next {
                filtered.push(line);
            }
        } else if line.is_empty() {
            if !filtered.is_empty() && !filtered.last().unwrap().is_empty() {
                filtered.push(line);
            }
        }
    }
    
    filtered.join("\n")
}

/// Initialize Prometheus exporter with default components
pub async fn initialize_prometheus_exporter(
    almanac_metrics: Arc<AlmanacMetrics>,
    health_monitor: Arc<HealthMonitor>,
    observability_system: Arc<ObservabilitySystem>,
) -> PrometheusExporterState {
    PrometheusExporterState {
        almanac_metrics,
        health_monitor,
        observability_system,
        custom_registries: Arc::new(RwLock::new(HashMap::new())),
        config: PrometheusExporterConfig::default(),
    }
}

/// Register a custom metrics registry
pub async fn register_custom_registry(
    state: &PrometheusExporterState,
    name: String,
    registry: Arc<MetricsRegistry>,
) -> Result<()> {
    let mut registries = state.custom_registries.write().await;
    registries.insert(name, registry);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    
    #[tokio::test]
    async fn test_metrics_endpoint() {
        let almanac_metrics = Arc::new(AlmanacMetrics::new().await.unwrap());
        let health_monitor = Arc::new(HealthMonitor::new());
        let observability_system = Arc::new(ObservabilitySystem::new(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ));
        
        let state = initialize_prometheus_exporter(
            almanac_metrics,
            health_monitor,
            observability_system,
        ).await;
        
        let app = create_prometheus_router(state);
        let server = TestServer::new(app).unwrap();
        
        // Test main metrics endpoint
        let response = server.get("/metrics").await;
        assert_eq!(response.status_code(), 200);
        assert!(response.headers().get("content-type").unwrap().to_str().unwrap().contains("text/plain"));
        
        let body = response.text();
        assert!(body.contains("# Almanac Indexer Metrics"));
        assert!(body.contains("almanac_events_processed_total"));
    }
    
    #[tokio::test]
    async fn test_metric_filtering() {
        let metrics = r#"# HELP test_metric_one Test metric one
# TYPE test_metric_one counter
test_metric_one 42

# HELP test_metric_two Test metric two
# TYPE test_metric_two gauge
test_metric_two 3.14

# HELP another_metric Another metric
# TYPE another_metric counter
another_metric 100"#;
        
        let filtered = filter_metrics(metrics, "test_metric");
        assert!(filtered.contains("test_metric_one"));
        assert!(filtered.contains("test_metric_two"));
        assert!(!filtered.contains("another_metric"));
    }
}