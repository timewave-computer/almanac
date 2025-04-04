// Purpose: Ethereum-specific implementation of chain reorganization handling

use async_trait::async_trait;
use ethers::providers::{Http, Middleware, Provider, StreamExt, Ws};
use ethers::types::{BlockNumber, U64};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;

use indexer_core::reorg::{
    BoxedReorgDetector, CanonicalBlock, ReorgConfig, ReorgDetector, ReorgEvent, ReorgSubscription,
};
use indexer_core::types::ChainId;
use indexer_core::{Error, Result};

use crate::provider::EthereumProvider;

/// Implementation of ReorgDetector for Ethereum chains
pub struct EthereumReorgDetector {
    /// Chain identifier
    chain_id: ChainId,
    
    /// Ethereum provider
    provider: EthereumProvider,
    
    /// Configuration for reorganization handling
    config: ReorgConfig,
    
    /// Recent blocks for detecting reorganizations
    recent_blocks: Arc<Mutex<VecDeque<CanonicalBlock>>>,
    
    /// Sender for broadcasting reorg events
    event_sender: Option<Sender<ReorgEvent>>,
    
    /// Task handle for the reorg detection background process
    detection_task: Option<JoinHandle<()>>,
    
    /// Maximum number of blocks to keep in memory
    max_blocks: usize,
}

impl EthereumReorgDetector {
    /// Create a new Ethereum reorg detector
    pub async fn new(chain_id: ChainId, provider: EthereumProvider, config: ReorgConfig) -> Result<Self> {
        let max_blocks = (config.max_depth as usize) * 2;
        
        Ok(Self {
            chain_id,
            provider,
            config,
            recent_blocks: Arc::new(Mutex::new(VecDeque::with_capacity(max_blocks))),
            event_sender: None,
            detection_task: None,
            max_blocks,
        })
    }
    
    /// Convert an Ethereum block to a canonical block
    async fn to_canonical_block(&self, block_num: u64) -> Result<CanonicalBlock> {
        let block = match &self.provider {
            EthereumProvider::Websocket(provider) => {
                provider
                    .get_block(BlockNumber::Number(block_num.into()))
                    .await
                    .map_err(|e| Error::chain(format!("Failed to get block: {}", e)))?
                    .ok_or_else(|| Error::chain(format!("Block {} not found", block_num)))?
            }
            EthereumProvider::Http(provider) => {
                provider
                    .get_block(BlockNumber::Number(block_num.into()))
                    .await
                    .map_err(|e| Error::chain(format!("Failed to get block: {}", e)))?
                    .ok_or_else(|| Error::chain(format!("Block {} not found", block_num)))?
            }
        };
        
        Ok(CanonicalBlock {
            number: block_num,
            hash: block.hash.unwrap_or_default().to_string(),
            parent_hash: block.parent_hash.to_string(),
            timestamp: block.timestamp.as_u64(),
        })
    }
    
    /// Add a new block to the tracker
    fn add_block(&self, block: CanonicalBlock) -> Result<()> {
        let mut blocks = self.recent_blocks.lock().map_err(|_| {
            Error::internal("Failed to acquire lock on recent blocks")
        })?;
        
        // Add new block to the front
        blocks.push_front(block);
        
        // Remove oldest block if we've exceeded the maximum
        if blocks.len() > self.max_blocks {
            blocks.pop_back();
        }
        
        Ok(())
    }
    
    /// Check if a reorganization has occurred
    fn check_reorg_internal(&self, new_block: &CanonicalBlock) -> Result<Option<ReorgEvent>> {
        let blocks = self.recent_blocks.lock().map_err(|_| {
            Error::internal("Failed to acquire lock on recent blocks")
        })?;
        
        // If this is the first block or the new block builds on the previous tip, no reorg
        if blocks.is_empty() || new_block.parent_hash == blocks.front().unwrap().hash {
            return Ok(None);
        }
        
        // Find the common ancestor
        let mut reorganized_blocks = Vec::new();
        let mut common_ancestor_found = false;
        let mut depth = 0;
        
        for (i, old_block) in blocks.iter().enumerate() {
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
            reorganized_blocks.push(indexer_core::reorg::ReorganizedBlock {
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
            return Ok(None);
        }
        
        // Create and return the reorganization event
        Ok(Some(ReorgEvent {
            chain_id: self.chain_id.clone(),
            reorganized_blocks,
            canonical_tip: new_block.clone(),
        }))
    }
    
    /// Background task for monitoring reorganizations
    async fn monitor_reorgs(
        detector: Arc<Mutex<Self>>,
        mut receiver: Receiver<()>,
    ) -> Result<()> {
        let chain_id = {
            let detector_guard = detector.lock().map_err(|_| {
                Error::internal("Failed to acquire lock on detector")
            })?;
            detector_guard.chain_id.clone()
        };
        
        // Get the initial block number
        let initial_block = {
            let detector_guard = detector.lock().map_err(|_| {
                Error::internal("Failed to acquire lock on detector")
            })?;
            
            match &detector_guard.provider {
                EthereumProvider::Websocket(provider) => {
                    provider.get_block_number().await.map_err(|e| {
                        Error::chain(format!("Failed to get block number: {}", e))
                    })?
                }
                EthereumProvider::Http(provider) => {
                    provider.get_block_number().await.map_err(|e| {
                        Error::chain(format!("Failed to get block number: {}", e))
                    })?
                }
            }
        };
        
        let mut current_block = initial_block.as_u64();
        
        // Process the initial block
        {
            let detector_guard = detector.lock().map_err(|_| {
                Error::internal("Failed to acquire lock on detector")
            })?;
            
            // Get the block details
            let block = detector_guard.to_canonical_block(current_block).await?;
            
            // Add it to the tracker
            detector_guard.add_block(block)?;
        }
        
        // Subscribe to new blocks
        let block_stream = match {
            let detector_guard = detector.lock().map_err(|_| {
                Error::internal("Failed to acquire lock on detector")
            })?;
            detector_guard.provider.clone()
        } {
            EthereumProvider::Websocket(provider) => {
                let stream = provider.subscribe_blocks().await.map_err(|e| {
                    Error::chain(format!("Failed to subscribe to blocks: {}", e))
                })?;
                Some(stream)
            }
            EthereumProvider::Http(_) => None,
        };
        
        if let Some(mut stream) = block_stream {
            // Process new blocks from the stream
            loop {
                tokio::select! {
                    // Check if we should stop
                    _ = receiver.recv() => {
                        break;
                    }
                    
                    // Process new block
                    block_opt = stream.next() => {
                        if let Some(block) = block_opt {
                            let block_number = block.number.unwrap_or_default().as_u64();
                            
                            // Create a canonical block representation
                            let canonical_block = CanonicalBlock {
                                number: block_number,
                                hash: block.hash.unwrap_or_default().to_string(),
                                parent_hash: block.parent_hash.to_string(),
                                timestamp: block.timestamp.as_u64(),
                            };
                            
                            // Check for reorganization
                            let detector_guard = detector.lock().map_err(|_| {
                                Error::internal("Failed to acquire lock on detector")
                            })?;
                            
                            if let Some(reorg_event) = detector_guard.check_reorg_internal(&canonical_block)? {
                                // If there's a reorg, broadcast it
                                if let Some(sender) = &detector_guard.event_sender {
                                    if let Err(e) = sender.send(reorg_event).await {
                                        // Just log the error and continue
                                        eprintln!("Failed to send reorg event: {}", e);
                                    }
                                }
                            }
                            
                            // Add the block to the tracker
                            detector_guard.add_block(canonical_block)?;
                        } else {
                            // Stream ended
                            break;
                        }
                    }
                }
            }
        } else {
            // For HTTP providers, we need to poll for new blocks periodically
            loop {
                tokio::select! {
                    // Check if we should stop
                    _ = receiver.recv() => {
                        break;
                    }
                    
                    // Poll for new blocks every 5 seconds
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                        let detector_guard = detector.lock().map_err(|_| {
                            Error::internal("Failed to acquire lock on detector")
                        })?;
                        
                        let latest_block = match &detector_guard.provider {
                            EthereumProvider::Websocket(provider) => {
                                provider.get_block_number().await.map_err(|e| {
                                    Error::chain(format!("Failed to get block number: {}", e))
                                })?
                            }
                            EthereumProvider::Http(provider) => {
                                provider.get_block_number().await.map_err(|e| {
                                    Error::chain(format!("Failed to get block number: {}", e))
                                })?
                            }
                        };
                        
                        let latest_block_num = latest_block.as_u64();
                        
                        // Process any new blocks
                        for block_num in (current_block + 1)..=latest_block_num {
                            // Get the block details
                            let block = detector_guard.to_canonical_block(block_num).await?;
                            
                            // Check for reorganization
                            if let Some(reorg_event) = detector_guard.check_reorg_internal(&block)? {
                                // If there's a reorg, broadcast it
                                if let Some(sender) = &detector_guard.event_sender {
                                    if let Err(e) = sender.send(reorg_event).await {
                                        // Just log the error and continue
                                        eprintln!("Failed to send reorg event: {}", e);
                                    }
                                }
                            }
                            
                            // Add it to the tracker
                            detector_guard.add_block(block)?;
                        }
                        
                        current_block = latest_block_num;
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl ReorgDetector for EthereumReorgDetector {
    async fn start(&self) -> Result<()> {
        let detector = Arc::new(Mutex::new(self.clone()));
        let (stop_sender, stop_receiver) = mpsc::channel(1);
        let (event_sender, _) = mpsc::channel(100);
        
        {
            let mut detector_guard = detector.lock().map_err(|_| {
                Error::internal("Failed to acquire lock on detector")
            })?;
            detector_guard.event_sender = Some(event_sender);
        }
        
        let task = tokio::spawn(async move {
            if let Err(e) = Self::monitor_reorgs(detector, stop_receiver).await {
                eprintln!("Reorg monitoring error: {}", e);
            }
        });
        
        {
            let mut detector_guard = detector.lock().map_err(|_| {
                Error::internal("Failed to acquire lock on detector")
            })?;
            detector_guard.detection_task = Some(task);
        }
        
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        // TODO: Implement stop functionality
        Ok(())
    }
    
    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }
    
    async fn check_reorg(&self, from_block: u64) -> Result<Option<ReorgEvent>> {
        // Get the block details
        let block = self.to_canonical_block(from_block).await?;
        
        // Check for reorganization
        self.check_reorg_internal(&block)
    }
    
    async fn subscribe(&self) -> Result<Box<dyn ReorgSubscription>> {
        let (sender, receiver) = mpsc::channel(100);
        
        // Add the sender to our list
        if let Some(existing_sender) = &self.event_sender {
            let _ = existing_sender.clone();
            // TODO: Implement proper subscription management
        }
        
        Ok(Box::new(EthereumReorgSubscription { receiver }))
    }
    
    async fn set_config(&mut self, config: ReorgConfig) -> Result<()> {
        self.config = config;
        self.max_blocks = (self.config.max_depth as usize) * 2;
        Ok(())
    }
    
    fn config(&self) -> &ReorgConfig {
        &self.config
    }
}

impl Clone for EthereumReorgDetector {
    fn clone(&self) -> Self {
        Self {
            chain_id: self.chain_id.clone(),
            provider: self.provider.clone(),
            config: self.config.clone(),
            recent_blocks: self.recent_blocks.clone(),
            event_sender: None, // Don't clone the sender
            detection_task: None, // Don't clone the task
            max_blocks: self.max_blocks,
        }
    }
}

/// Implementation of ReorgSubscription for Ethereum
pub struct EthereumReorgSubscription {
    /// Receiver for reorg events
    receiver: Receiver<ReorgEvent>,
}

#[async_trait]
impl ReorgSubscription for EthereumReorgSubscription {
    async fn next(&mut self) -> Option<ReorgEvent> {
        self.receiver.recv().await
    }
    
    async fn close(&mut self) -> Result<()> {
        // No need to do anything, the channel will be closed when this struct is dropped
        Ok(())
    }
} 