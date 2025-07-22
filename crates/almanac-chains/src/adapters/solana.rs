//! Solana chain adapter implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::adapters::{
    traits::{ChainAdapter, ChainAdapterError},
    types::{
        BlockMetadata, ChainConfig, ChainDataIR, ConsensusMetadata, FinalityStatus, ProofType,
        StateChange, StorageProof,
    },
};

/// Solana consensus configuration
#[cfg(feature = "solana")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConsensusConfig {
    /// Consensus algorithm (e.g., "proof-of-history")
    pub algorithm: String,
    /// Validator set configuration
    pub validator_set: HashMap<String, serde_json::Value>,
    /// Consensus parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Solana finality configuration
#[cfg(feature = "solana")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaFinalityConfig {
    /// Number of blocks required for finality
    pub finality_blocks: u64,
    /// Timeout for finality confirmation
    pub finality_timeout: Duration,
    /// Enable optimistic finality
    pub enable_optimistic: bool,
}

/// Solana fork choice rule
#[cfg(feature = "solana")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolanaForkChoiceRule {
    /// Longest chain rule
    LongestChain,
    /// Highest cumulative weight
    HighestWeight,
    /// Custom rule
    Custom(String),
}

/// Solana finality level
#[cfg(feature = "solana")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SolanaFinalityLevel {
    /// Transaction included in a block
    Included,
    /// Block confirmed by validators
    Confirmed,
    /// Block finalized (cannot be reverted)
    Finalized,
}

/// Solana block information
#[cfg(feature = "solana")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaBlockInfo {
    /// Block height (slot)
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

/// Solana consensus update
#[cfg(feature = "solana")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConsensusUpdate {
    /// Block height (slot)
    pub height: u64,
    /// Consensus change type
    pub change_type: String,
    /// Updated metadata
    pub metadata: ConsensusMetadata,
    /// Timestamp of the update
    pub timestamp: u64,
}

/// Solana chain adapter (stub implementation)
#[cfg(feature = "solana")]
pub struct SolanaAdapter {
    chain_id: String,
    #[allow(dead_code)]
    config: ChainConfig,
}

#[cfg(feature = "solana")]
impl SolanaAdapter {
    /// Create a new Solana adapter
    pub async fn new(config: ChainConfig) -> Result<Self, ChainAdapterError> {
        // Validate the RPC URL format
        if !config.rpc_url.starts_with("http://") && !config.rpc_url.starts_with("https://") {
            return Err(ChainAdapterError::Configuration(
                "Invalid RPC URL format for Solana".to_string(),
            ));
        }

        Ok(Self {
            chain_id: config.chain_id.clone(),
            config,
        })
    }
}

#[cfg(feature = "solana")]
#[async_trait]
impl ChainAdapter for SolanaAdapter {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }

    async fn fetch_block(&self, block_number: u64) -> Result<ChainDataIR, ChainAdapterError> {
        // For minimal e2e implementation, create mock block data based on slot number
        let block_metadata = BlockMetadata {
            block_height: block_number,
            block_hash: format!("0x{:064x}", block_number),
            parent_hash: format!("0x{:064x}", block_number.saturating_sub(1)),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            state_root: format!("state_root_{}", block_number),
            receipts_root: None, // Solana doesn't have receipts
            transactions_root: Some(format!("tx_root_{}", block_number)),
        };

        // Create minimal state changes for testing
        let state_changes = vec![StateChange {
            contract_address: format!("program_{}", block_number % 10),
            storage_slot: {
                let mut slot = [0u8; 32];
                let slot_bytes = block_number.to_be_bytes();
                slot[24..].copy_from_slice(&slot_bytes);
                slot
            },
            previous_value: None,
            new_value: {
                let mut new_value = [0u8; 32];
                let new_value_bytes = (block_number + 1).to_be_bytes();
                new_value[24..].copy_from_slice(&new_value_bytes);
                new_value
            },
            proof: None,
        }];

        let consensus_metadata = ConsensusMetadata {
            validator_set_hash: Some(format!("validator_set_{}", block_number)),
            proposer: Some(format!("leader_{}", block_number % 5)),
            round: Some(block_number),
            signatures: None,
            extra: std::collections::HashMap::new(),
        };

        Ok(ChainDataIR {
            block_metadata,
            state_changes,
            consensus_metadata,
            finality_status: if block_number < 32 {
                FinalityStatus::Pending
            } else {
                FinalityStatus::Finalized
            },
        })
    }

    async fn get_latest_block(&self) -> Result<u64, ChainAdapterError> {
        // For minimal implementation, simulate progressive block height
        let simulated_height = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 2; // New block every 2 seconds (roughly)

        Ok(simulated_height)
    }

    async fn check_block_finality(
        &self,
        block_number: u64,
        current_height: u64,
    ) -> Result<FinalityStatus, ChainAdapterError> {
        let confirmations = current_height.saturating_sub(block_number);

        // Solana finality rules (simplified)
        Ok(match confirmations {
            0 => FinalityStatus::Pending,
            1..=31 => FinalityStatus::Safe, // Less than 32 confirmations
            _ => FinalityStatus::Finalized, // 32+ confirmations (supermajority)
        })
    }

    async fn is_block_finalized(&self, block_number: u64) -> Result<bool, ChainAdapterError> {
        let latest = self.get_latest_block().await?;
        let confirmations = latest.saturating_sub(block_number);

        // Consider block finalized if it has 32+ confirmations
        Ok(confirmations >= 32)
    }

    async fn generate_storage_proofs(
        &self,
        _contract_address: &str,
        storage_keys: &[[u8; 32]],
        block_number: u64,
    ) -> Result<Vec<StorageProof>, ChainAdapterError> {
        // Generate mock storage proofs for minimal functionality
        let proofs = storage_keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let mut value = [0u8; 32];
                let value_bytes = (block_number + i as u64).to_be_bytes();
                value[24..].copy_from_slice(&value_bytes); // Put u64 in the last 8 bytes
                StorageProof {
                    key: *key,
                    value,
                    proof_nodes: vec![format!("merkle_proof_{}_{}", block_number, i).into_bytes()],
                    proof_type: ProofType::SparseMerkle,
                }
            })
            .collect();

        Ok(proofs)
    }

    async fn get_block_metadata(
        &self,
        block_number: u64,
    ) -> Result<BlockMetadata, ChainAdapterError> {
        Ok(BlockMetadata {
            block_height: block_number,
            block_hash: format!("0x{:064x}", block_number),
            parent_hash: format!("0x{:064x}", block_number.saturating_sub(1)),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            state_root: format!("state_root_{}", block_number),
            receipts_root: None, // Solana doesn't have receipts
            transactions_root: Some(format!("tx_root_{}", block_number)),
        })
    }

    async fn extract_state_changes(
        &self,
        block_number: u64,
    ) -> Result<Vec<StateChange>, ChainAdapterError> {
        // Generate minimal state changes for testing
        let mut storage_slot = [0u8; 32];
        let mut new_value = [0u8; 32];
        let slot_bytes = block_number.to_be_bytes();
        let value_bytes = (block_number + 1).to_be_bytes();
        storage_slot[24..].copy_from_slice(&slot_bytes);
        new_value[24..].copy_from_slice(&value_bytes);

        let state_changes = vec![StateChange {
            contract_address: format!("program_{}", block_number % 10),
            storage_slot,
            previous_value: None,
            new_value,
            proof: None,
        }];

        Ok(state_changes)
    }

    async fn subscribe_blocks(
        &self,
    ) -> Result<Box<dyn futures::Stream<Item = u64> + Send>, ChainAdapterError> {
        use futures::stream;
        use std::time::Duration;

        // Create a simple stream that emits progressive block numbers every 2 seconds
        let stream = stream::unfold(0u64, |mut block_num| async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            block_num += 1;
            Some((block_num, block_num))
        });

        Ok(Box::new(stream))
    }
}

// Stub implementation when solana feature is disabled
#[cfg(not(feature = "solana"))]
pub struct SolanaAdapter;

#[cfg(not(feature = "solana"))]
impl SolanaAdapter {
    pub async fn new(_config: ChainConfig) -> Result<Self, ChainAdapterError> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "solana")]
    async fn test_solana_adapter_creation() {
        let config = ChainConfig {
            chain_id: "solana".to_string(),
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            api_key: None,
            confirmations: 32,
            max_retries: 3,
            timeout_secs: 30,
            extra: serde_json::Value::Null,
        };

        let adapter = SolanaAdapter::new(config).await;
        assert!(adapter.is_ok());
    }
}
