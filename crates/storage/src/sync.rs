// sync.rs - Synchronization utilities for storage operations
//
// Purpose: Provides utilities for coordinating storage operations across
// different storage backends and handling concurrent access patterns.

use indexer_core::{Result, BlockStatus, Error};
use indexer_core::event::Event;
use indexer_core::types::ChainId;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use std::time::Duration;
use std::any::Any;

use crate::{Storage, BoxedStorage, EventFilter};
#[cfg(feature = "rocks")]
use crate::rocks::RocksStorage;
#[cfg(feature = "postgres")]
use crate::postgres::PostgresStorage;

use tokio::task::JoinHandle;
use tokio::time;
use futures::future::join_all;

/// Configuration for storage synchronization
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Synchronization interval in milliseconds
    pub sync_interval_ms: u64,
    
    /// Maximum number of events to synchronize in a single batch
    pub batch_size: usize,
    
    /// Chains to synchronize
    pub chains: Vec<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_ms: 1000, // 1 second
            batch_size: 100,
            chains: vec!["ethereum".to_string(), "cosmos".to_string()],
        }
    }
}

/// Storage synchronizer for multi-store consistency
pub struct StorageSynchronizer {
    /// Primary storage
    primary: BoxedStorage,
    
    /// Secondary storage
    secondary: BoxedStorage,
    
    /// Synchronization configuration
    config: SyncConfig,
    
    /// Synchronization task handle
    task_handle: RwLock<Option<JoinHandle<()>>>,
    
    /// Is the synchronizer running
    running: RwLock<bool>,
}

impl StorageSynchronizer {
    /// Create a new storage synchronizer with any two storage implementations
    pub async fn new_generic<P, S>(
        primary: Arc<P>, 
        secondary: Arc<S>,
        config: SyncConfig
    ) -> Self 
    where 
        P: Storage + Send + Sync + 'static,
        S: Storage + Send + Sync + 'static
    {
        Self {
            primary: primary as BoxedStorage,
            secondary: secondary as BoxedStorage,
            config,
            task_handle: RwLock::new(None),
            running: RwLock::new(false),
        }
    }
    
    /// Create a new storage synchronizer with RocksDB as primary and PostgreSQL as secondary
    #[cfg(all(feature = "rocks", feature = "postgres"))]
    pub async fn new_rocks_postgres(
        rocks: Arc<RocksStorage>,
        postgres: Arc<PostgresStorage>,
        config: SyncConfig
    ) -> Self {
        Self::new_generic(rocks, postgres, config).await
    }
    
    /// Create a new storage synchronizer with PostgreSQL as primary and RocksDB as secondary
    #[cfg(all(feature = "rocks", feature = "postgres"))]
    pub async fn new_postgres_rocks(
        postgres: Arc<PostgresStorage>,
        rocks: Arc<RocksStorage>,
        config: SyncConfig
    ) -> Self {
        Self::new_generic(postgres, rocks, config).await
    }
    
    /// Start synchronization
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        
        if *running {
            warn!("Synchronization is already running");
            return Ok(());
        }
        
        *running = true;
        
        let primary = self.primary.clone();
        let secondary = self.secondary.clone();
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            let sync_interval = Duration::from_millis(config.sync_interval_ms);
            
            loop {
                // Synchronize each chain
                for chain in &config.chains {
                    if let Err(e) = Self::sync_chain(&primary, &secondary, chain, config.batch_size).await {
                        error!("Failed to synchronize chain {}: {}", chain, e);
                    }
                }
                
                // Wait for the next sync interval
                time::sleep(sync_interval).await;
            }
        });
        
        let mut task_handle = self.task_handle.write().await;
        *task_handle = Some(handle);
        
        info!("Storage synchronization started");
        Ok(())
    }
    
    /// Stop synchronization
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        
        if !*running {
            warn!("Synchronization is not running");
            return Ok(());
        }
        
        let mut task_handle = self.task_handle.write().await;
        
        if let Some(handle) = task_handle.take() {
            handle.abort();
        }
        
        *running = false;
        
        info!("Storage synchronization stopped");
        Ok(())
    }
    
    /// Synchronize a specific chain
    async fn sync_chain(
        primary: &BoxedStorage, 
        secondary: &BoxedStorage, 
        chain: &str, 
        batch_size: usize
    ) -> Result<()> {
        debug!("Synchronizing chain {} with batch size {}", chain, batch_size);
        
        // Get latest block from primary and secondary storage
        let primary_latest = primary.get_latest_block(chain).await?;
        let secondary_latest = secondary.get_latest_block(chain).await?;
        
        debug!("Latest blocks - Primary: {}, Secondary: {}", primary_latest, secondary_latest);
        
        if secondary_latest >= primary_latest {
            debug!("Secondary storage is up-to-date for chain {}", chain);
            return Ok(());
        }
        
        // Define the range of blocks to synchronize
        let start_block = secondary_latest + 1;
        let end_block = std::cmp::min(
            primary_latest,
            secondary_latest + batch_size as u64
        );
        
        info!("Synchronizing chain {} from block {} to {}", chain, start_block, end_block);
        
        // Create filter to get events from the primary storage
        let mut filter = EventFilter::new();
        filter.chain_ids = Some(vec![ChainId::from(chain)]);
        filter.chain = Some(chain.to_string());
        filter.block_range = Some((start_block, end_block));
        filter.time_range = None;
        filter.event_types = None;
        filter.limit = None;
        filter.offset = None;
        
        // Get events from primary
        let events = primary.get_events(chain, start_block, end_block).await?;
        
        if events.is_empty() {
            debug!("No events to synchronize for chain {}", chain);
            
            // Update the secondary storage's latest block even if no events
            // This prevents getting stuck on empty blocks
            Self::update_secondary_latest_block(secondary, chain, end_block).await?;
            
            return Ok(());
        }
        
        debug!("Found {} events to synchronize for chain {}", events.len(), chain);
        
        // Store events in secondary storage
        let mut futures = Vec::new();
        
        for event in events {
            let secondary_clone = secondary.clone();
            let event_clone = event.clone();
            let chain_clone = chain.to_string();
            
            futures.push(tokio::spawn(async move {
                secondary_clone.store_event(&chain_clone, event_clone).await
            }));
        }
        
        // Wait for all events to be stored
        let results = join_all(futures).await;
        
        // Check for errors
        for result in results.into_iter().flatten() {
            if let Err(err) = result {
                error!("Failed to store event in secondary storage: {:?}", err);
                return Err(err);
            }
        }
        
        // Update block status for synchronized blocks
        for block_number in start_block..=end_block {
            // Get the status from primary storage
            let status = Self::get_block_status(primary, chain, block_number).await?;
            
            // Update the status in secondary storage
            secondary.update_block_status(chain, block_number, status).await?;
        }
        
        // Update secondary's latest block
        Self::update_secondary_latest_block(secondary, chain, end_block).await?;
        
        info!("Successfully synchronized chain {} from block {} to {}", chain, start_block, end_block);
        
        Ok(())
    }
    
    /// Get block status from storage
    #[allow(unused_variables)]
    async fn get_block_status(
        primary: &BoxedStorage,
        chain: &str,
        block_number: u64
    ) -> Result<BlockStatus> {
        // This is a simplified implementation
        // In a real system, we'd query the actual block status
        // For now, treat all blocks as confirmed
        Ok(BlockStatus::Confirmed)
    }
    
    /// Update the secondary storage's latest block tracking
    async fn update_secondary_latest_block(
        secondary: &BoxedStorage,
        chain: &str,
        latest_block: u64
    ) -> Result<()> {
        // Create a special event to record the latest block
        // This is a workaround since there's no direct way to set the latest block
        let event_id = format!("sync_latest_block_{}_{}", chain, latest_block);
        let event = Box::new(SyncEvent {
            id: event_id,
            chain: chain.to_string(),
            block_number: latest_block,
        });
        
        secondary.store_event(chain, event).await
    }

    /// Manually process an event
    pub async fn process_event(
        storage: &BoxedStorage,
        chain: &str,
        block_number: u64
    ) -> Result<()> {
        debug!("Processing events for chain {} at block {}", chain, block_number);
        
        // Get all events for this specific block
        let events = storage.get_events(chain, block_number, block_number).await?;
        
        if events.is_empty() {
            debug!("No events found for chain {} at block {}", chain, block_number);
            return Ok(());
        }
        
        info!("Processing {} events for chain {} at block {}", events.len(), chain, block_number);
        
        // Process each event with validation and type-specific handling
        for event in events {
            // Validate event data
            Self::validate_event(&event)?;
            
            // Clone event data for processing to avoid ownership issues
            let event_id = event.id().to_string();
            let event_type = event.event_type().to_string();
            let event_chain = event.chain().to_string();
            
            // Process based on event type
            match event_type.as_str() {
                // Valence Account events
                "valence_account_instantiated" => {
                    debug!("Processing Valence account instantiation for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_account_instantiation(account_info, initial_libraries).await?;
                },
                "valence_library_approved" => {
                    debug!("Processing Valence library approval for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_library_approval(account_id, library_info, block_number, tx_hash).await?;
                },
                "valence_library_removed" => {
                    debug!("Processing Valence library removal for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_library_removal(account_id, library_address, block_number, tx_hash).await?;
                },
                "valence_ownership_updated" => {
                    debug!("Processing Valence ownership update for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_ownership_update(...).await?;
                },
                "valence_execution" => {
                    debug!("Processing Valence execution for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_execution(execution_info).await?;
                },
                
                // Valence Processor events
                "valence_processor_instantiated" => {
                    debug!("Processing Valence processor instantiation for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_processor_instantiation(processor_info).await?;
                },
                "valence_processor_config_updated" => {
                    debug!("Processing Valence processor config update for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_processor_config_update(...).await?;
                },
                "valence_processor_message" => {
                    debug!("Processing Valence processor message for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_processor_message(message).await?;
                },
                
                // Valence Authorization events
                "valence_authorization_instantiated" => {
                    debug!("Processing Valence authorization instantiation for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_authorization_instantiation(...).await?;
                },
                "valence_authorization_grant" => {
                    debug!("Processing Valence authorization grant for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_authorization_grant(grant).await?;
                },
                
                // Valence Library events
                "valence_library_instantiated" => {
                    debug!("Processing Valence library instantiation for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_library_instantiation(...).await?;
                },
                "valence_library_version_added" => {
                    debug!("Processing Valence library version for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_library_version(version).await?;
                },
                "valence_library_usage" => {
                    debug!("Processing Valence library usage for event {}", event.id());
                    // In a real implementation, we would parse the event data and call:
                    // storage.store_valence_library_usage(usage).await?;
                },
                
                // Generic events
                _ => {
                    debug!("Processing generic event type: {}", event.event_type());
                    // For unknown event types, just ensure they're stored
                    storage.store_event(&event_chain, event).await?;
                    debug!("Successfully processed generic event {}", event_id);
                }
            }
        }
        
        // Mark the block as processed
        storage.mark_block_processed(chain, block_number, "manual_processing", BlockStatus::Confirmed).await?;
        
        info!("Successfully processed all events for chain {} at block {}", chain, block_number);
        Ok(())
    }
    
    /// Validate event data integrity
    #[allow(clippy::borrowed_box)]
    fn validate_event(event: &Box<dyn Event>) -> Result<()> {
        // Basic validation
        if event.id().is_empty() {
            return Err(Error::generic("Event ID cannot be empty"));
        }
        
        if event.chain().is_empty() {
            return Err(Error::generic("Event chain cannot be empty"));
        }
        
        if event.block_number() == 0 {
            return Err(Error::generic("Event block number cannot be zero"));
        }
        
        if event.block_hash().is_empty() {
            return Err(Error::generic("Event block hash cannot be empty"));
        }
        
        if event.tx_hash().is_empty() {
            return Err(Error::generic("Event transaction hash cannot be empty"));
        }
        
        if event.event_type().is_empty() {
            return Err(Error::generic("Event type cannot be empty"));
        }
        
        debug!("Event {} passed validation", event.id());
        Ok(())
    }
}

/// Special event type for synchronization purposes
#[derive(Debug, Clone)]
struct SyncEvent {
    id: String,
    chain: String,
    block_number: u64,
}

impl Event for SyncEvent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn chain(&self) -> &str {
        &self.chain
    }
    
    fn block_number(&self) -> u64 {
        self.block_number
    }
    
    fn block_hash(&self) -> &str {
        "sync_block_hash"
    }
    
    fn tx_hash(&self) -> &str {
        "sync_tx_hash"
    }
    
    fn timestamp(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }
    
    fn event_type(&self) -> &str {
        "sync"
    }
    
    fn raw_data(&self) -> &[u8] {
        &[]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Extension trait for Box<dyn Event> to add cloning capabilities
trait EventExt {
    fn clone(&self) -> Box<dyn Event>;
}

impl EventExt for Box<dyn Event> {
    fn clone(&self) -> Box<dyn Event> {
        Box::new(ClonedEvent {
            id: self.id().to_string(),
            chain: self.chain().to_string(),
            block_number: self.block_number(),
            block_hash: self.block_hash().to_string(),
            tx_hash: self.tx_hash().to_string(),
            timestamp: self.timestamp(),
            event_type: self.event_type().to_string(),
            raw_data: self.raw_data().to_vec(),
        })
    }
}

/// Helper struct for cloning events
#[derive(Debug)]
struct ClonedEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    timestamp: std::time::SystemTime,
    event_type: String,
    raw_data: Vec<u8>,
}

impl Event for ClonedEvent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn chain(&self) -> &str {
        &self.chain
    }
    
    fn block_number(&self) -> u64 {
        self.block_number
    }
    
    fn block_hash(&self) -> &str {
        &self.block_hash
    }
    
    fn tx_hash(&self) -> &str {
        &self.tx_hash
    }
    
    fn timestamp(&self) -> std::time::SystemTime {
        self.timestamp
    }
    
    fn event_type(&self) -> &str {
        &self.event_type
    }
    
    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
} 