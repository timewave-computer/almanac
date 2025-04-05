/// Cosmos blockchain event implementation
use std::time::{SystemTime, UNIX_EPOCH};
use std::any::Any;

use indexer_core::event::Event;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use indexer_common::Result;
use std::collections::HashMap;
use std::fmt;

use cosmrs::tendermint::abci::Event as AbciEvent;

/// Cosmos event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosEvent {
    /// Unique identifier
    id: String,
    
    /// Chain identifier
    chain_id: String,
    
    /// Block number
    block_number: u64,
    
    /// Block hash
    block_hash: String,
    
    /// Transaction hash
    tx_hash: String,
    
    /// Timestamp of the block as unix timestamp
    timestamp: SystemTime,
    
    /// Event type
    event_type: String,
    
    /// Event data attributes
    data: HashMap<String, String>,
    
    /// Raw event data
    raw_data: Vec<u8>,
}

/// Cosmos event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CosmosEventType {
    /// Transaction event
    Tx,
    /// Block begin event
    BeginBlock,
    /// Block end event
    EndBlock,
    /// Custom event type
    Custom(String),
}

impl fmt::Display for CosmosEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CosmosEventType::Tx => write!(f, "tx"),
            CosmosEventType::BeginBlock => write!(f, "begin_block"),
            CosmosEventType::EndBlock => write!(f, "end_block"),
            CosmosEventType::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl CosmosEvent {
    /// Create a new Cosmos event
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        chain_id: String,
        block_number: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: u64,
        event_type: String,
        data: HashMap<String, String>,
    ) -> Self {
        // Convert timestamp to SystemTime
        let system_time = if timestamp == 0 {
            SystemTime::now()
        } else {
            UNIX_EPOCH + std::time::Duration::from_secs(timestamp)
        };

        // Serialize data to JSON for raw_data
        let raw_data = serde_json::to_vec(&data).unwrap_or_default();
        
        Self {
            id,
            chain_id,
            block_number,
            block_hash,
            tx_hash,
            timestamp: system_time,
            event_type,
            data,
            raw_data,
        }
    }
    
    /// Create a new event from ABCI event
    pub fn from_abci_event(
        chain_id: String,
        block_number: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: u64,
        abci_event: &AbciEvent,
        event_source: CosmosEventType,
    ) -> Result<Self> {
        // Parse attributes
        let mut data = HashMap::new();
        for attr in &abci_event.attributes {
            // Convert Vec<u8> to strings
            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
            let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
            data.insert(key, value);
        }
        
        // Format event type - include source
        let event_type = format!("{}_{}", event_source, abci_event.kind);
        
        // Create a raw data representation
        let raw_data = serde_json::to_vec(&data).unwrap_or_default();
        
        // Convert timestamp to SystemTime
        let system_time = if timestamp == 0 {
            SystemTime::now()
        } else {
            UNIX_EPOCH + std::time::Duration::from_secs(timestamp)
        };
        
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            chain_id,
            block_number,
            block_hash,
            tx_hash,
            timestamp: system_time,
            event_type,
            data,
            raw_data,
        })
    }
    
    /// Create a mock event for testing
    pub fn new_mock(
        chain_id: String,
        block_number: u64,
        event_type: String,
    ) -> Self {
        // Current timestamp as SystemTime
        let timestamp = SystemTime::now();
        let data = HashMap::new();
        let raw_data = Vec::new();
            
        Self {
            id: Uuid::new_v4().to_string(),
            chain_id,
            block_number,
            block_hash: format!("block_hash_{}", block_number),
            tx_hash: format!("tx_hash_{}", block_number),
            timestamp,
            event_type,
            data,
            raw_data,
        }
    }
    
    /// Get attribute value by key
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
    
    /// Check if this event matches a specific event type
    pub fn is_event_type(&self, event_type: &str) -> bool {
        self.event_type == event_type
    }
    
    /// Check if this event has a specific attribute
    pub fn has_attribute(&self, key: &str, value: &str) -> bool {
        match self.data.get(key) {
            Some(attr_value) => attr_value == value,
            None => false,
        }
    }
    
    /// Get event data
    pub fn data(&self) -> &HashMap<String, String> {
        &self.data
    }
}

impl Event for CosmosEvent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn chain(&self) -> &str {
        &self.chain_id
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

// Placeholder for the function moved from contracts/valence_account.rs
// TODO: Implement the actual logic for processing Valence events on Cosmos
pub async fn process_valence_account_event(
    storage: &BoxedStorage,
    chain_id: &str,
    event: &CosmosEvent, // Assuming it takes a CosmosEvent reference
    tx_hash: &str
) -> Result<()> {
    warn!(
        chain_id = chain_id,
        tx_hash = tx_hash,
        event_type = event.event_type,
        account_id = ?event.account_id,
        "process_valence_account_event called (placeholder implementation)"
    );
    // Based on event.event_type and event.attributes, call appropriate storage methods
    // e.g., storage.store_valence_account_instantiation(...)
    //       storage.store_valence_library_approval(...)
    //       etc.
    Ok(())
} 