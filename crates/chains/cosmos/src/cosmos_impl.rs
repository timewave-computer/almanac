//! Cosmos implementation using valence-domain-clients

use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use std::sync::Arc;
use valence_domain_clients::clients::noble::NobleClient;
use valence_domain_clients::cosmos::base_client::BaseClient;

use almanac_infrastructure_core::Result;
use almanac_infrastructure_core::Error;
use almanac_infrastructure_core::BlockStatus;
use almanac_infrastructure_core::types::Event;
use almanac_infrastructure_core::service::{EventService, EventSubscription};

use std::time::SystemTime;
use std::any::Any;

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

/// Cosmos client wrapper that provides Almanac integration
pub struct CosmosClientWrapper {
    /// Internal valence Cosmos client (using Noble as example)
    valence_client: Arc<NobleClient>,
    /// Chain configuration
    config: CosmosChainConfig,
    /// Legacy chain_id for compatibility
    chain_id: almanac_infrastructure_core::types::ChainId,
}

impl std::fmt::Debug for CosmosClientWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CosmosClientWrapper")
            .field("config", &self.config)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

impl CosmosClientWrapper {
    /// Create a new Cosmos client with the given configuration
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
        let (rpc_url, rpc_port) = config.grpc_url.split_once(':').unwrap_or((&config.grpc_url, "443"));
        let valence_client = NobleClient::new(rpc_url, rpc_port, &mnemonic, &config.chain_id, &config.denom).await
            .map_err(|e| anyhow::anyhow!("Failed to create Noble client: {}", e))?;
        
        Ok(Self {
            valence_client: Arc::new(valence_client),
            chain_id: almanac_infrastructure_core::types::ChainId(config.chain_id.clone()),
            config,
        })
    }
    
    /// Get the chain ID for this client
    pub fn chain_id(&self) -> &almanac_infrastructure_core::types::ChainId {
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
    type EventType = almanac_infrastructure_core::types::UnifiedEvent;
    
    fn chain_id(&self) -> &almanac_infrastructure_core::types::ChainId {
        &self.chain_id
    }
    
    async fn get_events(&self, _filters: Vec<almanac_infrastructure_core::types::EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        // TODO: Implement actual Cosmos event querying
        Ok(Vec::new())
    }
    
    async fn get_latest_block(&self) -> Result<u64> {
        let header = self.valence_client.latest_block_header().await
            .map_err(|e| Error::generic(format!("Failed to get latest block: {}", e)))?;
            
        Ok(header.height as u64)
    }
    
    async fn get_latest_block_with_status(&self, _chain: &str, _status: BlockStatus) -> Result<u64> {
        self.get_latest_block().await
    }
    
    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> {
        let subscription = CosmosEventSubscription::new(
            self.valence_client.clone(),
            self.chain_id.clone(),
        ).await?;
        
        Ok(Box::new(subscription))
    }
}

/// Real Cosmos event subscription implementation
pub struct CosmosEventSubscription {
    valence_client: Arc<NobleClient>,
    chain_id: almanac_infrastructure_core::types::ChainId,
    current_height: u64,
    retry_count: u32,
    max_retries: u32,
}

impl std::fmt::Debug for CosmosEventSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CosmosEventSubscription")
            .field("chain_id", &self.chain_id)
            .field("current_height", &self.current_height)
            .field("retry_count", &self.retry_count)
            .field("max_retries", &self.max_retries)
            .finish()
    }
}

impl CosmosEventSubscription {
    pub async fn new(
        valence_client: Arc<NobleClient>,
        chain_id: almanac_infrastructure_core::types::ChainId,
    ) -> Result<Self> {
        let current_height = valence_client.latest_block_header().await
            .map_err(|e| Error::generic(format!("Failed to get latest block: {}", e)))?
            .height as u64;
        
        Ok(Self {
            valence_client,
            chain_id,
            current_height,
            retry_count: 0,
            max_retries: 3,
        })
    }
    
    async fn poll_for_events(&mut self) -> Result<Option<Box<dyn Event>>> {
        let latest_header = self.valence_client.latest_block_header().await
            .map_err(|e| Error::generic(format!("Failed to get latest block: {}", e)))?;
        
        let latest_height = latest_header.height as u64;
        
        if self.current_height >= latest_height {
            return Ok(None);
        }
        
        self.current_height += 1;
        
        let event = CosmosBlockEvent {
            chain_id: self.chain_id.0.clone(),
            block_height: self.current_height,
            block_hash: format!("cosmos-block-{}", self.current_height),
            timestamp: SystemTime::now(),
        };
        
        self.retry_count = 0;
        Ok(Some(Box::new(event)))
    }
}

#[async_trait]
impl EventSubscription for CosmosEventSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        loop {
            match self.poll_for_events().await {
                Ok(Some(event)) => return Some(event),
                Ok(None) => {
                    // No new events, wait a bit
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                Err(e) => {
                    self.retry_count += 1;
                    if self.retry_count >= self.max_retries {
                        eprintln!("Max retries reached for Cosmos event subscription: {}", e);
                        return None;
                    }
                    // Exponential backoff
                    let delay = std::time::Duration::from_millis(1000 * 2_u64.pow(self.retry_count));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Cosmos block event implementation
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
        // Block events don't have transaction hashes
        ""
    }

    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    fn event_type(&self) -> &str {
        "cosmos_block"
    }

    fn raw_data(&self) -> &[u8] {
        // Return empty for now, would contain block data in real implementation
        &[]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
} 