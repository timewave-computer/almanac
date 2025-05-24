/// Memory-based storage implementation for testing and examples
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use async_trait::async_trait;
use indexer_common::{Error, Result, BlockStatus};
use indexer_core::event::Event;
use serde::{Serialize, Deserialize};
use tracing::debug;

use crate::{
    Storage, EventFilter, ValenceAccountInfo, ValenceAccountLibrary, 
    ValenceAccountExecution, ValenceAccountState
};

use indexer_core::{
    types::{EventFilter, SortField, SortDirection},
    aggregation::{Aggregator, DefaultAggregator},
    types::{AggregationConfig, AggregationResult},
};

/// In-memory storage implementation suitable for examples
pub struct MemoryStorage {
    /// Events storage
    events: RwLock<Vec<EventWrapper>>,
    
    /// Latest blocks by chain
    latest_blocks: RwLock<HashMap<String, u64>>,
    
    /// Block statuses by chain and block
    block_statuses: RwLock<HashMap<String, BlockStatus>>,
    
    /// Valence account states
    valence_accounts: RwLock<HashMap<String, ValenceAccountState>>,
    
    /// Historical valence account states
    historical_valence_accounts: RwLock<HashMap<String, HashMap<u64, ValenceAccountState>>>,
    
    /// Latest historical blocks for valence accounts
    latest_historical_blocks: RwLock<HashMap<String, u64>>,
}

/// Event wrapper for storage
#[derive(Clone, Debug, Serialize, Deserialize)]
struct EventWrapper {
    /// Event ID
    id: String,
    
    /// Chain ID
    chain: String,
    
    /// Block number
    block_number: u64,
    
    /// Block hash
    block_hash: String,
    
    /// Transaction hash
    tx_hash: String,
    
    /// Event timestamp
    timestamp: u64,
    
    /// Event type
    event_type: String,
    
    /// Raw event data
    raw_data: Vec<u8>,
}

/// An event implementation for memory storage
#[derive(Debug)]
struct MemoryEvent {
    /// Event ID
    id: String,
    
    /// Chain ID
    chain: String,
    
    /// Block number
    block_number: u64,
    
    /// Block hash
    block_hash: String,
    
    /// Transaction hash
    tx_hash: String,
    
    /// Event timestamp
    timestamp: SystemTime,
    
    /// Event type
    event_type: String,
    
    /// Raw event data
    raw_data: Vec<u8>,
}

impl Event for MemoryEvent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn chain(&self) -> &str {
        &self.chain
    }
    
    fn block_number(&self) -> u64 {
        self.block_number
    }
    
    fn block_hash(&self) -> &str {
        &self.block_hash
    }
    
    fn tx_hash(&self) -> &str {
        &self.tx_hash
    }
    
    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    
    fn event_type(&self) -> &str {
        &self.event_type
    }
    
    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl MemoryStorage {
    /// Create a new memory storage instance
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
            latest_blocks: RwLock::new(HashMap::new()),
            block_statuses: RwLock::new(HashMap::new()),
            valence_accounts: RwLock::new(HashMap::new()),
            historical_valence_accounts: RwLock::new(HashMap::new()),
            latest_historical_blocks: RwLock::new(HashMap::new()),
        }
    }
    
    /// Helper function to create a block status key
    fn block_status_key(chain: &str, block_number: u64) -> String {
        format!("{}:{}", chain, block_number)
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        let event_wrapper = EventWrapper {
            id: event.id().to_string(),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp().duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            event_type: event.event_type().to_string(),
            raw_data: event.raw_data().to_vec(),
        };
        
        // Store the event
        let mut events = self.events.write().unwrap();
        events.push(event_wrapper);
        
        // Update latest block if needed
        let mut latest_blocks = self.latest_blocks.write().unwrap();
        let current_latest = latest_blocks.get(&event.chain().to_string()).copied().unwrap_or(0);
        if event.block_number() > current_latest {
            latest_blocks.insert(event.chain().to_string(), event.block_number());
        }
        
        Ok(())
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        let events = self.events.read().unwrap();
        
        let mut results: Vec<Box<dyn Event>> = Vec::new();
        
        for filter in filters {
            let mut matching_events: Vec<Box<dyn Event>> = events.iter()
                .filter(|e| filter.matches_event(e.as_ref()))
                .map(|e| {
                    let memory_event = MemoryEvent {
                        id: e.id.clone(),
                        chain: e.chain.clone(),
                        block_number: e.block_number,
                        block_hash: e.block_hash.clone(),
                        tx_hash: e.tx_hash.clone(),
                        timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(e.timestamp),
                        event_type: e.event_type.clone(),
                        raw_data: e.raw_data.clone(),
                    };
                    
                    Box::new(memory_event) as Box<dyn Event>
                })
                .collect();
            
            // If text search is enabled, apply it with proper scoring
            if let (Some(text_query), Some(text_config)) = (&filter.text_query, &filter.text_search_config) {
                use indexer_core::text_search::{DefaultTextSearcher, TextSearcher};
                
                let searcher = DefaultTextSearcher::new();
                if let Ok(search_results) = searcher.search(text_query, text_config, matching_events).await {
                    // Extract events from search results (sorted by score)
                    matching_events = search_results.into_iter().map(|r| r.event).collect();
                } else {
                    // If text search fails, continue with original results
                    debug!("Text search failed, using original filter results");
                }
            }
            
            // Apply sorting if specified
            if let (Some(sort_field), Some(sort_direction)) = (&filter.sort_by, &filter.sort_direction) {
                matching_events.sort_by(|a, b| {
                    let ordering = match sort_field {
                        indexer_core::types::SortField::BlockNumber => a.block_number().cmp(&b.block_number()),
                        indexer_core::types::SortField::Timestamp => a.timestamp().cmp(&b.timestamp()),
                        indexer_core::types::SortField::EventType => a.event_type().cmp(b.event_type()),
                        indexer_core::types::SortField::Chain => a.chain().cmp(b.chain()),
                        indexer_core::types::SortField::TxHash => a.tx_hash().cmp(b.tx_hash()),
                        indexer_core::types::SortField::Attribute(_attr) => {
                            // For attributes, we'd need access to event data
                            // This is a placeholder - would need event data parsing
                            std::cmp::Ordering::Equal
                        }
                    };
                    
                    match sort_direction {
                        indexer_core::types::SortDirection::Ascending => ordering,
                        indexer_core::types::SortDirection::Descending => ordering.reverse(),
                    }
                });
            }
            
            // Apply offset and limit
            if let Some(offset) = filter.offset {
                matching_events = matching_events.into_iter().skip(offset).collect();
            }

            if let Some(limit) = filter.limit {
                matching_events = matching_events.into_iter().take(limit).collect();
            }

            results.extend(matching_events);
        }
        
        Ok(results)
    }
    
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        let latest_blocks = self.latest_blocks.read().unwrap();
        Ok(latest_blocks.get(chain).copied().unwrap_or(0))
    }
    
    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        let key = Self::block_status_key(chain, block_number);
        let mut block_statuses = self.block_statuses.write().unwrap();
        block_statuses.insert(key, status);
        Ok(())
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        let block_statuses = self.block_statuses.read().unwrap();
        let latest_blocks = self.latest_blocks.read().unwrap();
        
        let max_block = latest_blocks.get(chain).copied().unwrap_or(0);
        
        // Find the highest block with the requested status
        let mut highest_matching = 0;
        for i in 0..=max_block {
            let key = Self::block_status_key(chain, i);
            if let Some(blk_status) = block_statuses.get(&key) {
                let matches = match status {
                    BlockStatus::Confirmed => true, // All blocks match confirmed
                    BlockStatus::Safe => *blk_status == BlockStatus::Safe || 
                                        *blk_status == BlockStatus::Justified || 
                                        *blk_status == BlockStatus::Finalized,
                    BlockStatus::Justified => *blk_status == BlockStatus::Justified || 
                                             *blk_status == BlockStatus::Finalized,
                    BlockStatus::Finalized => *blk_status == BlockStatus::Finalized,
                };
                
                if matches && i > highest_matching {
                    highest_matching = i;
                }
            }
        }
        
        Ok(highest_matching)
    }
    
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        // Get all events matching the filters
        let events = self.get_events(filters).await?;
        
        // Filter by block status
        let block_statuses = self.block_statuses.read().unwrap();
        
        let filtered_events = events.into_iter()
            .filter(|event| {
                let key = Self::block_status_key(event.chain(), event.block_number());
                if let Some(blk_status) = block_statuses.get(&key) {
                    match status {
                        BlockStatus::Confirmed => true, // All blocks match confirmed
                        BlockStatus::Safe => *blk_status == BlockStatus::Safe || 
                                           *blk_status == BlockStatus::Justified || 
                                           *blk_status == BlockStatus::Finalized,
                        BlockStatus::Justified => *blk_status == BlockStatus::Justified || 
                                                *blk_status == BlockStatus::Finalized,
                        BlockStatus::Finalized => *blk_status == BlockStatus::Finalized,
                    }
                } else {
                    // If no status is set, treat as unconfirmed
                    status == BlockStatus::Confirmed
                }
            })
            .collect();
        
        Ok(filtered_events)
    }

    // Valence Account Storage methods

    async fn store_valence_library_approval(
        &self,
        account_id: &str,
        library_info: ValenceAccountLibrary,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        let mut valence_accounts = self.valence_accounts.write().unwrap();
        
        if let Some(state) = valence_accounts.get_mut(account_id) {
            let library_address = library_info.library_address.clone();
            
            if !state.libraries.contains(&library_address) {
                state.libraries.push(library_address);
            }
            
            state.last_update_block = update_block;
            state.last_update_tx = update_tx.to_string();
        } else {
            // Account doesn't exist yet
            return Err(Error::generic(format!("Valence account not found: {}", account_id)));
        }
        
        Ok(())
    }

    // Additional minimal methods to satisfy the trait requirements

    async fn store_valence_account_instantiation(
        &self,
        account_info: ValenceAccountInfo,
        initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()> {
        // Create the account state
        let state = ValenceAccountState {
            account_id: account_info.id.clone(),
            chain_id: account_info.chain_id.clone(),
            address: account_info.contract_address.clone(),
            current_owner: account_info.current_owner.clone(),
            pending_owner: account_info.pending_owner.clone(),
            pending_owner_expiry: account_info.pending_owner_expiry,
            libraries: initial_libraries.iter().map(|lib| lib.library_address.clone()).collect(),
            last_update_block: account_info.created_at_block,
            last_update_tx: account_info.created_at_tx.clone(),
        };
        
        // Store the state
        let mut valence_accounts = self.valence_accounts.write().unwrap();
        valence_accounts.insert(account_info.id.clone(), state.clone());
        
        // Also store as historical state
        let mut historical = self.historical_valence_accounts.write().unwrap();
        let account_history = historical.entry(account_info.id.clone()).or_insert_with(HashMap::new);
        account_history.insert(account_info.created_at_block, state);
        
        // Update latest historical block
        let mut latest_blocks = self.latest_historical_blocks.write().unwrap();
        latest_blocks.insert(account_info.id, account_info.created_at_block);
        
        Ok(())
    }

    async fn store_valence_library_removal(
        &self,
        account_id: &str,
        library_address: &str,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        let mut valence_accounts = self.valence_accounts.write().unwrap();
        
        if let Some(state) = valence_accounts.get_mut(account_id) {
            state.libraries.retain(|lib| lib != library_address);
            state.last_update_block = update_block;
            state.last_update_tx = update_tx.to_string();
        } else {
            return Err(Error::generic(format!("Valence account not found: {}", account_id)));
        }
        
        Ok(())
    }

    async fn store_valence_ownership_update(
        &self,
        account_id: &str,
        new_owner: Option<String>,
        new_pending_owner: Option<String>,
        new_pending_expiry: Option<u64>,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        let mut valence_accounts = self.valence_accounts.write().unwrap();
        
        if let Some(state) = valence_accounts.get_mut(account_id) {
            state.current_owner = new_owner;
            state.pending_owner = new_pending_owner;
            state.pending_owner_expiry = new_pending_expiry;
            state.last_update_block = update_block;
            state.last_update_tx = update_tx.to_string();
        } else {
            return Err(Error::generic(format!("Valence account not found: {}", account_id)));
        }
        
        Ok(())
    }

    async fn store_valence_execution(
        &self,
        _execution_info: ValenceAccountExecution,
    ) -> Result<()> {
        // Simplified implementation for the example
        Ok(())
    }

    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>> {
        let valence_accounts = self.valence_accounts.read().unwrap();
        Ok(valence_accounts.get(account_id).cloned())
    }

    async fn set_valence_account_state(&self, account_id: &str, state: &ValenceAccountState) -> Result<()> {
        let mut valence_accounts = self.valence_accounts.write().unwrap();
        valence_accounts.insert(account_id.to_string(), state.clone());
        Ok(())
    }

    async fn delete_valence_account_state(&self, account_id: &str) -> Result<()> {
        let mut valence_accounts = self.valence_accounts.write().unwrap();
        valence_accounts.remove(account_id);
        Ok(())
    }

    async fn set_historical_valence_account_state(
        &self, 
        account_id: &str, 
        block_number: u64, 
        state: &ValenceAccountState
    ) -> Result<()> {
        let mut historical = self.historical_valence_accounts.write().unwrap();
        let account_history = historical.entry(account_id.to_string()).or_insert_with(HashMap::new);
        account_history.insert(block_number, state.clone());
        Ok(())
    }

    async fn get_historical_valence_account_state(
        &self, 
        account_id: &str, 
        block_number: u64
    ) -> Result<Option<ValenceAccountState>> {
        let historical = self.historical_valence_accounts.read().unwrap();
        if let Some(account_history) = historical.get(account_id) {
            Ok(account_history.get(&block_number).cloned())
        } else {
            Ok(None)
        }
    }

    async fn delete_historical_valence_account_state(
        &self, 
        account_id: &str, 
        block_number: u64
    ) -> Result<()> {
        let mut historical = self.historical_valence_accounts.write().unwrap();
        if let Some(account_history) = historical.get_mut(account_id) {
            account_history.remove(&block_number);
        }
        Ok(())
    }

    async fn set_latest_historical_valence_block(
        &self, 
        account_id: &str, 
        block_number: u64
    ) -> Result<()> {
        let mut latest_blocks = self.latest_historical_blocks.write().unwrap();
        latest_blocks.insert(account_id.to_string(), block_number);
        Ok(())
    }

    async fn get_latest_historical_valence_block(
        &self, 
        account_id: &str
    ) -> Result<Option<u64>> {
        let latest_blocks = self.latest_historical_blocks.read().unwrap();
        Ok(latest_blocks.get(account_id).copied())
    }

    async fn delete_latest_historical_valence_block(
        &self, 
        account_id: &str
    ) -> Result<()> {
        let mut latest_blocks = self.latest_historical_blocks.write().unwrap();
        latest_blocks.remove(account_id);
        Ok(())
    }

    // --- Valence Processor Methods ---
    
    async fn store_valence_processor_instantiation(
        &self,
        _processor_info: ValenceProcessorInfo,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn store_valence_processor_config_update(
        &self,
        _processor_id: &str,
        _config: ValenceProcessorConfig,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn store_valence_processor_message(
        &self,
        _message: ValenceProcessorMessage,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn update_valence_processor_message_status(
        &self,
        _message_id: &str,
        _new_status: ValenceMessageStatus,
        _processed_block: Option<u64>,
        _processed_tx: Option<&str>,
        _retry_count: Option<u32>,
        _next_retry_block: Option<u64>,
        _gas_used: Option<u64>,
        _error: Option<String>,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn get_valence_processor_state(&self, _processor_id: &str) -> Result<Option<ValenceProcessorState>> {
        // Simplified implementation for examples
        Ok(None)
    }
    
    async fn set_valence_processor_state(&self, _processor_id: &str, _state: &ValenceProcessorState) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn set_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
        _state: &ValenceProcessorState,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn get_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
    ) -> Result<Option<ValenceProcessorState>> {
        // Simplified implementation for examples
        Ok(None)
    }
    
    // --- Valence Authorization Methods ---
    
    async fn store_valence_authorization_instantiation(
        &self,
        _auth_info: ValenceAuthorizationInfo,
        _initial_policy: Option<ValenceAuthorizationPolicy>,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn store_valence_authorization_policy(
        &self,
        _policy: ValenceAuthorizationPolicy,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn update_active_authorization_policy(
        &self,
        _auth_id: &str,
        _policy_id: &str,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn store_valence_authorization_grant(
        &self,
        _grant: ValenceAuthorizationGrant,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn revoke_valence_authorization_grant(
        &self,
        _auth_id: &str,
        _grantee: &str,
        _resource: &str,
        _revoked_at_block: u64,
        _revoked_at_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn store_valence_authorization_request(
        &self,
        _request: ValenceAuthorizationRequest,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn update_valence_authorization_request_decision(
        &self,
        _request_id: &str,
        _decision: ValenceAuthorizationDecision,
        _processed_block: Option<u64>,
        _processed_tx: Option<&str>,
        _reason: Option<String>,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    // --- Valence Library Methods ---
    
    async fn store_valence_library_instantiation(
        &self,
        _library_info: ValenceLibraryInfo,
        _initial_version: Option<ValenceLibraryVersion>,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn store_valence_library_version(
        &self,
        _version: ValenceLibraryVersion,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn update_active_library_version(
        &self,
        _library_id: &str,
        _version: u32,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn store_valence_library_usage(
        &self,
        _usage: ValenceLibraryUsage,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn revoke_valence_library_approval(
        &self,
        _library_id: &str,
        _account_id: &str,
        _revoked_at_block: u64,
        _revoked_at_tx: &str,
    ) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn get_valence_library_state(&self, _library_id: &str) -> Result<Option<ValenceLibraryState>> {
        // Simplified implementation for examples
        Ok(None)
    }
    
    async fn set_valence_library_state(&self, _library_id: &str, _state: &ValenceLibraryState) -> Result<()> {
        // Simplified implementation for examples
        Ok(())
    }
    
    async fn get_valence_library_versions(&self, _library_id: &str) -> Result<Vec<ValenceLibraryVersion>> {
        // Simplified implementation for examples
        Ok(Vec::new())
    }
    
    async fn get_valence_library_approvals(&self, _library_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        // Simplified implementation for examples
        Ok(Vec::new())
    }
    
    async fn get_valence_libraries_for_account(&self, _account_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        // Simplified implementation for examples
        Ok(Vec::new())
    }
    
    async fn get_valence_library_usage_history(
        &self,
        _library_id: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<ValenceLibraryUsage>> {
        // Simplified implementation for examples
        Ok(Vec::new())
    }

    /// Get aggregated event data
    pub async fn aggregate_events(&self, config: AggregationConfig) -> Result<Vec<AggregationResult>> {
        let events = self.events.read().unwrap();
        
        // Convert stored events to Event trait objects
        let event_objects: Vec<Box<dyn Event>> = events.iter()
            .map(|e| {
                let memory_event = MemoryEvent {
                    id: e.id.clone(),
                    chain: e.chain.clone(),
                    block_number: e.block_number,
                    block_hash: e.block_hash.clone(),
                    tx_hash: e.tx_hash.clone(),
                    timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(e.timestamp),
                    event_type: e.event_type.clone(),
                    raw_data: e.raw_data.clone(),
                };
                Box::new(memory_event) as Box<dyn Event>
            })
            .collect();
        
        let aggregator = DefaultAggregator::new();
        aggregator.aggregate(event_objects, &config).await
    }
} 