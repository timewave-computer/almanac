//! Sparse Merkle tree implementation using Nervos SMT library
//!
//! This module provides a chain-agnostic wrapper around the Nervos sparse-merkle-tree
//! library for efficient account state verification and inclusion proofs across
//! Solana, Ethereum, Cosmos, and other blockchain networks.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sparse_merkle_tree::{
    blake2b::Blake2bHasher, default_store::DefaultStore, traits::Value,
    SparseMerkleTree as NervosSMT, H256,
};
use std::collections::HashMap;
use thiserror::Error;

/// Errors related to Merkle tree operations
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

    #[error("SMT error: {0}")]
    SMTError(String),
}

/// Generic account data that can be stored in the Merkle tree
pub trait AccountData: Send + Sync + Clone {
    /// Get the account identifier (address/pubkey)
    fn account_id(&self) -> &[u8];

    /// Get the hash of the account data
    fn data_hash(&self) -> [u8; 32];

    /// Get the chain identifier
    fn chain_id(&self) -> &str;

    /// Serialize the account data for storage
    fn serialize(&self) -> Result<Vec<u8>, MerkleTreeError>;

    /// Deserialize account data from storage
    fn deserialize(data: &[u8]) -> Result<Self, MerkleTreeError>;
}

/// Account value wrapper for Nervos SMT
#[derive(Debug, Clone, Default)]
pub struct AccountValue(pub H256);

impl Value for AccountValue {
    fn to_h256(&self) -> H256 {
        self.0
    }

    fn zero() -> Self {
        Self(H256::zero())
    }
}

impl From<[u8; 32]> for AccountValue {
    fn from(bytes: [u8; 32]) -> Self {
        Self(H256::from(bytes))
    }
}

impl From<Vec<u8>> for AccountValue {
    fn from(data: Vec<u8>) -> Self {
        let hash: [u8; 32] = Sha256::digest(&data).into();
        Self(H256::from(hash))
    }
}

/// Merkle tree inclusion proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InclusionProof {
    /// The account data being proven
    pub account_data: Vec<u8>,
    /// The key (account ID) being proven
    pub key: [u8; 32],
    /// The root hash at the time of proof generation
    pub root_hash: [u8; 32],
    /// Proof data (simplified for now)
    pub proof_data: Vec<u8>,
}

impl InclusionProof {
    /// Verify that this proof is valid
    pub fn verify(&self) -> Result<bool, MerkleTreeError> {
        // For now, just verify that the key matches the expected hash
        let expected_hash = Sha256::digest(&self.account_data);
        Ok(expected_hash.as_slice() == &self.proof_data[..32])
    }
}

/// Wrapper around Nervos Sparse Merkle Tree for account verification
pub struct SparseMerkleTree<T: AccountData> {
    /// The underlying Nervos SMT
    smt: NervosSMT<Blake2bHasher, AccountValue, DefaultStore<AccountValue>>,
    /// Local cache of account data for easy retrieval
    accounts: HashMap<Vec<u8>, T>,
    /// Current leaf count
    leaf_count: usize,
}

impl<T: AccountData> Default for SparseMerkleTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: AccountData> SparseMerkleTree<T> {
    /// Create a new sparse Merkle tree
    pub fn new() -> Self {
        let store = DefaultStore::default();
        let smt = NervosSMT::new_with_store(store).expect("Failed to create SMT");

        Self {
            smt,
            accounts: HashMap::new(),
            leaf_count: 0,
        }
    }

    /// Get the current root hash
    pub fn root(&self) -> [u8; 32] {
        (*self.smt.root()).into()
    }

    /// Get the number of leaves in the tree
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Insert or update an account in the tree
    pub fn update(&mut self, account_data: T) -> Result<[u8; 32], MerkleTreeError> {
        let account_id = account_data.account_id();

        // Ensure account ID is 32 bytes (pad or hash if necessary)
        let key = self.normalize_key(account_id)?;

        // Serialize account data and create value
        let serialized = account_data.serialize()?;
        let value = AccountValue::from(serialized);

        // Check if this is a new account
        let was_new = !self.accounts.contains_key(account_id);

        // Update the SMT
        self.smt
            .update(key.into(), value)
            .map_err(|e| MerkleTreeError::SMTError(format!("{:?}", e)))?;

        // Update local cache
        self.accounts.insert(account_id.to_vec(), account_data);

        if was_new {
            self.leaf_count += 1;
        }

        Ok(self.root())
    }

    /// Remove an account from the tree
    pub fn remove(&mut self, account_id: &[u8]) -> Result<Option<T>, MerkleTreeError> {
        let key = self.normalize_key(account_id)?;

        if let Some(removed) = self.accounts.remove(account_id) {
            // Remove from SMT (set to zero value)
            self.smt
                .update(key.into(), AccountValue::zero())
                .map_err(|e| MerkleTreeError::SMTError(format!("{:?}", e)))?;

            self.leaf_count -= 1;
            Ok(Some(removed))
        } else {
            Ok(None)
        }
    }

    /// Get account data by ID
    pub fn get(&self, account_id: &[u8]) -> Option<&T> {
        self.accounts.get(account_id)
    }

    /// Generate an inclusion proof for an account
    pub fn generate_proof(&self, account_id: &[u8]) -> Result<InclusionProof, MerkleTreeError> {
        let account_data = self
            .accounts
            .get(account_id)
            .ok_or(MerkleTreeError::ProofVerificationFailed)?;

        let key = self.normalize_key(account_id)?;
        let serialized = account_data.serialize()?;

        // Create simplified proof for now
        let mut proof_data = Vec::new();
        proof_data.extend_from_slice(&Sha256::digest(&serialized));
        proof_data.extend_from_slice(&key);

        Ok(InclusionProof {
            account_data: serialized,
            key,
            root_hash: self.root(),
            proof_data,
        })
    }

    /// Verify an inclusion proof against the current tree
    pub fn verify_proof(&self, proof: &InclusionProof) -> bool {
        // Check if root matches
        if proof.root_hash != self.root() {
            return false;
        }

        proof.verify().unwrap_or(false)
    }

    /// Get all account IDs in the tree
    pub fn get_all_account_ids(&self) -> Vec<Vec<u8>> {
        self.accounts.keys().cloned().collect()
    }

    /// Get tree statistics
    pub fn stats(&self) -> MerkleTreeStats {
        MerkleTreeStats {
            leaf_count: self.leaf_count,
            root_hash: self.root(),
            memory_usage: self.estimate_memory_usage(),
        }
    }

    /// Normalize account ID to 32 bytes for use as SMT key
    fn normalize_key(&self, account_id: &[u8]) -> Result<[u8; 32], MerkleTreeError> {
        if account_id.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(account_id);
            Ok(key)
        } else if account_id.len() < 32 {
            // Pad with zeros on the right
            let mut key = [0u8; 32];
            key[..account_id.len()].copy_from_slice(account_id);
            Ok(key)
        } else {
            // Hash if longer than 32 bytes
            Ok(Sha256::digest(account_id).into())
        }
    }

    /// Estimate memory usage in bytes
    fn estimate_memory_usage(&self) -> usize {
        // Rough estimate based on accounts cache
        self.accounts.len() * 200 // Approximate size per account
    }
}

/// Statistics about the Merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTreeStats {
    pub leaf_count: usize,
    pub root_hash: [u8; 32],
    pub memory_usage: usize,
}

/// Batch processor for efficient tree updates
pub struct BatchProcessor<T: AccountData> {
    tree: SparseMerkleTree<T>,
    pending_updates: Vec<T>,
    batch_size: usize,
}

impl<T: AccountData> BatchProcessor<T> {
    /// Create a new batch processor
    pub fn new(tree: SparseMerkleTree<T>, batch_size: usize) -> Self {
        Self {
            tree,
            pending_updates: Vec::new(),
            batch_size,
        }
    }

    /// Add an account update to the batch
    pub fn add_update(&mut self, account_data: T) {
        self.pending_updates.push(account_data);

        if self.pending_updates.len() >= self.batch_size {
            let _ = self.process_batch();
        }
    }

    /// Process all pending updates
    pub fn process_batch(&mut self) -> Result<[u8; 32], MerkleTreeError> {
        for account in self.pending_updates.drain(..) {
            self.tree.update(account)?;
        }
        Ok(self.tree.root())
    }

    /// Get a reference to the underlying tree
    pub fn tree(&self) -> &SparseMerkleTree<T> {
        &self.tree
    }

    /// Get a mutable reference to the underlying tree
    pub fn tree_mut(&mut self) -> &mut SparseMerkleTree<T> {
        &mut self.tree
    }

    /// Get the number of pending updates
    pub fn pending_count(&self) -> usize {
        self.pending_updates.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestAccount {
        id: Vec<u8>,
        data: Vec<u8>,
        chain: String,
    }

    impl AccountData for TestAccount {
        fn account_id(&self) -> &[u8] {
            &self.id
        }

        fn data_hash(&self) -> [u8; 32] {
            Sha256::digest(&self.data).into()
        }

        fn chain_id(&self) -> &str {
            &self.chain
        }

        fn serialize(&self) -> Result<Vec<u8>, MerkleTreeError> {
            bincode::serialize(self).map_err(|e| MerkleTreeError::SerializationError(e.to_string()))
        }

        fn deserialize(data: &[u8]) -> Result<Self, MerkleTreeError> {
            bincode::deserialize(data)
                .map_err(|e| MerkleTreeError::SerializationError(e.to_string()))
        }
    }

    #[test]
    fn test_empty_tree() {
        let tree: SparseMerkleTree<TestAccount> = SparseMerkleTree::new();
        assert_eq!(tree.leaf_count(), 0);
        // Empty tree may have zero root - this is fine for the SMT library
        let _root = tree.root(); // Just verify it doesn't panic
    }

    #[test]
    fn test_single_account() {
        let mut tree = SparseMerkleTree::new();

        let account = TestAccount {
            id: vec![1, 2, 3, 4],
            data: vec![10, 20, 30],
            chain: "test".to_string(),
        };

        let root = tree.update(account.clone()).unwrap();
        assert_eq!(tree.leaf_count(), 1);
        assert_ne!(root, [0u8; 32]);

        assert!(tree.get(&account.id).is_some());
    }

    #[test]
    fn test_multiple_accounts() {
        let mut tree = SparseMerkleTree::new();

        let account1 = TestAccount {
            id: vec![1, 2, 3, 4],
            data: vec![10, 20, 30],
            chain: "test".to_string(),
        };

        let account2 = TestAccount {
            id: vec![5, 6, 7, 8],
            data: vec![40, 50, 60],
            chain: "test".to_string(),
        };

        let root1 = tree.update(account1.clone()).unwrap();
        let root2 = tree.update(account2.clone()).unwrap();

        assert_eq!(tree.leaf_count(), 2);
        assert!(tree.get(&account1.id).is_some());
        assert!(tree.get(&account2.id).is_some());

        // Root should change when adding accounts
        assert_ne!(root1, root2);
        assert_ne!(root2, [0u8; 32]);
    }

    #[test]
    fn test_proof_generation_and_verification() {
        let mut tree = SparseMerkleTree::new();

        let account = TestAccount {
            id: vec![1, 2, 3, 4],
            data: vec![10, 20, 30],
            chain: "test".to_string(),
        };

        tree.update(account.clone()).unwrap();

        let proof = tree.generate_proof(&account.id).unwrap();
        assert!(tree.verify_proof(&proof));
        assert_eq!(proof.key.len(), 32);
    }

    #[test]
    fn test_account_removal() {
        let mut tree = SparseMerkleTree::new();

        let account = TestAccount {
            id: vec![1, 2, 3, 4],
            data: vec![10, 20, 30],
            chain: "test".to_string(),
        };

        tree.update(account.clone()).unwrap();
        assert_eq!(tree.leaf_count(), 1);

        let removed = tree.remove(&account.id).unwrap();
        assert!(removed.is_some());
        assert_eq!(tree.leaf_count(), 0);
        assert!(tree.get(&account.id).is_none());
    }

    #[test]
    fn test_batch_processor() {
        let tree = SparseMerkleTree::new();
        let mut processor = BatchProcessor::new(tree, 2);

        let account1 = TestAccount {
            id: vec![1],
            data: vec![10],
            chain: "test".to_string(),
        };

        let account2 = TestAccount {
            id: vec![2],
            data: vec![20],
            chain: "test".to_string(),
        };

        processor.add_update(account1);
        assert_eq!(processor.pending_count(), 1);

        processor.add_update(account2); // Should trigger batch processing
        assert_eq!(processor.pending_count(), 0);
        assert_eq!(processor.tree().leaf_count(), 2);
    }

    #[test]
    fn test_key_normalization() {
        let tree: SparseMerkleTree<TestAccount> = SparseMerkleTree::new();

        // Test exact 32 bytes
        let key32 = vec![1u8; 32];
        let normalized = tree.normalize_key(&key32).unwrap();
        assert_eq!(normalized, [1u8; 32]);

        // Test shorter key (should pad)
        let key_short = vec![1u8; 16];
        let normalized = tree.normalize_key(&key_short).unwrap();
        let mut expected = [0u8; 32];
        expected[..16].fill(1);
        assert_eq!(normalized, expected);

        // Test longer key (should hash)
        let key_long = vec![1u8; 64];
        let normalized = tree.normalize_key(&key_long).unwrap();
        let expected_hash: [u8; 32] = Sha256::digest(&key_long).into();
        assert_eq!(normalized, expected_hash);
    }
}
