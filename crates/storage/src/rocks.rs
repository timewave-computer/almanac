/// RocksDB storage implementation
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::any::Any;

use async_trait::async_trait;
use indexer_common::{BlockStatus, Error, Result};
use indexer_core::event::Event;
use rocksdb::{Options, DB, WriteBatch, IteratorMode, Direction, BlockBasedOptions};
use serde::{Deserialize, Serialize};
use tracing::debug;
use serde_json;

use crate::EventFilter;
use crate::Storage;
use crate::{ValenceAccountInfo, ValenceAccountLibrary, ValenceAccountExecution, ValenceAccountState};

/// Configuration for RocksDB storage
#[derive(Debug, Clone)]
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
        let mut batch = self.create_write_batch();

        let account_id_key = Key::new("va", format!("{}:{}", account_info.chain_id, account_info.contract_address));
        let owner_key = Key::new("va_owner_idx", format!("{}:{}", account_info.current_owner.as_deref().unwrap_or("_"), account_id_key.id));
        let libs_key = Key::new("va_libs", account_id_key.id.clone());

        let state = ValenceAccountState {
            current_owner: account_info.current_owner.clone(),
            libraries: initial_libraries.iter().map(|l| l.library_address.clone()).collect(),
        };
        let state_json = serde_json::to_vec(&state)?;

        // Store main account state (owner + libraries combined for simpler updates)
        batch.put(&libs_key, &state_json);

        // Add owner index
        if account_info.current_owner.is_some() {
             batch.put(&owner_key, &[1]);
        }

        // Add library indexes
        for lib in initial_libraries {
            let lib_idx_key = Key::new("va_lib_idx", format!("{}:{}", lib.library_address, account_id_key.id));
            batch.put(&lib_idx_key, &[1]);
        }

        // Persist account info (primarily for potential recovery/rebuild, state is in libs_key)
        // Consider if this is necessary if Postgres is the source of truth
        // let info_json = serde_json::to_vec(&account_info)?;
        // batch.put(&account_id_key, &info_json); // Maybe skip this if PG is source of truth

        self.write_batch(batch)?;
        Ok(())
    }

    async fn store_valence_library_approval(
        &self,
        account_id: &str, // format: "<chain_id>:<contract_address>"
        library_info: ValenceAccountLibrary,
        _update_block: u64, // Rocks only stores latest state
        _update_tx: &str,
    ) -> Result<()> {
        let libs_key = Key::new("va_libs", account_id);

        // Read-Modify-Write: Get current state, add library, write back
        // NOTE: This is not atomic across reads/writes. Assumes single-threaded indexer access for now.
        let current_state_bytes = self.get(&libs_key)?;
        let mut state: ValenceAccountState = match current_state_bytes {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => return Err(Error::NotFound(format!("Valence account state not found for ID: {}", account_id))),
        };

        let library_address = library_info.library_address;
        if !state.libraries.contains(&library_address) {
            state.libraries.push(library_address.clone());
            state.libraries.sort(); // Keep it sorted for consistency

            let state_json = serde_json::to_vec(&state)?;
            let lib_idx_key = Key::new("va_lib_idx", format!("{}:{}", library_address, account_id));

            let mut batch = self.create_write_batch();
            batch.put(&libs_key, &state_json);
            batch.put(&lib_idx_key, &[1]);
            self.write_batch(batch)?;
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
         let libs_key = Key::new("va_libs", account_id);

        // Read-Modify-Write
        let current_state_bytes = self.get(&libs_key)?;
        let mut state: ValenceAccountState = match current_state_bytes {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => return Err(Error::NotFound(format!("Valence account state not found for ID: {}", account_id))),
        };

        let initial_len = state.libraries.len();
        state.libraries.retain(|lib| lib != library_address);

        if state.libraries.len() < initial_len { // Only write if something changed
            let state_json = serde_json::to_vec(&state)?;
            let lib_idx_key = Key::new("va_lib_idx", format!("{}:{}", library_address, account_id));

            let mut batch = self.create_write_batch();
            batch.put(&libs_key, &state_json);
            batch.delete(&lib_idx_key);
            self.write_batch(batch)?;
        }
        Ok(())
    }

    async fn store_valence_ownership_update(
        &self,
        account_id: &str,
        new_owner: Option<String>,
        _new_pending_owner: Option<String>,      // Not storing pending state in RocksDB for now
        _new_pending_expiry: Option<u64>,
        _update_block: u64,
        _update_tx: &str,
    ) -> Result<()> {
        let libs_key = Key::new("va_libs", account_id);

        // Read-Modify-Write
        let current_state_bytes = self.get(&libs_key)?;
        let mut state: ValenceAccountState = match current_state_bytes {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => return Err(Error::NotFound(format!("Valence account state not found for ID: {}", account_id))),
        };

        let old_owner_opt = state.current_owner.clone();

        if state.current_owner != new_owner {
            state.current_owner = new_owner.clone();
            let state_json = serde_json::to_vec(&state)?;

            let mut batch = self.create_write_batch();
            batch.put(&libs_key, &state_json);

            // Update owner index
            if let Some(old_owner) = old_owner_opt {
                let old_owner_key = Key::new("va_owner_idx", format!("{}:{}", old_owner, account_id));
                batch.delete(&old_owner_key);
            }
            if let Some(new_owner_addr) = new_owner {
                 let new_owner_key = Key::new("va_owner_idx", format!("{}:{}", new_owner_addr, account_id));
                 batch.put(&new_owner_key, &[1]);
            }
            self.write_batch(batch)?;
        }
        Ok(())
    }

    async fn store_valence_execution(
        &self,
        _execution_info: ValenceAccountExecution, // Not storing execution history in RocksDB
    ) -> Result<()> {
        // RocksDB is for latest state, execution history goes to Postgres
        Ok(())
    }

    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>> {
        let libs_key = Key::new("va_libs", account_id);
        match self.get(&libs_key)? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            None => Ok(None),
        }
    }
}

impl RocksStorage {
    /// Create a new RocksDB storage instance
    pub fn new(config: RocksConfig) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(config.create_if_missing);
        
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
        
        let db = DB::open(&opts, Path::new(&config.path))
            .map_err(|e| Error::generic(format!("Failed to open RocksDB: {}", e)))?;
        
        Ok(Self {
            db: Arc::new(db),
        })
    }
    
    /// Get a value from storage
    pub fn get(&self, key: &Key) -> Result<Option<Vec<u8>>> {
        let result = self.db.get(key.to_bytes())
            .map_err(|e| Error::generic(format!("Failed to get from RocksDB: {}", e)))?;
        
        Ok(result)
    }
    
    /// Put a value in storage
    pub fn put(&self, key: &Key, value: &[u8]) -> Result<()> {
        self.db.put(key.to_bytes(), value)
            .map_err(|e| Error::generic(format!("Failed to put to RocksDB: {}", e)))?;
        
        Ok(())
    }
    
    /// Delete a value from storage
    pub fn delete(&self, key: &Key) -> Result<()> {
        self.db.delete(key.to_bytes())
            .map_err(|e| Error::generic(format!("Failed to delete from RocksDB: {}", e)))?;
        
        Ok(())
    }
    
    /// Get an iterator over a range of keys with a given prefix
    pub fn scan_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mode = IteratorMode::From(prefix, Direction::Forward);
        let iter = self.db.iterator(mode);
        
        // Create a vector to store the results
        let mut results = Vec::new();
        let prefix_vec = prefix.to_vec();
        
        // Process the iterator and handle results
        for item in iter {
            match item {
                Ok((key, value)) => {
                    let key_vec = key.to_vec();
                    // Stop when we reach a key that doesn't have our prefix
                    if !key_vec.starts_with(&prefix_vec) {
                        break;
                    }
                    results.push((key_vec, value.to_vec()));
                },
                Err(e) => {
                    return Err(Error::generic(format!("Failed to iterate RocksDB: {}", e)));
                }
            }
        }
        
        Ok(results)
    }
    
    /// Get event IDs by chain
    fn get_event_ids_by_chain(&self, chain: &str) -> Result<Vec<String>> {
        let prefix = Key::prefix(format!("index:chain_block:{}", chain));
        let mut event_ids = Vec::new();
        
        let scan_results = self.scan_prefix(&prefix)?;
        for (_, id_bytes) in scan_results {
            if let Ok(id) = String::from_utf8(id_bytes.to_vec()) {
                event_ids.push(id);
            }
        }
        
        Ok(event_ids)
    }
    
    /// Get event IDs by chain and block range
    fn get_event_ids_by_chain_and_block_range(&self, chain: &str, min_block: u64, max_block: u64) -> Result<Vec<String>> {
        let mut event_ids = Vec::new();
        
        for block_num in min_block..=max_block {
            let key = Key::new("index:chain_block", format!("{}:{}", chain, block_num));
            if let Some(id_bytes) = self.get(&key)? {
                if let Ok(id) = String::from_utf8(id_bytes.to_vec()) {
                    event_ids.push(id);
                }
            }
        }
        
        Ok(event_ids)
    }
    
    /// Get event IDs by chain and event types
    fn get_event_ids_by_chain_and_event_types(&self, chain: &str, event_types: &[String]) -> Result<Vec<String>> {
        let mut event_ids = Vec::new();
        
        for event_type in event_types {
            let key = Key::new("index:chain_type", format!("{}:{}", chain, event_type));
            if let Some(id_bytes) = self.get(&key)? {
                if let Ok(id) = String::from_utf8(id_bytes.to_vec()) {
                    event_ids.push(id);
                }
            }
        }
        
        Ok(event_ids)
    }
    
    /// Get event IDs by chain and time range
    fn get_event_ids_by_chain_and_time_range(&self, chain: &str, min_time: u64, max_time: u64) -> Result<Vec<String>> {
        let prefix = Key::prefix(format!("index:chain_time:{}", chain));
        let min_time_key = format!("{}:{:016x}", chain, min_time).into_bytes();
        let max_time_key = format!("{}:{:016x}", chain, max_time).into_bytes();
        
        let mut event_ids = Vec::new();
        
        let scan_results = self.scan_prefix(&prefix)?;
        for (key, id_bytes) in scan_results {
            // Check if the key is within the time range
            if key >= min_time_key && key <= max_time_key {
                if let Ok(id) = String::from_utf8(id_bytes.to_vec()) {
                    event_ids.push(id);
                }
            }
        }
        
        Ok(event_ids)
    }
    
    /// Get all event IDs (expensive operation)
    fn get_all_event_ids(&self) -> Result<Vec<String>> {
        let prefix = Key::prefix("events");
        let mut event_ids = Vec::new();
        
        let scan_results = self.scan_prefix(&prefix)?;
        for (key, _) in scan_results {
            if let Ok(key_obj) = Key::from_bytes(&key) {
                event_ids.push(key_obj.id);
            }
        }
        
        Ok(event_ids)
    }
    
    /// Get an event by its ID
    fn get_event_by_id(&self, id: &str) -> Result<Option<Box<dyn Event>>> {
        let key = Key::new("events", id);
        
        if let Some(event_bytes) = self.get(&key)? {
            if let Ok(event_data_str) = std::str::from_utf8(&event_bytes) {
                if let Ok(event_data) = serde_json::from_str::<EventData>(event_data_str) {
                    return Ok(Some(Box::new(event_data.to_mock_event())));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Apply remaining filters that weren't handled by index lookups
    fn apply_remaining_filters(&self, events: Vec<Box<dyn Event>>, filter: &EventFilter) -> Vec<Box<dyn Event>> {
        events.into_iter().filter(|event| {
            // Apply chain filter if specified
            if let Some(chain) = &filter.chain {
                if event.chain() != chain {
                    return false;
                }
            }
            
            // Apply block range filter if specified
            if let Some((min_block, max_block)) = filter.block_range {
                let block_num = event.block_number();
                if block_num < min_block || block_num > max_block {
                    return false;
                }
            }
            
            // Apply time range filter if specified
            if let Some((min_time, max_time)) = filter.time_range {
                if let Ok(event_time) = event.timestamp().duration_since(UNIX_EPOCH) {
                    let event_time_secs = event_time.as_secs();
                    if event_time_secs < min_time || event_time_secs > max_time {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            
            // Apply event type filter if specified
            if let Some(event_types) = &filter.event_types {
                if !event_types.is_empty() && !event_types.iter().any(|t| t == event.event_type()) {
                    return false;
                }
            }
            
            true
        }).collect()
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

impl RocksStorage {
    /// Create a new write batch for atomically writing multiple values
    pub fn create_write_batch(&self) -> KeyBatch {
        KeyBatch::new()
    }
    
    /// Write a batch of changes atomically
    pub fn write_batch(&self, batch: KeyBatch) -> Result<()> {
        self.db.write(batch.inner())
            .map_err(|e| Error::generic(format!("Failed to write batch to RocksDB: {}", e)))?;
        
        Ok(())
    }
}