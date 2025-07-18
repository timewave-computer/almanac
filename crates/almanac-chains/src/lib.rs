//! Chain integrations for Almanac
//!
//! This crate provides blockchain-specific implementations
//! for event indexing.

pub mod adapters;

// Re-exports
pub use adapters::{
    ChainAdapter, ChainAdapterError, ChainConfig, ChainDataIR,
    BlockMetadata, FinalityStatus, StateChange, StorageProof,
};
pub use adapters::types::ConsensusMetadata;

// Chain-specific exports
#[cfg(feature = "ethereum")]
pub use adapters::ethereum::EthereumAdapter;

#[cfg(feature = "cosmos")]
pub use adapters::cosmos::CosmosAdapter;

// #[cfg(feature = "solana")]
// pub use adapters::solana::SolanaAdapter;

// Chain-specific consensus types (exported under feature flags)
#[cfg(feature = "ethereum")]
pub use adapters::ethereum::{
    EthereumConsensusConfig, EthereumFinalityConfig, EthereumForkChoiceRule,
    EthereumFinalityLevel, EthereumBlockInfo,
    EthereumConsensusUpdate
};

#[cfg(feature = "cosmos")]
pub use adapters::cosmos::{
    CosmosConsensusConfig, CosmosFinalityConfig, CosmosForkChoiceRule,
    CosmosFinalityLevel, CosmosBlockInfo,
    CosmosConsensusUpdate
};

// #[cfg(feature = "solana")]
// pub use adapters::solana::{
//     SolanaConsensusConfig, SolanaFinalityConfig, SolanaForkChoiceRule,
//     SolanaFinalityLevel, SolanaBlockInfo,
//     SolanaConsensusUpdate
// };