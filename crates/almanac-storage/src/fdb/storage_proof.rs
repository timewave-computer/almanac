//! Storage proof types for FDB storage

use serde::{Serialize, Deserialize};

/// Storage proof key
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageProofKey {
    pub chain_id: String,
    pub contract_address: String,
    pub storage_key: [u8; 32],
    pub block_height: u64,
}

/// Storage proof value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageProofValue {
    pub value: [u8; 32],
    pub proof: Vec<Vec<u8>>,
    pub state_root: [u8; 32],
    pub block_hash: [u8; 32],
    pub verified: bool,
    pub verification_timestamp: u64,
    pub proof_type: ProofType,
}

/// Type of storage proof
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofType {
    MerklePatricia,  // Ethereum
    Iavl,            // Cosmos
    Smt,             // Sparse Merkle Tree
}