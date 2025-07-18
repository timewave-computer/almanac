//! Solana chain adapter implementation

#[cfg(feature = "solana")]
use async_trait::async_trait;
#[cfg(feature = "solana")]
use solana_client::rpc_client::RpcClient;
#[cfg(feature = "solana")]
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    clock::Slot,
};
#[cfg(feature = "solana")]
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
#[cfg(feature = "solana")]
use std::sync::Arc;
#[cfg(feature = "solana")]
use tracing::{debug, info};

#[cfg(feature = "solana")]
use crate::adapters::{
    traits::{ChainAdapter, ChainAdapterError},
    types::{BlockMetadata, ChainConfig, ChainDataIR, ConsensusMetadata, FinalityStatus, StateChange, StorageProof, ProofType},
};
#[cfg(feature = "solana")]
use std::collections::HashMap;
#[cfg(feature = "solana")]
use std::time::Duration;
#[cfg(feature = "solana")]
use serde::{Deserialize, Serialize};

/// Solana consensus configuration
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

/// Solana chain adapter
pub struct SolanaAdapter {
    client: Arc<RpcClient>,
    chain_id: String,
    config: ChainConfig,
}

impl SolanaAdapter {
    /// Create a new Solana adapter
    pub async fn new(config: ChainConfig) -> Result<Self, ChainAdapterError> {
        let client = Arc::new(RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        ));

        // Verify connection
        let version = client
            .get_version()
            .map_err(|e| ChainAdapterError::Network(format!("Failed to connect: {}", e)))?;

        info!("Connected to Solana version: {}", version.solana_core);

        Ok(Self {
            client,
            chain_id: config.chain_id.clone(),
            config,
        })
    }

    /// Convert Solana hash to [u8; 32]
    fn hash_to_bytes32(hash: &solana_sdk::hash::Hash) -> [u8; 32] {
        hash.to_bytes()
    }

    /// Convert pubkey to storage key format
    fn pubkey_to_storage_key(pubkey: &Pubkey) -> [u8; 32] {
        pubkey.to_bytes()
    }

    /// Extract state changes from account updates
    async fn extract_state_changes_from_slot(
        &self,
        slot: Slot,
    ) -> Result<Vec<StateChange>, ChainAdapterError> {
        let mut state_changes = Vec::new();

        // Get block with transactions
        let block = self.client
            .get_block_with_encoding(
                slot,
                solana_transaction_status::UiTransactionEncoding::Base64,
            )
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get block: {}", e)))?;

        // Process transactions for account changes
        if let Some(transactions) = block.transactions {
            for (tx_index, tx_with_meta) in transactions.iter().enumerate() {
                if let Some(meta) = &tx_with_meta.meta {
                    // Process pre and post token balances
                    if let Some(pre_token_balances) = &meta.pre_token_balances {
                        if let Some(post_token_balances) = &meta.post_token_balances {
                            for (i, pre_balance) in pre_token_balances.iter().enumerate() {
                                if let Some(post_balance) = post_token_balances.get(i) {
                                    if pre_balance.ui_token_amount.amount != post_balance.ui_token_amount.amount {
                                        // Token balance changed
                                        let account_index = pre_balance.account_index as usize;
                                        if let Some(account_key) = tx_with_meta.transaction
                                            .message()
                                            .account_keys()
                                            .get(account_index) 
                                        {
                                            let mut prev_value = [0u8; 32];
                                            let mut new_value = [0u8; 32];
                                            
                                            // Store amounts in first 8 bytes (u64)
                                            let prev_amount = pre_balance.ui_token_amount.amount.parse::<u64>()
                                                .unwrap_or(0);
                                            let new_amount = post_balance.ui_token_amount.amount.parse::<u64>()
                                                .unwrap_or(0);
                                            
                                            prev_value[..8].copy_from_slice(&prev_amount.to_le_bytes());
                                            new_value[..8].copy_from_slice(&new_amount.to_le_bytes());

                                            state_changes.push(StateChange {
                                                contract_address: account_key.to_string(),
                                                storage_slot: Self::pubkey_to_storage_key(account_key),
                                                previous_value: Some(prev_value),
                                                new_value,
                                                proof: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Process account state changes from logs
                    if let Some(log_messages) = &meta.log_messages {
                        for (log_index, log) in log_messages.iter().enumerate() {
                            if log.contains("Program log: state:") {
                                // Extract state changes from program logs
                                // This is simplified - real implementation would parse specific program logs
                                debug!("Found state change in log: {}", log);
                            }
                        }
                    }
                }
            }
        }

        Ok(state_changes)
    }

    /// Get finality status based on commitment
    fn get_finality_status(&self, slot: Slot, latest_slot: Slot) -> FinalityStatus {
        let age = latest_slot.saturating_sub(slot);
        
        match age {
            0..=31 => FinalityStatus::Pending,     // ~12.8 seconds (32 slots)
            32..=63 => FinalityStatus::Safe,       // ~25.6 seconds (64 slots)  
            _ => FinalityStatus::Finalized,        // Finalized after ~32 seconds
        }
    }
}

#[async_trait]
impl ChainAdapter for SolanaAdapter {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }
    

    async fn fetch_block(&self, block_number: u64) -> Result<ChainDataIR, ChainAdapterError> {
        debug!("Fetching Solana slot {}", block_number);

        let slot = block_number as Slot;

        // Get block
        let block = self.client
            .get_block(slot)
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get block: {}", e)))?;

        // Get latest slot for finality
        let latest_slot = self.client
            .get_slot()
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get latest slot: {}", e)))?;

        // Extract state changes
        let state_changes = self.extract_state_changes_from_slot(slot).await?;

        // Get finality status
        let finality_status = self.get_finality_status(slot, latest_slot);

        // Build block metadata
        let block_metadata = BlockMetadata {
            block_height: slot,
            block_hash: hex::encode(block.blockhash.to_bytes()),
            parent_hash: hex::encode(block.previous_blockhash.to_bytes()),
            timestamp: block.block_time.unwrap_or(0) as u64,
            state_root: hex::encode([0u8; 32]), // Solana doesn't have a single state root
            receipts_root: None,
            transactions_root: None,
        };

        // Build consensus metadata
        let mut extra = HashMap::new();
        extra.insert("slot".to_string(), serde_json::json!(slot));
        extra.insert("block_height".to_string(), serde_json::json!(block.block_height));
        extra.insert("parent_slot".to_string(), serde_json::json!(block.parent_slot));
        extra.insert("transactions_count".to_string(), serde_json::json!(block.transactions.as_ref().map(|t| t.len()).unwrap_or(0)));
        
        let consensus_metadata = ConsensusMetadata {
            validator_set_hash: None,
            proposer: block.rewards
                .as_ref()
                .and_then(|r| r.first())
                .map(|r| r.pubkey.clone()),
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
            .get_slot()
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get latest slot: {}", e)))
    }

    
    async fn check_block_finality(&self, block_number: u64, current_height: u64) -> Result<FinalityStatus, ChainAdapterError> {
        let slot = block_number as Slot;
        let latest_slot = current_height as Slot;
        
        Ok(self.get_finality_status(slot, latest_slot))
    }

    async fn is_block_finalized(&self, block_number: u64) -> Result<bool, ChainAdapterError> {
        let slot = block_number as Slot;
        
        // Check if slot is finalized
        let finalized_slot = self.client
            .get_slot_with_commitment(CommitmentConfig::finalized())
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get finalized slot: {}", e)))?;

        Ok(slot <= finalized_slot)
    }

    async fn generate_storage_proofs(
        &self,
        contract_address: &str,
        storage_keys: &[[u8; 32]],
        block_number: u64,
    ) -> Result<Vec<StorageProof>, ChainAdapterError> {
        let slot = block_number as Slot;
        let pubkey = Pubkey::from_str(contract_address)
            .map_err(|e| ChainAdapterError::Other(format!("Invalid pubkey: {}", e)))?;

        let mut proofs = Vec::new();

        // Get account info at specific slot
        let account = self.client
            .get_account_with_commitment(&pubkey, CommitmentConfig::confirmed())
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get account: {}", e)))?
            .value;

        if let Some(account_data) = account {
            // For each requested storage key, generate a proof
            // Note: Solana doesn't have the same Merkle proof structure as Ethereum
            // This is a simplified version - real implementation would need proper proof generation
            for storage_key in storage_keys {
                let mut value = [0u8; 32];
                
                // Extract value from account data based on storage key
                // This is highly simplified - real implementation would parse account data
                if account_data.data.len() >= 32 {
                    value.copy_from_slice(&account_data.data[..32]);
                }

                let storage_proof = StorageProof {
                    key: *storage_key,
                    value,
                    proof_nodes: vec![], // Solana proof generation would go here
                    proof_type: ProofType::Custom("solana".to_string()),
                };

                proofs.push(storage_proof);
            }
        }

        Ok(proofs)
    }

    async fn get_block_metadata(&self, block_number: u64) -> Result<BlockMetadata, ChainAdapterError> {
        let slot = block_number as Slot;

        let block = self.client
            .get_block(slot)
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get block: {}", e)))?;

        let latest_slot = self.client
            .get_slot()
            .map_err(|e| ChainAdapterError::Rpc(format!("Failed to get latest slot: {}", e)))?;

        let finality_status = self.get_finality_status(slot, latest_slot);

        Ok(BlockMetadata {
            block_height: slot,
            block_hash: hex::encode(block.blockhash.to_bytes()),
            parent_hash: hex::encode(block.previous_blockhash.to_bytes()),
            timestamp: block.block_time.unwrap_or(0) as u64,
            state_root: hex::encode([0u8; 32]),
            receipts_root: None,
            transactions_root: None,
        })
    }

    async fn extract_state_changes(&self, block_number: u64) -> Result<Vec<StateChange>, ChainAdapterError> {
        let slot = block_number as Slot;
        self.extract_state_changes_from_slot(slot).await
    }

    async fn subscribe_blocks(&self) -> Result<Box<dyn futures::Stream<Item = u64> + Send>, ChainAdapterError> {
        // Solana WebSocket subscription would be implemented here
        Err(ChainAdapterError::Unsupported(
            "Block subscription not yet implemented for Solana".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_conversion() {
        let hash = solana_sdk::hash::Hash::new_unique();
        let bytes32 = SolanaAdapter::hash_to_bytes32(&hash);
        assert_eq!(bytes32, hash.to_bytes());
    }

    #[test]
    fn test_pubkey_to_storage_key() {
        let pubkey = Pubkey::new_unique();
        let storage_key = SolanaAdapter::pubkey_to_storage_key(&pubkey);
        assert_eq!(storage_key, pubkey.to_bytes());
    }

    #[test]
    fn test_finality_status() {
        let adapter = SolanaAdapter {
            client: Arc::new(RpcClient::new("http://localhost")),
            chain_id: "solana-devnet".to_string(),
            config: ChainConfig {
                chain_id: "solana-devnet".to_string(),
                rpc_url: "http://localhost".to_string(),
                api_key: None,
                confirmations: 32,
                max_retries: 3,
                timeout_secs: 30,
                extra: serde_json::Value::Null,
            },
        };

        assert_eq!(adapter.get_finality_status(100, 100), FinalityStatus::Pending);
        assert_eq!(adapter.get_finality_status(100, 140), FinalityStatus::Safe);
        assert_eq!(adapter.get_finality_status(100, 200), FinalityStatus::Finalized);
    }
}

// Stub implementation when solana feature is disabled
#[cfg(not(feature = "solana"))]
pub struct SolanaAdapter;

#[cfg(not(feature = "solana"))]
impl SolanaAdapter {
    pub async fn new(_config: crate::adapters::ChainConfig) -> Result<Self, crate::adapters::ChainAdapterError> {
        Err(crate::adapters::ChainAdapterError::Configuration("Solana support not enabled".to_string()))
    }
}