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
                .filter(|e| {
                    // Apply chain filter
                    if let Some(chain) = &filter.chain {
                        if &e.chain != chain {
                            return false;
                        }
                    }
                    
                    // Apply block range filter
                    if let Some((min_block, max_block)) = filter.block_range {
                        if e.block_number < min_block || e.block_number > max_block {
                            return false;
                        }
                    }
                    
                    // Apply time range filter
                    if let Some((min_time, max_time)) = filter.time_range {
                        if e.timestamp < min_time || e.timestamp > max_time {
                            return false;
                        }
                    }
                    
                    // Apply event type filter
                    if let Some(event_types) = &filter.event_types {
                        if !event_types.contains(&e.event_type) {
                            return false;
                        }
                    }
                    
                    true
                })
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
} 