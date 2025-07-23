//! FDB storage implementation with ExtendedStorageReader support

#![allow(dead_code)]

use almanac_core::{
    BlockStatus, Error as CoreError, Event, EventFilter, Result as CoreResult, StorageReader,
    StorageWriter,
};
use async_trait::async_trait;
use std::sync::Arc;

use crate::fdb::{FdbConfig, FdbStorage, TupleFdbStorage};

/// Extended FDB storage with all API methods
pub struct FdbStorageExt {
    pub base: FdbStorage,
    pub tuple_storage: Option<TupleFdbStorage>,
    #[cfg(feature = "fdb")]
    db: Option<Arc<foundationdb::Database>>,
}

impl FdbStorageExt {
    /// Create a new extended FDB storage instance
    pub async fn new(config: FdbConfig) -> CoreResult<Self> {
        let base = FdbStorage::new(config.clone()).await?;

        // Initialize tuple storage
        #[cfg(feature = "fdb")]
        let (tuple_storage, db) = {
            use foundationdb::api::FdbApiBuilder;

            // Initialize FDB API (only once per process)
            static FDB_INIT: std::sync::Once = std::sync::Once::new();
            FDB_INIT.call_once(|| {
                let api = FdbApiBuilder::default()
                    .build()
                    .expect("Failed to initialize FoundationDB API");

                // Prevent API from being stopped when dropped
                #[allow(clippy::forget_non_drop)]
                std::mem::forget(api);
            });

            // Connect to cluster
            let cluster_file = config.cluster_file.as_deref();
            let db = Arc::new(
                foundationdb::Database::new(cluster_file)
                    .map_err(|e| CoreError::storage(format!("Failed to connect to FDB: {}", e)))?,
            );

            let tuple_storage = TupleFdbStorage::new(db.clone(), config.namespace.clone())
                .await
                .map_err(|e| {
                    CoreError::storage(format!("Failed to create tuple storage: {}", e))
                })?;

            (Some(tuple_storage), Some(db))
        };

        #[cfg(not(feature = "fdb"))]
        let (tuple_storage, db) = (None, None);

        Ok(Self {
            base,
            tuple_storage,
            #[cfg(feature = "fdb")]
            db,
        })
    }

    /// Get or create tuple storage
    fn tuple_storage(&self) -> CoreResult<&TupleFdbStorage> {
        self.tuple_storage
            .as_ref()
            .ok_or_else(|| CoreError::storage("FoundationDB tuple storage not initialized"))
    }
}

// Implement StorageWriter trait
#[async_trait]
impl StorageWriter for FdbStorageExt {
    async fn store_event(&self, event: Box<dyn Event>) -> CoreResult<()> {
        self.base.store_event(event).await
    }

    async fn store_events_batch(&self, events: Vec<Box<dyn Event>>) -> CoreResult<()> {
        self.base.store_events_batch(events).await
    }

    async fn update_block_status(
        &self,
        chain: &str,
        block_number: u64,
        status: BlockStatus,
    ) -> CoreResult<()> {
        self.base
            .update_block_status(chain, block_number, status)
            .await
    }

    async fn reorg_chain(&self, chain: &str, from_block: u64) -> CoreResult<()> {
        self.base.reorg_chain(chain, from_block).await
    }

    async fn health_check(&self) -> CoreResult<()> {
        StorageWriter::health_check(&self.base).await
    }
}

// Implement StorageReader trait
#[async_trait]
impl StorageReader for FdbStorageExt {
    async fn get_events(&self, filters: Vec<EventFilter>) -> CoreResult<Vec<Box<dyn Event>>> {
        self.base.get_events(filters).await
    }

    async fn get_latest_block(&self, chain: &str) -> CoreResult<u64> {
        self.base.get_latest_block(chain).await
    }

    async fn get_latest_block_with_status(
        &self,
        chain: &str,
        status: BlockStatus,
    ) -> CoreResult<u64> {
        self.base.get_latest_block_with_status(chain, status).await
    }

    async fn get_events_with_status(
        &self,
        filters: Vec<EventFilter>,
        status: BlockStatus,
    ) -> CoreResult<Vec<Box<dyn Event>>> {
        self.base.get_events_with_status(filters, status).await
    }

    async fn get_latest_block_before(&self, chain: &str, before_block: u64) -> CoreResult<u64> {
        self.base.get_latest_block_before(chain, before_block).await
    }

    async fn health_check(&self) -> CoreResult<()> {
        StorageWriter::health_check(&self.base).await
    }
}

// Now we need to implement the ExtendedStorageReader trait from the API
// This will be done in a separate trait implementation module
