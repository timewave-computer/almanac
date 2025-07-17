//! Stub implementation for Cosmos chain when cosmos feature is not enabled

use std::any::Any;
use std::time::SystemTime;
use almanac_infrastructure_core::Result;
use almanac_infrastructure_core::Error;
use almanac_infrastructure_core::types::{Event, ChainId};
// use almanac_infrastructure_core::service::EventService; // Not needed for stub

/// Stub Cosmos chain configuration (no valence-domain-clients dependency)
#[derive(Debug, Clone)]
pub struct CosmosChainConfig {
    pub chain_id: String,
    pub name: String,
    pub rpc_url: String,
    pub grpc_url: Option<String>,
    pub address_prefix: String,
    pub denom: String,
    pub gas_price: String,
}

impl CosmosChainConfig {
    /// Osmosis mainnet configuration
    pub fn osmosis_mainnet(rpc_url: String) -> Self {
        Self {
            chain_id: "osmosis-1".to_string(),
            name: "osmosis".to_string(),
            rpc_url,
            grpc_url: None,
            address_prefix: "osmo".to_string(),
            denom: "uosmo".to_string(),
            gas_price: "0.0025uosmo".to_string(),
        }
    }
    
    /// Neutron mainnet configuration
    pub fn neutron_mainnet(rpc_url: String) -> Self {
        Self {
            chain_id: "neutron-1".to_string(),
            name: "neutron".to_string(),
            rpc_url,
            grpc_url: None,
            address_prefix: "neutron".to_string(),
            denom: "untrn".to_string(),
            gas_price: "0.0053untrn".to_string(),
        }
    }
}

/// Stub Cosmos client that returns an error when cosmos feature is disabled
pub struct CosmosStubService {
    chain_id: ChainId,
    config: CosmosChainConfig,
}

impl CosmosStubService {
    /// Create a new stub Cosmos client - returns error about missing feature
    pub async fn new(_chain_id: ChainId) -> Result<Self> {
        Err(Error::generic(
            "Cosmos functionality disabled: enable 'cosmos' feature to use CosmosClient"
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

/// Stub Cosmos block event
#[derive(Debug, Clone)]
pub struct CosmosBlockEvent {
    chain_id: String,
    block_number: u64,
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
        self.block_number
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