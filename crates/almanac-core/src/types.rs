//! Core types used throughout Almanac

use serde::{Deserialize, Serialize};
use std::fmt;

/// Chain identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub String);

impl ChainId {
    /// Create a new chain ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the chain ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ChainId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ChainId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Block processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockStatus {
    /// Block is pending processing
    Pending,
    /// Block is currently being processed
    Processing,
    /// Block has been processed successfully
    Processed,
    /// Block processing failed
    Failed,
    /// Block has been confirmed/finalized
    Finalized,
    /// Block was part of a chain reorganization
    Reorged,
}

impl fmt::Display for BlockStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockStatus::Pending => write!(f, "pending"),
            BlockStatus::Processing => write!(f, "processing"),
            BlockStatus::Processed => write!(f, "processed"),
            BlockStatus::Failed => write!(f, "failed"),
            BlockStatus::Finalized => write!(f, "finalized"),
            BlockStatus::Reorged => write!(f, "reorged"),
        }
    }
}

/// Common chain identifiers
pub mod chains {
    /// Ethereum mainnet
    pub const ETHEREUM: &str = "ethereum";
    /// Cosmos Hub
    pub const COSMOS: &str = "cosmoshub";
    /// Osmosis
    pub const OSMOSIS: &str = "osmosis";
    /// Neutron
    pub const NEUTRON: &str = "neutron";
}
