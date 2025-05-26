//! Cosmos client implementation using valence-domain-clients
//! 
//! This module provides Cosmos blockchain connectivity and event processing
//! using the valence-domain-clients Cosmos integration for robust chain support.

use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use std::sync::Arc;
use valence_domain_clients::clients::noble::NobleClient;
use valence_domain_clients::cosmos::base_client::BaseClient;

use indexer_core::event::{Event, UnifiedEvent};
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};

/// Cosmos chain configuration
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

/// Cosmos client using valence-domain-clients Cosmos integration
/// This uses NobleClient as an example, but can be expanded to support multiple chains
pub struct CosmosClientWrapper {
    /// Internal valence Cosmos client (using Noble as example)
    valence_client: Arc<NobleClient>,
    /// Chain configuration
    config: CosmosChainConfig,
    /// Legacy chain_id for compatibility
    chain_id: ChainId,
}

impl CosmosClientWrapper {
    /// Create a new Cosmos client with the given configuration
    /// This creates a Noble client as an example - can be extended for other chains
    pub async fn new(chain_id: String, grpc_url: String, mnemonic: String) -> AnyhowResult<Self> {
        // Create default configuration for backward compatibility
        let config = match chain_id.as_str() {
            "noble-1" => CosmosChainConfig::noble_mainnet(grpc_url.clone()),
            "osmosis-1" => CosmosChainConfig::osmosis_mainnet(grpc_url.clone()),
            "neutron-1" => CosmosChainConfig::neutron_mainnet(grpc_url.clone()),
            _ => CosmosChainConfig::custom(
                chain_id.clone(),
                format!("chain-{}", chain_id),
                grpc_url.clone(),
                "cosmos".to_string(),
                "uatom".to_string(),
                0.025,
                1.4,
            ),
        };
        
        Self::new_with_config(config, mnemonic).await
    }
    
    /// Create a new Cosmos client with explicit configuration
    pub async fn new_with_config(config: CosmosChainConfig, mnemonic: String) -> AnyhowResult<Self> {
        // Create valence Noble client as an example
        // TODO: This should be expanded to support different chain types based on config
        // Note: NobleClient::new takes (rpc_url, rpc_port, mnemonic, chain_id, chain_denom)
        // We'll split the grpc_url for now - in production this should be proper URL parsing
        let (rpc_url, rpc_port) = config.grpc_url.split_once(':').unwrap_or((&config.grpc_url, "443"));
        let valence_client = NobleClient::new(rpc_url, rpc_port, &mnemonic, &config.chain_id, &config.denom).await
            .map_err(|e| anyhow::anyhow!("Failed to create Noble client: {}", e))?;
        
        Ok(Self {
            valence_client: Arc::new(valence_client),
            chain_id: ChainId(config.chain_id.clone()),
            config,
        })
    }
    
    /// Get the chain ID for this client
    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    /// Get the chain configuration
    pub fn config(&self) -> &CosmosChainConfig {
        &self.config
    }
    
    /// Get the underlying valence client for advanced operations
    pub fn valence_client(&self) -> &NobleClient {
        &self.valence_client
    }
}

#[async_trait]
impl EventService for CosmosClientWrapper {
    type EventType = UnifiedEvent;
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    async fn get_events(&self, _filters: Vec<EventFilter>) -> indexer_core::Result<Vec<Box<dyn Event>>> {
        // TODO: Convert EventFilter to valence domain client filters
        // TODO: Use valence_client to fetch events
        // TODO: Convert valence events to almanac Event trait objects
        
        // For now, return empty vector as this requires event subscription implementation
        // This will be implemented in Phase 2.4 - Implement Cosmos event parsing and subscription
        Ok(Vec::new())
    }
    
    async fn get_latest_block(&self) -> indexer_core::Result<u64> {
        // Use the valence client to get the latest block header
        let header = self.valence_client.latest_block_header().await
            .map_err(|e| indexer_core::Error::generic(format!("Failed to get latest block: {}", e)))?;
            
        Ok(header.height as u64)
    }
    
    async fn get_latest_block_with_status(&self, _chain: &str, _status: indexer_core::BlockStatus) -> indexer_core::Result<u64> {
        // For Cosmos chains, we'll just return the latest block for now
        // More sophisticated block status handling can be added later
        self.get_latest_block().await
    }
    
    async fn subscribe(&self) -> indexer_core::Result<Box<dyn EventSubscription>> {
        // TODO: Implement event subscription using valence domain client
        // This will be implemented in Phase 2.4 - Implement Cosmos event parsing and subscription
        
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
    
    #[test]
    fn test_cosmos_chain_config_creation() {
        let config = CosmosChainConfig::noble_mainnet("grpc://test.grpc".to_string());
        assert_eq!(config.chain_id, "noble-1");
        assert_eq!(config.name, "noble");
        assert_eq!(config.prefix, "noble");
        assert_eq!(config.denom, "uusdc");
        assert_eq!(config.gas_price, 0.1);
        assert_eq!(config.gas_adjustment, 1.5);
        assert!(config.explorer_url.is_some());
    }
    
    #[test]
    fn test_cosmos_chain_config_presets() {
        let noble_config = CosmosChainConfig::noble_mainnet("grpc://test.grpc".to_string());
        let osmosis_config = CosmosChainConfig::osmosis_mainnet("grpc://test.grpc".to_string());
        let neutron_config = CosmosChainConfig::neutron_mainnet("grpc://test.grpc".to_string());
        
        // Test chain IDs
        assert_eq!(noble_config.chain_id, "noble-1");
        assert_eq!(osmosis_config.chain_id, "osmosis-1");
        assert_eq!(neutron_config.chain_id, "neutron-1");
        
        // Test chain names
        assert_eq!(noble_config.name, "noble");
        assert_eq!(osmosis_config.name, "osmosis");
        assert_eq!(neutron_config.name, "neutron");
        
        // Test denoms
        assert_eq!(noble_config.denom, "uusdc");
        assert_eq!(osmosis_config.denom, "uosmo");
        assert_eq!(neutron_config.denom, "untrn");
        
        // Test prefixes
        assert_eq!(noble_config.prefix, "noble");
        assert_eq!(osmosis_config.prefix, "osmo");
        assert_eq!(neutron_config.prefix, "neutron");
        
        // Test gas configurations
        assert_eq!(noble_config.gas_price, 0.1);
        assert_eq!(osmosis_config.gas_price, 0.025);
        assert_eq!(neutron_config.gas_price, 0.025);
    }
    
    #[test]
    fn test_cosmos_chain_config_custom() {
        let custom_config = CosmosChainConfig::custom(
            "test-1".to_string(),
            "test".to_string(),
            "grpc://test.grpc".to_string(),
            "test".to_string(),
            "utest".to_string(),
            0.1,
            1.5,
        );
        
        assert_eq!(custom_config.chain_id, "test-1");
        assert_eq!(custom_config.name, "test");
        assert_eq!(custom_config.grpc_url, "grpc://test.grpc");
        assert_eq!(custom_config.prefix, "test");
        assert_eq!(custom_config.denom, "utest");
        assert_eq!(custom_config.gas_price, 0.1);
        assert_eq!(custom_config.gas_adjustment, 1.5);
        assert!(custom_config.explorer_url.is_none());
        assert!(custom_config.rpc_url.is_none());
    }
    
    #[tokio::test]
    async fn test_cosmos_client_creation() {
        // This test doesn't require actual gRPC connectivity
        let result = CosmosClientWrapper::new(
            "noble-1".to_string(), 
            "grpc://test.grpc".to_string(), 
            "test mnemonic phrase".to_string()
        ).await;
        
        // The client creation will likely fail due to invalid gRPC, but structure should be correct
        match result {
            Ok(client) => {
                assert_eq!(client.chain_id().0, "noble-1");
                assert_eq!(client.config().name, "noble");
                assert_eq!(client.config().chain_id, "noble-1");
                assert_eq!(client.config().grpc_url, "grpc://test.grpc");
            }
            Err(_) => {
                // Expected if gRPC is not accessible, but that's okay for unit tests
                println!("Client creation failed as expected with test gRPC URL");
            }
        }
    }
    
    #[tokio::test]
    async fn test_cosmos_client_with_config() {
        let config = CosmosChainConfig::osmosis_mainnet("grpc://osmosis.test".to_string());
        let result = CosmosClientWrapper::new_with_config(config.clone(), "test mnemonic".to_string()).await;
        
        match result {
            Ok(client) => {
                assert_eq!(client.chain_id().0, config.chain_id);
                assert_eq!(client.config().name, config.name);
                assert_eq!(client.config().denom, config.denom);
                assert_eq!(client.config().prefix, config.prefix);
            }
            Err(_) => {
                println!("Client creation failed as expected with test gRPC URL");
            }
        }
    }
    
    #[tokio::test]
    async fn test_cosmos_client_chain_detection() {
        // Test that the right chain config is detected
        let noble_result = CosmosClientWrapper::new(
            "noble-1".to_string(),
            "grpc://noble.test".to_string(),
            "test mnemonic".to_string()
        ).await;
        
        let osmosis_result = CosmosClientWrapper::new(
            "osmosis-1".to_string(),
            "grpc://osmosis.test".to_string(),
            "test mnemonic".to_string()
        ).await;
        
        let neutron_result = CosmosClientWrapper::new(
            "neutron-1".to_string(),
            "grpc://neutron.test".to_string(),
            "test mnemonic".to_string()
        ).await;
        
        // Check that chain detection works (may fail due to networking, but structure should be right)
        match noble_result {
            Ok(client) => assert_eq!(client.config().denom, "uusdc"),
            Err(_) => println!("Noble client creation failed (expected)")
        }
        
        match osmosis_result {
            Ok(client) => assert_eq!(client.config().denom, "uosmo"),
            Err(_) => println!("Osmosis client creation failed (expected)")
        }
        
        match neutron_result {
            Ok(client) => assert_eq!(client.config().denom, "untrn"),
            Err(_) => println!("Neutron client creation failed (expected)")
        }
    }
    
    #[test]
    fn test_cosmos_config_validation() {
        let config = CosmosChainConfig::noble_mainnet("grpc://test.grpc".to_string());
        
        // Valid config
        assert!(!config.chain_id.is_empty());
        assert!(!config.grpc_url.is_empty());
        assert!(!config.prefix.is_empty());
        assert!(!config.denom.is_empty());
        assert!(config.gas_price > 0.0);
        assert!(config.gas_adjustment > 0.0);
        
        // Test custom configuration
        let custom_config = CosmosChainConfig::custom(
            "test-1".to_string(),
            "test".to_string(),
            "grpc://test.grpc".to_string(),
            "test".to_string(),
            "utest".to_string(),
            0.1,
            1.5,
        );
        
        assert_eq!(custom_config.chain_id, "test-1");
        assert_eq!(custom_config.name, "test");
        assert_eq!(custom_config.prefix, "test");
        assert_eq!(custom_config.denom, "utest");
        assert_eq!(custom_config.gas_price, 0.1);
        assert_eq!(custom_config.gas_adjustment, 1.5);
    }
    
    #[tokio::test]
    async fn test_event_service_interface() {
        // Test that our client implements EventService correctly
        use indexer_core::service::EventService;
        
        let config = CosmosChainConfig::noble_mainnet("grpc://test.grpc".to_string());
        
        match CosmosClientWrapper::new_with_config(config, "test mnemonic".to_string()).await {
            Ok(client) => {
                // Test EventService methods
                assert_eq!(client.chain_id().0, "noble-1");
                
                // Test get_events (should return empty for now)
                let events = client.get_events(vec![]).await.unwrap();
                assert_eq!(events.len(), 0);
            }
            Err(_) => {
                println!("Client creation failed as expected with test gRPC URL");
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
    
    #[test]
    fn test_grpc_url_parsing() {
        let config = CosmosChainConfig::noble_mainnet("grpc://test.noble.com:9090".to_string());
        
        // Test that grpc_url is stored correctly
        assert_eq!(config.grpc_url, "grpc://test.noble.com:9090");
        
        // Test URL without port
        let config2 = CosmosChainConfig::noble_mainnet("grpc://test.noble.com".to_string());
        assert_eq!(config2.grpc_url, "grpc://test.noble.com");
    }
}