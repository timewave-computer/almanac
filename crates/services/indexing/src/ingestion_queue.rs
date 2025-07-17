// Event ingestion queue with buffered processing and backpressure handling
//
// This module implements an async event ingestion queue that provides buffered
// event processing with configurable buffer sizes, rate limiting, and backpressure
// mechanisms to ensure system stability under high load.

use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::{interval, Instant};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::Event;
use crate::{Error, Result};

/// Configuration for the event ingestion queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionQueueConfig {
    /// Maximum number of events to buffer per chain
    pub max_buffer_size: usize,
    
    /// Maximum number of events to process per second per chain
    pub max_events_per_second: u64,
    
    /// Backpressure threshold as percentage of buffer size (0.0 to 1.0)
    pub backpressure_threshold: f64,
    
    /// Maximum time to wait for buffer space when backpressure is active
    pub backpressure_timeout: Duration,
    
    /// Batch size for processing events
    pub batch_size: usize,
    
    /// Minimum time between batches
    pub batch_interval: Duration,
    
    /// Maximum time to wait for a batch to fill
    pub batch_timeout: Duration,
    
    /// Enable metrics collection
    pub enable_metrics: bool,
    
    /// Graceful shutdown timeout
    pub shutdown_timeout: Duration,
    
    /// Per-chain configurations
    pub chain_configs: HashMap<String, ChainIngestionConfig>,
}

/// Per-chain ingestion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainIngestionConfig {
    /// Chain identifier
    pub chain_id: String,
    
    /// Maximum buffer size for this chain
    pub max_buffer_size: usize,
    
    /// Maximum events per second for this chain
    pub max_events_per_second: u64,
    
    /// Priority for this chain (higher = more priority)
    pub priority: u8,
    
    /// Enable rate limiting for this chain
    pub enable_rate_limiting: bool,
    
    /// Enable backpressure for this chain
    pub enable_backpressure: bool,
}

impl Default for IngestionQueueConfig {
    fn default() -> Self {
        let mut chain_configs = HashMap::new();
        
        // Default Ethereum configuration
        chain_configs.insert("ethereum".to_string(), ChainIngestionConfig {
            chain_id: "ethereum".to_string(),
            max_buffer_size: 10_000,
            max_events_per_second: 1000,
            priority: 100,
            enable_rate_limiting: true,
            enable_backpressure: true,
        });
        
        // Default Polygon configuration
        chain_configs.insert("polygon".to_string(), ChainIngestionConfig {
            chain_id: "polygon".to_string(),
            max_buffer_size: 50_000,
            max_events_per_second: 5000,
            priority: 90,
            enable_rate_limiting: true,
            enable_backpressure: true,
        });
        
        // Default Base configuration
        chain_configs.insert("base".to_string(), ChainIngestionConfig {
            chain_id: "base".to_string(),
            max_buffer_size: 10_000,
            max_events_per_second: 1000,
            priority: 80,
            enable_rate_limiting: true,
            enable_backpressure: true,
        });
        
        // Default Osmosis configuration
        chain_configs.insert("osmosis".to_string(), ChainIngestionConfig {
            chain_id: "osmosis".to_string(),
            max_buffer_size: 5_000,
            max_events_per_second: 500,
            priority: 70,
            enable_rate_limiting: true,
            enable_backpressure: true,
        });
        
        // Default Noble configuration
        chain_configs.insert("noble".to_string(), ChainIngestionConfig {
            chain_id: "noble".to_string(),
            max_buffer_size: 1_000,
            max_events_per_second: 100,
            priority: 60,
            enable_rate_limiting: true,
            enable_backpressure: true,
        });
        
        // Default Neutron configuration
        chain_configs.insert("neutron".to_string(), ChainIngestionConfig {
            chain_id: "neutron".to_string(),
            max_buffer_size: 2_000,
            max_events_per_second: 200,
            priority: 50,
            enable_rate_limiting: true,
            enable_backpressure: true,
        });
        
        Self {
            max_buffer_size: 10_000,
            max_events_per_second: 1000,
            backpressure_threshold: 0.8,
            backpressure_timeout: Duration::from_secs(10),
            batch_size: 100,
            batch_interval: Duration::from_millis(50),
            batch_timeout: Duration::from_secs(1),
            enable_metrics: true,
            shutdown_timeout: Duration::from_secs(30),
            chain_configs,
        }
    }
}

/// Event ingestion statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionStats {
    /// Total events ingested
    pub total_ingested: u64,
    
    /// Total events processed
    pub total_processed: u64,
    
    /// Total events dropped due to backpressure
    pub total_dropped: u64,
    
    /// Current queue sizes per chain
    pub queue_sizes: HashMap<String, usize>,
    
    /// Events per second per chain
    pub events_per_second: HashMap<String, f64>,
    
    /// Average processing time per batch
    pub avg_batch_processing_time: Duration,
    
    /// Number of active backpressure situations
    pub active_backpressure: HashMap<String, bool>,
    
    /// Last processing time
    pub last_processed_at: Option<SystemTime>,
}

/// Event ingestion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IngestionResult {
    /// Event was successfully queued
    Queued,
    
    /// Event was queued but backpressure is active
    QueuedWithBackpressure,
    
    /// Event was dropped due to full buffer
    Dropped(String),
    
    /// Event was rejected due to rate limiting
    RateLimited,
    
    /// Event was rejected due to invalid configuration
    InvalidConfig(String),
}

/// Event processor trait for ingestion queue
#[async_trait]
pub trait EventIngestionProcessor: Send + Sync {
    /// Process a batch of events
    async fn process_batch(&self, events: Vec<Box<dyn Event>>) -> Result<()>;
    
    /// Handle backpressure situation
    async fn handle_backpressure(&self, chain_id: &str, queue_size: usize) -> Result<()>;
    
    /// Handle dropped events
    async fn handle_dropped_events(&self, chain_id: &str, events: Vec<Box<dyn Event>>) -> Result<()>;
    
    /// Get processing statistics
    async fn get_stats(&self) -> Result<HashMap<String, serde_json::Value>>;
}

/// Type alias for event buffers to reduce complexity
type EventBuffers = Arc<RwLock<HashMap<String, Vec<Box<dyn Event>>>>>;

/// Event ingestion queue with advanced rate limiting and backpressure
pub struct EventIngestionQueue {
    /// Configuration
    config: IngestionQueueConfig,
    
    /// Per-chain event buffers
    buffers: EventBuffers,
    
    /// Per-chain rate limiters
    rate_limiters: Arc<RwLock<HashMap<String, RateLimiter>>>,
    
    /// Per-chain backpressure status
    backpressure_status: Arc<RwLock<HashMap<String, bool>>>,
    
    /// Event processor
    processor: Arc<dyn EventIngestionProcessor>,
    
    /// Ingestion statistics
    stats: Arc<RwLock<IngestionStats>>,
    
    /// Shutdown signal
    shutdown_signal: Arc<tokio::sync::Notify>,
    
    /// Channel for ingestion events
    event_sender: mpsc::UnboundedSender<IngestionEvent>,
    
    /// Processing permits (for rate limiting)
    processing_permits: Arc<Semaphore>,
}

/// Internal ingestion event
#[derive(Debug)]
enum IngestionEvent {
    ProcessBatch {
        chain_id: String,
    },
    Shutdown,
}

/// Rate limiter for event ingestion
#[derive(Debug)]
struct RateLimiter {
    /// Maximum events per second
    max_events_per_second: u64,
    
    /// Tokens available
    tokens: Arc<AtomicU64>,
    
    /// Last refill time
    last_refill: Arc<RwLock<Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    fn new(max_events_per_second: u64) -> Self {
        Self {
            max_events_per_second,
            tokens: Arc::new(AtomicU64::new(max_events_per_second)),
            last_refill: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    /// Try to acquire a token
    async fn try_acquire(&self) -> bool {
        // Refill tokens based on elapsed time
        self.refill_tokens().await;
        
        // Try to acquire a token
        let current_tokens = self.tokens.load(Ordering::Relaxed);
        if current_tokens > 0 {
            self.tokens.fetch_sub(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
    
    /// Refill tokens based on elapsed time
    async fn refill_tokens(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.write().await;
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        
        if elapsed >= 1.0 {
            let tokens_to_add = (elapsed * self.max_events_per_second as f64) as u64;
            let current_tokens = self.tokens.load(Ordering::Relaxed);
            let new_tokens = std::cmp::min(current_tokens + tokens_to_add, self.max_events_per_second);
            
            self.tokens.store(new_tokens, Ordering::Relaxed);
            *last_refill = now;
        }
    }
}

impl EventIngestionQueue {
    /// Create a new event ingestion queue
    pub fn new(
        config: IngestionQueueConfig,
        processor: Arc<dyn EventIngestionProcessor>,
    ) -> Result<Self> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        // Initialize rate limiters
        let mut rate_limiters = HashMap::new();
        for (chain_id, chain_config) in &config.chain_configs {
            if chain_config.enable_rate_limiting {
                rate_limiters.insert(
                    chain_id.clone(),
                    RateLimiter::new(chain_config.max_events_per_second),
                );
            }
        }
        
        let processing_permits = Arc::new(Semaphore::new(config.batch_size * 10));
        
        let queue = Self {
            config,
            buffers: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(rate_limiters)),
            backpressure_status: Arc::new(RwLock::new(HashMap::new())),
            processor,
            stats: Arc::new(RwLock::new(IngestionStats {
                total_ingested: 0,
                total_processed: 0,
                total_dropped: 0,
                queue_sizes: HashMap::new(),
                events_per_second: HashMap::new(),
                avg_batch_processing_time: Duration::from_millis(0),
                active_backpressure: HashMap::new(),
                last_processed_at: None,
            })),
            shutdown_signal: Arc::new(tokio::sync::Notify::new()),
            event_sender,
            processing_permits,
        };
        
        // Start background processing
        let queue_clone = queue.clone();
        tokio::spawn(async move {
            queue_clone.process_events(event_receiver).await;
        });
        
        // Start batch processing for each chain
        for chain_id in queue.config.chain_configs.keys() {
            let queue_clone = queue.clone();
            let chain_id_clone = chain_id.clone();
            tokio::spawn(async move {
                queue_clone.batch_processor(chain_id_clone).await;
            });
        }
        
        // Start stats updater
        let queue_clone = queue.clone();
        tokio::spawn(async move {
            queue_clone.stats_updater().await;
        });
        
        Ok(queue)
    }
    
    /// Ingest an event
    pub async fn ingest_event(&self, chain_id: &str, event: Box<dyn Event>) -> Result<IngestionResult> {
        // Get chain configuration
        let chain_config = self.config.chain_configs.get(chain_id);
        if chain_config.is_none() {
            return Ok(IngestionResult::InvalidConfig(format!("Unknown chain: {}", chain_id)));
        }
        let chain_config = chain_config.unwrap();
        
        // Check rate limiting
        if chain_config.enable_rate_limiting {
            let rate_limiters = self.rate_limiters.read().await;
            if let Some(rate_limiter) = rate_limiters.get(chain_id) {
                if !rate_limiter.try_acquire().await {
                    return Ok(IngestionResult::RateLimited);
                }
            }
        }
        
        // Check backpressure
        if chain_config.enable_backpressure {
            let buffers = self.buffers.read().await;
            let current_size = buffers.get(chain_id).map(|b| b.len()).unwrap_or(0);
            let threshold = (chain_config.max_buffer_size as f64 * self.config.backpressure_threshold) as usize;
            
            if current_size >= threshold {
                // Backpressure active
                let backpressure_status = self.backpressure_status.read().await;
                let is_backpressure_active = backpressure_status.get(chain_id).copied().unwrap_or(false);
                
                if current_size >= chain_config.max_buffer_size {
                    // Buffer is full, drop event
                    self.update_stats_dropped(chain_id).await;
                    return Ok(IngestionResult::Dropped("Buffer full".to_string()));
                }
                
                if !is_backpressure_active {
                    // Activate backpressure
                    self.activate_backpressure(chain_id).await;
                }
                
                // Try to queue with backpressure
                self.queue_event(chain_id, event).await?;
                return Ok(IngestionResult::QueuedWithBackpressure);
            }
        }
        
        // Normal queuing
        self.queue_event(chain_id, event).await?;
        Ok(IngestionResult::Queued)
    }
    
    /// Queue an event
    async fn queue_event(&self, chain_id: &str, event: Box<dyn Event>) -> Result<()> {
        let mut buffers = self.buffers.write().await;
        let buffer = buffers.entry(chain_id.to_string()).or_default();
        buffer.push(event);
        
        // Send processing signal
        self.event_sender.send(IngestionEvent::ProcessBatch {
            chain_id: chain_id.to_string(),
        }).map_err(|e| Error::generic(format!("Failed to send processing signal: {}", e)))?;
        
        // Update ingestion stats
        self.update_stats_ingested(chain_id).await;
        
        Ok(())
    }
    
    /// Activate backpressure for a chain
    async fn activate_backpressure(&self, chain_id: &str) {
        let mut backpressure_status = self.backpressure_status.write().await;
        backpressure_status.insert(chain_id.to_string(), true);
        
        // Notify processor
        if let Err(e) = self.processor.handle_backpressure(chain_id, 
            self.buffers.read().await.get(chain_id).map(|b| b.len()).unwrap_or(0)
        ).await {
            tracing::error!("Failed to handle backpressure: {}", e);
        }
    }
    
    /// Deactivate backpressure for a chain
    async fn deactivate_backpressure(&self, chain_id: &str) {
        let mut backpressure_status = self.backpressure_status.write().await;
        backpressure_status.insert(chain_id.to_string(), false);
    }
    
    /// Background event processing
    async fn process_events(&self, mut event_receiver: mpsc::UnboundedReceiver<IngestionEvent>) {
        while let Some(event) = event_receiver.recv().await {
            match event {
                IngestionEvent::ProcessBatch { chain_id } => {
                    self.process_batch_for_chain(&chain_id).await;
                }
                IngestionEvent::Shutdown => {
                    break;
                }
            }
        }
    }
    
    /// Batch processor for a specific chain
    async fn batch_processor(&self, chain_id: String) {
        let mut interval = interval(self.config.batch_interval);
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.process_batch_for_chain(&chain_id).await;
                }
                _ = self.shutdown_signal.notified() => {
                    // Final batch processing before shutdown
                    self.process_batch_for_chain(&chain_id).await;
                    break;
                }
            }
        }
    }
    
    /// Process a batch of events for a specific chain
    async fn process_batch_for_chain(&self, chain_id: &str) {
        let _permit = self.processing_permits.acquire().await.unwrap();
        
        let batch = {
            let mut buffers = self.buffers.write().await;
            let buffer = buffers.entry(chain_id.to_string()).or_default();
            
            if buffer.is_empty() {
                return;
            }
            
            let batch_size = std::cmp::min(buffer.len(), self.config.batch_size);
            buffer.drain(0..batch_size).collect::<Vec<_>>()
        };
        
        if batch.is_empty() {
            return;
        }
        
        let start_time = Instant::now();
        
        // Process the batch
        match self.processor.process_batch(batch).await {
            Ok(()) => {
                self.update_stats_processed(chain_id, start_time.elapsed()).await;
                
                // Check if we can deactivate backpressure
                let current_size = self.buffers.read().await.get(chain_id).map(|b| b.len()).unwrap_or(0);
                let chain_config = self.config.chain_configs.get(chain_id).unwrap();
                let threshold = (chain_config.max_buffer_size as f64 * self.config.backpressure_threshold) as usize;
                
                if current_size < threshold {
                    self.deactivate_backpressure(chain_id).await;
                }
            }
            Err(e) => {
                tracing::error!("Failed to process batch for chain {}: {}", chain_id, e);
                // Could implement retry logic here
            }
        }
    }
    
    /// Stats updater
    async fn stats_updater(&self) {
        let mut interval = interval(Duration::from_secs(5));
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.update_stats().await;
                }
                _ = self.shutdown_signal.notified() => {
                    break;
                }
            }
        }
    }
    
    /// Update ingestion stats
    async fn update_stats_ingested(&self, chain_id: &str) {
        let mut stats = self.stats.write().await;
        stats.total_ingested += 1;
        
        let queue_size = self.buffers.read().await.get(chain_id).map(|b| b.len()).unwrap_or(0);
        stats.queue_sizes.insert(chain_id.to_string(), queue_size);
    }
    
    /// Update processed stats
    async fn update_stats_processed(&self, chain_id: &str, processing_time: Duration) {
        let mut stats = self.stats.write().await;
        stats.total_processed += 1;
        stats.last_processed_at = Some(SystemTime::now());
        
        // Update average processing time
        if stats.total_processed == 1 {
            stats.avg_batch_processing_time = processing_time;
        } else {
            let new_avg = Duration::from_nanos(
                (stats.avg_batch_processing_time.as_nanos() as u64 + processing_time.as_nanos() as u64) / 2
            );
            stats.avg_batch_processing_time = new_avg;
        }
        
        let queue_size = self.buffers.read().await.get(chain_id).map(|b| b.len()).unwrap_or(0);
        stats.queue_sizes.insert(chain_id.to_string(), queue_size);
    }
    
    /// Update dropped stats
    async fn update_stats_dropped(&self, chain_id: &str) {
        let mut stats = self.stats.write().await;
        stats.total_dropped += 1;
        
        let queue_size = self.buffers.read().await.get(chain_id).map(|b| b.len()).unwrap_or(0);
        stats.queue_sizes.insert(chain_id.to_string(), queue_size);
    }
    
    /// Update general stats
    async fn update_stats(&self) {
        let mut stats = self.stats.write().await;
        
        // Update queue sizes
        let buffers = self.buffers.read().await;
        for (chain_id, buffer) in buffers.iter() {
            stats.queue_sizes.insert(chain_id.clone(), buffer.len());
        }
        
        // Update backpressure status
        let backpressure_status = self.backpressure_status.read().await;
        for (chain_id, status) in backpressure_status.iter() {
            stats.active_backpressure.insert(chain_id.clone(), *status);
        }
    }
    
    /// Get current statistics
    pub async fn get_stats(&self) -> IngestionStats {
        self.stats.read().await.clone()
    }
    
    /// Gracefully shutdown the queue
    pub async fn shutdown(&self) -> Result<()> {
        // Signal shutdown
        self.shutdown_signal.notify_waiters();
        
        // Send shutdown event
        self.event_sender.send(IngestionEvent::Shutdown)
            .map_err(|e| Error::generic(format!("Failed to send shutdown signal: {}", e)))?;
        
        // Wait for shutdown timeout
        tokio::time::sleep(self.config.shutdown_timeout).await;
        
        // Process remaining events
        let buffers = self.buffers.read().await;
        for (chain_id, buffer) in buffers.iter() {
            if !buffer.is_empty() {
                tracing::warn!("Dropping {} events for chain {} during shutdown", buffer.len(), chain_id);
            }
        }
        
        Ok(())
    }
}

impl Clone for EventIngestionQueue {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            buffers: self.buffers.clone(),
            rate_limiters: self.rate_limiters.clone(),
            backpressure_status: self.backpressure_status.clone(),
            processor: self.processor.clone(),
            stats: self.stats.clone(),
            shutdown_signal: self.shutdown_signal.clone(),
            event_sender: self.event_sender.clone(),
            processing_permits: self.processing_permits.clone(),
        }
    }
}

/// Default implementation of event ingestion processor
pub struct DefaultIngestionProcessor;

#[async_trait]
impl EventIngestionProcessor for DefaultIngestionProcessor {
    async fn process_batch(&self, events: Vec<Box<dyn Event>>) -> Result<()> {
        tracing::debug!("Processing batch of {} events", events.len());
        
        // Simulate processing time
        tokio::time::sleep(Duration::from_millis(1)).await;
        
        Ok(())
    }
    
    async fn handle_backpressure(&self, chain_id: &str, queue_size: usize) -> Result<()> {
        tracing::warn!("Backpressure activated for chain {}: queue size {}", chain_id, queue_size);
        Ok(())
    }
    
    async fn handle_dropped_events(&self, chain_id: &str, events: Vec<Box<dyn Event>>) -> Result<()> {
        tracing::error!("Dropped {} events for chain {}", events.len(), chain_id);
        Ok(())
    }
    
    async fn get_stats(&self) -> Result<HashMap<String, serde_json::Value>> {
        Ok(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    #[derive(Debug)]
    struct TestEvent {
        id: String,
        chain: String,
        block_number: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: SystemTime,
        event_type: String,
        raw_data: Vec<u8>,
    }
    
    impl crate::types::Event for TestEvent {
        fn id(&self) -> &str { &self.id }
        fn chain(&self) -> &str { &self.chain }
        fn block_number(&self) -> u64 { self.block_number }
        fn block_hash(&self) -> &str { &self.block_hash }
        fn tx_hash(&self) -> &str { &self.tx_hash }
        fn timestamp(&self) -> SystemTime { self.timestamp }
        fn event_type(&self) -> &str { &self.event_type }
        fn raw_data(&self) -> &[u8] { &self.raw_data }
        fn as_any(&self) -> &dyn std::any::Any { self }
    }
    
    struct TestProcessor {
        processed_count: AtomicU64,
    }
    
    impl TestProcessor {
        fn new() -> Self {
            Self {
                processed_count: AtomicU64::new(0),
            }
        }
        
        fn get_processed_count(&self) -> u64 {
            self.processed_count.load(Ordering::Relaxed)
        }
    }
    
    #[async_trait]
    impl EventIngestionProcessor for TestProcessor {
        async fn process_batch(&self, events: Vec<Box<dyn Event>>) -> Result<()> {
            self.processed_count.fetch_add(events.len() as u64, Ordering::Relaxed);
            Ok(())
        }
        
        async fn handle_backpressure(&self, _chain_id: &str, _queue_size: usize) -> Result<()> {
            Ok(())
        }
        
        async fn handle_dropped_events(&self, _chain_id: &str, _events: Vec<Box<dyn Event>>) -> Result<()> {
            Ok(())
        }
        
        async fn get_stats(&self) -> Result<HashMap<String, serde_json::Value>> {
            Ok(HashMap::new())
        }
    }
    
    #[test]
    fn test_ingestion_queue_config() {
        let config = IngestionQueueConfig::default();
        
        // Test default configuration
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.max_buffer_size, 10000);
        assert_eq!(config.backpressure_threshold, 0.8);
        assert!(config.enable_metrics);
        
        // Test chain configurations
        assert!(config.chain_configs.contains_key("ethereum"));
        let eth_config = &config.chain_configs["ethereum"];
        assert_eq!(eth_config.chain_id, "ethereum");
        assert!(eth_config.enable_rate_limiting);
        assert!(eth_config.enable_backpressure);
    }
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let rate_limiter = RateLimiter::new(10); // 10 events per second
        
        // First token should be available
        assert!(rate_limiter.try_acquire().await);
        
        // More tokens should be available initially (burst allowance)
        assert!(rate_limiter.try_acquire().await);
        
        // Eventually should be rate limited
        let mut acquired = 0;
        for _ in 0..20 {
            if rate_limiter.try_acquire().await {
                acquired += 1;
            } else {
                break;
            }
        }
        
        // Should have acquired some but not all
        assert!(acquired > 0);
        assert!(acquired < 20);
    }
    
    #[test]
    fn test_ingestion_result_types() {
        // Test that all result types can be created
        let _queued = IngestionResult::Queued;
        let _backpressure = IngestionResult::QueuedWithBackpressure;
        let _dropped = IngestionResult::Dropped("test".to_string());
        let _rate_limited = IngestionResult::RateLimited;
        let _invalid = IngestionResult::InvalidConfig("test".to_string());
        
        // Test matching
        match IngestionResult::Queued {
            IngestionResult::Queued => {
                // Expected result - test passes
            },
            _ => panic!("Expected Queued result"),
        }
    }
    
    #[tokio::test]
    async fn test_processor_interface() {
        let processor = TestProcessor::new();
        
        // Test initial state
        assert_eq!(processor.get_processed_count(), 0);
        
        // Test processing a batch
        let events: Vec<Box<dyn Event>> = vec![
            Box::new(TestEvent {
                id: "test1".to_string(),
                chain: "ethereum".to_string(),
                block_number: 100,
                block_hash: "hash1".to_string(),
                tx_hash: "tx1".to_string(),
                timestamp: SystemTime::now(),
                event_type: "transfer".to_string(),
                raw_data: b"{}".to_vec(),
            }),
        ];
        
        processor.process_batch(events).await.unwrap();
        assert_eq!(processor.get_processed_count(), 1);
        
        // Test other methods don't fail
        processor.handle_backpressure("ethereum", 100).await.unwrap();
        processor.handle_dropped_events("ethereum", vec![]).await.unwrap();
        processor.get_stats().await.unwrap();
    }
} 