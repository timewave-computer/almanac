// Distributed logging infrastructure for the Almanac indexer
//
// This module provides:
// - Distributed correlation IDs for tracking requests across nodes
// - Centralized logging with structured context
// - Log aggregation and filtering capabilities
// - Integration with distributed tracing

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{info, debug, error, Span};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Distributed correlation ID for tracking requests across nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrelationId {
    /// Unique request ID
    pub request_id: String,
    /// Originating node ID
    pub origin_node: String,
    /// Current node ID
    pub current_node: String,
    /// Parent correlation ID for nested requests
    pub parent_id: Option<String>,
    /// Timestamp when correlation was created
    pub created_at: SystemTime,
    /// Additional context metadata
    pub metadata: HashMap<String, String>,
}

impl CorrelationId {
    /// Create a new correlation ID
    pub fn new(origin_node: String, current_node: String) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            origin_node,
            current_node,
            parent_id: None,
            created_at: SystemTime::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a child correlation ID
    pub fn child(&self, current_node: String) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            origin_node: self.origin_node.clone(),
            current_node,
            parent_id: Some(self.request_id.clone()),
            created_at: SystemTime::now(),
            metadata: self.metadata.clone(),
        }
    }

    /// Add metadata to the correlation ID
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Log entry with distributed context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistributedLogEntry {
    /// Timestamp of the log entry
    pub timestamp: SystemTime,
    /// Log level
    pub level: LogLevel,
    /// Node that generated the log
    pub node_id: String,
    /// Correlation ID for tracking
    pub correlation_id: Option<CorrelationId>,
    /// Log message
    pub message: String,
    /// Structured fields
    pub fields: HashMap<String, serde_json::Value>,
    /// Source location
    pub location: Option<LogLocation>,
    /// Associated span ID from tracing
    pub span_id: Option<String>,
}

/// Log level enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Source location information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogLocation {
    pub file: String,
    pub line: u32,
    pub module: String,
}

/// Centralized logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Enable centralized logging
    pub enabled: bool,
    /// Minimum log level to capture
    pub min_level: LogLevel,
    /// Buffer size for log entries
    pub buffer_size: usize,
    /// Flush interval for buffered logs
    pub flush_interval: Duration,
    /// Enable correlation ID tracking
    pub enable_correlation: bool,
    /// Log retention period
    pub retention_period: Duration,
    /// Remote logging endpoint (if any)
    pub remote_endpoint: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_level: LogLevel::Info,
            buffer_size: 10000,
            flush_interval: Duration::from_secs(5),
            enable_correlation: true,
            retention_period: Duration::from_secs(7 * 24 * 3600), // 7 days
            remote_endpoint: None,
        }
    }
}

/// Centralized logging service
pub struct CentralizedLogger {
    config: LoggingConfig,
    node_id: String,
    buffer: Arc<RwLock<Vec<DistributedLogEntry>>>,
    correlation_context: Arc<RwLock<HashMap<String, CorrelationId>>>,
}

/// Send logs to remote endpoint
async fn send_to_remote(
    endpoint: &str,
    logs: Vec<DistributedLogEntry>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // In a real implementation, this would use HTTP/gRPC to send logs
    // For now, we'll just log that we would send them
    debug!("Would send {} logs to {}", logs.len(), endpoint);
    Ok(())
}

/// Process a log entry locally
fn process_log_entry(entry: DistributedLogEntry) {
    // Format and output the log
    let correlation_str = entry
        .correlation_id
        .as_ref()
        .map(|c| format!(" [{}]", c.request_id))
        .unwrap_or_default();

    println!(
        "{:?} [{}]{} - {}",
        entry.timestamp,
        entry.node_id,
        correlation_str,
        entry.message
    );
}

impl CentralizedLogger {
    /// Create a new centralized logger
    pub fn new(config: LoggingConfig, node_id: String) -> Self {
        Self {
            config,
            node_id,
            buffer: Arc::new(RwLock::new(Vec::with_capacity(10000))),
            correlation_context: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the logging service
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Ok(());
        }

        info!("Starting centralized logging service for node: {}", self.node_id);

        // Start flush task
        let buffer = Arc::clone(&self.buffer);
        let flush_interval = self.config.flush_interval;
        let remote_endpoint = self.config.remote_endpoint.clone();
        let node_id = self.node_id.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(flush_interval);
            
            loop {
                interval.tick().await;
                
                let logs_to_flush = {
                    let mut buffer = buffer.write().await;
                    std::mem::take(&mut *buffer)
                };

                if !logs_to_flush.is_empty() {
                    debug!("Flushing {} log entries from node {}", logs_to_flush.len(), node_id);
                    
                    if let Some(endpoint) = &remote_endpoint {
                        // Send to remote logging service - clone endpoint for async move
                        let endpoint_clone = endpoint.clone();
                        tokio::spawn(async move {
                            if let Err(e) = send_to_remote(&endpoint_clone, logs_to_flush).await {
                                error!("Failed to send logs to remote endpoint: {}", e);
                            }
                        });
                    } else {
                        // Process locally (could write to file, database, etc.)
                        for log in logs_to_flush {
                            process_log_entry(log);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Log an entry with correlation context
    pub async fn log_with_correlation(
        &self,
        level: LogLevel,
        message: String,
        correlation_id: Option<CorrelationId>,
        fields: HashMap<String, serde_json::Value>,
    ) {
        if level < self.config.min_level {
            return;
        }

        let entry = DistributedLogEntry {
            timestamp: SystemTime::now(),
            level,
            node_id: self.node_id.clone(),
            correlation_id,
            message,
            fields,
            location: None, // Would be populated by macro
            span_id: None,  // Would be populated by tracing integration
        };

        let mut buffer = self.buffer.write().await;
        buffer.push(entry);

        // Check if buffer is full
        if buffer.len() >= self.config.buffer_size {
            // Force flush
            let logs_to_flush = std::mem::take(&mut *buffer);
            drop(buffer);

            if let Some(endpoint) = &self.config.remote_endpoint {
                let endpoint_clone = endpoint.clone();
                tokio::spawn(async move {
                    if let Err(e) = send_to_remote(&endpoint_clone, logs_to_flush).await {
                        error!("Failed to send logs to remote endpoint: {}", e);
                    }
                });
            }
        }
    }

    /// Create a new correlation context
    pub async fn create_correlation(&self, origin_node: Option<String>) -> CorrelationId {
        let correlation_id = CorrelationId::new(
            origin_node.unwrap_or_else(|| self.node_id.clone()),
            self.node_id.clone(),
        );

        if self.config.enable_correlation {
            let mut context = self.correlation_context.write().await;
            context.insert(correlation_id.request_id.clone(), correlation_id.clone());
        }

        correlation_id
    }

    /// Get correlation context by request ID
    pub async fn get_correlation(&self, request_id: &str) -> Option<CorrelationId> {
        let context = self.correlation_context.read().await;
        context.get(request_id).cloned()
    }

    /// Query logs with filters
    pub async fn query_logs(
        &self,
        filters: LogQueryFilters,
    ) -> Vec<DistributedLogEntry> {
        // In a real implementation, this would query from storage
        // For now, return current buffer contents that match filters
        let buffer = self.buffer.read().await;
        
        buffer
            .iter()
            .filter(|log| {
                // Apply filters
                if let Some(ref level) = filters.min_level {
                    if log.level < *level {
                        return false;
                    }
                }

                if let Some(ref node) = filters.node_id {
                    if &log.node_id != node {
                        return false;
                    }
                }

                if let Some(ref corr_id) = filters.correlation_id {
                    if log.correlation_id.as_ref().map(|c| &c.request_id) != Some(corr_id) {
                        return false;
                    }
                }

                if let Some(ref start) = filters.start_time {
                    if log.timestamp < *start {
                        return false;
                    }
                }

                if let Some(ref end) = filters.end_time {
                    if log.timestamp > *end {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }
}

/// Log query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryFilters {
    pub min_level: Option<LogLevel>,
    pub node_id: Option<String>,
    pub correlation_id: Option<String>,
    pub start_time: Option<SystemTime>,
    pub end_time: Option<SystemTime>,
    pub limit: Option<usize>,
}

/// Tracing integration layer
pub struct TracingIntegration {
    logger: Arc<CentralizedLogger>,
}

impl TracingIntegration {
    pub fn new(logger: Arc<CentralizedLogger>) -> Self {
        Self { logger }
    }

    /// Create a tracing span with correlation ID
    pub fn span_with_correlation(&self, _name: &str, correlation_id: &CorrelationId) -> Span {
        // Simplified implementation to avoid tracing macro issues
        tracing::info_span!(
            "correlation_span",
            request_id = %correlation_id.request_id,
        )
    }

    /// Log structured data with correlation
    pub async fn log_structured(
        &self,
        level: LogLevel,
        message: String,
        correlation_id: Option<CorrelationId>,
        fields: HashMap<String, serde_json::Value>,
    ) {
        self.logger
            .log_with_correlation(level, message, correlation_id, fields)
            .await;
    }
}

/// Macros for convenient logging with correlation
#[macro_export]
macro_rules! log_with_correlation {
    ($logger:expr, $level:expr, $correlation:expr, $msg:expr) => {
        $logger.log_with_correlation(
            $level,
            $msg.to_string(),
            Some($correlation.clone()),
            ::std::collections::HashMap::new(),
        ).await
    };
    ($logger:expr, $level:expr, $correlation:expr, $msg:expr, $($field:tt)*) => {
        {
            let mut fields = ::std::collections::HashMap::new();
            $(
                fields.insert(
                    stringify!($field).to_string(),
                    serde_json::to_value(&$field).unwrap_or(serde_json::Value::Null),
                );
            )*
            $logger.log_with_correlation(
                $level,
                $msg.to_string(),
                Some($correlation.clone()),
                fields,
            ).await
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_correlation_id_creation() {
        let correlation = CorrelationId::new("node1".to_string(), "node2".to_string());
        assert_eq!(correlation.origin_node, "node1");
        assert_eq!(correlation.current_node, "node2");
        assert!(correlation.parent_id.is_none());
    }

    #[tokio::test]
    async fn test_correlation_id_child() {
        let parent = CorrelationId::new("node1".to_string(), "node2".to_string());
        let child = parent.child("node3".to_string());
        
        assert_eq!(child.origin_node, "node1");
        assert_eq!(child.current_node, "node3");
        assert_eq!(child.parent_id, Some(parent.request_id));
    }

    #[tokio::test]
    async fn test_centralized_logger() {
        let config = LoggingConfig::default();
        let logger = CentralizedLogger::new(config, "test-node".to_string());
        
        let correlation = logger.create_correlation(None).await;
        assert_eq!(correlation.current_node, "test-node");
        
        logger.log_with_correlation(
            LogLevel::Info,
            "Test message".to_string(),
            Some(correlation.clone()),
            HashMap::new(),
        ).await;
        
        // Verify log was buffered
        let buffer = logger.buffer.read().await;
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0].message, "Test message");
    }

    #[tokio::test]
    async fn test_log_query_filters() {
        let config = LoggingConfig::default();
        let logger = CentralizedLogger::new(config, "test-node".to_string());
        
        // Add some test logs
        for i in 0..5 {
            logger.log_with_correlation(
                LogLevel::Info,
                format!("Test message {}", i),
                None,
                HashMap::new(),
            ).await;
        }
        
        // Query all logs
        let all_logs = logger.query_logs(LogQueryFilters {
            min_level: None,
            node_id: None,
            correlation_id: None,
            start_time: None,
            end_time: None,
            limit: None,
        }).await;
        
        assert_eq!(all_logs.len(), 5);
    }
}