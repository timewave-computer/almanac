/// RocksDB storage implementation
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use std::any::Any;
use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use indexer_common::{BlockStatus, Error, Result};
use indexer_core::event::Event;
use rocksdb::{Options, DB, WriteBatch, IteratorMode, Direction, BlockBasedOptions, BoundColumnFamily, ColumnFamily};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use serde_json;
use num_cpus;
use bincode;

use crate::EventFilter;
use crate::Storage;
use crate::{ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountExecution, ValenceAccountState};
use crate::{
    ValenceProcessorInfo, ValenceProcessorConfig, ValenceProcessorMessage, ValenceMessageStatus,
    ValenceProcessorState, ValenceAuthorizationInfo, ValenceAuthorizationPolicy, ValenceAuthorizationGrant,
    ValenceAuthorizationRequest, ValenceAuthorizationDecision, ValenceLibraryInfo, ValenceLibraryVersion,
    ValenceLibraryUsage, ValenceLibraryState, ValenceLibraryApproval
};

/// Configuration for RocksDB storage
pub struct RocksConfig {
    /// Path to the database
    pub path: String,
    
    /// Whether to create if missing
    pub create_if_missing: bool,

    /// Cache size in megabytes
    pub cache_size_mb: usize,
}

impl Default for RocksConfig {
    fn default() -> Self {
        Self {
            path: "data/rocksdb".to_string(),
            create_if_missing: true,
            cache_size_mb: 128,
        }
    }
}

/// A key in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    /// Key namespace
    pub namespace: String,
    
    /// Key identifier
    pub id: String,
}

impl Key {
    /// Create a new key
    pub fn new(namespace: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            id: id.into(),
        }
    }
    
    /// Convert to byte string for storage
    pub fn to_bytes(&self) -> Vec<u8> {
        format!("{}:{}", self.namespace, self.id).into_bytes()
    }
    
    /// Create from byte string
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let s = String::from_utf8(bytes.to_vec())
            .map_err(|_| Error::generic("Invalid key format"))?;
        
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(Error::generic("Invalid key format"));
        }
        
        Ok(Self {
            namespace: parts[0].to_string(),
            id: parts[1].to_string(),
        })
    }

    /// Create a prefix key for range scans
    pub fn prefix(namespace: impl Into<String>) -> Vec<u8> {
        format!("{}:", namespace.into()).into_bytes()
    }
}

/// RocksDB storage
pub struct RocksStorage {
    /// Database instance
    db: Arc<DB>,
}

#[async_trait]
impl Storage for RocksStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        let key = Key::new("events", event.id());
        
        // Convert event to JSON for storage
        let event_data = serde_json::to_string(&EventData {
            id: event.id().to_string(),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp().duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event_type: event.event_type().to_string(),
            raw_data: event.raw_data().to_vec(),
        })?;
        
        // Start a batch operation for atomicity
        let mut batch = self.create_write_batch();
        
        // Store the event
        batch.put(&key, event_data.as_bytes());
        
        // Add secondary indexes for efficient querying
        
        // Chain + block index (for querying by chain and block range)
        let chain_block_key = Key::new(
            "index:chain_block", 
            format!("{}:{}", event.chain(), event.block_number())
        );
        batch.put(&chain_block_key, event.id().as_bytes());
        
        // Chain + event type index (for filtering by event type)
        let chain_type_key = Key::new(
            "index:chain_type", 
            format!("{}:{}", event.chain(), event.event_type())
        );
        batch.put(&chain_type_key, event.id().as_bytes());
        
        // Chain + time index (for time-based queries)
        let timestamp = event.timestamp().duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let chain_time_key = Key::new(
            "index:chain_time", 
            format!("{}:{:016x}", event.chain(), timestamp)
        );
        batch.put(&chain_time_key, event.id().as_bytes());
        
        // Update latest block for chain
        let latest_block_key = Key::new("latest_block", event.chain());
        let current_latest = self.get(&latest_block_key)?
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        if event.block_number() > current_latest {
            batch.put(&latest_block_key, event.block_number().to_string().as_bytes());
        }
        
        // Update block hash mapping
        let block_key = Key::new("block", format!("{}:{}", event.chain(), event.block_number()));
        batch.put(&block_key, event.block_hash().as_bytes());
        
        // Write the batch
        self.write_batch(batch)?;
        
        Ok(())
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        debug!("Getting events from RocksDB with {} filters", filters.len());
        
        // If there are no filters, return empty results
        if filters.is_empty() {
            return Ok(Vec::new());
        }
        
        // We'll process each filter separately and then combine the results
        let mut all_results: Vec<Box<dyn Event>> = Vec::new();
        
        for filter in filters {
            // Determine the most efficient query strategy based on the filter
            let event_ids = if let Some(chain) = &filter.chain {
                if let Some((min_block, max_block)) = filter.block_range {
                    // If we have a chain and block range, use the chain_block index
                    self.get_event_ids_by_chain_and_block_range(chain, min_block, max_block)?
                } else if let Some(event_types) = &filter.event_types {
                    // If we have chain and event types, use the chain_type index
                    self.get_event_ids_by_chain_and_event_types(chain, event_types)?
                } else if let Some((min_time, max_time)) = filter.time_range {
                    // If we have chain and time range, use the chain_time index
                    self.get_event_ids_by_chain_and_time_range(chain, min_time, max_time)?
                } else {
                    // If we only have a chain, scan all events for that chain
                    self.get_event_ids_by_chain(chain)?
                }
            } else {
                // If no chain specified, scan all events (expensive!)
                self.get_all_event_ids()?
            };
            
            // Now get the actual events by their IDs
            let mut events = Vec::new();
            for id in event_ids {
                if let Some(event) = self.get_event_by_id(&id)? {
                    events.push(event);
                }
            }
            
            // Apply any remaining filters that weren't covered by the index lookup
            let filtered_events = self.apply_remaining_filters(events, &filter);
            
            // Apply limit and offset if specified
            let mut result = filtered_events;
            if let Some(offset) = filter.offset {
                result = result.into_iter().skip(offset).collect();
            }
            if let Some(limit) = filter.limit {
                result = result.into_iter().take(limit).collect();
            }
            
            all_results.extend(result);
        }
        
        Ok(all_results)
    }
    
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        debug!("Getting latest block for chain {}", chain);
        
        let latest_block_key = Key::new("latest_block", chain);
        let result = self.get(&latest_block_key)?
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        Ok(result)
    }
    
    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        // Convert the BlockStatus enum to a string representation
        let status_str = match status {
            BlockStatus::Confirmed => "confirmed",
            BlockStatus::Safe => "safe",
            BlockStatus::Justified => "justified",
            BlockStatus::Finalized => "finalized",
        };
        
        let key = Key::new("block_status", format!("{}:{}", chain, block_number));
        self.put(&key, status_str.as_bytes())?;
        
        // Also store the latest block with this status
        let latest_status_key = Key::new(format!("latest_block_status:{}", status_str), chain);
        let current_latest = self.get(&latest_status_key)?
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        if block_number > current_latest {
            self.put(&latest_status_key, block_number.to_string().as_bytes())?;
        }
        
        Ok(())
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        debug!("Getting latest block with status {:?} for chain {}", status, chain);
        
        let status_str = match status {
            BlockStatus::Confirmed => "confirmed",
            BlockStatus::Safe => "safe",
            BlockStatus::Justified => "justified",
            BlockStatus::Finalized => "finalized",
        };
        
        let latest_status_key = Key::new(format!("latest_block_status:{}", status_str), chain);
        let result = self.get(&latest_status_key)?
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        Ok(result)
    }
    
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        debug!("Getting events with status {:?}", status);
        
        let events = self.get_events(filters).await?;
        
        // Filter events by block status
        let mut filtered_events = Vec::new();
        for event in events {
            let block_status_key = Key::new("block_status", format!("{}:{}", event.chain(), event.block_number()));
            if let Some(status_bytes) = self.get(&block_status_key)? {
                let event_status = std::str::from_utf8(&status_bytes).unwrap_or("");
                let matches = match status {
                    BlockStatus::Confirmed => event_status == "confirmed" || event_status == "safe" || 
                                             event_status == "justified" || event_status == "finalized",
                    BlockStatus::Safe => event_status == "safe" || event_status == "justified" || event_status == "finalized",
                    BlockStatus::Justified => event_status == "justified" || event_status == "finalized",
                    BlockStatus::Finalized => event_status == "finalized",
                };
                
                if matches {
                    filtered_events.push(event);
                }
            }
        }
        
        Ok(filtered_events)
    }

    // --- Valence Account Storage Methods ---

    async fn store_valence_account_instantiation(
        &self,
        account_info: ValenceAccountInfo,
        initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()> {
        // Create the initial state from the info
        let state = ValenceAccountState {
            account_id: account_info.id.clone(), // Use clone if needed
            chain_id: account_info.chain_id.clone(),
            address: account_info.contract_address.clone(),
            current_owner: account_info.current_owner.clone(),
            pending_owner: account_info.pending_owner.clone(),
            pending_owner_expiry: account_info.pending_owner_expiry,
            libraries: initial_libraries
                .iter()
                .map(|lib| lib.library_address.clone())
                .collect(),
            last_update_block: account_info.created_at_block, // Initial update is creation
            last_update_tx: account_info.created_at_tx.clone(),
        };

        // Store the initial state
        self.set_valence_account_state(&account_info.id, &state).await?;

        // We might also want to store the initial historical state if required
        self.set_historical_valence_account_state(
            &account_info.id,
            account_info.created_at_block,
            &state,
        )
        .await?;

        // Set the latest historical block marker
        self.set_latest_historical_valence_block(&account_info.id, account_info.created_at_block)
            .await?;

        Ok(())
    }

    async fn store_valence_library_approval(
        &self,
        account_id: &str,
        library_info: ValenceAccountLibrary,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        // Fix the implementation
        let libraries_cf = self.cf_valence_state()?;
        let key = format!("{}:{}", account_id, library_info.library_address);
        
        // Serialize the library info
        let value = serde_json::to_string(&library_info)
            .map_err(|e| Error::generic(format!("Failed to serialize ValenceAccountLibrary: {}", e)))?;
        
        self.db.put_cf(libraries_cf, key, value)?;

        // Update the account's last updated block separately
        let state_key = self.valence_account_state_key(account_id);
        let cf = self.cf_valence_state()?;
        
        // Read-Modify-Write for the account state
        if let Some(state_bytes) = self.db.get_cf(cf, &state_key)? {
            let mut state: ValenceAccountState = serde_json::from_slice(&state_bytes)
                .map_err(|e| Error::generic(format!("Failed to deserialize Valence state: {}", e)))?;
            
            state.last_update_block = update_block;
            state.last_update_tx = update_tx.to_string();
            
            let updated_state = serde_json::to_vec(&state)
                .map_err(|e| Error::generic(format!("Failed to serialize Valence state: {}", e)))?;
            
            self.db.put_cf(cf, state_key, updated_state)?;
        }

        Ok(())
    }

    async fn store_valence_library_removal(
        &self,
        account_id: &str,
        library_address: &str,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        let state_key = self.valence_account_state_key(account_id);
        let cf = self.cf_valence_state()?;

        // Read-Modify-Write
        let current_state_bytes = self.db.get_cf(cf, &state_key)?;
        let mut state: ValenceAccountState = match current_state_bytes {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| Error::generic(format!("Failed to deserialize Valence state: {}", e)))?,
            None => return Err(Error::generic(format!("Valence account state not found for ID: {}", account_id))),
        };

        let initial_len = state.libraries.len();
        state.libraries.retain(|lib| lib != library_address);

        if state.libraries.len() < initial_len {
            state.last_update_block = _update_block;
            state.last_update_tx = _update_tx.to_string();

            let state_json = serde_json::to_vec(&state)
                .map_err(|e| Error::generic(format!("Failed to serialize Valence state: {}", e)))?;
            
            self.db.put_cf(cf, state_key, state_json)?;
        }
        Ok(())
    }

    async fn store_valence_ownership_update(
        &self,
        account_id: &str,
        new_owner: Option<String>,
        new_pending_owner: Option<String>,
        new_pending_expiry: Option<u64>,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        let state_key = self.valence_account_state_key(account_id);
        let cf = self.cf_valence_state()?;

        // Read-Modify-Write
        let current_state_bytes = self.db.get_cf(cf, &state_key)?;
        let mut state: ValenceAccountState = match current_state_bytes {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| Error::generic(format!("Failed to deserialize Valence state: {}", e)))?,
            None => return Err(Error::generic(format!("Valence account state not found for ID: {}", account_id))),
        };

        let mut changed = false;
        if state.current_owner != new_owner {
            state.current_owner = new_owner;
            changed = true;
        }
        if state.pending_owner != new_pending_owner {
            state.pending_owner = new_pending_owner;
            changed = true;
        }
        if state.pending_owner_expiry != new_pending_expiry {
             state.pending_owner_expiry = new_pending_expiry;
            changed = true;
        }

        if changed {
            state.last_update_block = _update_block;
            state.last_update_tx = _update_tx.to_string();
            let state_json = serde_json::to_vec(&state)
                .map_err(|e| Error::generic(format!("Failed to serialize Valence state: {}", e)))?;
            self.db.put_cf(cf, state_key, state_json)?;
        }
        Ok(())
    }

    async fn store_valence_execution(
        &self,
        _execution_info: ValenceAccountExecution,
    ) -> Result<()> {
        Ok(())
    }

    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>> {
        let key = self.valence_account_state_key(account_id);
        let cf = self.cf_valence_state()?;
        match self.db.get_cf(cf, key)? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)
                .map_err(|e| Error::generic(format!("Failed to deserialize Valence state: {}", e)))?)),
            None => Ok(None),
        }
    }

    async fn set_valence_account_state(&self, account_id: &str, state: &ValenceAccountState) -> Result<()> {
        let key = self.valence_account_state_key(account_id);
        let state_json = serde_json::to_vec(state)
             .map_err(|e| Error::generic(format!("Failed to serialize Valence state: {}", e)))?;
        let cf = self.cf_valence_state()?;
        self.db.put_cf(cf, key, state_json)
             .map_err(|e| Error::database(format!("RocksDB put error: {}", e)))?;
        Ok(())
    }

    async fn delete_valence_account_state(&self, account_id: &str) -> Result<()> {
        let key = self.valence_account_state_key(account_id);
        let cf = self.cf_valence_state()?;
        self.db.delete_cf(cf, key)
             .map_err(|e| Error::database(format!("RocksDB delete error: {}", e)))?;
        Ok(())
    }

    async fn set_historical_valence_account_state(&self, account_id: &str, block_number: u64, state: &ValenceAccountState) -> Result<()> {
        let key = self.historical_valence_account_state_key(account_id, block_number);
        let state_json = serde_json::to_vec(state)
             .map_err(|e| Error::generic(format!("Failed to serialize Valence state: {}", e)))?;
        let cf = self.cf_historical_valence_state()?;
        self.db.put_cf(cf, key, state_json)
             .map_err(|e| Error::database(format!("RocksDB put error: {}", e)))?;
        Ok(())
    }

    async fn get_historical_valence_account_state(&self, account_id: &str, block_number: u64) -> Result<Option<ValenceAccountState>> {
        let key = self.historical_valence_account_state_key(account_id, block_number);
        let cf = self.cf_historical_valence_state()?;
        match self.db.get_cf(cf, key)? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)
                .map_err(|e| Error::generic(format!("Failed to deserialize Valence state: {}", e)))?)),
            None => Ok(None),
        }
    }

    async fn delete_historical_valence_account_state(&self, account_id: &str, block_number: u64) -> Result<()> {
        let key = self.historical_valence_account_state_key(account_id, block_number);
        let cf = self.cf_historical_valence_state()?;
        self.db.delete_cf(cf, key)
             .map_err(|e| Error::database(format!("RocksDB delete error: {}", e)))?;
        Ok(())
    }

    async fn set_latest_historical_valence_block(&self, account_id: &str, block_number: u64) -> Result<()> {
        let key = self.latest_historical_valence_block_key(account_id);
        let cf = self.cf_latest_historical_valence_block()?;
        self.db.put_cf(cf, key, block_number.to_be_bytes())
             .map_err(|e| Error::database(format!("RocksDB put error: {}", e)))?;
        Ok(())
    }

    async fn get_latest_historical_valence_block(&self, account_id: &str) -> Result<Option<u64>> {
        let key = self.latest_historical_valence_block_key(account_id);
        let cf = self.cf_latest_historical_valence_block()?;
        match self.db.get_cf(cf, key)? {
            Some(bytes) => {
                if bytes.len() == 8 {
                    Ok(Some(u64::from_be_bytes(bytes.try_into().unwrap())))
                } else {
                    Err(Error::generic(
                        "Invalid byte length for block number".to_string(),
                    ))
                }
            }
            None => Ok(None),
        }
    }

    async fn delete_latest_historical_valence_block(&self, account_id: &str) -> Result<()> {
        let key = self.latest_historical_valence_block_key(account_id);
        let cf = self.cf_latest_historical_valence_block()?;
        self.db.delete_cf(cf, key)
             .map_err(|e| Error::database(format!("RocksDB delete error: {}", e)))?;
        Ok(())
    }

    async fn store_valence_processor_instantiation(
        &self,
        processor_info: ValenceProcessorInfo,
    ) -> Result<()> {
        // Create a basic processor state from the info
        let state = ValenceProcessorState {
            processor_id: processor_info.id.clone(),
            chain_id: processor_info.chain_id.clone(),
            address: processor_info.contract_address.clone(),
            owner: processor_info.current_owner.clone(),
            config: processor_info.config.clone(),
            pending_message_count: 0,
            completed_message_count: 0,
            failed_message_count: 0,
            last_update_block: processor_info.created_at_block,
            last_update_tx: processor_info.created_at_tx.clone(),
        };
        
        // Store the processor state
        self.set_valence_processor_state(&processor_info.id, &state).await?;
        
        // Optionally store historical state
        self.set_historical_valence_processor_state(
            &processor_info.id,
            processor_info.created_at_block,
            &state,
        ).await?;
        
        Ok(())
    }
    
    async fn store_valence_processor_config_update(
        &self,
        processor_id: &str,
        config: ValenceProcessorConfig,
        update_block: u64,
        update_tx: &str,
    ) -> Result<()> {
        // Get current state
        if let Some(mut state) = self.get_valence_processor_state(processor_id).await? {
            // Update config and metadata
            state.config = Some(config);
            state.last_update_block = update_block;
            state.last_update_tx = update_tx.to_string();
            
            // Store updated state
            self.set_valence_processor_state(processor_id, &state).await?;
            
            // Store historical state
            self.set_historical_valence_processor_state(
                processor_id,
                update_block,
                &state,
            ).await?;
        }
        
        Ok(())
    }
    
    async fn store_valence_processor_message(
        &self,
        _message: ValenceProcessorMessage,
    ) -> Result<()> {
        // Simplified implementation
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
        // Simplified implementation
        Ok(())
    }
    
    async fn get_valence_processor_state(&self, processor_id: &str) -> Result<Option<ValenceProcessorState>> {
        let key = self.valence_processor_state_key(processor_id);
        let cf = self.cf_valence_state()?;
        
        if let Some(data) = self.db.get_cf(cf, key)? {
            let state: ValenceProcessorState = serde_json::from_slice(&data)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }
    
    async fn set_valence_processor_state(&self, processor_id: &str, state: &ValenceProcessorState) -> Result<()> {
        let key = self.valence_processor_state_key(processor_id);
        let cf = self.cf_valence_state()?;
        let data = serde_json::to_vec(state)?;
        
        self.db.put_cf(cf, key, data)?;
        Ok(())
    }
    
    async fn set_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
        state: &ValenceProcessorState,
    ) -> Result<()> {
        let key = self.historical_valence_processor_state_key(processor_id, block_number);
        let cf = self.cf_historical_valence_state()?;
        let data = serde_json::to_vec(state)?;
        
        self.db.put_cf(cf, key, data)?;
        Ok(())
    }
    
    async fn get_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
    ) -> Result<Option<ValenceProcessorState>> {
        let key = self.historical_valence_processor_state_key(processor_id, block_number);
        let cf = self.cf_historical_valence_state()?;
        
        if let Some(data) = self.db.get_cf(cf, key)? {
            let state: ValenceProcessorState = serde_json::from_slice(&data)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }
    
    // --- Valence Authorization Methods ---
    
    async fn store_valence_authorization_instantiation(
        &self,
        _auth_info: ValenceAuthorizationInfo,
        _initial_policy: Option<ValenceAuthorizationPolicy>,
    ) -> Result<()> {
        // Simplified implementation
        Ok(())
    }
    
    async fn store_valence_authorization_policy(
        &self,
        _policy: ValenceAuthorizationPolicy,
    ) -> Result<()> {
        // Simplified implementation
        Ok(())
    }
    
    async fn update_active_authorization_policy(
        &self,
        _auth_id: &str,
        _policy_id: &str,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        // Simplified implementation
        Ok(())
    }
    
    async fn store_valence_authorization_grant(
        &self,
        _grant: ValenceAuthorizationGrant,
    ) -> Result<()> {
        // Simplified implementation
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
        // Simplified implementation
        Ok(())
    }
    
    async fn store_valence_authorization_request(
        &self,
        _request: ValenceAuthorizationRequest,
    ) -> Result<()> {
        // Simplified implementation
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
        // Simplified implementation
        Ok(())
    }

    // --- Valence Library Methods ---
    
    async fn store_valence_library_instantiation(
        &self,
        _library_info: ValenceLibraryInfo,
        _initial_version: Option<ValenceLibraryVersion>,
    ) -> Result<()> {
        // Simplified implementation
        Ok(())
    }
    
    async fn store_valence_library_version(
        &self,
        _version: ValenceLibraryVersion,
    ) -> Result<()> {
        // Simplified implementation
        Ok(())
    }
    
    async fn update_active_library_version(
        &self,
        _library_id: &str,
        _version: u32,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        // Simplified implementation
        Ok(())
    }
    
    async fn store_valence_library_usage(
        &self,
        _usage: ValenceLibraryUsage,
    ) -> Result<()> {
        // Simplified implementation
        Ok(())
    }
    
    async fn revoke_valence_library_approval(
        &self,
        _library_id: &str,
        _account_id: &str,
        _revoked_at_block: u64,
        _revoked_at_tx: &str,
    ) -> Result<()> {
        // Simplified implementation
        Ok(())
    }
    
    async fn get_valence_library_state(&self, _library_id: &str) -> Result<Option<ValenceLibraryState>> {
        // Simplified implementation
        Ok(None)
    }
    
    async fn set_valence_library_state(&self, _library_id: &str, _state: &ValenceLibraryState) -> Result<()> {
        // Simplified implementation
        Ok(())
    }
    
    async fn get_valence_library_versions(&self, _library_id: &str) -> Result<Vec<ValenceLibraryVersion>> {
        // Simplified implementation
        Ok(Vec::new())
    }
    
    async fn get_valence_library_approvals(&self, _library_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        // Simplified implementation
        Ok(Vec::new())
    }
    
    async fn get_valence_libraries_for_account(&self, _account_id: &str) -> Result<Vec<ValenceLibraryApproval>> {
        // Simplified implementation
        Ok(Vec::new())
    }
    
    async fn get_valence_library_usage_history(
        &self,
        _library_id: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<ValenceLibraryUsage>> {
        // Simplified implementation
        Ok(Vec::new())
    }
}

impl RocksStorage {
    /// Create a new RocksDB storage instance
    pub fn new(config: RocksConfig) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(config.create_if_missing);
        opts.create_missing_column_families(true); // Create CFs if they don't exist

        // Define expected Column Family names
        let cf_names = [
            "default",
            "events",
            "latest_block",
            "block_status",
            "valence_state",
            "historical_valence_state",
            "latest_historical_valence_block",
            // Add other CFs if needed
        ];
        let cf_opts: Vec<(&str, Options)> = cf_names.iter().skip(1) // Skip "default"
            .map(|name| (*name, Options::default()))
            .collect();

        // Set recommended options for indexing workloads
        opts.increase_parallelism(num_cpus::get() as i32);
        opts.optimize_level_style_compaction(512 * 1024 * 1024); // 512MB
        opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB

        // Configure block-based table options
        let mut block_opts = BlockBasedOptions::default();
        if config.cache_size_mb > 0 {
            // Set bloom filter and other block options
            block_opts.set_bloom_filter(10.0, false);
            block_opts.set_cache_index_and_filter_blocks(true);
        }
        opts.set_block_based_table_factory(&block_opts);

        let db = DB::open_cf_with_opts(&opts, Path::new(&config.path), cf_opts)
            .map_err(|e| Error::generic(format!("Failed to open RocksDB with CFs: {}", e)))?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    // --- Helper methods for Column Families ---
    // Returns a reference to the ColumnFamily handle.
    // The caller is responsible for using this with db methods like get_cf, put_cf.
    fn cf_handle_ref(&self, name: &str) -> Result<&ColumnFamily> {
        self.db.cf_handle(name)
            .ok_or_else(|| Error::generic(format!("Column family '{}' not found", name)))
    }

    // Convenience methods returning Result<&ColumnFamily>
    fn cf_events(&self) -> Result<&ColumnFamily> {
        self.cf_handle_ref("events")
    }

    fn cf_latest_block(&self) -> Result<&ColumnFamily> {
        self.cf_handle_ref("latest_block")
    }

    fn cf_block_status(&self) -> Result<&ColumnFamily> {
        self.cf_handle_ref("block_status")
    }

    fn cf_valence_state(&self) -> Result<&ColumnFamily> {
        self.cf_handle_ref("valence_state")
    }

    fn cf_historical_valence_state(&self) -> Result<&ColumnFamily> {
        self.cf_handle_ref("historical_valence_state")
    }

    fn cf_latest_historical_valence_block(&self) -> Result<&ColumnFamily> {
        self.cf_handle_ref("latest_historical_valence_block")
    }

    // --- Add Crate-Public Key generation helpers --- 
    pub(crate) fn valence_account_state_key(&self, account_id: &str) -> Vec<u8> {
        Key::new("valence_state", account_id).to_bytes()
    }

    pub(crate) fn historical_valence_account_state_key(&self, account_id: &str, block_number: u64) -> Vec<u8> {
        Key::new("historical_valence_state", format!("{}:{:016x}", account_id, block_number)).to_bytes()
    }

    pub(crate) fn latest_historical_valence_block_key(&self, account_id: &str) -> Vec<u8> {
        Key::new("latest_historical_valence_block", account_id).to_bytes()
    }

    pub(crate) fn valence_processor_state_key(&self, processor_id: &str) -> Vec<u8> {
        format!("processor_state:{}", processor_id).into_bytes()
    }
    
    pub(crate) fn historical_valence_processor_state_key(&self, processor_id: &str, block_number: u64) -> Vec<u8> {
        format!("historical_processor_state:{}:{}", processor_id, block_number).into_bytes()
    }

    // --- General DB Helpers --- 
    pub fn get(&self, key: &Key) -> Result<Option<Vec<u8>>> {
        let cf = self.cf_handle_ref(&key.namespace)?;
        self.db.get_cf(cf, key.to_bytes())
            .map_err(|e| Error::database(format!("RocksDB get error: {}", e)))
    }

    pub fn put(&self, key: &Key, value: &[u8]) -> Result<()> {
        let cf = self.cf_handle_ref(&key.namespace)?;
        self.db.put_cf(cf, key.to_bytes(), value)
            .map_err(|e| Error::database(format!("RocksDB put error: {}", e)))
    }

    pub fn delete(&self, key: &Key) -> Result<()> {
        let cf = self.cf_handle_ref(&key.namespace)?;
        self.db.delete_cf(cf, key.to_bytes())
            .map_err(|e| Error::database(format!("RocksDB delete error: {}", e)))
    }

    pub fn scan_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let cf_name = Key::from_bytes(prefix)?.namespace;
        let cf = self.cf_handle_ref(&cf_name)?;
        
        let mut iter = self.db.prefix_iterator_cf(cf, prefix);
        let mut results = Vec::new();
        
        while let Some(item) = iter.next() {
            match item {
                Ok((key, value)) => {
                    results.push((key.to_vec(), value.to_vec()));
                }
                Err(e) => {
                    return Err(Error::database(format!("RocksDB iterator error: {}", e)));
                }
            }
        }
        
        Ok(results)
    }

    // --- Index Query Helpers --- 
    fn get_event_ids_by_chain(&self, chain: &str) -> Result<Vec<String>> {
        let prefix = Key::prefix(format!("index:chain:{}", chain));
        let kv_pairs = self.scan_prefix(&prefix)?;
        kv_pairs.into_iter()
            .map(|(_, v)| String::from_utf8(v).map_err(|e| Error::generic(format!("Invalid UTF-8 in event ID: {}", e))))
            .collect()
    }

    fn get_event_ids_by_chain_and_block_range(&self, chain: &str, min_block: u64, max_block: u64) -> Result<Vec<String>> {
        let start_key = Key::new("index:chain_block", format!("{}:{:016x}", chain, min_block));
        let end_key = Key::new("index:chain_block", format!("{}:{:016x}", chain, max_block + 1)); 
        let cf = self.cf_handle_ref("index:chain_block")?; 

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::From(start_key.to_bytes().as_slice(), rocksdb::Direction::Forward));
        
        let mut results = Vec::new();
        for item in iter {
             let (key_bytes, value_bytes) = item.map_err(|e| Error::database(format!("RocksDB iterator error: {}", e)))?;
             if key_bytes.as_ref() >= end_key.to_bytes().as_slice() {
                 break;
             }
             let event_id = String::from_utf8(value_bytes.to_vec()).map_err(|e| Error::generic(format!("Invalid UTF-8 event ID: {}", e)))?;
             results.push(event_id);
        }
        Ok(results)
    }

    fn get_event_ids_by_chain_and_event_types(&self, chain: &str, event_types: &[String]) -> Result<Vec<String>> {
        let mut all_ids = HashSet::new(); 
        for event_type in event_types {
            let prefix = Key::prefix(format!("index:chain_type:{}:{}", chain, event_type));
            let kv_pairs = self.scan_prefix(&prefix)?;
            for (_, value_bytes) in kv_pairs {
                let event_id = String::from_utf8(value_bytes).map_err(|e| Error::generic(format!("Invalid UTF-8 event ID: {}", e)))?;
                all_ids.insert(event_id);
            }
        }
        Ok(all_ids.into_iter().collect())
    }

    fn get_event_ids_by_chain_and_time_range(&self, chain: &str, min_time: u64, max_time: u64) -> Result<Vec<String>> {
        let start_key = Key::new("index:chain_time", format!("{}:{:016x}", chain, min_time));
        let end_key = Key::new("index:chain_time", format!("{}:{:016x}", chain, max_time + 1)); 
        let cf = self.cf_handle_ref("index:chain_time")?; 

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::From(start_key.to_bytes().as_slice(), rocksdb::Direction::Forward));
        
        let mut results = Vec::new();
        for item in iter {
             let (key_bytes, value_bytes) = item.map_err(|e| Error::database(format!("RocksDB iterator error: {}", e)))?;
             if key_bytes.as_ref() >= end_key.to_bytes().as_slice() {
                 break;
             }
             let event_id = String::from_utf8(value_bytes.to_vec()).map_err(|e| Error::generic(format!("Invalid UTF-8 event ID: {}", e)))?;
             results.push(event_id);
        }
        Ok(results)
    }

    fn get_all_event_ids(&self) -> Result<Vec<String>> {
        let prefix = Key::prefix("event");
        let kv_pairs = self.scan_prefix(&prefix)?;
        kv_pairs.into_iter()
            .map(|(k, _)| Key::from_bytes(&k).map(|key| key.id))
            .collect()
    }

    // --- Event Handling Helpers --- 
    fn get_event_by_id(&self, id: &str) -> Result<Option<Box<dyn Event>>> {
        let key = Key::new("event", id);
        if let Some(bytes) = self.get(&key)? {
            let event_data: EventData = bincode::deserialize(&bytes)
                .map_err(|e| Error::generic(format!("Failed to deserialize event data: {}", e)))?;
            Ok(Some(Box::new(event_data.to_mock_event())))
        } else {
            Ok(None)
        }
    }

    fn apply_remaining_filters(&self, events: Vec<Box<dyn Event>>, filter: &EventFilter) -> Vec<Box<dyn Event>> {
        events.into_iter().filter(move |event| {
            if let Some(event_types) = &filter.event_types {
                if !event_types.is_empty() && !event_types.contains(&event.event_type().to_string()) {
                    return false;
                }
            }
            
            if let Some((min_time, max_time)) = filter.time_range {
                let event_timestamp = event.timestamp().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                if event_timestamp < min_time || event_timestamp > max_time {
                    return false;
                }
            }
            
            true
        }).collect()
    }
    
    // --- Write Batch Helpers --- 
    pub fn create_write_batch(&self) -> KeyBatch {
        KeyBatch::new()
    }

    pub fn write_batch(&self, batch: KeyBatch) -> Result<()> {
        self.db.write(batch.inner())
            .map_err(|e| Error::database(format!("RocksDB write batch error: {}", e)))
    }
}

/// Event data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventData {
    /// Event ID
    pub id: String,
    
    /// Chain ID
    pub chain: String,
    
    /// Block number
    pub block_number: u64,
    
    /// Block hash
    pub block_hash: String,
    
    /// Transaction hash
    pub tx_hash: String,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// Event type
    pub event_type: String,
    
    /// Raw event data
    pub raw_data: Vec<u8>,
}

impl EventData {
    /// Convert to a mock event implementation
    pub fn to_mock_event(&self) -> MockEvent {
        MockEvent {
            id: self.id.clone(),
            chain: self.chain.clone(),
            block_number: self.block_number,
            block_hash: self.block_hash.clone(),
            tx_hash: self.tx_hash.clone(),
            timestamp: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp),
            event_type: self.event_type.clone(),
            raw_data: self.raw_data.clone(),
        }
    }
}

/// Mock event implementation for deserialization
#[derive(Debug)]
struct MockEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    timestamp: SystemTime,
    event_type: String,
    raw_data: Vec<u8>,
}

impl Event for MockEvent {
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

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A wrapper around WriteBatch that works with our Key type
pub struct KeyBatch {
    batch: WriteBatch,
}

impl Default for KeyBatch {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyBatch {
    /// Create a new batch
    pub fn new() -> Self {
        Self {
            batch: WriteBatch::default(),
        }
    }
    
    /// Put a key-value pair
    pub fn put(&mut self, key: &Key, value: &[u8]) -> &mut Self {
        self.batch.put(key.to_bytes(), value);
        self
    }
    
    /// Delete a key
    pub fn delete(&mut self, key: &Key) -> &mut Self {
        self.batch.delete(key.to_bytes());
        self
    }
    
    /// Get the inner WriteBatch
    pub fn inner(self) -> WriteBatch {
        self.batch
    }
}
