/// Ethereum event service implementation
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use ethers::middleware::Middleware;
use ethers::providers::{Http, Provider, Ws};
use ethers::types::{BlockNumber, Filter, H256};
use indexer_common::{BlockStatus, Error, Result};
use indexer_core::event::Event;
use indexer_core::service::{EventService, EventSubscription, BoxedEventService};
use indexer_core::types::{ChainId, EventFilter};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

mod provider;
mod event;
mod reorg;
mod subscription;

pub use provider::{EthereumProvider, EthereumProviderConfig, BlockStatus as EthBlockStatus};
pub use event::{EthereumEvent, EthereumEventProcessor};
pub use reorg::EthereumReorgDetector;
pub use subscription::EthereumSubscription;

/// Configuration for the Ethereum event service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumEventServiceConfig {
    /// Chain ID
    pub chain_id: String,
    
    /// RPC URL for the Ethereum node
    pub rpc_url: String,
    
    /// Whether to use websocket connection
    pub use_websocket: bool,
    
    /// Block confirmation threshold
    pub confirmation_blocks: u64,
    
    /// Maximum batch size for fetching blocks
    pub max_batch_size: usize,
    
    /// How often to poll for new blocks (in milliseconds)
    pub poll_interval_ms: u64,
    
    /// Maximum number of parallel requests
    pub max_parallel_requests: usize,
}

impl Default for EthereumEventServiceConfig {
    fn default() -> Self {
        Self {
            chain_id: "ethereum".to_string(),
            rpc_url: "http://localhost:8545".to_string(),
            use_websocket: false,
            confirmation_blocks: 12,
            max_batch_size: 100,
            poll_interval_ms: 1000,
            max_parallel_requests: 10,
        }
    }
}

/// Ethereum event service
pub struct EthereumEventService {
    /// Chain ID
    chain_id: ChainId,
    
    /// Ethereum provider
    provider: Arc<EthereumProvider>,
    
    /// Event processor
    event_processor: Arc<RwLock<EthereumEventProcessor>>,
    
    /// Configuration
    config: EthereumEventServiceConfig,
    
    /// Block cache
    block_cache: Arc<RwLock<HashMap<u64, ethers::types::Block<ethers::types::Transaction>>>>,
}

impl EthereumEventService {
    /// Create a new Ethereum event service
    pub async fn new(config: EthereumEventServiceConfig) -> Result<Self> {
        // Create provider config
        let provider_config = EthereumProviderConfig {
            rpc_url: config.rpc_url.clone(),
            use_websocket: config.use_websocket,
            max_concurrent_requests: config.max_parallel_requests,
            ..Default::default()
        };
        
        // Create provider
        let provider = EthereumProvider::new(provider_config).await?;
        
        // Create event processor
        let event_processor = EthereumEventProcessor::new(config.chain_id.clone());
        
        // Create service
        let service = Self {
            chain_id: ChainId(config.chain_id.clone()),
            provider: Arc::new(provider),
            event_processor: Arc::new(RwLock::new(event_processor)),
            config,
            block_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Check connection
        service.check_connection().await?;
        
        Ok(service)
    }
    
    /// Check connection to the Ethereum node
    async fn check_connection(&self) -> Result<()> {
        self.provider.get_latest_block_number().await?;
        Ok(())
    }
    
    /// Get chain ID as string
    pub fn chain_id_str(&self) -> &str {
        &self.chain_id.0
    }
    
    /// Get the latest block number
    pub async fn get_latest_block_internal(&self) -> Result<u64> {
        self.provider.get_latest_block_number().await
    }
    
    /// Register a contract ABI for event parsing
    pub async fn register_contract(&self, address: String, name: String, abi: ethers::abi::Abi) -> Result<()> {
        let mut processor = self.event_processor.write().await;
        processor.register_contract(address, name, abi);
        Ok(())
    }
    
    /// Fetch blocks in a range
    pub async fn fetch_blocks(&self, from_block: u64, to_block: u64) -> Result<Vec<ethers::types::Block<ethers::types::Transaction>>> {
        // Check if any blocks are in the cache
        let mut blocks_to_fetch = Vec::new();
        let mut result_blocks = Vec::new();
        
        {
            let cache = self.block_cache.read().await;
            
            for block_num in from_block..=to_block {
                if let Some(block) = cache.get(&block_num) {
                    result_blocks.push(block.clone());
                } else {
                    blocks_to_fetch.push(block_num);
                }
            }
        }
        
        // If all blocks were in the cache, return early
        if blocks_to_fetch.is_empty() {
            return Ok(result_blocks);
        }
        
        // Fetch missing blocks
        let fetched_blocks = self.provider.get_blocks_in_range(
            *blocks_to_fetch.first().unwrap(),
            *blocks_to_fetch.last().unwrap()
        ).await?;
        
        // Update cache
        {
            let mut cache = self.block_cache.write().await;
            
            for block in &fetched_blocks {
                if let Some(number) = block.number {
                    cache.insert(number.as_u64(), block.clone());
                }
            }
        }
        
        // Combine cached and fetched blocks
        result_blocks.extend(fetched_blocks);
        
        // Sort blocks by number
        result_blocks.sort_by_key(|block| block.number.unwrap_or_default());
        
        Ok(result_blocks)
    }
    
    /// Fetch logs for a block range with filters
    pub async fn fetch_logs(&self, from_block: u64, to_block: u64, filters: &[EventFilter]) -> Result<Vec<Box<dyn Event>>> {
        // Prepare filter
        let mut addresses = Vec::new();
        
        // Collect addresses from filters
        for filter in filters {
            addresses.extend(filter.addresses.iter().map(|addr| {
                ethers::types::Address::from_str(addr)
                    .map_err(|e| Error::validation(format!("Invalid address {}: {}", addr, e)))
            }).collect::<Result<Vec<_>>>()?);
        }
        
        // Create a filter for the block range
        let filter = Filter::new()
            .from_block(from_block)
            .to_block(to_block)
            .address(addresses);
        
        // Fetch logs
        let logs = match &*self.provider {
            EthereumProvider::Http(provider) => {
                provider.get_logs(&filter).await
                    .map_err(|e| Error::chain(format!("Failed to get logs: {}", e)))?
            }
            EthereumProvider::Websocket(provider) => {
                provider.get_logs(&filter).await
                    .map_err(|e| Error::chain(format!("Failed to get logs: {}", e)))?
            }
        };
        
        // Fetch corresponding blocks
        let blocks = self.fetch_blocks(from_block, to_block).await?;
        
        // Create a map of block numbers to blocks for quick lookup
        let block_map: HashMap<u64, &ethers::types::Block<ethers::types::Transaction>> = blocks
            .iter()
            .filter_map(|block| {
                block.number.map(|num| (num.as_u64(), block))
            })
            .collect();
        
        // Process logs into events
        let processor = self.event_processor.read().await;
        let mut events = Vec::new();
        
        for log in logs {
            let block_number = log.block_number.unwrap_or_default().as_u64();
            
            if let Some(block) = block_map.get(&block_number) {
                let tx_hash = log.transaction_hash.unwrap_or_default();
                
                // Get transaction receipt (typically not needed for most use cases)
                let receipt = None;
                
                // Process the log
                match processor.process_log(log, block, receipt) {
                    Ok(event) => {
                        // Check if the event matches any of the filters
                        let matches = filters.is_empty() || filters.iter().any(|f| processor.matches_filter(&event, f));
                        
                        if matches {
                            events.push(Box::new(event) as Box<dyn Event>);
                        }
                    }
                    Err(e) => {
                        error!("Error processing log: {}", e);
                        continue;
                    }
                }
            } else {
                warn!("Block {} not found for log", block_number);
                continue;
            }
        }
        
        Ok(events)
    }
}

#[async_trait]
impl EventService for EthereumEventService {
    type EventType = EthereumEvent;
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        // Get the block range from filters
        let latest_block = self.get_latest_block().await?;
        let from_block = filters.iter()
            .filter_map(|f| f.from_block)
            .min()
            .unwrap_or(latest_block.saturating_sub(100)); // Default to last 100 blocks
        
        let to_block = filters.iter()
            .filter_map(|f| f.to_block)
            .max()
            .unwrap_or(latest_block);
        
        // Fetch logs for the block range
        self.fetch_logs(from_block, to_block, &filters).await
    }
    
    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> {
        // Create a WebSocket provider if we're using HTTP
        match &*self.provider {
            EthereumProvider::Http(_) => {
                if !self.config.use_websocket {
                    return Err(Error::validation(
                        "WebSocket connection required for subscription. Set use_websocket: true in the config."
                    ));
                }
                
                // Create a new provider config with WebSocket
                let provider_config = EthereumProviderConfig {
                    rpc_url: self.config.rpc_url.clone(),
                    use_websocket: true,
                    ..Default::default()
                };
                
                // Create a WebSocket provider
                let ws_provider = EthereumProvider::new(provider_config).await?;
                
                // Extract the WebSocket provider
                match ws_provider {
                    EthereumProvider::Websocket(provider) => {
                        // Create a subscription
                        let subscription = EthereumSubscription::new(
                            Arc::try_unwrap(provider).unwrap_or_else(|arc| (*arc).clone()),
                            self.chain_id.clone()
                        ).await?;
                        
                        Ok(Box::new(subscription))
                    }
                    _ => Err(Error::internal("Failed to create WebSocket provider")),
                }
            }
            EthereumProvider::Websocket(provider) => {
                // Create a subscription with the existing WebSocket provider
                let subscription = EthereumSubscription::new(
                    provider.clone(),
                    self.chain_id.clone()
                ).await?;
                
                Ok(Box::new(subscription))
            }
        }
    }
    
    async fn get_latest_block(&self) -> Result<u64> {
        self.get_latest_block_internal().await
    }
    
    async fn get_latest_block_with_status(&self, _chain: &str, status: BlockStatus) -> Result<u64> {
        // Convert from storage BlockStatus to Ethereum BlockStatus
        let eth_status = match status {
            BlockStatus::Confirmed => EthBlockStatus::Confirmed,
            BlockStatus::Safe => EthBlockStatus::Safe,
            BlockStatus::Finalized => EthBlockStatus::Finalized,
            _ => EthBlockStatus::Confirmed, // Default to confirmed
        };
        
        // Get block with the given status
        let (_, block_number) = self.provider.get_block_by_status(eth_status).await?;
        
        Ok(block_number)
    }
} 