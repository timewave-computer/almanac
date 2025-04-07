/// Ethereum event service implementation
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use std::str::FromStr;
use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use ethers::middleware::Middleware;
use ethers::providers::{Http, Provider, Ws};
use ethers::types::{BlockNumber, Filter, H256, Log};
use indexer_pipeline::{BlockStatus, Error, Result};
use indexer_core::event::Event;
use indexer_storage::Storage;
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use ethers::abi::{Address, RawLog};

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
    
    /// Optional storage backend
    storage: Option<Arc<dyn Storage + Send + Sync>>,
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
            storage: None,
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
    
    /// Helper to fetch logs based on combined filters
    async fn fetch_logs(
        &self,
        from_block: u64,
        to_block: u64,
        filters: &[EventFilter],
    ) -> Result<Vec<Box<dyn Event>>> {
        debug!(from = from_block, to = to_block, num_filters = filters.len(), "Fetching logs");

        // Determine the overall block range required by the filters
        let min_block_opt = filters
            .iter()
            .filter_map(|f| f.block_range.map(|(min, _)| min))
            .min();
        let max_block_opt = filters
            .iter()
            .filter_map(|f| f.block_range.map(|(_, max)| max))
            .max();

        // Combine with function arguments
        let final_from_block = min_block_opt.map_or(from_block, |min_f| std::cmp::max(from_block, min_f));
        let final_to_block = max_block_opt.map_or(to_block, |max_f| std::cmp::min(to_block, max_f));

        // Check if range is valid
        if final_from_block > final_to_block {
            warn!(final_from=%final_from_block, final_to=%final_to_block, "Invalid block range after applying filters, returning empty.");
            return Ok(Vec::new());
        }

        // Convert u64 block numbers to ethers::types::U64 for the filter
        let from_block_ethers = BlockNumber::Number(final_from_block.into());
        let to_block_ethers = BlockNumber::Number(final_to_block.into());

        // Extract unique addresses from all filters
        let addresses: Vec<Address> = filters
            .iter()
            .filter_map(|f| f.custom_filters.get("address")) // Assuming address is in custom_filters
            .filter_map(|addr_str| Address::from_str(addr_str).ok()) // Parse and ignore errors for now
            .collect::<HashSet<_>>() // Collect into HashSet for dedup
            .into_iter()
            .collect(); // Convert back to Vec

        // Create the base filter
        let mut filter = Filter::new()
            .from_block(from_block_ethers)
            .to_block(to_block_ethers);

        if !addresses.is_empty() {
            filter = filter.address(addresses);
        }

        // TODO: Add topic filtering based on filter.event_types if ABIs are available

        debug!(?filter, "Constructed ethers log filter");

        // Execute the get_logs call
        let logs = match &*self.provider { // Dereference Arc to access inner provider
            EthereumProvider::Websocket(provider) => provider
                .get_logs(&filter)
                .await
                .map_err(|e| Error::generic(format!("Websocket provider error fetching logs: {}", e)))?,
            EthereumProvider::Http(provider) => provider
                .get_logs(&filter)
                .await
                .map_err(|e| Error::generic(format!("HTTP provider error fetching logs: {}", e)))?,
        };

        debug!("Fetched {} logs", logs.len());

        // Group logs by block number to fetch blocks efficiently
        let mut logs_by_block: HashMap<u64, Vec<Log>> = HashMap::new();
        for log in logs {
            if let Some(block_num) = log.block_number {
                logs_by_block.entry(block_num.as_u64()).or_default().push(log);
            }
        }

        // Fetch the required blocks
        let block_numbers: Vec<u64> = logs_by_block.keys().cloned().collect();
        // Fetch blocks only if there are logs
        let blocks_map: HashMap<u64, ethers::types::Block<ethers::types::Transaction>> = if !block_numbers.is_empty() {
             self.fetch_blocks(
                 *block_numbers.iter().min().unwrap_or(&final_from_block),
                 *block_numbers.iter().max().unwrap_or(&final_to_block)
             ).await?
             .into_iter()
             .filter_map(|b| b.number.map(|n| (n.as_u64(), b)))
             .collect()
        } else {
            HashMap::new()
        };

        // Convert logs to events
        let mut events: Vec<Box<dyn Event>> = Vec::new();
        let processor = self.event_processor.read().await; // Acquire read lock before loop/filter
        for (block_num, block_logs) in logs_by_block {
             if let Some(block) = blocks_map.get(&block_num) {
                 for log in block_logs {
                    // process_log now returns Result<EthereumEvent>
                    match processor.process_log(log, block, None) { // Pass reference to block, None for receipt
                        Ok(event) => events.push(Box::new(event)), // Handle Result correctly
                        Err(e) => {
                            error!(log_block=?block_num, log_index=?log.log_index, tx_hash=?log.transaction_hash, "Failed to process log: {}", e);
                            // Decide if one error should stop all processing
                        }
                    }
                 }
             } else {
                 warn!(block_num=%block_num, "Could not find block data for logs in block, skipping.");
             }
        }
        drop(processor); // Drop read lock after parsing logs

        // Apply remaining filters (those not handled by get_logs)
        // Re-acquiring lock for filtering
        let processor_for_filter = self.event_processor.read().await;
        let filtered_events: Vec<Box<dyn Event>> = events // Collect type is correct
            .into_iter()
            .filter(|event| {
                 // Re-apply all filters for simplicity
                 filters.iter().all(|f| {
                     // Downcast event to EthereumEvent for matches_filter
                     if let Some(eth_event) = event.as_any().downcast_ref::<EthereumEvent>() {
                         processor_for_filter.matches_filter(eth_event, f)
                     } else {
                         false // Should not happen if only EthereumEvent is produced
                     }
                 })
            })
            .collect();
        drop(processor_for_filter); // Drop read lock after filtering

        Ok(filtered_events)
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
        
        // Determine range using block_range field
        let from_block = filters.iter()
            .filter_map(|f| f.block_range.map(|(min, _)| min)) // Use block_range
            .min()
            .unwrap_or(latest_block.saturating_sub(100)); // Default to last 100 blocks
        
        let to_block = filters.iter()
            .filter_map(|f| f.block_range.map(|(_, max)| max)) // Use block_range
            .max()
            .unwrap_or(latest_block);
        
        // Fetch logs for the block range
        self.fetch_logs(from_block, to_block, &filters).await
    }
    
    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> { // Use EventSubscription
        info!("Subscribing to Ethereum events");

        // Subscription requires a Websocket provider
        match &*self.provider { // Dereference Arc to check variant
            EthereumProvider::Websocket(provider_arc) => { // provider_arc is Arc<Provider<Ws>>
                // Clone the Arc for the subscription task
                let provider_clone = Arc::clone(provider_arc);
                let chain_id = ChainId(self.config.chain_id.clone()); // Use tuple struct constructor
                let subscription = EthereumSubscription::new(provider_clone, chain_id)
                    .await?;
                Ok(Box::new(subscription))
            }
            EthereumProvider::Http(_) => Err(Error::generic( 
                "Event subscription requires a Websocket provider".to_string(),
            )),
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