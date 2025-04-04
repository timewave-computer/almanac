use ethers::providers::Provider;
use ethers::providers::{Http, Ws};
use ethers::types::{BlockId, BlockNumber, BlockTag, U64};
use indexer_core::{Error, Result};

/// Ethereum provider types
pub enum EthereumProvider {
    /// HTTP provider
    Http(Provider<Http>),
    
    /// WebSocket provider
    Websocket(Provider<Ws>),
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

impl EthereumProvider {
    /// Get a block by number with a specific status requirement
    pub async fn get_block_by_status(&self, status: BlockStatus) -> Result<(ethers::types::Block<ethers::types::Transaction>, u64)> {
        let block_tag = match status {
            BlockStatus::Confirmed => BlockTag::Latest,
            BlockStatus::Safe => BlockTag::Safe,
            BlockStatus::Justified => BlockTag::Finalized, // Ethereum RPC doesn't expose "justified" directly
            BlockStatus::Finalized => BlockTag::Finalized,
        };
        
        let block = match self {
            EthereumProvider::Http(provider) => {
                provider
                    .get_block_with_txs(BlockId::Number(BlockNumber::BlockTag(block_tag)))
                    .await
                    .map_err(|e| Error::chain(format!("Failed to get block: {}", e)))?
                    .ok_or_else(|| Error::chain(format!("Block with status {:?} not found", status)))?
            }
            EthereumProvider::Websocket(provider) => {
                provider
                    .get_block_with_txs(BlockId::Number(BlockNumber::BlockTag(block_tag)))
                    .await
                    .map_err(|e| Error::chain(format!("Failed to get block: {}", e)))?
                    .ok_or_else(|| Error::chain(format!("Block with status {:?} not found", status)))?
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
                    .get_block(BlockNumber::Finalized)
                    .await
                    .map_err(|e| Error::chain(format!("Failed to get finalized block: {}", e)))?
                    .ok_or_else(|| Error::chain("Finalized block not found"))?;
                
                let number = block.number
                    .ok_or_else(|| Error::chain("Finalized block has no number"))?
                    .as_u64();
                
                Ok(number)
            }
            EthereumProvider::Websocket(provider) => {
                let block = provider
                    .get_block(BlockNumber::Finalized)
                    .await
                    .map_err(|e| Error::chain(format!("Failed to get finalized block: {}", e)))?
                    .ok_or_else(|| Error::chain("Finalized block not found"))?;
                
                let number = block.number
                    .ok_or_else(|| Error::chain("Finalized block has no number"))?
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
                    .get_block(BlockNumber::Safe)
                    .await
                    .map_err(|e| Error::chain(format!("Failed to get safe block: {}", e)))?
                    .ok_or_else(|| Error::chain("Safe block not found"))?;
                
                let number = block.number
                    .ok_or_else(|| Error::chain("Safe block has no number"))?
                    .as_u64();
                
                Ok(number)
            }
            EthereumProvider::Websocket(provider) => {
                let block = provider
                    .get_block(BlockNumber::Safe)
                    .await
                    .map_err(|e| Error::chain(format!("Failed to get safe block: {}", e)))?
                    .ok_or_else(|| Error::chain("Safe block not found"))?;
                
                let number = block.number
                    .ok_or_else(|| Error::chain("Safe block has no number"))?
                    .as_u64();
                
                Ok(number)
            }
        }
    }
} 