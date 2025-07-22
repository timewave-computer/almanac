//! Finality tracking and confirmation for blockchain events

use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use almanac_core::{BlockStatus, Error, Result};

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
    /// Convert to BlockStatus
    pub fn to_block_status(&self) -> BlockStatus {
        match self {
            FinalityLevel::Confirmed => BlockStatus::Processed,
            FinalityLevel::Safe => BlockStatus::Processed,
            FinalityLevel::Justified => BlockStatus::Finalized,
            FinalityLevel::Finalized => BlockStatus::Finalized,
        }
    }

    /// Get all finality levels up to and including this one
    pub fn all_up_to(&self) -> Vec<FinalityLevel> {
        match self {
            FinalityLevel::Confirmed => vec![FinalityLevel::Confirmed],
            FinalityLevel::Safe => vec![FinalityLevel::Confirmed, FinalityLevel::Safe],
            FinalityLevel::Justified => vec![
                FinalityLevel::Confirmed,
                FinalityLevel::Safe,
                FinalityLevel::Justified,
            ],
            FinalityLevel::Finalized => vec![
                FinalityLevel::Confirmed,
                FinalityLevel::Safe,
                FinalityLevel::Justified,
                FinalityLevel::Finalized,
            ],
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
    async fn process_block(
        &self,
        chain: &str,
        block_number: u64,
        block_time: SystemTime,
    ) -> Result<()>;

    /// Get finality level for a block
    async fn get_finality_level(&self, chain: &str, block_number: u64) -> Result<FinalityLevel>;

    /// Get all blocks at a specific finality level
    async fn get_blocks_at_level(&self, chain: &str, level: FinalityLevel) -> Result<Vec<u64>>;

    /// Check if a block has reached a specific finality level
    async fn has_reached_finality(
        &self,
        chain: &str,
        block_number: u64,
        level: FinalityLevel,
    ) -> Result<bool>;

    /// Get finality statistics
    async fn get_stats(&self, chain: &str) -> Result<FinalityStats>;
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

/// Advanced finality processor implementation
pub struct AdvancedFinalityProcessor {
    /// Chain configurations
    configs: Arc<DashMap<String, FinalityConfig>>,
    /// Block tracking
    block_info: Arc<DashMap<String, DashMap<u64, BlockFinalityInfo>>>,
    /// Latest block by chain
    latest_blocks: Arc<DashMap<String, u64>>,
    /// Finality statistics
    stats: Arc<DashMap<String, FinalityStats>>,
}

impl Default for AdvancedFinalityProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedFinalityProcessor {
    /// Create a new finality processor
    pub fn new() -> Self {
        Self {
            configs: Arc::new(DashMap::new()),
            block_info: Arc::new(DashMap::new()),
            latest_blocks: Arc::new(DashMap::new()),
            stats: Arc::new(DashMap::new()),
        }
    }

    /// Add chain configuration
    pub fn add_chain_config(&self, config: FinalityConfig) {
        self.configs.insert(config.chain_id.clone(), config);
    }

    /// Update block finality level
    #[allow(dead_code)]
    async fn update_finality_level(
        &self,
        chain: &str,
        block_number: u64,
        new_level: FinalityLevel,
    ) -> Result<()> {
        if let Some(chain_blocks) = self.block_info.get(chain) {
            if let Some(mut block_info) = chain_blocks.get_mut(&block_number) {
                if new_level > block_info.finality_level {
                    block_info.finality_level = new_level;

                    // Update stats
                    self.update_stats(chain, new_level).await?;
                }
            }
        }

        Ok(())
    }

    /// Update finality statistics
    #[allow(dead_code)]
    async fn update_stats(&self, chain: &str, level: FinalityLevel) -> Result<()> {
        let mut stats = self.stats.entry(chain.to_string()).or_default();

        *stats.blocks_by_level.entry(level).or_insert(0) += 1;
        stats.last_updated = Some(SystemTime::now());

        Ok(())
    }

    /// Check and update finality for all blocks
    pub async fn check_finality(&self, chain: &str) -> Result<()> {
        let config = self
            .configs
            .get(chain)
            .ok_or_else(|| Error::generic(format!("No config for chain {}", chain)))?;

        let latest_block = *self
            .latest_blocks
            .get(chain)
            .ok_or_else(|| Error::generic(format!("No latest block for chain {}", chain)))?;

        if let Some(chain_blocks) = self.block_info.get(chain) {
            let now = SystemTime::now();

            for mut entry in chain_blocks.iter_mut() {
                let block_info = entry.value_mut();
                let confirmations = latest_block.saturating_sub(block_info.block_number);
                block_info.confirmations = confirmations;

                // Check each finality level
                for level in &[
                    FinalityLevel::Confirmed,
                    FinalityLevel::Safe,
                    FinalityLevel::Justified,
                    FinalityLevel::Finalized,
                ] {
                    if block_info.finality_level >= *level {
                        continue;
                    }

                    // Check block confirmations
                    if let Some(required_confirmations) = config.finality_requirements.get(level) {
                        if confirmations < *required_confirmations {
                            continue;
                        }
                    }

                    // Check time requirements
                    if let Some(required_time) = config.time_requirements.get(level) {
                        let elapsed = now
                            .duration_since(block_info.block_time)
                            .unwrap_or(Duration::from_secs(0));
                        if elapsed < *required_time {
                            continue;
                        }
                    }

                    // Update finality level
                    block_info.finality_level = *level;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl FinalityProcessor for AdvancedFinalityProcessor {
    async fn process_block(
        &self,
        chain: &str,
        block_number: u64,
        block_time: SystemTime,
    ) -> Result<()> {
        // Update latest block
        self.latest_blocks.insert(chain.to_string(), block_number);

        // Add block info
        let chain_blocks = self.block_info.entry(chain.to_string()).or_default();

        chain_blocks.insert(
            block_number,
            BlockFinalityInfo {
                block_number,
                block_time,
                first_seen: SystemTime::now(),
                finality_level: FinalityLevel::Confirmed,
                confirmations: 0,
            },
        );

        // Check finality for all blocks
        self.check_finality(chain).await?;

        // Clean up old blocks
        if let Some(config) = self.configs.get(chain) {
            let cutoff = block_number.saturating_sub(config.max_reorg_depth * 2);
            chain_blocks.retain(|k, _| *k >= cutoff);
        }

        Ok(())
    }

    async fn get_finality_level(&self, chain: &str, block_number: u64) -> Result<FinalityLevel> {
        if let Some(chain_blocks) = self.block_info.get(chain) {
            if let Some(info) = chain_blocks.get(&block_number) {
                return Ok(info.finality_level);
            }
        }
        Err(Error::generic(format!(
            "Block {} not found for chain {}",
            block_number, chain
        )))
    }

    async fn get_blocks_at_level(&self, chain: &str, level: FinalityLevel) -> Result<Vec<u64>> {
        Ok(self
            .block_info
            .get(chain)
            .map(|chain_blocks| {
                chain_blocks
                    .iter()
                    .filter(|entry| entry.value().finality_level >= level)
                    .map(|entry| *entry.key())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn has_reached_finality(
        &self,
        chain: &str,
        block_number: u64,
        level: FinalityLevel,
    ) -> Result<bool> {
        if let Some(chain_blocks) = self.block_info.get(chain) {
            if let Some(info) = chain_blocks.get(&block_number) {
                return Ok(info.finality_level >= level);
            }
        }
        Ok(false)
    }

    async fn get_stats(&self, chain: &str) -> Result<FinalityStats> {
        Ok(self.stats.get(chain).map(|s| s.clone()).unwrap_or_default())
    }
}
