//! Storage layer for Almanac with FoundationDB backend
//!
//! This crate provides the storage implementation for Almanac,
//! focusing on FoundationDB with tuple-based encoding.

use almanac_core::{Result, Storage as StorageTrait};
use std::sync::Arc;

pub mod memory;
#[cfg(feature = "fdb")]
pub mod fdb;
mod traits;
pub mod api_adapter;

// Re-exports
pub use memory::MemoryStorage;
#[cfg(feature = "fdb")]
pub use fdb::{FdbStorage, FdbConfig, FdbStorageExt};
pub use traits::*;
pub use api_adapter::{ExtendedStorageReader, StorageReader};

/// Storage backend type
#[derive(Debug, Clone)]
pub enum StorageBackend {
    /// In-memory storage (for testing)
    Memory,
    /// FoundationDB storage
    #[cfg(feature = "fdb")]
    Fdb(fdb::FdbConfig),
}

/// Create a storage instance based on configuration
pub async fn create_storage(backend: StorageBackend) -> Result<Arc<dyn StorageTrait>> {
    match backend {
        StorageBackend::Memory => {
            Ok(Arc::new(MemoryStorage::new()))
        }
        #[cfg(feature = "fdb")]
        StorageBackend::Fdb(config) => {
            let storage = fdb::FdbStorage::new(config).await?;
            Ok(Arc::new(storage))
        }
    }
}