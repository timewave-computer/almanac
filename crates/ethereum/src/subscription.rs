use async_trait::async_trait;
use ethers::providers::{Middleware, Provider, StreamExt, Ws};
use ethers::types::{Block, Filter, Log, Transaction, TransactionReceipt, H256};
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use indexer_common::{Error as CommonError, Result};
use indexer_core::event::Event;
use indexer_core::service::EventSubscription;
use indexer_core::types::ChainId;

use crate::event::{EthereumEvent, EthereumEventProcessor};

/// Ethereum subscription implementation
pub struct EthereumSubscription {
    /// Chain ID
    chain_id: ChainId,
    
    /// Ethereum provider
    provider: Arc<Provider<Ws>>,
    
    /// Block subscription
    block_stream: Mutex<Pin<Box<dyn Stream<Item = Block<H256>> + Send>>>,
    
    /// Event processor
    event_processor: Arc<EthereumEventProcessor>,
}

impl EthereumSubscription {
    /// Create a new Ethereum subscription
    pub async fn new(provider: Arc<Provider<Ws>>, chain_id: ChainId) -> Result<Self> {
        // Clone the provider Arc specifically for the stream subscription
        let provider_for_stream = provider.clone(); 
        let block_stream = provider_for_stream.subscribe_blocks().await
            .map_err(|e| indexer_common::Error::generic(format!("Failed to subscribe to blocks: {}", e)))?;
        
        // The event processor likely needs the chain_id string, not the struct
        let event_processor = EthereumEventProcessor::new(chain_id.0.clone()); 
        
        Ok(Self {
            chain_id,
            provider,
            block_stream: Mutex::new(Box::pin(block_stream)),
            event_processor: Arc::new(event_processor),
        })
    }
    
    /// Get logs for a block
    async fn get_block_logs(&self, block_hash: H256) -> Result<Vec<Log>> {
        let filter = Filter::new()
            .at_block_hash(block_hash)
            .address(Vec::<ethers::types::Address>::new());
        
        let logs = self.provider.get_logs(&filter).await
            .map_err(|e| indexer_common::Error::generic(format!("Failed to get logs: {}", e)))?;
        
        Ok(logs)
    }
    
    /// Get the full block with transactions
    async fn get_full_block(&self, block_hash: H256) -> Result<Block<Transaction>> {
        let block = self.provider.get_block_with_txs(block_hash).await
            .map_err(|e| indexer_common::Error::generic(format!("Failed to get block: {}", e)))?
            .ok_or_else(|| indexer_common::Error::generic("Block not found"))?;
        
        Ok(block)
    }
    
    /// Get transaction receipt
    async fn get_transaction_receipt(&self, tx_hash: H256) -> Result<TransactionReceipt> {
        let receipt = self.provider.get_transaction_receipt(tx_hash).await
            .map_err(|e| indexer_common::Error::generic(format!("Failed to get transaction receipt: {}", e)))?
            .ok_or_else(|| indexer_common::Error::generic("Transaction receipt not found"))?;
        
        Ok(receipt)
    }
    
    /// Get transaction receipts for all transactions in a block
    async fn get_block_receipts(&self, block: &Block<Transaction>) -> Result<Vec<TransactionReceipt>> {
        let mut receipts = Vec::new();
        
        // Get receipts in parallel
        let mut receipt_futures = Vec::new();
        
        for tx in &block.transactions {
            let provider = self.provider.clone();
            let tx_hash = tx.hash;
            
            receipt_futures.push(tokio::spawn(async move {
                provider.get_transaction_receipt(tx_hash).await
            }));
        }
        
        // Await all receipt fetches
        for future in receipt_futures {
            match future.await {
                Ok(receipt_result) => {
                    match receipt_result {
                        Ok(Some(receipt)) => receipts.push(receipt),
                        Ok(None) => {}, // Skip missing receipts
                        Err(e) => {
                            // Log error but continue
                            warn!("Failed to get transaction receipt: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to join receipt task: {}", e);
                }
            }
        }
        
        Ok(receipts)
    }
}

#[async_trait]
impl EventSubscription for EthereumSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        let mut block_stream = self.block_stream.lock().await;
        
        if let Some(block) = block_stream.next().await {
            let block_hash = match block.hash {
                Some(hash) => hash,
                None => return None,
            };
            
            // Get the full block with transactions
            let full_block = match self.get_full_block(block_hash).await {
                Ok(b) => b,
                Err(e) => {
                    error!("Failed to get full block: {}", e);
                    return None;
                }
            };
            
            // Get logs for the block
            let logs = match self.get_block_logs(block_hash).await {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to get logs for block: {}", e);
                    return None;
                }
            };
            
            // If there are no logs, return None so we can move on to the next block
            if logs.is_empty() {
                return None;
            }
            
            // Get transaction receipts (optional)
            // let receipts = match self.get_block_receipts(&full_block).await {
            //     Ok(r) => r,
            //     Err(e) => {
            //         warn!("Failed to get block receipts: {}", e);
            //         Vec::new()
            //     }
            // };
            
            // For now, we'll just return the first log as an event
            // In a real implementation, we would batch process logs and return them one by one
            if let Some(log) = logs.first() {
                let receipt = if let Some(tx_hash) = log.transaction_hash {
                    match self.get_transaction_receipt(tx_hash).await {
                        Ok(r) => Some(r),
                        Err(_) => None,
                    }
                } else {
                    None
                };
                
                let event = EthereumEvent::from_log(
                    log.clone(),
                    full_block,
                    self.chain_id.0.clone(),
                    receipt,
                );
                
                return Some(Box::new(event));
            }
        }
        
        None
    }

    async fn close(&mut self) -> Result<()> {
        // Drop the block stream
        let mut block_stream = self.block_stream.lock().await;
        *block_stream = Box::pin(futures::stream::empty());
        
        Ok(())
    }
} 