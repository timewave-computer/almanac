use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use super::provider::{
    ConfigProvider, LogLevel, LoggerProvider, MetricsProvider, NetworkProvider, 
    ProviderError, StorageProvider, TimeProvider
};

/// Type alias for log entries
pub type LogEntry = (LogLevel, String, Vec<(String, String)>);

/// Type alias for log storage
pub type LogStorage = Arc<RwLock<Vec<LogEntry>>>;

/// Type alias for metric entries
pub type MetricEntry = (String, MetricValue, Vec<(String, String)>);

/// Type alias for metric storage
pub type MetricStorage = Arc<RwLock<Vec<MetricEntry>>>;

/// Mock storage provider for testing
pub struct MockStorageProvider<S> {
    storage: S,
}

impl<S> MockStorageProvider<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S: Send + Sync + Clone> StorageProvider for MockStorageProvider<S> {
    type Storage = S;
    
    fn provide_storage(&self) -> Result<Self::Storage, ProviderError> {
        Ok(self.storage.clone())
    }
}

/// Mock network provider for testing
pub struct MockNetworkProvider<C> {
    client: C,
}

impl<C> MockNetworkProvider<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }
}

impl<C: Send + Sync + Clone> NetworkProvider for MockNetworkProvider<C> {
    type Client = C;
    
    fn provide_client(&self) -> Result<Self::Client, ProviderError> {
        Ok(self.client.clone())
    }
}

/// Mock time provider for testing
#[derive(Clone)]
pub struct MockTimeProvider {
    fixed_time: Option<SystemTime>,
}

impl MockTimeProvider {
    pub fn new() -> Self {
        Self { fixed_time: None }
    }
    
    pub fn with_fixed_time(time: SystemTime) -> Self {
        Self { fixed_time: Some(time) }
    }
}

impl Default for MockTimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeProvider for MockTimeProvider {
    fn now(&self) -> SystemTime {
        self.fixed_time.unwrap_or_else(SystemTime::now)
    }
    
    fn now_millis(&self) -> u64 {
        self.now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

/// Mock config provider for testing
pub struct MockConfigProvider {
    config: Arc<RwLock<HashMap<String, String>>>,
}

impl MockConfigProvider {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn set(&self, key: &str, value: &str) {
        self.config.write().unwrap().insert(key.to_string(), value.to_string());
    }
}

impl Default for MockConfigProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigProvider for MockConfigProvider {
    fn get_string(&self, key: &str) -> Option<String> {
        self.config.read().unwrap().get(key).cloned()
    }
    
    fn get_int(&self, key: &str) -> Option<i64> {
        self.get_string(key)?.parse().ok()
    }
    
    fn get_bool(&self, key: &str) -> Option<bool> {
        self.get_string(key)?.parse().ok()
    }
}

/// Mock logger provider for testing
pub struct MockLoggerProvider {
    logs: LogStorage,
}

impl MockLoggerProvider {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn logs(&self) -> Vec<LogEntry> {
        self.logs.read().unwrap().clone()
    }
}

impl Default for MockLoggerProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggerProvider for MockLoggerProvider {
    fn log(&self, level: LogLevel, message: &str) {
        self.logs.write().unwrap().push((level, message.to_string(), vec![]));
    }
    
    fn log_with_context(&self, level: LogLevel, message: &str, context: &[(String, String)]) {
        self.logs.write().unwrap().push((level, message.to_string(), context.to_vec()));
    }
}

/// Mock metrics provider for testing
pub struct MockMetricsProvider {
    metrics: MetricStorage,
}

#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(f64),
}

impl MockMetricsProvider {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn metrics(&self) -> Vec<MetricEntry> {
        self.metrics.read().unwrap().clone()
    }
}

impl Default for MockMetricsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsProvider for MockMetricsProvider {
    fn increment(&self, metric: &str, tags: &[(String, String)]) {
        self.metrics.write().unwrap().push((
            metric.to_string(),
            MetricValue::Counter(1),
            tags.to_vec(),
        ));
    }
    
    fn gauge(&self, metric: &str, value: f64, tags: &[(String, String)]) {
        self.metrics.write().unwrap().push((
            metric.to_string(),
            MetricValue::Gauge(value),
            tags.to_vec(),
        ));
    }
    
    fn histogram(&self, metric: &str, value: f64, tags: &[(String, String)]) {
        self.metrics.write().unwrap().push((
            metric.to_string(),
            MetricValue::Histogram(value),
            tags.to_vec(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock_config_provider() {
        let provider = MockConfigProvider::new();
        provider.set("test.key", "value");
        provider.set("test.number", "42");
        provider.set("test.bool", "true");
        
        assert_eq!(provider.get_string("test.key"), Some("value".to_string()));
        assert_eq!(provider.get_int("test.number"), Some(42));
        assert_eq!(provider.get_bool("test.bool"), Some(true));
    }
    
    #[test]
    fn test_mock_logger_provider() {
        let provider = MockLoggerProvider::new();
        provider.log(LogLevel::Info, "test message");
        provider.log_with_context(
            LogLevel::Error,
            "error message",
            &[("key".to_string(), "value".to_string())],
        );
        
        let logs = provider.logs();
        assert_eq!(logs.len(), 2);
        assert!(matches!(logs[0].0, LogLevel::Info));
        assert_eq!(logs[0].1, "test message");
    }
    
    #[test]
    fn test_mock_metrics_provider() {
        let provider = MockMetricsProvider::new();
        provider.increment("test.counter", &[]);
        provider.gauge("test.gauge", 42.5, &[]);
        provider.histogram("test.histogram", 100.0, &[]);
        
        let metrics = provider.metrics();
        assert_eq!(metrics.len(), 3);
        assert!(matches!(metrics[0].1, MetricValue::Counter(1)));
        assert!(matches!(metrics[1].1, MetricValue::Gauge(v) if v == 42.5));
        assert!(matches!(metrics[2].1, MetricValue::Histogram(v) if v == 100.0));
    }
}