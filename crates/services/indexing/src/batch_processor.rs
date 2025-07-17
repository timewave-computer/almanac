// Enhanced batch processing system for high-throughput event processing
//
// This module provides sophisticated batch processing capabilities with dynamic
// batch sizing, parallel processing pools, and adaptive performance optimizations
// for processing large volumes of cross-chain events efficiently.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore, Mutex};
use tokio::time::sleep;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

use crate::types::{Event, UnifiedEvent, ProcessingResult};
use crate::{Error, Result};

/// Batch processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessorConfig {
    /// Initial batch size
    pub initial_batch_size: usize,
    
    /// Minimum batch size
    pub min_batch_size: usize,
    
    /// Maximum batch size
    pub max_batch_size: usize,
    
    /// Batch timeout (process even if not full)
    pub batch_timeout: Duration,
    
    /// Maximum concurrent batches
    pub max_concurrent_batches: usize,
    
    /// Worker thread count
    pub worker_threads: usize,
    
    /// Enable adaptive batch sizing
    pub adaptive_sizing: bool,
    
    /// Performance target (events per second)
    pub target_throughput: f64,
    
    /// Batch size adjustment factor
    pub size_adjustment_factor: f64,
    
    /// Memory pressure threshold (MB)
    pub memory_threshold_mb: usize,
    
    /// Enable batch compression
    pub enable_compression: bool,
    
    /// Retry configuration
    pub retry_config: BatchRetryConfig,
}

impl Default for BatchProcessorConfig {
    fn default() -> Self {
        Self {
            initial_batch_size: 100,
            min_batch_size: 10,
            max_batch_size: 1000,
            batch_timeout: Duration::from_secs(5),
            max_concurrent_batches: 8,
            worker_threads: 4,
            adaptive_sizing: true,
            target_throughput: 1000.0, // events per second
            size_adjustment_factor: 1.2,
            memory_threshold_mb: 500,
            enable_compression: false,
            retry_config: BatchRetryConfig::default(),
        }
    }
}

/// Batch retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRetryConfig {
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Initial retry delay
    pub initial_delay: Duration,
    
    /// Maximum retry delay
    pub max_delay: Duration,
    
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Enable jitter
    pub enable_jitter: bool,
}

impl Default for BatchRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            enable_jitter: true,
        }
    }
}

/// Batch processing metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetrics {
    /// Total batches processed
    pub total_batches: u64,
    
    /// Total events processed
    pub total_events: u64,
    
    /// Total processing time
    pub total_processing_time: Duration,
    
    /// Average batch size
    pub avg_batch_size: f64,
    
    /// Average processing time per batch
    pub avg_batch_time: Duration,
    
    /// Current throughput (events per second)
    pub current_throughput: f64,
    
    /// Peak throughput
    pub peak_throughput: f64,
    
    /// Failed batches
    pub failed_batches: u64,
    
    /// Retried batches
    pub retried_batches: u64,
    
    /// Current batch size
    pub current_batch_size: usize,
    
    /// Memory usage (MB)
    pub memory_usage_mb: f64,
    
    /// Active batches
    pub active_batches: usize,
    
    /// Queue depth
    pub queue_depth: usize,
}

/// Batch processing statistics
#[derive(Debug, Clone)]
struct BatchStats {
    /// Processing times for adaptive sizing
    processing_times: VecDeque<Duration>,
    
    /// Batch sizes for optimization
    batch_sizes: VecDeque<usize>,
    
    /// Throughput measurements
    throughput_samples: VecDeque<(Instant, u64)>,
}

/// Event batch for processing
#[derive(Debug)]
pub struct EventBatch {
    /// Batch ID
    pub id: String,
    
    /// Events in the batch
    pub events: Vec<Box<dyn Event>>,
    
    /// Batch creation time
    pub created_at: Instant,
    
    /// Batch priority
    pub priority: BatchPriority,
    
    /// Retry count
    pub retry_count: u32,
    
    /// Chain-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Batch priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BatchPriority {
    /// Low priority (historical data)
    Low = 1,
    
    /// Normal priority (regular events)
    Normal = 2,
    
    /// High priority (real-time events)
    High = 3,
    
    /// Critical priority (finality events)
    Critical = 4,
}

/// Batch processor trait for different processing strategies
#[async_trait::async_trait]
pub trait BatchProcessor: Send + Sync {
    /// Process a batch of events
    async fn process_batch(&self, batch: EventBatch) -> Result<BatchProcessingResult>;
    
    /// Get processor name
    fn name(&self) -> &str;
    
    /// Check if processor can handle the batch
    fn can_handle(&self, batch: &EventBatch) -> bool;
    
    /// Get processor metrics
    async fn get_metrics(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }
}

/// Batch processing result
#[derive(Debug, Clone)]
pub struct BatchProcessingResult {
    /// Batch ID
    pub batch_id: String,
    
    /// Successfully processed events
    pub successful_events: usize,
    
    /// Failed events
    pub failed_events: usize,
    
    /// Processing duration
    pub processing_duration: Duration,
    
    /// Results per event
    pub event_results: Vec<ProcessingResult>,
    
    /// Processing errors
    pub errors: Vec<Error>,
    
    /// Should retry failed events
    pub should_retry: bool,
}

/// Enhanced batch processor with dynamic optimization
pub struct EnhancedBatchProcessor {
    /// Configuration
    config: BatchProcessorConfig,
    
    /// Event queue
    event_queue: Arc<Mutex<VecDeque<Box<dyn Event>>>>,
    
    /// Batch processors by type
    processors: Arc<RwLock<HashMap<String, Arc<dyn BatchProcessor>>>>,
    
    /// Processing statistics
    stats: Arc<RwLock<BatchStats>>,
    
    /// Metrics
    metrics: Arc<RwLock<BatchMetrics>>,
    
    /// Semaphore for controlling concurrent batches
    batch_semaphore: Arc<Semaphore>,
    
    /// Shutdown signal
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    
    /// Processing task handles
    task_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl EnhancedBatchProcessor {
    /// Create a new enhanced batch processor
    pub fn new(config: BatchProcessorConfig) -> Self {
        let batch_semaphore = Arc::new(Semaphore::new(config.max_concurrent_batches));
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        
        let stats = BatchStats {
            processing_times: VecDeque::with_capacity(100),
            batch_sizes: VecDeque::with_capacity(100),
            throughput_samples: VecDeque::with_capacity(60), // 1 minute of samples
        };
        
        let metrics = BatchMetrics {
            total_batches: 0,
            total_events: 0,
            total_processing_time: Duration::from_millis(0),
            avg_batch_size: config.initial_batch_size as f64,
            avg_batch_time: Duration::from_millis(0),
            current_throughput: 0.0,
            peak_throughput: 0.0,
            failed_batches: 0,
            retried_batches: 0,
            current_batch_size: config.initial_batch_size,
            memory_usage_mb: 0.0,
            active_batches: 0,
            queue_depth: 0,
        };
        
        Self {
            config,
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            processors: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(stats)),
            metrics: Arc::new(RwLock::new(metrics)),
            batch_semaphore,
            shutdown_tx,
            task_handles: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Start the batch processor
    pub async fn start(&self) -> Result<()> {
        info!("Starting enhanced batch processor");
        
        // Start worker tasks
        let mut handles = self.task_handles.lock().await;
        
        // Start batch formation task
        handles.push(self.start_batch_formation_task().await);
        
        // Start metrics collection task
        handles.push(self.start_metrics_task().await);
        
        // Start adaptive sizing task
        if self.config.adaptive_sizing {
            handles.push(self.start_adaptive_sizing_task().await);
        }
        
        info!("Enhanced batch processor started with {} workers", self.config.worker_threads);
        Ok(())
    }
    
    /// Register a batch processor
    pub async fn register_processor(&self, processor: Arc<dyn BatchProcessor>) {
        let name = processor.name().to_string();
        self.processors.write().await.insert(name.clone(), processor);
        info!("Registered batch processor: {}", name);
    }
    
    /// Add events to the processing queue
    pub async fn queue_events(&self, events: Vec<Box<dyn Event>>) -> Result<()> {
        let mut queue = self.event_queue.lock().await;
        for event in events {
            queue.push_back(event);
        }
        
        // Update queue depth metric
        let queue_depth = queue.len();
        self.metrics.write().await.queue_depth = queue_depth;
        
        debug!("Queued events, queue depth: {}", queue_depth);
        Ok(())
    }
    
    /// Start batch formation task
    async fn start_batch_formation_task(&self) -> tokio::task::JoinHandle<()> {
        let event_queue = self.event_queue.clone();
        let processors = self.processors.clone();
        let stats = self.stats.clone();
        let metrics = self.metrics.clone();
        let batch_semaphore = self.batch_semaphore.clone();
        let config = self.config.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        tokio::spawn(async move {
            let mut last_batch_time = Instant::now();
            
            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(100)) => {
                        // Check if we should form a batch
                        let should_form_batch = {
                            let queue = event_queue.lock().await;
                            let current_batch_size = stats.read().await.batch_sizes.back()
                                .copied().unwrap_or(config.initial_batch_size);
                            
                            queue.len() >= current_batch_size ||
                            (queue.len() >= config.min_batch_size && 
                             last_batch_time.elapsed() >= config.batch_timeout)
                        };
                        
                        if should_form_batch {
                            match Self::form_and_process_batch(
                                &event_queue,
                                &processors,
                                &stats,
                                &metrics,
                                &batch_semaphore,
                                &config,
                            ).await {
                                Ok(_) => {
                                    last_batch_time = Instant::now();
                                }
                                Err(e) => {
                                    error!("Failed to form and process batch: {}", e);
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Batch formation task shutting down");
                        break;
                    }
                }
            }
        })
    }
    
    /// Form and process a batch
    async fn form_and_process_batch(
        event_queue: &Arc<Mutex<VecDeque<Box<dyn Event>>>>,
        processors: &Arc<RwLock<HashMap<String, Arc<dyn BatchProcessor>>>>,
        stats: &Arc<RwLock<BatchStats>>,
        metrics: &Arc<RwLock<BatchMetrics>>,
        batch_semaphore: &Arc<Semaphore>,
        config: &BatchProcessorConfig,
    ) -> Result<()> {
        // Acquire semaphore permit
        let _permit = batch_semaphore.acquire().await
            .map_err(|e| Error::generic(format!("Failed to acquire batch permit: {}", e)))?;
        
        // Form batch
        let batch = {
            let mut queue = event_queue.lock().await;
            let current_batch_size = stats.read().await.batch_sizes.back()
                .copied().unwrap_or(config.initial_batch_size);
            
            let batch_size = std::cmp::min(current_batch_size, queue.len());
            if batch_size == 0 {
                return Ok(());
            }
            
            let events: Vec<Box<dyn Event>> = (0..batch_size)
                .filter_map(|_| queue.pop_front())
                .collect();
            
            if events.is_empty() {
                return Ok(());
            }
            
            // Determine batch priority based on events
            let event_refs: Vec<&dyn Event> = events.iter().map(|e| e.as_ref()).collect();
            let priority = Self::determine_batch_priority(&event_refs);
            
            EventBatch {
                id: uuid::Uuid::new_v4().to_string(),
                events,
                created_at: Instant::now(),
                priority,
                retry_count: 0,
                metadata: HashMap::new(),
            }
        };
        
        // Update active batches count
        {
            let mut metrics_guard = metrics.write().await;
            metrics_guard.active_batches += 1;
            metrics_guard.queue_depth = event_queue.lock().await.len();
        }
        
        // Process batch
        let result = Self::process_batch_with_retry(
            batch,
            processors,
            config,
        ).await;
        
        // Update metrics
        match result {
            Ok(batch_result) => {
                Self::update_processing_stats(stats, metrics, &batch_result).await;
            }
            Err(e) => {
                error!("Batch processing failed: {}", e);
                let mut metrics_guard = metrics.write().await;
                metrics_guard.failed_batches += 1;
            }
        }
        
        // Decrement active batches count
        metrics.write().await.active_batches -= 1;
        
        Ok(())
    }
    
    /// Process batch with retry logic
    async fn process_batch_with_retry(
        batch: EventBatch,
        processors: &Arc<RwLock<HashMap<String, Arc<dyn BatchProcessor>>>>,
        _config: &BatchProcessorConfig,
    ) -> Result<BatchProcessingResult> {
        let batch_id = batch.id.clone();
        let mut _retry_count = batch.retry_count;
        
        // Find suitable processor once at the beginning
        let _processor = {
            let processors_guard = processors.read().await;
            let mut best_processor = None;
            
            for processor in processors_guard.values() {
                if processor.can_handle(&batch) {
                    best_processor = Some(processor.clone());
                    break;
                }
            }
            
            best_processor.ok_or_else(|| Error::generic("No suitable processor found"))?
        };
        
        // Try processing once, then use simpler retry mechanism for failures
        match _processor.process_batch(batch).await {
            Ok(result) => Ok(result),
            Err(e) => {
                warn!("Batch {} failed: {}", batch_id, e);
                // For retries, we would need a different approach since we can't clone the batch
                // This is a limitation of using trait objects
                Err(e)
            }
        }
    }
    
    /// Determine batch priority based on events
    fn determine_batch_priority(events: &[&dyn Event]) -> BatchPriority {
        let mut max_priority = BatchPriority::Low;
        
        for event in events {
            // Try to downcast to UnifiedEvent to access finality_status
            let priority = if let Some(unified_event) = event.as_any().downcast_ref::<UnifiedEvent>() {
                match unified_event.finality_status.as_str() {
                    "finalized" => BatchPriority::Critical,
                    "justified" => BatchPriority::High,
                    "safe" => BatchPriority::Normal,
                    _ => BatchPriority::Low,
                }
            } else {
                // Default priority for unknown event types
                BatchPriority::Normal
            };
            
            if priority > max_priority {
                max_priority = priority;
            }
        }
        
        max_priority
    }
    
    /// Update processing statistics
    async fn update_processing_stats(
        stats: &Arc<RwLock<BatchStats>>,
        metrics: &Arc<RwLock<BatchMetrics>>,
        result: &BatchProcessingResult,
    ) {
        let now = Instant::now();
        
        // Update stats
        {
            let mut stats_guard = stats.write().await;
            
            // Add processing time
            stats_guard.processing_times.push_back(result.processing_duration);
            if stats_guard.processing_times.len() > 100 {
                stats_guard.processing_times.pop_front();
            }
            
            // Add batch size
            let batch_size = result.successful_events + result.failed_events;
            stats_guard.batch_sizes.push_back(batch_size);
            if stats_guard.batch_sizes.len() > 100 {
                stats_guard.batch_sizes.pop_front();
            }
            
            // Add throughput sample
            stats_guard.throughput_samples.push_back((now, result.successful_events as u64));
            if stats_guard.throughput_samples.len() > 60 {
                stats_guard.throughput_samples.pop_front();
            }
        }
        
        // Update metrics
        {
            let mut metrics_guard = metrics.write().await;
            
            metrics_guard.total_batches += 1;
            metrics_guard.total_events += result.successful_events as u64;
            metrics_guard.total_processing_time += result.processing_duration;
            
            if result.failed_events > 0 {
                metrics_guard.failed_batches += 1;
            }
            
            if result.batch_id.contains("retry") {
                metrics_guard.retried_batches += 1;
            }
            
            // Calculate averages
            if metrics_guard.total_batches > 0 {
                metrics_guard.avg_batch_size = metrics_guard.total_events as f64 / metrics_guard.total_batches as f64;
                metrics_guard.avg_batch_time = metrics_guard.total_processing_time / metrics_guard.total_batches as u32;
            }
            
            // Calculate current throughput
            let stats_guard = stats.read().await;
            if let Some(oldest_sample) = stats_guard.throughput_samples.front() {
                let time_diff = now.duration_since(oldest_sample.0).as_secs_f64();
                if time_diff > 0.0 {
                    let total_events: u64 = stats_guard.throughput_samples.iter().map(|(_, count)| count).sum();
                    metrics_guard.current_throughput = total_events as f64 / time_diff;
                    
                    if metrics_guard.current_throughput > metrics_guard.peak_throughput {
                        metrics_guard.peak_throughput = metrics_guard.current_throughput;
                    }
                }
            }
        }
    }
    
    /// Start metrics collection task
    async fn start_metrics_task(&self) -> tokio::task::JoinHandle<()> {
        let metrics = self.metrics.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Update memory usage
                        if let Ok(memory_info) = sys_info::mem_info() {
                            let mut metrics_guard = metrics.write().await;
                            metrics_guard.memory_usage_mb = (memory_info.total - memory_info.free) as f64 / 1024.0;
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        })
    }
    
    /// Start adaptive sizing task
    async fn start_adaptive_sizing_task(&self) -> tokio::task::JoinHandle<()> {
        let stats = self.stats.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::adjust_batch_size(&stats, &metrics, &config).await;
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        })
    }
    
    /// Adjust batch size based on performance metrics
    async fn adjust_batch_size(
        stats: &Arc<RwLock<BatchStats>>,
        metrics: &Arc<RwLock<BatchMetrics>>,
        config: &BatchProcessorConfig,
    ) {
        let current_throughput = metrics.read().await.current_throughput;
        let target_throughput = config.target_throughput;
        
        if current_throughput > 0.0 {
            let performance_ratio = current_throughput / target_throughput;
            let mut new_batch_size = {
                let stats_guard = stats.read().await;
                stats_guard.batch_sizes.back().copied().unwrap_or(config.initial_batch_size)
            };
            
            if performance_ratio < 0.8 {
                // Performance below target, increase batch size
                new_batch_size = ((new_batch_size as f64) * config.size_adjustment_factor) as usize;
                new_batch_size = new_batch_size.min(config.max_batch_size);
                info!("Increasing batch size to {} (throughput: {:.1}/s)", new_batch_size, current_throughput);
            } else if performance_ratio > 1.5 {
                // Performance well above target, decrease batch size to reduce latency
                new_batch_size = ((new_batch_size as f64) / config.size_adjustment_factor) as usize;
                new_batch_size = new_batch_size.max(config.min_batch_size);
                info!("Decreasing batch size to {} (throughput: {:.1}/s)", new_batch_size, current_throughput);
            }
            
            // Update current batch size
            let mut metrics_guard = metrics.write().await;
            metrics_guard.current_batch_size = new_batch_size;
        }
    }
    
    /// Get current metrics
    pub async fn get_metrics(&self) -> BatchMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Smart batch formation based on event types and system resources
    pub async fn create_optimized_batches(&self) -> Result<Vec<EventBatch>> {
        let mut optimized_batches = Vec::new();
        let mut queue = self.event_queue.lock().await;
        
        if queue.is_empty() {
            return Ok(optimized_batches);
        }
        
        // Group events by type and chain for optimal batching
        let mut grouped_events: HashMap<String, Vec<Box<dyn Event>>> = HashMap::new();
        let mut processed_count = 0;
        let max_events_to_process = std::cmp::min(queue.len(), self.config.max_batch_size * 3);
        
        // Collect events and group them
        while processed_count < max_events_to_process && !queue.is_empty() {
            if let Some(event) = queue.pop_front() {
                let event_key = format!("{}_{}", event.chain(), event.event_type());
                grouped_events.entry(event_key).or_default().push(event);
                processed_count += 1;
            }
        }
        
        // Create optimized batches for each group
        for (group_key, mut events) in grouped_events {
            // Sort events by block number for optimal processing order
            events.sort_by_key(|e| e.block_number());
            
            // Calculate optimal batch size for this event type
            let optimal_size = self.calculate_optimal_batch_size(&events).await;
            
            // Split into optimally sized batches
            while !events.is_empty() {
                let batch_size = std::cmp::min(optimal_size, events.len());
                let batch_events = events.drain(0..batch_size).collect::<Vec<_>>();
                
                if !batch_events.is_empty() {
                    let event_refs: Vec<&dyn Event> = batch_events.iter().map(|e| e.as_ref()).collect();
                    let priority = Self::determine_batch_priority(&event_refs);
                    
                    let batch = EventBatch {
                        id: uuid::Uuid::new_v4().to_string(),
                        events: batch_events,
                        created_at: Instant::now(),
                        priority,
                        retry_count: 0,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("group_key".to_string(), serde_json::Value::String(group_key.clone()));
                            meta.insert("optimized".to_string(), serde_json::Value::Bool(true));
                            meta
                        },
                    };
                    
                    optimized_batches.push(batch);
                }
            }
        }
        
        // Sort batches by priority
        optimized_batches.sort_by_key(|batch| std::cmp::Reverse(batch.priority as u8));
        
        Ok(optimized_batches)
    }
    
    /// Calculate optimal batch size based on event characteristics and system resources
    async fn calculate_optimal_batch_size(&self, events: &[Box<dyn Event>]) -> usize {
        if events.is_empty() {
            return self.config.initial_batch_size;
        }
        
        // Get current system metrics
        let memory_usage = if let Ok(memory_info) = sys_info::mem_info() {
            (memory_info.total - memory_info.free) as f64 / 1024.0 / 1024.0 // MB
        } else {
            0.0
        };
        
        // Calculate base size from configuration
        let mut optimal_size = self.config.initial_batch_size;
        
        // Adjust based on memory pressure
        let memory_pressure_ratio = memory_usage / self.config.memory_threshold_mb as f64;
        if memory_pressure_ratio > 0.8 {
            // High memory pressure, reduce batch size
            let reduction_factor = (1.0_f64 - (memory_pressure_ratio - 0.8) * 2.0).max(0.3_f64);
            optimal_size = (optimal_size as f64 * reduction_factor) as usize;
        } else if memory_pressure_ratio < 0.5 {
            // Low memory pressure, can increase batch size
            optimal_size = (optimal_size as f64 * 1.5) as usize;
        }
        
        // Adjust based on event complexity (estimated by data size)
        let avg_event_complexity = events.iter()
            .map(|e| e.raw_data().len())
            .sum::<usize>() / events.len().max(1);
        
        if avg_event_complexity > 1000 {
            // Complex events, smaller batches
            optimal_size = optimal_size * 2 / 3;
        } else if avg_event_complexity < 100 {
            // Simple events, larger batches
            optimal_size = optimal_size * 3 / 2;
        }
        
        // Adjust based on current throughput
        let current_metrics = self.metrics.read().await;
        if current_metrics.current_throughput > 0.0 {
            let performance_ratio = current_metrics.current_throughput / self.config.target_throughput;
            if performance_ratio < 0.7 {
                // Underperforming, try larger batches
                optimal_size = (optimal_size as f64 * 1.3) as usize;
            } else if performance_ratio > 1.3 {
                // Over-performing, can use smaller batches for lower latency
                optimal_size = (optimal_size as f64 * 0.8) as usize;
            }
        }
        
        // Ensure within bounds
        optimal_size.clamp(self.config.min_batch_size, self.config.max_batch_size)
    }
    
    /// Process batch with atomic operations and rollback capability
    pub async fn process_batch_atomic(&self, batch: EventBatch) -> Result<BatchProcessingResult> {
        let batch_id = batch.id.clone();
        info!("Starting atomic processing for batch {}", batch_id);
        
        // Create atomic transaction context
        let mut transaction_context = AtomicTransactionContext::new(batch_id.clone());
        
        // Find suitable processor
        let _processor = {
            let processors_guard = self.processors.read().await;
            let mut best_processor = None;
            
            for proc in processors_guard.values() {
                if proc.can_handle(&batch) {
                    best_processor = Some(proc.clone());
                    break;
                }
            }
            
            best_processor.ok_or_else(|| Error::generic("No suitable processor found"))?
        };
        
        let start_time = Instant::now();
        let mut successful_events = 0;
        let mut failed_events = 0;
        let mut event_results = Vec::new();
        let mut errors = Vec::new();
        
        // Process events with rollback capability
        for (index, event) in batch.events.iter().enumerate() {
            // Create checkpoint for this event
            let checkpoint = transaction_context.create_checkpoint(format!("event_{}", index));
            
            match self.process_event_with_transaction(event.as_ref(), &mut transaction_context).await {
                Ok(result) => {
                    successful_events += 1;
                    event_results.push(result);
                    // Commit this event's changes
                    transaction_context.commit_checkpoint(checkpoint);
                }
                Err(e) => {
                    failed_events += 1;
                    errors.push(e.clone());
                    event_results.push(ProcessingResult {
                        success: false,
                        error: Some(e.to_string()),
                        metadata: HashMap::new(),
                    });
                    
                    // Rollback this event's changes
                    transaction_context.rollback_to_checkpoint(checkpoint).await?;
                    
                    // If we have a critical failure, rollback entire batch
                    if self.is_critical_error(&e) {
                        warn!("Critical error in batch {}, rolling back entire batch", batch_id);
                        transaction_context.rollback_all().await?;
                        break;
                    }
                }
            }
        }
        
        // Commit all successful changes
        if successful_events > 0 {
            transaction_context.commit_all().await?;
            info!("Committed {} events in batch {}", successful_events, batch_id);
        }
        
        let result = BatchProcessingResult {
            batch_id,
            successful_events,
            failed_events,
            processing_duration: start_time.elapsed(),
            event_results,
            errors,
            should_retry: failed_events > 0 && failed_events < batch.events.len(),
        };
        
        Ok(result)
    }
    
    /// Process a single event within a transaction context
    async fn process_event_with_transaction(
        &self,
        event: &dyn Event,
        transaction_context: &mut AtomicTransactionContext,
    ) -> Result<ProcessingResult> {
        // Record the operation for potential rollback
        transaction_context.record_operation(AtomicOperation {
            operation_type: OperationType::ProcessEvent,
            event_id: event.id().to_string(),
            data: serde_json::to_value(event.raw_data()).unwrap_or_default(),
        });
        
        // Simulate event processing (in real implementation, this would be the actual processing)
        sleep(Duration::from_millis(1)).await;
        
        Ok(ProcessingResult {
            success: true,
            error: None,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("transaction_id".to_string(), serde_json::Value::String(transaction_context.transaction_id.clone()));
                meta
            },
        })
    }
    
    /// Check if an error is critical enough to rollback the entire batch
    fn is_critical_error(&self, error: &Error) -> bool {
        // Define criteria for critical errors that require full rollback
        let error_str = error.to_string().to_lowercase();
        error_str.contains("corruption") || 
        error_str.contains("integrity") ||
        error_str.contains("deadlock") ||
        error_str.contains("timeout")
    }
    
    /// Process multiple batches in parallel with load balancing
    pub async fn process_batches_parallel(&self, batches: Vec<EventBatch>) -> Result<Vec<BatchProcessingResult>> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_batches));
        let mut handles = Vec::new();
        
        for batch in batches {
            let semaphore_clone = semaphore.clone();
            let self_clone = self.clone(); // Assuming we implement Clone
            
            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await
                    .map_err(|e| Error::generic(format!("Failed to acquire semaphore: {}", e)))?;
                
                self_clone.process_batch_atomic(batch).await
            });
            
            handles.push(handle);
        }
        
        // Wait for all batches to complete
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(Error::generic(format!("Task join error: {}", e))),
            }
        }
        
        Ok(results)
    }
    
    /// Get optimization recommendations based on current performance
    pub async fn get_optimization_recommendations(&self) -> Vec<BatchOptimizationRecommendation> {
        let mut recommendations = Vec::new();
        let metrics = self.metrics.read().await;
        let stats = self.stats.read().await;
        
        // Analyze current performance
        let _avg_processing_time = if !stats.processing_times.is_empty() {
            stats.processing_times.iter().sum::<Duration>() / stats.processing_times.len() as u32
        } else {
            Duration::default()
        };
        
        // Check if batch sizes are optimal
        if let Some(&current_batch_size) = stats.batch_sizes.back() {
            if current_batch_size < self.config.min_batch_size * 2 {
                recommendations.push(BatchOptimizationRecommendation {
                    recommendation_type: OptimizationType::IncreaseBatchSize,
                    description: "Consider increasing batch size to improve throughput".to_string(),
                    expected_improvement: 15.0,
                    priority: OptimizationPriority::Medium,
                });
            } else if current_batch_size > self.config.max_batch_size * 3 / 4 {
                recommendations.push(BatchOptimizationRecommendation {
                    recommendation_type: OptimizationType::DecreaseBatchSize,
                    description: "Consider decreasing batch size to reduce latency".to_string(),
                    expected_improvement: 10.0,
                    priority: OptimizationPriority::Low,
                });
            }
        }
        
        // Check memory usage
        if metrics.memory_usage_mb > self.config.memory_threshold_mb as f64 * 0.8 {
            recommendations.push(BatchOptimizationRecommendation {
                recommendation_type: OptimizationType::ReduceMemoryUsage,
                description: "High memory usage detected, consider enabling compression or reducing batch sizes".to_string(),
                expected_improvement: 20.0,
                priority: OptimizationPriority::High,
            });
        }
        
        // Check throughput efficiency
        if metrics.current_throughput > 0.0 && metrics.current_throughput < self.config.target_throughput * 0.7 {
            recommendations.push(BatchOptimizationRecommendation {
                recommendation_type: OptimizationType::IncreaseParallelism,
                description: "Low throughput detected, consider increasing concurrent batch processing".to_string(),
                expected_improvement: 25.0,
                priority: OptimizationPriority::High,
            });
        }
        
        recommendations
    }
    
    /// Shutdown the batch processor
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down enhanced batch processor");
        
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());
        
        // Wait for tasks to complete
        let mut handles = self.task_handles.lock().await;
        for handle in handles.drain(..) {
            if let Err(e) = handle.await {
                warn!("Task shutdown error: {}", e);
            }
        }
        
        info!("Enhanced batch processor shut down");
        Ok(())
    }
}

/// Default event batch processor implementation
pub struct DefaultEventProcessor {
    name: String,
}

impl DefaultEventProcessor {
    pub fn new() -> Self {
        Self {
            name: "default_event_processor".to_string(),
        }
    }
}

impl Default for DefaultEventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl BatchProcessor for DefaultEventProcessor {
    async fn process_batch(&self, batch: EventBatch) -> Result<BatchProcessingResult> {
        let start_time = Instant::now();
        let mut successful_events = 0;
        let mut failed_events = 0;
        let mut event_results = Vec::new();
        let mut errors = Vec::new();
        
        for event in &batch.events {
            // Simulate event processing
            match self.process_single_event(event.as_ref()).await {
                Ok(result) => {
                    successful_events += 1;
                    event_results.push(result);
                }
                Err(e) => {
                    failed_events += 1;
                    errors.push(e);
                    event_results.push(ProcessingResult {
                        success: false,
                        error: Some("Processing failed".to_string()),
                        metadata: HashMap::new(),
                    });
                }
            }
        }
        
        Ok(BatchProcessingResult {
            batch_id: batch.id,
            successful_events,
            failed_events,
            processing_duration: start_time.elapsed(),
            event_results,
            errors,
            should_retry: failed_events > 0 && batch.retry_count < 2,
        })
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn can_handle(&self, _batch: &EventBatch) -> bool {
        true // Can handle any batch
    }
}

impl DefaultEventProcessor {
    async fn process_single_event(&self, _event: &dyn Event) -> Result<ProcessingResult> {
        // Simulate processing time
        sleep(Duration::from_millis(1)).await;
        
        Ok(ProcessingResult {
            success: true,
            error: None,
            metadata: HashMap::new(),
        })
    }
}

/// Atomic transaction context for batch processing with rollback capability
#[derive(Debug)]
pub struct AtomicTransactionContext {
    /// Transaction identifier
    pub transaction_id: String,
    /// Operations performed in this transaction
    operations: Vec<AtomicOperation>,
    /// Checkpoints for partial rollback
    checkpoints: HashMap<String, usize>,
    /// Whether the transaction is committed
    is_committed: bool,
}

impl AtomicTransactionContext {
    /// Create a new transaction context
    pub fn new(transaction_id: String) -> Self {
        Self {
            transaction_id,
            operations: Vec::new(),
            checkpoints: HashMap::new(),
            is_committed: false,
        }
    }
    
    /// Record an operation for potential rollback
    pub fn record_operation(&mut self, operation: AtomicOperation) {
        self.operations.push(operation);
    }
    
    /// Create a checkpoint
    pub fn create_checkpoint(&mut self, checkpoint_name: String) -> String {
        let checkpoint_id = format!("{}_{}", checkpoint_name, self.operations.len());
        self.checkpoints.insert(checkpoint_id.clone(), self.operations.len());
        checkpoint_id
    }
    
    /// Commit changes up to a checkpoint
    pub fn commit_checkpoint(&mut self, _checkpoint_id: String) {
        // In a real implementation, this would commit database changes up to the checkpoint
        // For now, we just mark it as processed
    }
    
    /// Rollback to a specific checkpoint
    pub async fn rollback_to_checkpoint(&mut self, checkpoint_id: String) -> Result<()> {
        if let Some(&checkpoint_index) = self.checkpoints.get(&checkpoint_id) {
            // Remove operations after the checkpoint
            self.operations.truncate(checkpoint_index);
            // In a real implementation, this would undo database operations
            Ok(())
        } else {
            Err(Error::generic(format!("Checkpoint {} not found", checkpoint_id)))
        }
    }
    
    /// Commit all operations
    pub async fn commit_all(&mut self) -> Result<()> {
        // In a real implementation, this would commit all pending database changes
        self.is_committed = true;
        Ok(())
    }
    
    /// Rollback all operations
    pub async fn rollback_all(&mut self) -> Result<()> {
        // In a real implementation, this would undo all database operations
        self.operations.clear();
        self.checkpoints.clear();
        Ok(())
    }
}

/// Atomic operation that can be rolled back
#[derive(Debug, Clone)]
pub struct AtomicOperation {
    /// Type of operation
    pub operation_type: OperationType,
    /// Event ID being processed
    pub event_id: String,
    /// Operation data
    pub data: serde_json::Value,
}

/// Types of atomic operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    ProcessEvent,
    UpdateState,
    InsertRecord,
    UpdateRecord,
    DeleteRecord,
}

/// Batch optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOptimizationRecommendation {
    /// Type of optimization
    pub recommendation_type: OptimizationType,
    /// Description of the recommendation
    pub description: String,
    /// Expected performance improvement percentage
    pub expected_improvement: f64,
    /// Priority of this recommendation
    pub priority: OptimizationPriority,
}

/// Types of batch optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    IncreaseBatchSize,
    DecreaseBatchSize,
    IncreaseParallelism,
    DecreaseParallelism,
    ReduceMemoryUsage,
    EnableCompression,
    OptimizeEventGrouping,
}

/// Priority levels for optimization recommendations
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OptimizationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Clone implementation for EnhancedBatchProcessor (needed for parallel processing)
impl Clone for EnhancedBatchProcessor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            event_queue: self.event_queue.clone(),
            processors: self.processors.clone(),
            stats: self.stats.clone(),
            metrics: self.metrics.clone(),
            batch_semaphore: self.batch_semaphore.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
            task_handles: self.task_handles.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{UnifiedEvent, EventSource};
    use std::time::SystemTime;
    
    #[tokio::test]
    async fn test_batch_processor_creation() {
        let config = BatchProcessorConfig::default();
        let processor = EnhancedBatchProcessor::new(config);
        
        let metrics = processor.get_metrics().await;
        assert_eq!(metrics.total_batches, 0);
        assert_eq!(metrics.total_events, 0);
    }
    
    #[tokio::test]
    async fn test_event_queuing() {
        let config = BatchProcessorConfig::default();
        let processor = EnhancedBatchProcessor::new(config);
        
        let events: Vec<Box<dyn Event>> = vec![
            Box::new(UnifiedEvent {
                id: "test1".to_string(),
                chain_id: "ethereum".to_string(),
                block_number: 100,
                transaction_hash: "0x123".to_string(),
                log_index: 0,
                contract_address: "0xabc".to_string(),
                event_name: "Transfer".to_string(),
                event_data: serde_json::json!({}),
                finality_status: "confirmed".to_string(),
                timestamp: SystemTime::now(),
                source: EventSource::Chain,
            })
        ];
        
        assert!(processor.queue_events(events).await.is_ok());
        
        let metrics = processor.get_metrics().await;
        assert_eq!(metrics.queue_depth, 1);
    }
    
    #[tokio::test]
    async fn test_batch_priority_determination() {
        let event = UnifiedEvent {
            id: "test1".to_string(),
            chain_id: "ethereum".to_string(),
            block_number: 100,
            transaction_hash: "0x123".to_string(),
            log_index: 0,
            contract_address: "0xabc".to_string(),
            event_name: "Transfer".to_string(),
            event_data: serde_json::json!({}),
            finality_status: "finalized".to_string(),
            timestamp: SystemTime::now(),
            source: EventSource::Chain,
        };
        
        let events: Vec<&dyn Event> = vec![&event];
        let priority = EnhancedBatchProcessor::determine_batch_priority(&events);
        assert_eq!(priority, BatchPriority::Critical);
    }
    
    #[tokio::test]
    async fn test_optimized_batch_creation() {
        let config = BatchProcessorConfig::default();
        let processor = EnhancedBatchProcessor::new(config);
        
        // Queue events from different chains and types
        let events: Vec<Box<dyn Event>> = vec![
            Box::new(UnifiedEvent {
                id: "eth1".to_string(),
                chain_id: "ethereum".to_string(),
                block_number: 100,
                transaction_hash: "0x123".to_string(),
                log_index: 0,
                contract_address: "0xabc".to_string(),
                event_name: "Transfer".to_string(),
                event_data: serde_json::json!({}),
                finality_status: "confirmed".to_string(),
                timestamp: SystemTime::now(),
                source: EventSource::Chain,
            }),
            Box::new(UnifiedEvent {
                id: "poly1".to_string(),
                chain_id: "polygon".to_string(),
                block_number: 200,
                transaction_hash: "0x456".to_string(),
                log_index: 0,
                contract_address: "0xdef".to_string(),
                event_name: "Transfer".to_string(),
                event_data: serde_json::json!({}),
                finality_status: "confirmed".to_string(),
                timestamp: SystemTime::now(),
                source: EventSource::Chain,
            }),
        ];
        
        processor.queue_events(events).await.unwrap();
        
        let optimized_batches = processor.create_optimized_batches().await.unwrap();
        assert!(!optimized_batches.is_empty());
        
        // Should group by chain and event type
        for batch in &optimized_batches {
            assert!(batch.metadata.contains_key("group_key"));
            assert!(batch.metadata.contains_key("optimized"));
        }
    }
    
    #[tokio::test]
    async fn test_atomic_transaction_context() {
        let mut context = AtomicTransactionContext::new("test_tx".to_string());
        
        // Record an operation
        context.record_operation(AtomicOperation {
            operation_type: OperationType::ProcessEvent,
            event_id: "event1".to_string(),
            data: serde_json::json!({"test": "data"}),
        });
        
        // Create checkpoint
        let checkpoint = context.create_checkpoint("test_checkpoint".to_string());
        assert!(!checkpoint.is_empty());
        
        // Add another operation
        context.record_operation(AtomicOperation {
            operation_type: OperationType::UpdateRecord,
            event_id: "event2".to_string(),
            data: serde_json::json!({"test": "data2"}),
        });
        
        assert_eq!(context.operations.len(), 2);
        
        // Rollback to checkpoint
        context.rollback_to_checkpoint(checkpoint).await.unwrap();
        assert_eq!(context.operations.len(), 1);
        
        // Commit all
        context.commit_all().await.unwrap();
        assert!(context.is_committed);
    }
    
    #[tokio::test]
    async fn test_batch_size_optimization() {
        let config = BatchProcessorConfig::default();
        let processor = EnhancedBatchProcessor::new(config);
        
        // Create test events of different complexities
        let simple_events: Vec<Box<dyn Event>> = vec![
            Box::new(UnifiedEvent {
                id: "simple1".to_string(),
                chain_id: "ethereum".to_string(),
                block_number: 100,
                transaction_hash: "0x123".to_string(),
                log_index: 0,
                contract_address: "0xabc".to_string(),
                event_name: "Transfer".to_string(),
                event_data: serde_json::json!({"small": "data"}),
                finality_status: "confirmed".to_string(),
                timestamp: SystemTime::now(),
                source: EventSource::Chain,
            }),
        ];
        
        let optimal_size = processor.calculate_optimal_batch_size(&simple_events).await;
        assert!(optimal_size >= processor.config.min_batch_size);
        assert!(optimal_size <= processor.config.max_batch_size);
    }
    
    #[tokio::test]
    async fn test_optimization_recommendations() {
        let config = BatchProcessorConfig::default();
        let processor = EnhancedBatchProcessor::new(config);
        
        // Get initial recommendations
        let recommendations = processor.get_optimization_recommendations().await;
        
        // Should provide some recommendations based on default state
        for recommendation in &recommendations {
            assert!(!recommendation.description.is_empty());
            assert!(recommendation.expected_improvement >= 0.0);
        }
    }
} 