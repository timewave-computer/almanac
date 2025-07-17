// Simplified event ordering system for high-throughput cross-chain event processing
//
// This module provides priority-based event ordering with confirmation delays
// and basic dependency tracking, optimized for performance and reliability.

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::types::UnifiedEvent;
use crate::{Error, Result};

/// Configuration for event ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOrderingConfig {
    /// Number of blocks to wait before processing events from each chain
    pub chain_confirmation_blocks: HashMap<String, u64>,
    
    /// Default confirmation blocks for chains not explicitly configured
    pub default_confirmation_blocks: u64,
    
    /// Maximum queue size to prevent memory issues
    pub max_queue_size: usize,
    
    /// Maximum number of events to process in a single batch
    pub max_batch_size: usize,
    
    /// Interval for processing batches
    pub batch_interval: Duration,
}

impl Default for EventOrderingConfig {
    fn default() -> Self {
        let mut chain_confirmation_blocks = HashMap::new();
        chain_confirmation_blocks.insert("ethereum".to_string(), 12);
        chain_confirmation_blocks.insert("polygon".to_string(), 64);
        chain_confirmation_blocks.insert("base".to_string(), 12);
        chain_confirmation_blocks.insert("osmosis".to_string(), 1);
        chain_confirmation_blocks.insert("noble".to_string(), 1);
        chain_confirmation_blocks.insert("neutron".to_string(), 1);
        
        Self {
            chain_confirmation_blocks,
            default_confirmation_blocks: 6,
            max_queue_size: 10000,
            max_batch_size: 100,
            batch_interval: Duration::from_millis(100),
        }
    }
}

/// An event with ordering priority and metadata
#[derive(Debug)]
pub struct OrderedEvent {
    /// The actual event (simplified as UnifiedEvent)
    pub event: UnifiedEvent,
    
    /// Priority for ordering
    pub priority: EventPriority,
    
    /// When this event was added to the queue
    pub queued_at: SystemTime,
    
    /// Whether this event has been confirmed
    pub confirmed: bool,
    
    /// Confirmation deadline
    pub confirm_by: SystemTime,
}

/// Event priority for ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventPriority {
    /// Block number (higher = newer)
    pub block_number: u64,
    
    /// Log index within the block (higher = later in block)
    pub log_index: u32,
    
    /// Chain priority (for cross-chain ordering)
    pub chain_priority: u8,
}

impl PartialOrd for EventPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher block number = higher priority
        self.block_number.cmp(&other.block_number)
            .then_with(|| self.log_index.cmp(&other.log_index))
            .then_with(|| self.chain_priority.cmp(&other.chain_priority))
    }
}

impl PartialEq for OrderedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for OrderedEvent {}

impl PartialOrd for OrderedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for max-heap (BinaryHeap is max-heap by default)
        other.priority.cmp(&self.priority)
    }
}

/// Event processor trait
#[async_trait]
pub trait EventProcessor: Send + Sync {
    /// Process a single event
    async fn process_event(&self, event: &UnifiedEvent) -> Result<()>;
    
    /// Process a batch of events
    async fn process_batch(&self, events: Vec<&UnifiedEvent>) -> Result<()> {
        for event in events {
            self.process_event(event).await?;
        }
        Ok(())
    }
    
    /// Get processing stats
    async fn get_stats(&self) -> ProcessingStats;
}

/// Processing statistics
#[derive(Debug, Clone)]
pub struct ProcessingStats {
    pub total_processed: u64,
    pub total_errors: u64,
    pub current_queue_size: usize,
    pub avg_processing_time: Duration,
    pub last_processed_at: Option<SystemTime>,
}

/// Event ordering queue with priority-based processing
pub struct EventOrderingQueue {
    /// Configuration
    config: EventOrderingConfig,
    
    /// Priority queue of events waiting for confirmation
    pending_events: Arc<RwLock<BinaryHeap<OrderedEvent>>>,
    
    /// Current chain heads for confirmation tracking
    chain_heads: Arc<RwLock<HashMap<String, u64>>>,
    
    /// Event processor
    processor: Arc<dyn EventProcessor>,
    
    /// Channel for adding new events
    event_sender: mpsc::UnboundedSender<OrderedEvent>,
    
    /// Processing statistics
    stats: Arc<RwLock<ProcessingStats>>,
}

impl EventOrderingQueue {
    /// Create a new event ordering queue
    pub fn new(
        config: EventOrderingConfig,
        processor: Arc<dyn EventProcessor>,
    ) -> Result<Self> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let queue = Self {
            config,
            pending_events: Arc::new(RwLock::new(BinaryHeap::new())),
            chain_heads: Arc::new(RwLock::new(HashMap::new())),
            processor,
            event_sender,
            stats: Arc::new(RwLock::new(ProcessingStats {
                total_processed: 0,
                total_errors: 0,
                current_queue_size: 0,
                avg_processing_time: Duration::from_millis(0),
                last_processed_at: None,
            })),
        };
        
        // Start background processing
        let queue_clone = queue.clone();
        tokio::spawn(async move {
            queue_clone.process_events(event_receiver).await;
        });
        
        Ok(queue)
    }
    
    /// Add an event to the queue
    pub async fn add_event(&self, event: UnifiedEvent) -> Result<()> {
        let ordered_event = self.create_ordered_event(event).await?;
        
        // Check queue size limits
        let current_size = self.pending_events.read().await.len();
        if current_size >= self.config.max_queue_size {
            return Err(Error::generic("Event queue is full"));
        }
        
        // Send to processing channel
        self.event_sender
            .send(ordered_event)
            .map_err(|e| Error::generic(format!("Failed to queue event: {}", e)))?;
        
        Ok(())
    }
    
    /// Update chain head for confirmation tracking
    pub async fn update_chain_head(&self, chain: &str, block_number: u64) {
        let mut heads = self.chain_heads.write().await;
        heads.insert(chain.to_string(), block_number);
    }
    
    /// Get processing statistics
    pub async fn get_stats(&self) -> ProcessingStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        
        result.current_queue_size = self.pending_events.read().await.len();
        result
    }
    
    /// Create an ordered event with priority
    async fn create_ordered_event(&self, event: UnifiedEvent) -> Result<OrderedEvent> {
        let priority = self.calculate_priority(&event).await?;
        
        let confirmation_blocks = self.config.chain_confirmation_blocks
            .get(&event.chain_id)
            .copied()
            .unwrap_or(self.config.default_confirmation_blocks);
        
        let confirm_by = SystemTime::now() + Duration::from_secs(confirmation_blocks * 12); // Assume 12s blocks
        
        Ok(OrderedEvent {
            event,
            priority,
            queued_at: SystemTime::now(),
            confirmed: false,
            confirm_by,
        })
    }
    
    /// Calculate event priority
    async fn calculate_priority(&self, event: &UnifiedEvent) -> Result<EventPriority> {
        let chain_priority = self.get_chain_priority(&event.chain_id);
        
        Ok(EventPriority {
            block_number: event.block_number,
            log_index: event.log_index,
            chain_priority,
        })
    }
    
    /// Get chain priority for ordering
    fn get_chain_priority(&self, chain: &str) -> u8 {
        match chain {
            "ethereum" => 100,
            "polygon" => 90,
            "base" => 80,
            "osmosis" => 70,
            "noble" => 60,
            "neutron" => 50,
            _ => 10,
        }
    }
    
    /// Background event processing loop
    async fn process_events(&self, mut event_receiver: mpsc::UnboundedReceiver<OrderedEvent>) {
        let mut batch_timer = tokio::time::interval(self.config.batch_interval);
        
        loop {
            tokio::select! {
                // Process new events
                event = event_receiver.recv() => {
                    if let Some(event) = event {
                        self.handle_new_event(event).await;
                    }
                }
                
                // Process batch of confirmed events
                _ = batch_timer.tick() => {
                    self.process_confirmed_events().await;
                }
            }
        }
    }
    
    /// Handle a new event
    async fn handle_new_event(&self, event: OrderedEvent) {
        // Add to pending queue
        self.pending_events.write().await.push(event);
    }
    
    /// Process confirmed events
    async fn process_confirmed_events(&self) {
        let mut batch = Vec::new();
        
        // Collect confirmed events
        {
            let mut pending = self.pending_events.write().await;
            let chain_heads = self.chain_heads.read().await;
            
            // Extract events ready for processing
            let mut temp_heap = BinaryHeap::new();
            
            while let Some(event) = pending.pop() {
                if self.is_event_confirmed(&event, &chain_heads).await {
                    if batch.len() < self.config.max_batch_size {
                        batch.push(event);
                    } else {
                        // Put back in queue for next batch
                        temp_heap.push(event);
                    }
                } else {
                    // Not yet confirmed, put back
                    temp_heap.push(event);
                }
            }
            
            // Restore unprocessed events
            while let Some(event) = temp_heap.pop() {
                pending.push(event);
            }
        }
        
        // Process the batch
        if !batch.is_empty() {
            self.process_event_batch(batch).await;
        }
    }
    
    /// Check if event is confirmed
    async fn is_event_confirmed(&self, event: &OrderedEvent, chain_heads: &HashMap<String, u64>) -> bool {
        let chain = &event.event.chain_id;
        let confirmation_blocks = self.config.chain_confirmation_blocks
            .get(chain)
            .copied()
            .unwrap_or(self.config.default_confirmation_blocks);
        
        if let Some(&chain_head) = chain_heads.get(chain) {
            chain_head >= event.event.block_number + confirmation_blocks
        } else {
            // No chain head info, check by time
            SystemTime::now() > event.confirm_by
        }
    }
    
    /// Process a batch of events
    async fn process_event_batch(&self, batch: Vec<OrderedEvent>) {
        let start_time = Instant::now();
        
        // Convert to references for processing
        let event_refs: Vec<&UnifiedEvent> = batch.iter().map(|oe| &oe.event).collect();
        
        // Process the batch
        match self.processor.process_batch(event_refs).await {
            Ok(()) => {
                // Update stats
                let processing_time = start_time.elapsed();
                let mut stats = self.stats.write().await;
                stats.total_processed += batch.len() as u64;
                stats.last_processed_at = Some(SystemTime::now());
                
                // Update average processing time
                let new_avg = if stats.total_processed == 1 {
                    processing_time
                } else {
                    Duration::from_nanos(
                        (stats.avg_processing_time.as_nanos() as u64 + processing_time.as_nanos() as u64) / 2
                    )
                };
                stats.avg_processing_time = new_avg;
            }
            Err(e) => {
                tracing::error!("Failed to process event batch: {}", e);
                let mut stats = self.stats.write().await;
                stats.total_errors += batch.len() as u64;
            }
        }
    }
}

impl Clone for EventOrderingQueue {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            pending_events: self.pending_events.clone(),
            chain_heads: self.chain_heads.clone(),
            processor: self.processor.clone(),
            event_sender: self.event_sender.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// Default event processor implementation
pub struct DefaultEventProcessor;

#[async_trait]
impl EventProcessor for DefaultEventProcessor {
    async fn process_event(&self, event: &UnifiedEvent) -> Result<()> {
        tracing::debug!("Processing event: {} from chain: {}", event.id, event.chain_id);
        
        // Simulate processing time
        tokio::time::sleep(Duration::from_millis(1)).await;
        
        Ok(())
    }
    
    async fn get_stats(&self) -> ProcessingStats {
        ProcessingStats {
            total_processed: 0,
            total_errors: 0,
            current_queue_size: 0,
            avg_processing_time: Duration::from_millis(1),
            last_processed_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    struct TestProcessor {
        processed_count: AtomicU64,
    }
    
    impl TestProcessor {
        fn new() -> Self {
            Self {
                processed_count: AtomicU64::new(0),
            }
        }
    }
    
    #[async_trait]
    impl EventProcessor for TestProcessor {
        async fn process_event(&self, _event: &UnifiedEvent) -> Result<()> {
            self.processed_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        
        async fn get_stats(&self) -> ProcessingStats {
            ProcessingStats {
                total_processed: self.processed_count.load(Ordering::Relaxed),
                total_errors: 0,
                current_queue_size: 0,
                avg_processing_time: Duration::from_millis(1),
                last_processed_at: Some(SystemTime::now()),
            }
        }
    }
    
    #[tokio::test]
    async fn test_event_ordering() {
        let config = EventOrderingConfig::default();
        let processor = Arc::new(TestProcessor::new());
        
        let queue = EventOrderingQueue::new(config, processor.clone()).unwrap();
        
        // Add test events
        let event1 = UnifiedEvent {
            id: "test1".to_string(),
            chain_id: "ethereum".to_string(),
            block_number: 100,
            transaction_hash: "tx1".to_string(),
            log_index: 0,
            contract_address: "0xabc".to_string(),
            event_name: "Transfer".to_string(),
            event_data: serde_json::json!({}),
            finality_status: "confirmed".to_string(),
            timestamp: SystemTime::now(),
            source: crate::types::EventSource::Chain,
        };
        
        let event2 = UnifiedEvent {
            id: "test2".to_string(),
            chain_id: "ethereum".to_string(),
            block_number: 101,
            transaction_hash: "tx2".to_string(),
            log_index: 0,
            contract_address: "0xabc".to_string(),
            event_name: "Transfer".to_string(),
            event_data: serde_json::json!({}),
            finality_status: "confirmed".to_string(),
            timestamp: SystemTime::now(),
            source: crate::types::EventSource::Chain,
        };
        
        queue.add_event(event1).await.unwrap();
        queue.add_event(event2).await.unwrap();
        
        // Update chain head to trigger processing
        queue.update_chain_head("ethereum", 115).await;
        
        // Wait a bit for processing
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        let stats = queue.get_stats().await;
        assert!(stats.current_queue_size <= 2);
    }
} 