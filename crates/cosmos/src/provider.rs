/// Cosmos RPC provider
use std::sync::Arc;
use std::collections::HashMap;

use cosmrs::rpc::{Client, HttpClient, query::Query, response::Wrapper};
use cosmrs::tendermint::block::Height;
use cosmrs::tendermint::Hash as TendermintHash;
use cosmrs::proto::cosmos::tx::v1beta1::{GetTxRequest, GetTxResponse};
use cosmrs::proto::tendermint::abci::EventAttribute;
use indexer_common::{Error, Result};
use sha2::{Sha256, Digest};
use tracing::{info, debug, error};
use async_trait::async_trait;
use cosmrs::rpc::endpoint::abci_query::Response as AbciQueryResponse;

/// Cosmos provider for blockchain interactions
pub struct CosmosProvider {
    /// HTTP client for Cosmos RPC
    pub client: Arc<HttpClient>,
    
    /// RPC URL
    pub rpc_url: String,
}

/// Cosmos block status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CosmosBlockStatus {
    /// Block is included in chain (may be reversible)
    Confirmed,
    
    /// Block has enough attestations to be unlikely to be orphaned
    Safe,
    
    /// Block has been finalized
    Finalized,
}

impl CosmosProvider {
    /// Create a new Cosmos provider
    pub async fn new(rpc_url: &str) -> Result<Self> {
        info!("Connecting to Cosmos RPC at {}", rpc_url);
        let client = HttpClient::new(rpc_url)
            .map_err(|e| Error::generic(format!("Failed to connect to Cosmos RPC: {}", e)))?;
        
        Ok(Self {
            client: Arc::new(client),
            rpc_url: rpc_url.to_string(),
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
    
    /// Get block results which contain events
    pub async fn get_block_results(&self, height: u64) -> Result<cosmrs::rpc::endpoint::block_results::Response> {
        // Convert height to tendermint Height
        let height = Height::try_from(height)
            .map_err(|e| Error::generic(format!("Invalid block height {}: {}", height, e)))?;
        
        let results = self.client.block_results(height).await
            .map_err(|e| Error::generic(format!("Failed to get block results {}: {}", height, e)))?;
        
        Ok(results)
    }
    
    /// Get transactions in a block
    pub async fn get_block_txs(&self, height: u64) -> Result<Vec<Vec<u8>>> {
        let block = self.get_block(height).await?;
        
        let txs = block.data.iter()
            .map(|tx| tx.to_vec())
            .collect();
        
        Ok(txs)
    }
    
    /// Get event data for a block
    pub async fn get_block_event_data(&self, height: u64) -> Result<Vec<HashMap<String, String>>> {
        let results = self.get_block_results(height).await?;
        let mut event_data = Vec::new();
        
        // Get events from begin block
        if let Some(begin_block_events) = &results.begin_block_events {
            for event in begin_block_events {
                let mut data = HashMap::new();
                // Add event type (field is 'kind' not 'r#type')
                data.insert("event_type".to_string(), event.kind.clone());
                
                // Add all attributes
                for attr in &event.attributes {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                    data.insert(key, value);
                }
                
                event_data.push(data);
            }
        }
        
        // Get events from transactions
        if let Some(tx_results) = &results.txs_results {
            for tx_result in tx_results {
                for event in &tx_result.events {
                    let mut data = HashMap::new();
                    // Add event type (field is 'kind' not 'r#type')
                    data.insert("event_type".to_string(), event.kind.clone());
                    
                    // Add all attributes
                    for attr in &event.attributes {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                        data.insert(key, value);
                    }
                    
                    event_data.push(data);
                }
            }
        }
        
        // Get events from end block
        if let Some(end_block_events) = &results.end_block_events {
            for event in end_block_events {
                let mut data = HashMap::new();
                // Add event type (field is 'kind' not 'r#type')
                data.insert("event_type".to_string(), event.kind.clone());
                
                // Add all attributes
                for attr in &event.attributes {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                    data.insert(key, value);
                }
                
                event_data.push(data);
            }
        }
        
        Ok(event_data)
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
    
    /// Get transaction details (through gRPC endpoint)
    pub async fn get_tx_details(&self, tx_hash: &str) -> Result<GetTxResponse> {
        // This is a placeholder since cosmrs doesn't directly support the gRPC endpoints
        // In a real implementation, you would use a gRPC client for this
        debug!("GetTx endpoint requested for hash {}", tx_hash);
        Err(Error::generic("gRPC endpoint not implemented".to_string()))
    }
    
    /// Get a block by status
    pub async fn get_block_by_status(&self, status: CosmosBlockStatus) -> Result<(cosmrs::tendermint::Block, u64)> {
        // In Cosmos/Tendermint, once a block is included in the chain, it's considered final after a few blocks
        // For this implementation, we'll consider the latest block as confirmed,
        // the latest block - 2 as safe, and the latest block - 6 as finalized
        
        let latest_height = self.get_block_height().await?;
        
        let block_height = match status {
            CosmosBlockStatus::Confirmed => latest_height,
            CosmosBlockStatus::Safe => latest_height.saturating_sub(2),
            CosmosBlockStatus::Finalized => latest_height.saturating_sub(6),
        };
        
        let block = self.get_block(block_height).await?;
        
        Ok((block, block_height))
    }
    
    /// Parse ABCi event attributes into a HashMap
    pub fn parse_event_attributes(attributes: &[EventAttribute]) -> HashMap<String, String> {
        let mut result = HashMap::new();
        
        for attr in attributes {
            // Convert byte vectors to strings
            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
            let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
            result.insert(key, value);
        }
        
        result
    }
    
    /// Get blocks in a range
    pub async fn get_blocks_in_range(&self, from_height: u64, to_height: u64) -> Result<Vec<cosmrs::tendermint::Block>> {
        if from_height > to_height {
            return Err(Error::generic("from_height must be less than or equal to to_height".to_string()));
        }
        
        let mut blocks = Vec::with_capacity((to_height - from_height + 1) as usize);
        
        for height in from_height..=to_height {
            match self.get_block(height).await {
                Ok(block) => blocks.push(block),
                Err(e) => {
                    error!("Failed to get block {}: {}", height, e);
                    // Don't fail the entire range if one block fails
                    continue;
                }
            }
        }
        
        Ok(blocks)
    }
}

// --- Define the Provider Trait --- 
#[async_trait]
pub trait CosmosProviderTrait: Send + Sync + 'static {
    // Methods used by CosmosEventService
    async fn get_block_height(&self) -> Result<u64>;
    async fn get_block(&self, height: u64) -> Result<cosmrs::tendermint::Block>;
    async fn get_tx_results(&self, height: u64) -> Result<Vec<cosmrs::rpc::endpoint::tx::Response>>;
    async fn get_block_by_status(&self, status: CosmosBlockStatus) -> Result<(cosmrs::tendermint::Block, u64)>;
    async fn abci_query(&self, path: Option<String>, data: Vec<u8>, height: Option<Height>, prove: bool) -> Result<AbciQueryResponse>;
    // Add other methods if needed by other parts of the service
}

// --- Implement the Trait for the Real Provider --- 
#[async_trait]
impl CosmosProviderTrait for CosmosProvider {
    async fn get_block_height(&self) -> Result<u64> {
        let status = self.client.status().await
            .map_err(|e| Error::generic(format!("Failed to get status: {}", e)))?;
        Ok(status.sync_info.latest_block_height.value())
    }
    
    async fn get_block(&self, height: u64) -> Result<cosmrs::tendermint::Block> {
        let height_tm = Height::try_from(height)
            .map_err(|e| Error::generic(format!("Invalid block height {}: {}", height, e)))?;
        
        let block = self.client.block(height_tm).await
            .map_err(|e| Error::generic(format!("Failed to get block {}: {}", height, e)))?;
        Ok(block.block)
    }

    async fn get_tx_results(&self, height: u64) -> Result<Vec<cosmrs::rpc::endpoint::tx::Response>> {
        let txs = self.get_block_txs(height).await?;
        let mut results = Vec::with_capacity(txs.len());
        
        for tx_bytes in txs {
            let tx_hash_bytes = Sha256::digest(&tx_bytes);
             let hash_bytes: [u8; 32] = tx_hash_bytes.as_slice().try_into()
                .map_err(|_| Error::generic("Failed to convert hash bytes".to_string()))?;
            
            let tx_hash = TendermintHash::Sha256(hash_bytes);
            
            let result = self.client.tx(tx_hash, false).await
                 .map_err(|e| Error::generic(format!("Failed to get tx result: {}", e)))?;
            
             results.push(result);
        }
        
        Ok(results)
    }

     async fn get_block_by_status(&self, status: CosmosBlockStatus) -> Result<(cosmrs::tendermint::Block, u64)> {
        let latest_height = self.get_block_height().await?;
        
        let block_height = match status {
            CosmosBlockStatus::Confirmed => latest_height,
            CosmosBlockStatus::Safe => latest_height.saturating_sub(2),
            CosmosBlockStatus::Finalized => latest_height.saturating_sub(6),
        };
        
        let block = self.get_block(block_height).await?;
        
        Ok((block, block_height))
    }

    async fn abci_query(&self, path: Option<String>, data: Vec<u8>, height: Option<Height>, prove: bool) -> Result<AbciQueryResponse> {
         self.client.abci_query(path, data, height, prove).await
             .map_err(|e| Error::Rpc(e.to_string()))
    }
} 