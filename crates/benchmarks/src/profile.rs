// profile.rs - Profiling and resource usage tracking
//
// Purpose: Provides utilities for measuring CPU, memory, and other resource
// usage during benchmark execution

use std::time::Duration;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use indexer_core::Error;
use super::Measurement;

/// CPU usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuUsage {
    /// User CPU time (seconds)
    pub user: f64,
    
    /// System CPU time (seconds)
    pub system: f64,
    
    /// Total CPU time (user + system)
    pub total: f64,
    
    /// CPU usage percentage (0-100)
    pub percentage: f64,
}

/// Memory usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Resident set size (physical memory) in bytes
    pub rss: u64,
    
    /// Virtual memory size in bytes
    pub vms: u64,
    
    /// Shared memory size in bytes
    pub shared: u64,
    
    /// Memory usage percentage (0-100)
    pub percentage: f64,
}

/// I/O usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoUsage {
    /// Read bytes
    pub read_bytes: u64,
    
    /// Write bytes
    pub write_bytes: u64,
    
    /// Read operations
    pub read_ops: u64,
    
    /// Write operations
    pub write_ops: u64,
}

/// Network usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkUsage {
    /// Received bytes
    pub recv_bytes: u64,
    
    /// Sent bytes
    pub sent_bytes: u64,
    
    /// Received packets
    pub recv_packets: u64,
    
    /// Sent packets
    pub sent_packets: u64,
}

/// Resource usage profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage information
    pub cpu: CpuUsage,
    
    /// Memory usage information
    pub memory: MemoryUsage,
    
    /// I/O usage information
    pub io: IoUsage,
    
    /// Network usage information
    pub network: NetworkUsage,
    
    /// Timestamp when the profile was taken
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ResourceUsage {
    /// Create a new resource usage profile with current system info
    pub fn new() -> Result<Self, Error> {
        let timestamp = chrono::Utc::now();
        
        // Get process information using sysinfo
        let mut system = sysinfo::System::new();
        system.refresh_all();
        
        let pid = sysinfo::get_current_pid().map_err(|e| Error::Other(format!("Failed to get pid: {}", e)))?;
        let process = system.process(pid).ok_or_else(|| Error::Other("Process not found".to_string()))?;
        
        // CPU usage
        let cpu_usage = CpuUsage {
            user: process.user_time() as f64 / 100.0,
            system: process.system_time() as f64 / 100.0,
            total: (process.user_time() + process.system_time()) as f64 / 100.0,
            percentage: process.cpu_usage(),
        };
        
        // Memory usage
        let memory_usage = MemoryUsage {
            rss: process.memory(),
            vms: process.virtual_memory(),
            shared: 0, // Not available in sysinfo
            percentage: (process.memory() as f64 / system.total_memory() as f64) * 100.0,
        };
        
        // I/O usage - not directly available in sysinfo, using placeholder
        let io_usage = IoUsage {
            read_bytes: 0,
            write_bytes: 0,
            read_ops: 0,
            write_ops: 0,
        };
        
        // Network usage - not directly tied to process in sysinfo, using placeholder
        let network_usage = NetworkUsage {
            recv_bytes: 0,
            sent_bytes: 0,
            recv_packets: 0,
            sent_packets: 0,
        };
        
        Ok(Self {
            cpu: cpu_usage,
            memory: memory_usage,
            io: io_usage,
            network: network_usage,
            timestamp,
        })
    }
}

/// Profile sampler that periodically samples resource usage
pub struct ResourceProfiler {
    /// Sampling interval
    interval: Duration,
    
    /// Whether the profiler is running
    running: bool,
    
    /// Collected samples
    samples: Arc<std::sync::Mutex<Vec<ResourceUsage>>>,
    
    /// Join handle for the sampling task
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ResourceProfiler {
    /// Create a new resource profiler with the given sampling interval
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            running: false,
            samples: Arc::new(std::sync::Mutex::new(Vec::new())),
            task_handle: None,
        }
    }
    
    /// Start sampling resource usage
    pub fn start(&mut self) -> Result<(), Error> {
        if self.running {
            return Ok(());
        }
        
        self.running = true;
        let samples = self.samples.clone();
        let interval = self.interval;
        
        // Spawn a task to sample resource usage
        let handle = tokio::task::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                // Sample resource usage
                match ResourceUsage::new() {
                    Ok(usage) => {
                        let mut samples = samples.lock().unwrap();
                        samples.push(usage);
                    }
                    Err(e) => {
                        eprintln!("Error sampling resource usage: {}", e);
                    }
                }
            }
        });
        
        self.task_handle = Some(handle);
        
        Ok(())
    }
    
    /// Stop sampling resource usage
    pub fn stop(&mut self) -> Result<Vec<ResourceUsage>, Error> {
        if !self.running {
            return Ok(Vec::new());
        }
        
        self.running = false;
        
        // Abort the sampling task
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        
        // Return the collected samples
        let samples = {
            let mut samples_lock = self.samples.lock().unwrap();
            std::mem::replace(&mut *samples_lock, Vec::new())
        };
        
        Ok(samples)
    }
    
    /// Get the current samples without stopping
    pub fn get_samples(&self) -> Result<Vec<ResourceUsage>, Error> {
        let samples = self.samples.lock().unwrap();
        Ok(samples.clone())
    }
}

/// Measure CPU and memory usage during a function execution
pub async fn profile_function<F, Fut, T>(
    func: F,
    sample_interval: Duration,
) -> Result<(T, Vec<ResourceUsage>), Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    // Start profiling
    let mut profiler = ResourceProfiler::new(sample_interval);
    profiler.start()?;
    
    // Execute the function
    let result = func().await;
    
    // Stop profiling
    let samples = profiler.stop()?;
    
    // Return the result and samples
    Ok((result?, samples))
}

/// Generate measurement statistics from resource usage samples
pub fn resource_usage_to_measurement(
    name: &str,
    duration: Duration,
    operations: u64,
    data_size: u64,
    samples: &[ResourceUsage],
) -> Measurement {
    let mut measurement = Measurement::new(name, duration, operations, data_size);
    
    if samples.is_empty() {
        return measurement;
    }
    
    // Calculate average CPU usage
    let avg_cpu_usage = samples.iter().map(|s| s.cpu.percentage).sum::<f64>() / samples.len() as f64;
    
    // Calculate average memory usage
    let avg_memory_usage = samples.iter().map(|s| s.memory.rss).sum::<u64>() / samples.len() as u64;
    
    // Calculate peak memory usage
    let peak_memory_usage = samples.iter().map(|s| s.memory.rss).max().unwrap_or(0);
    
    // Add metrics
    measurement = measurement
        .with_metric("avg_cpu_percentage", avg_cpu_usage)
        .with_metric("avg_memory_bytes", avg_memory_usage as f64)
        .with_metric("peak_memory_bytes", peak_memory_usage as f64);
    
    measurement
}

/// Run a benchmark with profiling
pub async fn run_benchmark_with_profiling<F, Fut, T>(
    name: &str,
    func: F,
    sample_interval: Duration,
) -> Result<(T, Measurement), Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(u64, u64), Error>>,
{
    let start = std::time::Instant::now();
    
    // Run the function with profiling
    let ((operations, data_size), samples) = profile_function(func, sample_interval).await?;
    
    let duration = start.elapsed();
    
    // Create measurement
    let measurement = resource_usage_to_measurement(name, duration, operations, data_size, &samples);
    
    Ok(((operations, data_size), measurement))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_resource_usage() {
        // Get current resource usage
        let usage = ResourceUsage::new().unwrap();
        
        // Verify the structure
        assert!(usage.cpu.percentage >= 0.0);
        assert!(usage.memory.rss > 0);
        assert_eq!(usage.timestamp.date(), chrono::Utc::now().date());
    }
    
    #[tokio::test]
    async fn test_resource_profiler() {
        // Create a profiler with 10ms interval
        let mut profiler = ResourceProfiler::new(Duration::from_millis(10));
        
        // Start profiling
        profiler.start().unwrap();
        
        // Wait for a bit to collect samples
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Get samples
        let samples = profiler.get_samples().unwrap();
        
        // Should have collected some samples
        assert!(!samples.is_empty());
        
        // Stop profiling
        let final_samples = profiler.stop().unwrap();
        
        // Should have collected samples
        assert!(!final_samples.is_empty());
    }
    
    #[tokio::test]
    async fn test_profile_function() {
        // Profile a simple function
        let (result, samples) = profile_function(|| async {
            // Simulate some CPU work
            let mut sum = 0;
            for i in 0..1_000_000 {
                sum += i;
            }
            
            // Simulate some memory allocation
            let data = vec![0u8; 10 * 1024 * 1024];
            let _ = data.len();
            
            // Wait a bit
            tokio::time::sleep(Duration::from_millis(50)).await;
            
            Ok::<_, Error>(sum)
        }, Duration::from_millis(10)).await.unwrap();
        
        // Verify the result
        assert_eq!(result, (1_000_000 - 1) * 1_000_000 / 2);
        
        // Should have collected some samples
        assert!(!samples.is_empty());
    }
} 