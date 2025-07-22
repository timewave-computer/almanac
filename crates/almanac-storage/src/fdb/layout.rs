//! Contract layout types for FDB storage

use serde::{Deserialize, Serialize};

/// Contract layout key
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractLayoutKey {
    pub chain_id: String,
    pub contract_address: String,
    pub version: Option<String>,
}

/// Contract layout value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractLayoutValue {
    pub layout_commitment: [u8; 32],
    pub layout_data: Vec<u8>,
    pub compiler_version: String,
    pub source_hash: Option<[u8; 32]>,
    pub provided_by: String,
    pub project_id: Option<String>,
    pub documentation_url: Option<String>,
    pub verification_status: VerificationStatus,
    pub timestamp: u64,
}

/// Layout verification status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    Unverified = 0,
    Pending = 1,
    Verified = 2,
    Failed = 3,
}

/// Layout conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConflict {
    pub chain_id: String,
    pub contract_address: String,
    pub conflicting_layouts: Vec<(String, ContractLayoutValue)>, // (version, layout)
    pub detected_at: u64,
    pub resolution: Option<LayoutConflictResolution>,
}

/// Layout conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConflictResolution {
    pub chosen_version: String,
    pub reason: String,
    pub resolved_by: String,
    pub resolved_at: u64,
}
