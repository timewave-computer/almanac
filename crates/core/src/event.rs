use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::time::SystemTime;

/// Common trait for all events
pub trait Event: Debug + Send + Sync {
    /// Unique identifier for the event
    fn id(&self) -> &str;

    /// Chain from which the event originated
    fn chain(&self) -> &str;

    /// Block number or height at which the event occurred
    fn block_number(&self) -> u64;

    /// Hash of the block containing the event
    fn block_hash(&self) -> &str;

    /// Hash of the transaction containing the event
    fn tx_hash(&self) -> &str;

    /// Timestamp when the event occurred
    fn timestamp(&self) -> SystemTime;

    /// Type of the event
    fn event_type(&self) -> &str;

    /// Raw event data
    fn raw_data(&self) -> &[u8];
}

/// Common metadata for all events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique identifier for the event
    pub id: String,

    /// Chain from which the event originated
    pub chain: String,

    /// Block number or height at which the event occurred
    pub block_number: u64,

    /// Hash of the block containing the event
    pub block_hash: String,

    /// Hash of the transaction containing the event
    pub tx_hash: String,

    /// Timestamp when the event occurred
    pub timestamp: u64,

    /// Type of the event
    pub event_type: String,
}

/// Generic container for chain-specific events with common metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContainer<T> {
    /// Event metadata
    pub metadata: EventMetadata,

    /// Chain-specific event data
    pub data: T,

    /// Raw event data
    #[serde(with = "serde_bytes")]
    pub raw_data: Vec<u8>,
} 