/// Storage synchronization implementation for multi-store consistency
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time;
use futures::future::join_all;
use tracing::{debug, info, error, warn};

use indexer_common::{BlockStatus, Result, Error};
use indexer_core::event::Event;

use crate::{BoxedStorage, EventFilter};
use crate::rocks::RocksStorage;
use crate::postgres::PostgresStorage;

use chrono::Utc;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::mpsc::{self, Receiver, Sender};

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
    /// Create a new storage synchronizer with RocksDB as primary and PostgreSQL as secondary
    pub async fn new_rocks_postgres(
        rocks: Arc<RocksStorage>, 
        postgres: Arc<PostgresStorage>,
        config: SyncConfig
    ) -> Self {
        Self {
            primary: rocks as BoxedStorage,
            secondary: postgres as BoxedStorage,
            config,
            task_handle: RwLock::new(None),
            running: RwLock::new(false),
        }
    }
    
    /// Create a new storage synchronizer with PostgreSQL as primary and RocksDB as secondary
    pub async fn new_postgres_rocks(
        postgres: Arc<PostgresStorage>, 
        rocks: Arc<RocksStorage>,
        config: SyncConfig
    ) -> Self {
        Self {
            primary: postgres as BoxedStorage,
            secondary: rocks as BoxedStorage,
            config,
            task_handle: RwLock::new(None),
            running: RwLock::new(false),
        }
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
        let filter = EventFilter {
            chain: Some(chain.to_string()),
            block_range: Some((start_block, end_block)),
            time_range: None,
            event_types: None,
            limit: None,
            offset: None,
        };
        
        // Get events from primary
        let events = primary.get_events(vec![filter]).await?;
        
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
            
            futures.push(tokio::spawn(async move {
                secondary_clone.store_event(event_clone).await
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
    async fn get_block_status(
        storage: &BoxedStorage,
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
        
        secondary.store_event(event).await
    }

    /// Manually process an event
    pub async fn process_event(
        _storage: &BoxedStorage,
        _chain: &str,
        _block_number: u64
    ) -> Result<()> {
        // TODO: Implement event processing logic
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
} 