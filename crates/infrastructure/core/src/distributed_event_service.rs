use crate::{Result, Error};
use crate::types::Event;
use crate::service::EventService;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Minimal storage trait for distributed event service
#[async_trait]
pub trait EventStorage: Send + Sync {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()>;
    async fn get_latest_block(&self, chain: &str) -> Result<Option<u64>>;
    async fn get_event_count(&self) -> Result<u64>;
}

/// Configuration for distributed event processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedEventConfig {
    pub node_id: String,
    pub partition_strategy: PartitionStrategy,
    pub batch_size: usize,
    pub enable_distributed_transactions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionStrategy {
    ByChain,
    ByContract,
    ByBlockRange { blocks_per_partition: u64 },
    RoundRobin,
}

/// Distributed event service wrapper
pub struct DistributedEventService {
    node_id: String,
    storage: Arc<dyn EventStorage>,
    config: DistributedEventConfig,
    partition_assignments: Arc<RwLock<HashMap<String, String>>>, // partition -> node_id
}

impl DistributedEventService {
    pub fn new(storage: Arc<dyn EventStorage>, config: DistributedEventConfig) -> Self {
        Self {
            node_id: config.node_id.clone(),
            storage,
            config,
            partition_assignments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Determine which partition an event belongs to
    fn get_event_partition(&self, event: &dyn Event) -> String {
        match &self.config.partition_strategy {
            PartitionStrategy::ByChain => event.chain_id(),
            PartitionStrategy::ByContract => {
                event.transaction_hash().unwrap_or_else(|| event.chain_id())
            }
            PartitionStrategy::ByBlockRange { blocks_per_partition } => {
                let partition = event.block_number() / blocks_per_partition;
                format!("{}-{}", event.chain_id(), partition)
            }
            PartitionStrategy::RoundRobin => {
                let hash = event.transaction_hash().unwrap_or_default();
                let hash_bytes = hash.as_bytes();
                let hash_sum: u64 = hash_bytes.iter().map(|&b| b as u64).sum();
                format!("partition-{}", hash_sum % 16)
            }
        }
    }

    /// Check if this node should process the event
    async fn should_process_event(&self, event: &dyn Event) -> bool {
        let partition = self.get_event_partition(event);
        let assignments = self.partition_assignments.read().await;
        
        match assignments.get(&partition) {
            Some(assigned_node) => assigned_node == &self.node_id,
            None => {
                // If no assignment exists, claim it
                drop(assignments);
                let mut assignments = self.partition_assignments.write().await;
                assignments.insert(partition.clone(), self.node_id.clone());
                true
            }
        }
    }

    /// Update partition assignments from coordination service
    pub async fn update_partition_assignments(&self, assignments: HashMap<String, String>) {
        let mut current = self.partition_assignments.write().await;
        *current = assignments;
    }
}

#[async_trait]
impl EventService for DistributedEventService {
    type EventType = Box<dyn Event>;

    fn chain_id(&self) -> &crate::types::ChainId {
        // Return a default chain ID for the distributed service
        // In practice, this would be configurable
        static DEFAULT_CHAIN: std::sync::OnceLock<crate::types::ChainId> = std::sync::OnceLock::new();
        DEFAULT_CHAIN.get_or_init(|| crate::types::ChainId::from("distributed"))
    }

    async fn get_events(&self, _filters: Vec<crate::types::EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        // For distributed service, this would coordinate queries across nodes
        // For now, return empty results
        Ok(Vec::new())
    }

    async fn subscribe(&self) -> Result<Box<dyn crate::service::EventSubscription>> {
        // For distributed service, this would create a distributed subscription
        // For now, return an error
        Err(crate::Error::Generic("Distributed subscriptions not implemented".to_string()))
    }

    async fn get_latest_block(&self) -> Result<u64> {
        // Get the latest block across all managed chains
        // For now, delegate to storage
        self.storage.get_latest_block("distributed").await
            .map(|opt| opt.unwrap_or(0))
    }
}

impl DistributedEventService {
    /// Process an event (custom method for distributed processing)
    pub async fn process_event(&self, event: Box<dyn Event>) -> Result<()> {
        // Check if this node should process the event
        if !self.should_process_event(event.as_ref()).await {
            debug!(
                "Skipping event {} - assigned to different node",
                event.transaction_hash().unwrap_or_default()
            );
            return Ok(());
        }

        // Process with distributed transaction support if enabled
        if self.config.enable_distributed_transactions {
            // In a real implementation, this would use distributed transactions
            // For now, we'll use regular storage operations
            info!(
                "Processing event with distributed transaction: {}",
                event.transaction_hash().unwrap_or_default()
            );
        }

        // Store the event
        self.storage.store_event(event).await?;

        Ok(())
    }

    /// Health check for the distributed service
    pub async fn health_check(&self) -> Result<bool> {
        // Check storage health
        let storage_healthy = self.storage.get_event_count().await.is_ok();
        
        // Check partition assignments are up to date
        let assignments = self.partition_assignments.read().await;
        let has_assignments = !assignments.is_empty();
        
        Ok(storage_healthy && has_assignments)
    }
}

/// Batch processor with distributed awareness
pub struct DistributedBatchProcessor {
    service: Arc<DistributedEventService>,
    batch_size: usize,
    pending_events: Arc<RwLock<Vec<Box<dyn Event>>>>,
}

impl DistributedBatchProcessor {
    pub fn new(service: Arc<DistributedEventService>, batch_size: usize) -> Self {
        Self {
            service,
            batch_size,
            pending_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add event to batch
    pub async fn add_event(&self, event: Box<dyn Event>) -> Result<()> {
        let mut pending = self.pending_events.write().await;
        pending.push(event);

        if pending.len() >= self.batch_size {
            self.flush_batch().await?;
        }

        Ok(())
    }

    /// Flush pending events
    pub async fn flush_batch(&self) -> Result<()> {
        let mut pending = self.pending_events.write().await;
        if pending.is_empty() {
            return Ok(());
        }

        let events = std::mem::take(&mut *pending);
        drop(pending);

        // Process events in parallel batches by partition
        let mut partitioned_events: HashMap<String, Vec<Box<dyn Event>>> = HashMap::new();
        
        for event in events {
            let partition = self.service.get_event_partition(event.as_ref());
            partitioned_events.entry(partition).or_default().push(event);
        }

        // Process each partition's events
        let mut handles = Vec::new();
        for (partition, events) in partitioned_events {
            let service = Arc::clone(&self.service);
            let handle = tokio::spawn(async move {
                debug!("Processing {} events for partition {}", events.len(), partition);
                for event in events {
                    if let Err(e) = service.process_event(event).await {
                        tracing::error!("Failed to process event: {}", e);
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all partitions to complete
        for handle in handles {
            handle.await.map_err(|e| crate::Error::Generic(format!("Task join error: {}", e)))?;
        }

        Ok(())
    }
}

/// Pipeline support for distributed transactions
pub struct DistributedTransactionPipeline {
    stages: Vec<Box<dyn PipelineStage>>,
}

#[async_trait]
pub trait PipelineStage: Send + Sync {
    async fn process(&self, event: &dyn Event, context: &mut PipelineContext) -> Result<()>;
}

pub struct PipelineContext {
    pub transaction_id: String,
    pub metadata: HashMap<String, String>,
}

impl DistributedTransactionPipeline {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
        }
    }

    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    pub async fn process_event(&self, event: Box<dyn Event>) -> Result<()> {
        let mut context = PipelineContext {
            transaction_id: uuid::Uuid::new_v4().to_string(),
            metadata: HashMap::new(),
        };

        for stage in &self.stages {
            stage.process(event.as_ref(), &mut context).await?;
        }

        Ok(())
    }
}

