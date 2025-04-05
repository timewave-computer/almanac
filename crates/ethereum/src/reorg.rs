// # Purpose: Detects and handles blockchain reorganizations for Ethereum chains.

use std::sync::Arc;
use std::time::Duration;
use std::pin::Pin;
use futures::{Stream, StreamExt};
use tokio::sync::{mpsc::{self, Receiver, Sender}, Mutex, MutexGuard}; // Keep this combined one
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use ethers::providers::{Middleware, Provider, ProviderError};
use ethers::types::{Block, BlockNumber, H256, U64}; 

use indexer_common::{Error, Result};
use indexer_core::reorg::{
    BoxedReorgDetector, CanonicalBlock, ChainReorgTracker, ReorgConfig, ReorgDetector, ReorgEvent, ReorgSubscription,
};
use indexer_core::types::ChainId;

// use crate::{EthereumProvider, EthereumProviderConfig, EthereumProviderError}; // Defined below
use crate::provider::EthereumProvider; // Correct import

/// Implementation of ReorgDetector for Ethereum chains
// #[derive(Clone)] // Remove Clone if Mutex<ChainReorgTracker> prevents it
pub struct EthereumReorgDetector {
    chain_id: ChainId,
    provider: Arc<EthereumProvider>, // Store Arc directly
    config: ReorgConfig,
    tracker: Mutex<ChainReorgTracker>, // Tracker needs internal mutability
    event_sender: Option<Sender<ReorgEvent>>,
}

// Implement Debug manually if Clone is removed
impl std::fmt::Debug for EthereumReorgDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EthereumReorgDetector")
            .field("chain_id", &self.chain_id)
            // Avoid printing provider details
            .field("config", &self.config)
            // Avoid printing tracker details
            .field("event_sender_exists", &self.event_sender.is_some())
            .finish()
    }
}

impl EthereumReorgDetector {
   pub fn new(chain_id: ChainId, provider: Arc<EthereumProvider>, config: ReorgConfig) -> Self {
        let tracker = ChainReorgTracker::new(chain_id.clone(), config.clone());
        Self {
            chain_id,
            provider,
            config,
            tracker: Mutex::new(tracker),
            event_sender: None, // Initialize later if needed
        }
    }

    // Internal helper to check reorgs based on tracker
    async fn check_reorg_internal(&self, block: &CanonicalBlock) -> Result<Option<ReorgEvent>> {
        let mut tracker_guard = self.tracker.lock().await; // Use async lock
        let reorg_event = tracker_guard.check_reorg(block);
        // Add block inside the lock after checking
        tracker_guard.add_block(block.clone()); 
        Ok(reorg_event)
    }

    // Internal helper to convert ethers block to canonical representation
    async fn block_to_canonical(&self, block_number: u64) -> Result<CanonicalBlock> {
        // get_block_by_number already returns Result, handling not found
        let block = self.provider.get_block_by_number(block_number).await?; 
        
        Ok(CanonicalBlock {
            number: block.number.ok_or_else(|| Error::generic("Block has no number"))?.as_u64(),
            hash: format!("{:?}", block.hash.ok_or_else(|| Error::generic("Block has no hash"))?),
            parent_hash: format!("{:?}", block.parent_hash),
            timestamp: block.timestamp.as_u64(),
        })
    }
}

#[async_trait::async_trait]
impl ReorgDetector for EthereumReorgDetector {
    async fn start(&self) -> Result<()> {
        // Placeholder: Need to re-implement start logic
        info!("Starting EthereumReorgDetector (placeholder)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        // Placeholder: Need to re-implement stop logic
        info!("Stopping EthereumReorgDetector (placeholder)");
        Ok(())
    }

    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    async fn check_reorg(&self, from_block: u64) -> Result<Option<ReorgEvent>> {
        // Placeholder: Need to re-implement check_reorg logic using the tracker
        warn!(from_block=from_block, "check_reorg called (placeholder)");
        Ok(None)
    }

    async fn subscribe(&self) -> Result<Box<dyn ReorgSubscription>> {
        // Placeholder: Need to re-implement subscribe logic
        let (sender, receiver) = mpsc::channel(100);
        // TODO: Manage sender properly
        info!("Subscribing to reorg events (placeholder)");
        Ok(Box::new(EthereumReorgSubscription { receiver }))
    }

    // Note: set_config was removed in core, config should be immutable after creation
    // Re-adding because the trait seems to still require it.
    async fn set_config(&mut self, config: ReorgConfig) -> Result<()> {
        self.config = config.clone();
        // Update the tracker's config as well
        let mut tracker_guard = self.tracker.lock().await;
        tracker_guard.set_config(config);
        Ok(())
    }

    fn config(&self) -> &ReorgConfig {
        &self.config
    }
}

// Manually implement Clone if needed (tracker might prevent derive)
impl Clone for EthereumReorgDetector {
     fn clone(&self) -> Self {
         // Need to handle Mutex<ChainReorgTracker> cloning.
         // For now, create a new tracker, but this might not be desired behavior.
         // Consider if clone is truly needed or if Arc should be used more.
         let tracker = ChainReorgTracker::new(self.chain_id.clone(), self.config.clone());
         EthereumReorgDetector {
             chain_id: self.chain_id.clone(),
             provider: self.provider.clone(),
             config: self.config.clone(),
             tracker: Mutex::new(tracker), // Creates a new tracker!
             event_sender: None, // Cloned detector shouldn't share sender
         }
     }
}


/// Implementation of ReorgSubscription for Ethereum
#[derive(Debug)]
pub struct EthereumReorgSubscription {
    /// Receiver for reorg events
    receiver: Receiver<ReorgEvent>,
}

#[async_trait::async_trait]
impl ReorgSubscription for EthereumReorgSubscription {
    async fn next(&mut self) -> Option<ReorgEvent> {
        self.receiver.recv().await
    }

    async fn close(&mut self) -> Result<()> {
        // No explicit close needed for mpsc receiver
        Ok(())
    }
}

// Placeholder for monitoring logic - needs re-implementation
// This likely belongs in a separate service or task manager that owns the detector.
// async fn monitor_reorgs_task(...) -> Result<()> { ... }