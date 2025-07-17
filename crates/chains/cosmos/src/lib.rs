/// Cosmos chain integration for Almanac
///
/// This crate provides integration with Cosmos chains using the Valence Domain Client
/// for interacting with Cosmos-based blockchain networks.
pub mod codegen;
pub mod contracts;
pub mod cosmos_impl;
pub mod cosmos_stub;

// Re-export main types
pub use cosmos_impl::CosmosClientWrapper;
pub use cosmos_stub::CosmosStubService;