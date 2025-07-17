// Comprehensive Prometheus metrics system for cross-chain indexer monitoring
//
// This module provides metrics collection for all major components including
// event processing, storage operations, API requests, and system health.

use almanac_infrastructure_core::{Result, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

/// Metric types supported by the metrics system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    /// Counter that can only increase
    Counter,
    /// Gauge that can increase or decrease
    Gauge,
    /// Histogram for duration measurements
    Histogram,
    /// Summary for percentile calculations
    Summary,
}

/// Metric value with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    /// Metric value
    pub value: f64,
    /// Timestamp when metric was recorded
    pub timestamp: SystemTime,
    /// Optional labels for the metric
    pub labels: HashMap<String, String>,
}

/// Histogram bucket definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    /// Upper bound for this bucket
    pub upper_bound: f64,
    /// Count of observations in this bucket
    pub count: u64,
}

/// Histogram metric data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramMetric {
    /// Histogram buckets
    pub buckets: Vec<HistogramBucket>,
    /// Total count of observations
    pub count: u64,
    /// Sum of all observed values
    pub sum: f64,
}

/// Summary metric data with percentiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryMetric {
    /// Percentile values
    pub quantiles: Vec<(f64, f64)>, // percentile -> value pairs
    /// Total count of observations
    pub count: u64,
    /// Sum of all observed values
    pub sum: f64,
}

/// Generic metric data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricData {
    Counter(f64),
    Gauge(f64),
    Histogram(HistogramMetric),
    Summary(SummaryMetric),
}

/// Metric definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Help text describing the metric
    pub help: String,
    /// Current metric data
    pub data: MetricData,
    /// Labels attached to this metric
    pub labels: HashMap<String, String>,
    /// Last update timestamp
    pub last_updated: SystemTime,
}

/// Metrics registry for managing all application metrics
#[derive(Debug)]
pub struct MetricsRegistry {
    /// All registered metrics
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
    /// Metric name prefix
    prefix: String,
    /// Default labels applied to all metrics
    default_labels: HashMap<String, String>,
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new(prefix: String) -> Self {
        let mut default_labels = HashMap::new();
        default_labels.insert("service".to_string(), "almanac".to_string());
        default_labels.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            prefix,
            default_labels,
        }
    }
    
    /// Register a new counter metric
    pub async fn register_counter(&self, name: &str, help: &str, labels: Option<HashMap<String, String>>) -> Result<()> {
        let full_name = format!("{}_{}", self.prefix, name);
        let mut all_labels = self.default_labels.clone();
        if let Some(labels) = labels {
            all_labels.extend(labels);
        }
        
        let metric = Metric {
            name: full_name.clone(),
            metric_type: MetricType::Counter,
            help: help.to_string(),
            data: MetricData::Counter(0.0),
            labels: all_labels,
            last_updated: SystemTime::now(),
        };
        
        let mut metrics = self.metrics.write().await;
        metrics.insert(full_name, metric);
        Ok(())
    }
    
    /// Register a new gauge metric
    pub async fn register_gauge(&self, name: &str, help: &str, labels: Option<HashMap<String, String>>) -> Result<()> {
        let full_name = format!("{}_{}", self.prefix, name);
        let mut all_labels = self.default_labels.clone();
        if let Some(labels) = labels {
            all_labels.extend(labels);
        }
        
        let metric = Metric {
            name: full_name.clone(),
            metric_type: MetricType::Gauge,
            help: help.to_string(),
            data: MetricData::Gauge(0.0),
            labels: all_labels,
            last_updated: SystemTime::now(),
        };
        
        let mut metrics = self.metrics.write().await;
        metrics.insert(full_name, metric);
        Ok(())
    }
    
    /// Register a new histogram metric
    pub async fn register_histogram(&self, name: &str, help: &str, buckets: Vec<f64>, labels: Option<HashMap<String, String>>) -> Result<()> {
        let full_name = format!("{}_{}", self.prefix, name);
        let mut all_labels = self.default_labels.clone();
        if let Some(labels) = labels {
            all_labels.extend(labels);
        }
        
        let histogram_buckets = buckets.into_iter()
            .map(|upper_bound| HistogramBucket { upper_bound, count: 0 })
            .collect();
        
        let metric = Metric {
            name: full_name.clone(),
            metric_type: MetricType::Histogram,
            help: help.to_string(),
            data: MetricData::Histogram(HistogramMetric {
                buckets: histogram_buckets,
                count: 0,
                sum: 0.0,
            }),
            labels: all_labels,
            last_updated: SystemTime::now(),
        };
        
        let mut metrics = self.metrics.write().await;
        metrics.insert(full_name, metric);
        Ok(())
    }
    
    /// Increment a counter metric
    pub async fn increment_counter(&self, name: &str, value: f64) -> Result<()> {
        let full_name = format!("{}_{}", self.prefix, name);
        let mut metrics = self.metrics.write().await;
        
        if let Some(metric) = metrics.get_mut(&full_name) {
            if let MetricData::Counter(ref mut current_value) = metric.data {
                *current_value += value;
                metric.last_updated = SystemTime::now();
            } else {
                return Err(Error::generic(format!("Metric {} is not a counter", name)));
            }
        } else {
            return Err(Error::generic(format!("Counter metric {} not found", name)));
        }
        
        Ok(())
    }
    
    /// Set a gauge metric value
    pub async fn set_gauge(&self, name: &str, value: f64) -> Result<()> {
        let full_name = format!("{}_{}", self.prefix, name);
        let mut metrics = self.metrics.write().await;
        
        if let Some(metric) = metrics.get_mut(&full_name) {
            if let MetricData::Gauge(ref mut current_value) = metric.data {
                *current_value = value;
                metric.last_updated = SystemTime::now();
            } else {
                return Err(Error::generic(format!("Metric {} is not a gauge", name)));
            }
        } else {
            return Err(Error::generic(format!("Gauge metric {} not found", name)));
        }
        
        Ok(())
    }
    
    /// Record a histogram observation
    pub async fn observe_histogram(&self, name: &str, value: f64) -> Result<()> {
        let full_name = format!("{}_{}", self.prefix, name);
        let mut metrics = self.metrics.write().await;
        
        if let Some(metric) = metrics.get_mut(&full_name) {
            if let MetricData::Histogram(ref mut histogram) = metric.data {
                histogram.count += 1;
                histogram.sum += value;
                
                // Update buckets
                for bucket in &mut histogram.buckets {
                    if value <= bucket.upper_bound {
                        bucket.count += 1;
                    }
                }
                
                metric.last_updated = SystemTime::now();
            } else {
                return Err(Error::generic(format!("Metric {} is not a histogram", name)));
            }
        } else {
            return Err(Error::generic(format!("Histogram metric {} not found", name)));
        }
        
        Ok(())
    }
    
    /// Get all metrics for export
    pub async fn get_all_metrics(&self) -> HashMap<String, Metric> {
        self.metrics.read().await.clone()
    }
    
    /// Get a specific metric
    pub async fn get_metric(&self, name: &str) -> Option<Metric> {
        let full_name = format!("{}_{}", self.prefix, name);
        self.metrics.read().await.get(&full_name).cloned()
    }
    
    /// Generate Prometheus format output
    pub async fn export_prometheus(&self) -> String {
        let metrics = self.metrics.read().await;
        let mut output = String::new();
        
        for metric in metrics.values() {
            // Add help text
            output.push_str(&format!("# HELP {} {}\n", metric.name, metric.help));
            
            // Add type
            let type_str = match metric.metric_type {
                MetricType::Counter => "counter",
                MetricType::Gauge => "gauge",
                MetricType::Histogram => "histogram",
                MetricType::Summary => "summary",
            };
            output.push_str(&format!("# TYPE {} {}\n", metric.name, type_str));
            
            // Format labels
            let labels_str = if metric.labels.is_empty() {
                String::new()
            } else {
                let labels: Vec<String> = metric.labels.iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                format!("{{{}}}", labels.join(","))
            };
            
            // Add metric data
            match &metric.data {
                MetricData::Counter(value) | MetricData::Gauge(value) => {
                    output.push_str(&format!("{}{} {}\n", metric.name, labels_str, value));
                }
                MetricData::Histogram(histogram) => {
                    // Export buckets
                    for bucket in &histogram.buckets {
                        let mut bucket_labels = metric.labels.clone();
                        bucket_labels.insert("le".to_string(), bucket.upper_bound.to_string());
                        let bucket_labels_str = format!("{{{}}}", 
                            bucket_labels.iter()
                                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                        output.push_str(&format!("{}_bucket{} {}\n", metric.name, bucket_labels_str, bucket.count));
                    }
                    
                    // Export count and sum
                    output.push_str(&format!("{}_count{} {}\n", metric.name, labels_str, histogram.count));
                    output.push_str(&format!("{}_sum{} {}\n", metric.name, labels_str, histogram.sum));
                }
                MetricData::Summary(summary) => {
                    // Export quantiles
                    for (quantile, value) in &summary.quantiles {
                        let mut quantile_labels = metric.labels.clone();
                        quantile_labels.insert("quantile".to_string(), quantile.to_string());
                        let quantile_labels_str = format!("{{{}}}", 
                            quantile_labels.iter()
                                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                        output.push_str(&format!("{}{} {}\n", metric.name, quantile_labels_str, value));
                    }
                    
                    // Export count and sum
                    output.push_str(&format!("{}_count{} {}\n", metric.name, labels_str, summary.count));
                    output.push_str(&format!("{}_sum{} {}\n", metric.name, labels_str, summary.sum));
                }
            }
            
            output.push('\n');
        }
        
        output
    }
}

/// Pre-defined metrics for common operations
pub struct AlmanacMetrics {
    /// Main metrics registry
    pub registry: MetricsRegistry,
}

impl AlmanacMetrics {
    /// Create new Almanac metrics with all standard metrics registered
    pub async fn new() -> Result<Self> {
        let registry = MetricsRegistry::new("almanac".to_string());
        
        // Event processing metrics
        registry.register_counter("events_processed_total", "Total number of events processed", None).await?;
        registry.register_counter("events_failed_total", "Total number of events that failed processing", None).await?;
        registry.register_histogram("event_processing_duration_seconds", "Time spent processing events", 
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0], None).await?;
        
        // Queue metrics
        registry.register_gauge("queue_depth", "Current depth of event queues", None).await?;
        registry.register_counter("queue_items_added_total", "Total items added to queues", None).await?;
        registry.register_counter("queue_items_processed_total", "Total items processed from queues", None).await?;
        
        // Batch processing metrics
        registry.register_histogram("batch_size", "Size of processed batches", 
            vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0], None).await?;
        registry.register_histogram("batch_processing_duration_seconds", "Time spent processing batches", 
            vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0], None).await?;
        
        // Storage metrics
        registry.register_counter("storage_operations_total", "Total storage operations", None).await?;
        registry.register_counter("storage_errors_total", "Total storage errors", None).await?;
        registry.register_histogram("storage_operation_duration_seconds", "Duration of storage operations", 
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0], None).await?;
        
        // API metrics
        registry.register_counter("http_requests_total", "Total HTTP requests", None).await?;
        registry.register_histogram("http_request_duration_seconds", "HTTP request durations", 
            vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0], None).await?;
        registry.register_histogram("api_query_duration_seconds", "API query durations", 
            vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0], None).await?;
        
        // Health metrics
        registry.register_gauge("component_health_status", "Health status of components (1=healthy, 0=unhealthy)", None).await?;
        registry.register_counter("health_checks_total", "Total health checks performed", None).await?;
        registry.register_counter("component_restarts_total", "Total component restarts", None).await?;
        
        // System metrics
        registry.register_gauge("memory_usage_bytes", "Current memory usage in bytes", None).await?;
        registry.register_gauge("cpu_usage_percent", "Current CPU usage percentage", None).await?;
        registry.register_gauge("active_connections", "Number of active connections", None).await?;
        registry.register_gauge("goroutines_count", "Number of active goroutines", None).await?;
        
        // Chain-specific metrics
        registry.register_counter("chain_events_processed_total", "Events processed per chain", None).await?;
        registry.register_gauge("chain_latest_block", "Latest processed block per chain", None).await?;
        registry.register_counter("chain_reorg_events_total", "Reorg events per chain", None).await?;
        
        Ok(Self { registry })
    }
    
    /// Record event processing
    pub async fn record_event_processed(&self, chain: &str, duration: Duration, success: bool) -> Result<()> {
        let mut labels = HashMap::new();
        labels.insert("chain".to_string(), chain.to_string());
        labels.insert("status".to_string(), if success { "success" } else { "error" }.to_string());
        
        // Update registry with labels for this specific call
        if success {
            self.registry.increment_counter("events_processed_total", 1.0).await?;
        } else {
            self.registry.increment_counter("events_failed_total", 1.0).await?;
        }
        
        self.registry.observe_histogram("event_processing_duration_seconds", duration.as_secs_f64()).await?;
        
        Ok(())
    }
    
    /// Record batch processing
    pub async fn record_batch_processed(&self, batch_size: usize, duration: Duration) -> Result<()> {
        self.registry.observe_histogram("batch_size", batch_size as f64).await?;
        self.registry.observe_histogram("batch_processing_duration_seconds", duration.as_secs_f64()).await?;
        Ok(())
    }
    
    /// Update queue depth
    pub async fn update_queue_depth(&self, depth: usize) -> Result<()> {
        self.registry.set_gauge("queue_depth", depth as f64).await
    }
    
    /// Record storage operation
    pub async fn record_storage_operation(&self, operation: &str, duration: Duration, success: bool) -> Result<()> {
        let mut labels = HashMap::new();
        labels.insert("operation".to_string(), operation.to_string());
        labels.insert("status".to_string(), if success { "success" } else { "error" }.to_string());
        
        self.registry.increment_counter("storage_operations_total", 1.0).await?;
        if !success {
            self.registry.increment_counter("storage_errors_total", 1.0).await?;
        }
        self.registry.observe_histogram("storage_operation_duration_seconds", duration.as_secs_f64()).await?;
        
        Ok(())
    }
    
    /// Record HTTP request
    pub async fn record_http_request(&self, method: &str, path: &str, status_code: u16, duration: Duration) -> Result<()> {
        let mut labels = HashMap::new();
        labels.insert("method".to_string(), method.to_string());
        labels.insert("path".to_string(), path.to_string());
        labels.insert("status_code".to_string(), status_code.to_string());
        
        self.registry.increment_counter("http_requests_total", 1.0).await?;
        self.registry.observe_histogram("http_request_duration_seconds", duration.as_secs_f64()).await?;
        
        Ok(())
    }
    
    /// Update component health status
    pub async fn update_component_health(&self, component: &str, healthy: bool) -> Result<()> {
        let mut labels = HashMap::new();
        labels.insert("component".to_string(), component.to_string());
        
        self.registry.set_gauge("component_health_status", if healthy { 1.0 } else { 0.0 }).await
    }
    
    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> String {
        self.registry.export_prometheus().await
    }
}

/// Timer utility for measuring operation durations
pub struct Timer {
    start: std::time::Instant,
}

impl Timer {
    /// Start a new timer
    pub fn start() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
    
    /// Get elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    
    /// Stop timer and return duration
    pub fn stop(self) -> Duration {
        self.elapsed()
    }
}

/// Macro for timing operations
#[macro_export]
macro_rules! time_operation {
    ($metrics:expr, $metric_name:expr, $operation:expr) => {{
        let timer = $crate::metrics::Timer::start();
        let result = $operation;
        let duration = timer.stop();
        let _ = $metrics.registry.observe_histogram($metric_name, duration.as_secs_f64()).await;
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_metrics_registry_creation() {
        let registry = MetricsRegistry::new("test".to_string());
        
        // Register a counter
        registry.register_counter("test_counter", "A test counter", None).await.unwrap();
        
        // Increment the counter
        registry.increment_counter("test_counter", 1.0).await.unwrap();
        
        // Check the metric value
        let metric = registry.get_metric("test_counter").await.unwrap();
        match metric.data {
            MetricData::Counter(value) => assert_eq!(value, 1.0),
            _ => panic!("Expected counter metric"),
        }
    }
    
    #[tokio::test]
    async fn test_histogram_metric() {
        let registry = MetricsRegistry::new("test".to_string());
        
        // Register a histogram
        registry.register_histogram("test_histogram", "A test histogram", vec![0.1, 0.5, 1.0], None).await.unwrap();
        
        // Record some observations
        registry.observe_histogram("test_histogram", 0.05).await.unwrap();
        registry.observe_histogram("test_histogram", 0.3).await.unwrap();
        registry.observe_histogram("test_histogram", 0.8).await.unwrap();
        
        // Check the metric
        let metric = registry.get_metric("test_histogram").await.unwrap();
        match metric.data {
            MetricData::Histogram(histogram) => {
                assert_eq!(histogram.count, 3);
                assert_eq!(histogram.sum, 1.15);
                assert_eq!(histogram.buckets[0].count, 1); // 0.05 <= 0.1
                assert_eq!(histogram.buckets[1].count, 2); // 0.05, 0.3 <= 0.5
                assert_eq!(histogram.buckets[2].count, 3); // all <= 1.0
            }
            _ => panic!("Expected histogram metric"),
        }
    }
    
    #[tokio::test]
    async fn test_prometheus_export() {
        let registry = MetricsRegistry::new("test".to_string());
        
        registry.register_counter("requests", "Number of requests", None).await.unwrap();
        registry.increment_counter("requests", 42.0).await.unwrap();
        
        let prometheus_output = registry.export_prometheus().await;
        
        assert!(prometheus_output.contains("# HELP test_requests Number of requests"));
        assert!(prometheus_output.contains("# TYPE test_requests counter"));
        assert!(prometheus_output.contains("test_requests"));
        assert!(prometheus_output.contains("42"));
    }
    
    #[tokio::test]
    async fn test_almanac_metrics() {
        let metrics = AlmanacMetrics::new().await.unwrap();
        
        // Test event processing metric
        metrics.record_event_processed("ethereum", Duration::from_millis(100), true).await.unwrap();
        
        // Test batch processing metric
        metrics.record_batch_processed(25, Duration::from_millis(500)).await.unwrap();
        
        // Test queue depth
        metrics.update_queue_depth(100).await.unwrap();
        
        // Export to Prometheus format
        let output = metrics.export_prometheus().await;
        assert!(!output.is_empty());
        assert!(output.contains("almanac_events_processed_total"));
    }
    
    #[tokio::test]
    async fn test_timer() {
        let timer = Timer::start();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let elapsed = timer.stop();
        
        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(100)); // Should be much less than 100ms
    }
} 