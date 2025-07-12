//! Protocol buffer integration for valence-domain-clients
//! 
//! This module provides integration with valence-domain-clients protocol buffer
//! definitions for cross-chain communication and event processing.

// Conditionally export valence-domain-clients modules based on feature flags

// Re-export common types (always available when valence-domain-clients is enabled)
#[cfg(any(feature = "cosmos", feature = "ethereum"))]
pub use valence_domain_clients::common;

#[cfg(any(feature = "cosmos", feature = "ethereum"))]
pub use valence_domain_clients::clients;

// Cosmos-specific exports
#[cfg(feature = "cosmos")]
pub use valence_domain_clients::cosmos;

// Ethereum/EVM-specific exports
#[cfg(feature = "ethereum")]
pub use valence_domain_clients::evm;

/// Convert valence proto events to almanac events
#[cfg(any(feature = "cosmos", feature = "ethereum"))]
pub trait ProtoEventAdapter {
    /// Convert a proto message to an almanac Event
    fn to_almanac_event(&self, chain_id: &str) -> Box<dyn crate::types::Event>;
}

/// Utility functions for working with protocol buffers
pub mod utils {
    /// Check if data looks like a valid protocol buffer message
    pub fn is_proto_message(data: &[u8]) -> bool {
        // Basic heuristic - proto messages typically start with field tags
        !data.is_empty() && data[0] & 0x07 != 0
    }
    
    /// Parse chain identifier to determine client type
    pub fn parse_chain_type(chain_id: &str) -> &'static str {
        #[cfg(feature = "ethereum")]
        if chain_id.contains("ethereum") || chain_id.contains("base") || chain_id.contains("polygon") {
            return "evm";
        }
        
        #[cfg(feature = "cosmos")]
        if chain_id.contains("osmosis") || chain_id.contains("noble") || chain_id.contains("neutron") {
            return "cosmos";
        }
        
        "unknown"
    }
} 