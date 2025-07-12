// Example demonstrating distributed logging with correlation IDs
//
// This example shows how to use the logging infrastructure to track
// requests across multiple nodes in a distributed system.

use indexer_core::logging::{
    CentralizedLogger, LoggingConfig, CorrelationId, LogLevel, LogQueryFilters
};
use indexer_core::cluster_state::{ClusterStateInspector, NodeState, NodeStatus};
use indexer_core::debug_commands::{DebugInterface, DebugCommand};
use indexer_core::observability::{
    ObservabilitySystem, MetricsConfig, TracingConfig, HealthConfig, AlertingConfig
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio;
use tracing::{info, debug, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting distributed logging example");

    // Create logging configuration
    let mut logging_config = LoggingConfig::default();
    logging_config.min_level = LogLevel::Debug;
    logging_config.enable_correlation = true;

    // Create centralized loggers for multiple nodes
    let logger_node1 = Arc::new(CentralizedLogger::new(
        logging_config.clone(),
        "node-1".to_string(),
    ));
    
    let logger_node2 = Arc::new(CentralizedLogger::new(
        logging_config.clone(),
        "node-2".to_string(),
    ));
    
    let logger_node3 = Arc::new(CentralizedLogger::new(
        logging_config.clone(),
        "node-3".to_string(),
    ));

    // Start logging services
    logger_node1.start().await?;
    logger_node2.start().await?;
    logger_node3.start().await?;

    // Create cluster state inspector
    let cluster_inspector = Arc::new(ClusterStateInspector::new(
        "node-1".to_string(),
        "example-cluster".to_string(),
    ));

    // Create observability system
    let observability = Arc::new(ObservabilitySystem::new(
        MetricsConfig::default(),
        TracingConfig::default(),
        HealthConfig::default(),
        AlertingConfig::default(),
    ));

    // Start observability
    observability.start().await?;

    // Create debug interface
    let debug_interface = DebugInterface::new(
        Arc::clone(&cluster_inspector),
        Arc::clone(&logger_node1),
        Arc::clone(&observability),
        "node-1".to_string(),
    );

    // Simulate a distributed request flow
    info!("Simulating distributed request processing...");
    
    // 1. Request arrives at node-1
    let correlation = logger_node1.create_correlation(None).await;
    info!("Created correlation ID: {}", correlation.request_id);
    
    logger_node1.log_with_correlation(
        LogLevel::Info,
        "Received user request for block data".to_string(),
        Some(correlation.clone()),
        HashMap::from([
            ("endpoint".to_string(), serde_json::Value::String("/api/blocks/latest".to_string())),
            ("method".to_string(), serde_json::Value::String("GET".to_string())),
        ]),
    ).await;

    // 2. Node-1 forwards to node-2
    let correlation_node2 = correlation.child("node-2".to_string());
    
    logger_node1.log_with_correlation(
        LogLevel::Debug,
        "Forwarding request to node-2 for processing".to_string(),
        Some(correlation.clone()),
        HashMap::from([
            ("target_node".to_string(), serde_json::Value::String("node-2".to_string())),
        ]),
    ).await;
    
    logger_node2.log_with_correlation(
        LogLevel::Info,
        "Received forwarded request from node-1".to_string(),
        Some(correlation_node2.clone()),
        HashMap::new(),
    ).await;

    // 3. Node-2 queries database and forwards to node-3
    logger_node2.log_with_correlation(
        LogLevel::Debug,
        "Querying database for block information".to_string(),
        Some(correlation_node2.clone()),
        HashMap::from([
            ("query_time_ms".to_string(), serde_json::Value::Number(serde_json::Number::from(45))),
        ]),
    ).await;

    let correlation_node3 = correlation_node2.child("node-3".to_string());
    
    logger_node3.log_with_correlation(
        LogLevel::Info,
        "Processing additional block validation".to_string(),
        Some(correlation_node3.clone()),
        HashMap::new(),
    ).await;

    // 4. Simulate an error on node-3
    logger_node3.log_with_correlation(
        LogLevel::Error,
        "Failed to validate block signature".to_string(),
        Some(correlation_node3.clone()),
        HashMap::from([
            ("error_type".to_string(), serde_json::Value::String("InvalidSignature".to_string())),
            ("block_number".to_string(), serde_json::Value::Number(serde_json::Number::from(1000000))),
        ]),
    ).await;

    // 5. Error propagates back
    logger_node2.log_with_correlation(
        LogLevel::Warn,
        "Received error from node-3, attempting retry".to_string(),
        Some(correlation_node2.clone()),
        HashMap::new(),
    ).await;

    logger_node1.log_with_correlation(
        LogLevel::Error,
        "Request failed after retry attempts".to_string(),
        Some(correlation.clone()),
        HashMap::from([
            ("status_code".to_string(), serde_json::Value::Number(serde_json::Number::from(500))),
        ]),
    ).await;

    // Wait a bit for logs to be processed
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Demonstrate log querying
    info!("\n=== Querying logs by correlation ID ===");
    
    let trace_response = debug_interface
        .execute_command(DebugCommand::TraceRequest {
            correlation_id: correlation.request_id.clone(),
        })
        .await?;
    
    println!("\nTrace Response: {:?}", trace_response);

    // Query error logs
    info!("\n=== Querying error logs ===");
    
    let error_logs = logger_node1.query_logs(LogQueryFilters {
        min_level: Some(LogLevel::Error),
        node_id: None,
        correlation_id: None,
        start_time: Some(SystemTime::now() - Duration::from_secs(60)),
        end_time: None,
        limit: Some(10),
    }).await;

    println!("\nFound {} error logs", error_logs.len());
    for log in error_logs {
        println!("  [{:?}] {}: {}", log.level, log.node_id, log.message);
    }

    // Demonstrate cluster state inspection
    info!("\n=== Cluster State Inspection ===");
    
    // Update node states
    cluster_inspector.update_node_state(NodeState {
        node_id: "node-1".to_string(),
        status: NodeStatus::Active,
        capabilities: indexer_core::cluster_state::NodeCapabilities {
            supported_chains: vec!["ethereum".to_string(), "polygon".to_string()],
            max_concurrent_tasks: 100,
            features: vec!["indexing".to_string(), "api".to_string()],
            version: "1.0.0".to_string(),
        },
        resources: indexer_core::cluster_state::NodeResources {
            cpu_usage_percent: 45.5,
            memory_usage_mb: 2048,
            memory_total_mb: 8192,
            disk_usage_gb: 50.0,
            disk_total_gb: 500.0,
            network_bandwidth_mbps: 100.0,
            open_connections: 25,
        },
        workload: indexer_core::cluster_state::NodeWorkload {
            active_tasks: vec![],
            queued_tasks: 10,
            completed_tasks_24h: 50000,
            failed_tasks_24h: 25,
            average_task_duration: Duration::from_secs(5),
            current_throughput: 100.0,
        },
        last_heartbeat: SystemTime::now(),
        uptime: Duration::from_secs(86400), // 1 day
        errors: vec![],
    }).await;

    // Get cluster status
    let cluster_status = debug_interface
        .execute_command(DebugCommand::ClusterStatus)
        .await?;
    
    println!("\nCluster Status: {:?}", cluster_status);

    // Demonstrate error analysis
    info!("\n=== Error Analysis ===");
    
    let error_analysis = debug_interface
        .execute_command(DebugCommand::ErrorAnalysis {
            time_range: Duration::from_secs(3600), // Last hour
        })
        .await?;
    
    println!("\nError Analysis: {:?}", error_analysis);

    // Generate diagnostic report
    info!("\n=== Generating Diagnostic Report ===");
    
    let report = debug_interface
        .execute_command(DebugCommand::DiagnosticReport)
        .await?;
    
    println!("\nDiagnostic Report: {:?}", report);

    // Demonstrate the log_with_correlation macro
    use indexer_core::log_with_correlation;
    
    let test_correlation = logger_node1.create_correlation(Some("test-origin".to_string())).await;
    
    log_with_correlation!(
        logger_node1,
        LogLevel::Info,
        test_correlation,
        "Testing macro with correlation"
    );

    info!("\n✅ Distributed logging example completed successfully");

    Ok(())
}