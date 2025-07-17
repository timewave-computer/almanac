use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use thiserror::Error;

use super::event::Event;
use super::handler::BoxedHandler;
use super::subscription::{Subscription, SubscriptionId};

#[derive(Error, Debug)]
pub enum EventBusError {
    #[error("Handler error: {0}")]
    HandlerError(String),
    
    #[error("Subscription error: {0}")]
    SubscriptionError(String),
    
    #[error("Event publishing failed: {0}")]
    PublishError(String),
}

/// Simple in-process event bus using channels
pub struct EventBus {
    handlers: Arc<RwLock<HashMap<TypeId, Vec<BoxedHandler>>>>,
    subscriptions: Arc<RwLock<HashMap<SubscriptionId, Subscription>>>,
    event_queue: mpsc::UnboundedSender<Arc<dyn Event>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        let bus = Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_queue: event_tx,
            shutdown_tx: Some(shutdown_tx),
        };
        
        // Start the event processing task
        let handlers = bus.handlers.clone();
        let subscriptions = bus.subscriptions.clone();
        tokio::spawn(Self::process_events(event_rx, shutdown_rx, handlers, subscriptions));
        
        bus
    }
    
    /// Register an event handler
    pub async fn register_handler(&self, handler: BoxedHandler) -> Result<(), EventBusError> {
        let mut handlers = self.handlers.write().await;
        
        if let Some(type_id) = handler.handles().into_iter().next() {
            handlers
                .entry(type_id)
                .or_insert_with(Vec::new)
                .push(handler);
        }
        
        Ok(())
    }
    
    /// Subscribe to events
    pub async fn subscribe(&self, subscription: Subscription) -> Result<SubscriptionId, EventBusError> {
        let id = subscription.id;
        self.subscriptions.write().await.insert(id, subscription);
        Ok(id)
    }
    
    /// Unsubscribe from events
    pub async fn unsubscribe(&self, id: SubscriptionId) -> Result<(), EventBusError> {
        self.subscriptions.write().await.remove(&id);
        Ok(())
    }
    
    /// Publish an event
    pub async fn publish<E: Event + 'static>(&self, event: E) -> Result<(), EventBusError> {
        let event: Arc<dyn Event> = Arc::new(event);
        self.event_queue
            .send(event)
            .map_err(|e| EventBusError::PublishError(e.to_string()))
    }
    
    /// Publish a boxed event
    pub async fn publish_boxed(&self, event: Box<dyn Event>) -> Result<(), EventBusError> {
        let event: Arc<dyn Event> = Arc::from(event);
        self.event_queue
            .send(event)
            .map_err(|e| EventBusError::PublishError(e.to_string()))
    }
    
    /// Process events from the queue
    async fn process_events(
        mut event_rx: mpsc::UnboundedReceiver<Arc<dyn Event>>,
        mut shutdown_rx: mpsc::Receiver<()>,
        handlers: Arc<RwLock<HashMap<TypeId, Vec<BoxedHandler>>>>,
        subscriptions: Arc<RwLock<HashMap<SubscriptionId, Subscription>>>,
    ) {
        loop {
            tokio::select! {
                Some(event) = event_rx.recv() => {
                    // Process handlers
                    Self::process_handlers(&event, &handlers).await;
                    
                    // Process subscriptions
                    Self::process_subscriptions(&event, &subscriptions).await;
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    }
    
    /// Process event with handlers
    async fn process_handlers(
        event: &Arc<dyn Event>,
        handlers: &Arc<RwLock<HashMap<TypeId, Vec<BoxedHandler>>>>,
    ) {
        let type_id = super::event::Event::type_id(event.as_ref());
        let handlers_map = handlers.read().await;
        
        if let Some(event_handlers) = handlers_map.get(&type_id) {
            for handler in event_handlers {
                let event_clone = event.clone();
                
                // Handle the event
                if let Err(e) = handler.handle(event_clone).await {
                    tracing::error!("Handler error: {}", e);
                }
            }
        }
    }
    
    /// Process event with subscriptions
    async fn process_subscriptions(
        event: &Arc<dyn Event>,
        subscriptions_arc: &Arc<RwLock<HashMap<SubscriptionId, Subscription>>>,
    ) {
        let subscriptions = subscriptions_arc.read().await;
        let mut dead_subscriptions = Vec::new();
        
        for (id, subscription) in subscriptions.iter() {
            if subscription.matches(event.as_ref()) && subscription.send(event.clone()).is_err() {
                // Receiver dropped, mark for removal
                dead_subscriptions.push(*id);
            }
        }
        
        // Remove dead subscriptions
        if !dead_subscriptions.is_empty() {
            drop(subscriptions);
            let mut subs_write = subscriptions_arc.write().await;
            for id in dead_subscriptions {
                subs_write.remove(&id);
            }
        }
    }
    
    /// Shutdown the event bus
    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating an event bus with configuration
pub struct EventBusBuilder {
    buffer_size: Option<usize>,
}

impl EventBusBuilder {
    pub fn new() -> Self {
        Self {
            buffer_size: None,
        }
    }
    
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }
    
    pub fn build(self) -> EventBus {
        // For now, ignore buffer_size as we're using unbounded channels
        // In production, we'd use bounded channels with the specified size
        EventBus::new()
    }
}

impl Default for EventBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::event::{SystemStartedEvent, EventMetadata};
    use crate::event_bus::handler::AsyncFnHandler;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[tokio::test]
    async fn test_event_publishing() {
        let bus = EventBus::new();
        
        let event = SystemStartedEvent {
            metadata: EventMetadata::new("test"),
        };
        
        assert!(bus.publish(event).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_event_handler() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let handler = AsyncFnHandler::new(
            move |_event| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
            vec![TypeId::of::<SystemStartedEvent>()],
        );
        
        bus.register_handler(Box::new(handler)).await.unwrap();
        
        let event = SystemStartedEvent {
            metadata: EventMetadata::new("test"),
        };
        
        bus.publish(event).await.unwrap();
        
        // Give handler time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
    
    #[tokio::test]
    async fn test_event_subscription() {
        let bus = EventBus::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        let subscription = Subscription::new(vec!["system.started"], tx);
        let _id = bus.subscribe(subscription).await.unwrap();
        
        let event = SystemStartedEvent {
            metadata: EventMetadata::new("test"),
        };
        
        bus.publish(event).await.unwrap();
        
        // Check if event was received
        let received = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            rx.recv()
        ).await;
        
        assert!(received.is_ok());
        assert!(received.unwrap().is_some());
    }
}