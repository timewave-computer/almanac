//! Core domain types and traits for Almanac
//! 
//! This crate provides the fundamental types and traits used throughout
//! the Almanac indexing system.

pub mod error;
pub mod event;
pub mod filter;
pub mod storage;
pub mod types;

// Re-exports
pub use error::{Error, Result};
pub use event::{Event, UnifiedEvent, EventSource};
pub use filter::EventFilter;
pub use storage::Storage;
pub use types::{BlockStatus, ChainId};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");