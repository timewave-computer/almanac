/// Indexing pipeline and common types for processing blockchain events
///
/// This module contains the pipeline abstractions for processing blockchain events,
/// including service traits and registries.
use crate::{Error, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// A service that processes events from a blockchain
#[async_trait]
pub trait EventService: Send + Sync + 'static {
    /// Get the chain ID for this service
    fn chain_id(&self) -> &str;
    
    /// Start processing events
    async fn start(&self) -> Result<()>;
    
    /// Stop processing events
    async fn stop(&self) -> Result<()>;
    
    /// Current processing status
    async fn status(&self) -> Result<String>;
}

/// A factory for creating event services
pub trait EventServiceFactory: Send + Sync + 'static {
    /// Create a new event service for the given chain
    fn create_service(&self, chain_id: &str) -> Result<Arc<dyn EventService>>;
    
    /// Get all supported chains
    fn supported_chains(&self) -> Vec<String>;
    
    /// Check if a chain is supported
    fn is_supported(&self, chain_id: &str) -> bool {
        self.supported_chains().contains(&chain_id.to_string())
    }
}

/// A registry of event service factories
#[derive(Default)]
pub struct EventServiceRegistry {
    factories: Vec<Arc<dyn EventServiceFactory>>,
}

impl EventServiceRegistry {
    /// Create a new event service registry
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }
    
    /// Register a factory
    pub fn register(&mut self, factory: Arc<dyn EventServiceFactory>) {
        self.factories.push(factory);
    }
    
    /// Create a new event service for the given chain
    pub fn create_service(&self, chain_id: &str) -> Result<Arc<dyn EventService>> {
        for factory in &self.factories {
            if factory.is_supported(chain_id) {
                return factory.create_service(chain_id);
            }
        }
        
        Err(Error::missing_service(chain_id))
    }
    
    /// Get all supported chains
    pub fn supported_chains(&self) -> Vec<String> {
        let mut chains = Vec::new();
        for factory in &self.factories {
            chains.extend(factory.supported_chains());
        }
        chains
    }
} 