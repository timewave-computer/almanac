use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::time::SystemTime;
use std::any::Any;

/// Core event trait that all blockchain events must implement
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
}

/// Unified event format for cross-chain events
#[derive(Debug, Clone)]
pub struct UnifiedEvent {
    pub id: String,
    pub chain: String,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_hash: String,
    pub timestamp: SystemTime,
    pub event_type: String,
    pub event_data: EventData,
    pub raw_data: Vec<u8>,
}

/// Standardized event data across different chains
#[derive(Debug, Clone)]
pub enum EventData {
    /// EVM-style events with topics and data
    Evm {
        topics: Vec<String>,
        data: String,
        address: String,
    },
    /// Cosmos-style events with attributes
    Cosmos {
        attributes: Vec<EventAttribute>,
        module: String,
    },
    /// Generic key-value event data
    Generic {
        attributes: std::collections::HashMap<String, String>,
    },
}

/// Event attribute for Cosmos-style events
#[derive(Debug, Clone)]
pub struct EventAttribute {
    pub key: String,
    pub value: String,
    pub index: bool,
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
}

/// Event mapping utilities
pub mod mapping {
    /// Normalize event type across chains
    pub fn normalize_event_type(event_type: &str, chain: &str) -> String {
        match chain {
            chain if chain.starts_with("ethereum") || chain.starts_with("polygon") || chain.starts_with("base") => {
                // EVM chains - normalize common event types
                match event_type.to_lowercase().as_str() {
                    "transfer" => "token_transfer".to_string(),
                    "approval" => "token_approval".to_string(),
                    "swap" => "token_swap".to_string(),
                    _ => event_type.to_lowercase(),
                }
            },
            chain if chain.contains("osmosis") || chain.contains("noble") || chain.contains("neutron") => {
                // Cosmos chains - normalize common event types
                match event_type {
                    "coin_received" => "token_transfer".to_string(),
                    "coin_spent" => "token_transfer".to_string(),
                    "transfer" => "token_transfer".to_string(),
                    _ => event_type.to_string(),
                }
            },
            _ => event_type.to_string(),
        }
    }
} 