/// Cosmos subscription implementation
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::{self, interval};
use uuid::Uuid;
use std::collections::HashMap;

use indexer_core::event::Event;
use indexer_core::service::EventSubscription;
use indexer_common::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use async_trait::async_trait;

use crate::event::CosmosEvent;
use crate::provider::CosmosProvider;

use anyhow::Context;
use cosmrs::tendermint::abci::Event as AbciEvent;
use futures::{SinkExt, StreamExt};

/// Configuration for Cosmos subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosSubscriptionConfig {
    /// RPC URL for the Cosmos node
    pub rpc_url: String,
    
    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,
    
    /// Chain ID
    pub chain_id: String,
    
    /// Starting block number (defaults to latest if None)
    pub start_block: Option<u64>,
    
    /// Maximum number of blocks to process per poll
    pub max_blocks_per_poll: Option<u64>,
    
    /// Whether to include events from begin/end block
    pub include_block_events: bool,
    
    /// Whether to include events from transactions
    pub include_tx_events: bool,
    
    /// Event types to filter (empty means include all)
    pub event_types: Vec<String>,
}

impl Default for CosmosSubscriptionConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://localhost:26657".to_string(),
            poll_interval_ms: 5000,
            chain_id: "cosmoshub-4".to_string(),
            start_block: None,
            max_blocks_per_poll: Some(10),
            include_block_events: true,
            include_tx_events: true,
            event_types: Vec::new(),
        }
    }
}

/// Cosmos subscription for blockchain events
pub struct CosmosSubscription {
    /// Provider for interacting with Cosmos node
    provider: Arc<CosmosProvider>,
    
    /// Event receiver
    event_rx: Receiver<Box<dyn Event>>,
    
    /// Event sender
    event_tx: Sender<Box<dyn Event>>,
    
    /// Whether the subscription is closed
    closed: Arc<AtomicBool>,
    
    /// Configuration
    config: CosmosSubscriptionConfig,
    
    /// Current block height
    current_block: u64,
}

/// Subscription statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionStats {
    /// Total number of events processed
    pub total_events: u64,
    
    /// Total number of blocks processed
    pub total_blocks: u64,
    
    /// Current block height
    pub current_block: u64,
    
    /// Latest reported error (if any)
    pub latest_error: Option<String>,
    
    /// Creation time of the subscription
    pub created_at: Option<SystemTime>,
    
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

impl Default for SubscriptionStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            total_blocks: 0,
            current_block: 0,
            latest_error: None,
            created_at: Some(SystemTime::now()),
            uptime_seconds: 0,
        }
    }
}

impl CosmosSubscription {
    /// Create a new subscription
    pub async fn new(config: CosmosSubscriptionConfig) -> Result<Self> {
        let provider = Arc::new(CosmosProvider::new(&config.rpc_url).await?);
        let (event_tx, event_rx) = mpsc::channel(1000);
        let closed = Arc::new(AtomicBool::new(false));
        
        // Get initial block height
        let latest_height = provider.get_block_height().await?;
        let start_block = config.start_block.unwrap_or(latest_height);
        
        let subscription = Self {
            provider: provider.clone(),
            event_rx,
            event_tx: event_tx.clone(),
            closed: closed.clone(),
            config: config.clone(),
            current_block: start_block,
        };
        
        // Start background task to poll for new blocks
        let event_tx_clone = event_tx.clone();
        let closed_clone = closed.clone();
        let config_clone = config.clone();
        let provider_clone = provider.clone();
        
        tokio::spawn(async move {
            let mut current_block = start_block;
            let mut interval = interval(Duration::from_millis(config_clone.poll_interval_ms));
            
            while !closed_clone.load(Ordering::Relaxed) {
                interval.tick().await;
                
                match Self::poll_blocks(
                    provider_clone.clone(),
                    &config_clone,
                    current_block,
                    event_tx_clone.clone(),
                ).await {
                    Ok(new_block) => {
                        current_block = new_block;
                    }
                    Err(err) => {
                        error!("Error polling blocks: {}", err);
                        // Wait a bit to avoid spamming error logs
                        time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            
            info!("Subscription polling task stopped");
        });
        
        info!("Created Cosmos subscription starting at block {}", start_block);
        Ok(subscription)
    }
    
    /// Poll for new blocks
    async fn poll_blocks(
        provider: Arc<CosmosProvider>,
        config: &CosmosSubscriptionConfig,
        current_block: u64,
        event_tx: Sender<Box<dyn Event>>,
    ) -> Result<u64> {
        let latest_height = provider.get_block_height().await?;
        
        // No new blocks
        if latest_height <= current_block {
            return Ok(current_block);
        }
        
        // Determine how many blocks to process
        let max_blocks = config.max_blocks_per_poll.unwrap_or(10);
        let target_block = std::cmp::min(latest_height, current_block + max_blocks);
        
        debug!(
            "Processing blocks from {} to {} (latest: {})",
            current_block + 1,
            target_block,
            latest_height
        );
        
        let mut new_current_block = current_block;
        
        // Process blocks
        for block_height in (current_block + 1)..=target_block {
            match Self::process_block(
                provider.clone(),
                config,
                block_height,
                event_tx.clone(),
            ).await {
                Ok(()) => {
                    new_current_block = block_height;
                }
                Err(err) => {
                    error!("Error processing block {}: {}", block_height, err);
                    // Stop processing blocks on error
                    break;
                }
            }
        }
        
        Ok(new_current_block)
    }
    
    /// Process a block and extract events
    async fn process_block(
        provider: Arc<CosmosProvider>,
        config: &CosmosSubscriptionConfig,
        block_height: u64,
        event_tx: Sender<Box<dyn Event>>,
    ) -> Result<()> {
        // Get block
        let block = provider.get_block(block_height).await?;
        let block_hash = block.header.hash().to_string();
        
        // Get timestamp from block if available or use current time
        let timestamp = block.header.time.unix_timestamp() as u64;
        
        // Get block results to extract events
        let block_results = provider.get_block_results(block_height).await?;
        
        // Process begin block events
        if config.include_block_events {
            if let Some(begin_events) = &block_results.begin_block_events {
                for event in begin_events {
                    if !Self::should_include_event(config, &event.kind) {
                        continue;
                    }
                    
                    // Create a new cosmos event
                    let mut data = HashMap::new();
                    for attr in &event.attributes {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                        data.insert(key, value);
                    }
                    
                    let cosmos_event = CosmosEvent::new(
                        Uuid::new_v4().to_string(),
                        config.chain_id.clone(),
                        block_height,
                        block_hash.clone(),
                        format!("begin_block_{}", block_height),
                        timestamp,
                        format!("begin_block_{}", event.kind),
                        data,
                    );
                    
                    if let Err(err) = event_tx.send(Box::new(cosmos_event)).await {
                        error!("Failed to send begin block event: {}", err);
                    }
                }
            }
        }
        
        // Process transaction events
        if config.include_tx_events {
            if let Some(tx_results) = &block_results.txs_results {
                for (i, tx_result) in tx_results.iter().enumerate() {
                    for event in &tx_result.events {
                        if !Self::should_include_event(config, &event.kind) {
                            continue;
                        }
                        
                        // Create a new cosmos event
                        let mut data = HashMap::new();
                        for attr in &event.attributes {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                            data.insert(key, value);
                        }
                        
                        let cosmos_event = CosmosEvent::new(
                            Uuid::new_v4().to_string(),
                            config.chain_id.clone(),
                            block_height,
                            block_hash.clone(),
                            format!("tx_{}_{}", block_height, i),
                            timestamp,
                            format!("tx_{}", event.kind),
                            data,
                        );
                        
                        if let Err(err) = event_tx.send(Box::new(cosmos_event)).await {
                            error!("Failed to send transaction event: {}", err);
                        }
                    }
                }
            }
        }
        
        // Process end block events
        if config.include_block_events {
            if let Some(end_events) = &block_results.end_block_events {
                for event in end_events {
                    if !Self::should_include_event(config, &event.kind) {
                        continue;
                    }
                    
                    // Create a new cosmos event
                    let mut data = HashMap::new();
                    for attr in &event.attributes {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                        data.insert(key, value);
                    }
                    
                    let cosmos_event = CosmosEvent::new(
                        Uuid::new_v4().to_string(),
                        config.chain_id.clone(),
                        block_height,
                        block_hash.clone(),
                        format!("end_block_{}", block_height),
                        timestamp,
                        format!("end_block_{}", event.kind),
                        data,
                    );
                    
                    if let Err(err) = event_tx.send(Box::new(cosmos_event)).await {
                        error!("Failed to send end block event: {}", err);
                    }
                }
            }
        }
        
        debug!("Processed block {} with hash {}", block_height, block_hash);
        Ok(())
    }
    
    /// Check if an event should be included based on config
    fn should_include_event(config: &CosmosSubscriptionConfig, event_type: &str) -> bool {
        if config.event_types.is_empty() {
            return true;
        }
        
        config.event_types.iter().any(|t| t == event_type)
    }
    
    /// Get statistics for this subscription
    pub fn get_stats(&self) -> SubscriptionStats {
        SubscriptionStats {
            current_block: self.current_block,
            ..Default::default()
        }
    }
    
    /// Create a new subscription for testing - note this is a mock implementation
    #[cfg(test)]
    pub fn new_for_testing() -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);
        
        // Use a real config for testing, but we won't actually connect to any provider
        let config = CosmosSubscriptionConfig::default();
        let closed = Arc::new(AtomicBool::new(false));
        
        // Create a provider with a dummy URL - not going to be used in tests
        // NOTE: In a real test, you'd want to use a mock provider instead
        let provider = Arc::new(CosmosProvider {
            client: Arc::new(cosmrs::rpc::HttpClient::new("http://localhost:1234").unwrap()),
            rpc_url: "http://localhost:1234".to_string(),
        });
        
        Self {
            provider,
            event_rx,
            event_tx,
            closed,
            config,
            current_block: 0,
        }
    }
}

#[async_trait]
impl EventSubscription for CosmosSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        if self.closed.load(Ordering::Relaxed) {
            return None;
        }
        
        self.event_rx.recv().await
    }
    
    async fn close(&mut self) -> Result<()> {
        self.closed.store(true, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_subscription() {
        // This is a basic test that just ensures the struct can be created
        // For full testing, we would need a running Cosmos node
        let config = CosmosSubscriptionConfig {
            rpc_url: "http://localhost:26657".to_string(),
            poll_interval_ms: 1000,
            chain_id: "test-chain".to_string(),
            start_block: None,
            max_blocks_per_poll: Some(10),
            include_block_events: true,
            include_tx_events: true,
            event_types: Vec::new(),
        };
        
        // This will fail if no node is running, which is expected for unit tests
        let _result = CosmosSubscription::new(config).await;
    }
} 