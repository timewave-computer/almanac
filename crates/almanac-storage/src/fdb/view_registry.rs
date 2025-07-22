//! View registry types for FDB storage

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// View definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDefinition {
    pub view_id: String,
    pub contract_address: String,
    pub chain_id: String,
    pub slots: Vec<SlotDefinition>,
    pub decode_types: HashMap<String, String>,
    pub trust_policy: ViewTrustPolicy,
    pub created_by: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub description: Option<String>,
}

/// Slot definition in a view
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotDefinition {
    pub slot: String,
    pub field: String,
    pub decode_type: String,
    pub semantic_tags: Vec<String>,
}

/// Trust policy for views
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewTrustPolicy {
    Strict,
    Permissive,
    Explicit,
}

/// View query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewQueryResult {
    pub view_id: String,
    pub chain_id: String,
    pub contract_address: String,
    pub block_height: u64,
    pub slots: Vec<SlotResult>,
    pub proofs: Option<Vec<StorageProof>>,
    pub timestamp: u64,
}

/// Individual slot result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotResult {
    pub slot: String,
    pub field: String,
    pub raw_value: Vec<u8>,
    pub decoded_value: serde_json::Value,
    pub semantic_metadata: Option<SemanticInfo>,
}

/// Semantic information for a slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticInfo {
    pub data_type: String,
    pub semantic_tags: Vec<String>,
    pub zero_semantics: Option<String>,
    pub confidence: String,
    pub source: String,
}

/// Storage proof for a slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProof {
    pub slot: String,
    pub value: Vec<u8>,
    pub proof: Vec<Vec<u8>>,
    pub state_root: [u8; 32],
}
