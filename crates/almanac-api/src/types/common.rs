use serde::{Deserialize, Serialize};
use std::fmt;

/// Chain identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainId(String);

impl ChainId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ChainId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ChainId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Block number type
pub type BlockNumber = u64;

/// Simple event filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Chain to filter by
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<ChainId>,
    
    /// Starting block number (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_block: Option<BlockNumber>,
    
    /// Ending block number (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_block: Option<BlockNumber>,
    
    /// Event types to filter by
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub event_types: Vec<String>,
    
    /// Contract addresses to filter by
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub addresses: Vec<String>,
    
    /// Maximum number of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    
    /// Offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}

impl EventFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self {
            chain: None,
            from_block: None,
            to_block: None,
            event_types: Vec::new(),
            addresses: Vec::new(),
            limit: None,
            offset: None,
        }
    }
    
    /// Filter by chain
    pub fn chain(mut self, chain: impl Into<ChainId>) -> Self {
        self.chain = Some(chain.into());
        self
    }
    
    /// Filter by block range
    pub fn block_range(mut self, from: BlockNumber, to: BlockNumber) -> Self {
        self.from_block = Some(from);
        self.to_block = Some(to);
        self
    }
    
    /// Filter by event type
    pub fn event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_types.push(event_type.into());
        self
    }
    
    /// Filter by contract address
    pub fn address(mut self, address: impl Into<String>) -> Self {
        self.addresses.push(address.into());
        self
    }
    
    /// Set pagination
    pub fn paginate(mut self, limit: u32, offset: u32) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Sorting options
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Asc
    }
}

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health status
    pub status: HealthState,
    
    /// Individual component statuses
    pub components: Vec<ComponentHealth>,
    
    /// Additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// API key for authentication
#[derive(Debug, Clone)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Don't accidentally log API keys
impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ApiKey(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chain_id() {
        let chain = ChainId::new("ethereum");
        assert_eq!(chain.as_str(), "ethereum");
        assert_eq!(chain.to_string(), "ethereum");
    }
    
    #[test]
    fn test_event_filter() {
        let filter = EventFilter::new()
            .chain("ethereum")
            .block_range(1000, 2000)
            .event_type("Transfer")
            .address("0x123")
            .paginate(100, 0);
            
        assert_eq!(filter.chain.unwrap().as_str(), "ethereum");
        assert_eq!(filter.from_block, Some(1000));
        assert_eq!(filter.to_block, Some(2000));
        assert_eq!(filter.event_types, vec!["Transfer"]);
        assert_eq!(filter.addresses, vec!["0x123"]);
        assert_eq!(filter.limit, Some(100));
        assert_eq!(filter.offset, Some(0));
    }
    
    #[test]
    fn test_api_key_display() {
        let key = ApiKey::new("secret-key-123");
        assert_eq!(key.to_string(), "ApiKey(***)");
        assert_eq!(key.as_str(), "secret-key-123");
    }
}