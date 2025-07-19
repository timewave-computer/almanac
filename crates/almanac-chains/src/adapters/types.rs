//! Common types for chain adapters

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Block finality status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinalityStatus {
    /// Block is pending (not yet confirmed)
    Pending,
    /// Block has basic confirmation but may still be reorganized
    Confirmed,
    /// Block is safe from short-term reorganizations
    Safe,
    /// Block is justified (intermediate finality)
    Justified,
    /// Block is finalized and cannot be reorganized
    Finalized,
}

/// Chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: String,
    pub rpc_url: String,
    pub api_key: Option<String>,
    pub confirmations: u32,
    pub max_retries: u32,
    pub timeout_secs: u64,
    pub extra: serde_json::Value,
}

/// Normalized blockchain data representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainDataIR {
    pub block_metadata: BlockMetadata,
    pub state_changes: Vec<StateChange>,
    pub consensus_metadata: ConsensusMetadata,
    pub finality_status: FinalityStatus,
}

/// Block metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetadata {
    pub block_height: u64,
    pub block_hash: String,
    pub parent_hash: String,
    pub timestamp: u64,
    pub state_root: String,
    pub receipts_root: Option<String>,
    pub transactions_root: Option<String>,
}

/// State change representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub contract_address: String,
    pub storage_slot: [u8; 32],
    pub previous_value: Option<[u8; 32]>,
    pub new_value: [u8; 32],
    pub proof: Option<StorageProof>,
}

/// Storage proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProof {
    pub key: [u8; 32],
    pub value: [u8; 32],
    pub proof_nodes: Vec<Vec<u8>>,
    pub proof_type: ProofType,
}

/// Proof type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofType {
    MerklePatricia,
    SparseMerkle,
    Binary,
    Custom(String),
}

/// Consensus metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMetadata {
    pub validator_set_hash: Option<String>,
    pub proposer: Option<String>,
    pub round: Option<u64>,
    pub signatures: Option<Vec<String>>,
    pub extra: HashMap<String, serde_json::Value>,
}