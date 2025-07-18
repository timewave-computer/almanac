//! Event filtering types and utilities

use serde::{Serialize, Deserialize};

/// Event filter for querying events
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilter {
    /// Chain ID to filter
    pub chain: Option<String>,
    
    /// Block number range (inclusive)
    pub block_range: Option<(u64, u64)>,
    
    /// Time range in seconds since UNIX epoch
    pub time_range: Option<(u64, u64)>,
    
    /// Event types to include
    pub event_types: Option<Vec<String>>,
    
    /// Contract addresses to include
    pub addresses: Option<Vec<String>>,
    
    /// Transaction hashes to include
    pub tx_hashes: Option<Vec<String>>,
    
    /// Maximum number of events to return
    pub limit: Option<usize>,
    
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl EventFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set chain filter
    pub fn with_chain(mut self, chain: impl Into<String>) -> Self {
        self.chain = Some(chain.into());
        self
    }
    
    /// Set block range filter
    pub fn with_block_range(mut self, from: u64, to: u64) -> Self {
        self.block_range = Some((from, to));
        self
    }
    
    /// Set time range filter
    pub fn with_time_range(mut self, from: u64, to: u64) -> Self {
        self.time_range = Some((from, to));
        self
    }
    
    /// Add event type filter
    pub fn with_event_types(mut self, types: Vec<String>) -> Self {
        self.event_types = Some(types);
        self
    }
    
    /// Add address filter
    pub fn with_addresses(mut self, addresses: Vec<String>) -> Self {
        self.addresses = Some(addresses);
        self
    }
    
    /// Add transaction hash filter
    pub fn with_tx_hashes(mut self, hashes: Vec<String>) -> Self {
        self.tx_hashes = Some(hashes);
        self
    }
    
    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// Set offset for pagination
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
    
    /// Check if filter matches a chain
    pub fn matches_chain(&self, chain: &str) -> bool {
        self.chain.as_ref().is_none_or(|c| c == chain)
    }
    
    /// Check if filter matches a block number
    pub fn matches_block(&self, block: u64) -> bool {
        self.block_range.is_none_or(|(from, to)| block >= from && block <= to)
    }
    
    /// Check if filter matches an event type
    pub fn matches_event_type(&self, event_type: &str) -> bool {
        self.event_types.as_ref().is_none_or(|types| types.iter().any(|t| t == event_type))
    }
    
    /// Check if filter matches an address
    pub fn matches_address(&self, address: &str) -> bool {
        self.addresses.as_ref().is_none_or(|addrs| addrs.iter().any(|a| a == address))
    }
    
    /// Check if filter matches a transaction hash
    pub fn matches_tx_hash(&self, tx_hash: &str) -> bool {
        self.tx_hashes.as_ref().is_none_or(|hashes| hashes.iter().any(|h| h == tx_hash))
    }
    
    // Aliases for the handler methods
    pub fn chain(self, chain: String) -> Self {
        self.with_chain(chain)
    }
    
    pub fn block_range(self, from: u64, to: u64) -> Self {
        self.with_block_range(from, to)
    }
    
    pub fn event_type(self, event_type: String) -> Self {
        self.with_event_types(vec![event_type])
    }
    
    pub fn address(self, address: String) -> Self {
        self.with_addresses(vec![address])
    }
    
    pub fn paginate(self, limit: Option<usize>, offset: Option<usize>) -> Self {
        let mut filter = self;
        if let Some(l) = limit {
            filter = filter.with_limit(l);
        }
        if let Some(o) = offset {
            filter = filter.with_offset(o);
        }
        filter
    }
}

// Make EventFilter iterable for the handlers
impl IntoIterator for EventFilter {
    type Item = EventFilter;
    type IntoIter = std::iter::Once<EventFilter>;
    
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}