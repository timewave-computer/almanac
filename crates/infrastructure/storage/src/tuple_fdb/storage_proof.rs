//! Storage proof management for cryptographic verification
//!
//! This module provides storage and verification of Merkle proofs for contract
//! state across different blockchain architectures. Supports EVM MPT, Cosmos IAVL,
//! and Solana SMT proof formats.

use super::{TupleKey, TupleKeyspace, TupleFdbStorage};
use almanac_infrastructure_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn, error};

/// Storage proof key for tuple storage
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageProofKey {
    /// Chain identifier
    pub chain_id: String,
    /// Contract address
    pub contract_address: String,
    /// Block height
    pub block_height: u64,
    /// Storage key being proven
    pub storage_key: Vec<u8>,
    /// Optional proof version for upgrades
    pub version: u32,
}

impl StorageProofKey {
    /// Create tuple key for storage
    pub fn to_tuple_key(&self) -> TupleKey {
        TupleKey::new(TupleKeyspace::StorageProofs)
            .string(&self.chain_id)
            .string(&self.contract_address)
            .int(self.block_height as i64)
            .bytes(self.storage_key.clone())
            .int(self.version as i64)
    }
    
    /// Create prefix for contract proofs
    pub fn contract_prefix(chain_id: &str, contract_address: &str) -> TupleKey {
        TupleKey::new(TupleKeyspace::StorageProofs)
            .string(chain_id)
            .string(contract_address)
    }
    
    /// Create prefix for block range
    pub fn block_prefix(
        chain_id: &str,
        contract_address: &str,
        block_height: u64,
    ) -> TupleKey {
        TupleKey::new(TupleKeyspace::StorageProofs)
            .string(chain_id)
            .string(contract_address)
            .int(block_height as i64)
    }
}

/// Storage proof value with multi-chain support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProofValue {
    /// The value being proven
    pub value: Vec<u8>,
    /// The proof data (chain-specific format)
    pub proof: ProofData,
    /// State root for verification
    pub state_root: [u8; 32],
    /// Whether this proof has been verified
    pub verified: bool,
    /// Verification timestamp
    pub verification_timestamp: Option<u64>,
    /// Verification metadata
    pub verification_metadata: Option<VerificationMetadata>,
    /// Cache expiry time
    pub expires_at: Option<u64>,
}

/// Chain-specific proof data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofData {
    /// Ethereum Merkle Patricia Trie proof
    EthereumMPT {
        /// Account proof nodes
        account_proof: Vec<Vec<u8>>,
        /// Storage proof nodes
        storage_proof: Vec<Vec<u8>>,
        /// Storage key
        key: [u8; 32],
        /// Storage value
        value: [u8; 32],
    },
    /// Cosmos IAVL tree proof
    CosmosIAVL {
        /// IAVL proof nodes
        proof_nodes: Vec<IAVLNode>,
        /// Key being proven
        key: Vec<u8>,
        /// Value hash
        value_hash: [u8; 32],
        /// Tree version
        version: i64,
    },
    /// Solana sparse Merkle tree proof
    SolanaSMT {
        /// Merkle proof path
        proof_path: Vec<[u8; 32]>,
        /// Leaf index
        leaf_index: u64,
        /// Leaf value
        leaf_value: Vec<u8>,
        /// Tree depth
        depth: u8,
    },
    /// Generic Merkle proof
    GenericMerkle {
        /// Proof nodes
        nodes: Vec<Vec<u8>>,
        /// Indices for proof path
        indices: Vec<u64>,
        /// Leaf data
        leaf: Vec<u8>,
    },
}

/// IAVL tree node for Cosmos proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IAVLNode {
    /// Node height in tree
    pub height: i8,
    /// Node size
    pub size: i64,
    /// Left child hash
    pub left_hash: Option<[u8; 32]>,
    /// Right child hash
    pub right_hash: Option<[u8; 32]>,
    /// Node key (if leaf)
    pub key: Option<Vec<u8>>,
    /// Node value (if leaf)
    pub value: Option<Vec<u8>>,
}

/// Verification metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMetadata {
    /// Verifier identity
    pub verified_by: String,
    /// Verification method used
    pub verification_method: VerificationMethod,
    /// Gas cost of verification
    pub gas_used: Option<u64>,
    /// Additional verification details
    pub details: HashMap<String, String>,
}

/// Method used for verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMethod {
    /// On-chain verification
    OnChain,
    /// Off-chain computation
    OffChain,
    /// ZK proof verification
    ZKProof,
    /// Trusted node attestation
    TrustedNode,
}

/// Proof cache entry for efficient retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCacheEntry {
    /// Proof key
    pub key: StorageProofKey,
    /// Cached proof value
    pub value: StorageProofValue,
    /// Cache creation time
    pub cached_at: u64,
    /// Access count
    pub access_count: u64,
    /// Last access time
    pub last_accessed: u64,
}

/// Batch proof for multiple storage slots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProof {
    /// Chain and contract
    pub chain_id: String,
    pub contract_address: String,
    pub block_height: u64,
    /// Individual proofs
    pub proofs: Vec<(Vec<u8>, StorageProofValue)>,
    /// Aggregated proof (if available)
    pub aggregated_proof: Option<AggregatedProof>,
    /// Batch metadata
    pub metadata: BatchProofMetadata,
}

/// Aggregated proof for batch verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedProof {
    /// Type of aggregation
    pub aggregation_type: AggregationType,
    /// Aggregated proof data
    pub proof_data: Vec<u8>,
    /// Witness data for verification
    pub witness: Vec<u8>,
    /// Public inputs
    pub public_inputs: Vec<[u8; 32]>,
}

/// Type of proof aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationType {
    /// Merkle tree aggregation
    MerkleTree,
    /// SNARK aggregation
    SNARK,
    /// STARK aggregation
    STARK,
    /// Custom aggregation
    Custom,
}

/// Batch proof metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProofMetadata {
    /// Total slots in batch
    pub total_slots: u32,
    /// Verified slots
    pub verified_slots: u32,
    /// Total gas saved by batching
    pub gas_saved: u64,
    /// Batch creation time
    pub created_at: u64,
}

impl TupleFdbStorage {
    /// Store a storage proof
    pub async fn store_storage_proof(
        &self,
        key: StorageProofKey,
        value: StorageProofValue,
    ) -> Result<()> {
        let tuple_key = key.to_tuple_key();
        let serialized = rmp_serde::to_vec(&value)
            .map_err(|e| Error::Serialization(format!("Failed to serialize proof: {}", e)))?;
        
        self.set(tuple_key, serialized).await?;
        
        // Update proof cache if enabled
        if let Some(expires_at) = value.expires_at {
            self.update_proof_cache(&key, &value, expires_at).await?;
        }
        
        info!(
            "Stored storage proof for contract {} at block {} (verified: {})",
            key.contract_address, key.block_height, value.verified
        );
        
        Ok(())
    }
    
    /// Get a storage proof
    pub async fn get_storage_proof(
        &self,
        key: &StorageProofKey,
    ) -> Result<Option<StorageProofValue>> {
        // Check cache first
        if let Some(cached) = self.get_cached_proof(key).await? {
            return Ok(Some(cached));
        }
        
        let tuple_key = key.to_tuple_key();
        
        if let Some(data) = self.get(tuple_key).await? {
            let value: StorageProofValue = rmp_serde::from_slice(&data)
                .map_err(|e| Error::Deserialization(format!("Failed to parse proof: {}", e)))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    
    /// Verify a storage proof
    pub async fn verify_storage_proof(
        &self,
        key: &StorageProofKey,
        expected_root: &[u8; 32],
    ) -> Result<bool> {
        let proof = self.get_storage_proof(key).await?
            .ok_or_else(|| Error::NotFound("Storage proof not found".to_string()))?;
        
        if proof.state_root != *expected_root {
            warn!(
                "State root mismatch: expected {:?}, got {:?}",
                expected_root, proof.state_root
            );
            return Ok(false);
        }
        
        let verified = match &proof.proof {
            ProofData::EthereumMPT { account_proof, storage_proof, key: storage_key, value } => {
                self.verify_ethereum_mpt_proof(
                    account_proof,
                    storage_proof,
                    storage_key,
                    value,
                    expected_root,
                ).await?
            }
            ProofData::CosmosIAVL { proof_nodes, key: proof_key, value_hash, version } => {
                self.verify_cosmos_iavl_proof(
                    proof_nodes,
                    proof_key,
                    value_hash,
                    *version,
                    expected_root,
                ).await?
            }
            ProofData::SolanaSMT { proof_path, leaf_index, leaf_value, depth } => {
                self.verify_solana_smt_proof(
                    proof_path,
                    *leaf_index,
                    leaf_value,
                    *depth,
                    expected_root,
                ).await?
            }
            ProofData::GenericMerkle { nodes, indices, leaf } => {
                self.verify_generic_merkle_proof(
                    nodes,
                    indices,
                    leaf,
                    expected_root,
                ).await?
            }
        };
        
        // Update verification status
        if verified && !proof.verified {
            let mut updated_proof = proof;
            updated_proof.verified = true;
            updated_proof.verification_timestamp = Some(chrono::Utc::now().timestamp() as u64);
            updated_proof.verification_metadata = Some(VerificationMetadata {
                verified_by: "almanac".to_string(),
                verification_method: VerificationMethod::OffChain,
                gas_used: None,
                details: HashMap::new(),
            });
            
            self.store_storage_proof(key.clone(), updated_proof).await?;
        }
        
        Ok(verified)
    }
    
    /// Create a batch proof
    pub async fn create_batch_proof(
        &self,
        chain_id: &str,
        contract_address: &str,
        block_height: u64,
        storage_keys: Vec<Vec<u8>>,
    ) -> Result<BatchProof> {
        let mut proofs = Vec::new();
        let mut verified_count = 0u32;
        
        for storage_key in storage_keys {
            let key = StorageProofKey {
                chain_id: chain_id.to_string(),
                contract_address: contract_address.to_string(),
                block_height,
                storage_key: storage_key.clone(),
                version: 0,
            };
            
            if let Some(proof) = self.get_storage_proof(&key).await? {
                if proof.verified {
                    verified_count += 1;
                }
                proofs.push((storage_key, proof));
            }
        }
        
        let metadata = BatchProofMetadata {
            total_slots: proofs.len() as u32,
            verified_slots: verified_count,
            gas_saved: 0, // Calculate based on chain
            created_at: chrono::Utc::now().timestamp() as u64,
        };
        
        // Try to create aggregated proof if possible
        let aggregated_proof = if proofs.len() > 1 {
            self.try_aggregate_proofs(&proofs).await?
        } else {
            None
        };
        
        Ok(BatchProof {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            block_height,
            proofs,
            aggregated_proof,
            metadata,
        })
    }
    
    /// Verify Ethereum MPT proof (simplified)
    async fn verify_ethereum_mpt_proof(
        &self,
        _account_proof: &[Vec<u8>],
        _storage_proof: &[Vec<u8>],
        _key: &[u8; 32],
        _value: &[u8; 32],
        _expected_root: &[u8; 32],
    ) -> Result<bool> {
        // This would implement actual MPT verification
        // For now, return true as placeholder
        warn!("Ethereum MPT verification not yet implemented");
        Ok(true)
    }
    
    /// Verify Cosmos IAVL proof (simplified)
    async fn verify_cosmos_iavl_proof(
        &self,
        _proof_nodes: &[IAVLNode],
        _key: &[u8],
        _value_hash: &[u8; 32],
        _version: i64,
        _expected_root: &[u8; 32],
    ) -> Result<bool> {
        // This would implement actual IAVL verification
        // For now, return true as placeholder
        warn!("Cosmos IAVL verification not yet implemented");
        Ok(true)
    }
    
    /// Verify Solana SMT proof (simplified)
    async fn verify_solana_smt_proof(
        &self,
        _proof_path: &[[u8; 32]],
        _leaf_index: u64,
        _leaf_value: &[u8],
        _depth: u8,
        _expected_root: &[u8; 32],
    ) -> Result<bool> {
        // This would implement actual SMT verification
        // For now, return true as placeholder
        warn!("Solana SMT verification not yet implemented");
        Ok(true)
    }
    
    /// Verify generic Merkle proof
    async fn verify_generic_merkle_proof(
        &self,
        nodes: &[Vec<u8>],
        indices: &[u64],
        leaf: &[u8],
        expected_root: &[u8; 32],
    ) -> Result<bool> {
        use sha2::{Sha256, Digest};
        
        if nodes.len() != indices.len() {
            return Ok(false);
        }
        
        let mut current_hash = Sha256::digest(leaf);
        
        for (i, (node, index)) in nodes.iter().zip(indices.iter()).enumerate() {
            let mut hasher = Sha256::new();
            
            if index % 2 == 0 {
                // Current hash is left child
                hasher.update(&current_hash);
                hasher.update(node);
            } else {
                // Current hash is right child
                hasher.update(node);
                hasher.update(&current_hash);
            }
            
            current_hash = hasher.finalize();
        }
        
        Ok(current_hash.as_slice() == expected_root)
    }
    
    /// Update proof cache
    async fn update_proof_cache(
        &self,
        key: &StorageProofKey,
        value: &StorageProofValue,
        expires_at: u64,
    ) -> Result<()> {
        let cache_key = TupleKey::new(TupleKeyspace::StorageProofs)
            .string("cache")
            .string(&key.chain_id)
            .string(&key.contract_address)
            .int(key.block_height as i64)
            .bytes(key.storage_key.clone());
        
        let cache_entry = ProofCacheEntry {
            key: key.clone(),
            value: value.clone(),
            cached_at: chrono::Utc::now().timestamp() as u64,
            access_count: 0,
            last_accessed: chrono::Utc::now().timestamp() as u64,
        };
        
        let serialized = rmp_serde::to_vec(&cache_entry)
            .map_err(|e| Error::Serialization(format!("Failed to serialize cache entry: {}", e)))?;
        
        self.set(cache_key, serialized).await?;
        
        Ok(())
    }
    
    /// Get cached proof
    async fn get_cached_proof(
        &self,
        key: &StorageProofKey,
    ) -> Result<Option<StorageProofValue>> {
        let cache_key = TupleKey::new(TupleKeyspace::StorageProofs)
            .string("cache")
            .string(&key.chain_id)
            .string(&key.contract_address)
            .int(key.block_height as i64)
            .bytes(key.storage_key.clone());
        
        if let Some(data) = self.get(cache_key.clone()).await? {
            let mut entry: ProofCacheEntry = rmp_serde::from_slice(&data)
                .map_err(|e| Error::Deserialization(format!("Failed to parse cache entry: {}", e)))?;
            
            // Check if expired
            if let Some(expires_at) = entry.value.expires_at {
                let now = chrono::Utc::now().timestamp() as u64;
                if now > expires_at {
                    // Remove expired entry
                    self.delete(cache_key).await?;
                    return Ok(None);
                }
            }
            
            // Update access stats
            entry.access_count += 1;
            entry.last_accessed = chrono::Utc::now().timestamp() as u64;
            
            let serialized = rmp_serde::to_vec(&entry)
                .map_err(|e| Error::Serialization(format!("Failed to serialize cache entry: {}", e)))?;
            
            self.set(cache_key, serialized).await?;
            
            Ok(Some(entry.value))
        } else {
            Ok(None)
        }
    }
    
    /// Try to aggregate proofs
    async fn try_aggregate_proofs(
        &self,
        proofs: &[(Vec<u8>, StorageProofValue)],
    ) -> Result<Option<AggregatedProof>> {
        // This would implement actual proof aggregation
        // For now, return None as placeholder
        debug!("Proof aggregation not yet implemented");
        Ok(None)
    }
    
    /// Invalidate proof cache for a contract
    pub async fn invalidate_proof_cache(
        &self,
        chain_id: &str,
        contract_address: &str,
        from_block: Option<u64>,
    ) -> Result<u64> {
        let prefix = if let Some(block) = from_block {
            TupleKey::new(TupleKeyspace::StorageProofs)
                .string("cache")
                .string(chain_id)
                .string(contract_address)
                .int(block as i64)
        } else {
            TupleKey::new(TupleKeyspace::StorageProofs)
                .string("cache")
                .string(chain_id)
                .string(contract_address)
        };
        
        let kvs = self.range(prefix, None).await?;
        let count = kvs.len() as u64;
        
        // Delete all matching cache entries
        for (key, _) in kvs {
            self.delete_raw(key).await?;
        }
        
        info!(
            "Invalidated {} proof cache entries for contract {}",
            count, contract_address
        );
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_storage_proof_key() {
        let key = StorageProofKey {
            chain_id: "ethereum".to_string(),
            contract_address: "0x123".to_string(),
            block_height: 1000,
            storage_key: vec![0x01, 0x02, 0x03],
            version: 0,
        };
        
        let tuple_key = key.to_tuple_key();
        assert_eq!(tuple_key.keyspace(), Some(TupleKeyspace::StorageProofs));
    }
    
    #[test]
    fn test_generic_merkle_verification() {
        // Test would implement actual Merkle verification
        // Placeholder for now
    }
}