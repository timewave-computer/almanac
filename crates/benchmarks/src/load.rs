// load.rs - Load testing implementation for benchmarking
//
// Purpose: Provides specialized load tests for measuring performance under
// various workload conditions and concurrency levels

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;
use governor;
use indexer_storage::{
    postgres::PostgresStorage,
    rocks::RocksStorage,
};
use indexer_core::Error;
use super::{Measurement};
use rand::{Rng, distributions::Alphanumeric};

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// Number of concurrent requests
    pub concurrency: usize,
    
    /// Duration to run the test
    pub duration: Duration,
    
    /// Request rate limit per second (0 means unlimited)
    pub rate_limit: usize,
    
    /// Ramp-up time (gradually increase load)
    pub ramp_up: Duration,
    
    /// Wait time between requests (for controlled pacing)
    pub wait_time: Option<Duration>,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrency: 10,
            duration: Duration::from_secs(30),
            rate_limit: 0,
            ramp_up: Duration::from_secs(5),
            wait_time: None,
        }
    }
}

/// Statistics from a load test
#[derive(Debug, Clone)]
pub struct LoadTestStats {
    /// Total number of requests executed
    pub total_requests: u64,
    
    /// Total number of successful requests
    pub successful_requests: u64,
    
    /// Total number of failed requests
    pub failed_requests: u64,
    
    /// Total bytes processed
    pub total_bytes: u64,
    
    /// Test duration
    pub duration: Duration,
    
    /// Requests per second
    pub requests_per_second: f64,
    
    /// Average response time
    pub avg_response_time: Duration,
    
    /// 95th percentile response time
    pub p95_response_time: Duration,
    
    /// 99th percentile response time
    pub p99_response_time: Duration,
    
    /// Maximum response time
    pub max_response_time: Duration,
    
    /// Minimum response time
    pub min_response_time: Duration,
    
    /// Error rate
    pub error_rate: f64,
}

impl Default for LoadTestStats {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadTestStats {
    /// Create a new load test stats instance
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_bytes: 0,
            duration: Duration::from_secs(0),
            requests_per_second: 0.0,
            avg_response_time: Duration::from_secs(0),
            p95_response_time: Duration::from_secs(0),
            p99_response_time: Duration::from_secs(0),
            max_response_time: Duration::from_secs(0),
            min_response_time: Duration::from_secs(0),
            error_rate: 0.0,
        }
    }
    
    /// Calculate statistics from measurements
    pub fn from_measurements(measurements: &[ResponseTime], duration: Duration) -> Self {
        if measurements.is_empty() {
            return Self::new();
        }
        
        // Count successes and failures
        let total_requests = measurements.len() as u64;
        let successful_requests = measurements.iter().filter(|m| m.success).count() as u64;
        let failed_requests = total_requests - successful_requests;
        
        // Calculate bytes
        let total_bytes = measurements.iter().map(|m| m.bytes).sum();
        
        // Get response times
        let mut response_times: Vec<Duration> = measurements.iter()
            .map(|m| m.duration)
            .collect();
        
        response_times.sort();
        
        let avg_response_time = if !response_times.is_empty() {
            let total_nanos: u128 = response_times.iter()
                .map(|d| d.as_nanos())
                .sum();
            Duration::from_nanos((total_nanos / response_times.len() as u128) as u64)
        } else {
            Duration::from_secs(0)
        };
        
        let p95_index = (response_times.len() as f64 * 0.95) as usize;
        let p99_index = (response_times.len() as f64 * 0.99) as usize;
        
        let p95_response_time = if !response_times.is_empty() && p95_index < response_times.len() {
            response_times[p95_index]
        } else if !response_times.is_empty() {
            response_times[response_times.len() - 1]
        } else {
            Duration::from_secs(0)
        };
        
        let p99_response_time = if !response_times.is_empty() && p99_index < response_times.len() {
            response_times[p99_index]
        } else if !response_times.is_empty() {
            response_times[response_times.len() - 1]
        } else {
            Duration::from_secs(0)
        };
        
        let max_response_time = response_times.last().copied().unwrap_or_else(|| Duration::from_secs(0));
        let min_response_time = response_times.first().copied().unwrap_or_else(|| Duration::from_secs(0));
        
        // Calculate RPS
        let requests_per_second = if duration.as_secs_f64() > 0.0 {
            total_requests as f64 / duration.as_secs_f64()
        } else {
            0.0
        };
        
        // Calculate error rate
        let error_rate = if total_requests > 0 {
            failed_requests as f64 / total_requests as f64
        } else {
            0.0
        };
        
        Self {
            total_requests,
            successful_requests,
            failed_requests,
            total_bytes,
            duration,
            requests_per_second,
            avg_response_time,
            p95_response_time,
            p99_response_time,
            max_response_time,
            min_response_time,
            error_rate,
        }
    }
}

/// A single response time measurement
#[derive(Debug, Clone, Copy)]
pub struct ResponseTime {
    /// Duration of the request
    pub duration: Duration,
    
    /// Whether the request was successful
    pub success: bool,
    
    /// Number of bytes processed
    pub bytes: u64,
}

/// Generate a Measurement from LoadTestStats
impl From<LoadTestStats> for Measurement {
    fn from(stats: LoadTestStats) -> Self {
        let mut measurement = Self::new(
            "load_test",
            stats.duration,
            stats.total_requests,
            stats.total_bytes,
        );
        
        // Add metrics
        measurement = measurement
            .with_metric("requests_per_second", stats.requests_per_second)
            .with_metric("avg_response_time_ms", stats.avg_response_time.as_millis() as f64)
            .with_metric("p95_response_time_ms", stats.p95_response_time.as_millis() as f64)
            .with_metric("p99_response_time_ms", stats.p99_response_time.as_millis() as f64)
            .with_metric("max_response_time_ms", stats.max_response_time.as_millis() as f64)
            .with_metric("min_response_time_ms", stats.min_response_time.as_millis() as f64)
            .with_metric("error_rate", stats.error_rate);
        
        measurement
    }
}

/// A trait for operations that can be load tested
pub trait LoadTestable {
    /// Execute a single operation and return success/failure and bytes processed
    fn execute(&self) -> impl std::future::Future<Output = Result<u64, Error>> + Send;
}

/// Load test a function with the given configuration
pub async fn run_load_test<F, Fut>(
    config: &LoadTestConfig,
    operation: F,
) -> Result<LoadTestStats, Error>
where
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<u64, Error>> + Send + 'static,
{
    // Create a rate limiter if needed
    let rate_limiter = if config.rate_limit > 0 {
        Some(Arc::new(Mutex::new(
            governor::RateLimiter::direct(governor::Quota::per_second(
                std::num::NonZeroU32::new(config.rate_limit as u32).unwrap(),
            )),
        )))
    } else {
        None
    };
    
    // Create a semaphore to limit concurrency
    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    
    // Create a vector to store response times
    let response_times = Arc::new(Mutex::new(Vec::new()));
    
    // Create a flag to indicate when to stop
    let stop = Arc::new(Mutex::new(false));
    
    // Create a set of tasks
    let mut tasks = JoinSet::new();
    
    // Start time
    let start_time = Instant::now();
    
    // Spawn tasks to run the operation
    for i in 0..config.concurrency {
        let operation = operation.clone();
        let semaphore = semaphore.clone();
        let response_times = response_times.clone();
        let rate_limiter = rate_limiter.clone();
        let stop = stop.clone();
        
        // Stagger the start of tasks during ramp-up
        let ramp_up_delay = if config.ramp_up.as_millis() > 0 {
            let delay_fraction = i as f64 / config.concurrency as f64;
            let delay_ms = (config.ramp_up.as_millis() as f64 * delay_fraction) as u64;
            Duration::from_millis(delay_ms)
        } else {
            Duration::from_millis(0)
        };
        
        let wait_time_clone = config.wait_time;
        
        tasks.spawn(async move {
            // Wait for ramp-up delay
            if ramp_up_delay.as_millis() > 0 {
                tokio::time::sleep(ramp_up_delay).await;
            }
            
            loop {
                // Check if we should stop
                if *stop.lock().await {
                    break;
                }
                
                // Acquire a permit from the semaphore
                let _permit = semaphore.acquire().await.unwrap();
                
                // Wait for rate limit if needed
                if let Some(limiter) = &rate_limiter {
                    let limiter = limiter.lock().await;
                    limiter.until_ready().await;
                }
                
                // Measure the operation
                let start = Instant::now();
                let result = operation().await;
                let duration = start.elapsed();
                
                // Record the response time
                let mut response_times = response_times.lock().await;
                response_times.push(ResponseTime {
                    duration,
                    success: result.is_ok(),
                    bytes: result.unwrap_or(0),
                });
                
                // Wait between requests if configured
                if let Some(wait_time) = wait_time_clone {
                    tokio::time::sleep(wait_time).await;
                }
            }
        });
    }
    
    // Wait for the test duration
    tokio::time::sleep(config.duration).await;
    
    // Signal tasks to stop
    {
        let mut stop = stop.lock().await;
        *stop = true;
    }
    
    // Wait for all tasks to complete
    while tasks.join_next().await.is_some() {}
    
    // Calculate stats
    let test_duration = start_time.elapsed();
    let response_times = response_times.lock().await;
    let stats = LoadTestStats::from_measurements(&response_times, test_duration);
    
    Ok(stats)
}

/// Run a RocksDB load test
pub async fn run_rocksdb_load_test(
    rocks_db: Arc<RocksStorage>,
    config: &LoadTestConfig,
    operation: impl Fn(Arc<RocksStorage>) -> Result<u64, Error> + Clone + Send + Sync + 'static,
) -> Result<LoadTestStats, Error> {
    let rocks_db = rocks_db.clone();
    run_load_test(config, move || {
        let op = operation.clone();
        let db = rocks_db.clone();
        async move { op(db) }
    }).await
}

/// Run a PostgreSQL load test
pub async fn run_postgres_load_test(
    postgres: Arc<PostgresStorage>,
    config: &LoadTestConfig,
    operation: impl Fn(Arc<PostgresStorage>) -> Result<u64, Error> + Clone + Send + Sync + 'static,
) -> Result<LoadTestStats, Error> {
    let postgres = postgres.clone();
    run_load_test(config, move || {
        let op = operation.clone();
        let db = postgres.clone();
        async move { op(db) }
    }).await
}

/// Standard RocksDB benchmark operations
pub mod rocksdb_benchmarks {
    use super::*;
    use rand::{Rng, distributions::Alphanumeric};
    
    /// Generate a random key
    pub fn random_key(prefix: &str, len: usize) -> String {
        let random_part: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect();
        
        format!("{}:{}", prefix, random_part)
    }
    
    /// Generate random data
    pub fn random_data(size: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut data = Vec::with_capacity(size);
        for _ in 0..size {
            data.push(rng.gen::<u8>());
        }
        data
    }
    
    /// Benchmark RocksDB writes
    pub fn write_benchmark(
        _rocks_db: Arc<RocksStorage>,
        key_prefix: &'static str,
        key_len: usize,
        value_size: usize,
    ) -> impl Fn(Arc<RocksStorage>) -> Result<u64, Error> + Clone {
        move |db: Arc<RocksStorage>| {
            let key = random_key(key_prefix, key_len);
            let value = random_data(value_size);
            
            let key_obj = indexer_storage::rocks::Key::new("benchmark", &key);
            db.put(&key_obj, &value)?;
            
            Ok(key.len() as u64 + value.len() as u64)
        }
    }
    
    /// Benchmark RocksDB reads
    pub fn read_benchmark(
        _rocks_db: Arc<RocksStorage>,
        keys: Arc<Vec<String>>,
    ) -> impl Fn(Arc<RocksStorage>) -> Result<u64, Error> + Clone {
        move |db: Arc<RocksStorage>| {
            // Select a random key
            let mut rng = rand::thread_rng();
            let key_index = rng.gen_range(0..keys.len());
            let key = &keys[key_index];
            
            // Read the value
            let key_obj = indexer_storage::rocks::Key::new("benchmark", key);
            let result = db.get(&key_obj)?;
            let bytes = match &result {
                Some(value) => key.len() as u64 + value.len() as u64,
                None => key.len() as u64,
            };
            
            Ok(bytes)
        }
    }
    
    /// Benchmark RocksDB scans
    pub fn scan_benchmark(
        _rocks_db: Arc<RocksStorage>,
        prefix: &'static str,
        limit: usize,
    ) -> impl Fn(Arc<RocksStorage>) -> Result<u64, Error> + Clone {
        move |db: Arc<RocksStorage>| {
            let mut total_bytes = 0u64;
            
            let prefix_bytes = indexer_storage::rocks::Key::prefix(prefix);
            let results = db.scan_prefix(&prefix_bytes)?;
            
            for (i, (key, value)) in results.iter().enumerate() {
                if i >= limit {
                    break;
                }
                
                total_bytes += key.len() as u64 + value.len() as u64;
            }
            
            Ok(total_bytes)
        }
    }
}

/// Standard PostgreSQL benchmark operations
pub mod postgres_benchmarks {
    use super::*;
    
    /// Benchmark entity for testing
    #[derive(Debug, Clone)]
    pub struct BenchmarkEntity {
        pub id: String,
        pub chain_id: String,
        pub block_height: u64,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub data: String,
    }
    
    /// Generate a random entity
    pub fn random_entity() -> BenchmarkEntity {
        let random_id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        
        BenchmarkEntity {
            id: random_id,
            chain_id: "ethereum".to_string(),
            block_height: rand::thread_rng().gen_range(1..10_000_000),
            timestamp: chrono::Utc::now(),
            data: rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(100)
                .map(char::from)
                .collect(),
        }
    }
    
    /// Simulate a PostgreSQL insert operation
    pub fn insert_benchmark(
        _postgres: Arc<PostgresStorage>,
    ) -> impl Fn(Arc<PostgresStorage>) -> Result<u64, Error> + Clone {
        move |_db: Arc<PostgresStorage>| {
            // For now, this is a placeholder. In the actual implementation,
            // we would use PostgresRepository::insert_entity or similar
            let entity = random_entity();
            let entity_size = entity.id.len() + entity.chain_id.len() + 8 + 8 + entity.data.len();
            
            // Simulate insert
            Ok(entity_size as u64)
        }
    }
    
    /// Simulate a PostgreSQL query operation
    pub fn query_benchmark(
        _postgres: Arc<PostgresStorage>,
    ) -> impl Fn(Arc<PostgresStorage>) -> Result<u64, Error> + Clone {
        move |_db: Arc<PostgresStorage>| {
            // For now, this is a placeholder. In the actual implementation,
            // we would use PostgresRepository::query_entities or similar
            
            // Simulate query result size
            let result_size = 1024;
            
            Ok(result_size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_load_test_stats_calculation() {
        // Create some response times
        let response_times = vec![
            ResponseTime { duration: Duration::from_millis(10), success: true, bytes: 100 },
            ResponseTime { duration: Duration::from_millis(20), success: true, bytes: 200 },
            ResponseTime { duration: Duration::from_millis(30), success: false, bytes: 0 },
            ResponseTime { duration: Duration::from_millis(40), success: true, bytes: 300 },
            ResponseTime { duration: Duration::from_millis(50), success: true, bytes: 400 },
        ];
        
        // Calculate stats
        let stats = LoadTestStats::from_measurements(&response_times, Duration::from_secs(1));
        
        // Verify stats
        assert_eq!(stats.total_requests, 5);
        assert_eq!(stats.successful_requests, 4);
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.total_bytes, 1000);
        assert_eq!(stats.duration, Duration::from_secs(1));
        assert_eq!(stats.requests_per_second, 5.0);
        assert_eq!(stats.avg_response_time, Duration::from_millis(30));
        assert_eq!(stats.p95_response_time, Duration::from_millis(50)); // 95th percentile
        assert_eq!(stats.p99_response_time, Duration::from_millis(50)); // 99th percentile
        assert_eq!(stats.max_response_time, Duration::from_millis(50));
        assert_eq!(stats.min_response_time, Duration::from_millis(10));
        assert_eq!(stats.error_rate, 0.2); // 1 out of 5 failed
    }
    
    #[tokio::test]
    async fn test_run_load_test() {
        // Create a load test configuration
        let config = LoadTestConfig {
            concurrency: 5,
            duration: Duration::from_millis(100),
            rate_limit: 0,
            ramp_up: Duration::from_millis(0),
            wait_time: None,
        };
        
        // Run a load test with a simple operation
        let stats = run_load_test(&config, || async {
            // Simulate some work
            tokio::time::sleep(Duration::from_millis(5)).await;
            Ok(100)
        }).await.unwrap();
        
        // Verify the stats
        assert!(stats.total_requests > 0);
        assert_eq!(stats.successful_requests, stats.total_requests);
        assert_eq!(stats.failed_requests, 0);
        assert_eq!(stats.total_bytes, 100 * stats.total_requests);
        assert!(stats.duration.as_millis() >= 100);
        assert!(stats.requests_per_second > 0.0);
        assert!(stats.avg_response_time.as_millis() >= 5);
        assert_eq!(stats.error_rate, 0.0);
    }
} 