/// Cosmos subscription implementation
use std::sync::Arc;

use async_trait::async_trait;
use indexer_common::Result;
use indexer_core::event::Event;
use indexer_core::service::EventSubscription;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::event::CosmosEvent;
use crate::provider::CosmosProvider;

/// Cosmos subscription configuration
#[derive(Debug, Clone)]
pub struct CosmosSubscriptionConfig {
    /// RPC URL
    pub rpc_url: String,
    
    /// Polling interval in milliseconds
    pub polling_interval_ms: u64,
    
    /// Chain ID
    pub chain_id: String,
}

/// Cosmos event subscription
pub struct CosmosSubscription {
    /// Provider for interacting with the Cosmos node
    provider: Arc<CosmosProvider>,
    
    /// Event receiver
    event_receiver: mpsc::Receiver<Box<dyn Event>>,
    
    /// Event sender
    event_sender: mpsc::Sender<Box<dyn Event>>,
    
    /// Whether the subscription is closed
    closed: bool,
}

impl CosmosSubscription {
    /// Create a new subscription
    pub async fn new(config: CosmosSubscriptionConfig) -> Result<Self> {
        // Create channel for events
        let (event_sender, event_receiver) = mpsc::channel(100);
        
        // Create provider
        let provider = Arc::new(CosmosProvider::new(config.rpc_url).await?);
        let provider_clone = provider.clone();
        
        // Get the initial block height
        let mut last_height = provider.get_block_height().await?;
        info!("Starting Cosmos subscription from block {}", last_height);
        
        // Set up polling task
        let interval_ms = config.polling_interval_ms;
        let event_sender_clone = event_sender.clone();
        
        // Start background task to poll for new blocks
        tokio::spawn(async move {
            let mut interval = interval(std::time::Duration::from_millis(interval_ms));
            
            loop {
                interval.tick().await;
                
                // Get the latest block height
                match provider_clone.get_block_height().await {
                    Ok(current_height) => {
                        if current_height > last_height {
                            debug!("New blocks found: {} -> {}", last_height, current_height);
                            
                            // Process each new block
                            for height in (last_height + 1)..=current_height {
                                if let Err(e) = Self::process_block(&provider_clone, height, &event_sender_clone).await {
                                    warn!("Failed to process block {}: {}", height, e);
                                }
                            }
                            
                            last_height = current_height;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get latest block height: {}", e);
                    }
                }
            }
        });
        
        Ok(Self {
            provider,
            event_receiver,
            event_sender,
            closed: false,
        })
    }
    
    /// Process a block and extract events
    async fn process_block(
        provider: &CosmosProvider, 
        block_height: u64,
        event_sender: &mpsc::Sender<Box<dyn Event>>
    ) -> Result<()> {
        debug!("Processing block {}", block_height);
        
        // Get block from the provider
        let _block = provider.get_block(block_height).await?;
        
        // For now, just create a single mock event for testing
        let event = CosmosEvent::new_mock();
        
        // Send event to the channel
        if let Err(e) = event_sender.send(Box::new(event)).await {
            warn!("Failed to send event: {}", e);
        }
        
        Ok(())
    }
}

#[async_trait]
impl EventSubscription for CosmosSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        if self.closed {
            return None;
        }
        
        self.event_receiver.recv().await
    }
    
    async fn close(&mut self) -> Result<()> {
        self.closed = true;
        Ok(())
    }
} 