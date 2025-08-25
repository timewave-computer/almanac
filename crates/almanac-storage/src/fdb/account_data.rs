//! Account data implementations for subscription management

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Error type for merkle tree operations
#[derive(Debug, thiserror::Error)]
pub enum MerkleTreeError {
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Trait for account data that can be stored in merkle trees
pub trait AccountData {
    fn account_id(&self) -> &[u8];
    fn data_hash(&self) -> [u8; 32];
    fn chain_id(&self) -> &str;
    fn serialize(&self) -> Result<Vec<u8>, MerkleTreeError>;
    fn deserialize(data: &[u8]) -> Result<Self, MerkleTreeError>
    where
        Self: Sized;
}

// Removed circular dependency - define local types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AccountAddress {
    pub address: Vec<u8>,
    pub chain: String,
}

impl AccountAddress {
    pub fn new(address: Vec<u8>, chain: String) -> Self {
        Self { address, chain }
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.address)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdate {
    pub address: AccountAddress,
    pub data: Vec<u8>,
    pub slot: u64,
}

/// Chain-agnostic account data for Merkle tree storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainAccountData {
    /// Account identifier (address/pubkey)
    pub account_id: Vec<u8>,
    /// Chain identifier
    pub chain_id: String,
    /// Raw account data
    pub data: Vec<u8>,
    /// Block height when this data was last updated
    pub block_height: u64,
    /// Timestamp when this data was last updated
    pub timestamp: u64,
}

impl ChainAccountData {
    /// Create new account data
    pub fn new(
        account_id: Vec<u8>,
        chain_id: String,
        data: Vec<u8>,
        block_height: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            account_id,
            chain_id,
            data,
            block_height,
            timestamp,
        }
    }
}

impl AccountData for ChainAccountData {
    fn account_id(&self) -> &[u8] {
        &self.account_id
    }

    fn data_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.account_id);
        hasher.update(self.chain_id.as_bytes());
        hasher.update(&self.data);
        hasher.update(self.block_height.to_be_bytes());
        hasher.update(self.timestamp.to_be_bytes());
        hasher.finalize().into()
    }

    fn chain_id(&self) -> &str {
        &self.chain_id
    }

    fn serialize(&self) -> Result<Vec<u8>, MerkleTreeError> {
        bincode::serialize(self).map_err(|e| MerkleTreeError::SerializationError(e.to_string()))
    }

    fn deserialize(data: &[u8]) -> Result<Self, MerkleTreeError> {
        bincode::deserialize(data).map_err(|e| MerkleTreeError::SerializationError(e.to_string()))
    }
}

impl From<AccountUpdate> for ChainAccountData {
    fn from(update: AccountUpdate) -> Self {
        Self {
            account_id: update.address.address.clone(),
            chain_id: update.address.chain.clone(),
            data: update.data,
            block_height: update.slot,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
}

impl From<AccountAddress> for ChainAccountData {
    fn from(address: AccountAddress) -> Self {
        Self {
            account_id: address.address.clone(),
            chain_id: address.chain.clone(),
            data: Vec::new(), // Empty data for new subscriptions
            block_height: 0,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_account_data_creation() {
        let data = ChainAccountData::new(
            vec![1, 2, 3, 4],
            "test".to_string(),
            vec![10, 20, 30],
            100,
            1234567890,
        );

        assert_eq!(data.account_id(), &[1, 2, 3, 4]);
        assert_eq!(data.chain_id(), "test");
        assert_eq!(data.block_height, 100);
        assert_eq!(data.timestamp, 1234567890);
    }

    #[test]
    fn test_data_hash_consistency() {
        let data1 = ChainAccountData::new(
            vec![1, 2, 3, 4],
            "test".to_string(),
            vec![10, 20, 30],
            100,
            1234567890,
        );

        let data2 = ChainAccountData::new(
            vec![1, 2, 3, 4],
            "test".to_string(),
            vec![10, 20, 30],
            100,
            1234567890,
        );

        assert_eq!(data1.data_hash(), data2.data_hash());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let original = ChainAccountData::new(
            vec![1, 2, 3, 4],
            "test".to_string(),
            vec![10, 20, 30],
            100,
            1234567890,
        );

        let serialized = AccountData::serialize(&original).unwrap();
        let deserialized: ChainAccountData =
            AccountData::deserialize(&serialized).unwrap();

        assert_eq!(original.account_id, deserialized.account_id);
        assert_eq!(original.chain_id, deserialized.chain_id);
        assert_eq!(original.data, deserialized.data);
        assert_eq!(original.block_height, deserialized.block_height);
        assert_eq!(original.timestamp, deserialized.timestamp);
    }
}
