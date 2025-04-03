pub mod event;
pub mod error;
pub mod types;
pub mod service;

pub use error::Error;

/// Result type for core indexer operations
pub type Result<T> = std::result::Result<T, Error>; 