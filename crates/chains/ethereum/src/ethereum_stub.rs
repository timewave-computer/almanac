//! Stub implementation for Ethereum chain when ethereum feature is not enabled

use std::any::Any;
use std::time::SystemTime;
use almanac_infrastructure_core::Result;
use almanac_infrastructure_core::Error;
use almanac_infrastructure_core::types::{Event, ChainId};
// use almanac_infrastructure_core::service::EventService; // Not needed for stub

/// Stub EVM chain configuration (no valence-domain-clients dependency)
#[derive(Debug, Clone)]
pub struct EvmChainConfig {
    pub chain_id: String,
    pub name: String,
    pub rpc_url: String,
    pub network_id: u64,
    pub native_token: String,
    pub explorer_url: Option<String>,
}

impl EvmChainConfig {
    /// Ethereum mainnet configuration
    pub fn ethereum_mainnet(rpc_url: String) -> Self {
        Self {
            chain_id: "1".to_string(),
            name: "ethereum".to_string(),
            rpc_url,
            network_id: 1,
            native_token: "ETH".to_string(),
            explorer_url: Some("https://etherscan.io".to_string()),
        }
    }
    
    /// Polygon mainnet configuration  
    pub fn polygon_mainnet(rpc_url: String) -> Self {
        Self {
            chain_id: "137".to_string(),
            name: "polygon".to_string(),
            rpc_url,
            network_id: 137,
            native_token: "MATIC".to_string(),
            explorer_url: Some("https://polygonscan.com".to_string()),
        }
    }
    
    /// Base mainnet configuration
    pub fn base_mainnet(rpc_url: String) -> Self {
        Self {
            chain_id: "8453".to_string(),
            name: "base".to_string(),
            rpc_url,
            network_id: 8453,
            native_token: "ETH".to_string(),
            explorer_url: Some("https://basescan.org".to_string()),
        }
    }
    
    /// Custom EVM chain configuration
    pub fn custom(chain_id: String, name: String, rpc_url: String, network_id: u64, native_token: String) -> Self {
        Self {
            chain_id,
            name,
            rpc_url,
            network_id,
            native_token,
            explorer_url: None,
        }
    }
}

/// Stub Ethereum client that returns an error when ethereum feature is disabled
pub struct EthereumStubService {
    chain_id: ChainId,
    config: EvmChainConfig,
}

impl EthereumStubService {
    /// Create a new stub Ethereum client - returns error about missing feature
    pub async fn new(_chain_id: String, rpc_url: String) -> Result<Self> {
        Err(Error::generic(format!(
            "Ethereum functionality disabled: enable 'ethereum' feature to use EthereumClient. URL was: {}", 
            rpc_url
        )))
    }
    
    /// Create a new stub Ethereum client with config - returns error about missing feature
    pub async fn new_with_config(config: EvmChainConfig) -> Result<Self> {
        Err(Error::generic(format!(
            "Ethereum functionality disabled: enable 'ethereum' feature to use EthereumClient with chain {}", 
            config.chain_id
        )))
    }
    
    /// Get the chain ID for this client
    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    /// Get the chain configuration
    pub fn config(&self) -> &EvmChainConfig {
        &self.config
    }
}

/// Stub Ethereum block event
#[derive(Debug, Clone)]
pub struct EthereumBlockEvent {
    chain_id: String,
    block_number: u64,
    block_hash: String,
    timestamp: SystemTime,
}

impl Event for EthereumBlockEvent {
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
        "ethereum_block_stub"
    }
    
    fn raw_data(&self) -> &[u8] {
        &[]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Stub event adapter
#[allow(dead_code)]
pub struct ValenceEventAdapter {
    chain_id: String,
}

impl ValenceEventAdapter {
    #[allow(dead_code)]
    pub fn new(_response: (), chain_id: String) -> Self {
        Self { chain_id }
    }
}

impl Event for ValenceEventAdapter {
    fn id(&self) -> &str {
        "stub"
    }
    
    fn chain(&self) -> &str {
        &self.chain_id
    }
    
    fn block_number(&self) -> u64 {
        0
    }
    
    fn block_hash(&self) -> &str {
        "0x0"
    }
    
    fn tx_hash(&self) -> &str {
        "0x0"
    }
    
    fn timestamp(&self) -> SystemTime {
        SystemTime::now()
    }
    
    fn event_type(&self) -> &str {
        "ethereum_transaction_stub"
    }
    
    fn raw_data(&self) -> &[u8] {
        &[]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for ValenceEventAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValenceEventAdapter")
            .field("chain_id", &self.chain_id)
            .finish()
    }
} 