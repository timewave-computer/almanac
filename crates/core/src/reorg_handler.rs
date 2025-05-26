/// Chain reorganization detection and handling
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{Result, Error};

/// Type of reorganization detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ReorgType {
    /// Simple reorganization (fork at recent block)
    Simple,
    
    /// Deep reorganization (fork several blocks back)
    Deep,
    
    /// Critical reorganization (major chain split)
    Critical,
    
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

/// Block information for reorganization tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block number
    pub number: u64,
    
    /// Block hash
    pub hash: String,
    
    /// Parent block hash
    pub parent_hash: String,
    
    /// Block timestamp
    pub timestamp: SystemTime,
    
    /// Number of confirmations
    pub confirmations: u64,
    
    /// Whether block is confirmed
    pub is_confirmed: bool,
    
    /// Events in this block
    pub event_count: u64,
}

/// Reorganization detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgConfig {
    /// Chain identifier
    pub chain: String,
    
    /// Number of blocks to track for reorganization detection
    pub tracking_depth: u64,
    
    /// Minimum confirmations before considering block final
    pub confirmation_threshold: u64,
    
    /// Maximum reorganization depth to handle
    pub max_reorg_depth: u64,
    
    /// Whether to automatically rollback on reorganization
    pub auto_rollback: bool,
    
    /// Confidence threshold for reorganization detection
    pub confidence_threshold: f64,
    
    /// Maximum time to wait for block confirmation (seconds)
    pub confirmation_timeout: u64,
    
    /// Whether to track uncle blocks
    pub track_uncles: bool,
    
    /// Reorganization detection sensitivity
    pub sensitivity: ReorgSensitivity,
}

/// Reorganization detection sensitivity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReorgSensitivity {
    /// Low sensitivity (only detect deep reorganizations)
    Low,
    
    /// Medium sensitivity (balanced detection)
    Medium,
    
    /// High sensitivity (detect minor reorganizations)
    High,
    
    /// Paranoid (detect all possible reorganizations)
    Paranoid,
}

impl Default for ReorgConfig {
    fn default() -> Self {
        Self {
            chain: "unknown".to_string(),
            tracking_depth: 100,
            confirmation_threshold: 12,
            max_reorg_depth: 50,
            auto_rollback: true,
            confidence_threshold: 0.8,
            confirmation_timeout: 300,
            track_uncles: false,
            sensitivity: ReorgSensitivity::Medium,
        }
    }
}

/// Rollback operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Chain identifier
    pub chain: String,
    
    /// Block number rolled back to
    pub rollback_to_block: u64,
    
    /// Number of blocks rolled back
    pub blocks_rolled_back: u64,
    
    /// Number of events removed
    pub events_removed: u64,
    
    /// Whether rollback was successful
    pub success: bool,
    
    /// Error message if rollback failed
    pub error_message: Option<String>,
    
    /// Time taken for rollback
    pub rollback_duration: std::time::Duration,
    
    /// Timestamp of rollback
    pub rollback_timestamp: SystemTime,
}

/// Trait for handling blockchain reorganizations
#[async_trait]
pub trait ReorgHandler: Send + Sync {
    /// Configure reorganization handling for a chain
    async fn configure_chain(&self, config: ReorgConfig) -> Result<()>;
    
    /// Process new block and detect reorganizations
    async fn process_block(&self, chain: &str, block: BlockInfo) -> Result<Option<Reorganization>>;
    
    /// Manually check for reorganizations on a chain
    async fn check_reorganization(&self, chain: &str) -> Result<Option<Reorganization>>;
    
    /// Perform rollback to a specific block
    async fn rollback_to_block(&self, chain: &str, block_number: u64) -> Result<RollbackResult>;
    
    /// Get reorganization history for a chain
    async fn get_reorganization_history(&self, chain: &str) -> Result<Vec<Reorganization>>;
    
    /// Get current block tracking state
    async fn get_tracking_state(&self, chain: &str) -> Result<Option<Vec<BlockInfo>>>;
    
    /// Clear reorganization history
    async fn clear_history(&self, chain: &str) -> Result<()>;
    
    /// Get reorganization statistics
    async fn get_statistics(&self, chain: &str) -> Result<ReorgStatistics>;
}

/// Reorganization detection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgStatistics {
    /// Total reorganizations detected
    pub total_reorganizations: u64,
    
    /// Reorganizations by type
    pub reorgs_by_type: HashMap<ReorgType, u64>,
    
    /// Average reorganization depth
    pub avg_reorg_depth: f64,
    
    /// Maximum reorganization depth seen
    pub max_reorg_depth: u64,
    
    /// Total blocks rolled back
    pub total_blocks_rolled_back: u64,
    
    /// Total events affected
    pub total_events_affected: u64,
    
    /// Successful rollback rate
    pub rollback_success_rate: f64,
    
    /// Average rollback duration
    pub avg_rollback_duration: std::time::Duration,
    
    /// Last reorganization timestamp
    pub last_reorg_timestamp: Option<SystemTime>,
}

impl Default for ReorgStatistics {
    fn default() -> Self {
        Self {
            total_reorganizations: 0,
            reorgs_by_type: HashMap::new(),
            avg_reorg_depth: 0.0,
            max_reorg_depth: 0,
            total_blocks_rolled_back: 0,
            total_events_affected: 0,
            rollback_success_rate: 1.0,
            avg_rollback_duration: std::time::Duration::from_secs(0),
            last_reorg_timestamp: None,
        }
    }
}

/// Default implementation of reorganization handler
pub struct DefaultReorgHandler {
    /// Chain configurations
    configs: Arc<RwLock<HashMap<String, ReorgConfig>>>,
    
    /// Block tracking for each chain
    block_tracking: Arc<RwLock<HashMap<String, VecDeque<BlockInfo>>>>,
    
    /// Reorganization history
    reorg_history: Arc<RwLock<HashMap<String, Vec<Reorganization>>>>,
    
    /// Statistics for each chain
    statistics: Arc<RwLock<HashMap<String, ReorgStatistics>>>,
}

impl DefaultReorgHandler {
    /// Create a new reorg handler
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            block_tracking: Arc::new(RwLock::new(HashMap::new())),
            reorg_history: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Detect reorganization by comparing block chains
    async fn detect_reorganization(
        &self,
        chain: &str,
        new_block: &BlockInfo,
        config: &ReorgConfig,
    ) -> Result<Option<Reorganization>> {
        let tracking = self.block_tracking.read().await;
        let blocks = match tracking.get(chain) {
            Some(blocks) => blocks,
            None => return Ok(None),
        };
        
        // Check if new block extends the current chain
        if let Some(last_block) = blocks.back() {
            if new_block.parent_hash == last_block.hash {
                // Normal extension, no reorganization
                return Ok(None);
            }
        }
        
        // Look for fork point
        let mut fork_block = None;
        let mut fork_depth = 0;
        
        for (i, block) in blocks.iter().rev().enumerate() {
            if new_block.parent_hash == block.hash {
                fork_block = Some(block.number);
                fork_depth = i as u64;
                break;
            }
            
            if i >= config.max_reorg_depth as usize {
                break;
            }
        }
        
        if let Some(fork_at) = fork_block {
            // Calculate confidence based on depth and confirmations
            let confidence = self.calculate_confidence(fork_depth, new_block.confirmations, config);
            
            if confidence >= config.confidence_threshold {
                let reorg_type = self.classify_reorganization(fork_depth, config);
                
                let original_blocks: Vec<String> = blocks
                    .iter()
                    .rev()
                    .take(fork_depth as usize)
                    .map(|b| b.hash.clone())
                    .collect();
                
                let reorganization = Reorganization {
                    chain: chain.to_string(),
                    reorg_type,
                    fork_block: fork_at,
                    original_blocks,
                    new_blocks: vec![new_block.hash.clone()],
                    depth: fork_depth,
                    confidence,
                    detected_at: SystemTime::now(),
                    affected_events: self.count_affected_events(blocks, fork_depth).await,
                    rollback_performed: false,
                    metadata: HashMap::new(),
                };
                
                return Ok(Some(reorganization));
            }
        }
        
        Ok(None)
    }
    
    /// Calculate confidence score for reorganization detection
    fn calculate_confidence(&self, depth: u64, confirmations: u64, config: &ReorgConfig) -> f64 {
        let depth_factor = match config.sensitivity {
            ReorgSensitivity::Low => 1.0 / (depth as f64 + 1.0),
            ReorgSensitivity::Medium => 1.0 / (depth as f64 * 0.5 + 1.0),
            ReorgSensitivity::High => 1.0 / (depth as f64 * 0.25 + 1.0),
            ReorgSensitivity::Paranoid => 1.0,
        };
        
        let confirmation_factor = if confirmations >= config.confirmation_threshold {
            1.0
        } else {
            confirmations as f64 / config.confirmation_threshold as f64
        };
        
        (depth_factor * confirmation_factor).min(1.0)
    }
    
    /// Classify the type of reorganization
    fn classify_reorganization(&self, depth: u64, config: &ReorgConfig) -> ReorgType {
        match depth {
            1 => ReorgType::Uncle,
            2..=5 => ReorgType::Simple,
            6..=20 => ReorgType::Deep,
            _ => {
                if depth > config.max_reorg_depth / 2 {
                    ReorgType::Critical
                } else {
                    ReorgType::Deep
                }
            }
        }
    }
    
    /// Count events affected by reorganization
    async fn count_affected_events(&self, blocks: &VecDeque<BlockInfo>, depth: u64) -> u64 {
        blocks
            .iter()
            .rev()
            .take(depth as usize)
            .map(|b| b.event_count)
            .sum()
    }
    
    /// Perform the actual rollback operation
    async fn perform_rollback(
        &self,
        chain: &str,
        target_block: u64,
        reorg: &Reorganization,
    ) -> Result<RollbackResult> {
        let start_time = SystemTime::now();
        
        // Remove blocks after target block from tracking
        let mut tracking = self.block_tracking.write().await;
        if let Some(blocks) = tracking.get_mut(chain) {
            let original_count = blocks.len();
            blocks.retain(|b| b.number <= target_block);
            let blocks_removed = original_count - blocks.len();
            
            let result = RollbackResult {
                chain: chain.to_string(),
                rollback_to_block: target_block,
                blocks_rolled_back: blocks_removed as u64,
                events_removed: reorg.affected_events,
                success: true,
                error_message: None,
                rollback_duration: start_time.elapsed().unwrap_or_default(),
                rollback_timestamp: SystemTime::now(),
            };
            
            return Ok(result);
        }
        
        Ok(RollbackResult {
            chain: chain.to_string(),
            rollback_to_block: target_block,
            blocks_rolled_back: 0,
            events_removed: 0,
            success: false,
            error_message: Some("Chain not found in tracking".to_string()),
            rollback_duration: start_time.elapsed().unwrap_or_default(),
            rollback_timestamp: SystemTime::now(),
        })
    }
    
    /// Update statistics after reorganization
    async fn update_statistics(&self, chain: &str, reorg: &Reorganization, rollback: &RollbackResult) {
        let mut stats = self.statistics.write().await;
        let chain_stats = stats.entry(chain.to_string()).or_default();
        
        chain_stats.total_reorganizations += 1;
        *chain_stats.reorgs_by_type.entry(reorg.reorg_type.clone()).or_insert(0) += 1;
        
        // Update averages
        let total = chain_stats.total_reorganizations as f64;
        chain_stats.avg_reorg_depth = (chain_stats.avg_reorg_depth * (total - 1.0) + reorg.depth as f64) / total;
        
        if reorg.depth > chain_stats.max_reorg_depth {
            chain_stats.max_reorg_depth = reorg.depth;
        }
        
        chain_stats.total_blocks_rolled_back += rollback.blocks_rolled_back;
        chain_stats.total_events_affected += rollback.events_removed;
        
        if rollback.success {
            let success_count = chain_stats.rollback_success_rate * (total - 1.0) + 1.0;
            chain_stats.rollback_success_rate = success_count / total;
        } else {
            let success_count = chain_stats.rollback_success_rate * (total - 1.0);
            chain_stats.rollback_success_rate = success_count / total;
        }
        
        // Update average rollback duration
        let current_avg = chain_stats.avg_rollback_duration.as_millis() as f64;
        let new_duration = rollback.rollback_duration.as_millis() as f64;
        let new_avg = (current_avg * (total - 1.0) + new_duration) / total;
        chain_stats.avg_rollback_duration = std::time::Duration::from_millis(new_avg as u64);
        
        chain_stats.last_reorg_timestamp = Some(reorg.detected_at);
    }
}

impl Default for DefaultReorgHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReorgHandler for DefaultReorgHandler {
    async fn configure_chain(&self, config: ReorgConfig) -> Result<()> {
        let mut configs = self.configs.write().await;
        configs.insert(config.chain.clone(), config);
        Ok(())
    }
    
    async fn process_block(&self, chain: &str, block: BlockInfo) -> Result<Option<Reorganization>> {
        // Get configuration
        let configs = self.configs.read().await;
        let config = configs.get(chain)
            .ok_or_else(|| Error::generic(format!("Chain {} not configured", chain)))?
            .clone();
        drop(configs);
        
        // Detect reorganization
        let reorg = self.detect_reorganization(chain, &block, &config).await?;
        
        // Update block tracking
        let mut tracking = self.block_tracking.write().await;
        let blocks = tracking.entry(chain.to_string()).or_insert_with(VecDeque::new);
        
        if let Some(ref reorganization) = reorg {
            // Handle reorganization
            if config.auto_rollback {
                let rollback_result = self.perform_rollback(
                    chain,
                    reorganization.fork_block,
                    reorganization,
                ).await?;
                
                // Update statistics
                self.update_statistics(chain, reorganization, &rollback_result).await;
                
                // Store reorganization in history
                let mut history = self.reorg_history.write().await;
                history.entry(chain.to_string()).or_default().push(reorganization.clone());
            }
        }
        
        // Add new block to tracking
        blocks.push_back(block);
        
        // Maintain tracking depth
        while blocks.len() > config.tracking_depth as usize {
            blocks.pop_front();
        }
        
        Ok(reorg)
    }
    
    async fn check_reorganization(&self, _chain: &str) -> Result<Option<Reorganization>> {
        // This would typically involve querying the blockchain
        // For now, return None as no reorganization detected
        Ok(None)
    }
    
    async fn rollback_to_block(&self, chain: &str, block_number: u64) -> Result<RollbackResult> {
        let fake_reorg = Reorganization {
            chain: chain.to_string(),
            reorg_type: ReorgType::Deep,
            fork_block: block_number,
            original_blocks: vec![],
            new_blocks: vec![],
            depth: 0,
            confidence: 1.0,
            detected_at: SystemTime::now(),
            affected_events: 0,
            rollback_performed: false,
            metadata: HashMap::new(),
        };
        
        self.perform_rollback(chain, block_number, &fake_reorg).await
    }
    
    async fn get_reorganization_history(&self, chain: &str) -> Result<Vec<Reorganization>> {
        let history = self.reorg_history.read().await;
        Ok(history.get(chain).cloned().unwrap_or_default())
    }
    
    async fn get_tracking_state(&self, chain: &str) -> Result<Option<Vec<BlockInfo>>> {
        let tracking = self.block_tracking.read().await;
        Ok(tracking.get(chain).map(|blocks| blocks.iter().cloned().collect()))
    }
    
    async fn clear_history(&self, chain: &str) -> Result<()> {
        let mut history = self.reorg_history.write().await;
        history.remove(chain);
        Ok(())
    }
    
    async fn get_statistics(&self, chain: &str) -> Result<ReorgStatistics> {
        let stats = self.statistics.read().await;
        Ok(stats.get(chain).cloned().unwrap_or_default())
    }
}

/// Reorganization event listener trait
#[async_trait]
pub trait ReorgEventListener: Send + Sync {
    /// Called when a reorganization is detected
    async fn on_reorganization_detected(&self, reorg: &Reorganization) -> Result<()>;
    
    /// Called when a rollback is performed
    async fn on_rollback_performed(&self, result: &RollbackResult) -> Result<()>;
}

/// Predefined reorganization configurations for popular chains
pub struct PredefinedReorgConfigs;

impl PredefinedReorgConfigs {
    /// Ethereum reorganization configuration
    pub fn ethereum() -> ReorgConfig {
        ReorgConfig {
            chain: "ethereum".to_string(),
            tracking_depth: 200,
            confirmation_threshold: 12,
            max_reorg_depth: 50,
            auto_rollback: true,
            confidence_threshold: 0.9,
            confirmation_timeout: 180,
            track_uncles: true,
            sensitivity: ReorgSensitivity::Medium,
        }
    }
    
    /// Bitcoin reorganization configuration
    pub fn bitcoin() -> ReorgConfig {
        ReorgConfig {
            chain: "bitcoin".to_string(),
            tracking_depth: 100,
            confirmation_threshold: 6,
            max_reorg_depth: 20,
            auto_rollback: true,
            confidence_threshold: 0.95,
            confirmation_timeout: 600,
            track_uncles: false,
            sensitivity: ReorgSensitivity::High,
        }
    }
    
    /// Polygon reorganization configuration
    pub fn polygon() -> ReorgConfig {
        ReorgConfig {
            chain: "polygon".to_string(),
            tracking_depth: 500,
            confirmation_threshold: 20,
            max_reorg_depth: 100,
            auto_rollback: true,
            confidence_threshold: 0.8,
            confirmation_timeout: 60,
            track_uncles: true,
            sensitivity: ReorgSensitivity::Low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_reorg_handler_creation() {
        let handler = DefaultReorgHandler::new();
        
        // Configure a test chain
        let config = ReorgConfig {
            chain: "test".to_string(),
            tracking_depth: 50,
            ..Default::default()
        };
        
        assert!(handler.configure_chain(config).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_block_processing_no_reorg() {
        let handler = DefaultReorgHandler::new();
        let config = ReorgConfig {
            chain: "test".to_string(),
            ..Default::default()
        };
        handler.configure_chain(config).await.unwrap();
        
        let block1 = BlockInfo {
            number: 100,
            hash: "hash1".to_string(),
            parent_hash: "parent1".to_string(),
            timestamp: SystemTime::now(),
            confirmations: 12,
            is_confirmed: true,
            event_count: 5,
        };
        
        let block2 = BlockInfo {
            number: 101,
            hash: "hash2".to_string(),
            parent_hash: "hash1".to_string(),
            timestamp: SystemTime::now(),
            confirmations: 11,
            is_confirmed: true,
            event_count: 3,
        };
        
        // Process first block
        let result1 = handler.process_block("test", block1).await.unwrap();
        assert!(result1.is_none());
        
        // Process second block (normal extension)
        let result2 = handler.process_block("test", block2).await.unwrap();
        assert!(result2.is_none());
    }
    
    #[tokio::test]
    async fn test_reorganization_detection() {
        let handler = DefaultReorgHandler::new();
        let config = ReorgConfig {
            chain: "test".to_string(),
            auto_rollback: false, // Disable auto rollback for testing
            confidence_threshold: 0.5,
            ..Default::default()
        };
        handler.configure_chain(config).await.unwrap();
        
        // Add initial blocks
        let block1 = BlockInfo {
            number: 100,
            hash: "hash1".to_string(),
            parent_hash: "parent1".to_string(),
            timestamp: SystemTime::now(),
            confirmations: 12,
            is_confirmed: true,
            event_count: 5,
        };
        
        let block2 = BlockInfo {
            number: 101,
            hash: "hash2".to_string(),
            parent_hash: "hash1".to_string(),
            timestamp: SystemTime::now(),
            confirmations: 11,
            is_confirmed: true,
            event_count: 3,
        };
        
        handler.process_block("test", block1).await.unwrap();
        handler.process_block("test", block2).await.unwrap();
        
        // Add a block that causes reorganization
        let reorg_block = BlockInfo {
            number: 101,
            hash: "hash2_alt".to_string(),
            parent_hash: "hash1".to_string(), // Points to block1, not block2
            timestamp: SystemTime::now(),
            confirmations: 12,
            is_confirmed: true,
            event_count: 4,
        };
        
        let result = handler.process_block("test", reorg_block).await.unwrap();
        assert!(result.is_some());
        
        let reorg = result.unwrap();
        assert_eq!(reorg.chain, "test");
        assert_eq!(reorg.fork_block, 100);
        assert_eq!(reorg.depth, 1);
    }
    
    #[tokio::test]
    async fn test_rollback_operation() {
        let handler = DefaultReorgHandler::new();
        let config = ReorgConfig {
            chain: "test".to_string(),
            ..Default::default()
        };
        handler.configure_chain(config).await.unwrap();
        
        // Add some blocks
        for i in 100..110 {
            let block = BlockInfo {
                number: i,
                hash: format!("hash{}", i),
                parent_hash: format!("parent{}", i),
                timestamp: SystemTime::now(),
                confirmations: 12,
                is_confirmed: true,
                event_count: 1,
            };
            handler.process_block("test", block).await.unwrap();
        }
        
        // Perform rollback
        let result = handler.rollback_to_block("test", 105).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.rollback_to_block, 105);
        assert!(result.blocks_rolled_back > 0);
    }
    
    #[tokio::test]
    async fn test_statistics_tracking() {
        let handler = DefaultReorgHandler::new();
        let config = ReorgConfig {
            chain: "test".to_string(),
            ..Default::default()
        };
        handler.configure_chain(config).await.unwrap();
        
        let stats = handler.get_statistics("test").await.unwrap();
        assert_eq!(stats.total_reorganizations, 0);
        assert_eq!(stats.avg_reorg_depth, 0.0);
    }
    
    #[tokio::test]
    async fn test_tracking_state() {
        let handler = DefaultReorgHandler::new();
        let config = ReorgConfig {
            chain: "test".to_string(),
            tracking_depth: 5,
            ..Default::default()
        };
        handler.configure_chain(config).await.unwrap();
        
        // Add blocks beyond tracking depth
        for i in 100..110 {
            let block = BlockInfo {
                number: i,
                hash: format!("hash{}", i),
                parent_hash: format!("parent{}", i),
                timestamp: SystemTime::now(),
                confirmations: 12,
                is_confirmed: true,
                event_count: 1,
            };
            handler.process_block("test", block).await.unwrap();
        }
        
        let state = handler.get_tracking_state("test").await.unwrap();
        assert!(state.is_some());
        
        let blocks = state.unwrap();
        assert_eq!(blocks.len(), 5); // Should only track last 5 blocks
        assert_eq!(blocks[0].number, 105); // Oldest tracked block
        assert_eq!(blocks[4].number, 109); // Newest tracked block
    }
    
    #[test]
    fn test_predefined_configs() {
        let eth_config = PredefinedReorgConfigs::ethereum();
        assert_eq!(eth_config.chain, "ethereum");
        assert_eq!(eth_config.confirmation_threshold, 12);
        assert!(eth_config.track_uncles);
        
        let btc_config = PredefinedReorgConfigs::bitcoin();
        assert_eq!(btc_config.chain, "bitcoin");
        assert_eq!(btc_config.confirmation_threshold, 6);
        assert!(!btc_config.track_uncles);
        
        let polygon_config = PredefinedReorgConfigs::polygon();
        assert_eq!(polygon_config.chain, "polygon");
        assert_eq!(polygon_config.confirmation_threshold, 20);
    }
} 