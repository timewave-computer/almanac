//! Core storage trait definitions

use async_trait::async_trait;
use crate::{Result, Event, EventFilter};
use crate::types::BlockStatus;

/// Main storage trait that all storage implementations must implement
#[async_trait]
pub trait Storage: Send + Sync {
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
    
    /// Handle chain reorganization from a specific block
    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()>;
    
    /// Get the latest block before a specific block
    async fn get_latest_block_before(&self, chain: &str, _before_block: u64) -> Result<u64> {
        // Default implementation returns the latest block
        self.get_latest_block(chain).await
    }
}