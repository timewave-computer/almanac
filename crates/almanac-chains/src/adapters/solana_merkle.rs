//! Solana-specific Merkle tree implementation using the generic component
//!
//! This module provides a Solana-specific wrapper around the generic sparse Merkle tree
//! for efficient Solana account state verification and inclusion proofs.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[cfg(feature = "solana")]
use solana_sdk::{account::Account, pubkey::Pubkey};

use crate::merkle_tree::{
    AccountData, BatchProcessor, InclusionProof, MerkleTreeError, MerkleTreeStats, SparseMerkleTree,
};
use crate::subscription::AccountSubscriptionError;

#[cfg(feature = "solana")]
use crate::adapters::solana_geyser::RealGeyserAccountUpdate;

/// Errors specific to Solana Merkle tree operations
#[derive(Error, Debug)]
pub enum SolanaMerkleError {
    #[error("Generic Merkle tree error: {0}")]
    MerkleTreeError(#[from] MerkleTreeError),

    #[error("Account subscription error: {0}")]
    SubscriptionError(#[from] AccountSubscriptionError),

    #[error("Invalid Solana pubkey: {0}")]
    InvalidPubkey(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Solana account data that implements the generic AccountData trait
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SolanaAccountData {
    pub address: [u8; 32],   // Solana account address (32 bytes)
    pub lamports: u64,       // SOL balance in lamports
    pub owner: [u8; 32],     // Program that owns this account
    pub executable: bool,    // Whether the account is executable
    pub rent_epoch: u64,     // Rent collection epoch
    pub data_hash: [u8; 32], // Hash of account data
    pub slot: u64,           // Slot when account was last updated
    pub write_version: u64,  // Account write version
}

impl AccountData for SolanaAccountData {
    fn account_id(&self) -> &[u8] {
        &self.address
    }

    fn data_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.address);
        hasher.update(self.lamports.to_le_bytes());
        hasher.update(self.owner);
        hasher.update([self.executable as u8]);
        hasher.update(self.rent_epoch.to_le_bytes());
        hasher.update(self.data_hash);
        hasher.update(self.slot.to_le_bytes());
        hasher.update(self.write_version.to_le_bytes());
        hasher.finalize().into()
    }

    fn chain_id(&self) -> &str {
        "solana"
    }

    fn serialize(&self) -> Result<Vec<u8>, MerkleTreeError> {
        bincode::serialize(self).map_err(|e| MerkleTreeError::SerializationError(e.to_string()))
    }

    fn deserialize(data: &[u8]) -> Result<Self, MerkleTreeError> {
        bincode::deserialize(data).map_err(|e| MerkleTreeError::SerializationError(e.to_string()))
    }
}

impl SolanaAccountData {
    /// Create from real Geyser account update
    #[cfg(feature = "solana")]
    pub fn from_geyser_update(update: &RealGeyserAccountUpdate) -> Result<Self, SolanaMerkleError> {
        let address = update.pubkey.to_bytes();
        let owner = update.account.owner.to_bytes();
        let data_hash = Sha256::digest(&update.account.data);

        Ok(Self {
            address,
            lamports: update.account.lamports,
            owner,
            executable: update.account.executable,
            rent_epoch: update.account.rent_epoch,
            data_hash: data_hash.into(),
            slot: update.slot,
            write_version: update.write_version,
        })
    }

    /// Create from Solana account and metadata
    #[cfg(feature = "solana")]
    pub fn from_account(pubkey: Pubkey, account: Account, slot: u64, write_version: u64) -> Self {
        let data_hash = Sha256::digest(&account.data);

        Self {
            address: pubkey.to_bytes(),
            lamports: account.lamports,
            owner: account.owner.to_bytes(),
            executable: account.executable,
            rent_epoch: account.rent_epoch,
            data_hash: data_hash.into(),
            slot,
            write_version,
        }
    }

    /// Create a placeholder instance for non-Solana builds
    #[cfg(not(feature = "solana"))]
    pub fn new_placeholder() -> Self {
        Self {
            address: [0u8; 32],
            lamports: 0,
            owner: [0u8; 32],
            executable: false,
            rent_epoch: 0,
            data_hash: [0u8; 32],
            slot: 0,
            write_version: 0,
        }
    }
}

/// Solana-specific Merkle tree manager using the generic implementation
pub struct SolanaMerkleTreeManager {
    tree: Arc<RwLock<SparseMerkleTree<SolanaAccountData>>>,
    batch_processor: Arc<RwLock<BatchProcessor<SolanaAccountData>>>,
    stats: Arc<RwLock<SolanaMerkleStats>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaMerkleStats {
    pub total_accounts: usize,
    pub tree_depth: u8,
    pub last_root_hash: [u8; 32],
    pub last_update_slot: u64,
    pub total_updates_processed: u64,
    pub touched_accounts_count: usize,
}

impl Default for SolanaMerkleStats {
    fn default() -> Self {
        Self {
            total_accounts: 0,
            tree_depth: 32,
            last_root_hash: [0u8; 32],
            last_update_slot: 0,
            total_updates_processed: 0,
            touched_accounts_count: 0,
        }
    }
}

impl SolanaMerkleTreeManager {
    /// Create a new Solana Merkle tree manager
    pub fn new(_depth: u8) -> Self {
        let tree1 = SparseMerkleTree::new();
        let tree2 = SparseMerkleTree::new();
        let batch_processor = BatchProcessor::new(tree2, 100); // Default batch size of 100

        Self {
            tree: Arc::new(RwLock::new(tree1)),
            batch_processor: Arc::new(RwLock::new(batch_processor)),
            stats: Arc::new(RwLock::new(SolanaMerkleStats::default())),
        }
    }

    /// Update an account in the Merkle tree
    pub async fn update_account(
        &self,
        pubkey_bytes: &[u8],
        data_hash: &[u8; 32],
    ) -> Result<[u8; 32], SolanaMerkleError> {
        // Create a minimal SolanaAccountData for the update
        let mut address = [0u8; 32];
        if pubkey_bytes.len() >= 32 {
            address.copy_from_slice(&pubkey_bytes[..32]);
        }

        let account_data = SolanaAccountData {
            address,
            lamports: 0,
            owner: [0u8; 32],
            executable: false,
            rent_epoch: 0,
            data_hash: *data_hash,
            slot: 0,
            write_version: 0,
        };

        let mut tree = self.tree.write().await;
        let new_root = tree.update(account_data)?;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.last_root_hash = new_root;
            stats.total_updates_processed += 1;
            stats.total_accounts = tree.leaf_count();
        }

        Ok(new_root)
    }

    /// Update account with full account information
    pub async fn update_account_full(
        &self,
        account_data: SolanaAccountData,
    ) -> Result<[u8; 32], SolanaMerkleError> {
        let mut tree = self.tree.write().await;
        let new_root = tree.update(account_data.clone())?;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.last_root_hash = new_root;
            stats.last_update_slot = account_data.slot;
            stats.total_updates_processed += 1;
            stats.total_accounts = tree.leaf_count();
        }

        Ok(new_root)
    }

    /// Add account update to batch processor
    pub async fn batch_update_account(
        &self,
        account_data: SolanaAccountData,
    ) -> Result<(), SolanaMerkleError> {
        let mut processor = self.batch_processor.write().await;
        processor.add_update(account_data);
        Ok(())
    }

    /// Process all pending batch updates
    pub async fn process_batch(&self) -> Result<[u8; 32], SolanaMerkleError> {
        let mut processor = self.batch_processor.write().await;
        let new_root = processor.process_batch()?;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.last_root_hash = new_root;
            stats.total_updates_processed += 1;
            stats.total_accounts = processor.tree().leaf_count();
        }

        Ok(new_root)
    }

    /// Generate inclusion proof for a Solana account
    #[cfg(feature = "solana")]
    pub async fn generate_inclusion_proof(
        &self,
        pubkey: &Pubkey,
    ) -> Result<InclusionProof, SolanaMerkleError> {
        let tree = self.tree.read().await;
        let proof = tree.generate_proof(pubkey.as_ref())?;
        Ok(proof)
    }

    /// Generate inclusion proof for account by address bytes
    pub async fn generate_inclusion_proof_by_address(
        &self,
        address: &[u8; 32],
    ) -> Result<InclusionProof, SolanaMerkleError> {
        let tree = self.tree.read().await;
        let proof = tree.generate_proof(address)?;
        Ok(proof)
    }

    /// Verify an inclusion proof
    pub async fn verify_inclusion_proof(
        &self,
        proof: &InclusionProof,
    ) -> Result<bool, SolanaMerkleError> {
        let tree = self.tree.read().await;
        Ok(tree.verify_proof(proof))
    }

    /// Get the current root hash
    pub async fn get_root(&self) -> [u8; 32] {
        let tree = self.tree.read().await;
        tree.root()
    }

    /// Get account data by pubkey
    #[cfg(feature = "solana")]
    pub async fn get_account(&self, pubkey: &Pubkey) -> Option<SolanaAccountData> {
        let tree = self.tree.read().await;
        tree.get(pubkey.as_ref()).cloned()
    }

    /// Get account data by address
    pub async fn get_account_by_address(&self, address: &[u8; 32]) -> Option<SolanaAccountData> {
        let tree = self.tree.read().await;
        tree.get(address).cloned()
    }

    /// Remove an account from the tree
    #[cfg(feature = "solana")]
    pub async fn remove_account(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Option<SolanaAccountData>, SolanaMerkleError> {
        let mut tree = self.tree.write().await;
        let removed = tree.remove(pubkey.as_ref())?;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_accounts = tree.leaf_count();
        }

        Ok(removed)
    }

    /// Get all account addresses in the tree
    pub async fn get_all_accounts(&self) -> Vec<[u8; 32]> {
        let tree = self.tree.read().await;
        tree.get_all_account_ids()
            .into_iter()
            .filter_map(|bytes| {
                if bytes.len() == 32 {
                    let mut address = [0u8; 32];
                    address.copy_from_slice(&bytes);
                    Some(address)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get tree statistics
    pub async fn get_stats(&self) -> SolanaMerkleStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get underlying generic tree statistics
    pub async fn get_generic_stats(&self) -> MerkleTreeStats {
        let tree = self.tree.read().await;
        tree.stats()
    }

    /// Get pending batch count
    pub async fn get_pending_batch_count(&self) -> usize {
        let processor = self.batch_processor.read().await;
        processor.pending_count()
    }
}

/// Solana-specific inclusion proof with account state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaInclusionProof {
    pub base_proof: InclusionProof,
    pub account_state: SolanaAccountData,
}

/// Solana Merkle tree checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaMerkleCheckpoint {
    pub slot: u64,
    pub root_hash: [u8; 32],
    pub touched_accounts: HashSet<[u8; 32]>,
    pub stats: SolanaMerkleStats,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_account_data(address: [u8; 32], lamports: u64) -> SolanaAccountData {
        SolanaAccountData {
            address,
            lamports,
            owner: [1u8; 32],
            executable: false,
            rent_epoch: 250,
            data_hash: [2u8; 32],
            slot: 1000,
            write_version: 1,
        }
    }

    #[tokio::test]
    async fn test_solana_merkle_tree_creation() {
        let manager = SolanaMerkleTreeManager::new(8);
        let root = manager.get_root().await;
        assert_ne!(root, [0u8; 32]); // Should have non-zero empty root
    }

    #[tokio::test]
    async fn test_solana_account_update() {
        let manager = SolanaMerkleTreeManager::new(8);
        let address = [1u8; 32];
        let account_data = create_test_account_data(address, 1_000_000);

        let root = manager
            .update_account_full(account_data.clone())
            .await
            .unwrap();
        assert_ne!(root, [0u8; 32]);

        let retrieved = manager.get_account_by_address(&address).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().address, address);
    }

    #[tokio::test]
    async fn test_inclusion_proof() {
        let manager = SolanaMerkleTreeManager::new(8);
        let address = [1u8; 32];
        let account_data = create_test_account_data(address, 1_000_000);

        manager.update_account_full(account_data).await.unwrap();

        let proof = manager
            .generate_inclusion_proof_by_address(&address)
            .await
            .unwrap();
        let is_valid = manager.verify_inclusion_proof(&proof).await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let manager = SolanaMerkleTreeManager::new(8);

        // Add multiple accounts to batch
        for i in 0..5 {
            let mut address = [0u8; 32];
            address[0] = i as u8;
            let account_data = create_test_account_data(address, 1_000_000 + i as u64);
            manager.batch_update_account(account_data).await.unwrap();
        }

        assert_eq!(manager.get_pending_batch_count().await, 5);

        let root = manager.process_batch().await.unwrap();
        assert_ne!(root, [0u8; 32]);
        assert_eq!(manager.get_pending_batch_count().await, 0);

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_accounts, 5);
    }

    #[test]
    fn test_account_data_trait() {
        let address = [1u8; 32];
        let account_data = create_test_account_data(address, 1_000_000);

        assert_eq!(account_data.account_id(), &address);
        assert_eq!(account_data.chain_id(), "solana");
        assert_ne!(account_data.data_hash(), [0u8; 32]);

        let serialized =
            <SolanaAccountData as crate::merkle_tree::AccountData>::serialize(&account_data)
                .unwrap();
        let deserialized =
            <SolanaAccountData as crate::merkle_tree::AccountData>::deserialize(&serialized)
                .unwrap();
        assert_eq!(deserialized.address, account_data.address);
    }
}
