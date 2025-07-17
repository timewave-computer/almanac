use std::sync::Arc;
use uuid::Uuid;
use tokio::sync::mpsc;

use super::event::Event;
use super::handler::EventFilter;

/// Unique identifier for a subscription
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(Uuid);

impl SubscriptionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

/// A subscription to events
pub struct Subscription {
    pub id: SubscriptionId,
    pub event_types: Vec<&'static str>,
    pub filter: Option<EventFilter>,
    pub sender: mpsc::UnboundedSender<Arc<dyn Event>>,
}

impl Subscription {
    /// Create a new subscription
    pub fn new(
        event_types: Vec<&'static str>,
        sender: mpsc::UnboundedSender<Arc<dyn Event>>,
    ) -> Self {
        Self {
            id: SubscriptionId::new(),
            event_types,
            filter: None,
            sender,
        }
    }
    
    /// Create a subscription with a filter
    pub fn with_filter<F>(
        event_types: Vec<&'static str>,
        filter: F,
        sender: mpsc::UnboundedSender<Arc<dyn Event>>,
    ) -> Self
    where
        F: Fn(&dyn Event) -> bool + Send + Sync + 'static,
    {
        Self {
            id: SubscriptionId::new(),
            event_types,
            filter: Some(Box::new(filter)),
            sender,
        }
    }
    
    /// Check if this subscription should receive an event
    pub fn matches(&self, event: &dyn Event) -> bool {
        // Check event type
        if !self.event_types.is_empty() && !self.event_types.contains(&event.event_type()) {
            return false;
        }
        
        // Apply filter if present
        if let Some(filter) = &self.filter {
            filter(event)
        } else {
            true
        }
    }
    
    /// Send an event to the subscriber
    pub fn send(&self, event: Arc<dyn Event>) -> Result<(), mpsc::error::SendError<Arc<dyn Event>>> {
        self.sender.send(event)
    }
}

/// Builder for creating subscriptions
pub struct SubscriptionBuilder {
    event_types: Vec<&'static str>,
    filter: Option<EventFilter>,
}

impl SubscriptionBuilder {
    pub fn new() -> Self {
        Self {
            event_types: Vec::new(),
            filter: None,
        }
    }
    
    /// Add event types to subscribe to
    pub fn event_types(mut self, types: Vec<&'static str>) -> Self {
        self.event_types = types;
        self
    }
    
    /// Add a single event type
    pub fn event_type(mut self, event_type: &'static str) -> Self {
        self.event_types.push(event_type);
        self
    }
    
    /// Add a filter function
    pub fn filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&dyn Event) -> bool + Send + Sync + 'static,
    {
        self.filter = Some(Box::new(filter));
        self
    }
    
    /// Build the subscription
    pub fn build(self, sender: mpsc::UnboundedSender<Arc<dyn Event>>) -> Subscription {
        Subscription {
            id: SubscriptionId::new(),
            event_types: self.event_types,
            filter: self.filter,
            sender,
        }
    }
}

impl Default for SubscriptionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::event::{SystemStartedEvent, EventMetadata};
    
    #[test]
    fn test_subscription_matching() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let subscription = Subscription::new(vec!["system.started"], tx);
        
        let event = SystemStartedEvent {
            metadata: EventMetadata::new("test"),
        };
        
        assert!(subscription.matches(&event));
    }
    
    #[test]
    fn test_subscription_filter() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let subscription = Subscription::with_filter(
            vec![],
            |event| event.event_type() == "system.started",
            tx,
        );
        
        let event = SystemStartedEvent {
            metadata: EventMetadata::new("test"),
        };
        
        assert!(subscription.matches(&event));
    }
    
    #[test]
    fn test_subscription_builder() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let subscription = SubscriptionBuilder::new()
            .event_type("system.started")
            .event_type("system.stopped")
            .filter(|_| true)
            .build(tx);
            
        assert_eq!(subscription.event_types.len(), 2);
        assert!(subscription.filter.is_some());
    }
}