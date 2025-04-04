/// Cosmos blockchain event implementation
use std::time::SystemTime;

use indexer_core::event::Event;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Cosmos blockchain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosEvent {
    /// Unique event identifier
    pub id: String,
    
    /// Chain identifier
    pub chain: String,
    
    /// Block number/height
    pub block_number: u64,
    
    /// Block hash
    pub block_hash: String,
    
    /// Transaction hash
    pub tx_hash: String,
    
    /// Timestamp
    pub timestamp: SystemTime,
    
    /// Event type
    pub event_type: String,
    
    /// Raw event data
    pub raw_data: Vec<u8>,
}

impl CosmosEvent {
    /// Create a new Cosmos event
    pub fn new(
        chain: &str,
        block_number: u64,
        block_hash: &str,
        tx_hash: &str,
        timestamp: SystemTime,
        event_type: &str,
        raw_data: &[u8],
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            chain: chain.to_string(),
            block_number,
            block_hash: block_hash.to_string(),
            tx_hash: tx_hash.to_string(),
            timestamp,
            event_type: event_type.to_string(),
            raw_data: raw_data.to_vec(),
        }
    }
    
    /// Create a mock event for testing
    pub fn new_mock() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            chain: "cosmos".to_string(),
            block_number: 12345,
            block_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            tx_hash: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            timestamp: SystemTime::now(),
            event_type: "transfer".to_string(),
            raw_data: r#"{"from":"cosmos1...","to":"cosmos1...","amount":"1000000"}"#.as_bytes().to_vec(),
        }
    }
}

impl Event for CosmosEvent {
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
} 