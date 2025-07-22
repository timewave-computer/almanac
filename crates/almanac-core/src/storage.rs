//! CQRS-based storage trait definitions
//!
//! This module defines separate read and write responsibilities for storage,
//! enabling Command Query Responsibility Segregation (CQRS) patterns.

use crate::types::BlockStatus;
use crate::{Event, EventFilter, Result};
use async_trait::async_trait;

/// Storage writer trait for write operations (Command side of CQRS)
#[async_trait]
pub trait StorageWriter: Send + Sync + 'static {
    /// Store an event
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()>;

    /// Store multiple events in a batch
    async fn store_events_batch(&self, events: Vec<Box<dyn Event>>) -> Result<()>;

    /// Update block status
    async fn update_block_status(
        &self,
        chain: &str,
        block_number: u64,
        status: BlockStatus,
    ) -> Result<()>;

    /// Handle chain reorganization from a specific block
    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()>;

    /// Health check for write operations
    async fn health_check(&self) -> Result<()>;
}

/// Storage reader trait for read operations (Query side of CQRS)
#[async_trait]
pub trait StorageReader: Send + Sync + 'static {
    /// Get events by filters
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>>;

    /// Get the latest block height for a chain
    async fn get_latest_block(&self, chain: &str) -> Result<u64>;

    /// Get the latest block with a specific status for a chain
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64>;

    /// Get events with specific block status
    async fn get_events_with_status(
        &self,
        filters: Vec<EventFilter>,
        status: BlockStatus,
    ) -> Result<Vec<Box<dyn Event>>>;

    /// Get the latest block before a specific block
    async fn get_latest_block_before(&self, chain: &str, before_block: u64) -> Result<u64>;

    /// Health check for read operations
    async fn health_check(&self) -> Result<()>;
}

/// Combined storage trait for implementations that support both read and write
/// This is a convenience trait for storage backends that can handle both operations
pub trait Storage: StorageWriter + StorageReader {}

// Blanket implementation for any type that implements both traits
impl<T> Storage for T where T: StorageWriter + StorageReader {}
