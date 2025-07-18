//! Chain abstraction traits

use async_trait::async_trait;
use almanac_core::{Event, Result};

/// Chain identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chain {
    Ethereum,
    Cosmos(String), // Chain ID
}

/// Chain client trait
#[async_trait]
pub trait ChainClient: Send + Sync {
    /// Get the chain identifier
    fn chain(&self) -> Chain;
    
    /// Get the latest block number
    async fn get_latest_block(&self) -> Result<u64>;
    
    /// Get events from a block range
    async fn get_events(&self, from_block: u64, to_block: u64) -> Result<Vec<Box<dyn Event>>>;
    
    /// Subscribe to new events
    async fn subscribe_events(&self) -> Result<Box<dyn futures::Stream<Item = Box<dyn Event>> + Send>>;
}