/// Error types for the causality indexer
use thiserror::Error;

/// Result type for causality operations
pub type Result<T> = std::result::Result<T, CausalityError>;

/// Errors that can occur in the causality indexer
#[derive(Error, Debug)]
pub enum CausalityError {
    /// SMT operation failed
    #[error("SMT operation failed: {0}")]
    SmtError(String),

    /// Storage operation failed
    #[error("Storage operation failed: {0}")]
    StorageError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid proof
    #[error("Invalid proof: {0}")]
    InvalidProof(String),

    /// Invalid key format
    #[error("Invalid key format: {0}")]
    InvalidKey(String),

    /// Invalid hash format
    #[error("Invalid hash format: {0}")]
    InvalidHash(String),

    /// Causality relation not found
    #[error("Causality relation not found: {0}")]
    RelationNotFound(String),

    /// Cross-chain reference not found
    #[error("Cross-chain reference not found: {0}")]
    CrossChainRefNotFound(String),

    /// Resource flow not found
    #[error("Resource flow not found: {0}")]
    ResourceFlowNotFound(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Generic error
    #[error("Generic error: {0}")]
    Generic(String),

    /// Core indexer error
    #[error("Core indexer error: {0}")]
    CoreError(indexer_core::Error),

    /// Storage error
    #[error("Storage error: {0}")]
    IndexerStorageError(indexer_storage::Error),

    /// Anyhow error
    #[error("Anyhow error: {0}")]
    AnyhowError(anyhow::Error),

    /// Hex decoding error
    #[error("Hex decoding error: {0}")]
    HexError(#[from] hex::FromHexError),

    /// JSON serialization error
    #[cfg(feature = "serde")]
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl From<indexer_storage::Error> for CausalityError {
    fn from(err: indexer_storage::Error) -> Self {
        Self::IndexerStorageError(err)
    }
}

impl From<anyhow::Error> for CausalityError {
    fn from(err: anyhow::Error) -> Self {
        Self::AnyhowError(err)
    }
}

impl CausalityError {
    /// Create a generic error
    pub fn generic(msg: impl Into<String>) -> Self {
        Self::Generic(msg.into())
    }

    /// Create an SMT error
    pub fn smt_error(msg: impl Into<String>) -> Self {
        Self::SmtError(msg.into())
    }

    /// Create a storage error
    pub fn storage_error(msg: impl Into<String>) -> Self {
        Self::StorageError(msg.into())
    }

    /// Create a serialization error
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        Self::SerializationError(msg.into())
    }

    /// Create an invalid proof error
    pub fn invalid_proof(msg: impl Into<String>) -> Self {
        Self::InvalidProof(msg.into())
    }

    /// Create an invalid key error
    pub fn invalid_key(msg: impl Into<String>) -> Self {
        Self::InvalidKey(msg.into())
    }

    /// Create an invalid hash error
    pub fn invalid_hash(msg: impl Into<String>) -> Self {
        Self::InvalidHash(msg.into())
    }
} 