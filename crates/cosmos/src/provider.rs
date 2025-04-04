/// Cosmos RPC provider
use std::sync::Arc;

use cosmrs::rpc::{Client, HttpClient};
use cosmrs::tendermint::block::Height;
use cosmrs::tendermint::Hash as TendermintHash;
use indexer_common::{Error, Result};
use sha2::{Sha256, Digest};
use tracing::info;

/// Cosmos provider for blockchain interactions
pub struct CosmosProvider {
    /// HTTP client for Cosmos RPC
    pub client: Arc<HttpClient>,
}

impl CosmosProvider {
    /// Create a new Cosmos provider
    pub async fn new(rpc_url: String) -> Result<Self> {
        info!("Connecting to Cosmos RPC at {}", rpc_url);
        let client = HttpClient::new(rpc_url.as_str())
            .map_err(|e| Error::generic(format!("Failed to connect to Cosmos RPC: {}", e)))?;
        
        Ok(Self {
            client: Arc::new(client),
        })
    }
    
    /// Get the latest block height
    pub async fn get_block_height(&self) -> Result<u64> {
        let status = self.client.status().await
            .map_err(|e| Error::generic(format!("Failed to get status: {}", e)))?;
        Ok(status.sync_info.latest_block_height.value())
    }
    
    /// Get a block by height
    pub async fn get_block(&self, height: u64) -> Result<cosmrs::tendermint::Block> {
        // Convert height to tendermint Height
        let height = Height::try_from(height)
            .map_err(|e| Error::generic(format!("Invalid block height {}: {}", height, e)))?;
        
        let block = self.client.block(height).await
            .map_err(|e| Error::generic(format!("Failed to get block {}: {}", height, e)))?;
        Ok(block.block)
    }
    
    /// Get transactions in a block
    pub async fn get_block_txs(&self, height: u64) -> Result<Vec<Vec<u8>>> {
        let block = self.get_block(height).await?;
        
        let txs = block.data.iter()
            .map(|tx| tx.to_vec())
            .collect();
        
        Ok(txs)
    }
    
    /// Get transaction results for a block
    pub async fn get_tx_results(&self, height: u64) -> Result<Vec<cosmrs::rpc::endpoint::tx::Response>> {
        let txs = self.get_block_txs(height).await?;
        let mut results = Vec::with_capacity(txs.len());
        
        for tx_bytes in txs {
            let tx_hash = Sha256::digest(&tx_bytes);
            let hash_bytes: [u8; 32] = tx_hash.as_slice().try_into()
                .map_err(|_| Error::generic("Failed to convert hash bytes".to_string()))?;
            
            let tx_hash = TendermintHash::Sha256(hash_bytes);
            
            let result = self.client.tx(tx_hash, false).await
                .map_err(|e| Error::generic(format!("Failed to get tx result: {}", e)))?;
            
            results.push(result);
        }
        
        Ok(results)
    }
} 