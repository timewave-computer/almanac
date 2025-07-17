//! Ethereum implementation using valence-domain-clients

use std::sync::Arc;
use async_trait::async_trait;
use std::any::Any;
use std::time::SystemTime;
use almanac_infrastructure_core::{Result, Error};
use almanac_infrastructure_core::types::{Event, UnifiedEvent};
use almanac_infrastructure_core::service::{EventService, EventSubscription};
use almanac_infrastructure_core::types::{ChainId, EventFilter};
use valence_domain_clients::evm::base_client::EvmBaseClient;
use valence_domain_clients::clients::ethereum::EthereumClient as ValenceEthereumClient;
use valence_domain_clients::common::transaction::TransactionResponse;

/// EVM chain configuration
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

/// Ethereum client implementation
pub struct EthereumClient {
    /// Internal valence Ethereum client
    valence_client: Arc<ValenceEthereumClient>,
    /// Chain configuration
    config: EvmChainConfig,
    /// Legacy chain_id for compatibility
    chain_id: ChainId,
}

impl std::fmt::Debug for EthereumClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EthereumClient")
            .field("config", &self.config)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

impl EthereumClient {
    /// Create a new Ethereum client with the given configuration
    pub async fn new(chain_id: String, rpc_url: String) -> Result<Self> {
        // Create default configuration for backward compatibility
        let config = match chain_id.as_str() {
            "1" => EvmChainConfig::ethereum_mainnet(rpc_url.clone()),
            "137" => EvmChainConfig::polygon_mainnet(rpc_url.clone()),
            "8453" => EvmChainConfig::base_mainnet(rpc_url.clone()),
            _ => EvmChainConfig::custom(chain_id.clone(), format!("chain-{}", chain_id), rpc_url.clone(), 
                                      chain_id.parse().unwrap_or(1), "ETH".to_string()),
        };
        
        Self::new_with_config(config).await
    }
    
    /// Create a new Ethereum client with explicit configuration
    pub async fn new_with_config(config: EvmChainConfig) -> Result<Self> {
        // Create valence Ethereum client with a dummy mnemonic for now
        // In production, this should use proper key management
        let dummy_mnemonic = "test test test test test test test test test test test junk";
        let valence_client = ValenceEthereumClient::new(&config.rpc_url, dummy_mnemonic, None)
            .map_err(|e| Error::generic(format!("Failed to create Ethereum client: {}", e)))?;
        
        Ok(Self {
            valence_client: Arc::new(valence_client),
            chain_id: ChainId(config.chain_id.clone()),
            config,
        })
    }
    
    /// Create a new Ethereum client with private key for signing
    pub async fn new_with_private_key(chain_id: String, rpc_url: String, _private_key: [u8; 32]) -> Result<Self> {
        // For now, fallback to mnemonic-based creation
        let dummy_mnemonic = "test test test test test test test test test test test junk";
        let valence_client = ValenceEthereumClient::new(&rpc_url, dummy_mnemonic, None)
            .map_err(|e| Error::generic(format!("Failed to create Ethereum client: {}", e)))?;
            
        let chain_id_u64 = chain_id.parse::<u64>().unwrap_or(1);
        
        // Create default configuration
        let config = match chain_id.as_str() {
            "1" => EvmChainConfig::ethereum_mainnet(rpc_url),
            "137" => EvmChainConfig::polygon_mainnet(rpc_url),
            "8453" => EvmChainConfig::base_mainnet(rpc_url),
            _ => EvmChainConfig::custom(chain_id.clone(), format!("chain-{}", chain_id), rpc_url, 
                                      chain_id_u64, "ETH".to_string()),
        };
        
        Ok(Self {
            valence_client: Arc::new(valence_client),
            chain_id: ChainId(chain_id),
            config,
        })
    }
    
    /// Get the chain ID for this client
    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    /// Get the chain configuration
    pub fn config(&self) -> &EvmChainConfig {
        &self.config
    }
    
    /// Get the underlying valence client for advanced operations
    pub fn valence_client(&self) -> &ValenceEthereumClient {
        &self.valence_client
    }
}

/// Event adapter to convert valence TransactionResponse to almanac Event
#[allow(dead_code)]
pub struct ValenceEventAdapter {
    response: TransactionResponse,
    chain_id: String,
}

impl ValenceEventAdapter {
    #[allow(dead_code)]
    pub fn new(response: TransactionResponse, chain_id: String) -> Self {
        Self { response, chain_id }
    }
}

impl Event for ValenceEventAdapter {
    fn id(&self) -> &str {
        &self.response.hash
    }
    
    fn chain(&self) -> &str {
        &self.chain_id
    }
    
    fn block_number(&self) -> u64 {
        self.response.block_height
    }
    
    fn block_hash(&self) -> &str {
        // For cosmos TransactionResponse, we don't have block_hash, use placeholder
        "0x"
    }
    
    fn tx_hash(&self) -> &str {
        &self.response.hash
    }
    
    fn timestamp(&self) -> SystemTime {
        // For now, return current time as cosmos TransactionResponse doesn't have timestamp
        SystemTime::now()
    }
    
    fn event_type(&self) -> &str {
        "ethereum_transaction"
    }
    
    fn raw_data(&self) -> &[u8] {
        // For now, return empty slice - this could be enhanced to include transaction data
        &[]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for ValenceEventAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValenceEventAdapter")
            .field("hash", &self.response.hash)
            .field("block_height", &self.response.block_height)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

#[async_trait]
impl EventService for EthereumClient {
    type EventType = UnifiedEvent;
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    async fn get_events(&self, _filters: Vec<EventFilter>) -> almanac_infrastructure_core::Result<Vec<Box<dyn Event>>> {
        // TODO: Convert EventFilter to valence domain client filters
        Ok(Vec::new())
    }
    
    async fn get_latest_block(&self) -> almanac_infrastructure_core::Result<u64> {
        let block_number = self.valence_client.latest_block_height().await
            .map_err(|e| Error::generic(format!("Failed to get latest block: {}", e)))?;
            
        Ok(block_number)
    }
    
    async fn get_latest_block_with_status(&self, _chain: &str, _status: almanac_infrastructure_core::BlockStatus) -> almanac_infrastructure_core::Result<u64> {
        self.get_latest_block().await
    }
    
    async fn subscribe(&self) -> almanac_infrastructure_core::Result<Box<dyn EventSubscription>> {
        let subscription = EthereumEventSubscription::new(
            self.valence_client.clone(),
            self.chain_id.clone(),
        ).await?;
        
        Ok(Box::new(subscription))
    }
}

/// Ethereum event subscription implementation
pub struct EthereumEventSubscription {
    valence_client: Arc<ValenceEthereumClient>,
    chain_id: ChainId,
    current_block: u64,
    retry_count: u32,
    max_retries: u32,
}

impl std::fmt::Debug for EthereumEventSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EthereumEventSubscription")
            .field("chain_id", &self.chain_id)
            .field("current_block", &self.current_block)
            .field("retry_count", &self.retry_count)
            .field("max_retries", &self.max_retries)
            .finish()
    }
}

impl EthereumEventSubscription {
    pub async fn new(
        valence_client: Arc<ValenceEthereumClient>,
        chain_id: ChainId,
    ) -> almanac_infrastructure_core::Result<Self> {
        let current_block = valence_client.latest_block_height().await
            .map_err(|e| Error::generic(format!("Failed to get latest block: {}", e)))?;
        
        Ok(Self {
            valence_client,
            chain_id,
            current_block,
            retry_count: 0,
            max_retries: 3,
        })
    }
    
    async fn poll_for_events(&mut self) -> almanac_infrastructure_core::Result<Option<Box<dyn Event>>> {
        let latest_block = self.valence_client.latest_block_height().await
            .map_err(|e| Error::generic(format!("Failed to get latest block: {}", e)))?;
        
        if self.current_block >= latest_block {
            return Ok(None);
        }
        
        self.current_block += 1;
        
        let event = EthereumBlockEvent {
            chain_id: self.chain_id.0.clone(),
            block_number: self.current_block,
            block_hash: format!("0x{:064x}", self.current_block),
            timestamp: SystemTime::now(),
        };
        
        self.retry_count = 0;
        Ok(Some(Box::new(event)))
    }
}

#[async_trait]
impl EventSubscription for EthereumEventSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        loop {
            match self.poll_for_events().await {
                Ok(Some(event)) => return Some(event),
                Ok(None) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    continue;
                },
                Err(e) => {
                    self.retry_count += 1;
                    if self.retry_count >= self.max_retries {
                        eprintln!("Failed to poll for events after {} retries: {}", self.max_retries, e);
                        return None;
                    }
                    
                    let delay = 1000 * (1 << self.retry_count);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    continue;
                }
            }
        }
    }
    
    async fn close(&mut self) -> almanac_infrastructure_core::Result<()> {
        Ok(())
    }
}

/// Ethereum block event implementation
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
        "ethereum_block"
    }
    
    fn raw_data(&self) -> &[u8] {
        &[]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
} 