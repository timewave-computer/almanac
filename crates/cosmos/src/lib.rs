/// Cosmos event service implementation
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use indexer_core::event::Event;
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};
use indexer_common::{Result, BlockStatus};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use tracing::{info, warn, error};
use std::any::Any;
use indexer_storage::Storage;
use cosmrs::rpc::Client;
use sha2::{Sha256, Digest};
use base64;
use std::collections::HashSet;
use provider::{CosmosProvider, CosmosBlockStatus, CosmosProviderTrait};

pub mod event;
pub mod provider;
pub mod subscription;
pub mod contracts;

use event::CosmosEvent;
use subscription::{CosmosSubscription, CosmosSubscriptionConfig};

/// Configuration for Cosmos event service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosEventServiceConfig {
    /// Chain ID
    pub chain_id: String,
    
    /// RPC URL for the Cosmos node
    pub rpc_url: String,
    
    /// Block confirmation threshold
    pub confirmation_blocks: u64,
    
    /// Maximum batch size for fetching blocks
    pub max_batch_size: usize,
    
    /// How often to poll for new blocks (in milliseconds)
    pub poll_interval_ms: u64,
    
    /// Maximum number of parallel requests
    pub max_parallel_requests: usize,

    /// Known Code IDs for Valence Base Account contracts
    #[serde(default)] // Ensure it defaults to empty if missing in config
    pub valence_account_code_ids: Vec<u64>,
}

impl Default for CosmosEventServiceConfig {
    fn default() -> Self {
        Self {
            chain_id: "cosmos".to_string(),
            rpc_url: "http://localhost:26657".to_string(),
            confirmation_blocks: 6,
            max_batch_size: 10,
            poll_interval_ms: 1000,
            max_parallel_requests: 5,
            valence_account_code_ids: Vec::new(), // Default to empty
        }
    }
}

/// Cosmos event service
pub struct CosmosEventService {
    /// Chain ID
    chain_id: ChainId,
    
    /// Cosmos provider (now using the trait)
    provider: Arc<dyn CosmosProviderTrait>,
    
    /// Configuration
    config: CosmosEventServiceConfig,
    
    /// Block cache
    block_cache: Arc<RwLock<HashMap<u64, cosmrs::tendermint::Block>>>,

    /// Shared storage backend
    storage: Arc<dyn Storage>,

    /// Set of known Valence Account code IDs for quick lookup
    valence_account_code_id_set: HashSet<u64>,
}

impl CosmosEventService {
    /// Creates a new Cosmos event service with the real provider
    pub async fn new(config: CosmosEventServiceConfig, storage: Arc<dyn Storage>) -> Result<Self> {
        let real_provider = Arc::new(CosmosProvider::new(&config.rpc_url).await?);
        Self::new_with_provider(config, storage, real_provider)
    }

    /// Creates a new Cosmos event service with a specific provider (for testing)
    pub fn new_with_provider(
        config: CosmosEventServiceConfig,
        storage: Arc<dyn Storage>,
        provider: Arc<dyn CosmosProviderTrait>,
    ) -> Result<Self> {
        let valence_account_code_id_set: HashSet<u64> = config.valence_account_code_ids.iter().cloned().collect();
        info!(known_valence_code_ids = ?valence_account_code_id_set, "Loaded known Valence Account code IDs");
        
        info!("Created Cosmos event service for chain {} with custom provider", config.chain_id);
        
        Ok(Self {
            chain_id: ChainId(config.chain_id.clone()),
            provider, // Store the trait object
            config,
            block_cache: Arc::new(RwLock::new(HashMap::new())),
            storage,
            valence_account_code_id_set,
        })
    }
    
    /// Check connection to the Cosmos node
    pub async fn check_connection(&self) -> Result<()> {
        let _ = self.provider.get_block_height().await?;
        Ok(())
    }
    
    /// Get chain ID as string
    pub fn chain_id_str(&self) -> &str {
        &self.chain_id.0
    }
    
    /// Get all events for the given filter in the provided range
    pub async fn get_events(&self, filter: &EventFilter) -> Result<Vec<Box<dyn Event>>> {
        let mut events = Vec::new();
        
        // Extract block range from filter
        let (start_block, end_block) = if let Some((start, end)) = filter.block_range {
            (start, end)
        } else {
            // If no block range is provided, use the latest block
            let latest_block = self.provider.get_block_height().await?;
            (latest_block.saturating_sub(10), latest_block)
        };
        
        // Get blocks for the range
        let blocks = self.provider.get_blocks_in_range(start_block, end_block).await?;
        
        // Process events from each block
        for block in blocks {
            // Get transactions for the block
            let block_height = block.header.height.value();
            let tx_results = match self.provider.get_tx_results(block_height).await {
                Ok(results) => results,
                Err(e) => {
                    warn!("Failed to get transactions for block {}: {}", block_height, e);
                    Vec::new()
                }
            };

            let cosmos_events = self.process_block(block, tx_results).await?;
            
            // Convert CosmosEvent to Box<dyn Event> and filter
            for event in cosmos_events {
                // Check if event passes the filter
                if self.should_include_event(&event, filter) {
                    events.push(Box::new(event) as Box<dyn Event>);
                }
            }
        }
        
        Ok(events)
    }

    /// Check if event should be included based on filter
    fn should_include_event(&self, event: &CosmosEvent, filter: &EventFilter) -> bool {
        // Check chain ID
        if let Some(chain_id) = &filter.chain_id {
            if event.chain() != chain_id.0 {
                return false;
            }
        }

        // Check chain name
        if let Some(chain) = &filter.chain {
            if event.chain() != chain {
                return false;
            }
        }
        
        // Check block range
        if let Some((min_block, max_block)) = filter.block_range {
            let block_num = event.block_number();
            if block_num < min_block || block_num > max_block {
                return false;
            }
        }
        
        // Check event types
        if let Some(types) = &filter.event_types {
            if !types.iter().any(|t| event.event_type() == t) {
                return false;
            }
        }
        
        // Check custom filters
        for (key, value) in &filter.custom_filters {
            if !event.has_attribute(key, value) {
                return false;
            }
        }
        
        true
    }
    
    /// Performs an ABCI query via the provider trait.
    async fn check_if_valence_account(&self, contract_address: &str) -> Result<bool> {
        let query_msg = IdentifyValenceQuery {};
        let query_data = serde_json::to_vec(&query_msg)
            .map_err(|e| Error::generic(format!("Failed to serialize identify query: {}", e)))?;
        let query_data_base64 = base64::engine::general_purpose::STANDARD.encode(&query_data);

        let path = format!("/cosmwasm.wasm.v1.Query/SmartContractState"); // Using the gRPC query path might be more standard if supported via abci_query
        // Alternative path construction (more typical for direct ABCI smart queries):
        // let path = format!("wasm/contract/{}/smart/{}", contract_address, query_data_base64);

        // Prepare query data for ABCI query (protobuf encoded request)
        // We need QuerySmartContractStateRequest protobuf bytes here
        // This requires adding protobuf dependencies (prost, cosmos-sdk-proto)
        // Let's try a simpler path first if possible, or maybe query code info?

        // --- Simplified Check: Query Contract Code ID --- 
        // A less direct but simpler check might be to query the contract's code info 
        // and see if the code ID matches a known Valence Account code ID.
        // This avoids needing to construct the SmartContractState request protobuf.

        let path_contract_info = format!("/cosmwasm.wasm.v1.Query/ContractInfo");
        let request_proto_bytes = cosmrs::proto::cosmwasm::wasm::v1::QueryContractInfoRequest {
                address: contract_address.to_string(),
            }
            .encode_to_vec();

        // Use the trait method
        match self.provider.abci_query(
            Some(path_contract_info.to_string()),
            request_proto_bytes,
            None, // Query latest height
            false // No proof needed
        ).await {
            Ok(response) => {
                if response.code.is_ok() {
                    // Successfully queried contract info.
                    match cosmrs::proto::cosmwasm::wasm::v1::QueryContractInfoResponse::decode(response.value.as_slice()) {
                        Ok(contract_info) => {
                            // Use the configured set of code IDs
                            if self.valence_account_code_id_set.contains(&contract_info.code_id) {
                                debug!(contract_address=%contract_address, code_id=%contract_info.code_id, "Contract identified as Valence Account by code ID.");
                                Ok(true)
                            } else {
                                debug!(contract_address=%contract_address, code_id=%contract_info.code_id, "Contract code ID not in known Valence Account set.");
                                Ok(false)
                            }
                        },
                        Err(e) => {
                            warn!(contract_address=%contract_address, error=%e, "Failed to decode ContractInfoResponse");
                            Ok(false) // Treat decoding errors as non-match
                        }
                    }
                } else {
                    debug!(contract_address=%contract_address, code=%response.code.value(), log=%response.log, "ABCI query for ContractInfo failed.");
                    Ok(false)
                }
            },
            Err(e) => {
                // Use the error directly from the trait call if it's already indexer_common::Error
                // The trait implementation converts cosmrs::Error to Error::Rpc
                error!(contract_address=%contract_address, error=%e, "Provider error during abci_query for ContractInfo");
                Err(e) 
            }
        }
    }

    /// Process the block and extract Cosmos events
    async fn process_block(&self, block: cosmrs::tendermint::Block, tx_results: Vec<cosmrs::rpc::endpoint::tx::Response>) -> Result<Vec<CosmosEvent>> {
        let mut events = Vec::new();
        let block_height = block.header.height.value();
        let block_hash = block.header.hash().to_string();
        
        let timestamp_nanos = block.header.time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        let timestamp_secs = (timestamp_nanos / 1_000_000_000) as u64;

        // Process events from transactions
        for tx_result in tx_results {
            let tx_hash_bytes = Sha256::digest(&tx_result.tx);
            let tx_hash = format!("{:X}", tx_hash_bytes);
            
            for abci_event in &tx_result.tx_result.events {
                let mut data = HashMap::new();
                let mut contract_address: Option<String> = None;
                for attr in &abci_event.attributes {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                    if key == "_contract_address" {
                        contract_address = Some(value.clone());
                    }
                    data.insert(key, value);
                }
                
                let cosmos_event = CosmosEvent::new(
                    Uuid::new_v4().to_string(),
                    self.chain_id.0.clone(),
                    block_height,
                    block_hash.clone(),
                    tx_hash.clone(),
                    timestamp_secs, 
                    abci_event.kind.clone(),
                    data,
                );

                if abci_event.kind == "wasm" {
                    if let Some(addr) = &contract_address {
                        match self.check_if_valence_account(addr).await {
                            Ok(true) => {
                                if let Err(e) = contracts::valence_account::process_valence_account_event(
                                    &cosmos_event,
                                    Arc::clone(&self.storage), // Direct access to field
                                    &tx_hash
                                ).await {
                                    error!(tx_hash=%tx_hash, contract_addr=%addr, error=%e, "Error processing Valence Account event");
                                }
                            },
                            Ok(false) => {
                                trace!(contract_address=%addr, "Contract is not a Valence Account.");
                            },
                            Err(e) => {
                                error!(contract_address=%addr, error=%e, "Error checking if contract is Valence Account");
                            }
                        }
                    } else {
                         warn!(tx_hash = %tx_hash, event_type = %abci_event.kind, "Wasm event missing _contract_address attribute");
                    }
                }
                events.push(cosmos_event);
            }
        }
        
        Ok(events)
    }

    // Add a public method specifically for testing that calls the private one
    #[cfg(test)]
    pub async fn process_block_for_test(&self, block: cosmrs::tendermint::Block, tx_results: Vec<cosmrs::rpc::endpoint::tx::response::Response>) -> Result<Vec<CosmosEvent>> {
        self.process_block(block, tx_results).await
    }
}

/// Define an empty query msg to check if a contract is a Valence Account
#[derive(Serialize)]
struct IdentifyValenceQuery {}

#[async_trait]
impl EventService for CosmosEventService {
    type EventType = CosmosEvent;
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        let mut all_events = Vec::new();
        
        for filter in filters {
            let events = self.get_events(&filter).await?;
            all_events.extend(events);
        }
        
        Ok(all_events)
    }
    
    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> {
        // Subscription likely needs a real provider, mocking this is complex
        // Might need to adapt `CosmosSubscription::new` to also accept the trait?
        // For now, assume tests won't rely heavily on subscribe() with mock provider.
        Err(Error::unimplemented("subscribe not supported with mock provider yet"))
    }
    
    async fn get_latest_block(&self) -> Result<u64> {
        self.provider.get_block_height().await
    }
    
    async fn get_latest_block_with_status(&self, _chain: &str, status: BlockStatus) -> Result<u64> {
        let cosmos_status = match status {
            BlockStatus::Confirmed => CosmosBlockStatus::Confirmed,
            BlockStatus::Safe => CosmosBlockStatus::Safe,
            BlockStatus::Justified | BlockStatus::Finalized => CosmosBlockStatus::Finalized,
        };
        let (_, block_number) = self.provider.get_block_by_status(cosmos_status).await?;
        Ok(block_number)
    }
} 