use async_trait::async_trait;
use std::sync::Arc;

use crate::event::{Event, EventContainer};
use crate::types::{ChainId, EventFilter};
use crate::Result;

/// Service for fetching events from a chain
#[async_trait]
pub trait EventService: Send + Sync + 'static {
    /// The type of events this service handles
    type EventType: Send + Sync;

    /// Get the chain ID this service is for
    fn chain_id(&self) -> &ChainId;

    /// Get events based on the provided filter
    async fn get_events(&self, filter: EventFilter) -> Result<Vec<Box<dyn Event>>>;

    /// Subscribe to new events
    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>>;

    /// Get the latest block number
    async fn get_latest_block(&self) -> Result<u64>;
}

/// Type alias for a boxed event service
pub type BoxedEventService = Arc<dyn EventService<EventType = Box<dyn Event>>>;

/// Subscription to events
#[async_trait]
pub trait EventSubscription: Send + Sync + 'static {
    /// Wait for the next event
    async fn next(&mut self) -> Option<Box<dyn Event>>;

    /// Close the subscription
    async fn close(&mut self) -> Result<()>;
}

/// Registry of event services
#[async_trait]
pub trait EventServiceRegistry: Send + Sync + 'static {
    /// Register an event service
    async fn register(&mut self, service: BoxedEventService) -> Result<()>;

    /// Get an event service by chain ID
    async fn get(&self, chain_id: &ChainId) -> Result<BoxedEventService>;

    /// Get all registered event services
    async fn get_all(&self) -> Result<Vec<BoxedEventService>>;

    /// Remove an event service by chain ID
    async fn remove(&mut self, chain_id: &ChainId) -> Result<()>;
} 