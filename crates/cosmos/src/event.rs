use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use indexer_core::event::Event;

/// Cosmos-specific event data
#[derive(Clone, Serialize, Deserialize)]
pub struct CosmosEvent {
    /// Unique identifier for the event
    pub id: String,

    /// Chain ID
    pub chain: String,

    /// Block height
    pub block_number: u64,

    /// Block hash
    pub block_hash: String,

    /// Transaction hash
    pub tx_hash: String,

    /// Timestamp
    pub timestamp: u64,

    /// Event type
    pub event_type: String,

    /// Event attributes as key-value pairs
    pub attributes: Vec<(String, String)>,

    /// Transaction index within the block
    pub tx_index: u64,

    /// Raw event data
    pub raw_data: Vec<u8>,
}

impl fmt::Debug for CosmosEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CosmosEvent")
            .field("id", &self.id)
            .field("chain", &self.chain)
            .field("block_number", &self.block_number)
            .field("block_hash", &self.block_hash)
            .field("tx_hash", &self.tx_hash)
            .field("timestamp", &self.timestamp)
            .field("event_type", &self.event_type)
            .field("attributes", &self.attributes)
            .field("tx_index", &self.tx_index)
            .field("raw_data", &format!("<{} bytes>", self.raw_data.len()))
            .finish()
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
        UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp)
    }

    fn event_type(&self) -> &str {
        &self.event_type
    }

    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }
}

impl CosmosEvent {
    /// Create a new Cosmos event from the raw data
    pub fn new(
        chain_id: String,
        block_height: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: u64,
        event_type: String,
        attributes: Vec<(String, String)>,
        tx_index: u64,
        raw_data: Vec<u8>,
    ) -> Self {
        // Generate a unique ID for the event
        let id = format!("{}-{}-{}", chain_id, tx_hash, event_type);
        
        Self {
            id,
            chain: chain_id,
            block_number: block_height,
            block_hash,
            tx_hash,
            timestamp,
            event_type,
            attributes,
            tx_index,
            raw_data,
        }
    }
} 