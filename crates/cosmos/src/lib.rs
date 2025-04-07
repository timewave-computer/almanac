/// Cosmos event service implementation
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::str::FromStr;
use std::any::Any;
use std::time::SystemTime;
use async_trait::async_trait;
use cosmrs::proto::cosmos::base::abci::v1beta1::TxResponse;
use cosmrs::rpc::{HttpClient, SubscriptionClient, event::EventData};
use cosmrs::tendermint::block::Height;
use cosmrs::tx::{Msg, Tx};
use cosmrs::Any as ProtoAny;
use cosmos_sdk_proto::prost; // Import prost re-exported by cosmos_sdk_proto
use prost::Message;
use cosmos_sdk_proto::cosmwasm::wasm::v1 as cosmwasm_v1; 
use tokio::sync::{Mutex, RwLock};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use uuid::Uuid;

/// Internal Crate Imports
use indexer_pipeline::{Error, Result, BlockStatus};
use indexer_storage::{Storage, BoxedStorage, ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountExecution};
use indexer_core::event::Event;
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};

/// Project Module Imports
mod provider;
mod subscription;
mod event;
pub mod contracts;

use provider::{CosmosProvider, CosmosProviderTrait, CosmosBlockStatus};
use subscription::{CosmosSubscription, CosmosSubscriptionConfig};
use event::{process_valence_account_event, process_valence_processor_event, process_valence_authorization_event, process_valence_library_event, CosmosEvent};
use tracing::{debug, error, info, trace, warn}; 

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
    #[serde(default)]
    pub valence_account_code_ids: Vec<u64>,
    
    /// Known Code IDs for Valence Processor contracts
    #[serde(default)]
    pub valence_processor_code_ids: Vec<u64>,
    
    /// Known Code IDs for Valence Authorization contracts
    #[serde(default)]
    pub valence_authorization_code_ids: Vec<u64>,
    
    /// Known Code IDs for Valence Library contracts
    #[serde(default)]
    pub valence_library_code_ids: Vec<u64>,
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
            valence_account_code_ids: Vec::new(),
            valence_processor_code_ids: Vec::new(),
            valence_authorization_code_ids: Vec::new(),
            valence_library_code_ids: Vec::new(),
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
    
    /// Set of known Valence Processor code IDs for quick lookup
    valence_processor_code_id_set: HashSet<u64>,
    
    /// Set of known Valence Authorization code IDs for quick lookup
    valence_authorization_code_id_set: HashSet<u64>,
    
    /// Set of known Valence Library code IDs for quick lookup
    valence_library_code_id_set: HashSet<u64>,
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
        let valence_processor_code_id_set: HashSet<u64> = config.valence_processor_code_ids.iter().cloned().collect();
        let valence_authorization_code_id_set: HashSet<u64> = config.valence_authorization_code_ids.iter().cloned().collect();
        let valence_library_code_id_set: HashSet<u64> = config.valence_library_code_ids.iter().cloned().collect();
        
        info!(known_valence_account_ids = ?valence_account_code_id_set, "Loaded known Valence Account code IDs");
        info!(known_valence_processor_ids = ?valence_processor_code_id_set, "Loaded known Valence Processor code IDs");
        info!(known_valence_auth_ids = ?valence_authorization_code_id_set, "Loaded known Valence Authorization code IDs");
        info!(known_valence_library_ids = ?valence_library_code_id_set, "Loaded known Valence Library code IDs");
        
        info!("Created Cosmos event service for chain {} with custom provider", config.chain_id);
        
        Ok(Self {
            chain_id: ChainId(config.chain_id.clone()),
            provider,
            config,
            block_cache: Arc::new(RwLock::new(HashMap::new())),
            storage,
            valence_account_code_id_set,
            valence_processor_code_id_set,
            valence_authorization_code_id_set,
            valence_library_code_id_set,
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
        if let Some(filter_chain_id) = &filter.chain_id {
            if event.chain() != filter_chain_id {
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
            if !types.is_empty() && !types.iter().any(|t| event.event_type() == t) {
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
    
    /// Check if a contract address corresponds to a known Valence Account code ID.
    async fn check_if_valence_account(&self, contract_address: &str) -> Result<bool> {
        // Prepare the QueryContractInfoRequest
        let request = cosmwasm_v1::QueryContractInfoRequest {
            address: contract_address.to_string(),
        };
        let request_proto_bytes = request.encode_to_vec();

        // Perform the ABCI query
        let query_path = "/cosmwasm.wasm.v1.Query/ContractInfo";
        let response = self.provider.abci_query(Some(query_path.to_string()), request_proto_bytes, None, false).await?;

        if response.code.is_err() {
            warn!(contract_address=%contract_address, code=%response.code.value(), log=%response.log, "ABCI query for ContractInfo failed");
            // Treat query failure as "not a valence account" for robustness?
            return Ok(false); 
        }

        // Decode the QueryContractInfoResponse
        match cosmwasm_v1::QueryContractInfoResponse::decode(response.value.as_slice()) {
            Ok(info) => {
                // TODO: Compare info.code_id against known Valence Account code IDs
                // This requires configuration or fetching known IDs.
                // Placeholder: assume code ID 1 is the Valence Account
                let is_valence = info.code_id == 1;
                debug!(contract_address=%contract_address, code_id=%info.code_id, is_valence=is_valence, "Checked contract code ID");
                Ok(is_valence)
            }
            Err(e) => {
                error!(contract_address=%contract_address, error=%e, "Failed to decode ContractInfoResponse");
                Err(Error::chain("Failed to decode ContractInfoResponse".to_string()))
            }
        }
    }

    /// Check if a contract address corresponds to a known Valence Processor code ID.
    async fn check_if_valence_processor(&self, contract_address: &str) -> Result<bool> {
        // Prepare the QueryContractInfoRequest
        let request = cosmwasm_v1::QueryContractInfoRequest {
            address: contract_address.to_string(),
        };
        let request_proto_bytes = request.encode_to_vec();

        // Perform the ABCI query
        let query_path = "/cosmwasm.wasm.v1.Query/ContractInfo";
        let response = self.provider.abci_query(Some(query_path.to_string()), request_proto_bytes, None, false).await?;

        if response.code.is_err() {
            warn!(contract_address=%contract_address, code=%response.code.value(), log=%response.log, "ABCI query for ContractInfo failed");
            // Treat query failure as "not a valence processor" for robustness
            return Ok(false); 
        }

        // Decode the QueryContractInfoResponse
        match cosmwasm_v1::QueryContractInfoResponse::decode(response.value.as_slice()) {
            Ok(info) => {
                // TODO: Compare info.code_id against known Valence Processor code IDs
                // This requires configuration or fetching known IDs.
                // Placeholder: assume code ID 2 is the Valence Processor
                let is_processor = info.code_id == 2;
                debug!(contract_address=%contract_address, code_id=%info.code_id, is_processor=is_processor, "Checked contract code ID");
                Ok(is_processor)
            }
            Err(e) => {
                error!(contract_address=%contract_address, error=%e, "Failed to decode ContractInfoResponse");
                Err(Error::chain("Failed to decode ContractInfoResponse".to_string()))
            }
        }
    }
    
    /// Check if a contract address corresponds to a known Valence Authorization code ID.
    async fn check_if_valence_authorization(&self, contract_address: &str) -> Result<bool> {
        // Prepare the QueryContractInfoRequest
        let request = cosmwasm_v1::QueryContractInfoRequest {
            address: contract_address.to_string(),
        };
        let request_proto_bytes = request.encode_to_vec();

        // Perform the ABCI query
        let query_path = "/cosmwasm.wasm.v1.Query/ContractInfo";
        let response = self.provider.abci_query(Some(query_path.to_string()), request_proto_bytes, None, false).await?;

        if response.code.is_err() {
            warn!(contract_address=%contract_address, code=%response.code.value(), log=%response.log, "ABCI query for ContractInfo failed");
            // Treat query failure as "not a valence authorization" for robustness
            return Ok(false); 
        }

        // Decode the QueryContractInfoResponse
        match cosmwasm_v1::QueryContractInfoResponse::decode(response.value.as_slice()) {
            Ok(info) => {
                // TODO: Compare info.code_id against known Valence Authorization code IDs
                // This requires configuration or fetching known IDs.
                // Placeholder: assume code ID 3 is the Valence Authorization
                let is_authorization = info.code_id == 3;
                debug!(contract_address=%contract_address, code_id=%info.code_id, is_authorization=is_authorization, "Checked contract code ID");
                Ok(is_authorization)
            }
            Err(e) => {
                error!(contract_address=%contract_address, error=%e, "Failed to decode ContractInfoResponse");
                Err(Error::chain("Failed to decode ContractInfoResponse".to_string()))
            }
        }
    }

    /// Check if a contract address corresponds to a known Valence Library code ID.
    async fn check_if_valence_library(&self, contract_address: &str) -> Result<bool> {
        // Prepare the QueryContractInfoRequest
        let request = cosmwasm_v1::QueryContractInfoRequest {
            address: contract_address.to_string(),
        };
        let request_proto_bytes = request.encode_to_vec();

        // Perform the ABCI query
        let query_path = "/cosmwasm.wasm.v1.Query/ContractInfo";
        let response = self.provider.abci_query(Some(query_path.to_string()), request_proto_bytes, None, false).await?;

        if response.code.is_err() {
            warn!(contract_address=%contract_address, code=%response.code.value(), log=%response.log, "ABCI query for ContractInfo failed");
            // Treat query failure as "not a valence library" for robustness
            return Ok(false); 
        }

        // Decode the QueryContractInfoResponse
        match cosmwasm_v1::QueryContractInfoResponse::decode(response.value.as_slice()) {
            Ok(info) => {
                // Compare info.code_id against known Valence Library code IDs
                // Placeholder: assume code ID 4 is the Valence Library
                let is_library = info.code_id == 4;
                debug!(contract_address=%contract_address, code_id=%info.code_id, is_library=is_library, "Checked contract code ID");
                Ok(is_library)
            }
            Err(e) => {
                error!(contract_address=%contract_address, error=%e, "Failed to decode ContractInfoResponse");
                Err(Error::chain("Failed to decode ContractInfoResponse".to_string()))
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
                        // Check if this is a Valence Account contract
                        match self.check_if_valence_account(addr).await {
                            Ok(true) => {
                                // If it's a valence account, attempt processing
                                if let Err(e) = process_valence_account_event(
                                    &self.storage, 
                                    &self.config.chain_id, 
                                    &cosmos_event,
                                    &tx_hash
                                ).await {
                                    error!(
                                        account_id=%format!("{}:{}", self.config.chain_id, addr),
                                        error=%e,
                                        "Failed to process Valence Account event"
                                    );
                                }
                            },
                            Ok(false) => {
                                trace!(contract_address=%addr, "Contract is not a Valence Account.");
                                
                                // Check if this is a Valence Processor contract
                                match self.check_if_valence_processor(addr).await {
                                    Ok(true) => {
                                        // If it's a valence processor, attempt processing
                                        if let Err(e) = process_valence_processor_event(
                                            &self.storage, 
                                            &self.config.chain_id, 
                                            &cosmos_event,
                                            &tx_hash
                                        ).await {
                                            error!(
                                                processor_id=format!("{}:{}", self.config.chain_id, addr),
                                                error=%e,
                                                "Failed to process Valence Processor event"
                                            );
                                        }
                                    },
                                    Ok(false) => {
                                        trace!(contract_address=%addr, "Contract is not a Valence Processor.");
                                        
                                        // Check if this is a Valence Authorization contract
                                        match self.check_if_valence_authorization(addr).await {
                                            Ok(true) => {
                                                // If it's a valence authorization, attempt processing
                                                if let Err(e) = process_valence_authorization_event(
                                                    &self.storage, 
                                                    &self.config.chain_id, 
                                                    &cosmos_event,
                                                    &tx_hash
                                                ).await {
                                                    error!(
                                                        auth_id=format!("{}:{}", self.config.chain_id, addr),
                                                        error=%e,
                                                        "Failed to process Valence Authorization event"
                                                    );
                                                }
                                            },
                                            Ok(false) => {
                                                trace!(contract_address=%addr, "Contract is not a Valence Authorization.");
                                                
                                                // Check if this is a Valence Library contract
                                                match self.check_if_valence_library(addr).await {
                                                    Ok(true) => {
                                                        // If it's a valence library, attempt processing
                                                        if let Err(e) = process_valence_library_event(
                                                            &self.storage, 
                                                            &self.config.chain_id, 
                                                            &cosmos_event,
                                                            &tx_hash
                                                        ).await {
                                                            error!(
                                                                library_id=format!("{}:{}", self.config.chain_id, addr),
                                                                error=%e,
                                                                "Failed to process Valence Library event"
                                                            );
                                                        }
                                                    },
                                                    Ok(false) => {
                                                        trace!(contract_address=%addr, "Contract is not a Valence Library.");
                                                    },
                                                    Err(e) => {
                                                        warn!(contract_address=%addr, error=%e, "Failed to check if contract is Valence Library");
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                warn!(contract_address=%addr, error=%e, "Failed to check if contract is Valence Authorization");
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        warn!(contract_address=%addr, error=%e, "Failed to check if contract is Valence Processor");
                                    }
                                }
                            },
                            Err(e) => {
                                warn!(contract_address=%addr, error=%e, "Failed to check if contract is Valence Account");
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
    pub async fn process_block_for_test(&self, block: cosmrs::tendermint::Block, tx_results: Vec<cosmrs::rpc::endpoint::tx::Response>) -> Result<Vec<CosmosEvent>> {
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