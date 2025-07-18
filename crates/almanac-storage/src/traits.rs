//! Additional storage traits and types
//!
//! This module contains storage-specific traits and types that extend
//! the core storage functionality.

use almanac_core::{Result, Error};
use serde::{Serialize, Deserialize};

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total number of events stored
    pub total_events: u64,
    /// Total storage size in bytes
    pub total_size_bytes: u64,
    /// Number of chains being indexed
    pub chain_count: usize,
    /// Latest block by chain
    pub latest_blocks: std::collections::HashMap<String, u64>,
}

/// Storage health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHealth {
    /// Whether the storage is healthy
    pub is_healthy: bool,
    /// Connection status
    pub connection_status: ConnectionStatus,
    /// Error message if unhealthy
    pub error: Option<String>,
    /// Last check timestamp
    pub last_check: std::time::SystemTime,
}

/// Connection status for storage backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Connected and operational
    Connected,
    /// Connecting to backend
    Connecting,
    /// Disconnected from backend
    Disconnected,
    /// Connection error
    Error(String),
}

/// Extended storage operations
#[async_trait::async_trait]
pub trait StorageExt: almanac_core::Storage {
    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats>;
    
    /// Check storage health
    async fn health_check(&self) -> Result<StorageHealth>;
    
    /// Compact storage (if supported)
    async fn compact(&self) -> Result<()> {
        Ok(()) // Default no-op
    }
    
    /// Create a backup (if supported)
    async fn backup(&self, _path: &str) -> Result<()> {
        Err(Error::storage("Backup not supported for this storage backend"))
    }
    
    /// Restore from backup (if supported)
    async fn restore(&self, _path: &str) -> Result<()> {
        Err(Error::storage("Restore not supported for this storage backend"))
    }
}