/// Common types and utilities for Almanac indexers
use std::io;
use thiserror::Error;

/// Result type for indexer operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for indexer operations
#[derive(Error, Debug)]
pub enum Error {
    /// Generic error with message
    #[error("Error: {0}")]
    Generic(String),
    
    /// Database error
    #[error("Database error: {0}")]
    Database(String),
    
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),
    
    /// Missing service error
    #[error("Missing service for chain: {0}")]
    MissingService(String),
    
    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),
    
    /// Invalid data error
    #[error("Invalid data: {0}")]
    InvalidData(String),
    
    /// IO error
    #[error("IO error: {0}")]
    IO(#[from] io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Chain specific error
    #[error("Chain error ({chain}): {message}")]
    Chain {
        chain: String,
        message: String,
    },

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),
    
    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    /// Authorization error
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// API error
    #[error("API error: {0}")]
    Api(String),
    
    /// Invalid event error
    #[error("Invalid event: {0}")]
    InvalidEvent(String),
}

impl Error {
    /// Create a new generic error
    pub fn generic<S: Into<String>>(msg: S) -> Self {
        Error::Generic(msg.into())
    }
    
    /// Create a new database error
    pub fn database<S: Into<String>>(msg: S) -> Self {
        Error::Database(msg.into())
    }
    
    /// Create a new storage error
    pub fn storage<S: Into<String>>(msg: S) -> Self {
        Error::Storage(msg.into())
    }
    
    /// Create a new missing service error
    pub fn missing_service<S: Into<String>>(chain_id: S) -> Self {
        Error::MissingService(chain_id.into())
    }
    
    /// Create a new not found error
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        Error::NotFound(msg.into())
    }
    
    /// Create a new invalid data error
    pub fn invalid_data<S: Into<String>>(msg: S) -> Self {
        Error::InvalidData(msg.into())
    }
    
    /// Create a new chain error
    pub fn chain<S1: Into<String>, S2: Into<String>>(chain: S1, message: S2) -> Self {
        Error::Chain {
            chain: chain.into(),
            message: message.into(),
        }
    }
    
    /// Create a new connection error
    pub fn connection<S: Into<String>>(msg: S) -> Self {
        Error::Connection(msg.into())
    }
    
    /// Create a new authentication error
    pub fn authentication<S: Into<String>>(msg: S) -> Self {
        Error::Authentication(msg.into())
    }
    
    /// Create a new authorization error
    pub fn authorization<S: Into<String>>(msg: S) -> Self {
        Error::Authorization(msg.into())
    }
    
    /// Create a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Error::Config(msg.into())
    }
    
    /// Create a new API error
    pub fn api<S: Into<String>>(msg: S) -> Self {
        Error::Api(msg.into())
    }
    
    /// Create a new invalid event error
    pub fn invalid_event<S: Into<String>>(msg: S) -> Self {
        Error::InvalidEvent(msg.into())
    }
}

// Implement error conversions for database errors
#[cfg(feature = "postgres")]
impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Database(format!("Database error: {}", err))
    }
}

#[cfg(feature = "postgres")]
impl From<sqlx::migrate::MigrateError> for Error {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        Error::Database(format!("Migration error: {}", err))
    }
}

#[cfg(feature = "rocks")]
impl From<rocksdb::Error> for Error {
    fn from(err: rocksdb::Error) -> Self {
        Error::Storage(format!("RocksDB error: {}", err))
    }
}

/// Block processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStatus {
    /// Block is confirmed (included in the chain)
    Confirmed,
    /// Block is safe (unlikely to be orphaned)
    Safe,
    /// Block is justified (voted by validators)
    Justified,
    /// Block is finalized (irreversible)
    Finalized,
}

impl BlockStatus {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockStatus::Confirmed => "confirmed",
            BlockStatus::Safe => "safe",
            BlockStatus::Justified => "justified",
            BlockStatus::Finalized => "finalized",
        }
    }
    
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "confirmed" => Some(BlockStatus::Confirmed),
            "safe" => Some(BlockStatus::Safe),
            "justified" => Some(BlockStatus::Justified),
            "finalized" => Some(BlockStatus::Finalized),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_status_string_conversion() {
        assert_eq!(BlockStatus::Confirmed.as_str(), "confirmed");
        assert_eq!(BlockStatus::Safe.as_str(), "safe");
        assert_eq!(BlockStatus::Justified.as_str(), "justified");
        assert_eq!(BlockStatus::Finalized.as_str(), "finalized");

        assert_eq!(BlockStatus::from_str("confirmed"), Some(BlockStatus::Confirmed));
        assert_eq!(BlockStatus::from_str("safe"), Some(BlockStatus::Safe));
        assert_eq!(BlockStatus::from_str("justified"), Some(BlockStatus::Justified));
        assert_eq!(BlockStatus::from_str("finalized"), Some(BlockStatus::Finalized));
        assert_eq!(BlockStatus::from_str("unknown"), None);
    }

    #[test]
    fn test_error_creation_methods() {
        let generic_err = Error::generic("test error");
        match generic_err {
            Error::Generic(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Wrong error type"),
        }

        let db_err = Error::database("db error");
        match db_err {
            Error::Database(msg) => assert_eq!(msg, "db error"),
            _ => panic!("Wrong error type"),
        }

        let chain_err = Error::chain("ethereum", "tx failed");
        match chain_err {
            Error::Chain { chain, message } => {
                assert_eq!(chain, "ethereum");
                assert_eq!(message, "tx failed");
            },
            _ => panic!("Wrong error type"),
        }
    }
} 