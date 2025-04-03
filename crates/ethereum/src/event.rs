use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use ethers::types::{Block, Log, Transaction, TransactionReceipt};
use serde::{Deserialize, Serialize};

use indexer_core::event::Event;

/// Ethereum-specific event data
#[derive(Clone, Serialize, Deserialize)]
pub struct EthereumEvent {
    /// Unique identifier for the event
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

    /// Contract address where the event was emitted
    pub address: String,

    /// Topics of the event
    pub topics: Vec<String>,

    /// Data of the event
    pub data: Vec<u8>,

    /// Log index within the block
    pub log_index: u64,

    /// Transaction index within the block
    pub tx_index: u64,

    /// Raw event data
    pub raw_data: Vec<u8>,
}

impl fmt::Debug for EthereumEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EthereumEvent")
            .field("id", &self.id)
            .field("chain", &self.chain)
            .field("block_number", &self.block_number)
            .field("block_hash", &self.block_hash)
            .field("tx_hash", &self.tx_hash)
            .field("timestamp", &self.timestamp)
            .field("event_type", &self.event_type)
            .field("address", &self.address)
            .field("topics", &self.topics)
            .field("data", &format!("<{} bytes>", self.data.len()))
            .field("log_index", &self.log_index)
            .field("tx_index", &self.tx_index)
            .field("raw_data", &format!("<{} bytes>", self.raw_data.len()))
            .finish()
    }
}

impl Event for EthereumEvent {
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

impl EthereumEvent {
    /// Create a new Ethereum event from a log and block
    pub fn from_log(
        log: Log, 
        block: Block<Transaction>, 
        chain_id: String,
        receipt: Option<TransactionReceipt>
    ) -> Self {
        let timestamp = block.timestamp.as_u64();
        let block_number = block.number.unwrap_or_default().as_u64();
        let block_hash = block.hash.unwrap_or_default().to_string();
        let tx_hash = log.transaction_hash.unwrap_or_default().to_string();
        let log_index = log.log_index.unwrap_or_default().as_u64();
        let tx_index = log.transaction_index.unwrap_or_default().as_u64();
        
        // Generate a unique ID for the event
        let id = format!("{}-{}-{}", chain_id, tx_hash, log_index);
        
        // Determine event type from the first topic if available
        let event_type = if !log.topics.is_empty() {
            log.topics[0].to_string()
        } else {
            "unknown".to_string()
        };
        
        // Convert topics to strings
        let topics = log.topics.iter()
            .map(|t| t.to_string())
            .collect();
        
        // Serialize the log to get the raw data
        let raw_data = serde_json::to_vec(&log).unwrap_or_default();
        
        Self {
            id,
            chain: chain_id,
            block_number,
            block_hash,
            tx_hash,
            timestamp,
            event_type,
            address: log.address.to_string(),
            topics,
            data: log.data.to_vec(),
            log_index,
            tx_index,
            raw_data,
        }
    }
} 