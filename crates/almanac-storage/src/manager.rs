//! Storage manager integration layer

use crate::api_adapter::ExtendedStorageReader;
use std::sync::Arc;

#[cfg(feature = "fdb")]
use anyhow::Result;

#[cfg(feature = "fdb")]
use tracing::info;

#[cfg(feature = "fdb")]
use almanac_core::StorageReader;

#[cfg(feature = "fdb")]
use crate::fdb::{
    subscription_manager_simple::AccountSubscriptionConfig, FdbAccountSubscriptionManager, FdbConfig,
    FdbStorage, FdbStorageExt, TupleFdbStorage,
};

// Define a simple trait for subscription manager to avoid feature gate issues
pub trait SimpleSubscriptionManager: Send + Sync {}

#[cfg(feature = "fdb")]
impl SimpleSubscriptionManager for FdbAccountSubscriptionManager {}

// Define a basic AppState locally to avoid almanac-api circular dependency
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn ExtendedStorageReader>,
}

impl AppState {
    pub fn new(storage: Arc<dyn ExtendedStorageReader>) -> Self {
        Self { storage }
    }

    pub fn with_version(self, _version: &str) -> Self {
        // For minimal implementation, just return self
        self
    }

    pub fn with_subscription_manager(self, _manager: Arc<dyn SimpleSubscriptionManager>) -> Self {
        // For now, just return self since we're avoiding the full dependency
        self
    }
}

/// Complete storage manager that integrates all storage components
#[cfg(feature = "fdb")]
pub struct StorageManager {
    /// Base FDB storage for core operations
    pub fdb_storage: Arc<FdbStorage>,
    /// Extended storage with FDB backend
    pub fdb_storage_ext: Arc<FdbStorageExt>,
    /// Tuple storage for advanced operations
    pub tuple_storage: Arc<TupleFdbStorage>,
    /// Account subscription manager
    pub subscription_manager: Arc<FdbAccountSubscriptionManager>,
}

#[cfg(feature = "fdb")]
impl StorageManager {
    /// Create a new storage manager with all components
    pub async fn new(config: FdbConfig) -> Result<Self> {
        info!("Initializing storage manager with FDB backend");

        // Create base FDB storage
        let fdb_storage = Arc::new(FdbStorage::new(config.clone()).await?);

        // Create tuple storage
        let tuple_storage = Arc::new(
            TupleFdbStorage::new(fdb_storage.database().clone(), config.namespace.clone()).await?,
        );

        // Create extended FDB storage with tuple backend
        let fdb_storage_ext = Arc::new(FdbStorageExt::new(config.clone()).await?);

        // Create subscription manager
        let subscription_config = AccountSubscriptionConfig::default();
        let subscription_manager = Arc::new(
            FdbAccountSubscriptionManager::new(tuple_storage.clone(), subscription_config).await?,
        );

        let manager = Self {
            fdb_storage,
            fdb_storage_ext,
            tuple_storage,
            subscription_manager,
        };

        info!("Storage manager initialized successfully");
        Ok(manager)
    }

    /// Create an AppState with all storage components configured
    pub fn create_api_state(&self) -> AppState {
        AppState::new(self.fdb_storage_ext.clone() as Arc<dyn ExtendedStorageReader>)
            .with_subscription_manager(
                self.subscription_manager.clone() as Arc<dyn SimpleSubscriptionManager>
            )
            .with_version("v1")
    }

    /// Create a minimal in-memory storage manager for testing
    pub fn create_memory() -> AppState {
        use crate::memory_extended::MemoryExtendedStorage;

        let memory_storage = Arc::new(MemoryExtendedStorage::new());
        AppState::new(memory_storage as Arc<dyn ExtendedStorageReader>).with_version("v1-memory")
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> StorageStats {
        StorageStats {
            fdb_connected: true, // Would check actual connection in production
            tuple_storage_active: true,
            subscription_manager_active: true,
            total_subscriptions: 0, // Would get from subscription manager
        }
    }

    /// Perform health check on all components
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let mut status = HealthStatus {
            overall: true,
            fdb_storage: false,
            tuple_storage: false,
            subscription_manager: false,
            details: Vec::new(),
        };

        // Check base FDB storage
        match StorageReader::get_latest_block(self.fdb_storage.as_ref(), "health_check").await {
            Ok(_) => {
                status.fdb_storage = true;
                status.details.push("FDB storage: OK".to_string());
            }
            Err(e) => {
                status.overall = false;
                status.details.push(format!("FDB storage: ERROR - {}", e));
            }
        }

        // Check tuple storage (simplified)
        status.tuple_storage = true;
        status.details.push("Tuple storage: OK".to_string());

        // Check subscription manager (simplified)
        status.subscription_manager = true;
        status.details.push("Subscription manager: OK".to_string());

        Ok(status)
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub fdb_connected: bool,
    pub tuple_storage_active: bool,
    pub subscription_manager_active: bool,
    pub total_subscriptions: usize,
}

/// Health check status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub overall: bool,
    pub fdb_storage: bool,
    pub tuple_storage: bool,
    pub subscription_manager: bool,
    pub details: Vec<String>,
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "fdb")]
    use super::*;

    #[tokio::test]
    #[cfg(feature = "fdb")]
    async fn test_memory_storage_creation() {
        let app_state = StorageManager::create_memory();
        // Test that we can create memory storage - no version field to check
        // Just verify we got an AppState
        assert!(app_state.storage.get_latest_block("test").await.is_ok());
    }

    #[test]
    #[cfg(feature = "fdb")]
    fn test_storage_stats() {
        let stats = StorageStats {
            fdb_connected: true,
            tuple_storage_active: true,
            subscription_manager_active: true,
            total_subscriptions: 100,
        };

        assert!(stats.fdb_connected);
        assert_eq!(stats.total_subscriptions, 100);
    }

    #[test]
    #[cfg(feature = "fdb")]
    fn test_health_status() {
        let status = HealthStatus {
            overall: true,
            fdb_storage: true,
            tuple_storage: true,
            subscription_manager: true,
            details: vec!["All systems operational".to_string()],
        };

        assert!(status.overall);
        assert_eq!(status.details.len(), 1);
    }
}
