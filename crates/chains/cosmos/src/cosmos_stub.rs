//! Stub cosmos implementation when cosmos feature is disabled

use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use indexer_core::types::{Event, UnifiedEvent};
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};
use std::time::SystemTime;
use std::any::Any;

/// Stub cosmos chain configuration (no valence-domain-clients dependency)
#[derive(Debug, Clone)]
pub struct CosmosChainConfig {
    pub chain_id: String,
    pub name: String,
    pub grpc_url: String,
    pub rpc_url: Option<String>,
    pub prefix: String,
    pub denom: String,
    pub gas_price: f64,
    pub gas_adjustment: f64,
    pub explorer_url: Option<String>,
}

impl CosmosChainConfig {
    /// Noble mainnet configuration
    pub fn noble_mainnet(grpc_url: String) -> Self {
        Self {
            chain_id: "noble-1".to_string(),
            name: "noble".to_string(),
            grpc_url,
            rpc_url: None,
            prefix: "noble".to_string(),
            denom: "uusdc".to_string(),
            gas_price: 0.1,
            gas_adjustment: 1.5,
            explorer_url: Some("https://explorer.noble.xyz".to_string()),
        }
    }
    
    /// Osmosis mainnet configuration
    pub fn osmosis_mainnet(grpc_url: String) -> Self {
        Self {
            chain_id: "osmosis-1".to_string(),
            name: "osmosis".to_string(),
            grpc_url,
            rpc_url: None,
            prefix: "osmo".to_string(),
            denom: "uosmo".to_string(),
            gas_price: 0.025,
            gas_adjustment: 1.4,
            explorer_url: Some("https://www.mintscan.io/osmosis".to_string()),
        }
    }
    
    /// Neutron mainnet configuration
    pub fn neutron_mainnet(grpc_url: String) -> Self {
        Self {
            chain_id: "neutron-1".to_string(),
            name: "neutron".to_string(),
            grpc_url,
            rpc_url: None,
            prefix: "neutron".to_string(),
            denom: "untrn".to_string(),
            gas_price: 0.025,
            gas_adjustment: 1.4,
            explorer_url: Some("https://www.mintscan.io/neutron".to_string()),
        }
    }
    
    /// Custom Cosmos chain configuration
    pub fn custom(
        chain_id: String,
        name: String,
        grpc_url: String,
        prefix: String,
        denom: String,
        gas_price: f64,
        gas_adjustment: f64,
    ) -> Self {
        Self {
            chain_id,
            name,
            grpc_url,
            rpc_url: None,
            prefix,
            denom,
            gas_price,
            gas_adjustment,
            explorer_url: None,
        }
    }
}

/// Stub Cosmos client that returns an error when cosmos feature is disabled
pub struct CosmosClientWrapper {
    chain_id: ChainId,
    config: CosmosChainConfig,
}

impl CosmosClientWrapper {
    /// Create a new stub Cosmos client - returns error about missing feature
    pub async fn new(_chain_id: String, grpc_url: String, _mnemonic: String) -> AnyhowResult<Self> {
        Err(anyhow::anyhow!(
            "Cosmos functionality disabled: enable 'cosmos' feature to use CosmosClientWrapper. URL was: {}", 
            grpc_url
        ))
    }
    
    /// Create a new stub Cosmos client with config - returns error about missing feature
    pub async fn new_with_config(config: CosmosChainConfig, _mnemonic: String) -> AnyhowResult<Self> {
        Err(anyhow::anyhow!(
            "Cosmos functionality disabled: enable 'cosmos' feature to use CosmosClientWrapper with chain {}", 
            config.chain_id
        ))
    }
    
    /// Get the chain ID for this client
    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    /// Get the chain configuration
    pub fn config(&self) -> &CosmosChainConfig {
        &self.config
    }
}

#[async_trait]
impl EventService for CosmosClientWrapper {
    type EventType = UnifiedEvent;
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    async fn get_events(&self, _filters: Vec<EventFilter>) -> indexer_core::Result<Vec<Box<dyn Event>>> {
        Err(indexer_core::Error::generic("Cosmos functionality disabled: enable 'cosmos' feature"))
    }
    
    async fn get_latest_block(&self) -> indexer_core::Result<u64> {
        Err(indexer_core::Error::generic("Cosmos functionality disabled: enable 'cosmos' feature"))
    }
    
    async fn get_latest_block_with_status(&self, _chain: &str, _status: indexer_core::BlockStatus) -> indexer_core::Result<u64> {
        Err(indexer_core::Error::generic("Cosmos functionality disabled: enable 'cosmos' feature"))
    }
    
    async fn subscribe(&self) -> indexer_core::Result<Box<dyn EventSubscription>> {
        Err(indexer_core::Error::generic("Cosmos functionality disabled: enable 'cosmos' feature"))
    }
}

/// Stub Cosmos event subscription
pub struct CosmosEventSubscription {
    chain_id: ChainId,
}

impl CosmosEventSubscription {
    pub async fn new(_chain_id: ChainId) -> indexer_core::Result<Self> {
        Err(indexer_core::Error::generic("Cosmos functionality disabled: enable 'cosmos' feature"))
    }
}

#[async_trait]
impl EventSubscription for CosmosEventSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        None // No events available when feature is disabled
    }
    
    async fn close(&mut self) -> indexer_core::Result<()> {
        Ok(())
    }
}

/// Stub Cosmos block event
#[derive(Debug, Clone)]
pub struct CosmosBlockEvent {
    chain_id: String,
    block_height: u64,
    block_hash: String,
    timestamp: SystemTime,
}

impl Event for CosmosBlockEvent {
    fn id(&self) -> &str {
        &self.block_hash
    }
    
    fn chain(&self) -> &str {
        &self.chain_id
    }
    
    fn block_number(&self) -> u64 {
        self.block_height
    }
    
    fn block_hash(&self) -> &str {
        &self.block_hash
    }
    
    fn tx_hash(&self) -> &str {
        &self.block_hash
    }
    
    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    
    fn event_type(&self) -> &str {
        "cosmos_block_stub"
    }
    
    fn raw_data(&self) -> &[u8] {
        &[]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
} 