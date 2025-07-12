# Distributed Logging Infrastructure

The Almanac indexer includes a comprehensive distributed logging system designed to track requests and debug issues across multiple nodes in a distributed deployment.

## Features

### 1. Distributed Correlation IDs

Track requests as they flow through multiple nodes:

```rust
// Create a correlation ID at the entry point
let correlation = logger.create_correlation(None).await;

// Create child correlations as request moves between nodes
let child_correlation = correlation.child("node-2".to_string());

// Log with correlation context
logger.log_with_correlation(
    LogLevel::Info,
    "Processing request".to_string(),
    Some(correlation),
    HashMap::from([("key", "value")]),
).await;
```

### 2. Centralized Logging

All logs from distributed nodes can be aggregated and queried:

```rust
// Configure centralized logging
let config = LoggingConfig {
    enabled: true,
    min_level: LogLevel::Info,
    buffer_size: 10000,
    flush_interval: Duration::from_secs(5),
    enable_correlation: true,
    retention_period: Duration::from_secs(7 * 24 * 3600),
    remote_endpoint: Some("http://log-aggregator:8080".to_string()),
};

// Create logger for a node
let logger = CentralizedLogger::new(config, "node-1".to_string());
logger.start().await?;
```

### 3. Debug Commands

The CLI includes comprehensive debug commands for inspecting the distributed system:

```bash
# Show cluster status
almanac debug cluster-status --detailed

# Inspect specific node
almanac debug node-status --node-id node-2

# Show work distribution
almanac debug work-distribution

# Query logs
almanac debug logs --level error --time-range 60 --limit 100

# Trace a request by correlation ID
almanac debug trace-request abc123-def456

# Analyze errors
almanac debug error-analysis --time-range 60

# Show metrics
almanac debug metrics --metric-type system
almanac debug metrics --metric-type chain --chain ethereum

# Generate diagnostic report
almanac debug diagnostic-report --output report.txt
```

### 4. Cluster State Inspection

Monitor and inspect the state of all nodes in the cluster:

```rust
// Create cluster state inspector
let inspector = ClusterStateInspector::new(
    "node-1".to_string(),
    "my-cluster".to_string(),
);

// Get cluster state
let state = inspector.get_state().await;

// Get specific node state
let node_state = inspector.get_node_state("node-2").await;

// Get health status
let health = inspector.get_health().await;
```

## Architecture

### Components

1. **CentralizedLogger**: Core logging service that handles buffering, correlation tracking, and log forwarding
2. **ClusterStateInspector**: Monitors and tracks the state of all nodes in the cluster
3. **DebugInterface**: Provides a unified interface for all debug commands
4. **ObservabilitySystem**: Integrates with the existing metrics and tracing infrastructure

### Log Flow

1. Application code creates log entries with optional correlation IDs
2. Logs are buffered locally on each node
3. Buffered logs are periodically flushed to:
   - Local storage
   - Remote log aggregation service (if configured)
4. Debug commands can query logs across all nodes

### Correlation ID Structure

```rust
pub struct CorrelationId {
    pub request_id: String,      // Unique ID for the request
    pub origin_node: String,      // Node where request originated
    pub current_node: String,     // Current processing node
    pub parent_id: Option<String>, // Parent correlation for nested requests
    pub created_at: SystemTime,    // When correlation was created
    pub metadata: HashMap<String, String>, // Additional context
}
```

## Usage Examples

### Basic Logging with Correlation

```rust
// Create correlation at request entry
let correlation = logger.create_correlation(None).await;

// Log request received
logger.log_with_correlation(
    LogLevel::Info,
    "Received API request".to_string(),
    Some(correlation.clone()),
    HashMap::from([
        ("endpoint", json!("/api/blocks")),
        ("method", json!("GET")),
    ]),
).await;

// Forward to another service
let child_correlation = correlation.child("processing-service".to_string());
```

### Querying Logs

```rust
// Query logs by various filters
let logs = logger.query_logs(LogQueryFilters {
    min_level: Some(LogLevel::Warn),
    node_id: Some("node-2".to_string()),
    correlation_id: Some("abc123".to_string()),
    start_time: Some(SystemTime::now() - Duration::from_secs(3600)),
    end_time: None,
    limit: Some(100),
}).await;
```

### Using the Debug Interface

```rust
// Create debug interface
let debug_interface = DebugInterface::new(
    cluster_inspector,
    logger,
    observability,
    "node-1".to_string(),
);

// Execute debug commands
let response = debug_interface.execute_command(
    DebugCommand::ClusterStatus
).await?;
```

### Tracing Integration

```rust
use tracing::{info_span, Instrument};

// Create span with correlation
let span = tracing::info_span!(
    "process_request",
    request_id = %correlation.request_id,
    node = %correlation.current_node,
);

// Execute code within span
async {
    info!("Processing within correlated span");
    // ... processing logic
}.instrument(span).await;
```

## Configuration

### Logging Configuration

```toml
[logging]
enabled = true
min_level = "info"
buffer_size = 10000
flush_interval_secs = 5
enable_correlation = true
retention_days = 7
remote_endpoint = "http://log-aggregator:8080"
```

### Environment Variables

- `ALMANAC_LOG_LEVEL`: Override minimum log level
- `ALMANAC_LOG_BUFFER_SIZE`: Override buffer size
- `ALMANAC_LOG_ENDPOINT`: Override remote endpoint

## Best Practices

1. **Always use correlation IDs** for requests that span multiple services
2. **Include structured metadata** in logs for better queryability
3. **Use appropriate log levels** (Error for failures, Warn for anomalies, Info for events, Debug for details)
4. **Regular log rotation** to manage storage
5. **Set up alerts** on error patterns using the error analysis features

## Troubleshooting

### Common Issues

1. **Logs not appearing**: Check buffer size and flush interval
2. **High memory usage**: Reduce buffer size or decrease flush interval
3. **Missing correlations**: Ensure correlation IDs are propagated between services
4. **Performance impact**: Adjust log level to reduce volume

### Debug Tips

1. Use `trace-request` to follow a specific request through the system
2. Use `error-analysis` to identify patterns in failures
3. Generate diagnostic reports periodically for trend analysis
4. Monitor cluster health regularly with `cluster-status`

## Integration with Existing Systems

The logging infrastructure integrates with:

- **Prometheus**: Metrics about log volume, error rates
- **Jaeger**: Correlation IDs are included in trace spans
- **Grafana**: Dashboards for log analysis
- **External log aggregators**: Elasticsearch, Splunk, etc.

## Performance Considerations

- Buffering reduces I/O overhead
- Async logging prevents blocking application code
- Configurable retention prevents unbounded growth
- Efficient serialization using MessagePack or similar
- Log level filtering at source reduces volume