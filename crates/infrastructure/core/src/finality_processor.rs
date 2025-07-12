// Main finality processor implementation for cross-chain event tracking
//
// This processor handles finality detection and confirmation for events
// across different blockchain networks with different finality models

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock, mpsc};
use tokio::time::interval;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::{BlockStatus, Error, Result};

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
            FinalityLevel::Confirmed => BlockStatus::Confirmed,
            FinalityLevel::Safe => BlockStatus::Safe,
            FinalityLevel::Justified => BlockStatus::Justified,
            FinalityLevel::Finalized => BlockStatus::Finalized,
        }
    }
    
    /// Get finality level from block status
    pub fn from_block_status(status: BlockStatus) -> Option<Self> {
        match status {
            BlockStatus::Confirmed => Some(FinalityLevel::Confirmed),
            BlockStatus::Safe => Some(FinalityLevel::Safe),
            BlockStatus::Justified => Some(FinalityLevel::Justified),
            BlockStatus::Finalized => Some(FinalityLevel::Finalized),
            BlockStatus::Latest => None,
        }
    }
    
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
pub struct ChainFinalityConfig {
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

impl Default for ChainFinalityConfig {
    fn default() -> Self {
        let mut finality_requirements = HashMap::new();
        finality_requirements.insert(FinalityLevel::Confirmed, 1);
        finality_requirements.insert(FinalityLevel::Safe, 6);
        finality_requirements.insert(FinalityLevel::Justified, 12);
        finality_requirements.insert(FinalityLevel::Finalized, 32);
        
        let mut time_requirements = HashMap::new();
        time_requirements.insert(FinalityLevel::Confirmed, Duration::from_secs(12));
        time_requirements.insert(FinalityLevel::Safe, Duration::from_secs(72));
        time_requirements.insert(FinalityLevel::Justified, Duration::from_secs(144));
        time_requirements.insert(FinalityLevel::Finalized, Duration::from_secs(384));
        
        Self {
            chain_id: "unknown".to_string(),
            finality_requirements,
            time_requirements,
            max_reorg_depth: 100,
            probabilistic_finality: false,
            reorg_detection: true,
            auto_finality_timeout: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Configuration for the finality processor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityProcessorConfig {
    /// Per-chain finality configurations
    pub chains: HashMap<String, ChainFinalityConfig>,
    
    /// Global finality check interval
    pub check_interval: Duration,
    
    /// Maximum events to process in a single batch
    pub max_batch_size: usize,
    
    /// Enable metrics collection
    pub enable_metrics: bool,
    
    /// Finality state persistence
    pub persist_finality_state: bool,
}

impl Default for FinalityProcessorConfig {
    fn default() -> Self {
        let mut chains = HashMap::new();
        
        // Ethereum mainnet
        let mut eth_config = ChainFinalityConfig {
            chain_id: "ethereum".to_string(),
            ..Default::default()
        };
        eth_config.finality_requirements.insert(FinalityLevel::Confirmed, 1);
        eth_config.finality_requirements.insert(FinalityLevel::Safe, 3);
        eth_config.finality_requirements.insert(FinalityLevel::Justified, 12);
        eth_config.finality_requirements.insert(FinalityLevel::Finalized, 32);
        eth_config.probabilistic_finality = true;
        chains.insert("ethereum".to_string(), eth_config);
        
        // Polygon
        let mut polygon_config = ChainFinalityConfig {
            chain_id: "polygon".to_string(),
            ..Default::default()
        };
        polygon_config.finality_requirements.insert(FinalityLevel::Confirmed, 1);
        polygon_config.finality_requirements.insert(FinalityLevel::Safe, 128);
        polygon_config.finality_requirements.insert(FinalityLevel::Justified, 256);
        polygon_config.finality_requirements.insert(FinalityLevel::Finalized, 512);
        polygon_config.probabilistic_finality = true;
        chains.insert("polygon".to_string(), polygon_config);
        
        // Base
        let mut base_config = ChainFinalityConfig {
            chain_id: "base".to_string(),
            ..Default::default()
        };
        base_config.finality_requirements.insert(FinalityLevel::Confirmed, 1);
        base_config.finality_requirements.insert(FinalityLevel::Safe, 3);
        base_config.finality_requirements.insert(FinalityLevel::Justified, 10);
        base_config.finality_requirements.insert(FinalityLevel::Finalized, 30);
        base_config.probabilistic_finality = true;
        chains.insert("base".to_string(), base_config);
        
        // Osmosis
        let mut osmosis_config = ChainFinalityConfig {
            chain_id: "osmosis".to_string(),
            ..Default::default()
        };
        osmosis_config.finality_requirements.insert(FinalityLevel::Confirmed, 1);
        osmosis_config.finality_requirements.insert(FinalityLevel::Safe, 1);
        osmosis_config.finality_requirements.insert(FinalityLevel::Justified, 1);
        osmosis_config.finality_requirements.insert(FinalityLevel::Finalized, 1);
        osmosis_config.probabilistic_finality = false;
        chains.insert("osmosis".to_string(), osmosis_config);
        
        // Noble
        let mut noble_config = ChainFinalityConfig {
            chain_id: "noble".to_string(),
            ..Default::default()
        };
        noble_config.finality_requirements.insert(FinalityLevel::Confirmed, 1);
        noble_config.finality_requirements.insert(FinalityLevel::Safe, 1);
        noble_config.finality_requirements.insert(FinalityLevel::Justified, 1);
        noble_config.finality_requirements.insert(FinalityLevel::Finalized, 1);
        noble_config.probabilistic_finality = false;
        chains.insert("noble".to_string(), noble_config);
        
        // Neutron
        let mut neutron_config = ChainFinalityConfig {
            chain_id: "neutron".to_string(),
            ..Default::default()
        };
        neutron_config.finality_requirements.insert(FinalityLevel::Confirmed, 1);
        neutron_config.finality_requirements.insert(FinalityLevel::Safe, 1);
        neutron_config.finality_requirements.insert(FinalityLevel::Justified, 1);
        neutron_config.finality_requirements.insert(FinalityLevel::Finalized, 1);
        neutron_config.probabilistic_finality = false;
        chains.insert("neutron".to_string(), neutron_config);
        
        Self {
            chains,
            check_interval: Duration::from_secs(30),
            max_batch_size: 100,
            enable_metrics: true,
            persist_finality_state: true,
        }
    }
}

/// Block finality information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockFinality {
    /// Chain identifier
    pub chain_id: String,
    
    /// Block number
    pub block_number: u64,
    
    /// Block hash
    pub block_hash: String,
    
    /// Current finality level
    pub finality_level: FinalityLevel,
    
    /// Time when block was first seen
    pub first_seen: SystemTime,
    
    /// Time when finality level was last updated
    pub last_updated: SystemTime,
    
    /// Number of confirmations at current block tip
    pub confirmations: u64,
    
    /// Whether this block has been reorganized
    pub reorganized: bool,
}

/// Reorg detection event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgDetected {
    /// Chain identifier
    pub chain_id: String,
    
    /// Reorganized blocks (old chain)
    pub reorganized_blocks: Vec<BlockFinality>,
    
    /// New canonical blocks
    pub new_canonical_blocks: Vec<BlockFinality>,
    
    /// Depth of reorganization
    pub depth: u64,
    
    /// Time when reorg was detected
    pub detected_at: SystemTime,
}

/// Finality state change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityStateChange {
    /// Chain identifier
    pub chain_id: String,
    
    /// Block that changed finality state
    pub block: BlockFinality,
    
    /// Previous finality level
    pub previous_level: Option<FinalityLevel>,
    
    /// New finality level
    pub new_level: FinalityLevel,
    
    /// Time of state change
    pub timestamp: SystemTime,
}

/// Finality processor trait
#[async_trait]
pub trait FinalityProcessor: Send + Sync {
    /// Process a new block for finality
    async fn process_block(&self, chain_id: &str, block_number: u64, block_hash: &str) -> Result<()>;
    
    /// Update chain head for finality calculations
    async fn update_chain_head(&self, chain_id: &str, block_number: u64) -> Result<()>;
    
    /// Get current finality level for a block
    async fn get_finality_level(&self, chain_id: &str, block_number: u64) -> Result<Option<FinalityLevel>>;
    
    /// Check if a block has reached a specific finality level
    async fn is_finalized(&self, chain_id: &str, block_number: u64, level: FinalityLevel) -> Result<bool>;
    
    /// Get all blocks at a specific finality level
    async fn get_blocks_at_level(&self, chain_id: &str, level: FinalityLevel) -> Result<Vec<BlockFinality>>;
    
    /// Subscribe to finality state changes
    async fn subscribe_finality_changes(&self) -> Result<mpsc::Receiver<FinalityStateChange>>;
    
    /// Subscribe to reorg detection events
    async fn subscribe_reorg_events(&self) -> Result<mpsc::Receiver<ReorgDetected>>;
    
    /// Get processing statistics
    async fn get_stats(&self) -> Result<FinalityProcessorStats>;
}

/// Processing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityProcessorStats {
    /// Total blocks processed
    pub total_blocks_processed: u64,
    
    /// Total reorgs detected
    pub total_reorgs_detected: u64,
    
    /// Current finality levels per chain
    pub chain_finality_levels: HashMap<String, HashMap<FinalityLevel, u64>>,
    
    /// Average processing time
    pub avg_processing_time: Duration,
    
    /// Last processing time
    pub last_processed_at: Option<SystemTime>,
}

/// Advanced finality processor implementation
pub struct AdvancedFinalityProcessor {
    /// Configuration
    config: FinalityProcessorConfig,
    
    /// Current chain heads
    chain_heads: Arc<RwLock<HashMap<String, u64>>>,
    
    /// Block finality state
    block_finality: Arc<RwLock<HashMap<String, HashMap<u64, BlockFinality>>>>,
    
    /// Recent blocks for reorg detection
    recent_blocks: Arc<RwLock<HashMap<String, VecDeque<BlockFinality>>>>,
    
    /// Finality state change subscribers
    finality_subscribers: Arc<RwLock<Vec<mpsc::Sender<FinalityStateChange>>>>,
    
    /// Reorg event subscribers
    reorg_subscribers: Arc<RwLock<Vec<mpsc::Sender<ReorgDetected>>>>,
    
    /// Processing statistics
    stats: Arc<RwLock<FinalityProcessorStats>>,
    
    /// Event sender for processing
    event_sender: mpsc::UnboundedSender<ProcessingEvent>,
}

/// Internal processing event
#[derive(Debug, Clone)]
enum ProcessingEvent {
    NewBlock {
        chain_id: String,
        block_number: u64,
        block_hash: String,
    },
    ChainHeadUpdate {
        chain_id: String,
        block_number: u64,
    },
    FinalityCheck {
        chain_id: String,
    },
}

impl AdvancedFinalityProcessor {
    /// Create a new advanced finality processor
    pub fn new(config: FinalityProcessorConfig) -> Result<Self> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let processor = Self {
            config,
            chain_heads: Arc::new(RwLock::new(HashMap::new())),
            block_finality: Arc::new(RwLock::new(HashMap::new())),
            recent_blocks: Arc::new(RwLock::new(HashMap::new())),
            finality_subscribers: Arc::new(RwLock::new(Vec::new())),
            reorg_subscribers: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(FinalityProcessorStats {
                total_blocks_processed: 0,
                total_reorgs_detected: 0,
                chain_finality_levels: HashMap::new(),
                avg_processing_time: Duration::from_millis(0),
                last_processed_at: None,
            })),
            event_sender,
        };
        
        // Start background processing
        let processor_clone = processor.clone();
        tokio::spawn(async move {
            processor_clone.process_events(event_receiver).await;
        });
        
        // Start periodic finality check
        let processor_clone = processor.clone();
        tokio::spawn(async move {
            processor_clone.periodic_finality_check().await;
        });
        
        Ok(processor)
    }
    
    /// Background event processing loop
    async fn process_events(&self, mut event_receiver: mpsc::UnboundedReceiver<ProcessingEvent>) {
        while let Some(event) = event_receiver.recv().await {
            match event {
                ProcessingEvent::NewBlock { chain_id, block_number, block_hash } => {
                    if let Err(e) = self.handle_new_block(&chain_id, block_number, &block_hash).await {
                        tracing::error!("Failed to process new block: {}", e);
                    }
                }
                ProcessingEvent::ChainHeadUpdate { chain_id, block_number } => {
                    if let Err(e) = self.handle_chain_head_update(&chain_id, block_number).await {
                        tracing::error!("Failed to update chain head: {}", e);
                    }
                }
                ProcessingEvent::FinalityCheck { chain_id } => {
                    if let Err(e) = self.handle_finality_check(&chain_id).await {
                        tracing::error!("Failed to perform finality check: {}", e);
                    }
                }
            }
        }
    }
    
    /// Periodic finality check for all chains
    async fn periodic_finality_check(&self) {
        let mut interval = interval(self.config.check_interval);
        
        loop {
            interval.tick().await;
            
            // Get all chain IDs
            let chain_ids: Vec<String> = self.config.chains.keys().cloned().collect();
            
            // Schedule finality checks for all chains
            for chain_id in chain_ids {
                if let Err(e) = self.event_sender.send(ProcessingEvent::FinalityCheck { chain_id }) {
                    tracing::error!("Failed to schedule finality check: {}", e);
                }
            }
        }
    }
    
    /// Handle new block processing
    async fn handle_new_block(&self, chain_id: &str, block_number: u64, block_hash: &str) -> Result<()> {
        let start_time = Instant::now();
        
        // Get chain configuration
        let chain_config = self.config.chains.get(chain_id)
            .ok_or_else(|| Error::generic(format!("Unknown chain: {}", chain_id)))?;
        
        // Create block finality record
        let block_finality = BlockFinality {
            chain_id: chain_id.to_string(),
            block_number,
            block_hash: block_hash.to_string(),
            finality_level: FinalityLevel::Confirmed,
            first_seen: SystemTime::now(),
            last_updated: SystemTime::now(),
            confirmations: 0,
            reorganized: false,
        };
        
        // Check for reorg if enabled
        if chain_config.reorg_detection {
            if let Some(reorg) = self.detect_reorg(chain_id, &block_finality).await? {
                self.handle_reorg(reorg).await?;
            }
        }
        
        // Store block finality
        {
            let mut finality_map = self.block_finality.write().await;
            finality_map.entry(chain_id.to_string())
                .or_default()
                .insert(block_number, block_finality.clone());
        }
        
        // Update recent blocks for reorg detection
        {
            let mut recent = self.recent_blocks.write().await;
            let chain_recent = recent.entry(chain_id.to_string()).or_default();
            chain_recent.push_front(block_finality.clone());
            
            // Keep only recent blocks within max reorg depth
            while chain_recent.len() > chain_config.max_reorg_depth as usize {
                chain_recent.pop_back();
            }
        }
        
        // Update stats
        let processing_time = start_time.elapsed();
        let mut stats = self.stats.write().await;
        stats.total_blocks_processed += 1;
        stats.last_processed_at = Some(SystemTime::now());
        
        // Update average processing time
        if stats.total_blocks_processed == 1 {
            stats.avg_processing_time = processing_time;
        } else {
            let new_avg = Duration::from_nanos(
                (stats.avg_processing_time.as_nanos() as u64 + processing_time.as_nanos() as u64) / 2
            );
            stats.avg_processing_time = new_avg;
        }
        
        Ok(())
    }
    
    /// Handle chain head updates
    async fn handle_chain_head_update(&self, chain_id: &str, block_number: u64) -> Result<()> {
        // Update chain head
        {
            let mut heads = self.chain_heads.write().await;
            heads.insert(chain_id.to_string(), block_number);
        }
        
        // Update finality levels for affected blocks
        self.update_finality_levels(chain_id, block_number).await?;
        
        Ok(())
    }
    
    /// Handle finality check for a chain
    async fn handle_finality_check(&self, chain_id: &str) -> Result<()> {
        let chain_head = {
            let heads = self.chain_heads.read().await;
            heads.get(chain_id).copied().unwrap_or(0)
        };
        
        if chain_head > 0 {
            self.update_finality_levels(chain_id, chain_head).await?;
        }
        
        Ok(())
    }
    
    /// Update finality levels for blocks in a chain
    async fn update_finality_levels(&self, chain_id: &str, chain_head: u64) -> Result<()> {
        let chain_config = self.config.chains.get(chain_id)
            .ok_or_else(|| Error::generic(format!("Unknown chain: {}", chain_id)))?;
        
        let mut finality_map = self.block_finality.write().await;
        let chain_blocks = finality_map.entry(chain_id.to_string()).or_default();
        
        let mut state_changes = Vec::new();
        
        for (block_number, block_finality) in chain_blocks.iter_mut() {
            let confirmations = chain_head.saturating_sub(*block_number);
            
            block_finality.confirmations = confirmations;
            
            // Determine new finality level
            let new_level = self.calculate_finality_level(chain_config, block_finality).await?;
            
            if new_level != block_finality.finality_level {
                let previous_level = Some(block_finality.finality_level);
                block_finality.finality_level = new_level;
                block_finality.last_updated = SystemTime::now();
                
                state_changes.push(FinalityStateChange {
                    chain_id: chain_id.to_string(),
                    block: block_finality.clone(),
                    previous_level,
                    new_level,
                    timestamp: SystemTime::now(),
                });
            }
        }
        
        // Release the lock before notifying subscribers
        drop(finality_map);
        
        // Notify subscribers of state changes
        for state_change in state_changes {
            self.notify_finality_change(state_change).await;
        }
        
        Ok(())
    }
    
    /// Calculate finality level for a block
    async fn calculate_finality_level(&self, config: &ChainFinalityConfig, block: &BlockFinality) -> Result<FinalityLevel> {
        let time_since_creation = SystemTime::now().duration_since(block.first_seen)
            .unwrap_or_default();
        
        // Check each finality level from highest to lowest
        let levels = [
            FinalityLevel::Finalized,
            FinalityLevel::Justified,
            FinalityLevel::Safe,
            FinalityLevel::Confirmed,
        ];
        
        for level in levels {
            let required_confirmations = config.finality_requirements.get(&level).copied().unwrap_or(0);
            let required_time = config.time_requirements.get(&level).copied().unwrap_or_default();
            
            let confirmations_met = block.confirmations >= required_confirmations;
            let time_met = time_since_creation >= required_time;
            
            if confirmations_met && time_met {
                return Ok(level);
            }
        }
        
        Ok(FinalityLevel::Confirmed)
    }
    
    /// Detect reorganization
    async fn detect_reorg(&self, chain_id: &str, new_block: &BlockFinality) -> Result<Option<ReorgDetected>> {
        let recent = self.recent_blocks.read().await;
        let chain_recent = recent.get(chain_id);
        
        if let Some(recent_blocks) = chain_recent {
            // Check if the new block builds on the most recent block
            if let Some(latest_block) = recent_blocks.front() {
                // For now, we assume blocks are sequential
                // In a real implementation, we'd check parent hash
                if new_block.block_number <= latest_block.block_number {
                    // Potential reorg detected
                    let reorganized_blocks: Vec<BlockFinality> = recent_blocks
                        .iter()
                        .filter(|b| b.block_number >= new_block.block_number)
                        .cloned()
                        .collect();
                    
                    if !reorganized_blocks.is_empty() {
                        let depth = reorganized_blocks.len() as u64;
                        return Ok(Some(ReorgDetected {
                            chain_id: chain_id.to_string(),
                            reorganized_blocks,
                            new_canonical_blocks: vec![new_block.clone()],
                            depth,
                            detected_at: SystemTime::now(),
                        }));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Handle reorganization
    async fn handle_reorg(&self, reorg: ReorgDetected) -> Result<()> {
        tracing::warn!("Reorg detected on chain {}: depth {}", reorg.chain_id, reorg.depth);
        
        // Mark reorganized blocks
        {
            let mut finality_map = self.block_finality.write().await;
            if let Some(chain_blocks) = finality_map.get_mut(&reorg.chain_id) {
                for reorganized_block in &reorg.reorganized_blocks {
                    if let Some(block) = chain_blocks.get_mut(&reorganized_block.block_number) {
                        block.reorganized = true;
                        block.last_updated = SystemTime::now();
                    }
                }
            }
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_reorgs_detected += 1;
        }
        
        // Notify subscribers
        self.notify_reorg(reorg).await;
        
        Ok(())
    }
    
    /// Notify finality change subscribers
    async fn notify_finality_change(&self, change: FinalityStateChange) {
        let subscribers = self.finality_subscribers.read().await;
        for subscriber in subscribers.iter() {
            if let Err(e) = subscriber.send(change.clone()).await {
                tracing::debug!("Failed to notify finality change subscriber: {}", e);
            }
        }
    }
    
    /// Notify reorg subscribers
    async fn notify_reorg(&self, reorg: ReorgDetected) {
        let subscribers = self.reorg_subscribers.read().await;
        for subscriber in subscribers.iter() {
            if let Err(e) = subscriber.send(reorg.clone()).await {
                tracing::debug!("Failed to notify reorg subscriber: {}", e);
            }
        }
    }
}

impl Clone for AdvancedFinalityProcessor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            chain_heads: self.chain_heads.clone(),
            block_finality: self.block_finality.clone(),
            recent_blocks: self.recent_blocks.clone(),
            finality_subscribers: self.finality_subscribers.clone(),
            reorg_subscribers: self.reorg_subscribers.clone(),
            stats: self.stats.clone(),
            event_sender: self.event_sender.clone(),
        }
    }
}

#[async_trait]
impl FinalityProcessor for AdvancedFinalityProcessor {
    async fn process_block(&self, chain_id: &str, block_number: u64, block_hash: &str) -> Result<()> {
        self.event_sender.send(ProcessingEvent::NewBlock {
            chain_id: chain_id.to_string(),
            block_number,
            block_hash: block_hash.to_string(),
        }).map_err(|e| Error::generic(format!("Failed to queue block processing: {}", e)))?;
        
        Ok(())
    }
    
    async fn update_chain_head(&self, chain_id: &str, block_number: u64) -> Result<()> {
        self.event_sender.send(ProcessingEvent::ChainHeadUpdate {
            chain_id: chain_id.to_string(),
            block_number,
        }).map_err(|e| Error::generic(format!("Failed to queue chain head update: {}", e)))?;
        
        Ok(())
    }
    
    async fn get_finality_level(&self, chain_id: &str, block_number: u64) -> Result<Option<FinalityLevel>> {
        let finality_map = self.block_finality.read().await;
        Ok(finality_map
            .get(chain_id)
            .and_then(|chain_blocks| chain_blocks.get(&block_number))
            .map(|block| block.finality_level))
    }
    
    async fn is_finalized(&self, chain_id: &str, block_number: u64, level: FinalityLevel) -> Result<bool> {
        let current_level = self.get_finality_level(chain_id, block_number).await?;
        Ok(current_level.map(|l| l >= level).unwrap_or(false))
    }
    
    async fn get_blocks_at_level(&self, chain_id: &str, level: FinalityLevel) -> Result<Vec<BlockFinality>> {
        let finality_map = self.block_finality.read().await;
        Ok(finality_map
            .get(chain_id)
            .map(|chain_blocks| {
                chain_blocks.values()
                    .filter(|block| block.finality_level == level)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }
    
    async fn subscribe_finality_changes(&self) -> Result<mpsc::Receiver<FinalityStateChange>> {
        let (sender, receiver) = mpsc::channel(100);
        self.finality_subscribers.write().await.push(sender);
        Ok(receiver)
    }
    
    async fn subscribe_reorg_events(&self) -> Result<mpsc::Receiver<ReorgDetected>> {
        let (sender, receiver) = mpsc::channel(100);
        self.reorg_subscribers.write().await.push(sender);
        Ok(receiver)
    }
    
    async fn get_stats(&self) -> Result<FinalityProcessorStats> {
        Ok(self.stats.read().await.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_finality_processor_creation() {
        let config = FinalityProcessorConfig::default();
        let processor = AdvancedFinalityProcessor::new(config).unwrap();
        
        // Test basic functionality
        processor.process_block("ethereum", 100, "0xabc123").await.unwrap();
        processor.update_chain_head("ethereum", 110).await.unwrap();
        
        // Give some time for processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let finality_level = processor.get_finality_level("ethereum", 100).await.unwrap();
        assert!(finality_level.is_some());
        
        let stats = processor.get_stats().await.unwrap();
        assert!(stats.total_blocks_processed > 0);
    }
    
    #[tokio::test]
    async fn test_finality_levels() {
        let config = FinalityProcessorConfig::default();
        let processor = AdvancedFinalityProcessor::new(config).unwrap();
        
        // Test basic functionality without relying on background processing
        let result = processor.process_block("ethereum", 100, "0xabc123").await;
        assert!(result.is_ok());
        
        let result = processor.update_chain_head("ethereum", 140).await;
        assert!(result.is_ok());
        
        // Test that we can get stats
        let stats = processor.get_stats().await.unwrap();
        assert_eq!(stats.total_reorgs_detected, 0);
        
        // Test subscription functionality
        let _finality_sub = processor.subscribe_finality_changes().await;
        assert!(_finality_sub.is_ok());
        
        let _reorg_sub = processor.subscribe_reorg_events().await;
        assert!(_reorg_sub.is_ok());
        
        // Test getting blocks at level (should be empty initially)
        let blocks = processor.get_blocks_at_level("ethereum", FinalityLevel::Safe).await.unwrap();
        assert!(blocks.is_empty());
    }
} 