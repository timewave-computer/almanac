//! Chain-specific adapter implementations
//!
//! This module contains the adapter implementations for different blockchains,
//! providing normalized interfaces for blockchain data extraction and proof generation.

pub mod traits;
pub mod types;

#[cfg(feature = "ethereum")]
pub mod ethereum;

#[cfg(feature = "cosmos")]
pub mod cosmos;

// #[cfg(feature = "solana")]
// pub mod solana;

// Re-exports
pub use traits::{ChainAdapter, ChainAdapterError};
pub use types::{ChainConfig, ChainDataIR, BlockMetadata, StateChange, StorageProof, FinalityStatus};