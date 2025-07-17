use std::any::{Any, TypeId};
use std::time::SystemTime;
use uuid::Uuid;

/// Base trait for all events in the system
pub trait Event: Any + Send + Sync {
    /// Get the event type name
    fn event_type(&self) -> &'static str;
    
    /// Get the event timestamp
    fn timestamp(&self) -> SystemTime;
    
    /// Get the event ID
    fn event_id(&self) -> Uuid;
    
    /// Clone the event as a boxed trait object
    fn clone_event(&self) -> Box<dyn Event>;
    
    /// Get the TypeId for downcasting
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// Helper trait for downcasting events
pub trait EventDowncast: Event {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Event + 'static> EventDowncast for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Metadata associated with an event
#[derive(Debug, Clone)]
pub struct EventMetadata {
    pub event_id: Uuid,
    pub timestamp: SystemTime,
    pub correlation_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub source: String,
}

impl EventMetadata {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            correlation_id: None,
            causation_id: None,
            source: source.into(),
        }
    }
    
    pub fn with_correlation(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
    
    pub fn with_causation(mut self, causation_id: Uuid) -> Self {
        self.causation_id = Some(causation_id);
        self
    }
}

/// Type identifier for events
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EventType(pub &'static str);

impl EventType {
    pub fn new(name: &'static str) -> Self {
        Self(name)
    }
    
    pub fn name(&self) -> &'static str {
        self.0
    }
}

/// Common system events
#[derive(Debug, Clone)]
pub struct SystemStartedEvent {
    pub metadata: EventMetadata,
}

impl Event for SystemStartedEvent {
    fn event_type(&self) -> &'static str {
        "system.started"
    }
    
    fn timestamp(&self) -> SystemTime {
        self.metadata.timestamp
    }
    
    fn event_id(&self) -> Uuid {
        self.metadata.event_id
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct SystemStoppedEvent {
    pub metadata: EventMetadata,
}

impl Event for SystemStoppedEvent {
    fn event_type(&self) -> &'static str {
        "system.stopped"
    }
    
    fn timestamp(&self) -> SystemTime {
        self.metadata.timestamp
    }
    
    fn event_id(&self) -> Uuid {
        self.metadata.event_id
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Module communication events
#[derive(Debug, Clone)]
pub struct ModuleStartedEvent {
    pub metadata: EventMetadata,
    pub module_name: String,
}

impl Event for ModuleStartedEvent {
    fn event_type(&self) -> &'static str {
        "module.started"
    }
    
    fn timestamp(&self) -> SystemTime {
        self.metadata.timestamp
    }
    
    fn event_id(&self) -> Uuid {
        self.metadata.event_id
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ModuleStoppedEvent {
    pub metadata: EventMetadata,
    pub module_name: String,
}

impl Event for ModuleStoppedEvent {
    fn event_type(&self) -> &'static str {
        "module.stopped"
    }
    
    fn timestamp(&self) -> SystemTime {
        self.metadata.timestamp
    }
    
    fn event_id(&self) -> Uuid {
        self.metadata.event_id
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Service discovery events
#[derive(Debug, Clone)]
pub struct ServiceRegisteredEvent {
    pub metadata: EventMetadata,
    pub service_name: String,
    pub service_type: String,
}

impl Event for ServiceRegisteredEvent {
    fn event_type(&self) -> &'static str {
        "service.registered"
    }
    
    fn timestamp(&self) -> SystemTime {
        self.metadata.timestamp
    }
    
    fn event_id(&self) -> Uuid {
        self.metadata.event_id
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ServiceUnregisteredEvent {
    pub metadata: EventMetadata,
    pub service_name: String,
}

impl Event for ServiceUnregisteredEvent {
    fn event_type(&self) -> &'static str {
        "service.unregistered"
    }
    
    fn timestamp(&self) -> SystemTime {
        self.metadata.timestamp
    }
    
    fn event_id(&self) -> Uuid {
        self.metadata.event_id
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_metadata() {
        let metadata = EventMetadata::new("test")
            .with_correlation(Uuid::new_v4())
            .with_causation(Uuid::new_v4());
            
        assert_eq!(metadata.source, "test");
        assert!(metadata.correlation_id.is_some());
        assert!(metadata.causation_id.is_some());
    }
    
    #[test]
    fn test_system_events() {
        let event = SystemStartedEvent {
            metadata: EventMetadata::new("test"),
        };
        
        assert_eq!(event.event_type(), "system.started");
        assert_eq!(event.event_id(), event.metadata.event_id);
    }
}