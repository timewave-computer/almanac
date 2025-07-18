//! Chain reorganization detection and handling

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use dashmap::DashMap;

use almanac_core::Result;
use crate::types::{BlockInfo, ForkInfo};

/// Type of reorganization detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ReorgType {
    /// Simple reorganization (fork at recent block)
    Simple,
    /// Deep reorganization (fork several blocks back)
    Deep,
    /// Critical reorganization (major chain split)
    Critical,
    /// Complex reorganization (multiple forks)
    Complex,
    /// Uncle block detected
    Uncle,
    /// Orphaned block detected
    Orphaned,
}

/// Information about a detected reorganization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reorganization {
    /// Chain identifier
    pub chain: String,
    /// Type of reorganization
    pub reorg_type: ReorgType,
    /// Block number where fork occurred
    pub fork_block: u64,
    /// Original chain block hashes
    pub original_blocks: Vec<String>,
    /// New chain block hashes
    pub new_blocks: Vec<String>,
    /// Number of blocks reorganized
    pub depth: u64,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Timestamp when detected
    pub detected_at: SystemTime,
    /// Events affected by reorganization
    pub affected_events: u64,
    /// Whether rollback was performed
    pub rollback_performed: bool,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Reorganization detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgConfig {
    /// Chain identifier
    pub chain: String,
    /// Number of blocks to track for reorganization detection
    pub tracking_depth: u64,
    /// Minimum confirmations before considering block stable
    pub min_confirmations: u64,
    /// Enable automatic rollback on reorganization
    pub auto_rollback: bool,
    /// Minimum reorganization depth to trigger alerts
    pub alert_threshold: u64,
    /// Maximum reorganization depth to handle automatically
    pub max_auto_rollback_depth: u64,
}

impl Default for ReorgConfig {
    fn default() -> Self {
        Self {
            chain: String::new(),
            tracking_depth: 100,
            min_confirmations: 6,
            auto_rollback: true,
            alert_threshold: 3,
            max_auto_rollback_depth: 20,
        }
    }
}

/// Trait for reorganization handling
#[async_trait]
pub trait ReorgHandler: Send + Sync {
    /// Handle a detected reorganization
    async fn handle_reorganization(&self, reorg: &Reorganization) -> Result<()>;
    
    /// Rollback events from reorganized blocks
    async fn rollback_blocks(&self, chain: &str, from_block: u64, to_block: u64) -> Result<u64>;
    
    /// Notify about reorganization
    async fn notify_reorganization(&self, reorg: &Reorganization) -> Result<()>;
    
    /// Get reorganization statistics
    async fn get_reorg_stats(&self, chain: &str) -> Result<ReorgStats>;
}

/// Reorganization statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReorgStats {
    /// Total reorganizations detected
    pub total_reorgs: u64,
    /// Reorganizations by type
    pub reorgs_by_type: HashMap<ReorgType, u64>,
    /// Average reorganization depth
    pub avg_depth: f64,
    /// Maximum reorganization depth
    pub max_depth: u64,
    /// Total blocks affected
    pub total_blocks_affected: u64,
    /// Total events rolled back
    pub total_events_rolled_back: u64,
    /// Last reorganization
    pub last_reorg: Option<Reorganization>,
}

/// Reorganization detector
pub struct ReorgDetector {
    /// Configuration
    config: ReorgConfig,
    /// Block history by chain
    block_history: Arc<DashMap<String, VecDeque<BlockInfo>>>,
    /// Fork tracking
    #[allow(dead_code)]
    fork_tracker: Arc<DashMap<String, Vec<ForkInfo>>>,
    /// Reorg statistics
    stats: Arc<DashMap<String, ReorgStats>>,
}

impl ReorgDetector {
    /// Create a new reorg detector
    pub fn new(config: ReorgConfig) -> Self {
        Self {
            config,
            block_history: Arc::new(DashMap::new()),
            fork_tracker: Arc::new(DashMap::new()),
            stats: Arc::new(DashMap::new()),
        }
    }
    
    /// Process a new block
    pub async fn process_block(&self, block: BlockInfo) -> Result<Option<Reorganization>> {
        let chain = &self.config.chain;
        
        // Add block to history
        let mut history = self.block_history.entry(chain.clone()).or_default();
        
        // Check for reorganization
        if let Some(parent_idx) = history.iter().position(|b| b.hash == block.parent_hash) {
            if parent_idx != history.len() - 1 {
                // Reorganization detected
                let fork_block = history[parent_idx].number;
                let original_blocks: Vec<String> = history.iter()
                    .skip(parent_idx + 1)
                    .map(|b| b.hash.clone())
                    .collect();
                
                let depth = block.number - fork_block;
                let reorg_type = self.classify_reorg(depth);
                
                let reorg = Reorganization {
                    chain: chain.clone(),
                    reorg_type,
                    fork_block,
                    original_blocks,
                    new_blocks: vec![block.hash.clone()],
                    depth,
                    confidence: self.calculate_confidence(depth, block.confirmations),
                    detected_at: SystemTime::now(),
                    affected_events: 0, // To be calculated
                    rollback_performed: false,
                    metadata: HashMap::new(),
                };
                
                // Update stats
                self.update_stats(&reorg).await;
                
                // Trim history to fork point
                history.truncate(parent_idx + 1);
                
                return Ok(Some(reorg));
            }
        }
        
        // Add block to history
        history.push_back(block.clone());
        
        // Maintain history size
        while history.len() > self.config.tracking_depth as usize {
            history.pop_front();
        }
        
        Ok(None)
    }
    
    /// Classify reorganization type based on depth
    fn classify_reorg(&self, depth: u64) -> ReorgType {
        match depth {
            0..=2 => ReorgType::Simple,
            3..=10 => ReorgType::Deep,
            _ => ReorgType::Critical,
        }
    }
    
    /// Calculate confidence score
    fn calculate_confidence(&self, depth: u64, confirmations: u64) -> f64 {
        let depth_factor = 1.0 / (1.0 + depth as f64);
        let conf_factor = confirmations as f64 / self.config.min_confirmations as f64;
        (depth_factor + conf_factor) / 2.0
    }
    
    /// Update reorganization statistics
    async fn update_stats(&self, reorg: &Reorganization) {
        let mut stats = self.stats.entry(reorg.chain.clone())
            .or_default();
        
        stats.total_reorgs += 1;
        *stats.reorgs_by_type.entry(reorg.reorg_type.clone()).or_insert(0) += 1;
        
        // Update average depth
        let new_avg = (stats.avg_depth * (stats.total_reorgs - 1) as f64 + reorg.depth as f64) 
            / stats.total_reorgs as f64;
        stats.avg_depth = new_avg;
        
        // Update max depth
        if reorg.depth > stats.max_depth {
            stats.max_depth = reorg.depth;
        }
        
        stats.total_blocks_affected += reorg.depth;
        stats.last_reorg = Some(reorg.clone());
    }
    
    /// Get reorganization statistics
    pub async fn get_stats(&self, chain: &str) -> ReorgStats {
        self.stats.get(chain)
            .map(|s| s.clone())
            .unwrap_or_default()
    }
}