//! Storage layer for Almanac with FoundationDB backend
//!
//! This crate provides the storage implementation for Almanac,
//! focusing on FoundationDB with tuple-based encoding.

use almanac_core::{Result, Storage, StorageReader, StorageWriter};
use std::sync::Arc;

pub mod api_adapter;
#[cfg(feature = "fdb")]
pub mod fdb;
pub mod manager;
pub mod memory;
pub mod memory_extended;
mod traits;

// Re-exports
pub use api_adapter::ExtendedStorageReader;
#[cfg(feature = "fdb")]
pub use fdb::{FdbAccountSubscriptionManager, FdbConfig, FdbStorage, FdbStorageExt};
pub use manager::AppState;
#[cfg(feature = "fdb")]
pub use manager::StorageManager;
pub use memory::MemoryStorage;
pub use memory_extended::MemoryExtendedStorage;
pub use traits::*;

/// Storage backend type
#[derive(Debug, Clone)]
pub enum StorageBackend {
    /// In-memory storage (for testing)
    Memory,
    /// FoundationDB storage
    #[cfg(feature = "fdb")]
    Fdb(fdb::FdbConfig),
}

/// Create a storage reader instance based on configuration
pub async fn create_storage_reader(backend: StorageBackend) -> Result<Arc<dyn StorageReader>> {
    match backend {
        StorageBackend::Memory => Ok(Arc::new(MemoryStorage::new())),
        #[cfg(feature = "fdb")]
        StorageBackend::Fdb(config) => {
            let storage = fdb::FdbStorage::new(config).await?;
            Ok(Arc::new(storage))
        }
    }
}

/// Create a storage writer instance based on configuration  
pub async fn create_storage_writer(backend: StorageBackend) -> Result<Arc<dyn StorageWriter>> {
    match backend {
        StorageBackend::Memory => Ok(Arc::new(MemoryStorage::new())),
        #[cfg(feature = "fdb")]
        StorageBackend::Fdb(config) => {
            let storage = fdb::FdbStorage::new(config).await?;
            Ok(Arc::new(storage))
        }
    }
}

/// Create a unified storage instance (both reader and writer) based on configuration
pub async fn create_storage(backend: StorageBackend) -> Result<Arc<dyn Storage + Send + Sync>> {
    match backend {
        StorageBackend::Memory => Ok(Arc::new(MemoryStorage::new())),
        #[cfg(feature = "fdb")]
        StorageBackend::Fdb(config) => {
            let storage = fdb::FdbStorage::new(config).await?;
            Ok(Arc::new(storage))
        }
    }
}
