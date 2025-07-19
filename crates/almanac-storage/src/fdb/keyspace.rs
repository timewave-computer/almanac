//! FoundationDB keyspace definitions
//!
//! This module defines the tuple-based keyspaces used for storing
//! different types of data in FoundationDB.

/// Keyspace identifiers for FoundationDB
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TupleKeyspace {
    /// Contract storage slots
    ContractState = 1,
    /// Block headers and consensus data
    BlockMeta = 2,
    /// Merkle proofs for verification
    StorageProofs = 3,
    /// Trust-scoped semantic interpretations
    SemanticMeta = 4,
    /// Developer trust relationships
    TrustGraph = 5,
    /// Layout commitments from Traverse
    ContractLayouts = 6,
    /// Custom view definitions
    ViewRegistry = 7,
    /// Recursive proof coordination
    LightwaveState = 8,
    /// Network checkpoints
    TrustedCheckpoints = 9,
    /// SP1 verification keys
    CircuitKeys = 10,
    /// Solana account data
    SolanaAccounts = 11,
    /// State change logs
    ChangeTracking = 12,
}

impl TupleKeyspace {
    /// Get the numeric identifier for the keyspace
    pub fn id(&self) -> u8 {
        *self as u8
    }
    
    /// Get a human-readable name for the keyspace
    pub fn name(&self) -> &'static str {
        match self {
            TupleKeyspace::ContractState => "contract_state",
            TupleKeyspace::BlockMeta => "block_meta",
            TupleKeyspace::StorageProofs => "storage_proofs",
            TupleKeyspace::SemanticMeta => "semantic_meta",
            TupleKeyspace::TrustGraph => "trust_graph",
            TupleKeyspace::ContractLayouts => "contract_layouts",
            TupleKeyspace::ViewRegistry => "view_registry",
            TupleKeyspace::LightwaveState => "lightwave_state",
            TupleKeyspace::TrustedCheckpoints => "trusted_checkpoints",
            TupleKeyspace::CircuitKeys => "circuit_keys",
            TupleKeyspace::SolanaAccounts => "solana_accounts",
            TupleKeyspace::ChangeTracking => "change_tracking",
        }
    }
}