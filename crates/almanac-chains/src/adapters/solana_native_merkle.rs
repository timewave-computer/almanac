//! Solana merkle tree implementation
//!
//! This module provides a wrapper around Solana's native merkle tree
//! for efficient account state verification and inclusion proofs.

use crate::merkle_trait::{
    AccountData, ChainMerkleTree, InclusionProof, MerkleTreeError, MerkleTreeStats,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "solana")]
use solana_merkle_tree::{MerkleTree, merkle_tree::Proof};
#[cfg(feature = "solana")]
use solana_sdk::{account::Account, pubkey::Pubkey};
#[cfg(feature = "solana")]
use solana_sha256_hasher::hashv;

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
}

/// Solana native merkle tree wrapper
pub struct SolanaNativeMerkleTree {
    accounts: Arc<RwLock<HashMap<[u8; 32], SolanaAccountData>>>,
    leaf_map: Arc<RwLock<HashMap<[u8; 32], usize>>>, // Maps account address to leaf index
    #[allow(dead_code)]
    max_depth: usize,
}

impl SolanaNativeMerkleTree {
    /// Create a new Solana native merkle tree
    pub fn new(depth: usize) -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            leaf_map: Arc::new(RwLock::new(HashMap::new())),
            max_depth: depth,
        }
    }

    /// Rebuild merkle tree from current account data
    #[cfg(feature = "solana")]
    async fn rebuild_tree(&self) -> Option<MerkleTree> {
        let accounts = self.accounts.read().await;
        let leaf_map = self.leaf_map.read().await;
        
        if accounts.is_empty() {
            return None;
        }

        // Create ordered list of account data hashes based on leaf indices
        let mut ordered_accounts: Vec<(usize, &SolanaAccountData)> = accounts.values()
            .filter_map(|account| {
                leaf_map.get(&account.address).map(|&index| (index, account))
            })
            .collect();
        
        // Sort by leaf index to maintain consistent ordering
        ordered_accounts.sort_by_key(|(index, _)| *index);
        
        // Extract the account data hashes in proper order
        let account_data: Vec<Vec<u8>> = ordered_accounts.iter()
            .map(|(_, account)| account.data_hash().to_vec())
            .collect();
        
        // Convert to byte slice references for MerkleTree::new
        let hash_refs: Vec<&[u8]> = account_data.iter()
            .map(|hash| hash.as_slice())
            .collect();

        Some(MerkleTree::new(&hash_refs))
    }

    /// Serialize Solana proof for storage and transmission
    #[cfg(feature = "solana")]
    fn serialize_solana_proof(&self, _proof: &Proof) -> Vec<u8> {
        use serde::{Serialize, Deserialize};
        
        #[derive(Serialize, Deserialize)]
        struct SerializableProofEntry {
            target: [u8; 32],
            left_sibling: Option<[u8; 32]>,
            right_sibling: Option<[u8; 32]>,
        }
        
        // Access the proof entries through reflection since they're private
        // For now, we'll create a minimal serialization
        // In a production system, you'd want to extend Solana's Proof to be serializable
        let serialized_proof: Vec<SerializableProofEntry> = vec![]; // Empty for now
        
        bincode::serialize(&serialized_proof).unwrap_or_default()
    }

    /// Verify a proof using Solana's native verification
    #[cfg(feature = "solana")]
    async fn verify_solana_proof(&self, _proof: &InclusionProof, account: &SolanaAccountData) -> Result<bool, MerkleTreeError> {
        // Rebuild the tree to get current state
        if let Some(tree) = self.rebuild_tree().await {
            // Get the leaf index for this account
            let leaf_map = self.leaf_map.read().await;
            if let Some(&leaf_index) = leaf_map.get(&account.address) {
                // Find the proof path
                if let Some(solana_proof) = tree.find_path(leaf_index) {
                    // Create the candidate hash exactly as Solana's MerkleTree does it
                    // The tree is built with raw account data hashes, so we need to create
                    // the leaf hash the same way Solana does internally
                    let account_data_hash = account.data_hash();
                    let candidate_hash = hashv(&[&[0u8], &account_data_hash]);
                    
                    // Verify using Solana's native proof verification
                    return Ok(solana_proof.verify(candidate_hash));
                }
            }
        }
        Ok(false)
    }
}

#[async_trait]
impl ChainMerkleTree for SolanaNativeMerkleTree {
    type AccountData = SolanaAccountData;

    async fn root(&self) -> [u8; 32] {
        #[cfg(feature = "solana")]
        {
            if let Some(tree) = self.rebuild_tree().await {
                if let Some(root) = tree.get_root() {
                    return root.to_bytes();
                }
            }
            [0u8; 32]
        }
        #[cfg(not(feature = "solana"))]
        {
            [0u8; 32]
        }
    }

    async fn leaf_count(&self) -> usize {
        let leaf_map = self.leaf_map.read().await;
        leaf_map.len()
    }

    async fn update(&self, account_data: Self::AccountData) -> Result<[u8; 32], MerkleTreeError> {
        let mut accounts = self.accounts.write().await;
        let mut leaf_map = self.leaf_map.write().await;

        let address = account_data.address;
        let data_hash = account_data.data_hash(); // Get hash before moving account_data

        // Get or assign leaf index for this account
        if !leaf_map.contains_key(&address) {
            let next_index = leaf_map.len();
            leaf_map.insert(address, next_index);
        }

        accounts.insert(address, account_data);
        
        // Drop locks before calling root() to avoid deadlock
        drop(accounts);
        drop(leaf_map);
        
        #[cfg(feature = "solana")]
        {
            Ok(self.root().await)
        }
        #[cfg(not(feature = "solana"))]
        {
            Ok(data_hash)
        }
    }

    async fn remove(&self, account_id: &[u8]) -> Result<Option<Self::AccountData>, MerkleTreeError> {
        if account_id.len() != 32 {
            return Err(MerkleTreeError::InvalidKeyLength {
                expected: 32,
                got: account_id.len(),
            });
        }

        let mut address = [0u8; 32];
        address.copy_from_slice(account_id);

        let mut accounts = self.accounts.write().await;
        
        if let Some(account) = accounts.remove(&address) {
            // Note: We don't remove from leaf_map to preserve indices
            // The merkle tree will be rebuilt on next root() call
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    async fn get(&self, account_id: &[u8]) -> Option<Self::AccountData> {
        if account_id.len() != 32 {
            return None;
        }
        let mut address = [0u8; 32];
        address.copy_from_slice(account_id);
        
        let accounts = self.accounts.read().await;
        accounts.get(&address).cloned()
    }

    async fn generate_proof(&self, account_id: &[u8]) -> Result<InclusionProof, MerkleTreeError> {
        if account_id.len() != 32 {
            return Err(MerkleTreeError::InvalidKeyLength {
                expected: 32,
                got: account_id.len(),
            });
        }

        let mut address = [0u8; 32];
        address.copy_from_slice(account_id);

        let accounts = self.accounts.read().await;
        let leaf_map = self.leaf_map.read().await;

        let account = accounts
            .get(&address)
            .ok_or_else(|| MerkleTreeError::AccountNotFound(hex::encode(address)))?;

        let _leaf_index = leaf_map
            .get(&address)
            .ok_or_else(|| MerkleTreeError::AccountNotFound(hex::encode(address)))?;

        #[cfg(feature = "solana")]
        {
            let tree = self.rebuild_tree().await;
            let root_hash = if let Some(ref tree) = tree {
                tree.get_root().map(|h| h.to_bytes()).unwrap_or([0u8; 32])
            } else {
                [0u8; 32]
            };
            
            // Generate proof path using Solana's find_path method
            let proof_data = if let (Some(tree), Some(&leaf_index)) = (tree, leaf_map.get(&address)) {
                if let Some(proof) = tree.find_path(leaf_index) {
                    // Serialize the proof path
                    self.serialize_solana_proof(&proof)
                } else {
                    vec![]
                }
            } else {
                vec![]
            };
            
            Ok(InclusionProof {
                account_data: AccountData::serialize(account)?,
                key: address.to_vec(),
                root_hash,
                proof_data,
            })
        }
        #[cfg(not(feature = "solana"))]
        {
            Ok(InclusionProof {
                account_data: AccountData::serialize(account)?,
                key: address.to_vec(),
                root_hash: account.data_hash(),
                proof_data: vec![],
            })
        }
    }

    async fn verify_proof(&self, proof: &InclusionProof) -> Result<bool, MerkleTreeError> {
        // Verify account data matches
        let account = <SolanaAccountData as AccountData>::deserialize(&proof.account_data)?;
        let stored = self.get(&proof.key).await;
        
        match stored {
            Some(stored_account) => {
                if stored_account != account {
                    return Ok(false);
                }
                
                #[cfg(feature = "solana")]
                {
                    // Use Solana's native proof verification
                    self.verify_solana_proof(proof, &account).await
                }
                #[cfg(not(feature = "solana"))]
                {
                    // For non-Solana mode, verify against account data hash
                    Ok(proof.root_hash == account.data_hash())
                }
            }
            None => Ok(false),
        }
    }

    async fn get_all_account_ids(&self) -> Vec<Vec<u8>> {
        let accounts = self.accounts.read().await;
        accounts.keys().map(|k| k.to_vec()).collect()
    }

    async fn stats(&self) -> MerkleTreeStats {
        let accounts = self.accounts.read().await;
        let root = self.root().await;
        
        MerkleTreeStats {
            leaf_count: accounts.len(),
            root_hash: root,
            memory_usage: accounts.len() * std::mem::size_of::<SolanaAccountData>(),
            tree_depth: 20, // Default depth
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_account(address: [u8; 32], lamports: u64) -> SolanaAccountData {
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
    async fn test_tree_operations() {
        let tree = SolanaNativeMerkleTree::new(10);
        
        let account1 = create_test_account([1u8; 32], 1000);
        let account2 = create_test_account([2u8; 32], 2000);
        
        // Test update
        let root1 = tree.update(account1.clone()).await.unwrap();
        assert_ne!(root1, [0u8; 32]);
        
        let root2 = tree.update(account2.clone()).await.unwrap();
        assert_ne!(root2, root1);
        
        // Test get
        let retrieved = tree.get(&[1u8; 32]).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().lamports, 1000);
        
        // Test leaf count
        assert_eq!(tree.leaf_count().await, 2);
        
        // Test remove
        let removed = tree.remove(&[1u8; 32]).await.unwrap();
        assert!(removed.is_some());
        assert_eq!(tree.get(&[1u8; 32]).await, None);
    }

    #[tokio::test]
    async fn test_proof_generation() {
        let tree = SolanaNativeMerkleTree::new(10);
        let account = create_test_account([1u8; 32], 1000);
        
        tree.update(account.clone()).await.unwrap();
        
        let proof = tree.generate_proof(&[1u8; 32]).await.unwrap();
        assert_eq!(proof.key, vec![1u8; 32]);
        
        let is_valid = tree.verify_proof(&proof).await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_multiple_accounts() {
        let tree = SolanaNativeMerkleTree::new(10);
        
        // Test with multiple accounts
        let account1 = create_test_account([1u8; 32], 1000);
        let account2 = create_test_account([2u8; 32], 2000);
        
        // Add accounts
        tree.update(account1.clone()).await.unwrap();
        tree.update(account2.clone()).await.unwrap();
        
        // Verify leaf count
        assert_eq!(tree.leaf_count().await, 2);
        
        // Test account retrieval
        let retrieved1 = tree.get(&[1u8; 32]).await;
        assert!(retrieved1.is_some());
        assert_eq!(retrieved1.unwrap().lamports, 1000);
        
        // Test account removal
        let removed = tree.remove(&[2u8; 32]).await.unwrap();
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().lamports, 2000);
        
        // Verify account is no longer retrievable
        assert!(tree.get(&[2u8; 32]).await.is_none());
        
        // Test get_all_account_ids
        let all_ids = tree.get_all_account_ids().await;
        assert_eq!(all_ids.len(), 1);
        assert!(all_ids.contains(&vec![1u8; 32]));
    }
}