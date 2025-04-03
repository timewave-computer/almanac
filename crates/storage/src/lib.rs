use std::sync::Arc;
use async_trait::async_trait;

use indexer_core::event::Event;
use indexer_core::Result;

pub mod rocks;
pub mod postgres;
pub mod migrations;
#[cfg(test)]
mod tests;

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
}

/// Boxed storage
pub type BoxedStorage = Arc<dyn Storage>;

/// Filter for retrieving events from storage
pub struct EventFilter {
    /// Chain ID
    pub chain: Option<String>,
    
    /// Block number range
    pub block_range: Option<(u64, u64)>,
    
    /// Timestamp range
    pub time_range: Option<(u64, u64)>,
    
    /// Event types
    pub event_types: Option<Vec<String>>,
    
    /// Maximum number of events to return
    pub limit: Option<usize>,
    
    /// Offset for pagination
    pub offset: Option<usize>,
} 