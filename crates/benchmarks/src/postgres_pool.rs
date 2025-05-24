// postgres_pool.rs - PostgreSQL connection pool management and optimization
//
// Purpose: Provides utilities for configuring, benchmarking, and optimizing
// PostgreSQL connection pools for maximum performance under various workloads

use std::sync::Arc;
use std::time::{Duration, Instant};
use futures::{stream, StreamExt};
use tokio::sync::Semaphore;
use indexer_core::Error;
use sqlx::{Executor, PgPool, Pool, Postgres};
use super::{Measurement, BenchmarkReport};
use super::postgres_opt::ConnectionPoolConfig;

/// Statistics collected during connection pool testing
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of connections in the pool
    pub connections: u32,
    
    /// Number of idle connections
    pub idle_connections: u32,
    
    /// Number of active connections
    pub active_connections: u32,
    
    /// Number of pending connection requests
    pub pending_requests: u32,
    
    /// Maximum wait time for a connection (ms)
    pub max_wait_ms: f64,
    
    /// Average wait time for a connection (ms)
    pub avg_wait_ms: f64,
    
    /// Throughput (operations per second)
    pub throughput: f64,
}

/// Connection pool test configuration
#[derive(Debug, Clone)]
pub struct PoolTestConfig {
    /// Pool configuration to test
    pub pool_config: ConnectionPoolConfig,
    
    /// Number of concurrent clients
    pub concurrent_clients: u32,
    
    /// Duration of the test
    pub test_duration: Duration,
    
    /// SQL query to execute
    pub test_query: String,
    
    /// Query iterations per client
    pub iterations_per_client: u32,
    
    /// Whether to use transactions
    pub use_transactions: bool,
    
    /// Transaction isolation level (if using transactions)
    pub isolation_level: Option<String>,
}

impl Default for PoolTestConfig {
    fn default() -> Self {
        Self {
            pool_config: ConnectionPoolConfig::default(),
            concurrent_clients: 10,
            test_duration: Duration::from_secs(30),
            test_query: "SELECT 1".to_string(),
            iterations_per_client: 1000,
            use_transactions: false,
            isolation_level: None,
        }
    }
}

/// Benchmark a connection pool configuration
pub async fn benchmark_connection_pool(
    database_url: &str,
    config: &PoolTestConfig,
) -> Result<(Measurement, PoolStats), Error> {
    // Create the connection pool
    let pool = config.pool_config.create_pool(database_url).await?;
    
    // Create a semaphore to control concurrency
    let semaphore = Arc::new(Semaphore::new(config.concurrent_clients as usize));
    
    // Track stats
    let start_time = Instant::now();
    let operations_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let wait_times = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    
    // Create client tasks
    let mut tasks = Vec::new();
    
    for i in 0..config.concurrent_clients {
        let pool = pool.clone();
        let semaphore = semaphore.clone();
        let operations_counter = operations_counter.clone();
        let wait_times = wait_times.clone();
        let query = config.test_query.clone();
        let use_transactions = config.use_transactions;
        let isolation_level = config.isolation_level.clone();
        let iterations = config.iterations_per_client;
        
        // Stagger client start times slightly to avoid thundering herd
        let delay = Duration::from_millis((i * 10) as u64);
        
        let task = tokio::spawn(async move {
            // Wait before starting
            tokio::time::sleep(delay).await;
            
            // Acquire permit to run
            let _permit = semaphore.acquire().await.unwrap();
            
            for _ in 0..iterations {
                // Record time waiting for a connection
                let connection_start = Instant::now();
                
                // Execute query with or without transaction
                if use_transactions {
                    let mut conn = pool.acquire().await.map_err(|e| {
                        Error::Other(format!("Failed to acquire connection: {}", e))
                    })?;
                    
                    let isolation = match isolation_level.as_deref() {
                        Some("read_uncommitted") => "READ UNCOMMITTED",
                        Some("read_committed") => "READ COMMITTED",
                        Some("repeatable_read") => "REPEATABLE READ",
                        Some("serializable") => "SERIALIZABLE",
                        _ => "READ COMMITTED", // Default
                    };
                    
                    // Start transaction with specified isolation level
                    conn.execute(&format!("BEGIN ISOLATION LEVEL {}", isolation))
                        .await
                        .map_err(|e| Error::Other(format!("Failed to begin transaction: {}", e)))?;
                    
                    // Execute query
                    conn.execute(&query)
                        .await
                        .map_err(|e| Error::Other(format!("Failed to execute query: {}", e)))?;
                    
                    // Commit transaction
                    conn.execute("COMMIT")
                        .await
                        .map_err(|e| Error::Other(format!("Failed to commit transaction: {}", e)))?;
                } else {
                    // Simple query execution
                    pool.execute(&query)
                        .await
                        .map_err(|e| Error::Other(format!("Failed to execute query: {}", e)))?;
                }
                
                // Record connection acquisition time
                let wait_time = connection_start.elapsed();
                wait_times.lock().await.push(wait_time);
                
                // Increment operation counter
                operations_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                // Check if the test duration has elapsed
                if start_time.elapsed() > config.test_duration {
                    break;
                }
            }
            
            Ok::<_, Error>(())
        });
        
        tasks.push(task);
    }
    
    // Wait for the test duration
    tokio::time::sleep(config.test_duration).await;
    
    // Wait for all tasks to complete or timeout after additional 5 seconds
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        futures::future::join_all(tasks),
    ).await;
    
    // Calculate statistics
    let elapsed = start_time.elapsed();
    let operations = operations_counter.load(std::sync::atomic::Ordering::Relaxed);
    let throughput = operations as f64 / elapsed.as_secs_f64();
    
    // Calculate wait time statistics
    let wait_times = wait_times.lock().await;
    let (avg_wait_ms, max_wait_ms) = if !wait_times.is_empty() {
        let total_wait_ms: f64 = wait_times.iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .sum();
        let max_wait_ms = wait_times.iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .fold(0.0, f64::max);
        (total_wait_ms / wait_times.len() as f64, max_wait_ms)
    } else {
        (0.0, 0.0)
    };
    
    // Get pool statistics (this is a best effort as sqlx doesn't expose all pool stats)
    // We'd normally use pool.status() but it's not accessible, so we estimate
    let idle_connections = config.pool_config.min_connections;
    let active_connections = (config.concurrent_clients as u32).min(config.pool_config.max_connections);
    let pending_requests = config.concurrent_clients.saturating_sub(config.pool_config.max_connections);
    
    let pool_stats = PoolStats {
        connections: config.pool_config.max_connections,
        idle_connections,
        active_connections,
        pending_requests,
        max_wait_ms,
        avg_wait_ms,
        throughput,
    };
    
    // Create measurement
    let measurement = Measurement::new(
        &format!(
            "pool_test_clients_{}_max_conn_{}", 
            config.concurrent_clients,
            config.pool_config.max_connections
        ),
        elapsed,
        operations,
        0, // We don't track data size for connection pool tests
    )
    .with_metric("avg_wait_ms", avg_wait_ms)
    .with_metric("max_wait_ms", max_wait_ms)
    .with_metric("throughput", throughput);
    
    // Close the pool
    pool.close().await;
    
    Ok((measurement, pool_stats))
}

/// Find optimal connection pool size for a workload
pub async fn find_optimal_pool_size(
    database_url: &str,
    test_query: &str,
    max_concurrency: u32,
) -> Result<(ConnectionPoolConfig, BenchmarkReport), Error> {
    let mut measurements = Vec::new();
    let mut best_throughput = 0.0;
    let mut optimal_config = ConnectionPoolConfig::default();
    
    // Test different pool sizes
    for &pool_size in &[5, 10, 20, 50, 100] {
        // Skip if pool_size > max_concurrency (no point in having more connections than clients)
        if pool_size > max_concurrency {
            continue;
        }
        
        let min_connections = pool_size / 5; // Rule of thumb for min connections
        
        let pool_config = ConnectionPoolConfig {
            min_connections,
            max_connections: pool_size,
            ..ConnectionPoolConfig::default()
        };
        
        let test_config = PoolTestConfig {
            pool_config,
            concurrent_clients: max_concurrency,
            test_duration: Duration::from_secs(10), // Shorter test for optimization
            test_query: test_query.to_string(),
            iterations_per_client: 1000,
            use_transactions: false,
            isolation_level: None,
        };
        
        // Run benchmark
        let (measurement, stats) = benchmark_connection_pool(database_url, &test_config).await?;
        measurements.push(measurement.clone());
        
        // Check if this configuration is better
        if stats.throughput > best_throughput {
            best_throughput = stats.throughput;
            optimal_config = pool_config;
        }
    }
    
    // Create benchmark report
    let report = BenchmarkReport::new("connection_pool_optimization", measurements);
    
    Ok((optimal_config, report))
}

/// Test different transaction isolation levels
pub async fn benchmark_transaction_isolation(
    database_url: &str,
    pool_config: &ConnectionPoolConfig,
    test_query: &str,
    concurrent_clients: u32,
) -> Result<BenchmarkReport, Error> {
    let mut measurements = Vec::new();
    
    // Test with different isolation levels
    for isolation_level in &[
        "read_uncommitted",
        "read_committed",
        "repeatable_read",
        "serializable",
    ] {
        let test_config = PoolTestConfig {
            pool_config: pool_config.clone(),
            concurrent_clients,
            test_duration: Duration::from_secs(10),
            test_query: test_query.to_string(),
            iterations_per_client: 1000,
            use_transactions: true,
            isolation_level: Some(isolation_level.to_string()),
        };
        
        // Run benchmark
        let (measurement, _) = benchmark_connection_pool(database_url, &test_config).await?;
        
        // Add isolation level to measurement name
        let mut named_measurement = measurement;
        named_measurement.name = format!("isolation_{}", isolation_level);
        
        measurements.push(named_measurement);
    }
    
    // Create benchmark report
    let report = BenchmarkReport::new("transaction_isolation_benchmark", measurements);
    
    Ok(report)
}

/// Stress test a connection pool to find its breaking point
pub async fn stress_test_connection_pool(
    database_url: &str,
    pool_config: &ConnectionPoolConfig,
    test_query: &str,
) -> Result<(u32, BenchmarkReport), Error> {
    let mut measurements = Vec::new();
    let mut max_stable_clients = 0;
    
    // Start with a low number of clients and increase until performance degrades
    for concurrency in &[10, 20, 50, 100, 200, 500] {
        let test_config = PoolTestConfig {
            pool_config: pool_config.clone(),
            concurrent_clients: *concurrency,
            test_duration: Duration::from_secs(10),
            test_query: test_query.to_string(),
            iterations_per_client: 1000,
            use_transactions: false,
            isolation_level: None,
        };
        
        // Run benchmark
        match benchmark_connection_pool(database_url, &test_config).await {
            Ok((measurement, stats)) => {
                // Check if performance is acceptable
                // We consider acceptable if avg_wait_ms < 100
                if stats.avg_wait_ms < 100.0 {
                    max_stable_clients = *concurrency;
                }
                
                measurements.push(measurement);
            },
            Err(_) => {
                // If the test fails, we've reached the breaking point
                break;
            }
        }
    }
    
    // Create benchmark report
    let report = BenchmarkReport::new("connection_pool_stress_test", measurements);
    
    Ok((max_stable_clients, report))
}

/// Create a comprehensive connection pool configuration plan
pub async fn create_pool_configuration_plan(
    database_url: &str,
    typical_queries: &[&str],
    max_expected_concurrency: u32,
) -> Result<ConnectionPoolConfig, Error> {
    // First find the optimal pool size
    let (mut optimal_config, _) = find_optimal_pool_size(
        database_url,
        // Use the first query as a representative workload
        typical_queries.first().unwrap_or(&"SELECT 1"),
        max_expected_concurrency,
    ).await?;
    
    // Now determine the connection timeouts based on workload
    // For high concurrency workloads, we want shorter idle timeouts
    if max_expected_concurrency > 50 {
        optimal_config.idle_timeout = Duration::from_secs(300); // 5 minutes
    } else {
        optimal_config.idle_timeout = Duration::from_secs(600); // 10 minutes
    }
    
    // Set statement timeout based on query complexity
    // Check if any of the queries are complex (contain JOINs or GROUP BY)
    let has_complex_queries = typical_queries.iter().any(|q| {
        let q = q.to_lowercase();
        q.contains("join") || q.contains("group by") || q.contains("order by") || q.contains("having")
    });
    
    if has_complex_queries {
        optimal_config.statement_timeout = Duration::from_secs(60); // 60 seconds
    } else {
        optimal_config.statement_timeout = Duration::from_secs(30); // 30 seconds
    }
    
    Ok(optimal_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // This test requires a real database
    async fn test_benchmark_connection_pool() {
        // This test would require a real database connection
        // For demonstration purposes only
        /*
        let database_url = "postgres://postgres:postgres@localhost/test_db";
        
        let config = PoolTestConfig {
            pool_config: ConnectionPoolConfig::default(),
            concurrent_clients: 10,
            test_duration: Duration::from_secs(5),
            test_query: "SELECT 1".to_string(),
            iterations_per_client: 100,
            use_transactions: false,
            isolation_level: None,
        };
        
        let (measurement, stats) = benchmark_connection_pool(database_url, &config).await.unwrap();
        
        assert!(measurement.operations > 0);
        assert!(stats.throughput > 0.0);
        */
    }
} 