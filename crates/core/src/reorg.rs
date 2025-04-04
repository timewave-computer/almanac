// Purpose: Defines traits and types for handling chain reorganizations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

use crate::types::ChainId;
use crate::Result;

/// Represents a canonical block in a blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalBlock {
    /// Block number
    pub number: u64,
    
    /// Block hash
    pub hash: String,
    
    /// Parent block hash
    pub parent_hash: String,
    
    /// Timestamp of the block in seconds since UNIX epoch
    pub timestamp: u64,
}

/// Represents a block that has been reorganized out of the canonical chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorganizedBlock {
    /// The block that was removed from the canonical chain
    pub old_block: CanonicalBlock,
    
    /// The block that replaced it in the canonical chain
    pub new_block: CanonicalBlock,
    
    /// Depth of the reorganization (how many blocks back from the tip)
    pub depth: u64,
    
    /// Timestamp when the reorganization was detected
    pub detected_at: u64,
}

/// Event emitted when a chain reorganization is detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgEvent {
    /// Chain identifier
    pub chain_id: ChainId,
    
    /// Blocks that were reorganized out of the canonical chain
    pub reorganized_blocks: Vec<ReorganizedBlock>,
    
    /// New canonical block that triggered the reorganization detection
    pub canonical_tip: CanonicalBlock,
}

/// Strategy for handling chain reorganizations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReorgStrategy {
    /// Ignore reorganizations (not recommended for production)
    Ignore,
    
    /// Revert and reprocess affected blocks
    RevertAndReprocess,
    
    /// Apply custom handling logic
    Custom,
}

/// Configuration for reorganization handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgConfig {
    /// Maximum depth of reorganizations to handle
    pub max_depth: u64,
    
    /// Strategy for handling reorganizations
    pub strategy: ReorgStrategy,
    
    /// Number of confirmations required before considering a block final
    pub confirmations: u64,
}

impl Default for ReorgConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            strategy: ReorgStrategy::RevertAndReprocess,
            confirmations: 12,
        }
    }
}

/// Trait for detecting chain reorganizations
#[async_trait]
pub trait ReorgDetector: Send + Sync + 'static {
    /// Start monitoring for reorganizations
    async fn start(&self) -> Result<()>;
    
    /// Stop monitoring for reorganizations
    async fn stop(&self) -> Result<()>;
    
    /// Get the current chain ID
    fn chain_id(&self) -> &ChainId;
    
    /// Check if a reorganization has occurred since the specified block
    async fn check_reorg(&self, from_block: u64) -> Result<Option<ReorgEvent>>;
    
    /// Subscribe to reorganization events
    async fn subscribe(&self) -> Result<Box<dyn ReorgSubscription>>;
    
    /// Set the configuration for reorganization handling
    async fn set_config(&mut self, config: ReorgConfig) -> Result<()>;
    
    /// Get the current configuration
    fn config(&self) -> &ReorgConfig;
}

/// Type alias for a boxed reorg detector
pub type BoxedReorgDetector = Arc<dyn ReorgDetector>;

/// Trait for subscribing to reorganization events
#[async_trait]
pub trait ReorgSubscription: Send + Sync + 'static {
    /// Wait for the next reorganization event
    async fn next(&mut self) -> Option<ReorgEvent>;
    
    /// Close the subscription
    async fn close(&mut self) -> Result<()>;
}

/// Trait for handling chain reorganizations
#[async_trait]
pub trait ReorgHandler: Send + Sync + 'static {
    /// Handle a chain reorganization event
    async fn handle_reorg(&self, event: ReorgEvent) -> Result<()>;
    
    /// Get the chain ID this handler is for
    fn chain_id(&self) -> &ChainId;
    
    /// Get the current configuration
    fn config(&self) -> &ReorgConfig;
    
    /// Set the configuration for reorganization handling
    async fn set_config(&mut self, config: ReorgConfig) -> Result<()>;
}

/// Implementation of a basic chain reorganization tracker
pub struct ChainReorgTracker {
    /// Chain identifier
    chain_id: ChainId,
    
    /// Configuration for reorganization handling
    config: ReorgConfig,
    
    /// Recent canonical blocks, kept for detecting reorganizations
    /// The most recent block is at the front of the deque
    recent_blocks: VecDeque<CanonicalBlock>,
    
    /// Maximum number of blocks to keep in memory
    max_blocks: usize,
}

impl ChainReorgTracker {
    /// Create a new chain reorganization tracker
    pub fn new(chain_id: ChainId, config: ReorgConfig) -> Self {
        let max_blocks = (config.max_depth as usize) * 2;
        Self {
            chain_id,
            config,
            recent_blocks: VecDeque::with_capacity(max_blocks),
            max_blocks,
        }
    }
    
    /// Add a new canonical block to the tracker
    pub fn add_block(&mut self, block: CanonicalBlock) {
        // Add new block to the front
        self.recent_blocks.push_front(block);
        
        // Remove oldest block if we've exceeded the maximum
        if self.recent_blocks.len() > self.max_blocks {
            self.recent_blocks.pop_back();
        }
    }
    
    /// Check if a reorganization has occurred
    pub fn check_reorg(&self, new_block: &CanonicalBlock) -> Option<ReorgEvent> {
        // If this is the first block or the new block builds on the previous tip, no reorg
        if self.recent_blocks.is_empty() || new_block.parent_hash == self.recent_blocks.front()?.hash {
            return None;
        }
        
        // Find the common ancestor
        let mut reorganized_blocks = Vec::new();
        let mut common_ancestor_found = false;
        let mut depth = 0;
        
        for (i, old_block) in self.recent_blocks.iter().enumerate() {
            if new_block.parent_hash == old_block.hash {
                // Found common ancestor, no need to continue
                common_ancestor_found = true;
                break;
            }
            
            depth = i as u64 + 1;
            
            // If we've gone beyond the max depth, stop tracking
            if depth > self.config.max_depth {
                break;
            }
            
            // Add the reorganized block
            reorganized_blocks.push(ReorganizedBlock {
                old_block: old_block.clone(),
                new_block: new_block.clone(), // For now, we don't have the actual replacement blocks
                depth,
                detected_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }
        
        // If we didn't find a common ancestor or the depth exceeds our max, return None
        if !common_ancestor_found || depth > self.config.max_depth {
            return None;
        }
        
        // Create and return the reorganization event
        Some(ReorgEvent {
            chain_id: self.chain_id.clone(),
            reorganized_blocks,
            canonical_tip: new_block.clone(),
        })
    }
} 