//! Core domain types and traits for Almanac
//!
//! This crate provides the fundamental types and traits used throughout
//! the Almanac indexing system.

pub mod error;
pub mod event;
pub mod filter;
pub mod messaging;
pub mod storage;
pub mod types;

// Re-exports
pub use error::{Error, Result};
pub use event::{Event, EventSource, UnifiedEvent};
pub use filter::EventFilter;
pub use messaging::{EventPublisher, EventSubscriber, InProcessPubSub, RawBlockEvent};
pub use storage::{Storage, StorageReader, StorageWriter};
pub use types::{BlockStatus, ChainId};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
