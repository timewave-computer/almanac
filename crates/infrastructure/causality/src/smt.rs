/// Sparse Merkle Tree implementation for causality indexing
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use blake3;
use sha2::{Digest, Sha256};

use crate::error::{CausalityError, Result};
use crate::types::{Hash, SmtChildren, SmtProof, empty_hash};
pub use crate::types::SmtHasher;

/// Hash length in bytes
pub const HASH_LEN: usize = 32;

/// Storage backend trait for SMT data
#[async_trait]
pub trait SmtBackend: Send + Sync + Clone {
    /// Get data by prefix and key
    async fn get(&self, prefix: &[u8], key: &Hash) -> Result<Option<Vec<u8>>>;
    
    /// Set data by prefix and key, returning previous value if any
    async fn set(&self, prefix: &[u8], key: &Hash, data: &[u8]) -> Result<Option<Vec<u8>>>;
    
    /// Remove data by prefix and key, returning previous value if any
    async fn remove(&self, prefix: &[u8], key: &Hash) -> Result<Option<Vec<u8>>>;
    
    /// Check if data exists
    async fn has(&self, prefix: &[u8], key: &Hash) -> Result<bool>;
}

/// Storage key type for memory backend
type MemoryStorageKey = (Vec<u8>, Hash);

/// Storage data type for memory backend
type MemoryStorageData = HashMap<MemoryStorageKey, Vec<u8>>;

/// In-memory SMT backend for testing and development
#[derive(Debug, Clone, Default)]
pub struct MemorySmtBackend {
    data: Arc<Mutex<MemoryStorageData>>,
}

impl MemorySmtBackend {
    /// Create a new memory backend
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SmtBackend for MemorySmtBackend {
    async fn get(&self, prefix: &[u8], key: &Hash) -> Result<Option<Vec<u8>>> {
        let storage_key: MemoryStorageKey = (prefix.to_vec(), *key);
        let data = self.data.lock()
            .map_err(|e| CausalityError::storage_error(format!("Failed to lock memory backend: {}", e)))?;
        Ok(data.get(&storage_key).cloned())
    }
    
    async fn set(&self, prefix: &[u8], key: &Hash, data: &[u8]) -> Result<Option<Vec<u8>>> {
        let storage_key: MemoryStorageKey = (prefix.to_vec(), *key);
        let mut storage = self.data.lock()
            .map_err(|e| CausalityError::storage_error(format!("Failed to lock memory backend: {}", e)))?;
        Ok(storage.insert(storage_key, data.to_vec()))
    }
    
    async fn remove(&self, prefix: &[u8], key: &Hash) -> Result<Option<Vec<u8>>> {
        let storage_key: MemoryStorageKey = (prefix.to_vec(), *key);
        let mut data = self.data.lock()
            .map_err(|e| CausalityError::storage_error(format!("Failed to lock memory backend: {}", e)))?;
        Ok(data.remove(&storage_key))
    }
    
    async fn has(&self, prefix: &[u8], key: &Hash) -> Result<bool> {
        let storage_key: MemoryStorageKey = (prefix.to_vec(), *key);
        let data = self.data.lock()
            .map_err(|e| CausalityError::storage_error(format!("Failed to lock memory backend: {}", e)))?;
        Ok(data.contains_key(&storage_key))
    }
}

/// PostgreSQL SMT backend (placeholder for future implementation)
#[derive(Debug, Clone)]
pub struct PostgresSmtBackend {
    // TODO: Add PostgreSQL connection pool
    _placeholder: (),
}

impl Default for PostgresSmtBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresSmtBackend {
    /// Create a new PostgreSQL backend
    pub fn new() -> Self {
        Self {
            _placeholder: (),
        }
    }
}

#[async_trait]
impl SmtBackend for PostgresSmtBackend {
    async fn get(&self, _prefix: &[u8], _key: &Hash) -> Result<Option<Vec<u8>>> {
        // TODO: Implement PostgreSQL storage
        Err(CausalityError::storage_error("PostgreSQL backend not yet implemented"))
    }
    
    async fn set(&self, _prefix: &[u8], _key: &Hash, _data: &[u8]) -> Result<Option<Vec<u8>>> {
        // TODO: Implement PostgreSQL storage
        Err(CausalityError::storage_error("PostgreSQL backend not yet implemented"))
    }
    
    async fn remove(&self, _prefix: &[u8], _key: &Hash) -> Result<Option<Vec<u8>>> {
        // TODO: Implement PostgreSQL storage
        Err(CausalityError::storage_error("PostgreSQL backend not yet implemented"))
    }
    
    async fn has(&self, _prefix: &[u8], _key: &Hash) -> Result<bool> {
        // TODO: Implement PostgreSQL storage
        Err(CausalityError::storage_error("PostgreSQL backend not yet implemented"))
    }
}

/// Blake3 hasher implementation
#[derive(Debug, Clone, Default)]
pub struct Blake3SmtHasher;

impl SmtHasher for Blake3SmtHasher {
    fn hash(&self, data: &[u8]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[0x00]); // Data prefix
        hasher.update(data);
        hasher.finalize().into()
    }
    
    fn merge(&self, left: &Hash, right: &Hash) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[0x01]); // Merge prefix
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }
    
    fn key(&self, context: &str, data: &[u8]) -> Hash {
        blake3::derive_key(context, data)
    }
    
    fn digest(&self, data_list: &[&[u8]]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[0x00]); // Data prefix
        for data in data_list {
            hasher.update(data);
        }
        hasher.finalize().into()
    }
}

/// SHA256 hasher implementation
#[derive(Debug, Clone, Default)]
pub struct Sha256SmtHasher;

impl SmtHasher for Sha256SmtHasher {
    fn hash(&self, data: &[u8]) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update([0x00]); // Data prefix
        hasher.update(data);
        hasher.finalize().into()
    }
    
    fn merge(&self, left: &Hash, right: &Hash) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update([0x01]); // Merge prefix
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }
    
    fn key(&self, context: &str, data: &[u8]) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(context.as_bytes());
        hasher.update(data);
        hasher.finalize().into()
    }
    
    fn digest(&self, data_list: &[&[u8]]) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update([0x00]); // Data prefix
        for data in data_list {
            hasher.update(data);
        }
        hasher.finalize().into()
    }
}

/// Sparse Merkle Tree implementation
pub struct SparseMerkleTree<B: SmtBackend> {
    backend: B,
    hasher: Box<dyn SmtHasher>,
}

impl<B: SmtBackend> SparseMerkleTree<B> {
    /// Prefix for tree nodes
    pub const PREFIX_NODE: &'static [u8] = b"smt-node";
    
    /// Prefix for data storage
    pub const PREFIX_DATA: &'static [u8] = b"smt-data";
    
    /// Prefix for key mapping
    pub const PREFIX_KEY: &'static [u8] = b"smt-key";

    /// Create a new SMT with the given backend and hasher
    pub fn new(backend: B, hasher: Box<dyn SmtHasher>) -> Self {
        Self { backend, hasher }
    }

    /// Create a new SMT with Blake3 hasher
    pub fn with_blake3(backend: B) -> Self {
        Self::new(backend, Box::new(Blake3SmtHasher))
    }

    /// Create a new SMT with SHA256 hasher (default for reverse-causality compatibility)
    pub fn with_sha256(backend: B) -> Self {
        Self::new(backend, Box::new(Sha256SmtHasher))
    }

    /// Create a new SMT with default hasher (SHA256)
    pub fn with_default_hasher(backend: B) -> Self {
        Self::with_sha256(backend)
    }

    /// Get the empty tree root
    pub fn empty_root() -> Hash {
        empty_hash()
    }

    /// Insert data into the tree, returning the new root
    pub async fn insert(&self, root: Hash, key: &Hash, data: &[u8]) -> Result<Hash> {
        let leaf_hash = self.hasher.hash(data);
        
        // Store the data and key mapping
        self.backend.set(Self::PREFIX_DATA, key, data).await?;
        self.backend.set(Self::PREFIX_KEY, &leaf_hash, key.as_ref()).await?;

        // If tree is empty, return the leaf hash
        if root == empty_hash() {
            return Ok(leaf_hash);
        }

        // If root is a leaf, we need to create a new internal node
        if self.is_leaf(&root).await? {
            return self.insert_with_existing_leaf(root, key, &leaf_hash).await;
        }

        // Traverse the tree to find insertion point
        self.insert_recursive(root, key, &leaf_hash, 0).await
    }

    /// Get data from the tree
    pub async fn get(&self, _root: Hash, key: &Hash) -> Result<Option<Vec<u8>>> {
        self.backend.get(Self::PREFIX_DATA, key).await
    }

    /// Generate a proof for the given key
    pub async fn get_proof(&self, root: Hash, key: &Hash) -> Result<Option<SmtProof>> {
        if root == empty_hash() {
            return Ok(None);
        }

        let mut current = root;
        let mut siblings = Vec::new();
        let mut directions = Vec::new();
        let mut depth = 0;

        // Traverse from root to leaf
        while let Some(children) = self.get_children(&current).await? {
            if self.has_node_key(&current).await? {
                break; // Reached a leaf
            }

            if depth >= HASH_LEN * 8 {
                return Err(CausalityError::smt_error("Maximum tree depth exceeded"));
            }

            let bit = self.get_bit(key, depth);
            let (next, sibling) = if bit {
                (children.right, children.left)
            } else {
                (children.left, children.right)
            };

            siblings.push(sibling);
            directions.push(bit);
            current = next;
            depth += 1;
        }

        Ok(Some(SmtProof::new(siblings, directions)))
    }

    /// Verify a proof
    pub fn verify_proof(&self, root: &Hash, key: &Hash, data: &[u8], proof: &SmtProof) -> bool {
        proof.verify(root, key, data, self.hasher.as_ref())
    }

    /// Get the hasher
    pub fn hasher(&self) -> &dyn SmtHasher {
        self.hasher.as_ref()
    }

    /// Check if a node is a leaf
    async fn is_leaf(&self, node: &Hash) -> Result<bool> {
        if *node == empty_hash() {
            return Ok(true);
        }
        self.has_node_key(node).await
    }

    /// Get children of a node
    async fn get_children(&self, node: &Hash) -> Result<Option<SmtChildren>> {
        if let Some(data) = self.backend.get(Self::PREFIX_NODE, node).await? {
            if data.len() != 64 {
                return Err(CausalityError::smt_error("Invalid children data length"));
            }
            
            let mut left = [0u8; 32];
            let mut right = [0u8; 32];
            left.copy_from_slice(&data[0..32]);
            right.copy_from_slice(&data[32..64]);
            
            Ok(Some(SmtChildren { left, right }))
        } else {
            Ok(None)
        }
    }

    /// Set children for a node
    async fn set_children(&self, node: &Hash, children: &SmtChildren) -> Result<()> {
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(&children.left);
        data.extend_from_slice(&children.right);
        self.backend.set(Self::PREFIX_NODE, node, &data).await?;
        Ok(())
    }

    /// Check if node has a key mapping
    async fn has_node_key(&self, node: &Hash) -> Result<bool> {
        self.backend.has(Self::PREFIX_KEY, node).await
    }

    /// Get bit at position from key
    fn get_bit(&self, key: &Hash, position: usize) -> bool {
        let byte_index = position / 8;
        let bit_index = position % 8;
        if byte_index >= HASH_LEN {
            return false;
        }
        (key[byte_index] >> (7 - bit_index)) & 1 == 1
    }

    /// Insert with an existing leaf at root
    async fn insert_with_existing_leaf(&self, existing_leaf: Hash, new_key: &Hash, new_leaf: &Hash) -> Result<Hash> {
        // Get the key of the existing leaf
        let existing_key_data = self.backend.get(Self::PREFIX_KEY, &existing_leaf).await?
            .ok_or_else(|| CausalityError::smt_error("Existing leaf has no key mapping"))?;
        
        if existing_key_data.len() != 32 {
            return Err(CausalityError::smt_error("Invalid existing key length"));
        }
        
        let mut existing_key = [0u8; 32];
        existing_key.copy_from_slice(&existing_key_data);

        // If keys are the same, replace the value
        if existing_key == *new_key {
            return Ok(*new_leaf);
        }

        // Find the first differing bit
        let mut depth = 0;
        while depth < HASH_LEN * 8 {
            let existing_bit = self.get_bit(&existing_key, depth);
            let new_bit = self.get_bit(new_key, depth);
            
            if existing_bit != new_bit {
                break;
            }
            depth += 1;
        }

        // Create internal nodes from root down to the differing bit
        let new_bit = self.get_bit(new_key, depth);
        let children = if new_bit {
            SmtChildren { left: existing_leaf, right: *new_leaf }
        } else {
            SmtChildren { left: *new_leaf, right: existing_leaf }
        };

        let mut current_node = self.hasher.merge(&children.left, &children.right);
        self.set_children(&current_node, &children).await?;

        // Create parent nodes up to the root
        while depth > 0 {
            depth -= 1;
            let bit = self.get_bit(new_key, depth);
            let sibling = empty_hash();
            
            let parent_children = if bit {
                SmtChildren { left: sibling, right: current_node }
            } else {
                SmtChildren { left: current_node, right: sibling }
            };
            
            current_node = self.hasher.merge(&parent_children.left, &parent_children.right);
            self.set_children(&current_node, &parent_children).await?;
        }

        Ok(current_node)
    }

    /// Recursive insertion helper
    fn insert_recursive<'a>(&'a self, node: Hash, key: &'a Hash, leaf: &'a Hash, depth: usize) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Hash>> + Send + 'a>> {
        Box::pin(async move {
            if depth >= HASH_LEN * 8 {
                return Err(CausalityError::smt_error("Maximum tree depth exceeded"));
            }

            let children = self.get_children(&node).await?
                .ok_or_else(|| CausalityError::smt_error("Node has no children"))?;

            let bit = self.get_bit(key, depth);
            let (target, sibling) = if bit {
                (children.right, children.left)
            } else {
                (children.left, children.right)
            };

            let new_target = if target == empty_hash() {
                *leaf
            } else if self.is_leaf(&target).await? {
                self.insert_with_existing_leaf(target, key, leaf).await?
            } else {
                self.insert_recursive(target, key, leaf, depth + 1).await?
            };

            let new_children = if bit {
                SmtChildren { left: sibling, right: new_target }
            } else {
                SmtChildren { left: new_target, right: sibling }
            };

            let new_node = self.hasher.merge(&new_children.left, &new_children.right);
            self.set_children(&new_node, &new_children).await?;
            
            Ok(new_node)
        })
    }
}

impl<B: SmtBackend> Clone for SparseMerkleTree<B> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            hasher: Box::new(Sha256SmtHasher), // Default to SHA256 for cloning
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_tree() {
        let backend = MemorySmtBackend::new();
        let _smt = SparseMerkleTree::with_sha256(backend);
        
        let root = SparseMerkleTree::<MemorySmtBackend>::empty_root();
        assert_eq!(root, empty_hash());
    }

    #[tokio::test]
    async fn test_single_insertion() {
        let backend = MemorySmtBackend::new();
        let smt = SparseMerkleTree::with_sha256(backend);
        
        let key = Sha256SmtHasher.key("test", b"key1");
        let data = b"value1";
        
        let root = smt.insert(empty_hash(), &key, data).await.unwrap();
        assert_ne!(root, empty_hash());
        
        let retrieved = smt.get(root, &key).await.unwrap();
        assert_eq!(retrieved, Some(data.to_vec()));
    }

    #[tokio::test]
    async fn test_proof_generation_and_verification() {
        let backend = MemorySmtBackend::new();
        let smt = SparseMerkleTree::with_sha256(backend);
        
        let key = Sha256SmtHasher.key("test", b"key1");
        let data = b"value1";
        
        let root = smt.insert(empty_hash(), &key, data).await.unwrap();
        let proof = smt.get_proof(root, &key).await.unwrap();
        
        assert!(proof.is_some());
        let proof = proof.unwrap();
        assert!(smt.verify_proof(&root, &key, data, &proof));
    }
} 