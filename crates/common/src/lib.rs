/// Common types and utilities for the indexer

/// Block status in the chain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStatus {
    /// Block is included in chain (may be reversible)
    Confirmed,
    
    /// Block has enough attestations to be unlikely to be orphaned
    Safe,
    
    /// Block has been voted on by validators in current epoch
    Justified,
    
    /// Block has been irreversibly agreed upon
    Finalized,
}

impl std::fmt::Display for BlockStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockStatus::Confirmed => write!(f, "confirmed"),
            BlockStatus::Safe => write!(f, "safe"),
            BlockStatus::Justified => write!(f, "justified"),
            BlockStatus::Finalized => write!(f, "finalized"),
        }
    }
}

impl From<&str> for BlockStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "safe" => BlockStatus::Safe,
            "justified" => BlockStatus::Justified,
            "finalized" => BlockStatus::Finalized,
            _ => BlockStatus::Confirmed,
        }
    }
}

/// Error type for indexer operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Generic error with message
    #[error("{0}")]
    Generic(String),
    
    /// Database error
    #[error("Database error: {0}")]
    Database(String),
    
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),
    
    /// Missing event service
    #[error("Missing event service for chain: {0}")]
    MissingService(String),
    
    /// Invalid event data
    #[error("Invalid event data: {0}")]
    InvalidEvent(String),
    
    /// API error
    #[error("API error: {0}")]
    Api(String),

    /// RocksDB error
    #[error("RocksDB error: {0}")]
    RocksDB(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
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
    
    /// Create a new invalid event error
    pub fn invalid_event<S: Into<String>>(msg: S) -> Self {
        Error::InvalidEvent(msg.into())
    }
    
    /// Create a new API error
    pub fn api<S: Into<String>>(msg: S) -> Self {
        Error::Api(msg.into())
    }
    
    /// Create a new RocksDB error
    pub fn rocksdb<S: Into<String>>(msg: S) -> Self {
        Error::RocksDB(msg.into())
    }
    
    /// Create a new serialization error
    pub fn serialization<S: Into<String>>(msg: S) -> Self {
        Error::Serialization(msg.into())
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

// Implement From for common error types
#[cfg(feature = "postgres")]
impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Database(format!("SQLx error: {}", err))
    }
}

#[cfg(feature = "rocks")]
impl From<rocksdb::Error> for Error {
    fn from(err: rocksdb::Error) -> Self {
        Error::RocksDB(format!("RocksDB error: {}", err))
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(format!("JSON serialization error: {}", err))
    }
}

#[cfg(feature = "postgres")]
impl From<sqlx::migrate::MigrateError> for Error {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        Error::Database(format!("Migration error: {}", err))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Storage(format!("IO error: {}", err))
    }
} 