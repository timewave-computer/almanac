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

// Add structs to pass Valence data around
// Could also define these in indexer-cosmos or a new valence-types crate

#[derive(Debug, Clone)]
pub struct ValenceAccountInfo {
    pub id: String,                         // Unique ID (e.g., chain_id + contract_address)
    pub chain_id: String,
    pub contract_address: String,
    pub created_at_block: u64,
    pub created_at_tx: String,
    pub current_owner: Option<String>,
    pub pending_owner: Option<String>,
    pub pending_owner_expiry: Option<u64>,
    pub last_updated_block: u64,
    pub last_updated_tx: String,
}

#[derive(Debug, Clone)]
pub struct ValenceAccountLibrary {
    pub account_id: String,
    pub library_address: String,
    pub approved_at_block: u64,
    pub approved_at_tx: String,
}

#[derive(Debug, Clone)]
pub struct ValenceAccountExecution {
    pub account_id: String,
    pub chain_id: String,
    pub block_number: u64,
    pub tx_hash: String,
    pub executor_address: String,
    pub message_index: i32, // Assuming i32 fits message index
    pub correlated_event_ids: Option<Vec<String>>,
    pub raw_msgs: Option<serde_json::Value>,
    pub payload: Option<String>,
    pub executed_at: std::time::SystemTime,
}

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

    // --- Valence Account Storage Methods ---

    /// Stores the initial information for a newly instantiated Valence account.
    /// Also stores the initial list of approved libraries.
    async fn store_valence_account_instantiation(
        &self,
        account_info: ValenceAccountInfo,
        initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()>;

    /// Adds a library to an existing Valence account's approved list.
    async fn store_valence_library_approval(
        &self,
        account_id: &str,
        library_info: ValenceAccountLibrary,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()>;

    /// Removes a library from an existing Valence account's approved list.
    async fn store_valence_library_removal(
        &self,
        account_id: &str,
        library_address: &str,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()>;

    /// Updates the ownership details (owner, pending owner, expiry) of a Valence account.
    async fn store_valence_ownership_update(
        &self,
        account_id: &str,
        new_owner: Option<String>,
        new_pending_owner: Option<String>,
        new_pending_expiry: Option<u64>,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()>;

    /// Stores a record of an execution triggered by a Valence account.
    async fn store_valence_execution(
        &self,
        execution_info: ValenceAccountExecution,
    ) -> Result<()>;

    /// Retrieves the current state of a Valence account (owner, libraries).
    /// Needed for calculating updates (e.g., removing old owner index).
    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>>;
}

/// Represents the current state needed for updates (primarily for RocksDB logic)
#[derive(Debug, Clone)]
pub struct ValenceAccountState {
    pub current_owner: Option<String>,
    pub libraries: Vec<String>,
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