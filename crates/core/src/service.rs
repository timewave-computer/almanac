use async_trait::async_trait;
use std::sync::Arc;

use crate::event::Event;
use crate::types::{ChainId, EventFilter};
use indexer_common::{BlockStatus, Result};

/// Define common interfaces for chain services

/// Trait for event subscription
#[async_trait]
pub trait EventSubscription: Send + Sync {
    /// Wait for the next event
    async fn next(&mut self) -> Option<Box<dyn Event>>;

    /// Close the subscription
    async fn close(&mut self) -> Result<()>;
}

/// Trait for event services
#[async_trait]
pub trait EventService: Send + Sync {
    /// The type of event handled by this service
    type EventType: Event + 'static;

    /// Get the chain ID
    fn chain_id(&self) -> &ChainId;

    /// Get the latest block with the specified status
    async fn get_latest_block_with_status(&self, _chain: &str, _status: BlockStatus) -> Result<u64> {
        self.get_latest_block().await
    }

    /// Get events matching the given filters and status
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, _status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        self.get_events(filters).await
    }

    /// Get events matching the given filters
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>>;

    /// Subscribe to new events
    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>>;

    /// Get the latest block number
    async fn get_latest_block(&self) -> Result<u64>;
}

/// Type alias for a boxed event service
pub type BoxedEventService = Arc<dyn EventService<EventType = Box<dyn Event>>>;

/// Registry for event services
pub trait EventServiceRegistry: Send + Sync {
    /// Register an event service
    fn register_service(&mut self, chain_id: ChainId, service: BoxedEventService);

    /// Get an event service for the given chain
    fn get_service(&self, chain_id: &str) -> Option<BoxedEventService>;

    /// Get all registered services
    fn get_services(&self) -> Vec<BoxedEventService>;

    /// Remove a service
    fn remove_service(&mut self, chain_id: &str) -> Option<BoxedEventService>;
} 