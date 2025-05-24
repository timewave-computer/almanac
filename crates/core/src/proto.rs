//! Protocol buffer integration for valence-domain-clients
//! 
//! This module provides integration with valence-domain-clients protocol buffer
//! definitions for cross-chain communication and event processing.

// Note: The specific commit 766a1b593bcea9ed67b45c8c1ea9c548d0692a71 has a different structure
// than what was previously used. We'll use the available modules: common, cosmos, evm, clients

// Re-export common types and utilities
pub use valence_domain_clients::common;
pub use valence_domain_clients::cosmos;
pub use valence_domain_clients::evm;
pub use valence_domain_clients::clients;

/// Convert valence proto events to almanac events
pub trait ProtoEventAdapter {
    /// Convert a proto message to an almanac Event
    fn to_almanac_event(&self, chain_id: &str) -> Box<dyn crate::event::Event>;
}

/// Utility functions for working with protocol buffers
pub mod utils {
    use super::*;
    
    /// Check if data looks like a valid protocol buffer message
    pub fn is_proto_message(data: &[u8]) -> bool {
        // Basic heuristic - proto messages typically start with field tags
        !data.is_empty() && data[0] & 0x07 != 0
    }
    
    /// Parse chain identifier to determine client type
    pub fn parse_chain_type(chain_id: &str) -> &'static str {
        if chain_id.contains("ethereum") || chain_id.contains("base") || chain_id.contains("polygon") {
            "evm"
        } else if chain_id.contains("osmosis") || chain_id.contains("noble") || chain_id.contains("neutron") {
            "cosmos"
        } else {
            "unknown"
        }
    }
} 