//! SMT-based causality indexing for the Almanac cross-chain indexer
//!
//! This crate provides Sparse Merkle Tree (SMT) based indexing capabilities
//! for tracking causal relationships between events across different chains.
//! It integrates with the reverse-causality framework to enable verifiable
//! cross-domain computations and zero-knowledge proofs.
//!
//! ## Compatibility
//!
//! This crate defaults to SHA256 hashing for SMT operations to maintain
//! compatibility with the reverse-causality project. While Blake3 is also
//! supported, SHA256 is the recommended choice for production use.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

/// Sparse Merkle Tree implementation
pub mod smt;
/// Causality tracking and relationship management
pub mod causality;
/// Storage backends for causality data
pub mod storage;
/// Main causality indexer implementation
pub mod indexer;
/// Core types for causality indexing
pub mod types;
/// Error types for causality operations
pub mod error;

// Re-export core types and traits
pub use error::{CausalityError, Result};
pub use types::{
    CausalityEvent, CausalityResource, CausalityProof, 
    SmtRoot, SmtKey, SmtProof, CausalityIndex, SmtHasher,
    ResourceFlow, CrossChainReference
};
pub use smt::{
    SmtBackend, MemorySmtBackend, PostgresSmtBackend,
    SparseMerkleTree, Blake3SmtHasher, Sha256SmtHasher
};
pub use causality::{
    CausalityTracker, CausalityRelation, CausalityGraph,
    CrossChainCausality
};
pub use indexer::{
    CausalityIndexer, CausalityIndexerConfig, CausalityEventProcessor
};
pub use storage::{
    CausalityStorage, SmtStorage, CausalityStorageBackend
};

/// Version of the causality indexer
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default SMT depth for causality trees
pub const DEFAULT_SMT_DEPTH: usize = 256;

/// Default namespace for causality events
pub const CAUSALITY_NAMESPACE: &str = "almanac-causality";

/// Default namespace for cross-chain references
pub const CROSS_CHAIN_NAMESPACE: &str = "almanac-cross-chain";

/// Default namespace for resource flows
pub const RESOURCE_FLOW_NAMESPACE: &str = "almanac-resource-flow"; 