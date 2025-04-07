/// Indexer core types and utilities
pub mod event;
pub mod types;
pub mod service;
pub mod reorg;

/// Re-export common types from indexer-common
pub use indexer_common::{Error, Result, BlockStatus}; 