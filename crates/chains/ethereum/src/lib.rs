/// Ethereum chain integration for Almanac
///
/// This crate provides integration with Ethereum chains using Alloy
/// for interacting with Ethereum-based blockchain networks.
pub mod codegen;
pub mod ethereum_impl;
pub mod ethereum_stub;

// Re-export main types
pub use ethereum_impl::EthereumClient;
pub use ethereum_stub::EthereumStubService; 