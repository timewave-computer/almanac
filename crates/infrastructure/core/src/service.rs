//! Service layer traits for Almanac infrastructure
//!
//! This module contains the core service traits that define the interface
//! for event services and subscriptions across different blockchain networks.

use crate::Result;
use crate::types::{ChainId, EventFilter, Event, UnifiedEvent};
use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;

/// Type alias for a boxed event service
pub type BoxedEventService = Arc<dyn EventService<EventType = UnifiedEvent>>;

/// Trait for blockchain event services
#[async_trait]
pub trait EventService: Send + Sync + Debug {
    /// Associated event type that this service handles
    type EventType: Event;
    
    /// Get the chain ID this service handles
    fn chain_id(&self) -> &ChainId;
    
    /// Get events matching the given filters
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>>;
    
    /// Get the latest block number
    async fn get_latest_block(&self) -> Result<u64>;
    
    /// Get the latest block number for a specific chain with status
    async fn get_latest_block_with_status(&self, chain: &str, status: crate::BlockStatus) -> Result<u64>;
    
    /// Subscribe to real-time events
    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>>;
}

/// Trait for event subscriptions - provides streaming access to events
#[async_trait]
pub trait EventSubscription: Send + Sync + Debug {
    /// Get the next event from the subscription
    async fn next(&mut self) -> Option<Box<dyn Event>>;
    
    /// Close the subscription and clean up resources
    async fn close(&mut self) -> Result<()>;
}

/// Trait object for event subscriptions
pub type BoxedEventSubscription = Box<dyn EventSubscription>;

/// Default implementation for EventService when no specific implementation is needed
#[derive(Debug)]
pub struct NoOpEventService {
    chain_id: ChainId,
}

impl NoOpEventService {
    pub fn new(chain_id: ChainId) -> Self {
        Self { chain_id }
    }
}

#[async_trait]
impl EventService for NoOpEventService {
    type EventType = UnifiedEvent;
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    async fn get_events(&self, _filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        Ok(Vec::new())
    }
    
    async fn get_latest_block(&self) -> Result<u64> {
        Ok(0)
    }
    
    async fn get_latest_block_with_status(&self, _chain: &str, _status: crate::BlockStatus) -> Result<u64> {
        Ok(0)
    }
    
    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> {
        Ok(Box::new(NoOpEventSubscription))
    }
}

/// Default implementation for EventSubscription when no specific implementation is needed
#[derive(Debug)]
pub struct NoOpEventSubscription;

#[async_trait]
impl EventSubscription for NoOpEventSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        None
    }
    
    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_noop_event_service() {
        let service = NoOpEventService::new(ChainId::new("test"));
        assert_eq!(service.chain_id().as_str(), "test");
        
        let events = service.get_events(vec![]).await.unwrap();
        assert!(events.is_empty());
        
        let latest_block = service.get_latest_block().await.unwrap();
        assert_eq!(latest_block, 0);
    }
    
    #[tokio::test]
    async fn test_noop_event_subscription() {
        let mut subscription = NoOpEventSubscription;
        assert!(subscription.next().await.is_none());
        assert!(subscription.close().await.is_ok());
    }
} 