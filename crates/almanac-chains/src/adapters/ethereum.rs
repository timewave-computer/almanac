//! Ethereum chain adapter implementation

use async_trait::async_trait;
use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

use crate::adapters::{
    traits::{ChainAdapter, ChainAdapterError},
    types::{
        BlockMetadata, ChainConfig, ChainDataIR, ConsensusMetadata, FinalityStatus, ProofType,
        StateChange, StorageProof,
    },
};

/// Ethereum-specific consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumConsensusConfig {
    /// Fork choice rule
    pub fork_choice_rule: EthereumForkChoiceRule,
    /// Maximum reorganization depth
    pub max_reorg_depth: u64,
    /// Probabilistic finality (for PoW chains)
    pub probabilistic_finality: bool,
    /// Consensus-specific parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Ethereum-specific finality configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumFinalityConfig {
    /// Block confirmations required for each finality level
    pub confirmations: HashMap<EthereumFinalityLevel, u64>,
    /// Time requirements for finality levels
    pub time_requirements: HashMap<EthereumFinalityLevel, Duration>,
    /// Auto-finality timeout
    pub auto_finality_timeout: Duration,
}

/// Ethereum fork choice rules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EthereumForkChoiceRule {
    /// Longest chain (Bitcoin, early Ethereum)
    LongestChain,
    /// Heaviest chain by difficulty
    HeaviestChain,
    /// GHOST protocol
    Ghost,
}

/// Ethereum finality levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EthereumFinalityLevel {
    /// Block is confirmed
    Confirmed,
    /// Block is safe from reorgs
    Safe,
    /// Block is justified by validators
    Justified,
    /// Block is finalized (irreversible)
    Finalized,
}

/// Ethereum block information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumBlockInfo {
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: u64,
}

/// Ethereum consensus update result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumConsensusUpdate {
    /// Whether canonical head changed
    pub head_changed: bool,
    /// Previous head if changed
    pub previous_head: Option<EthereumBlockInfo>,
    /// New canonical head
    pub new_head: EthereumBlockInfo,
    /// Reorganization depth if any
    pub reorg_depth: Option<u64>,
}

/// Ethereum chain adapter
pub struct EthereumAdapter {
    client: Arc<Provider<Http>>,
    chain_id: String,
    config: ChainConfig,
}

impl EthereumAdapter {
    /// Create a new Ethereum adapter
    pub async fn new(config: ChainConfig) -> Result<Self, ChainAdapterError> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)
            .map_err(|e| ChainAdapterError::Configuration(format!("Invalid RPC URL: {}", e)))?;

        let client = Arc::new(provider);

        // Verify connection
        let chain_id = client
            .get_chainid()
            .await
            .map_err(|e| ChainAdapterError::Network(format!("Failed to get chain ID: {}", e)))?;

        info!("Connected to Ethereum chain ID: {}", chain_id);

        Ok(Self {
            client,
            chain_id: config.chain_id.clone(),
            config,
        })
    }

    /// Convert H256 to [u8; 32]
    #[allow(dead_code)]
    fn h256_to_bytes32(h: H256) -> [u8; 32] {
        h.0
    }

    /// Convert U256 to [u8; 32]
    fn u256_to_bytes32(u: U256) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        u.to_big_endian(&mut bytes);
        bytes
    }

    /// Extract state changes from block traces
    async fn extract_state_changes_from_block(
        &self,
        block: &Block<H256>,
    ) -> Result<Vec<StateChange>, ChainAdapterError> {
        let state_changes = Vec::new();

        // For each transaction in the block, get the state changes
        for tx_hash in block.transactions.iter() {
            let trace = self.client.trace_transaction(*tx_hash).await.map_err(|e| {
                ChainAdapterError::Rpc(format!("Failed to trace transaction: {}", e))
            })?;

            // Process traces to extract state changes
            // Note: For now, we'll extract basic information from traces
            // In a full implementation, you would use debug_traceTransaction with proper tracer
            for trace_item in trace {
                // Extract state changes from trace actions
                // This is a simplified approach - in practice you'd need more sophisticated tracing
                match &trace_item.action {
                    ethers::types::Action::Call(call_action) => {
                        // For call actions, we can infer potential state changes
                        // but we don't have actual storage diffs from basic traces
                        info!(
                            "Call trace found: {:?} -> {:?}",
                            call_action.from, call_action.to
                        );
                    }
                    _ => {
                        // Handle other action types as needed
                    }
                }
            }
        }

        Ok(state_changes)
    }

    /// Get finality status based on confirmations
    async fn get_finality_status(
        &self,
        block_number: u64,
    ) -> Result<FinalityStatus, ChainAdapterError> {
        let latest = self.get_latest_block().await?;
        let confirmations = latest.saturating_sub(block_number);

        Ok(match confirmations {
            0..=5 => FinalityStatus::Pending,
            6..=31 => FinalityStatus::Safe,
            _ => FinalityStatus::Finalized,
        })
    }
}

#[async_trait]
impl ChainAdapter for EthereumAdapter {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }

    async fn fetch_block(&self, block_number: u64) -> Result<ChainDataIR, ChainAdapterError> {
        debug!("Fetching Ethereum block {}", block_number);

        let block = self
            .client
            .get_block(block_number)
            .await
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get block: {}", e)))?
            .ok_or(ChainAdapterError::BlockNotFound(block_number))?;

        // Extract state changes
        let state_changes = self.extract_state_changes_from_block(&block).await?;

        // Get finality status
        let finality_status = self.get_finality_status(block_number).await?;

        // Build block metadata
        let block_metadata = BlockMetadata {
            block_height: block_number,
            block_hash: format!("{:?}", block.hash.unwrap_or_default()),
            parent_hash: format!("{:?}", block.parent_hash),
            timestamp: block.timestamp.as_u64(),
            state_root: format!("{:?}", block.state_root),
            receipts_root: Some(format!("{:?}", block.receipts_root)),
            transactions_root: Some(format!("{:?}", block.transactions_root)),
        };

        // Build consensus metadata
        let mut extra = HashMap::new();
        extra.insert(
            "nonce".to_string(),
            serde_json::json!(format!("{:?}", block.nonce)),
        );
        extra.insert(
            "mix_hash".to_string(),
            serde_json::json!(format!("{:?}", block.mix_hash)),
        );
        extra.insert(
            "base_fee_per_gas".to_string(),
            serde_json::json!(block.base_fee_per_gas.map(|fee| fee.to_string())),
        );
        extra.insert(
            "difficulty".to_string(),
            serde_json::json!(block.difficulty.to_string()),
        );
        extra.insert(
            "total_difficulty".to_string(),
            serde_json::json!(block.total_difficulty.map(|td| td.to_string())),
        );
        extra.insert(
            "gas_limit".to_string(),
            serde_json::json!(block.gas_limit.to_string()),
        );
        extra.insert(
            "gas_used".to_string(),
            serde_json::json!(block.gas_used.to_string()),
        );

        let consensus_metadata = ConsensusMetadata {
            validator_set_hash: None,
            proposer: block.author.map(|a| format!("{:?}", a)),
            round: None,
            signatures: None,
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
        self.client
            .get_block_number()
            .await
            .map(|n| n.as_u64())
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get latest block: {}", e)))
    }

    async fn is_block_finalized(&self, block_number: u64) -> Result<bool, ChainAdapterError> {
        let latest = self.get_latest_block().await?;
        let confirmations = latest.saturating_sub(block_number);
        Ok(confirmations >= self.config.confirmations as u64)
    }

    async fn generate_storage_proofs(
        &self,
        contract_address: &str,
        storage_keys: &[[u8; 32]],
        block_number: u64,
    ) -> Result<Vec<StorageProof>, ChainAdapterError> {
        let address = contract_address
            .parse::<Address>()
            .map_err(|e| ChainAdapterError::Other(format!("Invalid address: {}", e)))?;

        let block_id = Some(BlockId::Number(BlockNumber::Number(block_number.into())));

        let mut proofs = Vec::new();

        for storage_key in storage_keys {
            let key = H256::from_slice(storage_key);

            let proof = self
                .client
                .get_proof(address, vec![key], block_id)
                .await
                .map_err(|e| {
                    ChainAdapterError::ProofGeneration(format!("Failed to get proof: {}", e))
                })?;

            // Get the storage value
            let value = proof
                .storage_proof
                .first()
                .map(|sp| Self::u256_to_bytes32(sp.value))
                .unwrap_or([0u8; 32]);

            // Get the state root for verification
            let _block = self
                .client
                .get_block(block_number)
                .await
                .map_err(|e| ChainAdapterError::Rpc(e.to_string()))?
                .ok_or(ChainAdapterError::BlockNotFound(block_number))?;

            let storage_proof = StorageProof {
                key: *storage_key,
                value,
                proof_nodes: proof
                    .account_proof
                    .into_iter()
                    .map(|bytes| bytes.to_vec())
                    .collect(),
                proof_type: ProofType::MerklePatricia,
            };

            proofs.push(storage_proof);
        }

        Ok(proofs)
    }

    async fn check_block_finality(
        &self,
        block_number: u64,
        current_height: u64,
    ) -> Result<FinalityStatus, ChainAdapterError> {
        let confirmations = current_height.saturating_sub(block_number);

        Ok(match confirmations {
            0 => FinalityStatus::Pending,
            1..=5 => FinalityStatus::Confirmed,
            6..=31 => FinalityStatus::Safe,
            32..=63 => FinalityStatus::Justified,
            _ => FinalityStatus::Finalized,
        })
    }

    async fn get_block_metadata(
        &self,
        block_number: u64,
    ) -> Result<BlockMetadata, ChainAdapterError> {
        let block = self
            .client
            .get_block(block_number)
            .await
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get block: {}", e)))?
            .ok_or(ChainAdapterError::BlockNotFound(block_number))?;

        let _finality_status = self.get_finality_status(block_number).await?;

        Ok(BlockMetadata {
            block_height: block_number,
            block_hash: format!("{:?}", block.hash.unwrap_or_default()),
            parent_hash: format!("{:?}", block.parent_hash),
            timestamp: block.timestamp.as_u64(),
            state_root: format!("{:?}", block.state_root),
            receipts_root: Some(format!("{:?}", block.receipts_root)),
            transactions_root: Some(format!("{:?}", block.transactions_root)),
        })
    }

    async fn extract_state_changes(
        &self,
        block_number: u64,
    ) -> Result<Vec<StateChange>, ChainAdapterError> {
        let block = self
            .client
            .get_block(block_number)
            .await
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get block: {}", e)))?
            .ok_or(ChainAdapterError::BlockNotFound(block_number))?;

        self.extract_state_changes_from_block(&block).await
    }

    async fn subscribe_blocks(
        &self,
    ) -> Result<Box<dyn futures::Stream<Item = u64> + Send>, ChainAdapterError> {
        // For WebSocket support, we'd need a different provider type
        // This is a placeholder for HTTP-only implementation
        Err(ChainAdapterError::Unsupported(
            "Block subscription requires WebSocket connection".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_h256_conversion() {
        let h = H256::from_low_u64_be(12345);
        let bytes = EthereumAdapter::h256_to_bytes32(h);
        assert_eq!(bytes[31], 57); // 12345 & 0xFF
        assert_eq!(bytes[30], 48); // (12345 >> 8) & 0xFF
    }

    #[tokio::test]
    async fn test_u256_conversion() {
        let u = U256::from(12345u64);
        let bytes = EthereumAdapter::u256_to_bytes32(u);
        assert_eq!(bytes[31], 57);
        assert_eq!(bytes[30], 48);
    }
}
