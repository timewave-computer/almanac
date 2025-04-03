use cosmrs::rpc::{Client, HttpClient};
use indexer_core::Result;

/// Cosmos chain provider
pub struct CosmosProvider {
    /// Cosmos RPC client
    pub client: HttpClient,
    
    /// RPC URL
    pub rpc_url: String,
}

impl CosmosProvider {
    /// Create a new Cosmos provider
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let client = HttpClient::new(rpc_url)
            .map_err(|e| indexer_core::Error::chain(format!("Failed to create Cosmos client: {}", e)))?;
        
        Ok(Self {
            client,
            rpc_url: rpc_url.to_string(),
        })
    }
    
    /// Get the latest block height
    pub async fn get_latest_block_height(&self) -> Result<u64> {
        let status = self.client.status().await
            .map_err(|e| indexer_core::Error::chain(format!("Failed to get status: {}", e)))?;
        
        Ok(status.sync_info.latest_block_height.value() as u64)
    }
    
    /// Get a block by height
    pub async fn get_block(&self, height: u64) -> Result<cosmrs::rpc::endpoint::block::Response> {
        let height = height.into();
        
        let block = self.client.block(height).await
            .map_err(|e| indexer_core::Error::chain(format!("Failed to get block: {}", e)))?;
        
        Ok(block)
    }
    
    /// Get transactions in a block
    pub async fn get_block_txs(&self, height: u64) -> Result<Vec<cosmrs::tx::Raw>> {
        let block = self.get_block(height).await?;
        
        Ok(block.block.data.iter().cloned().collect())
    }
    
    /// Get transaction results
    pub async fn get_tx_results(&self, height: u64) -> Result<Vec<cosmrs::rpc::endpoint::tx::Response>> {
        let block = self.get_block(height).await?;
        let mut results = Vec::new();
        
        for tx in block.block.data.iter() {
            let tx_hash = tx.hash();
            
            let tx_result = self.client.tx(tx_hash, false).await
                .map_err(|e| indexer_core::Error::chain(format!("Failed to get tx result: {}", e)))?;
            
            results.push(tx_result);
        }
        
        Ok(results)
    }
} 