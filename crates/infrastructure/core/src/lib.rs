/// Core types, traits, and utilities for Almanac infrastructure
///
/// This crate contains only the essential types that other infrastructure
/// crates depend on. All specialized functionality has been moved to
/// dedicated submodules.
use thiserror::Error;

/// Result type for infrastructure operations
pub type Result<T> = std::result::Result<T, Error>;

/// Core error type for infrastructure operations
#[derive(Error, Debug, Clone)]
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
    IO(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    
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
    
    /// Create a new IO error
    pub fn io<S: Into<String>>(msg: S) -> Self {
        Error::IO(msg.into())
    }
    
    /// Create a new serialization error
    pub fn serialization<S: Into<String>>(msg: S) -> Self {
        Error::Serialization(msg.into())
    }
    
    /// Create a new deserialization error
    pub fn deserialization<S: Into<String>>(msg: S) -> Self {
        Error::Deserialization(msg.into())
    }
    
    /// Create a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Error::Config(msg.into())
    }
}

// From trait implementations for automatic error conversion
#[cfg(feature = "postgres")]
impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Database(err.to_string())
    }
}

#[cfg(feature = "postgres")]
impl From<sqlx::migrate::MigrateError> for Error {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        Error::Database(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Config(err.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Error::Config(err.to_string())
    }
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Error::IO(err.to_string())
    }
}

impl From<regex::Error> for Error {
    fn from(err: regex::Error) -> Self {
        Error::Generic(err.to_string())
    }
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        
        let (status, message) = match self {
            Error::Generic(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Storage(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::MissingService(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Error::InvalidData(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::IO(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Serialization(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Deserialization(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Chain { message, .. } => (StatusCode::BAD_REQUEST, message),
            Error::Connection(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            Error::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg),
            Error::Authorization(msg) => (StatusCode::FORBIDDEN, msg),
            Error::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Api(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::InvalidEvent(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let json = serde_json::json!({
            "error": message,
        });

        (status, axum::Json(json)).into_response()
    }
}

/// Block finality status for different chains
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockStatus {
    /// Block is confirmed but not yet safe
    Confirmed,
    /// Block is safe from reorgs (chain-specific criteria)
    Safe,
    /// Block is justified (Ethereum-specific)
    Justified,
    /// Block is finalized and cannot be reverted
    Finalized,
    /// Latest block (may be unsafe)
    Latest,
}

impl BlockStatus {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockStatus::Confirmed => "confirmed",
            BlockStatus::Safe => "safe",
            BlockStatus::Justified => "justified",
            BlockStatus::Finalized => "finalized",
            BlockStatus::Latest => "latest",
        }
    }
}

impl std::str::FromStr for BlockStatus {
    type Err = ();
    
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "confirmed" => Ok(BlockStatus::Confirmed),
            "safe" => Ok(BlockStatus::Safe),
            "justified" => Ok(BlockStatus::Justified),
            "finalized" => Ok(BlockStatus::Finalized),
            "latest" => Ok(BlockStatus::Latest),
            _ => Err(()),
        }
    }
}

// Core modules with essential types only
pub mod types;
pub mod config;
pub mod health;
pub mod event_bus;
pub mod di;
pub mod service;
pub mod security;
pub mod cluster_state;

// Re-export commonly used types
pub use types::{Event, Block, Transaction, ChainId, BlockNumber, EventId, ApiConfig};
pub use config::Config;
pub use health::{HealthCheck, HealthStatus};
pub use service::BoxedEventService;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = Error::generic("test error");
        assert_eq!(err.to_string(), "Error: test error");
        
        let err = Error::database("connection failed");
        assert_eq!(err.to_string(), "Database error: connection failed");
    }
    
    #[test]
    fn test_block_status() {
        assert_eq!(BlockStatus::Finalized.as_str(), "finalized");
        assert_eq!("finalized".parse::<BlockStatus>().unwrap(), BlockStatus::Finalized);
    }
}