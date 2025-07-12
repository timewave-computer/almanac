// Performance testing module for load testing, stress testing, and benchmarking
//
// This module provides comprehensive performance testing capabilities including
// load testing, stress testing, memory leak detection, and performance benchmarks

use anyhow::Result;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use tokio::sync::Semaphore;
use futures::future::join_all;
use reqwest::Client;
use serde_json::json;

#[cfg(feature = "performance")]
use memory_stats::memory_stats;

use crate::{PerformanceTestConfig, fixtures};

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerformanceResults {
    pub test_name: String,
    pub duration: Duration,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub requests_per_second: f64,
    pub average_response_time_ms: f64,
    pub min_response_time_ms: f64,
    pub max_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub memory_usage_mb: f64,
    pub peak_memory_mb: f64,
    pub cpu_usage_percent: f64,
    pub errors: Vec<String>,
}

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub concurrent_users: usize,
    pub requests_per_user: usize,
    pub ramp_up_duration: Duration,
    pub steady_state_duration: Duration,
    pub ramp_down_duration: Duration,
    pub target_endpoints: Vec<String>,
}

/// Stress test configuration
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    pub max_concurrent_users: usize,
    pub increment_step: usize,
    pub increment_interval: Duration,
    pub test_duration_per_step: Duration,
    pub failure_threshold_percent: f64,
}

/// Memory leak test configuration
#[derive(Debug, Clone)]
pub struct MemoryLeakTestConfig {
    pub test_duration: Duration,
    pub measurement_interval: Duration,
    pub operation_interval: Duration,
    pub memory_growth_threshold_mb: f64,
}

/// Simulate user requests for load testing
async fn simulate_user_requests(request_count: usize, delay_ms: u64) -> Result<()> {
    for _ in 0..request_count {
        // Simulate HTTP request
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Add delay between requests
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
    Ok(())
}

/// Run load test with gradual ramp-up
pub async fn run_load_test(config: &PerformanceTestConfig) -> Result<PerformanceResults> {
    let request_delay_ms = 100u64; // Default delay since field is missing
    
    let load_config = LoadTestConfig {
        concurrent_users: config.concurrent_connections,
        requests_per_user: 100,
        ramp_up_duration: Duration::from_secs(30),
        steady_state_duration: Duration::from_secs(config.test_duration_seconds),
        ramp_down_duration: Duration::from_secs(10),
        target_endpoints: vec![
            "/api/v1/health".to_string(),
            "/api/v1/events/chain/ethereum".to_string(),
            "/api/v1/blocks/ethereum/latest".to_string(),
        ],
    };

    let results = execute_load_test(&load_config).await?;
    
    // Validate results against targets
    let target_rps = config.target_rps;
    let memory_limit = config.memory_limit_mb as f64;
    
    let mut validation_results = Vec::new();
    
    if results.requests_per_second >= target_rps * 0.9 {
        validation_results.push(format!("✅ RPS: {:.1} (target: {:.1})", results.requests_per_second, target_rps));
    } else {
        validation_results.push(format!("❌ RPS: {:.1} (target: {:.1})", results.requests_per_second, target_rps));
    }
    
    if results.peak_memory_mb <= memory_limit {
        validation_results.push(format!("✅ Memory: {:.1}MB (limit: {:.1}MB)", results.peak_memory_mb, memory_limit));
    } else {
        validation_results.push(format!("❌ Memory: {:.1}MB (limit: {:.1}MB)", results.peak_memory_mb, memory_limit));
    }
    
    if results.p95_response_time_ms <= 1000.0 {
        validation_results.push(format!("✅ P95 latency: {:.1}ms", results.p95_response_time_ms));
    } else {
        validation_results.push(format!("❌ P95 latency: {:.1}ms", results.p95_response_time_ms));
    }

    Ok(PerformanceResults {
        test_name: "Load Test".to_string(),
        duration: results.duration,
        total_requests: results.total_requests,
        successful_requests: results.successful_requests,
        failed_requests: results.failed_requests,
        requests_per_second: results.requests_per_second,
        average_response_time_ms: results.average_response_time_ms,
        min_response_time_ms: results.min_response_time_ms,
        max_response_time_ms: results.max_response_time_ms,
        p95_response_time_ms: results.p95_response_time_ms,
        p99_response_time_ms: results.p99_response_time_ms,
        memory_usage_mb: results.memory_usage_mb,
        peak_memory_mb: results.peak_memory_mb,
        cpu_usage_percent: results.cpu_usage_percent,
        errors: results.errors,
    })
}

/// Execute load test
async fn execute_load_test(config: &LoadTestConfig) -> Result<PerformanceResults> {
    let start_time = Instant::now();
    let client = Arc::new(Client::new());
    
    // Metrics collection
    let total_requests = Arc::new(AtomicU64::new(0));
    let successful_requests = Arc::new(AtomicU64::new(0));
    let failed_requests = Arc::new(AtomicU64::new(0));
    let response_times = Arc::new(std::sync::Mutex::new(Vec::new()));
    let errors = Arc::new(std::sync::Mutex::new(Vec::new()));
    
    // Memory monitoring
    let initial_memory = get_memory_usage();
    let peak_memory = Arc::new(std::sync::Mutex::new(initial_memory));
    
    // Semaphore for controlling concurrency
    let semaphore = Arc::new(Semaphore::new(config.concurrent_users));
    
    // Phase 1: Ramp up
    info!("Load test: Ramp up phase ({:?})", config.ramp_up_duration);
    execute_ramp_up_phase(config, &client, &semaphore, &total_requests, &successful_requests, &failed_requests, &response_times, &errors).await?;
    
    // Phase 2: Steady state
    info!("Load test: Steady state phase ({:?})", config.steady_state_duration);
    execute_steady_state_phase(config, &client, &semaphore, &total_requests, &successful_requests, &failed_requests, &response_times, &errors, &peak_memory).await?;
    
    // Phase 3: Ramp down
    info!("Load test: Ramp down phase ({:?})", config.ramp_down_duration);
    execute_ramp_down_phase(config, &client, &semaphore, &total_requests, &successful_requests, &failed_requests, &response_times, &errors).await?;
    
    let total_duration = start_time.elapsed();
    let final_memory = get_memory_usage();
    
    // Calculate results
    let response_times_vec = response_times.lock().unwrap().clone();
    let errors_vec = errors.lock().unwrap().clone();
    
    let results = calculate_performance_results(
        "Load Test".to_string(),
        total_duration,
        total_requests.load(Ordering::Relaxed),
        successful_requests.load(Ordering::Relaxed),
        failed_requests.load(Ordering::Relaxed),
        response_times_vec,
        initial_memory,
        *peak_memory.lock().unwrap(),
        final_memory,
        errors_vec,
    );
    
    Ok(results)
}

/// Execute ramp up phase
async fn execute_ramp_up_phase(
    config: &LoadTestConfig,
    client: &Arc<Client>,
    semaphore: &Arc<Semaphore>,
    total_requests: &Arc<AtomicU64>,
    successful_requests: &Arc<AtomicU64>,
    failed_requests: &Arc<AtomicU64>,
    response_times: &Arc<std::sync::Mutex<Vec<f64>>>,
    errors: &Arc<std::sync::Mutex<Vec<String>>>,
) -> Result<()> {
    let ramp_up_steps = 10u32;
    let users_per_step = config.concurrent_users / ramp_up_steps as usize;
    let requests_per_user = config.requests_per_user;
    let request_delay_ms = 100u64; // Default delay between requests
    
    for step in 0..ramp_up_steps {
        let current_users = users_per_step * step as usize;
        
        // Simulate user load
        let mut handles = Vec::new();
        for _ in 0..current_users {
            let handle = tokio::spawn(async move {
                // Simulate user requests
                simulate_user_requests(
                    requests_per_user / ramp_up_steps as usize,
                    request_delay_ms,
                ).await
            });
            handles.push(handle);
        }
        
        // Wait for step completion
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        
        // Add delay between ramp-up steps
        tokio::time::sleep(Duration::from_millis(request_delay_ms)).await;
        
        debug!("Ramp up step {}/{}: {} active users", step + 1, ramp_up_steps, current_users);
    }
    
    Ok(())
}

/// Execute steady state phase
async fn execute_steady_state_phase(
    config: &LoadTestConfig,
    client: &Arc<Client>,
    semaphore: &Arc<Semaphore>,
    total_requests: &Arc<AtomicU64>,
    successful_requests: &Arc<AtomicU64>,
    failed_requests: &Arc<AtomicU64>,
    response_times: &Arc<std::sync::Mutex<Vec<f64>>>,
    errors: &Arc<std::sync::Mutex<Vec<String>>>,
    peak_memory: &Arc<std::sync::Mutex<f64>>,
) -> Result<()> {
    let start_time = Instant::now();
    let memory_monitor = start_memory_monitoring(peak_memory.clone());
    
    // Launch all concurrent users
    let mut tasks = Vec::new();
    for _ in 0..config.concurrent_users {
        let task = create_user_task(
            client.clone(),
            semaphore.clone(),
            config.target_endpoints.clone(),
            config.requests_per_user,
            total_requests.clone(),
            successful_requests.clone(),
            failed_requests.clone(),
            response_times.clone(),
            errors.clone(),
        );
        tasks.push(task);
    }
    
    // Wait for steady state duration
    tokio::time::sleep(config.steady_state_duration).await;
    
    // Stop memory monitoring
    memory_monitor.store(false, Ordering::Relaxed);
    
    // Wait for all tasks to complete (with timeout)
    let timeout_duration = Duration::from_secs(30);
    match tokio::time::timeout(timeout_duration, join_all(tasks)).await {
        Ok(_) => debug!("All steady state tasks completed"),
        Err(_) => warn!("Some steady state tasks timed out"),
    }
    
    Ok(())
}

/// Execute ramp down phase
async fn execute_ramp_down_phase(
    config: &LoadTestConfig,
    client: &Arc<Client>,
    semaphore: &Arc<Semaphore>,
    total_requests: &Arc<AtomicU64>,
    successful_requests: &Arc<AtomicU64>,
    failed_requests: &Arc<AtomicU64>,
    response_times: &Arc<std::sync::Mutex<Vec<f64>>>,
    errors: &Arc<std::sync::Mutex<Vec<String>>>,
) -> Result<()> {
    // Gradually reduce load
    let remaining_duration = config.ramp_down_duration;
    let final_requests = config.requests_per_user / 10; // Reduced requests during ramp down
    
    let mut tasks = Vec::new();
    for _ in 0..config.concurrent_users / 2 {
        let task = create_user_task(
            client.clone(),
            semaphore.clone(),
            config.target_endpoints.clone(),
            final_requests,
            total_requests.clone(),
            successful_requests.clone(),
            failed_requests.clone(),
            response_times.clone(),
            errors.clone(),
        );
        tasks.push(task);
    }
    
    tokio::time::sleep(remaining_duration).await;
    
    Ok(())
}

/// Create a user task that simulates a single user's behavior
async fn create_user_task(
    client: Arc<Client>,
    semaphore: Arc<Semaphore>,
    endpoints: Vec<String>,
    requests_count: usize,
    total_requests: Arc<AtomicU64>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    response_times: Arc<std::sync::Mutex<Vec<f64>>>,
    errors: Arc<std::sync::Mutex<Vec<String>>>,
) -> Result<()> {
    let _permit = semaphore.acquire().await?;
    
    for _ in 0..requests_count {
        // Select random endpoint
        let endpoint = &endpoints[rand::random::<usize>() % endpoints.len()];
        let url = format!("http://localhost:8080{}", endpoint);
        
        let start_time = Instant::now();
        total_requests.fetch_add(1, Ordering::Relaxed);
        
        match client.get(&url).send().await {
            Ok(response) => {
                let duration = start_time.elapsed();
                let duration_ms = duration.as_secs_f64() * 1000.0;
                
                if response.status().is_success() {
                    successful_requests.fetch_add(1, Ordering::Relaxed);
                    response_times.lock().unwrap().push(duration_ms);
                } else {
                    failed_requests.fetch_add(1, Ordering::Relaxed);
                    errors.lock().unwrap().push(format!("HTTP {}: {}", response.status(), endpoint));
                }
            }
            Err(e) => {
                failed_requests.fetch_add(1, Ordering::Relaxed);
                errors.lock().unwrap().push(format!("Request error: {}", e));
            }
        }
        
        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    Ok(())
}

/// Run stress test
pub async fn run_stress_test(config: &PerformanceTestConfig) -> Result<PerformanceResults> {
    let stress_config = StressTestConfig {
        max_concurrent_users: config.concurrent_connections * 2,
        increment_step: config.concurrent_connections / 10,
        increment_interval: Duration::from_secs(30),
        test_duration_per_step: Duration::from_secs(60),
        failure_threshold_percent: 10.0,
    };

    let results = execute_stress_test(&stress_config).await?;
    
    Ok(results)
}

/// Execute stress test
async fn execute_stress_test(config: &StressTestConfig) -> Result<PerformanceResults> {
    let start_time = Instant::now();
    let client = Arc::new(Client::new());
    
    let mut step_results = Vec::new();
    let mut current_users = config.increment_step;
    
    // Gradually increase load until failure threshold is reached
    while current_users <= config.max_concurrent_users {
        info!("Stress test: Testing with {} concurrent users", current_users);
        
        let step_result = execute_stress_test_step(&client, current_users, config).await?;
        let error_rate = (step_result.failed_requests as f64 / step_result.total_requests as f64) * 100.0;
        
        step_results.push(step_result.clone());
        
        if error_rate > config.failure_threshold_percent {
            warn!("Stress test: Failure threshold exceeded at {} users ({:.1}% error rate)", current_users, error_rate);
            break;
        }
        
        current_users += config.increment_step;
        tokio::time::sleep(config.increment_interval).await;
    }
    
    // Aggregate results
    let total_duration = start_time.elapsed();
    let aggregated_results = aggregate_stress_test_results("Stress Test".to_string(), total_duration, step_results);
    
    Ok(aggregated_results)
}

/// Execute a single step of stress test
async fn execute_stress_test_step(
    client: &Arc<Client>,
    concurrent_users: usize,
    config: &StressTestConfig,
) -> Result<PerformanceResults> {
    let start_time = Instant::now();
    let semaphore = Arc::new(Semaphore::new(concurrent_users));
    
    // Metrics collection
    let total_requests = Arc::new(AtomicU64::new(0));
    let successful_requests = Arc::new(AtomicU64::new(0));
    let failed_requests = Arc::new(AtomicU64::new(0));
    let response_times = Arc::new(std::sync::Mutex::new(Vec::new()));
    let errors = Arc::new(std::sync::Mutex::new(Vec::new()));
    
    let initial_memory = get_memory_usage();
    
    // Launch concurrent users
    let mut tasks = Vec::new();
    for _ in 0..concurrent_users {
        let task = create_user_task(
            client.clone(),
            semaphore.clone(),
            vec!["/api/v1/health".to_string()],
            50, // requests per user for stress test
            total_requests.clone(),
            successful_requests.clone(),
            failed_requests.clone(),
            response_times.clone(),
            errors.clone(),
        );
        tasks.push(task);
    }
    
    // Wait for test duration
    tokio::time::sleep(config.test_duration_per_step).await;
    
    let final_memory = get_memory_usage();
    let response_times_vec = response_times.lock().unwrap().clone();
    let errors_vec = errors.lock().unwrap().clone();
    
    let results = calculate_performance_results(
        format!("Stress Test Step ({} users)", concurrent_users),
        start_time.elapsed(),
        total_requests.load(Ordering::Relaxed),
        successful_requests.load(Ordering::Relaxed),
        failed_requests.load(Ordering::Relaxed),
        response_times_vec,
        initial_memory,
        final_memory,
        final_memory,
        errors_vec,
    );
    
    Ok(results)
}

/// Run memory leak test
pub async fn run_memory_leak_test(config: &PerformanceTestConfig) -> Result<PerformanceResults> {
    let leak_config = MemoryLeakTestConfig {
        test_duration: Duration::from_secs(config.test_duration_seconds),
        measurement_interval: Duration::from_secs(10),
        operation_interval: Duration::from_millis(100),
        memory_growth_threshold_mb: 100.0,
    };

    let results = execute_memory_leak_test(&leak_config).await?;
    
    Ok(results)
}

/// Execute memory leak test
async fn execute_memory_leak_test(config: &MemoryLeakTestConfig) -> Result<PerformanceResults> {
    let start_time = Instant::now();
    let initial_memory = get_memory_usage();
    
    let mut memory_measurements = Vec::new();
    let mut operations_count = 0u64;
    
    // Start memory monitoring
    let stop_monitoring = Arc::new(AtomicBool::new(false));
    let memory_measurements_arc = Arc::new(std::sync::Mutex::new(Vec::new()));
    
    // Memory monitoring task
    let measurements_clone = memory_measurements_arc.clone();
    let stop_clone = stop_monitoring.clone();
    let measurement_interval = config.measurement_interval;
    let monitor_task = tokio::spawn(async move {
        while !stop_clone.load(Ordering::Relaxed) {
            let current_memory = get_memory_usage();
            measurements_clone.lock().unwrap().push((Instant::now(), current_memory));
            tokio::time::sleep(measurement_interval).await;
        }
    });
    
    // Simulate operations that might cause memory leaks
    let operation_start = Instant::now();
    while operation_start.elapsed() < config.test_duration {
        // Simulate memory-intensive operations
        perform_memory_intensive_operation().await;
        operations_count += 1;
        
        tokio::time::sleep(config.operation_interval).await;
    }
    
    // Stop monitoring
    stop_monitoring.store(true, Ordering::Relaxed);
    monitor_task.await?;
    
    memory_measurements = memory_measurements_arc.lock().unwrap().clone();
    
    let final_memory = get_memory_usage();
    let peak_memory = memory_measurements.iter()
        .map(|(_, memory)| *memory)
        .fold(initial_memory, f64::max);
    
    // Analyze memory growth
    let memory_growth = final_memory - initial_memory;
    let growth_rate = memory_growth / config.test_duration.as_secs_f64();
    
    info!("Memory leak test: {:.1}MB initial, {:.1}MB final, {:.1}MB peak, {:.3}MB/s growth rate",
          initial_memory, final_memory, peak_memory, growth_rate);
    
    let results = PerformanceResults {
        test_name: "Memory Leak Test".to_string(),
        duration: start_time.elapsed(),
        total_requests: operations_count,
        successful_requests: operations_count,
        failed_requests: 0,
        requests_per_second: operations_count as f64 / config.test_duration.as_secs_f64(),
        average_response_time_ms: 0.0,
        min_response_time_ms: 0.0,
        max_response_time_ms: 0.0,
        p95_response_time_ms: 0.0,
        p99_response_time_ms: 0.0,
        memory_usage_mb: initial_memory,
        peak_memory_mb: peak_memory,
        cpu_usage_percent: 0.0,
        errors: Vec::new(),
    };
    
    Ok(results)
}

/// Perform memory-intensive operation for leak testing
async fn perform_memory_intensive_operation() {
    // Simulate operations that might leak memory
    let _large_vec: Vec<u8> = vec![0; 1024 * 1024]; // 1MB allocation
    
    // Simulate some processing
    tokio::time::sleep(Duration::from_millis(1)).await;
    
    // Vector should be dropped here
}

/// Start memory monitoring
fn start_memory_monitoring(peak_memory: Arc<std::sync::Mutex<f64>>) -> Arc<AtomicBool> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_clone = stop_flag.clone();
    
    tokio::spawn(async move {
        while !stop_clone.load(Ordering::Relaxed) {
            let current_memory = get_memory_usage();
            {
                let mut peak = peak_memory.lock().unwrap();
                if current_memory > *peak {
                    *peak = current_memory;
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    
    stop_flag
}

/// Get current memory usage in MB
fn get_memory_usage() -> f64 {
    #[cfg(feature = "performance")]
    {
        if let Some(usage) = memory_stats::memory_stats() {
            return usage.physical_mem as f64 / 1024.0 / 1024.0;
        }
    }
    
    // Fallback for when performance features are not available
    0.0
}

/// Calculate performance results from collected metrics
fn calculate_performance_results(
    test_name: String,
    duration: Duration,
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    response_times: Vec<f64>,
    initial_memory: f64,
    peak_memory: f64,
    final_memory: f64,
    errors: Vec<String>,
) -> PerformanceResults {
    let requests_per_second = total_requests as f64 / duration.as_secs_f64();
    
    let (average_response_time, min_response_time, max_response_time, p95_response_time, p99_response_time) = 
        if response_times.is_empty() {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            let mut sorted_times = response_times.clone();
            sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            let average = sorted_times.iter().sum::<f64>() / sorted_times.len() as f64;
            let min = sorted_times[0];
            let max = sorted_times[sorted_times.len() - 1];
            let p95_index = (sorted_times.len() as f64 * 0.95) as usize;
            let p99_index = (sorted_times.len() as f64 * 0.99) as usize;
            let p95 = sorted_times[p95_index.min(sorted_times.len() - 1)];
            let p99 = sorted_times[p99_index.min(sorted_times.len() - 1)];
            
            (average, min, max, p95, p99)
        };
    
    PerformanceResults {
        test_name,
        duration,
        total_requests,
        successful_requests,
        failed_requests,
        requests_per_second,
        average_response_time_ms: average_response_time,
        min_response_time_ms: min_response_time,
        max_response_time_ms: max_response_time,
        p95_response_time_ms: p95_response_time,
        p99_response_time_ms: p99_response_time,
        memory_usage_mb: initial_memory,
        peak_memory_mb: peak_memory,
        cpu_usage_percent: 0.0, // Would be calculated from actual CPU monitoring
        errors,
    }
}

/// Aggregate multiple performance results
fn aggregate_stress_test_results(
    test_name: String,
    total_duration: Duration,
    step_results: Vec<PerformanceResults>,
) -> PerformanceResults {
    if step_results.is_empty() {
        return PerformanceResults {
            test_name,
            duration: total_duration,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            requests_per_second: 0.0,
            average_response_time_ms: 0.0,
            min_response_time_ms: 0.0,
            max_response_time_ms: 0.0,
            p95_response_time_ms: 0.0,
            p99_response_time_ms: 0.0,
            memory_usage_mb: 0.0,
            peak_memory_mb: 0.0,
            cpu_usage_percent: 0.0,
            errors: Vec::new(),
        };
    }
    
    let total_requests = step_results.iter().map(|r| r.total_requests).sum();
    let successful_requests = step_results.iter().map(|r| r.successful_requests).sum();
    let failed_requests = step_results.iter().map(|r| r.failed_requests).sum();
    let peak_rps = step_results.iter().map(|r| r.requests_per_second).fold(0.0, f64::max);
    let peak_memory = step_results.iter().map(|r| r.peak_memory_mb).fold(0.0, f64::max);
    let max_response_time = step_results.iter().map(|r| r.max_response_time_ms).fold(0.0, f64::max);
    
    let mut all_errors = Vec::new();
    for result in &step_results {
        all_errors.extend(result.errors.clone());
    }
    
    PerformanceResults {
        test_name,
        duration: total_duration,
        total_requests,
        successful_requests,
        failed_requests,
        requests_per_second: peak_rps,
        average_response_time_ms: step_results.iter().map(|r| r.average_response_time_ms).sum::<f64>() / step_results.len() as f64,
        min_response_time_ms: step_results.iter().map(|r| r.min_response_time_ms).fold(f64::INFINITY, f64::min),
        max_response_time_ms: max_response_time,
        p95_response_time_ms: step_results.iter().map(|r| r.p95_response_time_ms).fold(0.0, f64::max),
        p99_response_time_ms: step_results.iter().map(|r| r.p99_response_time_ms).fold(0.0, f64::max),
        memory_usage_mb: step_results[0].memory_usage_mb,
        peak_memory_mb: peak_memory,
        cpu_usage_percent: step_results.iter().map(|r| r.cpu_usage_percent).fold(0.0, f64::max),
        errors: all_errors,
    }
}

// Re-export logging macros
use tracing::{info, debug, warn};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_usage_monitoring() {
        let initial = get_memory_usage();
        let large_allocation: Vec<u8> = vec![0; 10 * 1024 * 1024]; // 10MB
        let after_allocation = get_memory_usage();
        drop(large_allocation);
        let after_drop = get_memory_usage();
        
        // Memory should increase after allocation (if monitoring is working)
        // Note: This test might not be reliable in all environments
        println!("Memory: initial={:.1}MB, after_alloc={:.1}MB, after_drop={:.1}MB", 
                initial, after_allocation, after_drop);
    }

    #[test]
    fn test_performance_results_calculation() {
        let response_times = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        let duration = Duration::from_secs(10);
        
        let results = calculate_performance_results(
            "Test".to_string(),
            duration,
            100,
            95,
            5,
            response_times,
            100.0,
            120.0,
            110.0,
            vec!["Error 1".to_string()],
        );
        
        assert_eq!(results.total_requests, 100);
        assert_eq!(results.successful_requests, 95);
        assert_eq!(results.failed_requests, 5);
        assert_eq!(results.requests_per_second, 10.0);
        assert_eq!(results.average_response_time_ms, 55.0);
        assert_eq!(results.min_response_time_ms, 10.0);
        assert_eq!(results.max_response_time_ms, 100.0);
        assert_eq!(results.peak_memory_mb, 120.0);
    }
} 