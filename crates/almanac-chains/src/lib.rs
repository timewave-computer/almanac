//! Chain integrations for Almanac
//!
//! This crate provides blockchain-specific implementations
//! for event indexing and account subscription.

pub mod adapters;
pub mod merkle_trait;
pub mod subscription;

// Valence domain client integration
#[cfg(feature = "valence")]
pub mod valence;

// Re-exports
pub use adapters::types::ConsensusMetadata;
pub use adapters::{
    BlockMetadata, ChainAdapter, ChainAdapterError, ChainConfig, ChainDataIR, FinalityStatus,
    StateChange, StorageProof,
};

// Account subscription exports
pub use subscription::{
    AccountAddress, AccountSubscriptionConfig, AccountSubscriptionError,
    AccountSubscriptionManager, AccountUpdate, ChainAccountSubscription, InclusionProof,
    SubscriptionEvent, SubscriptionEventHandler, SubscriptionFilter, SubscriptionStats,
};

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
    EthereumBlockInfo, EthereumConsensusConfig, EthereumConsensusUpdate, EthereumFinalityConfig,
    EthereumFinalityLevel, EthereumForkChoiceRule,
};

#[cfg(feature = "cosmos")]
pub use adapters::cosmos::{
    CosmosBlockInfo, CosmosConsensusConfig, CosmosConsensusUpdate, CosmosFinalityConfig,
    CosmosFinalityLevel, CosmosForkChoiceRule,
};

// #[cfg(feature = "solana")]
// pub use adapters::solana::{
//     SolanaConsensusConfig, SolanaFinalityConfig, SolanaForkChoiceRule,
//     SolanaFinalityLevel, SolanaBlockInfo,
//     SolanaConsensusUpdate
// };

// Valence domain client exports
#[cfg(feature = "valence")]
pub use valence::{
    ValenceClientManager, ValenceDomainClient, ValenceEvent, ValenceExecutionRequest,
    ValenceExecutionResult, ValenceExecutionStatus,
};

#[cfg(feature = "valence-ethereum")]
pub use valence::ethereum::ValenceEthereumClient;

#[cfg(feature = "valence-cosmos")]
pub use valence::cosmos::ValenceCosmosClient;

#[cfg(feature = "valence-solana")]
pub use valence::solana::ValenceSolanaClient;
