//! Cosmos chain adapter implementation

use async_trait::async_trait;
use tendermint_rpc::{client::Client, HttpClient};
use tracing::{debug, info};
use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::adapters::{
    traits::{ChainAdapter, ChainAdapterError},
    types::{BlockMetadata, ChainConfig, ChainDataIR, ConsensusMetadata, FinalityStatus, StateChange, StorageProof},
};

/// Cosmos consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosConsensusConfig {
    /// Consensus algorithm (e.g., "tendermint")
    pub algorithm: String,
    /// Validator set configuration
    pub validator_set: HashMap<String, serde_json::Value>,
    /// Consensus parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Cosmos finality configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosFinalityConfig {
    /// Number of blocks required for finality
    pub finality_blocks: u64,
    /// Timeout for finality confirmation
    pub finality_timeout: Duration,
    /// Enable Byzantine fault tolerance
    pub enable_bft: bool,
}

/// Cosmos fork choice rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CosmosForkChoiceRule {
    /// Longest chain rule
    LongestChain,
    /// Highest cumulative weight
    HighestWeight,
    /// Custom rule
    Custom(String),
}

/// Cosmos finality level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CosmosFinalityLevel {
    /// Transaction included in a block
    Included,
    /// Block confirmed by validators
    Confirmed,
    /// Block finalized (cannot be reverted)
    Finalized,
}

/// Cosmos block information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosBlockInfo {
    /// Block height
    pub height: u64,
    /// Block hash
    pub hash: String,
    /// Previous block hash
    pub parent_hash: String,
    /// Block timestamp
    pub timestamp: u64,
    /// Consensus metadata
    pub consensus: ConsensusMetadata,
}

/// Cosmos consensus update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosConsensusUpdate {
    /// Block height
    pub height: u64,
    /// Consensus change type
    pub change_type: String,
    /// Updated metadata
    pub metadata: ConsensusMetadata,
    /// Timestamp of the update
    pub timestamp: u64,
}

/// Cosmos chain adapter
pub struct CosmosAdapter {
    client: HttpClient,
    chain_id: String,
    #[allow(dead_code)]
    config: ChainConfig,
}

impl CosmosAdapter {
    /// Create a new Cosmos adapter
    pub async fn new(config: ChainConfig) -> Result<Self, ChainAdapterError> {
        let client = HttpClient::new(config.rpc_url.as_str())
            .map_err(|e| ChainAdapterError::Configuration(format!("Failed to create client: {}", e)))?;

        // Verify connection by getting status
        let status = client
            .status()
            .await
            .map_err(|e| ChainAdapterError::Network(format!("Failed to connect: {}", e)))?;

        let chain_id = status.node_info.network.to_string();
        info!("Connected to Cosmos chain: {}", chain_id);

        Ok(Self {
            client,
            chain_id: config.chain_id.clone(),
            config,
        })
    }

    /// Convert hash to [u8; 32]
    #[allow(dead_code)]
    fn hash_to_bytes32(hash: &[u8]) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        let len = hash.len().min(32);
        bytes[..len].copy_from_slice(&hash[..len]);
        bytes
    }

    /// Extract state changes from block results
    async fn extract_state_changes_from_block(
        &self,
        block_height: u64,
    ) -> Result<Vec<StateChange>, ChainAdapterError> {
        let mut state_changes = Vec::new();

        // Get block results to access events
        let block_results = self.client
            .block_results(block_height as u32)
            .await
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get block results: {}", e)))?;

        // Process begin block events
        if let Some(events_list) = &block_results.begin_block_events {
            for event in events_list {
                if event.kind == "store_update" {
                    // Extract store updates from events
                    let mut address = String::new();
                    let mut key = [0u8; 32];
                    let mut value = [0u8; 32];

                    for attr in &event.attributes {
                    match attr.key.as_str() {
                        "store" => address = attr.value.clone(),
                        "key" => {
                            let key_bytes = hex::decode(&attr.value)
                                .unwrap_or_default();
                            let len = key_bytes.len().min(32);
                            key[..len].copy_from_slice(&key_bytes[..len]);
                        }
                        "value" => {
                            let value_bytes = hex::decode(&attr.value)
                                .unwrap_or_default();
                            let len = value_bytes.len().min(32);
                            value[..len].copy_from_slice(&value_bytes[..len]);
                        }
                        _ => {}
                    }
                }

                if !address.is_empty() {
                    state_changes.push(StateChange {
                        contract_address: address,
                        storage_slot: key,
                        previous_value: Some([0u8; 32]), // Cosmos doesn't provide previous value in events
                        new_value: value,
                        proof: None,
                    });
                }
                }
            }
        }

        // Process transaction events
        for _tx_result in block_results.txs_results.iter() {
            // TODO: Process transaction events when the API structure is clarified
            // For now, we'll leave event processing as a stub
        }

        Ok(state_changes)
    }

    /// Get finality status for Cosmos (instant finality with Tendermint)
    /// Cosmos has instant finality - all confirmed blocks are finalized
    fn get_finality_status(&self, block_height: u64, latest_height: u64) -> FinalityStatus {
        if block_height <= latest_height {
            // In Cosmos, all blocks are instantly finalized once confirmed
            FinalityStatus::Finalized
        } else {
            FinalityStatus::Pending
        }
    }
}

#[async_trait]
impl ChainAdapter for CosmosAdapter {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }
    

    async fn fetch_block(&self, block_number: u64) -> Result<ChainDataIR, ChainAdapterError> {
        debug!("Fetching Cosmos block {}", block_number);

        // Get block
        let block_response = self.client
            .block(block_number as u32)
            .await
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get block: {}", e)))?;

        let block = block_response.block;
        let header = &block.header;

        // Get latest height for finality calculation
        let latest_response = self.client
            .latest_block()
            .await
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get latest block: {}", e)))?;
        
        let latest_height = latest_response.block.header.height.value();

        // Extract state changes
        let state_changes = self.extract_state_changes_from_block(block_number).await?;

        // Get finality status
        let finality_status = self.get_finality_status(block_number, latest_height);

        // Build block metadata
        let block_metadata = BlockMetadata {
            block_height: block_number,
            block_hash: hex::encode(block_response.block_id.hash.as_bytes()),
            parent_hash: hex::encode(header.last_block_id.as_ref().map(|id| id.hash.as_bytes()).unwrap_or(&[0u8; 32])),
            timestamp: header.time.unix_timestamp() as u64,
            state_root: hex::encode(header.app_hash.as_bytes()),
            receipts_root: None, // Not applicable for Cosmos
            transactions_root: Some(hex::encode(header.data_hash.as_ref().map(|h| h.as_bytes()).unwrap_or(&[0u8; 32]))),
        };

        // Build consensus metadata
        let mut extra = HashMap::new();
        extra.insert("consensus_hash".to_string(), serde_json::json!(hex::encode(header.consensus_hash.as_bytes())));
        extra.insert("next_validators_hash".to_string(), serde_json::json!(hex::encode(header.next_validators_hash.as_bytes())));
        extra.insert("evidence_hash".to_string(), serde_json::json!(header.evidence_hash.as_ref().map(|h| hex::encode(h.as_bytes()))));
        extra.insert("last_results_hash".to_string(), serde_json::json!(header.last_results_hash.as_ref().map(|h| hex::encode(h.as_bytes()))));
        extra.insert("validators_hash".to_string(), serde_json::json!(hex::encode(header.validators_hash.as_bytes())));
        
        let consensus_metadata = ConsensusMetadata {
            validator_set_hash: Some(hex::encode(header.validators_hash.as_bytes())),
            proposer: Some(hex::encode(header.proposer_address.as_bytes())),
            round: None, // Would need to extract from commit info
            signatures: None, // Would need to extract from block signatures
            extra,
        };

        Ok(ChainDataIR {
            block_metadata,
            state_changes,
            consensus_metadata,
            finality_status,
        })
    }

    async fn get_latest_block(&self) -> Result<u64, ChainAdapterError> {
        let response = self.client
            .latest_block()
            .await
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get latest block: {}", e)))?;

        Ok(response.block.header.height.value())
    }

    
    async fn check_block_finality(&self, block_number: u64, current_height: u64) -> Result<FinalityStatus, ChainAdapterError> {
        let confirmations = current_height.saturating_sub(block_number);
        
        Ok(match confirmations {
            0 => FinalityStatus::Pending,
            1 => FinalityStatus::Safe,
            _ => FinalityStatus::Finalized,
        })
    }

    async fn is_block_finalized(&self, block_number: u64) -> Result<bool, ChainAdapterError> {
        let latest = self.get_latest_block().await?;
        let confirmations = latest.saturating_sub(block_number);
        
        // Cosmos achieves finality quickly with Tendermint consensus
        Ok(confirmations >= 2)
    }

    async fn generate_storage_proofs(
        &self,
        _contract_address: &str,
        _storage_keys: &[[u8; 32]],
        _block_number: u64,
    ) -> Result<Vec<StorageProof>, ChainAdapterError> {
        // Cosmos doesn't provide storage proofs in the same way as Ethereum
        // This would need to be implemented based on the specific Cosmos SDK modules
        Err(ChainAdapterError::Unsupported(
            "Storage proofs not yet implemented for Cosmos".to_string()
        ))
    }

    async fn get_block_metadata(&self, block_number: u64) -> Result<BlockMetadata, ChainAdapterError> {
        let block_response = self.client
            .block(block_number as u32)
            .await
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get block: {}", e)))?;

        let block = block_response.block;
        let header = &block.header;

        let latest_response = self.client
            .latest_block()
            .await
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get latest block: {}", e)))?;
        
        let latest_height = latest_response.block.header.height.value();
        let _finality_status = self.get_finality_status(block_number, latest_height);

        Ok(BlockMetadata {
            block_height: block_number,
            block_hash: hex::encode(block_response.block_id.hash.as_bytes()),
            parent_hash: hex::encode(header.last_block_id.as_ref().map(|id| id.hash.as_bytes()).unwrap_or(&[0u8; 32])),
            timestamp: header.time.unix_timestamp() as u64,
            state_root: hex::encode(header.app_hash.as_bytes()),
            receipts_root: None,
            transactions_root: Some(hex::encode(header.data_hash.as_ref().map(|h| h.as_bytes()).unwrap_or(&[0u8; 32]))),
        })
    }

    async fn extract_state_changes(&self, block_number: u64) -> Result<Vec<StateChange>, ChainAdapterError> {
        self.extract_state_changes_from_block(block_number).await
    }

    async fn subscribe_blocks(&self) -> Result<Box<dyn futures::Stream<Item = u64> + Send>, ChainAdapterError> {
        // WebSocket subscription would be implemented here
        // For now, return unsupported
        Err(ChainAdapterError::Unsupported(
            "Block subscription not yet implemented for Cosmos".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_conversion() {
        let hash = vec![1, 2, 3, 4, 5];
        let bytes32 = CosmosAdapter::hash_to_bytes32(&hash);
        assert_eq!(bytes32[0], 1);
        assert_eq!(bytes32[1], 2);
        assert_eq!(bytes32[2], 3);
        assert_eq!(bytes32[3], 4);
        assert_eq!(bytes32[4], 5);
        assert_eq!(bytes32[5], 0); // Rest should be zeros
    }

    #[test]
    fn test_finality_status() {
        let adapter = CosmosAdapter {
            client: HttpClient::new("http://localhost").unwrap(),
            chain_id: "test".to_string(),
            config: ChainConfig {
                chain_id: "test".to_string(),
                rpc_url: "http://localhost".to_string(),
                api_key: None,
                confirmations: 2,
                max_retries: 3,
                timeout_secs: 30,
                extra: serde_json::Value::Null,
            },
        };

        assert_eq!(adapter.get_finality_status(100, 100), FinalityStatus::Finalized);
        assert_eq!(adapter.get_finality_status(100, 101), FinalityStatus::Finalized);
        assert_eq!(adapter.get_finality_status(101, 100), FinalityStatus::Pending);
    }
}