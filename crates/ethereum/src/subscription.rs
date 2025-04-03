use async_trait::async_trait;
use ethers::providers::{Middleware, Provider, StreamExt, Ws};
use ethers::types::{Block, Filter, Log, Transaction, TransactionReceipt, H256};
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

use indexer_core::event::Event;
use indexer_core::service::EventSubscription;
use indexer_core::types::ChainId;
use indexer_core::Result;

use crate::event::EthereumEvent;

/// Ethereum subscription implementation
pub struct EthereumSubscription {
    /// Chain ID
    chain_id: ChainId,
    
    /// Ethereum provider
    provider: Provider<Ws>,
    
    /// Block subscription
    block_stream: Mutex<Pin<Box<dyn Stream<Item = Block<H256>> + Send>>>,
}

impl EthereumSubscription {
    /// Create a new Ethereum subscription
    pub async fn new(provider: Provider<Ws>, chain_id: ChainId) -> Result<Self> {
        let block_stream = provider.subscribe_blocks().await
            .map_err(|e| indexer_core::Error::chain(format!("Failed to subscribe to blocks: {}", e)))?;
        
        Ok(Self {
            chain_id,
            provider,
            block_stream: Mutex::new(Box::pin(block_stream)),
        })
    }
    
    /// Get logs for a block
    async fn get_block_logs(&self, block_hash: H256) -> Result<Vec<Log>> {
        let filter = Filter::new()
            .at_block_hash(block_hash)
            .address(Vec::<ethers::types::Address>::new());
        
        let logs = self.provider.get_logs(&filter).await
            .map_err(|e| indexer_core::Error::chain(format!("Failed to get logs: {}", e)))?;
        
        Ok(logs)
    }
    
    /// Get the full block with transactions
    async fn get_full_block(&self, block_hash: H256) -> Result<Block<Transaction>> {
        let block = self.provider.get_block_with_txs(block_hash).await
            .map_err(|e| indexer_core::Error::chain(format!("Failed to get block: {}", e)))?
            .ok_or_else(|| indexer_core::Error::chain("Block not found"))?;
        
        Ok(block)
    }
    
    /// Get transaction receipt
    async fn get_transaction_receipt(&self, tx_hash: H256) -> Result<TransactionReceipt> {
        let receipt = self.provider.get_transaction_receipt(tx_hash).await
            .map_err(|e| indexer_core::Error::chain(format!("Failed to get transaction receipt: {}", e)))?
            .ok_or_else(|| indexer_core::Error::chain("Transaction receipt not found"))?;
        
        Ok(receipt)
    }
}

#[async_trait]
impl EventSubscription for EthereumSubscription {
    async fn next(&mut self) -> Option<Box<dyn Event>> {
        let mut block_stream = self.block_stream.lock().await;
        
        if let Some(block) = block_stream.next().await {
            // Got a new block notification, now fetch the full block with transactions
            if let Ok(full_block) = self.get_full_block(block.hash.unwrap_or_default()).await {
                // Get logs for the block
                if let Ok(logs) = self.get_block_logs(block.hash.unwrap_or_default()).await {
                    if !logs.is_empty() {
                        // If we have logs, create an event from the first log
                        // In a real implementation, we would process all logs,
                        // but for simplicity, we just use the first one here
                        let log = logs[0].clone();
                        
                        // Optionally get the transaction receipt for more details
                        let receipt = if let Some(tx_hash) = log.transaction_hash {
                            self.get_transaction_receipt(tx_hash).await.ok()
                        } else {
                            None
                        };
                        
                        // Create an Ethereum event from the log and block
                        let event = EthereumEvent::from_log(
                            log,
                            full_block,
                            self.chain_id.0.clone(),
                            receipt,
                        );
                        
                        return Some(Box::new(event));
                    }
                }
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