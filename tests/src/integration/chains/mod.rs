//! Chain-specific integration tests

#[cfg(feature = "ethereum")]
pub mod ethereum_indexing;

#[cfg(feature = "solana")]
pub mod solana_accounts;

#[cfg(feature = "solana")]
pub mod solana_idl_indexing;

#[cfg(feature = "solana")]
pub mod test_accounts;