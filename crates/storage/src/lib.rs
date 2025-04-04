use std::sync::Arc;
use async_trait::async_trait;

use indexer_core::event::Event;
use indexer_common::{BlockStatus, Result};

pub mod rocks;
pub mod postgres;
pub mod migrations;
pub mod tests;
pub mod sync;

// Re-export repositories from postgres module
pub use postgres::repositories;

/// Re-export contract schema types for convenience
pub use migrations::schema::{
    ContractSchemaVersion, EventSchema, FieldSchema, FunctionSchema,
    ContractSchemaRegistry
};

/// Storage backend for indexer
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// Store an event
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()>;
    
    /// Get events by filters
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>>;
    
    /// Get the latest block height for a chain
    async fn get_latest_block(&self, chain: &str) -> Result<u64>;
    
    /// Get the latest block with a specific status for a chain
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64>;
    
    /// Update block status
    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()>;
    
    /// Get events with specific block status
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: BlockStatus) -> Result<Vec<Box<dyn Event>>>;
}

/// Default implementations for Storage trait methods
pub mod storage_defaults {
    use super::*;
    
    /// Default implementation for get_latest_block_with_status
    pub async fn get_latest_block_with_status(
        storage: &dyn Storage,
        chain: &str, 
        _status: BlockStatus
    ) -> Result<u64> {
        // Default implementation just returns the latest block
        // This is compatible with chain adapters that don't support block status
        storage.get_latest_block(chain).await
    }
    
    /// Default implementation for update_block_status
    pub async fn update_block_status(
        _storage: &dyn Storage,
        _chain: &str, 
        _block_number: u64, 
        _status: BlockStatus
    ) -> Result<()> {
        // Default implementation does nothing
        // This is compatible with storage backends that don't support block status
        Ok(())
    }
    
    /// Default implementation for get_events_with_status
    pub async fn get_events_with_status(
        storage: &dyn Storage,
        filters: Vec<EventFilter>, 
        _status: BlockStatus
    ) -> Result<Vec<Box<dyn Event>>> {
        // Default implementation ignores status
        // This is compatible with storage backends that don't support block status
        storage.get_events(filters).await
    }
}

/// Boxed storage
pub type BoxedStorage = Arc<dyn Storage>;

/// Event filter for querying events
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Chain filter
    pub chain: Option<String>,
    
    /// Block range filter (min, max)
    pub block_range: Option<(u64, u64)>,
    
    /// Time range filter (min, max) in unix seconds
    pub time_range: Option<(u64, u64)>,
    
    /// Event type filter
    pub event_types: Option<Vec<String>>,
    
    /// Result limit
    pub limit: Option<usize>,
    
    /// Result offset
    pub offset: Option<usize>,
} 