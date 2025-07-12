/// RocksDB storage implementation - simplified for testing compatibility
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::any::Any;

use async_trait::async_trait;
use indexer_core::{BlockStatus, Error, Result};
use indexer_core::types::{Event, EventFilter};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn, info};

use crate::{
    Storage, ValenceAccountExecution, ValenceAccountInfo,
    ValenceAccountLibrary, ValenceAccountState, ValenceAuthorizationDecision,
    ValenceAuthorizationGrant, ValenceAuthorizationInfo, ValenceAuthorizationPolicy,
    ValenceAuthorizationRequest, ValenceLibraryApproval,
    ValenceLibraryInfo, ValenceLibraryUsage, ValenceLibraryVersion, ValenceLibraryState,
    ValenceMessageStatus, ValenceProcessorConfig, ValenceProcessorInfo,
    ValenceProcessorMessage, ValenceProcessorState,
    ValenceLibraryApprovalRequest, ValenceLibraryRemovalRequest, ValenceOwnershipUpdateRequest,
    ValenceProcessorConfigUpdateRequest, ValenceProcessorMessageStatusRequest,
    ValenceAuthorizationPolicyUpdateRequest, ValenceAuthorizationGrantRevocationRequest,
    ValenceAuthorizationRequestDecisionRequest, ValenceLibraryVersionUpdateRequest,
    ValenceLibraryApprovalRevocationRequest,
};

// Compression types for compatibility
#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    None = 0,
    Snappy = 1,
    Zlib = 2,
    Bz2 = 3,
    Lz4 = 4,
    Lz4hc = 5,
    Xpress = 6,
    Zstd = 7,
}

/// Configuration for RocksDB storage
#[derive(Debug, Clone)]
pub struct RocksConfig {
    /// Path to the database
    pub path: String,
    
    /// Whether to create if missing
    pub create_if_missing: bool,

    /// Cache size in megabytes
    pub cache_size_mb: usize,
    
    /// Write buffer size in MB
    pub write_buffer_size_mb: usize,
    
    /// Max write buffer number
    pub max_write_buffer_number: i32,
    
    /// Block size in KB
    pub block_size_kb: usize,
    
    /// Compression type
    pub compression: CompressionType,
    
    /// Whether to use column families for data organization
    pub use_column_families: bool,
}

impl Default for RocksConfig {
    fn default() -> Self {
        Self {
            path: "data/rocksdb".to_string(),
            create_if_missing: true,
            cache_size_mb: 128,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 3,
            block_size_kb: 4,
            compression: CompressionType::Snappy,
            use_column_families: true,
        }
    }
}

/// Column family names for organizing data
pub struct ColumnFamilies {
    pub events: String,
    pub blocks: String,
    pub metadata: String,
    pub valence_accounts: String,
    pub valence_processors: String,
    pub valence_libraries: String,
    pub valence_authorization: String,
    pub indexes: String,
}

impl Default for ColumnFamilies {
    fn default() -> Self {
        Self {
            events: "events".to_string(),
            blocks: "blocks".to_string(),
            metadata: "metadata".to_string(),
            valence_accounts: "valence_accounts".to_string(),
            valence_processors: "valence_processors".to_string(),
            valence_libraries: "valence_libraries".to_string(),
            valence_authorization: "valence_authorization".to_string(),
            indexes: "indexes".to_string(),
        }
    }
}

/// Memory-based storage implementation for testing (simulates RocksDB)
pub struct RocksStorage {
    /// In-memory key-value store organized by column families
    data: Arc<RwLock<HashMap<String, HashMap<String, Vec<u8>>>>>,
    
    /// Configuration
    config: RocksConfig,
    
    /// Column family names
    column_families: ColumnFamilies,
}

impl RocksStorage {
    /// Create a new memory-based storage instance (for testing)
    pub fn new(config: RocksConfig) -> Result<Self> {
        info!("Creating memory-based storage (simulating RocksDB) at: {}", config.path);
        
        let mut data = HashMap::new();
        let cf_names = ColumnFamilies::default();
        
        // Initialize column families
        if config.use_column_families {
            data.insert(cf_names.events.clone(), HashMap::new());
            data.insert(cf_names.blocks.clone(), HashMap::new());
            data.insert(cf_names.metadata.clone(), HashMap::new());
            data.insert(cf_names.valence_accounts.clone(), HashMap::new());
            data.insert(cf_names.valence_processors.clone(), HashMap::new());
            data.insert(cf_names.valence_libraries.clone(), HashMap::new());
            data.insert(cf_names.valence_authorization.clone(), HashMap::new());
            data.insert(cf_names.indexes.clone(), HashMap::new());
        } else {
            data.insert("default".to_string(), HashMap::new());
        }
        
        info!("Successfully created memory-based storage with {} column families", 
              if config.use_column_families { "enhanced" } else { "basic" });
        
        Ok(Self {
            data: Arc::new(RwLock::new(data)),
            config,
            column_families: cf_names,
        })
    }
    
    /// Generate a key for storing data with namespace
    fn make_key(&self, namespace: &str, id: &str) -> String {
        format!("{}:{}", namespace, id)
    }
    
    /// Store raw bytes under a key in specific column family
    fn put_raw_cf(&self, cf_name: &str, key: &str, value: &[u8]) -> Result<()> {
        let mut data = self.data.write().unwrap();
        let cf_name = if self.config.use_column_families { cf_name } else { "default" };
        
        let cf = data.entry(cf_name.to_string()).or_insert_with(HashMap::new);
        cf.insert(key.to_string(), value.to_vec());
        Ok(())
    }
    
    /// Get raw bytes by key from specific column family
    fn get_raw_cf(&self, cf_name: &str, key: &str) -> Result<Option<Vec<u8>>> {
        let data = self.data.read().unwrap();
        let cf_name = if self.config.use_column_families { cf_name } else { "default" };
        
        Ok(data.get(cf_name)
            .and_then(|cf| cf.get(key))
            .cloned())
    }
    
    /// Store a serializable object in specific column family with chain-aware key
    fn put_event_cf(&self, cf_name: &str, event_data: &EventData) -> Result<()> {
        // Store with both event ID key and chain-based key for efficient retrieval
        let event_key = self.make_key("events", &event_data.id);
        let chain_event_key = format!("events:{}:{}:{}", event_data.chain, event_data.block_number, event_data.id);
        
        let value = serde_json::to_vec(event_data)
            .map_err(|e| Error::database(format!("JSON serialization failed: {}", e)))?;
            
        // Store under event ID for direct lookup
        self.put_raw_cf(cf_name, &event_key, &value)?;
        
        // Store under chain+block+id for range queries
        self.put_raw_cf(cf_name, &chain_event_key, &value)?;
        
        Ok(())
    }
    
    /// Store a serializable object in specific column family
    fn put_json_cf<T: Serialize>(&self, cf_name: &str, namespace: &str, id: &str, data: &T) -> Result<()> {
        let key = self.make_key(namespace, id);
        let value = serde_json::to_vec(data)
            .map_err(|e| Error::database(format!("JSON serialization failed: {}", e)))?;
        self.put_raw_cf(cf_name, &key, &value)
    }
    
    /// Get a deserializable object from specific column family
    fn get_json_cf<T: for<'de> Deserialize<'de>>(&self, cf_name: &str, namespace: &str, id: &str) -> Result<Option<T>> {
        let key = self.make_key(namespace, id);
        match self.get_raw_cf(cf_name, &key)? {
            Some(data) => {
                let obj = serde_json::from_slice(&data)
                    .map_err(|e| Error::database(format!("JSON deserialization failed: {}", e)))?;
                Ok(Some(obj))
            }
            None => Ok(None),
        }
    }
    
    /// Scan events by chain and block range using memory scan
    fn scan_events_by_range(&self, chain: &str, from_block: u64, to_block: u64) -> Result<Vec<EventData>> {
        let mut events = Vec::new();
        
        let data = self.data.read().unwrap();
        let cf_name = if self.config.use_column_families { &self.column_families.events } else { "default" };
        
        if let Some(cf) = data.get(cf_name) {
            // Scan all keys that start with "events:chain:"
            let chain_prefix = format!("events:{}:", chain);
            
            for (key, value) in cf.iter() {
                if key.starts_with(&chain_prefix) {
                    if let Ok(event_data) = serde_json::from_slice::<EventData>(value) {
                        if event_data.block_number >= from_block && event_data.block_number <= to_block {
                            events.push(event_data);
                        }
                    }
                }
            }
        }
        
        // Sort by block number for consistent ordering
        events.sort_by_key(|e| (e.block_number, e.id.clone()));
        Ok(events)
    }
    
    /// Get all events for a specific chain using memory scan
    fn get_chain_events(&self, chain: &str) -> Result<Vec<EventData>> {
        let data = self.data.read().unwrap();
        let cf_name = if self.config.use_column_families { &self.column_families.events } else { "default" };
        
        let mut events = Vec::new();
        if let Some(cf) = data.get(cf_name) {
            let chain_prefix = format!("events:{}:", chain);
            
            for (key, value) in cf.iter() {
                if key.starts_with(&chain_prefix) {
                    if let Ok(event_data) = serde_json::from_slice::<EventData>(value) {
                        events.push(event_data);
                    }
                }
            }
        }
        
        Ok(events)
    }
    
    /// Perform maintenance operations
    pub fn maintenance(&self) -> Result<()> {
        info!("Performing memory storage maintenance (no-op for in-memory implementation)");
        info!("Memory storage maintenance completed");
        Ok(())
    }
}

#[async_trait]
impl Storage for RocksStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        let chain = event.chain();
        debug!("Storing event {} for chain {}", event.id(), chain);
        
        // Create event data for storage
        let event_data = EventData {
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
        };
        
        // Store the event
        self.put_event_cf(&self.column_families.events, &event_data)?;
        
        // Update latest block for chain
        let latest_block_key = self.make_key("latest_block", chain);
        let current_latest = match self.get_raw_cf(&self.column_families.blocks, &latest_block_key)? {
            Some(bytes) => String::from_utf8(bytes)
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0),
            None => 0,
        };
        
        if event.block_number() > current_latest {
            let block_bytes = event.block_number().to_string().into_bytes();
            self.put_raw_cf(&self.column_families.blocks, &latest_block_key, &block_bytes)?;
        }
        
        // Store block-to-events mapping for efficient querying
        let block_key = format!("{}:{}", chain, event.block_number());
        let mut event_ids: Vec<String> = self.get_json_cf(&self.column_families.indexes, "block_events", &block_key)?.unwrap_or_default();
        if !event_ids.contains(&event.id().to_string()) {
            event_ids.push(event.id().to_string());
            self.put_json_cf(&self.column_families.indexes, "block_events", &block_key, &event_ids)?;
        }
        
        debug!("Successfully stored event {} for chain {}", event.id(), chain);
        Ok(())
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        debug!("Getting events from memory storage with {} filters", filters.len());
        
        let mut all_events: Vec<Box<dyn Event>> = Vec::new();
        
        for filter in filters {
            // Extract chain and block range from filter
            let chains = if let Some(chain) = &filter.chain {
                vec![chain.clone()]
            } else if let Some(chains) = &filter.chains {
                chains.clone()
            } else {
                // If no chain specified, get all available chains
                let data = self.data.read().unwrap();
                let cf_name = if self.config.use_column_families { &self.column_families.events } else { "default" };
                
                let mut chains = std::collections::HashSet::new();
                if let Some(cf) = data.get(cf_name) {
                    for key in cf.keys() {
                        if key.starts_with("events:") && key.matches(':').count() >= 3 {
                            if let Some(chain_part) = key.split(':').nth(1) {
                                chains.insert(chain_part.to_string());
                            }
                        }
                    }
                }
                chains.into_iter().collect()
            };
            
            let (from_block, to_block) = filter.block_range.unwrap_or((0, u64::MAX));
            
            debug!("Processing filter for chains {:?}, range {}-{}", chains, from_block, to_block);
            
            // Get events for each chain
            for chain in chains {
                let events_data = self.scan_events_by_range(&chain, from_block, to_block)?;
                
                for event_data in events_data {
                    let event = Box::new(event_data.to_mock_event()) as Box<dyn Event>;
                    if filter.matches_event(event.as_ref()) {
                        all_events.push(event);
                    }
                }
            }
        }
        
        debug!("Found {} total events after filtering", all_events.len());
        Ok(all_events)
    }

    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        let key = self.make_key("latest_block", chain);
        match self.get_raw_cf(&self.column_families.blocks, &key)? {
            Some(bytes) => {
                let s = String::from_utf8(bytes)
                    .map_err(|_| Error::database("Invalid latest block format"))?;
                s.parse::<u64>()
                    .map_err(|_| Error::database("Invalid latest block format"))
            }
            None => Ok(0),
        }
    }

    async fn get_latest_block_with_status(&self, chain: &str, _status: BlockStatus) -> Result<u64> {
        // Simplified implementation - just return latest block
        // In a full implementation, we'd track block statuses separately
        self.get_latest_block(chain).await
    }

    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        let key = self.make_key("block_status", &format!("{}:{}", chain, block_number));
        self.put_raw_cf(&self.column_families.blocks, &key, status.as_str().as_bytes())
    }

    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        let mut events = Vec::new();
        
        for filter in filters {
            let chain = filter.chain.as_deref().unwrap_or("default");
            let (from_block, to_block) = filter.block_range.unwrap_or((0, u64::MAX));
            
            for block_num in from_block..=to_block {
                let key = self.make_key("block_status", &format!("{}:{}", chain, block_num));
                if let Some(status_bytes) = self.get_raw_cf(&self.column_families.blocks, &key)? {
                    let status_str = String::from_utf8(status_bytes)
                        .map_err(|_| Error::database("Invalid block status format"))?;
                    
                    if status_str == status.as_str() {
                        let block_filter = vec![EventFilter {
                            chain: Some(chain.to_string()),
                            block_range: Some((block_num, block_num)),
                            ..Default::default()
                        }];
                        let block_events = self.get_events(block_filter).await?;
                        events.extend(block_events);
                    }
                }
            }
        }
        
        Ok(events)
    }

    async fn reorg_chain(&self, _chain: &str, _from_block: u64) -> Result<()> {
        // Simplified implementation - in practice, we'd need to remove events and update state
        warn!("Chain reorganization handling is simplified in memory storage implementation");
        Ok(())
    }

    // --- Valence Account State Methods ---
    
    async fn store_valence_account_instantiation(
        &self,
        account_info: ValenceAccountInfo,
        initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()> {
        let state = ValenceAccountState {
            account_id: account_info.id.clone(),
            chain_id: account_info.chain_id.clone(),
            address: account_info.contract_address.clone(),
            current_owner: account_info.current_owner,
            pending_owner: account_info.pending_owner,
            pending_owner_expiry: account_info.pending_owner_expiry,
            libraries: initial_libraries.into_iter().map(|l| l.library_address).collect(),
            last_update_block: account_info.last_updated_block,
            last_update_tx: account_info.last_updated_tx,
        };
        
        self.set_valence_account_state(&account_info.id, &state).await?;
        self.set_historical_valence_account_state(&account_info.id, account_info.created_at_block, &state).await?;
        self.set_latest_historical_valence_block(&account_info.id, account_info.created_at_block).await
    }

    async fn store_valence_library_approval(
        &self,
        request: ValenceLibraryApprovalRequest,
    ) -> Result<()> {
        if let Some(mut state) = self.get_valence_account_state(&request.account_id).await? {
            if !state.libraries.contains(&request.library_info.library_address) {
                state.libraries.push(request.library_info.library_address);
                state.last_update_block = request.update_block;
                state.last_update_tx = request.update_tx.clone();
                self.set_valence_account_state(&request.account_id, &state).await?;
                self.set_historical_valence_account_state(&request.account_id, request.update_block, &state).await?;
            }
        } else {
            warn!("Attempted library approval for non-existent account state: {}", request.account_id);
        }
        Ok(())
    }

    async fn store_valence_library_removal(
        &self,
        request: ValenceLibraryRemovalRequest,
    ) -> Result<()> {
        if let Some(mut state) = self.get_valence_account_state(&request.account_id).await? {
            state.libraries.retain(|lib| lib != &request.library_address);
            state.last_update_block = request.update_block;
            state.last_update_tx = request.update_tx.clone();
            self.set_valence_account_state(&request.account_id, &state).await?;
            self.set_historical_valence_account_state(&request.account_id, request.update_block, &state).await?;
        }
        Ok(())
    }

    async fn store_valence_ownership_update(
        &self,
        request: ValenceOwnershipUpdateRequest,
    ) -> Result<()> {
        if let Some(mut state) = self.get_valence_account_state(&request.account_id).await? {
            state.current_owner = request.new_owner;
            state.pending_owner = request.new_pending_owner;
            state.pending_owner_expiry = request.new_pending_expiry;
            state.last_update_block = request.update_block;
            state.last_update_tx = request.update_tx.clone();
            self.set_valence_account_state(&request.account_id, &state).await?;
            self.set_historical_valence_account_state(&request.account_id, request.update_block, &state).await?;
        } else {
            warn!("Attempted ownership update for non-existent account state: {}", request.account_id);
        }
        Ok(())
    }

    async fn store_valence_execution(&self, _execution_info: ValenceAccountExecution) -> Result<()> {
        // Simplified implementation - could store in separate namespace
        Ok(())
    }

    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>> {
        self.get_json_cf(&self.column_families.valence_accounts, "valence_account_state", account_id)
    }
    
    async fn set_valence_account_state(&self, account_id: &str, state: &ValenceAccountState) -> Result<()> {
        self.put_json_cf(&self.column_families.valence_accounts, "valence_account_state", account_id, state)
    }

    async fn delete_valence_account_state(&self, account_id: &str) -> Result<()> {
        let key = self.make_key("valence_account_state", account_id);
        // Store empty data to simulate deletion
        self.put_raw_cf(&self.column_families.valence_accounts, &key, &[])
    }

    async fn set_historical_valence_account_state(
        &self,
        account_id: &str,
        block_number: u64,
        state: &ValenceAccountState,
    ) -> Result<()> {
        let key = format!("{}:{}", account_id, block_number);
        self.put_json_cf(&self.column_families.valence_accounts, "historical_valence_account_state", &key, state)
    }

    async fn get_historical_valence_account_state(
        &self,
        account_id: &str,
        block_number: u64,
    ) -> Result<Option<ValenceAccountState>> {
        let key = format!("{}:{}", account_id, block_number);
        self.get_json_cf(&self.column_families.valence_accounts, "historical_valence_account_state", &key)
    }

    async fn delete_historical_valence_account_state(
        &self,
        account_id: &str,
        block_number: u64,
    ) -> Result<()> {
        let key_str = format!("{}:{}", account_id, block_number);
        let key = self.make_key("historical_valence_account_state", &key_str);
        self.put_raw_cf(&self.column_families.valence_accounts, &key, &[]) // Simplified deletion
    }
    
    async fn set_latest_historical_valence_block(&self, account_id: &str, block_number: u64) -> Result<()> {
        self.put_json_cf(&self.column_families.valence_accounts, "latest_historical_valence_block", account_id, &block_number)
    }

    async fn get_latest_historical_valence_block(&self, account_id: &str) -> Result<Option<u64>> {
        self.get_json_cf(&self.column_families.valence_accounts, "latest_historical_valence_block", account_id)
    }

    async fn delete_latest_historical_valence_block(&self, account_id: &str) -> Result<()> {
        let key = self.make_key("latest_historical_valence_block", account_id);
        self.put_raw_cf(&self.column_families.valence_accounts, &key, &[]) // Simplified deletion
    }

    // --- Simplified implementations for other required methods ---
    
    async fn store_valence_processor_instantiation(&self, processor_info: ValenceProcessorInfo) -> Result<()> {
        let state = ValenceProcessorState {
            processor_id: processor_info.id.clone(),
            chain_id: processor_info.chain_id.clone(),
            address: processor_info.contract_address.clone(),
            owner: processor_info.current_owner,
            config: processor_info.config,
            pending_message_count: 0,
            completed_message_count: 0,
            failed_message_count: 0,
            last_update_block: processor_info.last_updated_block,
            last_update_tx: processor_info.last_updated_tx,
        };
        self.set_valence_processor_state(&processor_info.id, &state).await
    }
    
    async fn store_valence_processor_config_update(
        &self,
        request: ValenceProcessorConfigUpdateRequest,
    ) -> Result<()> {
        if let Some(mut state) = self.get_valence_processor_state(&request.processor_id).await? {
            state.config = Some(request.config);
            state.last_update_block = request.update_block;
            state.last_update_tx = request.update_tx.clone();
            self.set_valence_processor_state(&request.processor_id, &state).await?;
        }
        Ok(())
    }
    
    async fn store_valence_processor_message(&self, _message: ValenceProcessorMessage) -> Result<()> {
        Ok(()) // Simplified
    }
    
    async fn update_valence_processor_message_status(
        &self,
        _request: ValenceProcessorMessageStatusRequest,
    ) -> Result<()> {
        Ok(()) // Simplified
    }
    
    async fn get_valence_processor_state(&self, processor_id: &str) -> Result<Option<ValenceProcessorState>> {
        self.get_json_cf(&self.column_families.valence_processors, "valence_processor_state", processor_id)
    }

    async fn set_valence_processor_state(&self, processor_id: &str, state: &ValenceProcessorState) -> Result<()> {
        self.put_json_cf(&self.column_families.valence_processors, "valence_processor_state", processor_id, state)
    }

    async fn set_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
        state: &ValenceProcessorState,
    ) -> Result<()> {
        let key = format!("{}:{}", processor_id, block_number);
        self.put_json_cf(&self.column_families.valence_processors, "historical_valence_processor_state", &key, state)
    }

    async fn get_historical_valence_processor_state(
        &self,
        processor_id: &str,
        block_number: u64,
    ) -> Result<Option<ValenceProcessorState>> {
        let key = format!("{}:{}", processor_id, block_number);
        self.get_json_cf(&self.column_families.valence_processors, "historical_valence_processor_state", &key)
    }

    // Simplified placeholder implementations for other required methods
    async fn store_valence_authorization_instantiation(&self, _auth_info: ValenceAuthorizationInfo, _initial_policy: Option<ValenceAuthorizationPolicy>) -> Result<()> { Ok(()) }
    async fn store_valence_authorization_policy(&self, _policy: ValenceAuthorizationPolicy) -> Result<()> { Ok(()) }
    async fn update_active_authorization_policy(&self, _request: ValenceAuthorizationPolicyUpdateRequest) -> Result<()> { Ok(()) }
    async fn store_valence_authorization_grant(&self, _grant: ValenceAuthorizationGrant) -> Result<()> { Ok(()) }
    async fn revoke_valence_authorization_grant(&self, _request: ValenceAuthorizationGrantRevocationRequest) -> Result<()> { Ok(()) }
    async fn store_valence_authorization_request(&self, _request: ValenceAuthorizationRequest) -> Result<()> { Ok(()) }
    async fn update_valence_authorization_request_decision(&self, _request: ValenceAuthorizationRequestDecisionRequest) -> Result<()> { Ok(()) }
    async fn store_valence_library_instantiation(&self, _library_info: ValenceLibraryInfo, _initial_version: Option<ValenceLibraryVersion>) -> Result<()> { Ok(()) }
    async fn store_valence_library_version(&self, _version: ValenceLibraryVersion) -> Result<()> { Ok(()) }
    async fn update_active_library_version(&self, _request: ValenceLibraryVersionUpdateRequest) -> Result<()> { Ok(()) }
    async fn store_valence_library_usage(&self, _usage: ValenceLibraryUsage) -> Result<()> { Ok(()) }
    async fn revoke_valence_library_approval(&self, _request: ValenceLibraryApprovalRevocationRequest) -> Result<()> { Ok(()) }
    async fn get_valence_library_state(&self, _library_id: &str) -> Result<Option<ValenceLibraryState>> { Ok(None) }
    async fn set_valence_library_state(&self, _library_id: &str, _state: &ValenceLibraryState) -> Result<()> { Ok(()) }
    async fn get_valence_library_versions(&self, _library_id: &str) -> Result<Vec<ValenceLibraryVersion>> { Ok(Vec::new()) }
    async fn get_valence_library_approvals(&self, _library_id: &str) -> Result<Vec<ValenceLibraryApproval>> { Ok(Vec::new()) }
    async fn get_valence_libraries_for_account(&self, _account_id: &str) -> Result<Vec<ValenceLibraryApproval>> { Ok(Vec::new()) }
    async fn get_valence_library_usage_history(&self, _library_id: &str, _limit: Option<usize>, _offset: Option<usize>) -> Result<Vec<ValenceLibraryUsage>> { Ok(Vec::new()) }
    async fn set_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<()> {
        let key = format!("{}:{}", chain, block_number);
        self.put_json_cf(&self.column_families.metadata, "processor_state", &key, &state.to_string())
    }
    async fn get_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>> {
        let key = format!("{}:{}", chain, block_number);
        self.get_json_cf(&self.column_families.metadata, "processor_state", &key)
    }
    async fn set_historical_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<()> {
        let key = format!("{}:{}", chain, block_number);
        self.put_json_cf(&self.column_families.metadata, "historical_processor_state", &key, &state.to_string())
    }
    async fn get_historical_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>> {
        let key = format!("{}:{}", chain, block_number);
        self.get_json_cf(&self.column_families.metadata, "historical_processor_state", &key)
    }
}

/// Event data structure for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub id: String,
    pub chain: String,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_hash: String,
    pub timestamp: u64,
    pub event_type: String,
    pub raw_data: Vec<u8>,
}

impl EventData {
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

/// Mock event implementation for testing
#[derive(Debug, Clone)]
pub struct MockEvent {
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
    fn id(&self) -> &str { &self.id }
    fn chain(&self) -> &str { &self.chain }
    fn block_number(&self) -> u64 { self.block_number }
    fn block_hash(&self) -> &str { &self.block_hash }
    fn tx_hash(&self) -> &str { &self.tx_hash }
    fn timestamp(&self) -> SystemTime { self.timestamp }
    fn event_type(&self) -> &str { &self.event_type }
    fn raw_data(&self) -> &[u8] { &self.raw_data }
    fn as_any(&self) -> &dyn Any { self }
}
