use async_trait::async_trait;
use std::sync::Arc;

use crate::event::Event;
use crate::types::{ChainId, EventFilter};
use indexer_pipeline::{BlockStatus, Result};

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

/// Trait for services that work with boxed events
#[async_trait]
pub trait BoxedEventServiceTrait: Send + Sync {
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

/// Type alias for a boxed event service that works with boxed events
pub type BoxedEventService = Arc<dyn BoxedEventServiceTrait>;

/// Generic wrapper to convert EventService to BoxedEventServiceTrait
pub struct EventServiceWrapper<T> {
    service: Arc<T>,
}

impl<T> EventServiceWrapper<T> {
    pub fn new(service: Arc<T>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl<T> BoxedEventServiceTrait for EventServiceWrapper<T>
where
    T: EventService + Send + Sync + 'static,
{
    fn chain_id(&self) -> &ChainId {
        self.service.chain_id()
    }

    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        self.service.get_latest_block_with_status(chain, status).await
    }

    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        self.service.get_events_with_status(filters, status).await
    }

    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        self.service.get_events(filters).await
    }

    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> {
        self.service.subscribe().await
    }

    async fn get_latest_block(&self) -> Result<u64> {
        self.service.get_latest_block().await
    }
}

/// Helper function to wrap an EventService into a BoxedEventService
pub fn wrap_event_service<T>(service: Arc<T>) -> BoxedEventService
where
    T: EventService + Send + Sync + 'static,
{
    Arc::new(EventServiceWrapper::new(service))
}

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