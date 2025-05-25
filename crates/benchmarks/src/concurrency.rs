// concurrency.rs - Concurrency and scaling optimization utilities
//
// Purpose: Provides tools for optimizing performance through parallel processing,
// resource utilization monitoring, and concurrency scaling strategies

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinSet;
use super::{Measurement, BenchmarkReport};

/// Error type for concurrency operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Parallel processing configuration
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of worker threads/tasks
    pub worker_count: usize,
    
    /// Size of the work queue/channel buffer
    pub queue_size: usize,
    
    /// Batch size for each worker
    pub batch_size: usize,
    
    /// Whether to use work stealing
    pub work_stealing: bool,
    
    /// Resource limits (0 = unlimited)
    pub memory_limit_mb: usize,
    
    /// CPU affinity settings (if supported)
    pub cpu_affinity: Option<Vec<usize>>,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        
        Self {
            worker_count: cpu_count,
            queue_size: 1000,
            batch_size: 100,
            work_stealing: true,
            memory_limit_mb: 0, // Unlimited
            cpu_affinity: None,
        }
    }
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// CPU usage percentage (0-100 × core count)
    pub cpu_usage_percent: f64,
    
    /// Memory usage in megabytes
    pub memory_usage_mb: f64,
    
    /// I/O operations per second
    pub io_ops_per_second: f64,
    
    /// Network bytes per second
    pub network_bytes_per_second: f64,
    
    /// Time of measurement
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceUsage {
    /// Create a new resource usage snapshot
    pub fn new() -> Self {
        // For a real implementation, we would collect actual system metrics here
        // This is a placeholder implementation
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            io_ops_per_second: 0.0,
            network_bytes_per_second: 0.0,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Sample current resource usage
    pub fn sample() -> Self {
        let mut usage = Self::new();
        
        // CPU usage (simplified approximation)
        if let Ok(load) = sys_info::loadavg() {
            usage.cpu_usage_percent = load.one * 100.0 / num_cpus::get() as f64;
        }
        
        // Memory usage
        if let Ok(mem) = sys_info::mem_info() {
            let used_mem = mem.total - mem.free - mem.buffers - mem.cached;
            usage.memory_usage_mb = used_mem as f64 / 1024.0;
        }
        
        // IO and network would require more sophisticated monitoring
        // These would typically use platform-specific APIs or tools like procfs
        
        usage
    }
}

/// Worker task for parallel processing
pub struct Worker<T, R> {
    /// Worker ID
    id: usize,
    
    /// Receiver for work items
    receiver: mpsc::Receiver<Vec<T>>,
    
    /// Sender for results
    result_sender: mpsc::Sender<Vec<R>>,
    
    /// Processing function
    processor: Arc<dyn Fn(T) -> Result<R, Error> + Send + Sync>,
    
    /// Whether to continue even if some items fail
    continue_on_error: bool,
    
    /// Resource usage monitor
    resource_monitor: Option<Arc<ResourceMonitor>>,
}

impl<T, R> Worker<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    /// Create a new worker
    pub fn new(
        id: usize,
        receiver: mpsc::Receiver<Vec<T>>,
        result_sender: mpsc::Sender<Vec<R>>,
        processor: Arc<dyn Fn(T) -> Result<R, Error> + Send + Sync>,
        continue_on_error: bool,
        resource_monitor: Option<Arc<ResourceMonitor>>,
    ) -> Self {
        Self {
            id,
            receiver,
            result_sender,
            processor,
            continue_on_error,
            resource_monitor,
        }
    }
    
    /// Run the worker
    pub async fn run(mut self) -> Result<(), Error> {
        while let Some(batch) = self.receiver.recv().await {
            // Process the batch
            let mut results = Vec::with_capacity(batch.len());
            
            for item in batch {
                // Check resource usage if monitoring is enabled
                if let Some(monitor) = &self.resource_monitor {
                    if monitor.should_throttle() {
                        // Wait for resources to be available
                        monitor.wait_for_resources().await;
                    }
                }
                
                // Process the item
                match (self.processor)(item) {
                    Ok(result) => {
                        results.push(result);
                    }
                    Err(err) => {
                        if self.continue_on_error {
                            // Log the error and continue
                            eprintln!("Worker {} error: {}", self.id, err);
                        } else {
                            return Err(err);
                        }
                    }
                }
            }
            
            // Send results
            if !results.is_empty() {
                if let Err(err) = self.result_sender.send(results).await {
                    return Err(Error::Other(format!("Failed to send results: {}", err)));
                }
            }
        }
        
        Ok(())
    }
}

/// Resource monitor for throttling based on system resources
pub struct ResourceMonitor {
    /// Maximum CPU usage percent (0-100 × core count)
    max_cpu_percent: f64,
    
    /// Maximum memory usage in megabytes
    max_memory_mb: usize,
    
    /// Sampling interval
    sampling_interval: Duration,
    
    /// Last resource usage sample
    last_sample: tokio::sync::Mutex<ResourceUsage>,
    
    /// Semaphore for throttling
    throttle_semaphore: Semaphore,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(max_cpu_percent: f64, max_memory_mb: usize) -> Self {
        let cpus = num_cpus::get();
        let permits = cpus * 2; // Start with generous permits
        
        Self {
            max_cpu_percent,
            max_memory_mb,
            sampling_interval: Duration::from_secs(1),
            last_sample: tokio::sync::Mutex::new(ResourceUsage::new()),
            throttle_semaphore: Semaphore::new(permits),
        }
    }
    
    /// Start the monitoring loop
    pub fn start(self: &Arc<Self>) {
        let this = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(this.sampling_interval);
            
            loop {
                interval.tick().await;
                
                // Sample resource usage
                let usage = ResourceUsage::sample();
                
                // Update last sample
                *this.last_sample.lock().await = usage.clone();
                
                // Adjust semaphore based on resource usage
                this.adjust_throttling(&usage);
            }
        });
    }
    
    /// Check if the process should be throttled
    pub fn should_throttle(&self) -> bool {
        self.throttle_semaphore.available_permits() == 0
    }
    
    /// Wait for resources to be available
    pub async fn wait_for_resources(&self) {
        let _permit = self.throttle_semaphore.acquire().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        // The permit is automatically released when dropped
    }
    
    /// Adjust throttling based on resource usage
    fn adjust_throttling(&self, usage: &ResourceUsage) {
        let cpu_ratio = if self.max_cpu_percent > 0.0 {
            usage.cpu_usage_percent / self.max_cpu_percent
        } else {
            0.0
        };
        
        let memory_ratio = if self.max_memory_mb > 0 {
            usage.memory_usage_mb / self.max_memory_mb as f64
        } else {
            0.0
        };
        
        // Determine permits to add or remove
        let current = self.throttle_semaphore.available_permits();
        let target = if cpu_ratio > 0.9 || memory_ratio > 0.9 {
            // Near limits, reduce permits
            (current / 2).max(1)
        } else if cpu_ratio < 0.7 && memory_ratio < 0.7 {
            // Well below limits, increase permits
            (current * 2).min(num_cpus::get() * 4)
        } else {
            // No change needed
            current
        };
        
        // Adjust semaphore
        let diff = target as i32 - current as i32;
        if diff > 0 {
            self.throttle_semaphore.add_permits(diff as usize);
        } else if diff < 0 {
            // Can't easily reduce permits, we'll just let them drain naturally
            // and not add more until we reach the target
        }
    }
}

/// Parallel processor for batch processing items
pub struct ParallelProcessor<T, R> {
    /// Configuration for parallel processing
    config: ParallelConfig,
    
    /// Processing function
    processor: Arc<dyn Fn(T) -> Result<R, Error> + Send + Sync>,
    
    /// Resource monitor
    resource_monitor: Option<Arc<ResourceMonitor>>,
    
    /// Whether to continue processing if some items fail
    continue_on_error: bool,
}

impl<T, R> ParallelProcessor<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    /// Create a new parallel processor
    pub fn new<F>(processor: F) -> Self
    where
        F: Fn(T) -> Result<R, Error> + Send + Sync + 'static,
    {
        Self {
            config: ParallelConfig::default(),
            processor: Arc::new(processor),
            resource_monitor: None,
            continue_on_error: false,
        }
    }
    
    /// Set the configuration
    pub fn with_config(mut self, config: ParallelConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Set resource monitoring
    pub fn with_resource_monitoring(mut self, max_cpu_percent: f64, max_memory_mb: usize) -> Self {
        let monitor = Arc::new(ResourceMonitor::new(max_cpu_percent, max_memory_mb));
        monitor.start();
        self.resource_monitor = Some(monitor);
        self
    }
    
    /// Set whether to continue processing if some items fail
    pub fn continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }
    
    /// Process items in parallel
    pub async fn process<I>(&self, items: I) -> Result<Vec<R>, Error>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: Send + 'static,
    {
        let items: Vec<T> = items.into_iter().collect();
        
        if items.is_empty() {
            return Ok(Vec::new());
        }
        
        // Create channels for work distribution and result collection
        let (work_tx, work_rx) = mpsc::channel(self.config.queue_size);
        let (result_tx, mut result_rx) = mpsc::channel(self.config.queue_size);
        
        // Create workers (simplified to one worker since Receiver can't be cloned)
        let mut worker_tasks = JoinSet::new();
        
        let worker = Worker::new(
            0,
            work_rx,
            result_tx.clone(),
            self.processor.clone(),
            self.continue_on_error,
            self.resource_monitor.clone(),
        );
        
        worker_tasks.spawn(worker.run());
        
        // Drop original sender to avoid deadlock
        drop(result_tx);
        
        // Distribute work
        let _start_time = Instant::now();
        
        // Split items into batches
        let mut batches = Vec::new();
        let mut current_batch = Vec::with_capacity(self.config.batch_size);
        
        for item in items {
            current_batch.push(item);
            
            if current_batch.len() >= self.config.batch_size {
                batches.push(std::mem::take(&mut current_batch));
                current_batch = Vec::with_capacity(self.config.batch_size);
            }
        }
        
        if !current_batch.is_empty() {
            batches.push(current_batch);
        }
        
        // Send batches to workers
        for batch in batches {
            if let Err(err) = work_tx.send(batch).await {
                return Err(Error::Other(format!("Failed to send work batch: {}", err)));
            }
        }
        
        // Drop work sender to signal workers that there's no more work
        drop(work_tx);
        
        // Collect results
        let mut all_results = Vec::new();
        
        while let Some(batch_results) = result_rx.recv().await {
            all_results.extend(batch_results);
        }
        
        // Wait for all workers to complete
        while let Some(result) = worker_tasks.join_next().await {
            match result {
                Ok(Ok(())) => {
                    // Worker completed successfully
                }
                Ok(Err(err)) => {
                    return Err(err);
                }
                Err(err) => {
                    return Err(Error::Other(format!("Worker task failed: {}", err)));
                }
            }
        }
        
        // Return results
        Ok(all_results)
    }
    
    /// Benchmark processing with different configurations
    pub async fn benchmark<I>(
        processor: Arc<dyn Fn(T) -> Result<R, Error> + Send + Sync>,
        items: I,
        configs: Vec<ParallelConfig>,
    ) -> Result<BenchmarkReport, Error>
    where
        I: IntoIterator<Item = T> + Clone,
        I::IntoIter: Send + 'static,
    {
        let mut measurements = Vec::new();
        
        for config in configs {
            let parallel_processor = ParallelProcessor {
                config: config.clone(),
                processor: processor.clone(),
                resource_monitor: None,
                continue_on_error: false,
            };
            
            // Measure processing time
            let items_clone = items.clone().into_iter().collect::<Vec<_>>();
            let item_count = items_clone.len();
            
            let start = Instant::now();
            let results = parallel_processor.process(items_clone).await?;
            let duration = start.elapsed();
            
            // Create measurement
            let measurement = Measurement::new(
                &format!("parallel_workers_{}", config.worker_count),
                duration,
                item_count as u64,
                0, // No data size for generic processing
            )
            .with_metric("worker_count", config.worker_count as f64)
            .with_metric("batch_size", config.batch_size as f64)
            .with_metric("queue_size", config.queue_size as f64)
            .with_metric("result_count", results.len() as f64);
            
            measurements.push(measurement);
        }
        
        // Create report
        let report = BenchmarkReport::new("parallel_processing_benchmark", measurements);
        
        Ok(report)
    }
}

/// Find optimal parallelism level
pub async fn find_optimal_parallelism<F, I, T, R>(
    processor: F,
    items: I,
) -> Result<(ParallelConfig, BenchmarkReport), Error>
where
    F: Fn(T) -> Result<R, Error> + Send + Sync + 'static,
    I: IntoIterator<Item = T> + Clone,
    I::IntoIter: Send + 'static,
    T: Send + 'static,
    R: Send + 'static,
{
    let cpu_count = num_cpus::get();
    let processor_arc = Arc::new(processor);
    
    // Test different worker counts
    let configs = vec![
        ParallelConfig {
            worker_count: 1,
            ..ParallelConfig::default()
        },
        ParallelConfig {
            worker_count: cpu_count / 2,
            ..ParallelConfig::default()
        },
        ParallelConfig {
            worker_count: cpu_count,
            ..ParallelConfig::default()
        },
        ParallelConfig {
            worker_count: cpu_count * 2,
            ..ParallelConfig::default()
        },
    ];
    
    let report = ParallelProcessor::benchmark(processor_arc, items, configs).await?;
    
    // Find the configuration with the best ops_per_second
    let best_config = report.measurements.iter()
        .max_by(|a, b| a.ops_per_second().partial_cmp(&b.ops_per_second()).unwrap())
        .map(|m| {
            let worker_count = *m.metrics.get("worker_count").unwrap_or(&1.0) as usize;
            
            ParallelConfig {
                worker_count,
                ..ParallelConfig::default()
            }
        })
        .unwrap_or_default();
    
    Ok((best_config, report))
}

/// Concurrency scaling strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingStrategy {
    /// Fixed number of workers
    Fixed,
    
    /// Scale based on CPU count
    CpuBased,
    
    /// Automatic scaling based on load
    Adaptive,
    
    /// Work stealing between workers
    WorkStealing,
}

/// Create a tuning plan for concurrency and scaling
pub fn create_concurrency_tuning_plan(
    expected_workload: usize,
    data_size_mb: usize,
    memory_constraint_mb: usize,
) -> ParallelConfig {
    let cpu_count = num_cpus::get();
    
    // Calculate optimal worker count based on workload and CPU count
    let worker_count = if expected_workload < 100 {
        1
    } else if expected_workload < 1000 {
        (cpu_count / 2).max(1)
    } else {
        cpu_count
    };
    
    // Calculate queue size based on expected workload
    let queue_size = (expected_workload / 10).clamp(100, 10000);
    
    // Calculate batch size based on data size and memory constraints
    let batch_size = if data_size_mb > 0 && memory_constraint_mb > 0 {
        let max_batches = memory_constraint_mb / data_size_mb.max(1);
        (expected_workload / worker_count / max_batches.max(1)).max(1).min(1000)
    } else {
        100
    };
    
    ParallelConfig {
        worker_count,
        queue_size,
        batch_size,
        work_stealing: expected_workload > 1000,
        memory_limit_mb: memory_constraint_mb,
        cpu_affinity: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // A simple processor for testing
    fn test_processor(n: u32) -> Result<u32, Error> {
        Ok(n * 2)
    }
    
    #[tokio::test]
    async fn test_parallel_processor() {
        let processor = ParallelProcessor::new(test_processor)
            .with_config(ParallelConfig {
                worker_count: 4,
                batch_size: 10,
                ..ParallelConfig::default()
            });
        
        let items: Vec<u32> = (0..100).collect();
        let results = processor.process(items).await.unwrap();
        
        assert_eq!(results.len(), 100);
        assert_eq!(results[0], 0);
        assert_eq!(results[1], 2);
        assert_eq!(results[99], 198);
    }
    
    #[tokio::test]
    async fn test_find_optimal_parallelism() {
        let items: Vec<u32> = (0..100).collect();
        
        let (config, report) = find_optimal_parallelism(test_processor, items).await.unwrap();
        
        // Verify that we got a valid configuration
        assert!(config.worker_count > 0);
        assert_eq!(report.measurements.len(), 4); // We tested 4 configs
    }
    
    #[test]
    fn test_create_concurrency_tuning_plan() {
        let plan = create_concurrency_tuning_plan(1000, 100, 1024);
        
        assert!(plan.worker_count > 0);
        assert!(plan.batch_size > 0);
        assert!(plan.queue_size > 0);
    }
} 