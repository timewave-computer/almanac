use ethers::prelude::*;
use ethers::providers::{Http, Provider, Ws};
use ethers::middleware::Middleware;
use ethers::types::{BlockId, BlockNumber, Transaction, U64, H256, Block};
use indexer_common::{Error, Result};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use futures::StreamExt;

/// Ethereum provider types
#[derive(Debug, Clone)]
pub enum EthereumProvider {
    /// HTTP provider
    Http(Arc<Provider<Http>>),
    
    /// WebSocket provider
    Websocket(Arc<Provider<Ws>>),
}

/// Block status in Ethereum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStatus {
    /// Block is included in chain (may be reversible)
    Confirmed,
    
    /// Block has enough attestations to be unlikely to be orphaned
    Safe,
    
    /// Block has been voted on by validators in current epoch
    Justified,
    
    /// Block has been irreversibly agreed upon
    Finalized,
}

/// Configuration for the Ethereum provider
#[derive(Debug, Clone)]
pub struct EthereumProviderConfig {
    /// RPC URL
    pub rpc_url: String,
    
    /// Whether to use WebSocket
    pub use_websocket: bool,
    
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,
    
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    
    /// Number of retry attempts
    pub retry_attempts: usize,
}

impl Default for EthereumProviderConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://localhost:8545".to_string(),
            use_websocket: false,
            max_concurrent_requests: 10,
            request_timeout_secs: 30,
            retry_attempts: 3,
        }
    }
}

impl EthereumProvider {
    /// Create a new Ethereum provider
    pub async fn new(config: EthereumProviderConfig) -> Result<Self> {
        if config.use_websocket {
            let ws_provider = Provider::<Ws>::connect(&config.rpc_url).await
                .map_err(|e| Error::generic(format!("Failed to connect to Ethereum node via WebSocket: {}", e)))?;
            
            Ok(Self::Websocket(Arc::new(ws_provider)))
        } else {
            let http_provider = Provider::<Http>::try_from(&config.rpc_url)
                .map_err(|e| Error::generic(format!("Failed to create Ethereum HTTP provider: {}", e)))?;
            
            Ok(Self::Http(Arc::new(http_provider)))
        }
    }

    /// Get a block by number with a specific status requirement
    pub async fn get_block_by_status(&self, status: BlockStatus) -> Result<(ethers::types::Block<ethers::types::Transaction>, u64)> {
        let block_number_enum = match status {
            BlockStatus::Confirmed => BlockNumber::Latest,
            BlockStatus::Safe => BlockNumber::Safe,
            BlockStatus::Justified => BlockNumber::Finalized,
            BlockStatus::Finalized => BlockNumber::Finalized,
        };
        
        let block = match self {
            EthereumProvider::Http(provider) => {
                provider
                    .get_block_with_txs(BlockId::Number(block_number_enum))
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block: {}", e)))?
                    .ok_or_else(|| Error::generic(format!("Block with status {:?} not found", status)))?
            }
            EthereumProvider::Websocket(provider) => {
                provider
                    .get_block_with_txs(BlockId::Number(block_number_enum))
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block: {}", e)))?
                    .ok_or_else(|| Error::generic(format!("Block with status {:?} not found", status)))?
            }
        };
        
        let block_number = block.number.unwrap_or_default().as_u64();
        Ok((block, block_number))
    }
    
    /// Get latest finalized block number
    pub async fn get_finalized_block_number(&self) -> Result<u64> {
        match self {
            EthereumProvider::Http(provider) => {
                let block = provider
                    .get_block(BlockId::Number(BlockNumber::Finalized))
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get finalized block: {}", e)))?
                    .ok_or_else(|| Error::generic("Finalized block not found"))?;
                
                let number = block.number
                    .ok_or_else(|| Error::generic("Finalized block has no number"))?
                    .as_u64();
                
                Ok(number)
            }
            EthereumProvider::Websocket(provider) => {
                let block = provider
                    .get_block(BlockId::Number(BlockNumber::Finalized))
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get finalized block: {}", e)))?
                    .ok_or_else(|| Error::generic("Finalized block not found"))?;
                
                let number = block.number
                    .ok_or_else(|| Error::generic("Finalized block has no number"))?
                    .as_u64();
                
                Ok(number)
            }
        }
    }
    
    /// Get latest safe block number
    pub async fn get_safe_block_number(&self) -> Result<u64> {
        match self {
            EthereumProvider::Http(provider) => {
                let block = provider
                    .get_block(BlockId::Number(BlockNumber::Safe))
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get safe block: {}", e)))?
                    .ok_or_else(|| Error::generic("Safe block not found"))?;
                
                let number = block.number
                    .ok_or_else(|| Error::generic("Safe block has no number"))?
                    .as_u64();
                
                Ok(number)
            }
            EthereumProvider::Websocket(provider) => {
                let block = provider
                    .get_block(BlockId::Number(BlockNumber::Safe))
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get safe block: {}", e)))?
                    .ok_or_else(|| Error::generic("Safe block not found"))?;
                
                let number = block.number
                    .ok_or_else(|| Error::generic("Safe block has no number"))?
                    .as_u64();
                
                Ok(number)
            }
        }
    }
    
    /// Get latest block number
    pub async fn get_latest_block_number(&self) -> Result<u64> {
        match self {
            EthereumProvider::Http(provider) => {
                let block_number = provider.get_block_number().await
                    .map_err(|e| Error::generic(format!("Failed to get latest block number: {}", e)))?;
                
                Ok(block_number.as_u64())
            }
            EthereumProvider::Websocket(provider) => {
                let block_number = provider.get_block_number().await
                    .map_err(|e| Error::generic(format!("Failed to get latest block number: {}", e)))?;
                
                Ok(block_number.as_u64())
            }
        }
    }
    
    /// Get block by number
    pub async fn get_block_by_number(&self, number: u64) -> Result<ethers::types::Block<Transaction>> {
        match self {
            EthereumProvider::Http(provider) => {
                provider
                    .get_block_with_txs(BlockId::Number(number.into()))
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block {}: {}", number, e)))?
                    .ok_or_else(|| Error::generic(format!("Block {} not found", number)))
            }
            EthereumProvider::Websocket(provider) => {
                provider
                    .get_block_with_txs(BlockId::Number(number.into()))
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block {}: {}", number, e)))?
                    .ok_or_else(|| Error::generic(format!("Block {} not found", number)))
            }
        }
    }
    
    /// Get blocks in a range
    pub async fn get_blocks_in_range(&self, from_block: u64, to_block: u64) -> Result<Vec<ethers::types::Block<Transaction>>> {
        if from_block > to_block {
            return Err(Error::generic(format!("from_block must be less than or equal to to_block")));
        }
        
        let batch_size = 10; // Fetch 10 blocks at a time to avoid overloading the node
        let mut blocks = Vec::with_capacity((to_block - from_block + 1) as usize);
        
        for batch_start in (from_block..=to_block).step_by(batch_size) {
            let batch_end = std::cmp::min(batch_start + batch_size as u64 - 1, to_block);
            let mut batch_futures = Vec::with_capacity(batch_size);
            
            for block_num in batch_start..=batch_end {
                match self {
                    EthereumProvider::Http(provider) => {
                        let provider_clone = provider.clone();
                        batch_futures.push(tokio::spawn(async move {
                            provider_clone
                                .get_block_with_txs(BlockId::Number(block_num.into()))
                                .await
                                .map_err(|e| Error::generic(format!("Failed to get block {}: {}", block_num, e)))
                                .and_then(|block_opt| {
                                    block_opt.ok_or_else(|| Error::generic(format!("Block {} not found", block_num)))
                                })
                        }));
                    }
                    EthereumProvider::Websocket(provider) => {
                        let provider_clone = provider.clone();
                        batch_futures.push(tokio::spawn(async move {
                            provider_clone
                                .get_block_with_txs(BlockId::Number(block_num.into()))
                                .await
                                .map_err(|e| Error::generic(format!("Failed to get block {}: {}", block_num, e)))
                                .and_then(|block_opt| {
                                    block_opt.ok_or_else(|| Error::generic(format!("Block {} not found", block_num)))
                                })
                        }));
                    }
                }
            }
            
            // Await all block fetch tasks
            for task in batch_futures {
                match task.await {
                    Ok(block_result) => {
                        match block_result {
                            Ok(block) => blocks.push(block),
                            Err(e) => {
                                error!("Error fetching block: {}", e);
                                // Don't fail the entire batch if one block fails
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Task join error: {}", e);
                        continue;
                    }
                }
            }
            
            // Small delay to avoid overwhelming the node
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        
        Ok(blocks)
    }

    // Get block details by number (e.g., Latest, Safe, Finalized)
    async fn get_block_by_number_enum(&self, block_number: BlockNumber) -> Result<Option<Block<H256>>> {
        let block_id = BlockId::Number(block_number);
        match self {
            EthereumProvider::Websocket(provider) => {
                provider
                    .get_block(block_id)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block by number enum: {}", e)))
            }
            EthereumProvider::Http(provider) => {
                provider
                    .get_block(block_id)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block by number enum: {}", e)))
            }
        }
    }

    // Get block details with transactions by number enum
    async fn get_block_with_txs_by_number_enum(&self, block_number: BlockNumber) -> Result<Option<Block<Transaction>>> {
        let block_id = BlockId::Number(block_number);
        match self {
            EthereumProvider::Websocket(provider) => {
                provider
                    .get_block_with_txs(block_id)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block with txs by number enum: {}", e)))
            }
            EthereumProvider::Http(provider) => {
                provider
                    .get_block_with_txs(block_id)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block with txs by number enum: {}", e)))
            }
        }
    }

    // Renamed methods previously using BlockTag
    async fn get_block(&self, block_number: BlockNumber) -> Result<Option<Block<H256>>> {
        let block_id = BlockId::Number(block_number);
        match self {
            EthereumProvider::Websocket(provider) => {
                provider
                    .get_block(block_id)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block by number enum: {}", e)))
            }
            EthereumProvider::Http(provider) => {
                provider
                    .get_block(block_id)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block by number enum: {}", e)))
            }
        }
    }

    async fn get_block_with_txs(&self, block_number: BlockNumber) -> Result<Option<Block<Transaction>>> {
        let block_id = BlockId::Number(block_number);
        match &self {
            EthereumProvider::Websocket(provider) => {
                provider
                    .get_block_with_txs(block_id)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block with txs by number enum: {}", e)))
            }
            EthereumProvider::Http(provider) => {
                provider
                    .get_block_with_txs(block_id)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to get block with txs by number enum: {}", e)))
            }
        }
    }

    async fn get_block_number(&self) -> Result<u64> {
        match &self {
            EthereumProvider::Websocket(provider) => provider
                .get_block_number()
                .await
                .map_err(|e| Error::generic(format!("Websocket provider error: {}", e)))
                .map(|n| n.as_u64()),
            EthereumProvider::Http(provider) => provider
                .get_block_number()
                .await
                .map_err(|e| Error::generic(format!("HTTP provider error: {}", e)))
                .map(|n| n.as_u64()),
        }
    }

    async fn get_block_with_txs_by_number(&self, block_number: u64) -> Result<Option<Block<Transaction>>> {
        let block_id = BlockId::Number(block_number.into());
        match &self {
            EthereumProvider::Websocket(provider) => provider
                .get_block_with_txs(block_id)
                .await
                .map_err(|e| Error::generic(format!("Websocket provider error: {}", e))),
            EthereumProvider::Http(provider) => provider
                .get_block_with_txs(block_id)
                .await
                .map_err(|e| Error::generic(format!("HTTP provider error: {}", e))),
        }
    }
} 