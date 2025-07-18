//! Finality tracking and confirmation for blockchain events

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::adapters::types::FinalityStatus;

/// Finality level for blockchain blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FinalityLevel {
    /// Block is confirmed (basic inclusion)
    Confirmed = 0,
    /// Block is safe (unlikely to be reorganized)
    Safe = 1,
    /// Block is justified (voted by validators)
    Justified = 2,
    /// Block is finalized (irreversible)
    Finalized = 3,
}

impl FinalityLevel {
    /// Get all finality levels up to and including this one
    pub fn all_up_to(&self) -> Vec<FinalityLevel> {
        match self {
            FinalityLevel::Confirmed => vec![FinalityLevel::Confirmed],
            FinalityLevel::Safe => vec![FinalityLevel::Confirmed, FinalityLevel::Safe],
            FinalityLevel::Justified => vec![FinalityLevel::Confirmed, FinalityLevel::Safe, FinalityLevel::Justified],
            FinalityLevel::Finalized => vec![FinalityLevel::Confirmed, FinalityLevel::Safe, FinalityLevel::Justified, FinalityLevel::Finalized],
        }
    }
}

/// Chain-specific finality configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityConfig {
    /// Chain identifier
    pub chain_id: String,
    /// Block confirmation requirements for each finality level
    pub finality_requirements: HashMap<FinalityLevel, u64>,
    /// Time-based finality requirements (in seconds)
    pub time_requirements: HashMap<FinalityLevel, Duration>,
    /// Maximum depth to track for reorg detection
    pub max_reorg_depth: u64,
    /// Enable probabilistic finality (for PoW chains)
    pub probabilistic_finality: bool,
    /// Reorg detection enabled
    pub reorg_detection: bool,
    /// Auto-finality based on time
    pub auto_finality_timeout: Duration,
}

impl Default for FinalityConfig {
    fn default() -> Self {
        Self::ethereum_mainnet()
    }
}

impl FinalityConfig {
    /// Configuration for Ethereum mainnet
    pub fn ethereum_mainnet() -> Self {
        let mut finality_requirements = HashMap::new();
        finality_requirements.insert(FinalityLevel::Confirmed, 1);
        finality_requirements.insert(FinalityLevel::Safe, 6);
        finality_requirements.insert(FinalityLevel::Justified, 32);
        finality_requirements.insert(FinalityLevel::Finalized, 64);
        
        let mut time_requirements = HashMap::new();
        time_requirements.insert(FinalityLevel::Confirmed, Duration::from_secs(12));
        time_requirements.insert(FinalityLevel::Safe, Duration::from_secs(72));
        time_requirements.insert(FinalityLevel::Justified, Duration::from_secs(384));
        time_requirements.insert(FinalityLevel::Finalized, Duration::from_secs(768));
        
        Self {
            chain_id: "ethereum".to_string(),
            finality_requirements,
            time_requirements,
            max_reorg_depth: 100,
            probabilistic_finality: true,
            reorg_detection: true,
            auto_finality_timeout: Duration::from_secs(3600),
        }
    }
    
    /// Configuration for Cosmos chains
    pub fn cosmos(chain_id: &str) -> Self {
        let mut finality_requirements = HashMap::new();
        finality_requirements.insert(FinalityLevel::Confirmed, 1);
        finality_requirements.insert(FinalityLevel::Finalized, 2);
        
        let mut time_requirements = HashMap::new();
        time_requirements.insert(FinalityLevel::Confirmed, Duration::from_secs(6));
        time_requirements.insert(FinalityLevel::Finalized, Duration::from_secs(12));
        
        Self {
            chain_id: chain_id.to_string(),
            finality_requirements,
            time_requirements,
            max_reorg_depth: 10,
            probabilistic_finality: false,
            reorg_detection: true,
            auto_finality_timeout: Duration::from_secs(300),
        }
    }
}

/// Trait for finality processing
#[async_trait]
pub trait FinalityProcessor: Send + Sync {
    /// Process a new block
    async fn process_block(&self, chain: &str, block_number: u64, block_time: SystemTime) -> Result<(), String>;
    
    /// Get finality level for a block
    async fn get_finality_level(&self, chain: &str, block_number: u64) -> Result<FinalityLevel, String>;
    
    /// Get all blocks at a specific finality level
    async fn get_blocks_at_level(&self, chain: &str, level: FinalityLevel) -> Result<Vec<u64>, String>;
    
    /// Check if a block has reached a specific finality level
    async fn has_reached_finality(&self, chain: &str, block_number: u64, level: FinalityLevel) -> Result<bool, String>;
    
    /// Get finality statistics
    async fn get_stats(&self, chain: &str) -> Result<FinalityStats, String>;
}

/// Finality statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinalityStats {
    /// Latest block at each finality level
    pub latest_blocks: HashMap<FinalityLevel, u64>,
    /// Average confirmation times
    pub avg_confirmation_times: HashMap<FinalityLevel, Duration>,
    /// Number of blocks at each level
    pub blocks_by_level: HashMap<FinalityLevel, u64>,
    /// Last update time
    pub last_updated: Option<SystemTime>,
}

/// Block finality information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlockFinalityInfo {
    /// Block number
    pub block_number: u64,
    /// Block timestamp
    pub block_time: SystemTime,
    /// First seen timestamp
    pub first_seen: SystemTime,
    /// Current finality level
    pub finality_level: FinalityLevel,
    /// Confirmations
    pub confirmations: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_finality_level_ordering() {
        assert!(FinalityLevel::Confirmed < FinalityLevel::Safe);
        assert!(FinalityLevel::Safe < FinalityLevel::Justified);
        assert!(FinalityLevel::Justified < FinalityLevel::Finalized);
    }
    
    #[test]
    fn test_all_up_to() {
        let levels = FinalityLevel::Justified.all_up_to();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], FinalityLevel::Confirmed);
        assert_eq!(levels[1], FinalityLevel::Safe);
        assert_eq!(levels[2], FinalityLevel::Justified);
    }
    
    #[test]
    fn test_ethereum_config() {
        let config = FinalityConfig::ethereum_mainnet();
        assert_eq!(config.chain_id, "ethereum");
        assert_eq!(config.finality_requirements.get(&FinalityLevel::Confirmed), Some(&1));
        assert_eq!(config.finality_requirements.get(&FinalityLevel::Safe), Some(&6));
        assert_eq!(config.finality_requirements.get(&FinalityLevel::Justified), Some(&32));
        assert_eq!(config.finality_requirements.get(&FinalityLevel::Finalized), Some(&64));
    }
    
    #[test]
    fn test_cosmos_config() {
        let config = FinalityConfig::cosmos("cosmoshub-4");
        assert_eq!(config.chain_id, "cosmoshub-4");
        assert_eq!(config.finality_requirements.get(&FinalityLevel::Confirmed), Some(&1));
        assert_eq!(config.finality_requirements.get(&FinalityLevel::Finalized), Some(&2));
        assert!(!config.probabilistic_finality);
    }
}