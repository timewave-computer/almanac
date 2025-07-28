//! Generic merkle tree trait for chain-specific implementations
//!
//! This module defines a common interface that each blockchain can implement
//! with its native merkle tree type.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use thiserror::Error;

/// Errors related to merkle tree operations
#[derive(Error, Debug)]
pub enum MerkleTreeError {
    #[error("Invalid key length: expected {expected}, got {got}")]
    InvalidKeyLength { expected: usize, got: usize },

    #[error("Invalid proof")]
    InvalidProof,

    #[error("Proof verification failed")]
    ProofVerificationFailed,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Tree error: {0}")]
    TreeError(String),
}

/// Generic account data that can be stored in the merkle tree
pub trait AccountData: Send + Sync + Clone + Debug {
    /// Get the account identifier (address/pubkey)
    fn account_id(&self) -> &[u8];

    /// Get the hash of the account data
    fn data_hash(&self) -> [u8; 32];

    /// Get the chain identifier
    fn chain_id(&self) -> &str;

    /// Serialize the account data for storage
    fn serialize(&self) -> Result<Vec<u8>, MerkleTreeError>;

    /// Deserialize account data from storage
    fn deserialize(data: &[u8]) -> Result<Self, MerkleTreeError>
    where
        Self: Sized;
}

/// Merkle tree inclusion proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InclusionProof {
    /// The account data being proven
    pub account_data: Vec<u8>,
    /// The key (account ID) being proven
    pub key: Vec<u8>,
    /// The root hash at the time of proof generation
    pub root_hash: [u8; 32],
    /// Chain-specific proof data
    pub proof_data: Vec<u8>,
}

/// Statistics about the merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTreeStats {
    pub leaf_count: usize,
    pub root_hash: [u8; 32],
    pub memory_usage: usize,
    pub tree_depth: usize,
}

/// Generic merkle tree trait that each chain implements
#[async_trait]
pub trait ChainMerkleTree: Send + Sync {
    /// The account data type for this chain
    type AccountData: AccountData;

    /// Get the current root hash
    async fn root(&self) -> [u8; 32];

    /// Get the number of leaves in the tree
    async fn leaf_count(&self) -> usize;

    /// Insert or update an account in the tree
    async fn update(&self, account_data: Self::AccountData) -> Result<[u8; 32], MerkleTreeError>;

    /// Remove an account from the tree
    async fn remove(&self, account_id: &[u8]) -> Result<Option<Self::AccountData>, MerkleTreeError>;

    /// Get account data by ID
    async fn get(&self, account_id: &[u8]) -> Option<Self::AccountData>;

    /// Generate an inclusion proof for an account
    async fn generate_proof(&self, account_id: &[u8]) -> Result<InclusionProof, MerkleTreeError>;

    /// Verify an inclusion proof
    async fn verify_proof(&self, proof: &InclusionProof) -> Result<bool, MerkleTreeError>;

    /// Get all account IDs in the tree
    async fn get_all_account_ids(&self) -> Vec<Vec<u8>>;

    /// Get tree statistics
    async fn stats(&self) -> MerkleTreeStats;

    /// Batch update multiple accounts
    async fn batch_update(&self, accounts: Vec<Self::AccountData>) -> Result<[u8; 32], MerkleTreeError> {
        let mut root = self.root().await;
        for account in accounts {
            root = self.update(account).await?;
        }
        Ok(root)
    }
}

/// Batch processor trait for efficient tree updates
#[async_trait]
pub trait BatchProcessor<T: AccountData>: Send + Sync {
    /// Add an account update to the batch
    async fn add_update(&self, account_data: T) -> Result<(), MerkleTreeError>;

    /// Process all pending updates
    async fn process_batch(&self) -> Result<[u8; 32], MerkleTreeError>;

    /// Get the number of pending updates
    async fn pending_count(&self) -> usize;
}