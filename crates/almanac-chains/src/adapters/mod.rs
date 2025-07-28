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

#[cfg(any(feature = "solana", feature = "solana-core"))]
pub mod solana;
#[cfg(feature = "solana")]
pub mod solana_geyser; // Real Geyser streaming implementation (requires gRPC)
#[cfg(any(feature = "solana", feature = "solana-core"))]
pub mod solana_native_merkle; // Native Solana merkle tree
#[cfg(any(feature = "solana", feature = "solana-core"))]
pub mod solana_idl; // Anchor IDL detection and parsing
#[cfg(feature = "solana")]
pub mod solana_idl_fetcher; // Non-blocking IDL fetcher service (requires gRPC)

// Re-exports
pub use traits::{ChainAdapter, ChainAdapterError};
pub use types::{
    BlockMetadata, ChainConfig, ChainDataIR, FinalityStatus, StateChange, StorageProof,
};
