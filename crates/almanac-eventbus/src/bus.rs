use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};

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

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Channel is full")]
    ChannelFull,
}

/// Simple in-process event bus using channels
pub struct EventBus {
    handlers: Arc<RwLock<HashMap<TypeId, Vec<BoxedHandler>>>>,
    subscriptions: Arc<RwLock<HashMap<SubscriptionId, Subscription>>>,
    event_queue: EventSender,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Event sender wrapper to support both bounded and unbounded channels
#[derive(Clone)]
pub enum EventSender {
    /// Unbounded channel sender
    Unbounded(mpsc::UnboundedSender<Arc<dyn Event>>),
    /// Bounded channel sender
    Bounded(mpsc::Sender<Arc<dyn Event>>),
}

impl EventSender {
    /// Send an event through the channel
    pub async fn send(&self, event: Arc<dyn Event>) -> Result<(), EventBusError> {
        match self {
            EventSender::Unbounded(sender) => {
                sender.send(event).map_err(|_| EventBusError::ChannelClosed)
            }
            EventSender::Bounded(sender) => sender
                .send(event)
                .await
                .map_err(|_| EventBusError::ChannelClosed),
        }
    }

    /// Try to send an event without blocking (bounded channels only)
    pub fn try_send(&self, event: Arc<dyn Event>) -> Result<(), EventBusError> {
        match self {
            EventSender::Unbounded(sender) => {
                sender.send(event).map_err(|_| EventBusError::ChannelClosed)
            }
            EventSender::Bounded(sender) => sender.try_send(event).map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => EventBusError::ChannelFull,
                mpsc::error::TrySendError::Closed(_) => EventBusError::ChannelClosed,
            }),
        }
    }
}

/// Event receiver wrapper to support both bounded and unbounded channels  
pub enum EventReceiver {
    /// Unbounded channel receiver
    Unbounded(mpsc::UnboundedReceiver<Arc<dyn Event>>),
    /// Bounded channel receiver
    Bounded(mpsc::Receiver<Arc<dyn Event>>),
}

impl EventReceiver {
    /// Receive the next event
    pub async fn recv(&mut self) -> Option<Arc<dyn Event>> {
        match self {
            EventReceiver::Unbounded(receiver) => receiver.recv().await,
            EventReceiver::Bounded(receiver) => receiver.recv().await,
        }
    }
}

impl EventBus {
    /// Create a new event bus with unbounded channels
    pub fn new() -> Self {
        Self::with_buffer_size(None)
    }

    /// Create a new event bus with bounded channels of specified size
    pub fn with_buffer_size(buffer_size: Option<usize>) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let (event_queue, event_rx) = match buffer_size {
            Some(size) => {
                let (tx, rx) = mpsc::channel(size);
                (EventSender::Bounded(tx), EventReceiver::Bounded(rx))
            }
            None => {
                let (tx, rx) = mpsc::unbounded_channel();
                (EventSender::Unbounded(tx), EventReceiver::Unbounded(rx))
            }
        };

        let bus = Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_queue,
            shutdown_tx: Some(shutdown_tx),
        };

        // Start the event processing task
        let handlers = bus.handlers.clone();
        let subscriptions = bus.subscriptions.clone();
        tokio::spawn(Self::process_events(
            event_rx,
            shutdown_rx,
            handlers,
            subscriptions,
        ));

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
    pub async fn subscribe(
        &self,
        subscription: Subscription,
    ) -> Result<SubscriptionId, EventBusError> {
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
        self.event_queue.send(event).await
    }

    /// Publish a boxed event
    pub async fn publish_boxed(&self, event: Box<dyn Event>) -> Result<(), EventBusError> {
        let event: Arc<dyn Event> = Arc::from(event);
        self.event_queue.send(event).await
    }

    /// Try to publish an event without blocking (useful for bounded channels)
    pub fn try_publish<E: Event + 'static>(&self, event: E) -> Result<(), EventBusError> {
        let event: Arc<dyn Event> = Arc::new(event);
        self.event_queue.try_send(event)
    }

    /// Try to publish a boxed event without blocking
    pub fn try_publish_boxed(&self, event: Box<dyn Event>) -> Result<(), EventBusError> {
        let event: Arc<dyn Event> = Arc::from(event);
        self.event_queue.try_send(event)
    }

    /// Process events from the queue
    async fn process_events(
        mut event_rx: EventReceiver,
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
        Self { buffer_size: None }
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }

    pub fn build(self) -> EventBus {
        EventBus::with_buffer_size(self.buffer_size)
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
    use crate::event::{EventMetadata, SystemStartedEvent};
    use crate::handler::AsyncFnHandler;
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
        let received =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), rx.recv()).await;

        assert!(received.is_ok());
        assert!(received.unwrap().is_some());
    }
}
