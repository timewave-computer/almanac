/// Indexer core types and utilities
pub mod event;
pub mod types;
pub mod service;
pub mod reorg;
pub mod proto;
pub mod config;
pub mod security;
pub mod text_search;

/// Re-export common types from indexer-common
pub use indexer_common::{Error, Result, BlockStatus}; 