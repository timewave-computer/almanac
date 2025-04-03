use async_trait::async_trait;
use cosmrs::rpc::HttpClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

use indexer_core::event::Event;
use indexer_core::service::EventSubscription;
use indexer_core::types::ChainId;
use indexer_core::Result;

use crate::event::CosmosEvent;
use crate::provider::CosmosProvider;

/// Cosmos subscription implementation
pub struct CosmosSubscription {
    /// Chain ID
    chain_id: ChainId,
    
    /// Cosmos RPC client
    client: HttpClient,
    
    /// Current block height
    current_height: Mutex<u64>,
    
    /// Poll interval in milliseconds
    poll_interval: Duration,
    
    /// Whether the subscription is closed
    closed: Mutex<bool>,
}

impl CosmosSubscription {
    /// Create a new Cosmos subscription
    pub async fn new(client: HttpClient, chain_id: ChainId) -> Result<Self> {
        // Get the latest block height to start with
        let status = client.status().await
            .map_err(|e| indexer_core::Error::chain(format!("Failed to get status: {}", e)))?;
        
        let latest_height = status.sync_info.latest_block_height.value() as u64;
        
        Ok(Self {
            chain_id,
            client,
            current_height: Mutex::new(latest_height),
            poll_interval: Duration::from_millis(1000), // 1 second by default
            closed: Mutex::new(false),
        })
    }
    
    /// Set the poll interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }
    
    /// Process a new block
    async fn process_block(&self, height: u64) -> Result<Option<CosmosEvent>> {
        // Convert height to cosmrs height
        let height = height.into();
        
        // Get the block
        let block = self.client.block(height).await
            .map_err(|e| indexer_core::Error::chain(format!("Failed to get block: {}", e)))?;
        
        // If there are no transactions, return None
        if block.block.data.is_empty() {
            return Ok(None);
        }
        
        // Get the first transaction for simplicity
        // In a real implementation, we would process all transactions
        let tx = &block.block.data[0];
        let tx_hash = tx.hash();
        
        // Get the transaction result
        let tx_result = self.client.tx(tx_hash, false).await
            .map_err(|e| indexer_core::Error::chain(format!("Failed to get tx result: {}", e)))?;
        
        // Extract the first event for simplicity
        // In a real implementation, we would process all events
        if let Some(event) = tx_result.tx_result.events.first() {
            // Convert attributes to key-value pairs
            let attributes = event.attributes.iter()
                .map(|attr| (attr.key.clone(), attr.value.clone()))
                .collect();
            
            // Create a Cosmos event
            let cosmos_event = CosmosEvent::new(
                self.chain_id.0.clone(),
                block.block.header.height.value() as u64,
                block.block_id.hash.to_string(),
                tx_hash.to_string(),
                block.block.header.time.unix_timestamp() as u64,
                event.type_str.clone(),
                attributes,
                0, // tx_index, would need to be properly set in a real implementation
                serde_json::to_vec(&event).unwrap_or_default(),
            );
            
            return Ok(Some(cosmos_event));
        }
        
        Ok(None)
    }
}

#[async_trait]
impl EventSubscription for CosmosSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        // Check if the subscription is closed
        if *self.closed.lock().await {
            return None;
        }
        
        loop {
            // Wait for the poll interval
            sleep(self.poll_interval).await;
            
            // Get the current height
            let mut current_height = self.current_height.lock().await;
            
            // Try to get the status to see if there are new blocks
            match self.client.status().await {
                Ok(status) => {
                    let latest_height = status.sync_info.latest_block_height.value() as u64;
                    
                    // If there's a new block
                    if latest_height > *current_height {
                        // Increment the current height
                        *current_height += 1;
                        
                        // Process the block
                        match self.process_block(*current_height).await {
                            Ok(Some(event)) => return Some(Box::new(event)),
                            Ok(None) => continue, // No events in this block, try the next one
                            Err(_) => continue,  // Error processing this block, try the next one
                        }
                    }
                },
                Err(_) => {
                    // Error getting status, wait and try again
                    continue;
                }
            }
        }
    }

    async fn close(&mut self) -> Result<()> {
        let mut closed = self.closed.lock().await;
        *closed = true;
        Ok(())
    }
} 