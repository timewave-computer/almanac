// Mock types for e2e tests until actual implementations are available

use almanac_core::{
    Event, UnifiedEvent, EventFilter, EventSource,
    Storage, Result as CoreResult,
    ChainId, BlockStatus,
};
use serde_json::{Value as JsonValue, json};
use std::collections::HashMap;
use async_trait::async_trait;

// Define missing types for tests
pub type StorageError = Box<dyn std::error::Error + Send + Sync>;
pub type Address = String;
pub type Hash = String;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct U256(pub u64);

impl U256 {
    pub fn zero() -> Self {
        U256(0)
    }
    
    pub fn from(val: u64) -> Self {
        U256(val)
    }
    
    pub fn max_value() -> Self {
        U256(u64::MAX)
    }
    
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
    
    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }
}

impl std::ops::Add for U256 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        U256(self.0.saturating_add(other.0))
    }
}

impl std::ops::Sub for U256 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        U256(self.0.saturating_sub(other.0))
    }
}

impl From<Vec<u8>> for U256 {
    fn from(bytes: Vec<u8>) -> Self {
        // Simple conversion for testing
        U256(bytes.get(0).copied().unwrap_or(0) as u64)
    }
}

#[derive(Clone)]
pub struct FdbConfig {
    pub cluster_file: String,
    pub namespace: String,
    pub api_version: u32,
    pub transaction_timeout_ms: u32,
    pub retry_limit: u32,
}

pub struct FdbStorage {
    config: FdbConfig,
    // Mock storage
    events: tokio::sync::RwLock<Vec<UnifiedEvent>>,
    custom_data: tokio::sync::RwLock<HashMap<String, JsonValue>>,
}

impl FdbStorage {
    pub async fn new(config: FdbConfig) -> Result<Self, StorageError> {
        Ok(Self {
            config,
            events: tokio::sync::RwLock::new(Vec::new()),
            custom_data: tokio::sync::RwLock::new(HashMap::new()),
        })
    }
    
    pub fn get_config(&self) -> &FdbConfig {
        &self.config
    }
    
    pub async fn clear_namespace(&self) -> Result<(), StorageError> {
        self.events.write().await.clear();
        self.custom_data.write().await.clear();
        Ok(())
    }
    
    // Mock implementations for test methods
    pub async fn store_event(&self, event: &UnifiedEvent) -> Result<(), StorageError> {
        self.events.write().await.push(event.clone());
        Ok(())
    }
    
    pub async fn get_events(&self, filter: &EventFilter) -> Result<Vec<UnifiedEvent>, StorageError> {
        let events = self.events.read().await;
        let mut result = Vec::new();
        
        for event in events.iter() {
            if filter.matches_chain(&event.chain) &&
               filter.matches_block(event.block_number) &&
               filter.matches_event_type(&event.event_type) &&
               filter.matches_address(&event.contract_address.as_deref().unwrap_or("")) &&
               filter.matches_tx_hash(&event.tx_hash) {
                result.push(event.clone());
            }
        }
        
        // Apply pagination
        if let Some(offset) = filter.offset {
            result = result.into_iter().skip(offset).collect();
        }
        if let Some(limit) = filter.limit {
            result.truncate(limit);
        }
        
        Ok(result)
    }
    
    pub async fn store_events_batch(&self, events: &[UnifiedEvent]) -> Result<(), StorageError> {
        let mut storage = self.events.write().await;
        for event in events {
            storage.push(event.clone());
        }
        Ok(())
    }
    
    pub async fn delete_events(&self, filter: &EventFilter) -> Result<(), StorageError> {
        let to_delete = self.get_events(filter).await?;
        let to_delete_ids: Vec<String> = to_delete.iter().map(|e| e.id.clone()).collect();
        let mut events = self.events.write().await;
        events.retain(|e| !to_delete_ids.contains(&e.id));
        Ok(())
    }
    
    pub async fn store_custom_data(
        &self,
        namespace: &str,
        key: &str,
        data: &JsonValue,
    ) -> Result<(), StorageError> {
        let full_key = format!("{}:{}", namespace, key);
        self.custom_data.write().await.insert(full_key, data.clone());
        Ok(())
    }
    
    pub async fn get_custom_data(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<JsonValue>, StorageError> {
        let full_key = format!("{}:{}", namespace, key);
        Ok(self.custom_data.read().await.get(&full_key).cloned())
    }
}

// Mock chain adapter types
pub struct EthereumConfig {
    pub chain_id: ChainId,
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub confirmations: u64,
    pub batch_size: u64,
    pub max_retries: u32,
    pub timeout_secs: u64,
}

pub struct EthereumAdapter;

impl EthereumAdapter {
    pub async fn new(_config: EthereumConfig, _storage: std::sync::Arc<FdbStorage>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self)
    }
    
    pub async fn start_indexing(&self, _start_block: u64) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    pub async fn index_range(&self, _start: u64, _end: u64) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    pub async fn handle_reorg(&self, _block: u64) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

pub struct CosmosConfig {
    pub chain_id: ChainId,
    pub rpc_url: String,
    pub grpc_url: Option<String>,
    pub confirmations: u64,
    pub batch_size: u64,
    pub max_retries: u32,
    pub timeout_secs: u64,
}

pub struct CosmosAdapter;

impl CosmosAdapter {
    pub async fn new(_config: CosmosConfig, _storage: std::sync::Arc<FdbStorage>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self)
    }
    
    pub async fn index_range(&self, _start: u64, _end: u64) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// Mock Anvil for Ethereum tests
pub struct AnvilInstance;

impl AnvilInstance {
    pub fn endpoint(&self) -> String {
        "http://localhost:8545".to_string()
    }
    
    pub fn ws_endpoint(&self) -> String {
        "ws://localhost:8546".to_string()
    }
}

pub struct Anvil;

impl Anvil {
    pub fn new() -> Self {
        Self
    }
    
    pub fn port(self, _port: u16) -> Self {
        self
    }
    
    pub fn chain_id(self, _id: u32) -> Self {
        self
    }
    
    pub fn block_time(self, _time: u64) -> Self {
        self
    }
    
    pub fn spawn(self) -> AnvilInstance {
        AnvilInstance
    }
}

// Extension methods for storage
#[async_trait]
impl Storage for FdbStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> CoreResult<()> {
        // Convert to UnifiedEvent for storage
        let unified = UnifiedEvent {
            id: event.id().to_string(),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp(),
            event_type: event.event_type().to_string(),
            contract_address: Some(event.contract_address().to_string()).filter(|s| !s.is_empty()),
            raw_data: event.raw_data().to_vec(),
            attributes: event.attributes(),
            source: EventSource::Chain,
        };
        self.events.write().await.push(unified);
        Ok(())
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> CoreResult<Vec<Box<dyn Event>>> {
        let events = self.events.read().await;
        let mut result: Vec<Box<dyn Event>> = Vec::new();
        
        for event in events.iter() {
            let mut matches = true;
            for filter in &filters {
                if !filter.matches_chain(&event.chain) ||
                   !filter.matches_block(event.block_number) ||
                   !filter.matches_event_type(&event.event_type) ||
                   !filter.matches_address(&event.contract_address.as_deref().unwrap_or("")) ||
                   !filter.matches_tx_hash(&event.tx_hash) {
                    matches = false;
                    break;
                }
            }
            if matches {
                result.push(Box::new(event.clone()) as Box<dyn Event>);
            }
        }
        
        // Apply limit and offset from first filter if present
        if let Some(filter) = filters.first() {
            if let Some(offset) = filter.offset {
                result = result.into_iter().skip(offset).collect();
            }
            if let Some(limit) = filter.limit {
                result.truncate(limit);
            }
        }
        
        Ok(result)
    }
    
    async fn get_latest_block(&self, chain: &str) -> CoreResult<u64> {
        let events = self.events.read().await;
        Ok(events.iter()
            .filter(|e| e.chain == chain)
            .map(|e| e.block_number)
            .max()
            .unwrap_or(0))
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, _status: BlockStatus) -> CoreResult<u64> {
        // For mock, just return latest block
        self.get_latest_block(chain).await
    }
    
    async fn update_block_status(&self, _chain: &str, _block_number: u64, _status: BlockStatus) -> CoreResult<()> {
        Ok(())
    }
    
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, _status: BlockStatus) -> CoreResult<Vec<Box<dyn Event>>> {
        // For mock, just return filtered events
        let events = self.events.read().await;
        let mut result: Vec<Box<dyn Event>> = Vec::new();
        
        for event in events.iter() {
            let mut matches = true;
            for filter in &filters {
                if !filter.matches_chain(&event.chain) ||
                   !filter.matches_block(event.block_number) ||
                   !filter.matches_event_type(&event.event_type) ||
                   !filter.matches_address(&event.contract_address.as_deref().unwrap_or("")) ||
                   !filter.matches_tx_hash(&event.tx_hash) {
                    matches = false;
                    break;
                }
            }
            if matches {
                result.push(Box::new(event.clone()) as Box<dyn Event>);
            }
        }
        
        Ok(result)
    }
    
    async fn reorg_chain(&self, chain: &str, from_block: u64) -> CoreResult<()> {
        let mut events = self.events.write().await;
        events.retain(|e| !(e.chain == chain && e.block_number >= from_block));
        Ok(())
    }
}

// Additional mock methods needed by tests
impl FdbStorage {
    pub async fn store_contract_state(
        &self,
        _chain_id: &ChainId,
        _contract: &Address,
        _key: &U256,
        _value: &U256,
        _block: u64,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    
    pub async fn get_contract_state(
        &self,
        _chain_id: &ChainId,
        _contract: &Address,
        _key: &U256,
        _block: Option<u64>,
    ) -> Result<Option<U256>, StorageError> {
        Ok(Some(U256::zero()))
    }
    
    pub async fn get_contract_state_range(
        &self,
        _chain_id: &ChainId,
        _contract: &Address,
        _from: &U256,
        _to: &U256,
        _block: Option<u64>,
    ) -> Result<Vec<(U256, U256)>, StorageError> {
        Ok(vec![])
    }
    
    pub async fn store_block_metadata(
        &self,
        _chain_id: &ChainId,
        _block: u64,
        _data: &JsonValue,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    
    pub async fn get_block_metadata(
        &self,
        _chain_id: &ChainId,
        _block: u64,
    ) -> Result<Option<JsonValue>, StorageError> {
        Ok(Some(json!({})))
    }
    
    pub async fn get_blocks_range(
        &self,
        _chain_id: &ChainId,
        _from: u64,
        _to: u64,
    ) -> Result<Vec<JsonValue>, StorageError> {
        Ok(vec![])
    }
    
    pub async fn update_chain_height(
        &self,
        _chain_id: &ChainId,
        _height: u64,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    
    pub async fn get_all_chain_heights(&self) -> Result<HashMap<ChainId, u64>, StorageError> {
        Ok(HashMap::new())
    }
    
    pub async fn transaction<F, Fut, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, StorageError>>,
    {
        f().await
    }
}