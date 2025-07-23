//! FoundationDB storage implementation
//!
//! This module provides a high-performance storage backend using FoundationDB
//! with tuple-based encoding for efficient multi-dimensional queries.

#![allow(dead_code)]

use almanac_core::{BlockStatus, Error, Event, EventFilter, Result, StorageReader, StorageWriter};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[cfg(feature = "fdb")]
use foundationdb::{
    api::FdbApiBuilder, options::StreamingMode, tuple::Subspace, Database, RangeOption, Transaction,
};

mod account_data;
mod impl_actual;
mod keyspace;
pub mod merkle_tree;
mod storage_impl;
pub mod subscription_manager;
mod tuple_encoding;
mod tuple_layer;
mod tuple_storage_impl;

// Type modules
pub mod layout;
pub mod semantic;
pub mod storage_proof;
pub mod trust;
pub mod view_registry;

pub use account_data::ChainAccountData;
pub use impl_actual::TupleFdbStorage;
pub use keyspace::TupleKeyspace;
pub use storage_impl::FdbStorageExt;
pub use subscription_manager::{
    AccountSubscriptionConfig, AccountSubscriptionManager, FdbAccountSubscriptionManager,
};
pub use tuple_layer::*;

/// FoundationDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FdbConfig {
    /// Path to cluster file (optional)
    pub cluster_file: Option<String>,
    /// Namespace for key isolation
    pub namespace: String,
    /// Connection timeout in seconds
    pub connection_timeout: u32,
    /// Maximum retry attempts
    pub max_retries: u32,
}

impl Default for FdbConfig {
    fn default() -> Self {
        Self {
            cluster_file: None,
            namespace: "almanac".to_string(),
            connection_timeout: 30,
            max_retries: 3,
        }
    }
}

/// FoundationDB storage implementation
pub struct FdbStorage {
    config: FdbConfig,
    #[cfg(feature = "fdb")]
    database: Arc<Database>,
    #[cfg(not(feature = "fdb"))]
    _phantom: std::marker::PhantomData<()>,
}

#[cfg(feature = "fdb")]
async fn scan_with_prefix(
    tx: &Transaction,
    prefix: &[u8],
    limit: usize,
) -> std::result::Result<Vec<(Vec<u8>, Vec<u8>)>, foundationdb::FdbError> {
    let subspace = Subspace::from_bytes(prefix);
    let range = subspace.range();
    let range_opt = RangeOption {
        limit: Some(limit),
        reverse: false,
        mode: StreamingMode::WantAll,
        ..RangeOption::from((range.0.as_slice(), range.1.as_slice()))
    };
    let kvs = tx.get_range(&range_opt, 1, false).await?;
    let mut results = Vec::new();
    for kv in kvs {
        results.push((kv.key().to_vec(), kv.value().to_vec()));
    }
    Ok(results)
}

impl FdbStorage {
    /// Create a new FoundationDB storage instance
    pub async fn new(config: FdbConfig) -> Result<Self> {
        #[cfg(feature = "fdb")]
        {
            use std::env;
            use tracing::{info, warn};

            // Set cluster file from config or environment
            if let Some(cluster_file) = &config.cluster_file {
                env::set_var("FDB_CLUSTER_FILE", cluster_file);
                info!("Using FDB cluster file: {}", cluster_file);
            } else if let Ok(cluster_file) = env::var("FDB_CLUSTER_FILE") {
                info!("Using FDB cluster file from environment: {}", cluster_file);
            } else {
                warn!("No FDB cluster file specified, using default");
            }

            // Initialize FoundationDB API
            let _fdb = FdbApiBuilder::default().build().map_err(|e| {
                Error::storage(format!("Failed to initialize FoundationDB API: {}", e))
            })?;

            // Create database handle using foundationdb 0.8 API
            let database = match Database::new(config.cluster_file.as_deref()) {
                Ok(db) => Arc::new(db),
                Err(e) => {
                    return Err(Error::storage(format!(
                        "Failed to open FoundationDB database: {}",
                        e
                    )));
                }
            };

            info!(
                "FoundationDB storage initialized with namespace: {}",
                config.namespace
            );

            Ok(Self { config, database })
        }

        #[cfg(not(feature = "fdb"))]
        {
            use tracing::warn;
            warn!("FoundationDB feature not enabled, using placeholder implementation");
            Ok(Self {
                config,
                _phantom: std::marker::PhantomData,
            })
        }
    }

    #[cfg(feature = "fdb")]
    /// Get the database handle
    pub fn database(&self) -> &Arc<Database> {
        &self.database
    }

    /// Get the namespace for key prefixing
    pub fn namespace(&self) -> &str {
        &self.config.namespace
    }
}

#[async_trait]
impl StorageWriter for FdbStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        #[cfg(feature = "fdb")]
        {
            use crate::fdb::keyspace::TupleKeyspace;
            use bincode;
            use foundationdb::options::TransactionOption;
            use tracing::{debug, error};

            let database = &self.database;

            // Create event tuple key: (namespace, keyspace, chain, block_number, event_id)
            let event_key = format!(
                "{}\x00{}\x00{}\x00{}\x00{}",
                self.config.namespace,
                TupleKeyspace::ContractState.id(),
                event.chain(),
                event.block_number(),
                event.id()
            );

            // Convert event to UnifiedEvent for serialization
            let unified_event = almanac_core::UnifiedEvent {
                id: event.id().to_string(),
                chain: event.chain().to_string(),
                block_number: event.block_number(),
                block_hash: event.block_hash().to_string(),
                tx_hash: event.tx_hash().to_string(),
                timestamp: event.timestamp(),
                event_type: event.event_type().to_string(),
                contract_address: if event.contract_address().is_empty() {
                    None
                } else {
                    Some(event.contract_address().to_string())
                },
                raw_data: event.raw_data().to_vec(),
                attributes: event.attributes(),
                source: almanac_core::EventSource::Chain,
            };

            // Serialize event data
            let event_data = bincode::serialize(&unified_event)
                .map_err(|e| Error::storage(format!("Failed to serialize event: {}", e)))?;

            // Block key for latest block tracking
            let block_key = format!(
                "{}\x00{}\x00{}\x00latest",
                self.config.namespace,
                TupleKeyspace::BlockMeta.id(),
                event.chain()
            );
            let block_data = event.block_number().to_be_bytes();

            // Commit transaction with retry logic
            let mut retry_count = 0;
            loop {
                let tx = database.create_trx().map_err(|e| {
                    Error::storage(format!("FDB transaction creation failed: {}", e))
                })?;

                // Set transaction timeout
                tx.set_option(TransactionOption::Timeout(
                    (self.config.connection_timeout * 1000) as i32,
                ))
                .map_err(|e| {
                    Error::storage(format!("Failed to set transaction timeout: {}", e))
                })?;

                // Store event and latest block
                tx.set(event_key.as_bytes(), &event_data);
                tx.set(block_key.as_bytes(), &block_data);

                match tx.commit().await {
                    Ok(_) => {
                        debug!(
                            "Event stored successfully: {} on chain {}",
                            event.id(),
                            event.chain()
                        );
                        return Ok(());
                    }
                    Err(e) => {
                        retry_count += 1;
                        if retry_count >= self.config.max_retries {
                            error!("Failed to store event after {} retries: {}", retry_count, e);
                            return Err(Error::storage(format!("Failed to store event: {}", e)));
                        }
                        // Exponential backoff
                        tokio::time::sleep(std::time::Duration::from_millis(
                            100 * retry_count as u64,
                        ))
                        .await;
                    }
                }
            }
        }

        #[cfg(not(feature = "fdb"))]
        {
            use tracing::warn;
            warn!("Attempted to store event but FoundationDB feature is not enabled");
            Err(Error::storage("FoundationDB feature not enabled"))
        }
    }

    async fn store_events_batch(&self, events: Vec<Box<dyn Event>>) -> Result<()> {
        // Store events individually for now
        for event in events {
            self.store_event(event).await?;
        }
        Ok(())
    }

    async fn update_block_status(
        &self,
        chain: &str,
        block_number: u64,
        status: BlockStatus,
    ) -> Result<()> {
        #[cfg(feature = "fdb")]
        {
            use crate::fdb::keyspace::TupleKeyspace;
            use bincode;
            use foundationdb::options::TransactionOption;
            use tracing::{debug, error};

            let database = &self.database;
            let tx = database
                .create_trx()
                .map_err(|e| Error::storage(format!("FDB transaction creation failed: {}", e)))?;

            tx.set_option(TransactionOption::Timeout(
                (self.config.connection_timeout * 1000) as i32,
            ))
            .map_err(|e| Error::storage(format!("Failed to set transaction timeout: {}", e)))?;

            let status_key = format!(
                "{}\x00{}\x00{}\x00{}\x00status",
                self.config.namespace,
                TupleKeyspace::BlockMeta.id(),
                chain,
                block_number
            );

            let status_data = bincode::serialize(&status)
                .map_err(|e| Error::storage(format!("Failed to serialize block status: {}", e)))?;

            tx.set(status_key.as_bytes(), &status_data);

            match tx.commit().await {
                Ok(_) => {
                    debug!(
                        "Block status updated for {}:{} to {:?}",
                        chain, block_number, status
                    );
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to update block status: {}", e);
                    Err(Error::storage(format!(
                        "Failed to update block status: {}",
                        e
                    )))
                }
            }
        }

        #[cfg(not(feature = "fdb"))]
        {
            use tracing::warn;
            warn!("Attempted to update block status but FoundationDB feature is not enabled");
            Err(Error::storage("FoundationDB feature not enabled"))
        }
    }

    async fn health_check(&self) -> Result<()> {
        #[cfg(feature = "fdb")]
        {
            let database = &self.database;
            let tx = database
                .create_trx()
                .map_err(|e| Error::storage(format!("FDB transaction creation failed: {}", e)))?;

            // Simple health check - try to read a test key
            let test_key = format!("{}\x00healthcheck", self.config.namespace);
            match tx.get(test_key.as_bytes(), false).await {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::storage(format!(
                    "FoundationDB health check failed: {}",
                    e
                ))),
            }
        }

        #[cfg(not(feature = "fdb"))]
        {
            Err(Error::storage("FoundationDB feature not enabled"))
        }
    }

    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()> {
        #[cfg(feature = "fdb")]
        {
            use crate::fdb::keyspace::TupleKeyspace;
            use foundationdb::options::TransactionOption;
            use tracing::{debug, error, warn};

            let database = &self.database;
            let tx = database
                .create_trx()
                .map_err(|e| Error::storage(format!("FDB transaction creation failed: {}", e)))?;

            tx.set_option(TransactionOption::Timeout(
                (self.config.connection_timeout * 1000) as i32,
            ))
            .map_err(|e| Error::storage(format!("Failed to set transaction timeout: {}", e)))?;

            // Find all events from the reorg point
            let key_prefix = format!(
                "{}\x00{}\x00{}\x00{}",
                self.config.namespace,
                TupleKeyspace::ContractState.id(),
                chain,
                from_block
            );

            warn!(
                "Performing chain reorganization for {} from block {}",
                chain, from_block
            );

            match scan_with_prefix(&tx, key_prefix.as_bytes(), 10000).await {
                Ok(kvs) => {
                    let mut deleted_count = 0;

                    for (key, _) in kvs {
                        tx.clear(&key);
                        deleted_count += 1;
                    }

                    // Update latest block number
                    if from_block > 0 {
                        let block_key = format!(
                            "{}\x00{}\x00{}\x00latest",
                            self.config.namespace,
                            TupleKeyspace::BlockMeta.id(),
                            chain
                        );
                        let block_data = (from_block - 1).to_be_bytes();
                        tx.set(block_key.as_bytes(), &block_data);
                    }

                    match tx.commit().await {
                        Ok(_) => {
                            debug!(
                                "Chain reorg completed for {}: deleted {} events from block {}",
                                chain, deleted_count, from_block
                            );
                            Ok(())
                        }
                        Err(e) => {
                            error!("Failed to commit reorg transaction: {}", e);
                            Err(Error::storage(format!("Failed to commit reorg: {}", e)))
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to query events for reorg: {}", e);
                    Err(Error::storage(format!(
                        "Failed to query events for reorg: {}",
                        e
                    )))
                }
            }
        }

        #[cfg(not(feature = "fdb"))]
        {
            use tracing::warn;
            warn!("Attempted to reorg chain but FoundationDB feature is not enabled");
            Err(Error::storage("FoundationDB feature not enabled"))
        }
    }
}

#[async_trait]
impl StorageReader for FdbStorage {
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        #[cfg(feature = "fdb")]
        {
            use crate::fdb::keyspace::TupleKeyspace;
            use bincode;
            use tracing::{debug, error};

            let database = &self.database;
            let tx = database
                .create_trx()
                .map_err(|e| Error::storage(format!("FDB transaction creation failed: {}", e)))?;

            let mut events: Vec<Box<dyn Event>> = Vec::new();

            for filter in filters {
                // Create range query based on filter
                let key_prefix = if let Some(chain) = filter.chain.as_ref() {
                    format!(
                        "{}\x00{}\x00{}",
                        self.config.namespace,
                        TupleKeyspace::ContractState.id(),
                        chain
                    )
                } else {
                    format!(
                        "{}\x00{}",
                        self.config.namespace,
                        TupleKeyspace::ContractState.id()
                    )
                };

                let limit = filter.limit.unwrap_or(1000);

                // Query events using FoundationDB 0.8 API
                let range_result = scan_with_prefix(&tx, key_prefix.as_bytes(), limit).await;

                match range_result {
                    Ok(kvs) => {
                        for (_, value) in kvs {
                            match bincode::deserialize::<almanac_core::UnifiedEvent>(&value) {
                                Ok(unified_event) => {
                                    events.push(Box::new(unified_event) as Box<dyn Event>);
                                }
                                Err(e) => {
                                    error!("Failed to deserialize event: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to query events: {}", e);
                        return Err(Error::storage(format!("Failed to query events: {}", e)));
                    }
                }
            }

            debug!("Retrieved {} events", events.len());
            Ok(events)
        }

        #[cfg(not(feature = "fdb"))]
        {
            use tracing::warn;
            warn!("Attempted to get events but FoundationDB feature is not enabled");
            Err(Error::storage("FoundationDB feature not enabled"))
        }
    }

    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        #[cfg(feature = "fdb")]
        {
            use crate::fdb::keyspace::TupleKeyspace;
            use tracing::{debug, error};

            let database = &self.database;
            let tx = database
                .create_trx()
                .map_err(|e| Error::storage(format!("FDB transaction creation failed: {}", e)))?;

            let block_key = format!(
                "{}\x00{}\x00{}\x00latest",
                self.config.namespace,
                TupleKeyspace::BlockMeta.id(),
                chain
            );

            match tx.get(block_key.as_bytes(), false).await {
                Ok(Some(value)) => {
                    if value.len() >= 8 {
                        let block_number =
                            u64::from_be_bytes(value[0..8].try_into().map_err(|e| {
                                Error::storage(format!("Invalid block number format: {}", e))
                            })?);
                        debug!("Latest block for chain {}: {}", chain, block_number);
                        Ok(block_number)
                    } else {
                        Err(Error::storage("Invalid block number data"))
                    }
                }
                Ok(None) => {
                    debug!("No blocks found for chain {}", chain);
                    Ok(0)
                }
                Err(e) => {
                    error!("Failed to get latest block for chain {}: {}", chain, e);
                    Err(Error::storage(format!(
                        "Failed to get latest block: {}",
                        e
                    )))
                }
            }
        }

        #[cfg(not(feature = "fdb"))]
        {
            use tracing::warn;
            warn!("Attempted to get latest block but FoundationDB feature is not enabled");
            Err(Error::storage("FoundationDB feature not enabled"))
        }
    }

    async fn get_latest_block_with_status(&self, chain: &str, _status: BlockStatus) -> Result<u64> {
        // Minimal implementation with status-specific logic
        debug!(
            "Getting latest block with status {:?} for chain {}",
            _status, chain
        );

        // For minimal implementation, delegate to get_latest_block
        // Real implementation would filter by block status
        let latest = self.get_latest_block(chain).await?;
        debug!(
            "Found latest block {} for chain {} (status filtering not implemented)",
            latest, chain
        );
        Ok(latest)
    }

    async fn get_events_with_status(
        &self,
        filters: Vec<EventFilter>,
        _status: BlockStatus,
    ) -> Result<Vec<Box<dyn Event>>> {
        // Minimal implementation with status-specific filtering
        debug!("Getting events with status {:?}", _status);

        // For minimal implementation, delegate to get_events
        // Real implementation would filter events by block status
        let events = self.get_events(filters).await?;
        debug!(
            "Found {} events (status filtering not implemented)",
            events.len()
        );
        Ok(events)
    }

    async fn get_latest_block_before(&self, chain: &str, before_block: u64) -> Result<u64> {
        self.get_latest_block(chain)
            .await
            .map(|latest| latest.min(before_block.saturating_sub(1)))
    }

    async fn health_check(&self) -> Result<()> {
        #[cfg(feature = "fdb")]
        {
            let database = &self.database;
            let tx = database
                .create_trx()
                .map_err(|e| Error::storage(format!("FDB transaction creation failed: {}", e)))?;

            // Simple health check - try to read a test key
            let test_key = format!("{}\x00healthcheck", self.config.namespace);
            match tx.get(test_key.as_bytes(), false).await {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::storage(format!(
                    "FoundationDB health check failed: {}",
                    e
                ))),
            }
        }

        #[cfg(not(feature = "fdb"))]
        {
            Err(Error::storage("FoundationDB feature not enabled"))
        }
    }
}
