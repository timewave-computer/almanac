//! Enhanced block metadata storage for multi-chain consensus tracking
//!
//! This module provides comprehensive block metadata storage that includes
//! state roots, validator sets, and other consensus data critical for
//! Lightwave network coordination and Traverse proof generation.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use almanac_infrastructure_core::{Error, Result};
use super::{TupleKey, TupleKeyspace, keys};

/// Block identifier that can represent different chain block formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockIdentifier {
    /// Block height (most common)
    Height(u64),
    /// Block hash (for specific block reference)
    Hash(String),
    /// Slot number (Solana, Ethereum 2.0)
    Slot(u64),
    /// Epoch and slot (Ethereum 2.0 beacon chain)
    EpochSlot { epoch: u64, slot: u64 },
}

impl BlockIdentifier {
    /// Get the primary ordering value for this identifier
    pub fn ordering_value(&self) -> u64 {
        match self {
            BlockIdentifier::Height(h) => *h,
            BlockIdentifier::Slot(s) => *s,
            BlockIdentifier::EpochSlot { slot, .. } => *slot,
            BlockIdentifier::Hash(_) => 0, // Hash can't be ordered
        }
    }
}

/// Block metadata key for tuple storage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDataKey {
    /// Chain identifier
    pub chain_id: String,
    /// Block identifier
    pub block_identifier: BlockIdentifier,
}

impl BlockDataKey {
    /// Create a new block data key
    pub fn new(chain_id: String, block_identifier: BlockIdentifier) -> Self {
        Self {
            chain_id,
            block_identifier,
        }
    }

    /// Convert to tuple key for storage
    pub fn to_tuple_key(&self) -> TupleKey {
        match &self.block_identifier {
            BlockIdentifier::Height(height) => {
                keys::block_meta(&self.chain_id, *height)
            }
            BlockIdentifier::Slot(slot) => {
                TupleKey::new(TupleKeyspace::BlockMeta)
                    .string(&self.chain_id)
                    .string("slot")
                    .int(*slot as i64)
            }
            BlockIdentifier::Hash(hash) => {
                TupleKey::new(TupleKeyspace::BlockMeta)
                    .string(&self.chain_id)
                    .string("hash")
                    .string(hash)
            }
            BlockIdentifier::EpochSlot { epoch, slot } => {
                TupleKey::new(TupleKeyspace::BlockMeta)
                    .string(&self.chain_id)
                    .string("epoch_slot")
                    .int(*epoch as i64)
                    .int(*slot as i64)
            }
        }
    }
}

/// Comprehensive block metadata value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDataValue {
    /// Block height (primary ordering)
    pub block_height: u64,
    /// Block hash
    pub block_hash: [u8; 32],
    /// State root (critical for Traverse)
    pub state_root: [u8; 32],
    /// Parent block hash
    pub parent_hash: [u8; 32],
    /// Block timestamp
    pub timestamp: u64,
    
    // Ethereum specific fields
    /// Gas limit (Ethereum)
    pub gas_limit: Option<u64>,
    /// Gas used (Ethereum)
    pub gas_used: Option<u64>,
    /// Difficulty (Ethereum PoW)
    pub difficulty: Option<String>,
    /// Base fee per gas (Ethereum EIP-1559)
    pub base_fee_per_gas: Option<u64>,
    
    // Tendermint/Cosmos specific fields
    /// Validator set hash (Tendermint)
    pub validator_set_hash: Option<[u8; 32]>,
    /// App hash (Tendermint application state)
    pub app_hash: Option<[u8; 32]>,
    /// Consensus params hash
    pub consensus_params_hash: Option<[u8; 32]>,
    /// Evidence hash
    pub evidence_hash: Option<[u8; 32]>,
    
    // Solana specific fields
    /// Previous blockhash (Solana)
    pub previous_blockhash: Option<String>,
    /// Bank hash (Solana)
    pub bank_hash: Option<String>,
    /// Leader (Solana)
    pub leader: Option<String>,
    
    // General consensus metadata
    /// Finality status
    pub finality_status: FinalityStatus,
    /// Number of confirmations
    pub confirmations: u64,
    /// When this block data was stored
    pub indexed_at: u64,
    /// Additional chain-specific metadata
    pub metadata: HashMap<String, String>,
}

/// Finality status for blocks across different consensus mechanisms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinalityStatus {
    /// Block is pending (not yet included)
    Pending,
    /// Block is included but not confirmed
    Included,
    /// Block has some confirmations
    Confirmed { confirmations: u64 },
    /// Block is finalized (irreversible)
    Finalized,
    /// Block was reorganized out
    Reorganized,
}

impl BlockDataValue {
    /// Create a new block data value with minimal required fields
    pub fn new(
        block_height: u64,
        block_hash: [u8; 32],
        state_root: [u8; 32],
        parent_hash: [u8; 32],
        timestamp: u64,
    ) -> Self {
        Self {
            block_height,
            block_hash,
            state_root,
            parent_hash,
            timestamp,
            gas_limit: None,
            gas_used: None,
            difficulty: None,
            base_fee_per_gas: None,
            validator_set_hash: None,
            app_hash: None,
            consensus_params_hash: None,
            evidence_hash: None,
            previous_blockhash: None,
            bank_hash: None,
            leader: None,
            finality_status: FinalityStatus::Included,
            confirmations: 0,
            indexed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: HashMap::new(),
        }
    }

    /// Add Ethereum-specific fields
    pub fn with_ethereum_data(
        mut self,
        gas_limit: u64,
        gas_used: u64,
        base_fee_per_gas: Option<u64>,
        difficulty: Option<String>,
    ) -> Self {
        self.gas_limit = Some(gas_limit);
        self.gas_used = Some(gas_used);
        self.base_fee_per_gas = base_fee_per_gas;
        self.difficulty = difficulty;
        self
    }

    /// Add Tendermint/Cosmos specific fields
    pub fn with_tendermint_data(
        mut self,
        validator_set_hash: [u8; 32],
        app_hash: [u8; 32],
        consensus_params_hash: Option<[u8; 32]>,
        evidence_hash: Option<[u8; 32]>,
    ) -> Self {
        self.validator_set_hash = Some(validator_set_hash);
        self.app_hash = Some(app_hash);
        self.consensus_params_hash = consensus_params_hash;
        self.evidence_hash = evidence_hash;
        self
    }

    /// Add Solana specific fields
    pub fn with_solana_data(
        mut self,
        previous_blockhash: String,
        bank_hash: String,
        leader: String,
    ) -> Self {
        self.previous_blockhash = Some(previous_blockhash);
        self.bank_hash = Some(bank_hash);
        self.leader = Some(leader);
        self
    }

    /// Set finality status
    pub fn with_finality_status(mut self, status: FinalityStatus) -> Self {
        self.finality_status = status;
        self
    }

    /// Add confirmations
    pub fn with_confirmations(mut self, confirmations: u64) -> Self {
        self.confirmations = confirmations;
        if confirmations >= 6 {
            self.finality_status = FinalityStatus::Confirmed { confirmations };
        }
        self
    }

    /// Add custom metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if the block is considered final for the given chain
    pub fn is_final(&self, chain_finality_threshold: u64) -> bool {
        match self.finality_status {
            FinalityStatus::Finalized => true,
            FinalityStatus::Confirmed { confirmations } => confirmations >= chain_finality_threshold,
            _ => false,
        }
    }

    /// Validate the block data for consistency
    pub fn validate(&self) -> Result<()> {
        // Check timestamp is reasonable
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if self.timestamp > now + 300 { // Allow 5 minutes clock skew
            return Err(Error::InvalidData("Block timestamp in the future".to_string()));
        }

        // Check that indexed_at is not before block timestamp
        if self.indexed_at < self.timestamp {
            return Err(Error::InvalidData("Indexed time before block timestamp".to_string()));
        }

        // Validate Ethereum fields if present
        if let (Some(gas_used), Some(gas_limit)) = (self.gas_used, self.gas_limit) {
            if gas_used > gas_limit {
                return Err(Error::InvalidData("Gas used exceeds gas limit".to_string()));
            }
        }

        Ok(())
    }
}

/// Block reorganization event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockReorgEvent {
    /// Chain where reorganization occurred
    pub chain_id: String,
    /// Block height where reorg started
    pub reorg_height: u64,
    /// Old block hash that was replaced
    pub old_block_hash: [u8; 32],
    /// New block hash that replaced it
    pub new_block_hash: [u8; 32],
    /// Depth of the reorganization
    pub reorg_depth: u64,
    /// When the reorg was detected
    pub detected_at: u64,
}

/// Latest block tracker for efficient queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestBlockInfo {
    /// Chain identifier
    pub chain_id: String,
    /// Latest block height
    pub latest_height: u64,
    /// Latest block hash
    pub latest_hash: [u8; 32],
    /// Latest finalized height
    pub finalized_height: u64,
    /// When this was last updated
    pub updated_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_identifier_ordering() {
        let height = BlockIdentifier::Height(100);
        let slot = BlockIdentifier::Slot(100);
        let epoch_slot = BlockIdentifier::EpochSlot { epoch: 5, slot: 100 };

        assert_eq!(height.ordering_value(), 100);
        assert_eq!(slot.ordering_value(), 100);
        assert_eq!(epoch_slot.ordering_value(), 100);
    }

    #[test]
    fn test_block_data_validation() {
        let block_data = BlockDataValue::new(
            12345,
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1640995200, // 2022-01-01
        ).with_ethereum_data(21000000, 15000000, Some(20), None);

        assert!(block_data.validate().is_ok());

        // Test invalid gas usage
        let invalid_block = BlockDataValue::new(
            12345,
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1640995200,
        ).with_ethereum_data(15000000, 21000000, Some(20), None); // gas_used > gas_limit

        assert!(invalid_block.validate().is_err());
    }

    #[test]
    fn test_finality_status() {
        let mut block = BlockDataValue::new(
            12345,
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            1640995200,
        );

        assert!(!block.is_final(6));

        block = block.with_confirmations(10);
        assert!(block.is_final(6));

        block = block.with_finality_status(FinalityStatus::Finalized);
        assert!(block.is_final(100)); // Always final if explicitly finalized
    }

    #[test]
    fn test_tuple_key_generation() {
        let key = BlockDataKey::new(
            "ethereum".to_string(),
            BlockIdentifier::Height(12345),
        );

        let tuple_key = key.to_tuple_key();
        let packed = tuple_key.pack();
        assert!(!packed.is_empty());

        // Test different identifier types
        let slot_key = BlockDataKey::new(
            "solana".to_string(),
            BlockIdentifier::Slot(67890),
        );

        let slot_tuple = slot_key.to_tuple_key();
        assert_ne!(tuple_key.pack(), slot_tuple.pack());
    }
} 