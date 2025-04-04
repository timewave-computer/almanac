/// RocksDB storage implementation
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use indexer_common::{BlockStatus, Error, Result};
use indexer_core::event::Event;
use rocksdb::{Options, DB, WriteBatch};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::EventFilter;
use crate::Storage;

/// Configuration for RocksDB storage
#[derive(Debug, Clone)]
pub struct RocksConfig {
    /// Path to the database
    pub path: String,
    
    /// Whether to create if missing
    pub create_if_missing: bool,
}

impl Default for RocksConfig {
    fn default() -> Self {
        Self {
            path: "data/rocksdb".to_string(),
            create_if_missing: true,
        }
    }
}

/// A key in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    /// Key namespace
    pub namespace: String,
    
    /// Key identifier
    pub id: String,
}

impl Key {
    /// Create a new key
    pub fn new(namespace: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            id: id.into(),
        }
    }
    
    /// Convert to byte string for storage
    pub fn to_bytes(&self) -> Vec<u8> {
        format!("{}:{}", self.namespace, self.id).into_bytes()
    }
    
    /// Create from byte string
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let s = String::from_utf8(bytes.to_vec())
            .map_err(|_| Error::generic("Invalid key format"))?;
        
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(Error::generic("Invalid key format"));
        }
        
        Ok(Self {
            namespace: parts[0].to_string(),
            id: parts[1].to_string(),
        })
    }
}

/// RocksDB storage
pub struct RocksStorage {
    /// Database instance
    db: Arc<DB>,
}

#[async_trait]
impl Storage for RocksStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        let key = Key::new("events", event.id());
        
        // Convert event to JSON for storage
        let event_data = serde_json::to_string(&EventData {
            id: event.id().to_string(),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp().duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event_type: event.event_type().to_string(),
            raw_data: event.raw_data().to_vec(),
        })?;
        
        self.put(&key, event_data.as_bytes())?;
        
        // Update block height
        let block_key = Key::new("blocks", &format!("{}:{}", event.chain(), event.block_number()));
        self.put(&block_key, event.block_hash().as_bytes())?;
        
        Ok(())
    }
    
    async fn get_events(&self, _filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        // This is a placeholder implementation
        // In a real implementation, we would:
        // 1. Iterate through the database with a specific prefix
        // 2. Filter events based on the provided filters
        // 3. Convert each event to the appropriate type
        
        info!("Getting events from RocksDB (mock implementation)");
        
        // Return an empty vector for now
        Ok(Vec::new())
    }
    
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        // This is a placeholder implementation
        // In a real implementation, we would:
        // 1. Use a special key to store the latest block for each chain
        // 2. Retrieve and parse that value
        
        info!("Getting latest block from RocksDB for chain {} (mock implementation)", chain);
        
        // Return a mock value for now
        Ok(0)
    }
    
    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        // Convert the BlockStatus enum to a string representation
        let status_str = match status {
            BlockStatus::Confirmed => "confirmed",
            BlockStatus::Safe => "safe",
            BlockStatus::Justified => "justified",
            BlockStatus::Finalized => "finalized",
        };
        
        let key = Key::new("block_status", &format!("{}:{}", chain, block_number));
        self.put(&key, status_str.as_bytes())?;
        
        Ok(())
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        // This is a placeholder implementation
        // In a real implementation, we would query for the latest block with the given status
        
        info!("Getting latest block with status {:?} for chain {} (mock implementation)", status, chain);
        
        // For now, just return the latest block without filtering by status
        self.get_latest_block(chain).await
    }
    
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        // This is a placeholder implementation
        // In a real implementation, we would:
        // 1. Get events with the basic filters
        // 2. Filter out events from blocks that don't have the required status
        
        info!("Getting events with status {:?} (mock implementation)", status);
        
        // For now, just return all events without filtering by status
        self.get_events(filters).await
    }
}

impl RocksStorage {
    /// Create a new RocksDB storage instance
    pub fn new(config: RocksConfig) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(config.create_if_missing);
        
        let db = DB::open(&opts, Path::new(&config.path))
            .map_err(|e| Error::generic(format!("Failed to open RocksDB: {}", e)))?;
        
        Ok(Self {
            db: Arc::new(db),
        })
    }
    
    /// Get a value from storage
    pub fn get(&self, key: &Key) -> Result<Option<Vec<u8>>> {
        let result = self.db.get(key.to_bytes())
            .map_err(|e| Error::generic(format!("Failed to get from RocksDB: {}", e)))?;
        
        Ok(result)
    }
    
    /// Put a value in storage
    pub fn put(&self, key: &Key, value: &[u8]) -> Result<()> {
        self.db.put(key.to_bytes(), value)
            .map_err(|e| Error::generic(format!("Failed to put to RocksDB: {}", e)))?;
        
        Ok(())
    }
    
    /// Delete a value from storage
    pub fn delete(&self, key: &Key) -> Result<()> {
        self.db.delete(key.to_bytes())
            .map_err(|e| Error::generic(format!("Failed to delete from RocksDB: {}", e)))?;
        
        Ok(())
    }
}

/// Event data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventData {
    /// Event ID
    pub id: String,
    
    /// Chain ID
    pub chain: String,
    
    /// Block number
    pub block_number: u64,
    
    /// Block hash
    pub block_hash: String,
    
    /// Transaction hash
    pub tx_hash: String,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// Event type
    pub event_type: String,
    
    /// Raw event data
    pub raw_data: Vec<u8>,
}

/// A wrapper around WriteBatch that works with our Key type
pub struct KeyBatch {
    batch: WriteBatch,
}

impl KeyBatch {
    /// Create a new batch
    pub fn new() -> Self {
        Self {
            batch: WriteBatch::default(),
        }
    }
    
    /// Put a key-value pair
    pub fn put(&mut self, key: &Key, value: &[u8]) -> &mut Self {
        self.batch.put(key.to_bytes(), value);
        self
    }
    
    /// Delete a key
    pub fn delete(&mut self, key: &Key) -> &mut Self {
        self.batch.delete(key.to_bytes());
        self
    }
    
    /// Get the inner WriteBatch
    pub fn inner(self) -> WriteBatch {
        self.batch
    }
}

impl RocksStorage {
    /// Create a new write batch for atomically writing multiple values
    pub fn create_write_batch(&self) -> KeyBatch {
        KeyBatch::new()
    }
    
    /// Write a batch of changes atomically
    pub fn write_batch(&self, batch: KeyBatch) -> Result<()> {
        self.db.write(batch.inner())
            .map_err(|e| Error::generic(format!("Failed to write batch to RocksDB: {}", e)))?;
        
        Ok(())
    }
}