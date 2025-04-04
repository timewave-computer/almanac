use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Chain identifier
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub String);

impl From<&str> for ChainId {
    fn from(s: &str) -> Self {
        ChainId(s.to_string())
    }
}

impl From<String> for ChainId {
    fn from(s: String) -> Self {
        ChainId(s)
    }
}

/// Filter for querying events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Chain ID to filter
    pub chain_id: Option<ChainId>,
    
    /// Chain string (used in simpler interfaces)
    pub chain: Option<String>,
    
    /// Block number range
    pub block_range: Option<(u64, u64)>,
    
    /// Time range in seconds since UNIX epoch
    pub time_range: Option<(u64, u64)>,
    
    /// Event types to include
    pub event_types: Option<Vec<String>>,
    
    /// Additional filters as key-value pairs
    pub custom_filters: HashMap<String, String>,
    
    /// Maximum number of events to return
    pub limit: Option<usize>,
    
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl EventFilter {
    /// Create a new empty event filter
    pub fn new() -> Self {
        Self {
            chain_id: None,
            chain: None,
            block_range: None,
            time_range: None,
            event_types: None,
            custom_filters: HashMap::new(),
            limit: None,
            offset: None,
        }
    }
}

/// Configuration for an indexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    /// Chain identifiers to index
    pub chains: Vec<ChainConfig>,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// API configuration
    pub api: ApiConfig,
}

/// Configuration for a specific chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain identifier
    pub chain_id: ChainId,
    
    /// Chain type (e.g., "ethereum", "cosmos")
    pub chain_type: String,
    
    /// RPC endpoint URLs
    pub rpc_urls: Vec<String>,
    
    /// Starting block number
    pub start_block: Option<u64>,
    
    /// Chain-specific configuration parameters
    pub params: HashMap<String, String>,
}

/// Configuration for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage type (e.g., "rocksdb", "postgres")
    pub storage_type: String,
    
    /// Path or connection string
    pub connection: String,
    
    /// Storage-specific configuration parameters
    pub params: HashMap<String, String>,
}

/// Configuration for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Host to bind to
    pub host: String,
    
    /// Port to listen on
    pub port: u16,
    
    /// Enable GraphQL API
    pub enable_graphql: bool,
    
    /// Enable REST API
    pub enable_rest: bool,
    
    /// Additional API configuration parameters
    pub params: HashMap<String, String>,
} 