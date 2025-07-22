//! Core event types and traits

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::time::SystemTime;

/// Core trait that all blockchain events must implement
pub trait Event: Send + Sync + std::fmt::Debug {
    /// Unique identifier for this event
    fn id(&self) -> &str;

    /// Chain identifier where this event occurred
    fn chain(&self) -> &str;

    /// Block number where this event was included
    fn block_number(&self) -> u64;

    /// Block hash where this event was included
    fn block_hash(&self) -> &str;

    /// Transaction hash that generated this event
    fn tx_hash(&self) -> &str;

    /// Timestamp when this event occurred
    fn timestamp(&self) -> SystemTime;

    /// Type of event (e.g., "transfer", "swap", "deposit")
    fn event_type(&self) -> &str;

    /// Raw event data as bytes
    fn raw_data(&self) -> &[u8];

    /// Get the event as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Contract address - default implementation returns empty string
    fn contract_address(&self) -> &str {
        ""
    }

    /// Event attributes - default implementation returns empty map
    fn attributes(&self) -> HashMap<String, Value> {
        HashMap::new()
    }
}

/// Event source type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSource {
    /// Event came from blockchain
    Chain,
    /// Event came from cache
    Cache,
    /// Event came from API
    Api,
    /// Event was generated internally
    Internal,
}

/// Unified event format for cross-chain events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedEvent {
    /// Unique event ID
    pub id: String,
    /// Chain identifier
    pub chain: String,
    /// Block number
    pub block_number: u64,
    /// Block hash
    pub block_hash: String,
    /// Transaction hash
    pub tx_hash: String,
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Event type identifier
    pub event_type: String,
    /// Contract address (if applicable)
    pub contract_address: Option<String>,
    /// Raw event data
    pub raw_data: Vec<u8>,
    /// Structured attributes
    pub attributes: HashMap<String, Value>,
    /// Event source
    pub source: EventSource,
}

impl Event for UnifiedEvent {
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

    fn contract_address(&self) -> &str {
        self.contract_address.as_deref().unwrap_or("")
    }

    fn attributes(&self) -> HashMap<String, Value> {
        self.attributes.clone()
    }
}
