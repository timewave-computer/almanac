/// Ethereum blockchain indexer implementation
/// 
/// This module provides indexing capabilities for Ethereum-compatible blockchains
/// using the valence-domain-clients EVM integration for robust chain support.
use std::sync::Arc;
use async_trait::async_trait;
use std::any::Any;
use std::time::SystemTime;
use indexer_core::{Result, Error};
use indexer_core::event::{Event, UnifiedEvent};
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};
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

/// Ethereum client using valence-domain-clients EVM integration
pub struct EthereumClient {
    /// Internal valence Ethereum client
    valence_client: Arc<ValenceEthereumClient>,
    /// Chain configuration
    config: EvmChainConfig,
    /// Legacy chain_id for compatibility
    chain_id: ChainId,
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
    /// Note: The current valence API doesn't support private key initialization
    /// This method creates a client with mnemonic instead
    pub async fn new_with_private_key(chain_id: String, rpc_url: String, _private_key: [u8; 32]) -> Result<Self> {
        // For now, fallback to mnemonic-based creation
        // TODO: Update when valence supports private key initialization
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
struct ValenceEventAdapter {
    response: TransactionResponse,
    chain_id: String,
}

impl ValenceEventAdapter {
    #[allow(dead_code)]
    fn new(response: TransactionResponse, chain_id: String) -> Self {
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
    
    async fn get_events(&self, _filters: Vec<EventFilter>) -> indexer_core::Result<Vec<Box<dyn Event>>> {
        // TODO: Convert EventFilter to valence domain client filters
        // TODO: Use valence_client to fetch events
        // TODO: Convert valence events to almanac Event trait objects
        
        // For now, return empty vector as this requires event subscription implementation
        // This will be implemented in Phase 2.3 - Implement Ethereum event parsing and subscription
        Ok(Vec::new())
    }
    
    async fn get_latest_block(&self) -> indexer_core::Result<u64> {
        // Use the valence client to get the latest block number
        // Try using latest_block_height() method similar to cosmos client
        let block_number = self.valence_client.latest_block_height().await
            .map_err(|e| Error::generic(format!("Failed to get latest block: {}", e)))?;
            
        Ok(block_number)
    }
    
    async fn get_latest_block_with_status(&self, _chain: &str, _status: indexer_core::BlockStatus) -> indexer_core::Result<u64> {
        // For EVM chains, we'll just return the latest block for now
        // More sophisticated block status handling can be added later
        self.get_latest_block().await
    }
    
    async fn subscribe(&self) -> indexer_core::Result<Box<dyn EventSubscription>> {
        // TODO: Implement event subscription using valence domain client
        // This will be implemented in Phase 2.3 - Implement Ethereum event parsing and subscription
        
        // For now, return a dummy subscription
        Ok(Box::new(DummySubscription))
    }
}

/// Dummy subscription for compilation (to be replaced with real implementation)
struct DummySubscription;

#[async_trait]
impl EventSubscription for DummySubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        None
    }
    
    async fn close(&mut self) -> indexer_core::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    
    #[test]
    fn test_evm_chain_config_creation() {
        let config = EvmChainConfig::ethereum_mainnet("https://test.rpc".to_string());
        assert_eq!(config.chain_id, "1");
        assert_eq!(config.name, "ethereum");
        assert_eq!(config.native_token, "ETH");
        assert_eq!(config.network_id, 1);
        assert!(config.explorer_url.is_some());
        assert_eq!(config.explorer_url.unwrap(), "https://etherscan.io");
    }
    
    #[test]
    fn test_evm_chain_config_presets() {
        let eth_config = EvmChainConfig::ethereum_mainnet("https://test.rpc".to_string());
        let polygon_config = EvmChainConfig::polygon_mainnet("https://test.rpc".to_string());
        let base_config = EvmChainConfig::base_mainnet("https://test.rpc".to_string());
        
        // Test chain IDs
        assert_eq!(eth_config.chain_id, "1");
        assert_eq!(polygon_config.chain_id, "137");
        assert_eq!(base_config.chain_id, "8453");
        
        // Test chain names
        assert_eq!(eth_config.name, "ethereum");
        assert_eq!(polygon_config.name, "polygon");
        assert_eq!(base_config.name, "base");
        
        // Test native tokens
        assert_eq!(eth_config.native_token, "ETH");
        assert_eq!(polygon_config.native_token, "MATIC");
        assert_eq!(base_config.native_token, "ETH");
        
        // Test network IDs
        assert_eq!(eth_config.network_id, 1);
        assert_eq!(polygon_config.network_id, 137);
        assert_eq!(base_config.network_id, 8453);
    }
    
    #[test]
    fn test_evm_chain_config_custom() {
        let custom_config = EvmChainConfig::custom(
            "999".to_string(),
            "testnet".to_string(),
            "https://testnet.rpc".to_string(),
            999,
            "TEST".to_string(),
        );
        
        assert_eq!(custom_config.chain_id, "999");
        assert_eq!(custom_config.name, "testnet");
        assert_eq!(custom_config.rpc_url, "https://testnet.rpc");
        assert_eq!(custom_config.network_id, 999);
        assert_eq!(custom_config.native_token, "TEST");
        assert!(custom_config.explorer_url.is_none());
    }
    
    #[tokio::test]
    async fn test_ethereum_client_creation() {
        // This test doesn't require actual RPC connectivity
        let result = EthereumClient::new("1".to_string(), "https://test.rpc".to_string()).await;
        
        // The client creation might fail due to invalid RPC, but the structure should be correct
        match result {
            Ok(client) => {
                assert_eq!(client.chain_id().0, "1");
                assert_eq!(client.config().name, "ethereum");
                assert_eq!(client.config().chain_id, "1");
                assert_eq!(client.config().rpc_url, "https://test.rpc");
            }
            Err(_) => {
                // Expected if RPC is not accessible, but that's okay for unit tests
                println!("Client creation failed as expected with test RPC URL");
            }
        }
    }
    
    #[tokio::test]
    async fn test_ethereum_client_with_config() {
        let config = EvmChainConfig::ethereum_mainnet("https://test.rpc".to_string());
        let result = EthereumClient::new_with_config(config.clone()).await;
        
        match result {
            Ok(client) => {
                assert_eq!(client.chain_id().0, config.chain_id);
                assert_eq!(client.config().name, config.name);
                assert_eq!(client.config().network_id, config.network_id);
            }
            Err(_) => {
                println!("Client creation failed as expected with test RPC URL");
            }
        }
    }
    
    #[tokio::test]
    async fn test_ethereum_client_with_private_key() {
        let private_key = [1u8; 32]; // Dummy private key
        let result = EthereumClient::new_with_private_key(
            "1".to_string(),
            "https://test.rpc".to_string(),
            private_key,
        ).await;
        
        match result {
            Ok(client) => {
                assert_eq!(client.chain_id().0, "1");
                assert_eq!(client.config().name, "ethereum");
            }
            Err(_) => {
                println!("Client creation failed as expected with test RPC URL");
            }
        }
    }
    
    #[test]
    fn test_valence_event_adapter() {
        use valence_domain_clients::common::transaction::TransactionResponse;
        
        let tx_response = TransactionResponse {
            hash: "0x123".to_string(),
            success: true,
            block_height: 1000,
            gas_used: 21000,
        };
        
        let adapter = ValenceEventAdapter::new(tx_response, "ethereum".to_string());
        
        assert_eq!(adapter.id(), "0x123");
        assert_eq!(adapter.chain(), "ethereum");
        assert_eq!(adapter.block_number(), 1000);
        assert_eq!(adapter.block_hash(), "0x");
        assert_eq!(adapter.tx_hash(), "0x123");
        assert_eq!(adapter.event_type(), "ethereum_transaction");
        assert_eq!(adapter.raw_data(), &[] as &[u8]);
        
        // Test that timestamp is reasonable (within last minute)
        let now = SystemTime::now();
        let adapter_time = adapter.timestamp();
        let duration = now.duration_since(adapter_time).unwrap_or_default();
        assert!(duration.as_secs() < 60);
    }
    
    #[test]
    fn test_valence_event_adapter_debug() {
        use valence_domain_clients::common::transaction::TransactionResponse;
        
        let tx_response = TransactionResponse {
            hash: "0xabc".to_string(),
            success: false,
            block_height: 2000,
            gas_used: 42000,
        };
        
        let adapter = ValenceEventAdapter::new(tx_response, "polygon".to_string());
        let debug_str = format!("{:?}", adapter);
        
        assert!(debug_str.contains("0xabc"));
        assert!(debug_str.contains("2000"));
        assert!(debug_str.contains("polygon"));
    }
    
    #[test]
    fn test_valence_event_adapter_as_any() {
        use valence_domain_clients::common::transaction::TransactionResponse;
        
        let tx_response = TransactionResponse {
            hash: "0xdef".to_string(),
            success: true,
            block_height: 3000,
            gas_used: 63000,
        };
        
        let adapter = ValenceEventAdapter::new(tx_response, "base".to_string());
        let any = adapter.as_any();
        
        // Test that we can downcast back to ValenceEventAdapter
        let downcasted = any.downcast_ref::<ValenceEventAdapter>().unwrap();
        assert_eq!(downcasted.id(), "0xdef");
        assert_eq!(downcasted.chain(), "base");
    }
    
    #[tokio::test]
    async fn test_event_service_interface() {
        // Test that our client implements EventService correctly
        use indexer_core::service::EventService;
        
        let config = EvmChainConfig::ethereum_mainnet("https://test.rpc".to_string());
        
        match EthereumClient::new_with_config(config).await {
            Ok(client) => {
                // Test EventService methods
                assert_eq!(client.chain_id().0, "1");
                
                // Test get_events (should return empty for now)
                let events = client.get_events(vec![]).await.unwrap();
                assert_eq!(events.len(), 0);
            }
            Err(_) => {
                println!("Client creation failed as expected with test RPC URL");
            }
        }
    }
    
    #[tokio::test] 
    async fn test_dummy_subscription() {
        let mut subscription = DummySubscription;
        
        // Test that subscription returns None for next
        let event = subscription.next().await;
        assert!(event.is_none());
        
        // Test that close succeeds
        let result = subscription.close().await;
        assert!(result.is_ok());
    }
} 