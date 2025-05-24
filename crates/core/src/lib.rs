/// Indexer core types and utilities
pub mod event;
pub mod types;
pub mod service;
pub mod reorg;
pub mod proto;
pub mod config;
pub mod security;
pub mod text_search;
pub mod aggregation;
pub mod correlation;
pub mod indexing;
pub mod caching;
pub mod pool;
pub mod migrations;
pub mod cross_chain;
pub mod validation;
pub mod sync_tracker;
pub mod reorg_handler;

/// Re-export common types from indexer-common
pub use indexer_common::{Error, Result, BlockStatus}; 