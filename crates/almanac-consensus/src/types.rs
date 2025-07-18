//! Common types for consensus operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Block information for consensus tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block number
    pub number: u64,
    
    /// Block hash
    pub hash: String,
    
    /// Parent block hash
    pub parent_hash: String,
    
    /// Block timestamp
    pub timestamp: SystemTime,
    
    /// Number of confirmations
    pub confirmations: u64,
    
    /// Whether block is confirmed
    pub is_confirmed: bool,
    
    /// Events in this block
    pub event_count: u64,
    
    /// Block metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Consensus state for a chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusState {
    /// Chain identifier
    pub chain_id: String,
    
    /// Latest confirmed block
    pub confirmed_block: u64,
    
    /// Latest safe block
    pub safe_block: u64,
    
    /// Latest finalized block
    pub finalized_block: u64,
    
    /// Current head block
    pub head_block: u64,
    
    /// Active forks
    pub forks: Vec<ForkInfo>,
    
    /// Last update time
    pub last_updated: SystemTime,
}

/// Information about a fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkInfo {
    /// Fork identifier
    pub fork_id: String,
    
    /// Fork point block number
    pub fork_point: u64,
    
    /// Fork head block
    pub head_block: u64,
    
    /// Fork weight/difficulty
    pub weight: u128,
    
    /// Number of blocks in fork
    pub length: u64,
    
    /// Fork status
    pub status: ForkStatus,
}

/// Fork status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ForkStatus {
    /// Fork is active and competing
    Active,
    /// Fork is the canonical chain
    Canonical,
    /// Fork has been abandoned
    Abandoned,
    /// Fork is being evaluated
    Evaluating,
}