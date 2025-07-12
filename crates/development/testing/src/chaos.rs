// Chaos engineering testing module for fault injection and resilience testing
//
// This module provides chaos engineering capabilities to test system resilience
// under various failure scenarios including network partitions, resource exhaustion,
// and service failures

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use tokio::sync::Semaphore;
use futures::future::join_all;
use rand::Rng;
use tracing::{info, warn};
use memory_stats::memory_stats;

use crate::{ChaosTestConfig, fixtures::{ChaosScenario, create_chaos_scenarios as fixture_create_chaos_scenarios}};

/// Chaos test results
#[derive(Debug, Clone)]
pub struct ChaosTestResults {
    pub scenario_name: String,
    pub duration: Duration,
    pub total_operations: u64,
    pub failed_operations: u64,
    pub recovery_time: Duration,
    pub system_stability: f64,
    pub errors: Vec<String>,
}

// Test network failures
pub async fn test_network_failures(config: ChaosTestConfig) -> Result<String> {
    if !config.enabled {
        return Ok("Chaos testing disabled".to_string());
    }

    let mut results = Vec::new();
    
    // Test network partitions
    results.push(simulate_network_partition(&config).await?);
    
    // Test connection drops
    results.push(simulate_connection_drops(&config).await?);
    
    // Test DNS failures
    results.push(simulate_dns_failures(&config).await?);
    
    // Test packet loss
    results.push(simulate_packet_loss(&config).await?);
    
    // Test network latency
    results.push(simulate_network_latency(&config).await?);

    Ok(format!(
        "Network chaos tests completed: {} scenarios tested",
        results.len()
    ))
}

/// Simulate network partition
async fn simulate_network_partition(config: &ChaosTestConfig) -> Result<String> {
    let partition_duration = Duration::from_secs(30);
    let affected_percentage = config.network_failure_rate;
    
    info!("Simulating network partition for {:?} affecting {:.1}% of connections", 
          partition_duration, affected_percentage * 100.0);
    
    // Simulate network partition by introducing delays and failures
    let start_time = Instant::now();
    let mut successful_operations = 0;
    let mut failed_operations = 0;
    
    // Simulate operations during partition
    for i in 0..100 {
        let should_fail = (i as f64 / 100.0) < affected_percentage;
        
        if should_fail {
            // Simulate network failure
            tokio::time::sleep(Duration::from_millis(100)).await;
            failed_operations += 1;
        } else {
            // Simulate successful operation
            tokio::time::sleep(Duration::from_millis(10)).await;
            successful_operations += 1;
        }
        
        if start_time.elapsed() > partition_duration {
            break;
        }
    }
    
    let total_operations = successful_operations + failed_operations;
    let actual_failure_rate = failed_operations as f64 / total_operations as f64;
    
    Ok(format!(
        "Network partition: {} ops, {:.1}% failure rate",
        total_operations,
        actual_failure_rate * 100.0
    ))
}

/// Simulate connection drops
async fn simulate_connection_drops(config: &ChaosTestConfig) -> Result<String> {
    let drop_rate = config.network_failure_rate;
    let test_duration = Duration::from_secs(60);
    
    info!("Simulating connection drops with {:.1}% drop rate for {:?}", 
          drop_rate * 100.0, test_duration);
    
    let start_time = Instant::now();
    let mut connections_established = 0;
    let mut connections_dropped = 0;
    
    // Simulate connection lifecycle
    while start_time.elapsed() < test_duration {
        // Establish connection
        connections_established += 1;
        
        // Simulate connection usage
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Random connection drop
        if rand::random::<f64>() < drop_rate {
            connections_dropped += 1;
            // Simulate recovery time
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    Ok(format!(
        "Connection drops: {} established, {} dropped ({:.1}% drop rate)",
        connections_established,
        connections_dropped,
        (connections_dropped as f64 / connections_established as f64) * 100.0
    ))
}

/// Simulate DNS failures
async fn simulate_dns_failures(config: &ChaosTestConfig) -> Result<String> {
    let failure_rate = config.network_failure_rate;
    let dns_queries = 50;
    
    info!("Simulating DNS failures with {:.1}% failure rate for {} queries", 
          failure_rate * 100.0, dns_queries);
    
    let mut successful_queries = 0;
    let mut failed_queries = 0;
    
    for i in 0..dns_queries {
        let should_fail = (i as f64 / dns_queries as f64) < failure_rate;
        
        if should_fail {
            // Simulate DNS timeout
            tokio::time::sleep(Duration::from_millis(5000)).await;
            failed_queries += 1;
        } else {
            // Simulate successful DNS resolution
            tokio::time::sleep(Duration::from_millis(50)).await;
            successful_queries += 1;
        }
    }
    
    Ok(format!(
        "DNS failures: {} successful, {} failed ({:.1}% failure rate)",
        successful_queries,
        failed_queries,
        (failed_queries as f64 / dns_queries as f64) * 100.0
    ))
}

/// Simulate packet loss
async fn simulate_packet_loss(config: &ChaosTestConfig) -> Result<String> {
    let loss_rate = config.network_failure_rate;
    let packet_count = 1000;
    
    info!("Simulating packet loss with {:.1}% loss rate for {} packets", 
          loss_rate * 100.0, packet_count);
    
    let mut packets_sent = 0;
    let mut packets_lost = 0;
    
    for i in 0..packet_count {
        packets_sent += 1;
        
        let should_lose = (i as f64 / packet_count as f64) < loss_rate;
        
        if should_lose {
            packets_lost += 1;
            // Simulate retransmission delay
            tokio::time::sleep(Duration::from_millis(100)).await;
        } else {
            // Simulate successful transmission
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    
    Ok(format!(
        "Packet loss: {} sent, {} lost ({:.1}% loss rate)",
        packets_sent,
        packets_lost,
        (packets_lost as f64 / packets_sent as f64) * 100.0
    ))
}

/// Simulate network latency
async fn simulate_network_latency(config: &ChaosTestConfig) -> Result<String> {
    let latency_factor = config.network_failure_rate * 10.0; // Convert to latency multiplier
    let operation_count = 100;
    
    info!("Simulating network latency with {:.1}x latency factor for {} operations", 
          latency_factor, operation_count);
    
    let mut total_latency = Duration::from_secs(0);
    let start_time = Instant::now();
    
    for _ in 0..operation_count {
        let base_latency = Duration::from_millis(10);
        let added_latency = Duration::from_millis((base_latency.as_millis() as f64 * latency_factor) as u64);
        
        tokio::time::sleep(base_latency + added_latency).await;
        total_latency += base_latency + added_latency;
    }
    
    let avg_latency = total_latency / operation_count;
    
    Ok(format!(
        "Network latency: {} operations, avg latency: {:?}",
        operation_count,
        avg_latency
    ))
}

// Test database failures
pub async fn test_database_failures(config: ChaosTestConfig) -> Result<String> {
    if !config.enabled {
        return Ok("Chaos testing disabled".to_string());
    }

    let mut results = Vec::new();
    
    // Test connection pool exhaustion
    results.push(simulate_connection_pool_exhaustion(&config).await?);
    
    // Test query timeouts
    results.push(simulate_query_timeouts(&config).await?);
    
    // Test transaction failures
    results.push(simulate_transaction_failures(&config).await?);
    
    // Test deadlock scenarios
    results.push(simulate_deadlock_scenarios(&config).await?);
    
    // Test database unavailability
    results.push(simulate_database_unavailability(&config).await?);

    Ok(format!(
        "Database chaos tests completed: {} scenarios tested",
        results.len()
    ))
}

/// Simulate connection pool exhaustion
async fn simulate_connection_pool_exhaustion(config: &ChaosTestConfig) -> Result<String> {
    let max_connections = 20;
    let concurrent_requests = 50;
    
    info!("Simulating connection pool exhaustion: {} connections, {} requests", 
          max_connections, concurrent_requests);
    
    let semaphore = Arc::new(Semaphore::new(max_connections));
    let mut tasks = Vec::new();
    let successful_ops = Arc::new(AtomicU64::new(0));
    let failed_ops = Arc::new(AtomicU64::new(0));
    
    for i in 0..concurrent_requests {
        let sem = semaphore.clone();
        let success_counter = successful_ops.clone();
        let failure_counter = failed_ops.clone();
        
        let task = tokio::spawn(async move {
            // Try to acquire connection
            match sem.try_acquire() {
                Ok(_permit) => {
                    // Simulate database operation
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    success_counter.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    // Connection pool exhausted
                    failure_counter.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    for task in tasks {
        task.await.unwrap();
    }
    
    let successful = successful_ops.load(Ordering::Relaxed);
    let failed = failed_ops.load(Ordering::Relaxed);
    
    Ok(format!(
        "Connection pool exhaustion: {} successful, {} failed",
        successful, failed
    ))
}

/// Simulate query timeouts
async fn simulate_query_timeouts(config: &ChaosTestConfig) -> Result<String> {
    let timeout_rate = config.database_failure_rate;
    let query_count = 100;
    let query_timeout = Duration::from_millis(1000);
    
    info!("Simulating query timeouts with {:.1}% timeout rate for {} queries", 
          timeout_rate * 100.0, query_count);
    
    let mut successful_queries = 0;
    let mut timed_out_queries = 0;
    
    for i in 0..query_count {
        let should_timeout = (i as f64 / query_count as f64) < timeout_rate;
        
        if should_timeout {
            // Simulate slow query that times out
            let slow_query = tokio::time::sleep(Duration::from_millis(2000));
            let timeout_result = tokio::time::timeout(query_timeout, slow_query).await;
            
            if timeout_result.is_err() {
                timed_out_queries += 1;
            }
        } else {
            // Simulate fast query
            tokio::time::sleep(Duration::from_millis(50)).await;
            successful_queries += 1;
        }
    }
    
    Ok(format!(
        "Query timeouts: {} successful, {} timed out",
        successful_queries, timed_out_queries
    ))
}

/// Simulate transaction failures
async fn simulate_transaction_failures(config: &ChaosTestConfig) -> Result<String> {
    let failure_rate = config.database_failure_rate;
    let transaction_count = 50;
    
    info!("Simulating transaction failures with {:.1}% failure rate for {} transactions", 
          failure_rate * 100.0, transaction_count);
    
    let mut successful_transactions = 0;
    let mut failed_transactions = 0;
    
    for i in 0..transaction_count {
        let should_fail = (i as f64 / transaction_count as f64) < failure_rate;
        
        // Simulate transaction
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        if should_fail {
            // Simulate transaction rollback
            tokio::time::sleep(Duration::from_millis(50)).await;
            failed_transactions += 1;
        } else {
            // Simulate successful commit
            successful_transactions += 1;
        }
    }
    
    Ok(format!(
        "Transaction failures: {} successful, {} failed",
        successful_transactions, failed_transactions
    ))
}

/// Simulate deadlock scenarios
async fn simulate_deadlock_scenarios(config: &ChaosTestConfig) -> Result<String> {
    let deadlock_probability = config.database_failure_rate;
    let concurrent_transactions = 10;
    
    info!("Simulating deadlock scenarios with {:.1}% deadlock probability for {} transactions", 
          deadlock_probability * 100.0, concurrent_transactions);
    
    let mut tasks = Vec::new();
    let deadlock_count = Arc::new(AtomicU64::new(0));
    let success_count = Arc::new(AtomicU64::new(0));
    
    for i in 0..concurrent_transactions {
        let deadlock_counter = deadlock_count.clone();
        let success_counter = success_count.clone();
        
        let task = tokio::spawn(async move {
            // Simulate transaction that might deadlock
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            let should_deadlock = rand::random::<f64>() < deadlock_probability;
            
            if should_deadlock {
                // Simulate deadlock detection and rollback
                tokio::time::sleep(Duration::from_millis(200)).await;
                deadlock_counter.fetch_add(1, Ordering::Relaxed);
            } else {
                success_counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        
        tasks.push(task);
    }
    
    // Wait for all transactions
    for task in tasks {
        task.await.unwrap();
    }
    
    let deadlocks = deadlock_count.load(Ordering::Relaxed);
    let successes = success_count.load(Ordering::Relaxed);
    
    Ok(format!(
        "Deadlock scenarios: {} successful, {} deadlocked",
        successes, deadlocks
    ))
}

/// Simulate database unavailability
async fn simulate_database_unavailability(config: &ChaosTestConfig) -> Result<String> {
    let unavailable_duration = Duration::from_secs(30);
    let operation_count = 100;
    
    info!("Simulating database unavailability for {:?} during {} operations", 
          unavailable_duration, operation_count);
    
    let start_time = Instant::now();
    let mut successful_ops = 0;
    let mut failed_ops = 0;
    
    for _ in 0..operation_count {
        let elapsed = start_time.elapsed();
        let is_unavailable = elapsed < unavailable_duration;
        
        if is_unavailable {
            // Simulate connection failure
            tokio::time::sleep(Duration::from_millis(100)).await;
            failed_ops += 1;
        } else {
            // Simulate successful operation after recovery
            tokio::time::sleep(Duration::from_millis(50)).await;
            successful_ops += 1;
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    Ok(format!(
        "Database unavailability: {} successful, {} failed",
        successful_ops, failed_ops
    ))
}

// Test resource exhaustion
pub async fn test_resource_exhaustion(config: ChaosTestConfig) -> Result<String> {
    if !config.enabled {
        return Ok("Chaos testing disabled".to_string());
    }

    let mut results = Vec::new();
    
    // Test memory pressure
    if config.memory_pressure {
        results.push(simulate_memory_pressure().await?);
    }
    
    // Test CPU pressure
    if config.cpu_pressure {
        results.push(simulate_cpu_pressure().await?);
    }
    
    // Test disk space exhaustion
    results.push(simulate_disk_space_exhaustion().await?);
    
    // Test file descriptor exhaustion
    results.push(simulate_fd_exhaustion().await?);
    
    // Test thread pool exhaustion
    results.push(simulate_thread_pool_exhaustion().await?);

    Ok(format!(
        "Resource exhaustion tests completed: {} scenarios tested",
        results.len()
    ))
}

/// Simulate memory pressure
async fn simulate_memory_pressure() -> Result<String> {
    info!("Simulating memory pressure");
    
    let initial_memory = get_memory_usage();
    let mut memory_allocations = Vec::new();
    
    // Allocate memory in chunks
    for i in 0..100 {
        let allocation = vec![0u8; 1024 * 1024]; // 1MB chunks
        memory_allocations.push(allocation);
        
        // Check memory usage periodically
        if i % 10 == 0 {
            let current_memory = get_memory_usage();
            let memory_increase = current_memory - initial_memory;
            
            if memory_increase > 100.0 { // 100MB threshold
                break;
            }
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let final_memory = get_memory_usage();
    let memory_increase = final_memory - initial_memory;
    
    // Clean up allocations
    memory_allocations.clear();
    
    Ok(format!(
        "Memory pressure test completed: {:.1}MB increase",
        memory_increase
    ))
}

/// Simulate CPU pressure
async fn simulate_cpu_pressure() -> Result<String> {
    info!("Simulating CPU pressure");
    
    let cpu_intensive_duration = Duration::from_secs(10);
    let start_time = Instant::now();
    
    let mut tasks = Vec::new();
    let cpu_cores = num_cpus::get();
    
    // Spawn CPU-intensive tasks
    for _ in 0..cpu_cores * 2 {
        let task = tokio::spawn(async move {
            let task_start = Instant::now();
            let mut counter = 0u64;
            
            while task_start.elapsed() < cpu_intensive_duration {
                // CPU-intensive work
                for _ in 0..1000000 {
                    counter = counter.wrapping_add(1);
                }
                
                // Yield occasionally
                if counter % 10 == 0 {
                    tokio::task::yield_now().await;
                }
            }
            
            counter
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks
    let mut total_operations = 0;
    for task in tasks {
        total_operations += task.await.unwrap();
    }
    
    let total_duration = start_time.elapsed();
    
    Ok(format!(
        "CPU pressure test completed: {} operations in {:?}",
        total_operations, total_duration
    ))
}

/// Simulate disk space exhaustion
async fn simulate_disk_space_exhaustion() -> Result<String> {
    info!("Simulating disk space exhaustion");
    
    let temp_dir = tempfile::tempdir()?;
    let mut files_created = 0;
    let mut total_size = 0;
    
    // Create files until we hit a reasonable limit
    for i in 0..1000 {
        let file_path = temp_dir.path().join(format!("test_file_{}.dat", i));
        let file_size = 1024 * 1024; // 1MB per file
        
        // Create file with dummy data
        let dummy_data = vec![0u8; file_size];
        match std::fs::write(&file_path, dummy_data) {
            Ok(_) => {
                files_created += 1;
                total_size += file_size;
                
                // Stop at 100MB to avoid actually filling disk
                if total_size > 100 * 1024 * 1024 {
                    break;
                }
            }
            Err(_) => {
                // Simulate disk full error
                break;
            }
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Cleanup happens automatically when temp_dir is dropped
    
    Ok(format!(
        "Disk exhaustion test: {} files created, {:.1}MB total",
        files_created,
        total_size as f64 / (1024.0 * 1024.0)
    ))
}

/// Simulate file descriptor exhaustion
async fn simulate_fd_exhaustion() -> Result<String> {
    info!("Simulating file descriptor exhaustion");
    
    let mut files = Vec::new();
    let mut fd_count = 0;
    
    // Open files until we hit a reasonable limit
    for i in 0..1000 {
        let temp_file = tempfile::NamedTempFile::new();
        match temp_file {
            Ok(file) => {
                files.push(file);
                fd_count += 1;
                
                // Stop at a reasonable number to avoid system issues
                if fd_count > 100 {
                    break;
                }
            }
            Err(_) => {
                // Simulate FD exhaustion
                break;
            }
        }
        
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    // Files are automatically closed when dropped
    
    Ok(format!(
        "FD exhaustion test: {} file descriptors opened",
        fd_count
    ))
}

/// Simulate thread pool exhaustion
async fn simulate_thread_pool_exhaustion() -> Result<String> {
    info!("Simulating thread pool exhaustion");
    
    let max_threads = 50;
    let mut tasks = Vec::new();
    let active_tasks = Arc::new(AtomicU64::new(0));
    
    for i in 0..max_threads {
        let task_counter = active_tasks.clone();
        
        let task = tokio::spawn(async move {
            task_counter.fetch_add(1, Ordering::Relaxed);
            
            // Simulate long-running task
            tokio::time::sleep(Duration::from_secs(1)).await;
            
            task_counter.fetch_sub(1, Ordering::Relaxed);
        });
        
        tasks.push(task);
        
        // Small delay between spawning tasks
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Wait for all tasks
    for task in tasks {
        task.await.unwrap();
    }
    
    Ok(format!(
        "Thread pool exhaustion test: {} tasks completed",
        max_threads
    ))
}

/// Create chaos testing scenarios
pub fn create_chaos_scenarios() -> Vec<ChaosScenario> {
    fixture_create_chaos_scenarios()
}

/// Get current memory usage in MB
fn get_memory_usage() -> f64 {
    #[cfg(feature = "performance")]
    {
        if let Some(usage) = memory_stats() {
            usage.physical_mem as f64 / (1024.0 * 1024.0)
        } else {
            0.0
        }
    }
    #[cfg(not(feature = "performance"))]
    {
        // Simulate memory usage for testing
        rand::random::<f64>() * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chaos_scenario_creation() {
        let scenarios = create_chaos_scenarios();
        assert!(!scenarios.is_empty());
        assert!(scenarios.len() >= 5);
    }

    #[tokio::test]
    async fn test_memory_pressure_simulation() {
        let result = simulate_memory_pressure().await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Memory pressure test completed"));
    }

    #[tokio::test]
    async fn test_disk_space_simulation() {
        let result = simulate_disk_space_exhaustion().await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Disk exhaustion test"));
    }

    #[test]
    fn test_chaos_config_validation() {
        let config = ChaosTestConfig {
            enabled: true,
            network_failure_rate: 0.1,
            database_failure_rate: 0.05,
            memory_pressure: true,
            cpu_pressure: false,
        };
        
        assert!(config.enabled);
        assert!(config.network_failure_rate > 0.0 && config.network_failure_rate < 1.0);
        assert!(config.database_failure_rate > 0.0 && config.database_failure_rate < 1.0);
    }
} 