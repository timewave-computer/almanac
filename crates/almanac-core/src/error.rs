//! Core error types for Almanac

use thiserror::Error;

/// Core error type for Almanac
#[derive(Error, Debug)]
pub enum Error {
    /// Storage operation failed
    #[error("Storage error: {0}")]
    Storage(String),

    /// Invalid configuration
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Chain interaction error
    #[error("Chain error for {chain}: {message}")]
    Chain { chain: String, message: String },

    /// Event processing error
    #[error("Event processing error: {0}")]
    EventProcessing(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic error with context
    #[error("{0}")]
    Generic(String),

    /// Error from another source
    #[error(transparent)]
    Other(#[from] anyhow::Error),

    /// FoundationDB error
    #[cfg(feature = "fdb")]
    #[error("FoundationDB error: {0}")]
    FoundationDB(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create a configuration error
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    /// Create a chain error
    pub fn chain(chain: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Chain {
            chain: chain.into(),
            message: message.into(),
        }
    }

    /// Create an event processing error
    pub fn event_processing(msg: impl Into<String>) -> Self {
        Self::EventProcessing(msg.into())
    }

    /// Create a generic error
    pub fn generic(msg: impl Into<String>) -> Self {
        Self::Generic(msg.into())
    }
}

#[cfg(feature = "fdb")]
impl From<foundationdb::FdbError> for Error {
    fn from(err: foundationdb::FdbError) -> Self {
        Self::FoundationDB(err.to_string())
    }
}
