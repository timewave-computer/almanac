/// Shared chain-specific logic and abstractions
///
/// This module provides common functionality and abstractions
/// for different blockchain integrations.
use almanac_infrastructure_core::{Result, ChainId, BlockNumber};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Common chain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainInfo {
    pub id: ChainId,
    pub name: String,
    pub chain_type: ChainType,
    pub native_token: String,
    pub block_time: u64, // Average block time in milliseconds
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ChainType {
    Ethereum,
    Cosmos,
    Other(u32),
}

/// Common interface for chain clients
#[async_trait]
pub trait ChainClient: Send + Sync {
    /// Get current block number
    async fn get_current_block(&self) -> Result<BlockNumber>;
    
    /// Get chain information
    async fn get_chain_info(&self) -> Result<ChainInfo>;
    
    /// Check if chain is synced
    async fn is_synced(&self) -> Result<bool>;
    
    /// Get estimated finality time in blocks
    fn finality_blocks(&self) -> u64;
}

/// Common chain utilities
pub mod utils {
    use super::*;
    
    /// Calculate finality time based on block time and finality blocks
    pub fn calculate_finality_time(chain_info: &ChainInfo, finality_blocks: u64) -> u64 {
        chain_info.block_time * finality_blocks
    }
    
    /// Parse chain ID from string
    pub fn parse_chain_id(s: &str) -> Result<ChainId> {
        Ok(ChainId(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chain_type() {
        assert_eq!(ChainType::Ethereum, ChainType::Ethereum);
        assert_ne!(ChainType::Cosmos, ChainType::Ethereum);
    }
    
    #[test]
    fn test_finality_calculation() {
        let chain_info = ChainInfo {
            id: ChainId("test".to_string()),
            name: "Test Chain".to_string(),
            chain_type: ChainType::Other(1),
            native_token: "TEST".to_string(),
            block_time: 1000, // 1 second
        };
        
        assert_eq!(utils::calculate_finality_time(&chain_info, 10), 10000);
    }
}