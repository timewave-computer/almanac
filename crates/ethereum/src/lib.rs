use async_trait::async_trait;
use ethers::providers::{Http, Provider, Ws};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use indexer_core::event::{Event, EventMetadata};
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};
use indexer_core::{Error, Result};

pub mod event;
pub mod provider;
pub mod subscription;

use event::EthereumEvent;
use provider::EthereumProvider;
use subscription::EthereumSubscription;

/// Implementation of the EventService trait for Ethereum
pub struct EthereumEventService {
    /// Chain ID
    chain_id: ChainId,
    
    /// Ethereum provider
    provider: EthereumProvider,
}

impl EthereumEventService {
    /// Create a new Ethereum event service
    pub async fn new(chain_id: ChainId, rpc_url: &str) -> Result<Self> {
        let provider = if rpc_url.starts_with("ws") {
            let ws = Ws::connect(rpc_url).await
                .map_err(|e| Error::chain(format!("Failed to connect to WebSocket: {}", e)))?;
            EthereumProvider::Websocket(Provider::new(ws))
        } else {
            let http = Http::new(rpc_url);
            EthereumProvider::Http(Provider::new(http))
        };

        Ok(Self {
            chain_id,
            provider,
        })
    }
}

#[async_trait]
impl EventService for EthereumEventService {
    type EventType = EthereumEvent;

    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    async fn get_events(&self, filter: EventFilter) -> Result<Vec<Box<dyn Event>>> {
        // Implementation of get_events for Ethereum
        // This is a placeholder that will need to be implemented based on specific requirements
        
        let block_range = filter.block_range.unwrap_or((0, 0));
        let events: Vec<Box<dyn Event>> = Vec::new();
        
        // Actual implementation would fetch events from Ethereum based on the filter
        // For example, we might:
        // 1. Get all blocks in the specified range
        // 2. Extract events from each block
        // 3. Filter events based on other criteria in the filter
        // 4. Convert to the common Event interface
        
        Ok(events)
    }

    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> {
        match &self.provider {
            EthereumProvider::Websocket(ws_provider) => {
                let subscription = EthereumSubscription::new(ws_provider.clone(), self.chain_id.clone())
                    .await
                    .map_err(|e| Error::chain(format!("Failed to create subscription: {}", e)))?;
                
                Ok(Box::new(subscription))
            },
            EthereumProvider::Http(_) => {
                Err(Error::chain("Cannot subscribe with HTTP provider, WebSocket required"))
            }
        }
    }

    async fn get_latest_block(&self) -> Result<u64> {
        match &self.provider {
            EthereumProvider::Websocket(provider) => {
                let block_number = provider.get_block_number().await
                    .map_err(|e| Error::chain(format!("Failed to get latest block: {}", e)))?;
                
                Ok(block_number.as_u64())
            },
            EthereumProvider::Http(provider) => {
                let block_number = provider.get_block_number().await
                    .map_err(|e| Error::chain(format!("Failed to get latest block: {}", e)))?;
                
                Ok(block_number.as_u64())
            }
        }
    }
} 