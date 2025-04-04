/// Cosmos chain service implementation
use std::sync::Arc;

use async_trait::async_trait;
use indexer_common::Result;
use indexer_core::event::Event;
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};
use tracing::debug;

mod event;
mod provider;
mod subscription;

pub use event::CosmosEvent;
use provider::CosmosProvider;
use subscription::{CosmosSubscription, CosmosSubscriptionConfig};

/// Cosmos event service
pub struct CosmosEventService {
    /// Chain ID
    chain_id: ChainId,
    
    /// RPC endpoint
    rpc_url: String,
    
    /// Polling interval in milliseconds
    polling_interval_ms: u64,
    
    /// Provider for interacting with the Cosmos node
    provider: Arc<CosmosProvider>,
}

impl CosmosEventService {
    /// Create a new Cosmos event service
    pub async fn new(chain_id: &str, options: impl Into<CosmosServiceOptions>) -> Result<Self> {
        let options = options.into();
        let rpc_url = options.rpc_url.unwrap_or_else(|| {
            format!("http://localhost:26657")
        });
        
        debug!("Created Cosmos service for chain: {}", chain_id);
        
        // Create provider
        let provider = Arc::new(CosmosProvider::new(rpc_url.clone()).await?);
        
        Ok(Self {
            chain_id: ChainId(chain_id.to_string()),
            rpc_url,
            polling_interval_ms: options.polling_interval_ms.unwrap_or(1000),
            provider,
        })
    }
}

/// Options for creating a Cosmos service
#[derive(Debug, Clone, Default)]
pub struct CosmosServiceOptions {
    /// RPC endpoint URL
    pub rpc_url: Option<String>,
    
    /// Polling interval in milliseconds
    pub polling_interval_ms: Option<u64>,
}

impl From<()> for CosmosServiceOptions {
    fn from(_: ()) -> Self {
        Self::default()
    }
}

impl From<&str> for CosmosServiceOptions {
    fn from(rpc_url: &str) -> Self {
        Self {
            rpc_url: Some(rpc_url.to_string()),
            ..Default::default()
        }
    }
}

impl From<String> for CosmosServiceOptions {
    fn from(rpc_url: String) -> Self {
        Self {
            rpc_url: Some(rpc_url),
            ..Default::default()
        }
    }
}

#[async_trait]
impl EventService for CosmosEventService {
    type EventType = CosmosEvent;
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    async fn get_events(&self, _filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        debug!("Getting Cosmos events (mock)");
        
        // Create a mock event for testing
        let event = CosmosEvent::new_mock();
        
        Ok(vec![Box::new(event)])
    }
    
    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> {
        debug!("Subscribing to Cosmos events");
        
        // Create subscription config
        let config = CosmosSubscriptionConfig {
            rpc_url: self.rpc_url.clone(),
            polling_interval_ms: self.polling_interval_ms,
            chain_id: self.chain_id.0.clone(),
        };
        
        // Create the subscription
        let subscription = CosmosSubscription::new(config).await?;
        
        Ok(Box::new(subscription))
    }
    
    async fn get_latest_block(&self) -> Result<u64> {
        self.provider.get_block_height().await
    }
} 