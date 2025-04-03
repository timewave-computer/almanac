use thiserror::Error;

#[cfg(feature = "sqlx-postgres")]
use sqlx;

/// Core error types for the indexer
#[derive(Debug, Error)]
pub enum Error {
    /// Generic error with a message
    #[error("{0}")]
    Generic(String),

    /// Error when parsing data
    #[error("Failed to parse data: {0}")]
    Parse(String),

    /// Error when validating data
    #[error("Data validation failed: {0}")]
    Validation(String),

    /// Error when interacting with a chain
    #[error("Chain error: {0}")]
    Chain(String),

    /// Error when interacting with storage
    #[error("Storage error: {0}")]
    Storage(String),

    /// Error related to network operations
    #[error("Network error: {0}")]
    Network(String),
    
    /// Error related to serialization or deserialization
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Any other error with its source
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    /// Create a new generic error
    pub fn generic<S: Into<String>>(msg: S) -> Self {
        Error::Generic(msg.into())
    }

    /// Create a new parse error
    pub fn parse<S: Into<String>>(msg: S) -> Self {
        Error::Parse(msg.into())
    }

    /// Create a new validation error
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Error::Validation(msg.into())
    }

    /// Create a new chain error
    pub fn chain<S: Into<String>>(msg: S) -> Self {
        Error::Chain(msg.into())
    }

    /// Create a new storage error
    pub fn storage<S: Into<String>>(msg: S) -> Self {
        Error::Storage(msg.into())
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(msg: S) -> Self {
        Error::Network(msg.into())
    }
    
    /// Create a new serialization error
    pub fn serialization<S: Into<String>>(msg: S) -> Self {
        Error::Serialization(msg.into())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Generic(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Generic(s.to_string())
    }
}

// Add implementation for serde_json errors
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Parse(format!("JSON parsing error: {}", err))
    }
}

#[cfg(feature = "sqlx-postgres")]
mod sqlx_impls {
    use super::Error;

    // Add implementation for sqlx errors
    impl From<sqlx::Error> for Error {
        fn from(err: sqlx::Error) -> Self {
            Error::Storage(format!("Database error: {}", err))
        }
    }

    // Add implementation for sqlx migrate errors
    impl From<sqlx::migrate::MigrateError> for Error {
        fn from(err: sqlx::migrate::MigrateError) -> Self {
            Error::Storage(format!("Migration error: {}", err))
        }
    }
}

// Add implementation for std I/O errors
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Storage(format!("I/O error: {}", err))
    }
} 