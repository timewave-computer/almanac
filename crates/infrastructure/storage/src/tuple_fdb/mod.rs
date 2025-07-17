//! Tuple-based FoundationDB storage implementation
//! 
//! This module provides a clean, type-safe interface to FoundationDB using tuple encoding
//! for all key structures. Designed for semantic metadata, trust-scoped interpretations,
//! and ZK proof generation support.

pub mod semantic;
pub mod layout;
// pub mod layout_resolver; // Requires traverse_core
// pub mod layout_compatibility; // Requires traverse_core
pub mod block_meta;
pub mod contract_state;
// pub mod contract_state_extended; // Requires layout_resolver
pub mod change_tracking;
pub mod storage_proof;
pub mod view_registry;
// pub mod query_engine; // Requires layout_resolver
pub mod trust;
pub mod fdb_stub;

use async_trait::async_trait;

// Use stub implementation for now
use fdb_stub::{FdbApiBuilder, NetworkStop, Database, Transaction, Tuple, Element, TransactError, RangeOption, FdbResult};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

use almanac_infrastructure_core::{Error, Result, BlockStatus};
use almanac_infrastructure_core::types::{Event, EventFilter};
use crate::Storage;

/// Tuple keyspace enumeration for type-safe key construction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TupleKeyspace {
    /// Contract storage slots: ("contract_state", chain_id, contract_address, block_height, storage_key)
    ContractState,
    /// Block headers and consensus data: ("block_meta", chain_id, block_height)
    BlockMeta,
    /// Trust-scoped semantic interpretations: ("semantic_meta", chain_id, contract_address, storage_key, developer_id)
    SemanticMeta,
    /// Developer trust relationships: ("trust_graph", trustor, trustee, scope)
    TrustGraph,
    /// Custom view definitions: ("view_registry", view_id)
    ViewRegistry,
    /// Merkle proofs for verification: ("storage_proofs", chain_id, contract_address, block_height, storage_key)
    StorageProofs,
    /// Recursive proof coordination: ("lightwave_state", chain_id, checkpoint_type, height_or_slot)
    LightwaveState,
}

impl TupleKeyspace {
    /// Get the string prefix for this keyspace
    pub fn prefix(&self) -> &'static str {
        match self {
            TupleKeyspace::ContractState => "contract_state",
            TupleKeyspace::BlockMeta => "block_meta",
            TupleKeyspace::SemanticMeta => "semantic_meta",
            TupleKeyspace::TrustGraph => "trust_graph",
            TupleKeyspace::ViewRegistry => "view_registry",
            TupleKeyspace::StorageProofs => "storage_proofs",
            TupleKeyspace::LightwaveState => "lightwave_state",
        }
    }
}

/// Type-safe tuple key builder
#[derive(Debug, Clone)]
pub struct TupleKey {
    elements: Vec<Element>,
}

impl TupleKey {
    /// Create a new tuple key with keyspace prefix
    pub fn new(keyspace: TupleKeyspace) -> Self {
        Self {
            elements: vec![Element::Bytes(keyspace.prefix().as_bytes().to_vec())],
        }
    }
    
    /// Get the keyspace from this tuple key
    pub fn keyspace(&self) -> Option<TupleKeyspace> {
        if let Some(Element::Bytes(prefix)) = self.elements.first() {
            let prefix_str = std::str::from_utf8(prefix).ok()?;
            match prefix_str {
                "contract_state" => Some(TupleKeyspace::ContractState),
                "block_meta" => Some(TupleKeyspace::BlockMeta),
                "semantic_meta" => Some(TupleKeyspace::SemanticMeta),
                "trust_graph" => Some(TupleKeyspace::TrustGraph),
                "view_registry" => Some(TupleKeyspace::ViewRegistry),
                "storage_proofs" => Some(TupleKeyspace::StorageProofs),
                "lightwave_state" => Some(TupleKeyspace::LightwaveState),
                _ => None,
            }
        } else {
            None
        }
    }
    
    /// Get the components of this tuple key (excluding keyspace)
    pub fn components(&self) -> &[Element] {
        if self.elements.len() > 1 {
            &self.elements[1..]
        } else {
            &[]
        }
    }

    /// Add a string element to the tuple
    pub fn string<S: Into<String>>(mut self, value: S) -> Self {
        self.elements.push(Element::String(value.into()));
        self
    }

    /// Add an integer element to the tuple
    pub fn int(mut self, value: i64) -> Self {
        self.elements.push(Element::Int(value));
        self
    }

    /// Add a bytes element to the tuple
    pub fn bytes<B: Into<Vec<u8>>>(mut self, value: B) -> Self {
        self.elements.push(Element::Bytes(value.into()));
        self
    }

    /// Add an optional string element (skip if None)
    pub fn optional_string<S: Into<String>>(mut self, value: Option<S>) -> Self {
        if let Some(v) = value {
            self.elements.push(Element::String(v.into()));
        }
        self
    }
    
    /// Add a string element (alias for string method)
    pub fn append_string<S: Into<String>>(self, value: S) -> Self {
        self.string(value)
    }
    
    /// Add an integer element (alias for int method)
    pub fn append_int(self, value: i64) -> Self {
        self.int(value)
    }
    
    /// Add a bytes element (alias for bytes method)
    pub fn append_bytes<B: Into<Vec<u8>>>(self, value: B) -> Self {
        self.bytes(value)
    }

    /// Pack the tuple into a binary key
    pub fn pack(&self) -> Vec<u8> {
        let mut tuple = Tuple::new();
        for element in &self.elements {
            match element {
                Element::String(s) => tuple.push_string(s.clone()),
                Element::Int(i) => tuple.push_int(*i),
                Element::Bytes(b) => tuple.push_bytes(b.clone()),
            }
        }
        tuple.pack()
    }
    
    /// Build the tuple into a binary key (alias for pack)
    pub fn build(&self) -> Vec<u8> {
        self.pack()
    }

    /// Create a range key for prefix scanning
    pub fn range_prefix(&self) -> (Vec<u8>, Vec<u8>) {
        let start = self.pack();
        let mut end = start.clone();
        
        // Create end key by incrementing the last byte
        if let Some(last_byte) = end.last_mut() {
            if *last_byte < 255 {
                *last_byte += 1;
            } else {
                // Handle overflow by extending the key
                end.push(0);
            }
        }
        
        (start, end)
    }
}

/// Configuration for tuple-based FoundationDB storage
#[derive(Debug, Clone)]
pub struct TupleFdbConfig {
    /// Optional cluster file path
    pub cluster_file: Option<String>,
    /// Namespace for all keys
    pub namespace: String,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Transaction retry limit
    pub max_retries: u32,
}

impl Default for TupleFdbConfig {
    fn default() -> Self {
        Self {
            cluster_file: None,
            namespace: "almanac".to_string(),
            connection_timeout: 30,
            max_retries: 3,
        }
    }
}

/// Main tuple-based FoundationDB storage implementation
pub struct TupleFdbStorage {
    /// FoundationDB database handle
    db: Database,
    /// Configuration
    config: TupleFdbConfig,
    /// Network stop handle for cleanup
    _network: Arc<RwLock<Option<NetworkStop>>>,
}

impl TupleFdbStorage {
    /// Create a new tuple-based FoundationDB storage
    pub async fn new(config: TupleFdbConfig) -> Result<Self> {
        let network_builder = FdbApiBuilder::default()
            .build()
            .map_err(|e| Error::Storage(format!("Failed to initialize FDB API: {}", e)))?;

        let network = Arc::new(RwLock::new(Some(unsafe { network_builder.boot() })));

        let db = if let Some(cluster_file) = &config.cluster_file {
            Database::from_cluster_file(cluster_file)
                .await
                .map_err(|e| Error::Storage(format!("Failed to connect to FDB cluster: {}", e)))?
        } else {
            Database::default()
                .map_err(|e| Error::Storage(format!("Failed to connect to default FDB cluster: {}", e)))?
        };

        info!("Connected to FoundationDB cluster with tuple encoding");

        Ok(Self {
            db,
            config,
            _network: network,
        })
    }

    /// Execute a transaction with retry logic
    pub async fn transact<F, R>(&self, f: F) -> Result<R>
    where
        F: Fn(Transaction) -> Pin<Box<dyn Future<Output = FdbResult<R>> + Send>> + Send + Sync + 'static,
        R: Send + 'static,
    {
        let mut retries = 0;
        loop {
            match self.db.run(|trx| f(trx)).await {
                Ok(result) => return Ok(result),
                Err(e) if retries < self.config.max_retries => {
                    retries += 1;
                    warn!("Transaction failed, retrying {}/{}: {}", retries, self.config.max_retries, e);
                    // Exponential backoff
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * (1 << retries))).await;
                }
                Err(e) => {
                    return Err(Error::Storage(format!("Transaction failed after {} retries: {}", self.config.max_retries, e)));
                }
            }
        }
    }

    /// Add namespace prefix to a tuple key
    fn namespace_key(&self, key: &[u8]) -> Vec<u8> {
        let mut namespaced = Vec::with_capacity(self.config.namespace.len() + key.len() + 1);
        namespaced.push(b'/');
        namespaced.extend_from_slice(self.config.namespace.as_bytes());
        namespaced.push(b'/');
        namespaced.extend_from_slice(key);
        namespaced
    }

    /// Get a single value by tuple key
    pub async fn get(&self, key: TupleKey) -> Result<Option<Vec<u8>>> {
        let namespaced_key = self.namespace_key(&key.pack());
        
        self.transact(move |trx| {
            let key_copy = namespaced_key.clone();
            Box::pin(async move {
                trx.get(&key_copy, false).await
            })
        }).await
    }

    /// Set a single value by tuple key
    pub async fn set(&self, key: TupleKey, value: Vec<u8>) -> Result<()> {
        let namespaced_key = self.namespace_key(&key.pack());
        
        self.transact(move |mut trx| {
            trx.set(&namespaced_key, &value);
            Ok(())
        }).await
    }

    /// Delete a single value by tuple key
    pub async fn delete(&self, key: TupleKey) -> Result<()> {
        let namespaced_key = self.namespace_key(&key.pack());
        
        self.transact(move |mut trx| {
            Box::pin(async move {
                trx.clear(&namespaced_key);
                Ok(())
            })
        }).await
    }

    /// Get a range of key-value pairs by tuple prefix
    pub async fn range(&self, prefix: TupleKey, limit: Option<usize>) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let (start, end) = prefix.range_prefix();
        let namespaced_start = self.namespace_key(&start);
        let namespaced_end = self.namespace_key(&end);
        
        self.transact(move |trx| {
            Box::pin(async move {
                #[cfg(all(feature = "fdb", not(use_fdb_stub)))]
                let range_opt = RangeOption {
                    begin: KeySelector::first_greater_or_equal(&namespaced_start),
                    end: KeySelector::first_greater_or_equal(&namespaced_end),
                    mode: StreamingMode::Small,
                    limit: limit.map(|l| l as i32),
                    reverse: false,
                };
                
                #[cfg(any(not(feature = "fdb"), use_fdb_stub))]
                let range_opt = RangeOption {
                    begin: fdb_stub::KeySelector::first_greater_or_equal(&namespaced_start),
                    end: fdb_stub::KeySelector::first_greater_or_equal(&namespaced_end),
                    mode: fdb_stub::StreamingMode::Small,
                    limit: limit.map(|l| l as i32),
                    reverse: false,
                };

                let mut results = Vec::new();
                let range_stream = trx.get_range(&range_opt, 1, false);
                
                #[cfg(all(feature = "fdb", not(use_fdb_stub)))]
                let kvs: Vec<KeyValue> = range_stream.try_collect().await?;
                
                #[cfg(any(not(feature = "fdb"), use_fdb_stub))]
                let kvs: Vec<fdb_stub::KeyValue> = range_stream.try_collect().await?;
                
                for kv in kvs {
                    results.push((kv.key().to_vec(), kv.value().to_vec()));
                }
                
                Ok(results)
            })
        }).await
    }

    /// Get range of keys in reverse order
    pub async fn range_reverse(&self, prefix: TupleKey, limit: Option<usize>) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let (start, end) = prefix.range_prefix();
        let namespaced_start = self.namespace_key(&start);
        let namespaced_end = self.namespace_key(&end);
        
        self.transact(move |trx| {
            Box::pin(async move {
                #[cfg(all(feature = "fdb", not(use_fdb_stub)))]
                let range_opt = RangeOption {
                    begin: KeySelector::first_greater_or_equal(&namespaced_start),
                    end: KeySelector::first_greater_or_equal(&namespaced_end),
                    mode: StreamingMode::Small,
                    limit: limit.map(|l| l as i32),
                    reverse: true, // Key difference: reverse order
                };
                
                #[cfg(any(not(feature = "fdb"), use_fdb_stub))]
                let range_opt = RangeOption {
                    begin: fdb_stub::KeySelector::first_greater_or_equal(&namespaced_start),
                    end: fdb_stub::KeySelector::first_greater_or_equal(&namespaced_end),
                    mode: fdb_stub::StreamingMode::Small,
                    limit: limit.map(|l| l as i32),
                    reverse: true, // Key difference: reverse order
                };

                let mut results = Vec::new();
                let range_stream = trx.get_range(&range_opt, 1, false);
                
                #[cfg(all(feature = "fdb", not(use_fdb_stub)))]
                let kvs: Vec<KeyValue> = range_stream.try_collect().await?;
                
                #[cfg(any(not(feature = "fdb"), use_fdb_stub))]
                let kvs: Vec<fdb_stub::KeyValue> = range_stream.try_collect().await?;
                
                for kv in kvs {
                    results.push((kv.key().to_vec(), kv.value().to_vec()));
                }
                
                Ok(results)
            })
        }).await
    }

    /// Batch set multiple key-value pairs atomically
    pub async fn batch_set(&self, kvs: Vec<(TupleKey, Vec<u8>)>) -> Result<()> {
        let namespaced_kvs: Vec<(Vec<u8>, Vec<u8>)> = kvs.into_iter()
            .map(|(key, value)| (self.namespace_key(&key.pack()), value))
            .collect();
        
        self.transact(move |mut trx| {
            Box::pin(async move {
                for (key, value) in &namespaced_kvs {
                    trx.set(key, value);
                }
                Ok(())
            })
        }).await
    }
    
    /// Delete a raw key (already namespaced)
    pub async fn delete_raw(&self, key: Vec<u8>) -> Result<()> {
        self.transact(move |mut trx| {
            Box::pin(async move {
                trx.clear(&key);
                Ok(())
            })
        }).await
    }
    
    /// Batch delete multiple keys atomically
    pub async fn batch_delete(&self, keys: Vec<TupleKey>) -> Result<()> {
        let namespaced_keys: Vec<Vec<u8>> = keys.into_iter()
            .map(|key| self.namespace_key(&key.pack()))
            .collect();
        
        self.transact(move |mut trx| {
            Box::pin(async move {
                for key in &namespaced_keys {
                    trx.clear(key);
                }
                Ok(())
            })
        }).await
    }

    /// Clear all data in a keyspace (dangerous - use with caution)
    pub async fn clear_keyspace(&self, keyspace: TupleKeyspace) -> Result<()> {
        let prefix = TupleKey::new(keyspace);
        let (start, end) = prefix.range_prefix();
        let namespaced_start = self.namespace_key(&start);
        let namespaced_end = self.namespace_key(&end);
        
        self.transact(move |mut trx| {
            trx.clear_range(&namespaced_start, &namespaced_end);
            Ok(())
        }).await
    }
}

/// Utility functions for common tuple key patterns
pub mod keys {
    use super::*;

    /// Create a contract state key
    pub fn contract_state(chain_id: &str, contract_address: &str, block_height: u64, storage_key: &[u8]) -> TupleKey {
        TupleKey::new(TupleKeyspace::ContractState)
            .string(chain_id)
            .string(contract_address)
            .int(block_height as i64)
            .bytes(storage_key)
    }

    /// Create a block metadata key
    pub fn block_meta(chain_id: &str, block_height: u64) -> TupleKey {
        TupleKey::new(TupleKeyspace::BlockMeta)
            .string(chain_id)
            .int(block_height as i64)
    }

    /// Create a semantic metadata key
    pub fn semantic_meta(chain_id: &str, contract_address: &str, storage_key: &[u8], developer_id: &str) -> TupleKey {
        TupleKey::new(TupleKeyspace::SemanticMeta)
            .string(chain_id)
            .string(contract_address)
            .bytes(storage_key)
            .string(developer_id)
    }

    /// Create a trust relationship key
    pub fn trust_graph(trustor: &str, trustee: &str, scope: &str) -> TupleKey {
        TupleKey::new(TupleKeyspace::TrustGraph)
            .string(trustor)
            .string(trustee)
            .string(scope)
    }

    /// Create a view registry key
    pub fn view_registry(view_id: &str) -> TupleKey {
        TupleKey::new(TupleKeyspace::ViewRegistry)
            .string(view_id)
    }

    /// Create a storage proof key
    pub fn storage_proof(chain_id: &str, contract_address: &str, block_height: u64, storage_key: &[u8]) -> TupleKey {
        TupleKey::new(TupleKeyspace::StorageProofs)
            .string(chain_id)
            .string(contract_address)
            .int(block_height as i64)
            .bytes(storage_key)
    }

    /// Create a lightwave state key
    pub fn lightwave_state(chain_id: &str, checkpoint_type: &str, height_or_slot: u64) -> TupleKey {
        TupleKey::new(TupleKeyspace::LightwaveState)
            .string(chain_id)
            .string(checkpoint_type)
            .int(height_or_slot as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuple_key_construction() {
        let key = TupleKey::new(TupleKeyspace::ContractState)
            .string("ethereum")
            .string("0x123...")
            .int(12345)
            .bytes(&[0x01, 0x02, 0x03]);
        
        let packed = key.pack();
        assert!(!packed.is_empty());
        
        // Test range prefix
        let (start, end) = key.range_prefix();
        assert!(end > start);
    }

    #[test]
    fn test_keyspace_prefixes() {
        assert_eq!(TupleKeyspace::ContractState.prefix(), "contract_state");
        assert_eq!(TupleKeyspace::SemanticMeta.prefix(), "semantic_meta");
        assert_eq!(TupleKeyspace::TrustGraph.prefix(), "trust_graph");
    }

    #[test]
    fn test_key_helpers() {
        let state_key = keys::contract_state("ethereum", "0x123", 12345, &[0x01]);
        let packed = state_key.pack();
        assert!(!packed.is_empty());
        
        let meta_key = keys::semantic_meta("ethereum", "0x123", &[0x01], "developer_alice");
        let packed_meta = meta_key.pack();
        assert!(!packed_meta.is_empty());
                 assert_ne!(packed, packed_meta);
     }
}

// Storage trait implementation for TupleFdbStorage
#[async_trait]
impl Storage for TupleFdbStorage {
    /// Store an event in the tuple-based storage
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        // Create contract state key for the event
        let storage_key = format!("event_{}", event.id()).into_bytes();
        let state_key = keys::contract_state(
            event.chain(),
            &format!("event_contract_{}", event.chain()),
            event.block_number(),
            &storage_key,
        );
        
        // Serialize event data
        let event_data = serde_json::json!({
            "id": event.id(),
            "chain": event.chain(),
            "block_number": event.block_number(),
            "block_hash": event.block_hash(),
            "tx_hash": event.tx_hash(),
            "timestamp": event.timestamp().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            "event_type": event.event_type(),
            "raw_data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, event.raw_data()),
        });
        
        let value = serde_json::to_vec(&event_data)
            .map_err(|e| Error::Serialization(format!("Failed to serialize event: {}", e)))?;
        
        self.set(state_key, value).await?;
        
        debug!("Stored event {} for chain {} at block {}", event.id(), event.chain(), event.block_number());
        Ok(())
    }
    
    /// Get events by filters (basic implementation)
    async fn get_events(&self, _filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        // TODO: Implement proper event filtering with tuple keys
        // For now, return empty vector as this is a foundational implementation
        warn!("Event filtering not yet implemented in tuple FDB storage");
        Ok(Vec::new())
    }
    
    /// Get the latest block height for a chain
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        // Use a special key to track the latest block for each chain
        let latest_key = TupleKey::new(TupleKeyspace::BlockMeta)
            .string(chain)
            .string("latest");
        
        match self.get(latest_key).await? {
            Some(data) => {
                let block_str = String::from_utf8(data)
                    .map_err(|e| Error::Serialization(format!("Invalid block number format: {}", e)))?;
                block_str.parse::<u64>()
                    .map_err(|e| Error::Serialization(format!("Failed to parse block number: {}", e)))
            }
            None => Ok(0), // No blocks stored yet
        }
    }
    
    /// Get the latest block with a specific status for a chain
    async fn get_latest_block_with_status(&self, chain: &str, _status: BlockStatus) -> Result<u64> {
        // For now, delegate to get_latest_block - status filtering will be implemented later
        self.get_latest_block(chain).await
    }
    
    /// Update block status
    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        let block_key = keys::block_meta(chain, block_number);
        
        let block_data = serde_json::json!({
            "chain": chain,
            "block_number": block_number,
            "status": format!("{:?}", status),
            "updated_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        });
        
        let value = serde_json::to_vec(&block_data)
            .map_err(|e| Error::Serialization(format!("Failed to serialize block data: {}", e)))?;
        
        self.set(block_key, value).await?;
        
        // Update latest block tracker if this is the highest block
        let current_latest = self.get_latest_block(chain).await?;
        if block_number > current_latest {
            let latest_key = TupleKey::new(TupleKeyspace::BlockMeta)
                .string(chain)
                .string("latest");
            self.set(latest_key, block_number.to_string().into_bytes()).await?;
        }
        
        debug!("Updated block status: chain={}, block={}, status={:?}", chain, block_number, status);
        Ok(())
    }
    
    /// Get events with specific block status (basic implementation)
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, _status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        // For now, delegate to get_events - status filtering will be implemented later
        self.get_events(filters).await
    }
    
    /// Handle chain reorganization from a specific block
    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()> {
        warn!("Chain reorg handling not yet fully implemented for chain {} from block {}", chain, from_block);
        
        // Clear all blocks and events from the reorg point
        // This is a simplified implementation - a full implementation would be more sophisticated
        let current_latest = self.get_latest_block(chain).await?;
        
        // Update latest block to the reorg point
        if from_block < current_latest {
            let latest_key = TupleKey::new(TupleKeyspace::BlockMeta)
                .string(chain)
                .string("latest");
            self.set(latest_key, from_block.to_string().into_bytes()).await?;
        }
        
        info!("Handled chain reorg for {} from block {}", chain, from_block);
        Ok(())
    }
    
    /// Get the latest block before a specific block
    async fn get_latest_block_before(&self, chain: &str, before_block: u64) -> Result<u64> {
        let latest = self.get_latest_block(chain).await?;
        if latest < before_block {
            Ok(latest)
        } else {
            Ok(before_block.saturating_sub(1))
        }
    }
    
    // Valence Account methods
    
    async fn store_valence_account_instantiation(
        &self,
        _account_info: crate::ValenceAccountInfo,
        _initial_libraries: Vec<crate::ValenceAccountLibrary>,
    ) -> Result<()> {
        warn!("Valence account instantiation not yet implemented in tuple FDB storage");
        Ok(())
    }
    
    async fn store_valence_library_approval(&self, _request: crate::ValenceLibraryApprovalRequest) -> Result<()> {
        warn!("Valence library approval not yet implemented in tuple FDB storage");
        Ok(())
    }
    
    async fn store_valence_library_removal(&self, _request: crate::ValenceLibraryRemovalRequest) -> Result<()> {
        warn!("Valence library removal not yet implemented in tuple FDB storage");
        Ok(())
    }
    
    async fn store_valence_ownership_update(&self, _request: crate::ValenceOwnershipUpdateRequest) -> Result<()> {
        warn!("Valence ownership update not yet implemented in tuple FDB storage");
        Ok(())
    }
    
    async fn store_valence_execution(&self, _execution_info: crate::ValenceAccountExecution) -> Result<()> {
        warn!("Valence execution not yet implemented in tuple FDB storage");
        Ok(())
    }
    
    async fn get_valence_account_state(&self, _account_id: &str) -> Result<Option<crate::ValenceAccountState>> {
        warn!("Valence account state retrieval not yet implemented in tuple FDB storage");
        Ok(None)
    }
    
    async fn set_valence_account_state(&self, _account_id: &str, _state: &crate::ValenceAccountState) -> Result<()> {
        warn!("Set valence account state not yet implemented in tuple FDB storage");
        Ok(())
    }
    
    async fn delete_valence_account_state(&self, _account_id: &str) -> Result<()> {
        warn!("Delete valence account state not yet implemented in tuple FDB storage");
        Ok(())
    }
    
    async fn set_historical_valence_account_state(
        &self,
        _account_id: &str,
        _block_number: u64,
        _state: &crate::ValenceAccountState,
    ) -> Result<()> {
        warn!("Set historical valence account state not yet implemented in tuple FDB storage");
        Ok(())
    }
    
    async fn get_historical_valence_account_state(
        &self,
        _account_id: &str,
        _block_number: u64,
    ) -> Result<Option<crate::ValenceAccountState>> {
        warn!("Get historical valence account state not yet implemented in tuple FDB storage");
        Ok(None)
    }
    
    async fn delete_historical_valence_account_state(
        &self,
        _account_id: &str,
        _block_number: u64,
    ) -> Result<()> {
        warn!("Delete historical valence account state not yet implemented in tuple FDB storage");
        Ok(())
    }
    
    async fn set_latest_historical_valence_block(
        &self,
        _account_id: &str,
        _block_number: u64,
    ) -> Result<()> {
        warn!("Set latest historical valence block not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn get_latest_historical_valence_block(&self, _account_id: &str) -> Result<Option<u64>> {
        warn!("Get latest historical valence block not yet implemented in tuple FDB storage");
        Ok(None)
    }

    async fn delete_latest_historical_valence_block(&self, _account_id: &str) -> Result<()> {
        warn!("Delete latest historical valence block not yet implemented in tuple FDB storage");
        Ok(())
    }

    // Valence Processor data models

    async fn store_valence_processor_instantiation(
        &self,
        _processor_info: crate::ValenceProcessorInfo,
    ) -> Result<()> {
        warn!("Valence processor instantiation not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn store_valence_processor_config_update(
        &self,
        _request: crate::ValenceProcessorConfigUpdateRequest,
    ) -> Result<()> {
        warn!("Valence processor config update not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn store_valence_processor_message(
        &self,
        _message: crate::ValenceProcessorMessage,
    ) -> Result<()> {
        warn!("Valence processor message storage not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn update_valence_processor_message_status(
        &self,
        _request: crate::ValenceProcessorMessageStatusRequest,
    ) -> Result<()> {
        warn!("Valence processor message status update not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn get_valence_processor_state(&self, _processor_id: &str) -> Result<Option<crate::ValenceProcessorState>> {
        warn!("Valence processor state retrieval not yet implemented in tuple FDB storage");
        Ok(None)
    }

    async fn set_valence_processor_state(&self, _processor_id: &str, _state: &crate::ValenceProcessorState) -> Result<()> {
        warn!("Set valence processor state not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn set_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
        _state: &crate::ValenceProcessorState,
    ) -> Result<()> {
        warn!("Set historical valence processor state not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn get_historical_valence_processor_state(
        &self,
        _processor_id: &str,
        _block_number: u64,
    ) -> Result<Option<crate::ValenceProcessorState>> {
        warn!("Get historical valence processor state not yet implemented in tuple FDB storage");
        Ok(None)
    }

    // Valence Authorization data models

    async fn store_valence_authorization_instantiation(
        &self,
        _auth_info: crate::ValenceAuthorizationInfo,
        _initial_policy: Option<crate::ValenceAuthorizationPolicy>,
    ) -> Result<()> {
        warn!("Valence authorization instantiation not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn store_valence_authorization_policy(
        &self,
        _policy: crate::ValenceAuthorizationPolicy,
    ) -> Result<()> {
        warn!("Valence authorization policy storage not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn update_active_authorization_policy(
        &self,
        _request: crate::ValenceAuthorizationPolicyUpdateRequest,
    ) -> Result<()> {
        warn!("Update active authorization policy not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn store_valence_authorization_grant(
        &self,
        _grant: crate::ValenceAuthorizationGrant,
    ) -> Result<()> {
        warn!("Valence authorization grant not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn revoke_valence_authorization_grant(
        &self,
        _request: crate::ValenceAuthorizationGrantRevocationRequest,
    ) -> Result<()> {
        warn!("Revoke valence authorization grant not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn store_valence_authorization_request(
        &self,
        _request: crate::ValenceAuthorizationRequest,
    ) -> Result<()> {
        warn!("Valence authorization request storage not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn update_valence_authorization_request_decision(
        &self,
        _request: crate::ValenceAuthorizationRequestDecisionRequest,
    ) -> Result<()> {
        warn!("Update valence authorization request decision not yet implemented in tuple FDB storage");
        Ok(())
    }

    // Valence Library data models

    async fn store_valence_library_instantiation(
        &self,
        _library_info: crate::ValenceLibraryInfo,
        _initial_version: Option<crate::ValenceLibraryVersion>,
    ) -> Result<()> {
        warn!("Valence library instantiation not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn store_valence_library_version(
        &self,
        _version: crate::ValenceLibraryVersion,
    ) -> Result<()> {
        warn!("Valence library version storage not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn update_active_library_version(
        &self,
        _request: crate::ValenceLibraryVersionUpdateRequest,
    ) -> Result<()> {
        warn!("Update active library version not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn store_valence_library_usage(
        &self,
        _usage: crate::ValenceLibraryUsage,
    ) -> Result<()> {
        warn!("Valence library usage storage not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn revoke_valence_library_approval(
        &self,
        _request: crate::ValenceLibraryApprovalRevocationRequest,
    ) -> Result<()> {
        warn!("Revoke valence library approval not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn get_valence_library_state(&self, _library_id: &str) -> Result<Option<crate::ValenceLibraryState>> {
        warn!("Valence library state retrieval not yet implemented in tuple FDB storage");
        Ok(None)
    }

    async fn set_valence_library_state(&self, _library_id: &str, _state: &crate::ValenceLibraryState) -> Result<()> {
        warn!("Set valence library state not yet implemented in tuple FDB storage");
        Ok(())
    }

    async fn get_valence_library_versions(&self, _library_id: &str) -> Result<Vec<crate::ValenceLibraryVersion>> {
        warn!("Get valence library versions not yet implemented in tuple FDB storage");
        Ok(Vec::new())
    }

    async fn get_valence_library_approvals(&self, _library_id: &str) -> Result<Vec<crate::ValenceLibraryApproval>> {
        warn!("Get valence library approvals not yet implemented in tuple FDB storage");
        Ok(Vec::new())
    }

    async fn get_valence_libraries_for_account(&self, _account_id: &str) -> Result<Vec<crate::ValenceLibraryApproval>> {
        warn!("Get valence libraries for account not yet implemented in tuple FDB storage");
        Ok(Vec::new())
    }

    async fn get_valence_library_usage_history(
        &self,
        _library_id: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<crate::ValenceLibraryUsage>> {
        warn!("Get valence library usage history not yet implemented in tuple FDB storage");
        Ok(Vec::new())
    }
    
    async fn set_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<()> {
        let key = TupleKey::new(TupleKeyspace::ContractState)
            .string(chain)
            .string("processor_state")
            .int(block_number as i64);
        
        self.set(key, state.as_bytes().to_vec()).await
    }
    
    async fn get_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>> {
        let key = TupleKey::new(TupleKeyspace::ContractState)
            .string(chain)
            .string("processor_state")
            .int(block_number as i64);
        
        match self.get(key).await? {
            Some(data) => Ok(Some(String::from_utf8(data)
                .map_err(|e| Error::Serialization(format!("Invalid processor state format: {}", e)))?)),
            None => Ok(None),
        }
    }
    
    async fn set_historical_processor_state(&self, chain: &str, block_number: u64, state: &str) -> Result<()> {
        let key = TupleKey::new(TupleKeyspace::ContractState)
            .string(chain)
            .string("historical_processor_state")
            .int(block_number as i64);
        
        self.set(key, state.as_bytes().to_vec()).await
    }
    
    async fn get_historical_processor_state(&self, chain: &str, block_number: u64) -> Result<Option<String>> {
        let key = TupleKey::new(TupleKeyspace::ContractState)
            .string(chain)
            .string("historical_processor_state")
            .int(block_number as i64);
        
        match self.get(key).await? {
            Some(data) => Ok(Some(String::from_utf8(data)
                .map_err(|e| Error::Serialization(format!("Invalid historical processor state format: {}", e)))?)),
            None => Ok(None),
        }
    }
}

// Semantic metadata operations for TupleFdbStorage
impl TupleFdbStorage {
    /// Store semantic metadata for a storage slot
    pub async fn store_semantic_metadata(
        &self,
        key: semantic::SemanticMetadataKey,
        value: semantic::SemanticMetadataValue,
    ) -> Result<()> {
        // Validate the metadata
        value.validate()?;
        
        let tuple_key = key.to_tuple_key();
        let serialized = serde_json::to_vec(&value)
            .map_err(|e| Error::Serialization(format!("Failed to serialize semantic metadata: {}", e)))?;
        
        self.set(tuple_key, serialized).await?;
        
        debug!("Stored semantic metadata for {}/{} by {}", 
               key.chain_id, key.contract_address, key.developer_id);
        Ok(())
    }
    
    /// Get semantic metadata for a storage slot with trust policy
    pub async fn get_semantic_metadata(
        &self,
        chain_id: &str,
        contract_address: &str,
        storage_key: &[u8; 32],
        requested_by: &str,
        trust_policy: &semantic::TrustPolicy,
    ) -> Result<Vec<(String, semantic::SemanticMetadataValue)>> {
        // Get trust graph for the requesting developer
        let trust_graph = self.get_trust_graph(requested_by).await?;
        
        // Get all semantic metadata for this storage slot
        let prefix = TupleKey::new(TupleKeyspace::SemanticMeta)
            .string(chain_id)
            .string(contract_address)
            .bytes(storage_key);
        
        let results = self.range(prefix, None).await?;
        let mut semantics = Vec::new();
        
        for (key_bytes, value_bytes) in results {
            // Parse the key to extract developer_id
            // This is a simplified approach - in production, we'd parse the tuple properly
            if let Ok(metadata_value) = serde_json::from_slice::<semantic::SemanticMetadataValue>(&value_bytes) {
                // Extract developer_id from key (simplified - assumes it's at the end)
                if let Ok(key_str) = String::from_utf8(key_bytes) {
                    if let Some(developer_id) = key_str.split('/').last() {
                        // Check trust policy
                        if trust_policy.is_trusted(developer_id, &trust_graph) {
                            semantics.push((developer_id.to_string(), metadata_value));
                        }
                    }
                }
            }
        }
        
        // Sort by confidence level (highest first)
        semantics.sort_by(|a, b| b.1.confidence_level.cmp(&a.1.confidence_level));
        
        Ok(semantics)
    }
    
    /// Store a developer trust relationship
    pub async fn store_trust_relationship(
        &self,
        key: semantic::DeveloperTrustKey,
        value: semantic::DeveloperTrustValue,
    ) -> Result<()> {
        let tuple_key = key.to_tuple_key();
        let serialized = serde_json::to_vec(&value)
            .map_err(|e| Error::Serialization(format!("Failed to serialize trust relationship: {}", e)))?;
        
        self.set(tuple_key, serialized).await?;
        
        debug!("Stored trust relationship: {} trusts {} with level {:?}", 
               key.trustor, key.trustee, value.trust_level);
        Ok(())
    }
    
    /// Get trust graph for a developer
    pub async fn get_trust_graph(&self, developer_id: &str) -> Result<HashMap<String, semantic::TrustLevel>> {
        let prefix = TupleKey::new(TupleKeyspace::TrustGraph)
            .string(developer_id);
        
        let results = self.range(prefix, None).await?;
        let mut trust_graph = HashMap::new();
        
        for (key_bytes, value_bytes) in results {
            if let Ok(trust_value) = serde_json::from_slice::<semantic::DeveloperTrustValue>(&value_bytes) {
                if trust_value.is_valid() {
                    // Extract trustee from key (simplified parsing)
                    if let Ok(key_str) = String::from_utf8(key_bytes) {
                        let parts: Vec<&str> = key_str.split('/').collect();
                        if parts.len() >= 3 {
                            let trustee = parts[2].to_string();
                            trust_graph.insert(trustee, trust_value.trust_level);
                        }
                    }
                }
            }
        }
        
        Ok(trust_graph)
    }
    
    /// Detect semantic conflicts for a storage slot
    pub async fn detect_semantic_conflicts(
        &self,
        chain_id: &str,
        contract_address: &str,
        storage_key: &[u8; 32],
    ) -> Result<Option<semantic::SemanticConflict>> {
        // Get all interpretations for this slot
        let prefix = TupleKey::new(TupleKeyspace::SemanticMeta)
            .string(chain_id)
            .string(contract_address)
            .bytes(storage_key);
        
        let results = self.range(prefix, None).await?;
        let mut interpretations = Vec::new();
        
        for (key_bytes, value_bytes) in results {
            if let Ok(metadata_value) = serde_json::from_slice::<semantic::SemanticMetadataValue>(&value_bytes) {
                if let Ok(key_str) = String::from_utf8(key_bytes) {
                    if let Some(developer_id) = key_str.split('/').last() {
                        interpretations.push((developer_id.to_string(), metadata_value));
                    }
                }
            }
        }
        
        // Check for conflicts (different zero_semantics)
        if interpretations.len() > 1 {
            let first_semantics = interpretations[0].1.zero_semantics;
            let has_conflict = interpretations.iter()
                .any(|(_, meta)| meta.zero_semantics != first_semantics);
            
            if has_conflict {
                return Ok(Some(semantic::SemanticConflict {
                    chain_id: chain_id.to_string(),
                    contract_address: contract_address.to_string(),
                    storage_key: *storage_key,
                    interpretations,
                    detected_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    resolution: None,
                }));
            }
        }
        
        Ok(None)
    }
    
    /// Resolve a semantic conflict
    pub async fn resolve_semantic_conflict(
        &self,
        chain_id: &str,
        contract_address: &str,
        storage_key: &[u8; 32],
        chosen_developer: &str,
        reason: &str,
        resolved_by: &str,
    ) -> Result<()> {
        // Store the resolution
        let resolution_key = TupleKey::new(TupleKeyspace::SemanticMeta)
            .string(chain_id)
            .string(contract_address)
            .bytes(storage_key)
            .string("_resolution");
        
        let resolution = semantic::ConflictResolution {
            chosen_developer: chosen_developer.to_string(),
            reason: reason.to_string(),
            resolved_by: resolved_by.to_string(),
            resolved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        let serialized = serde_json::to_vec(&resolution)
            .map_err(|e| Error::Serialization(format!("Failed to serialize conflict resolution: {}", e)))?;
        
        self.set(resolution_key, serialized).await?;
        
        info!("Resolved semantic conflict for {}/{} - chose {} for reason: {}", 
              chain_id, contract_address, chosen_developer, reason);
        Ok(())
    }

    /// Store a contract layout commitment from developer
    pub async fn store_contract_layout(
        &self,
        key: layout::ContractLayoutKey,
        value: layout::ContractLayoutValue,
    ) -> Result<()> {
        // Validate the layout
        value.validate()?;
        
        let tuple_key = key.to_tuple_key();
        let serialized = serde_json::to_vec(&value)
            .map_err(|e| Error::Serialization(format!("Failed to serialize layout: {}", e)))?;
        
        self.set(tuple_key, serialized).await?;
        
        info!("Stored contract layout for {}/{} by {}", 
              key.chain_id, key.contract_address, value.provided_by);
        Ok(())
    }

    /// Get contract layout with trust context
    pub async fn get_contract_layout(
        &self,
        chain_id: &str,
        contract_address: &str,
        version: Option<&str>,
        requested_by: &str,
        trust_policy: &semantic::TrustPolicy,
    ) -> Result<Option<(layout::ContractLayoutValue, bool)>> {
        // Get trust graph for the requesting developer
        let trust_graph = self.get_trust_graph(requested_by).await?;
        
        let key = layout::ContractLayoutKey::new(
            chain_id.to_string(),
            contract_address.to_string(),
            version.map(|v| v.to_string()),
        );
        
        let tuple_key = key.to_tuple_key();
        match self.get(tuple_key).await? {
            Some(data) => {
                let layout: layout::ContractLayoutValue = serde_json::from_slice(&data)
                    .map_err(|e| Error::Serialization(format!("Failed to parse layout: {}", e)))?;
                
                // Check if developer is trusted according to policy
                let is_trusted = trust_policy.is_trusted(&layout.provided_by, &trust_graph);
                
                Ok(Some((layout, is_trusted)))
            }
            None => Ok(None),
        }
    }

    /// Detect layout conflicts for a contract
    pub async fn detect_layout_conflicts(
        &self,
        chain_id: &str,
        contract_address: &str,
    ) -> Result<Option<layout::LayoutConflict>> {
        // Get all layouts for this contract (all versions)
        let prefix = TupleKey::new(TupleKeyspace::ViewRegistry)
            .string("layout")
            .string(chain_id)
            .string(contract_address);
        
        let results = self.range(prefix, None).await?;
        let mut layouts = Vec::new();
        
        for (key_bytes, value_bytes) in results {
            if let Ok(layout_value) = serde_json::from_slice::<layout::ContractLayoutValue>(&value_bytes) {
                // Extract version from key (simplified parsing)
                if let Ok(key_str) = String::from_utf8(key_bytes) {
                    let parts: Vec<&str> = key_str.split('/').collect();
                    let version = if parts.len() > 4 {
                        parts[4].to_string()
                    } else {
                        "default".to_string()
                    };
                    layouts.push((version, layout_value));
                }
            }
        }
        
        // Check for conflicts (different layout commitments)
        if layouts.len() > 1 {
            let first_commitment = layouts[0].1.layout_commitment;
            let has_conflict = layouts.iter()
                .any(|(_, layout)| layout.layout_commitment != first_commitment);
            
            if has_conflict {
                return Ok(Some(layout::LayoutConflict {
                    chain_id: chain_id.to_string(),
                    contract_address: contract_address.to_string(),
                    conflicting_layouts: layouts,
                    detected_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    resolution: None,
                }));
            }
        }
        
        Ok(None)
    }

    /// Resolve a layout conflict
    pub async fn resolve_layout_conflict(
        &self,
        chain_id: &str,
        contract_address: &str,
        chosen_version: &str,
        reason: &str,
        resolved_by: &str,
    ) -> Result<()> {
        // Store the resolution
        let resolution_key = TupleKey::new(TupleKeyspace::ViewRegistry)
            .string("layout_resolution")
            .string(chain_id)
            .string(contract_address);
        
        let resolution = layout::LayoutConflictResolution {
            chosen_version: chosen_version.to_string(),
            reason: reason.to_string(),
            resolved_by: resolved_by.to_string(),
            resolved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        let serialized = serde_json::to_vec(&resolution)
            .map_err(|e| Error::Serialization(format!("Failed to serialize layout resolution: {}", e)))?;
        
        self.set(resolution_key, serialized).await?;
        
        info!("Resolved layout conflict for {}/{} - chose version {} for reason: {}", 
              chain_id, contract_address, chosen_version, reason);
        Ok(())
    }

    /// Store enhanced block metadata
    pub async fn store_block_metadata(
        &self,
        key: block_meta::BlockDataKey,
        value: block_meta::BlockDataValue,
    ) -> Result<()> {
        // Validate the block data
        value.validate()?;
        
        let tuple_key = key.to_tuple_key();
        let serialized = serde_json::to_vec(&value)
            .map_err(|e| Error::Serialization(format!("Failed to serialize block data: {}", e)))?;
        
        self.set(tuple_key, serialized).await?;
        
        // Update latest block tracker if this is newer
        let latest_key = TupleKey::new(TupleKeyspace::BlockMeta)
            .string(&key.chain_id)
            .string("latest");
        
        let latest_info = block_meta::LatestBlockInfo {
            chain_id: key.chain_id.clone(),
            latest_height: value.block_height,
            latest_hash: value.block_hash,
            finalized_height: if value.is_final(6) { value.block_height } else { 0 },
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        let latest_serialized = serde_json::to_vec(&latest_info)
            .map_err(|e| Error::Serialization(format!("Failed to serialize latest info: {}", e)))?;
        
        self.set(latest_key, latest_serialized).await?;
        
        debug!("Stored block metadata for {}/{} with state root {:02x?}", 
               key.chain_id, value.block_height, &value.state_root[0..8]);
        Ok(())
    }

    /// Get block metadata by identifier
    pub async fn get_block_metadata(
        &self,
        chain_id: &str,
        block_identifier: &block_meta::BlockIdentifier,
    ) -> Result<Option<block_meta::BlockDataValue>> {
        let key = block_meta::BlockDataKey::new(
            chain_id.to_string(),
            block_identifier.clone(),
        );
        
        let tuple_key = key.to_tuple_key();
        match self.get(tuple_key).await? {
            Some(data) => {
                let block_data: block_meta::BlockDataValue = serde_json::from_slice(&data)
                    .map_err(|e| Error::Serialization(format!("Failed to parse block data: {}", e)))?;
                Ok(Some(block_data))
            }
            None => Ok(None),
        }
    }

    /// Get latest block info for a chain
    pub async fn get_latest_block_info(&self, chain_id: &str) -> Result<Option<block_meta::LatestBlockInfo>> {
        let latest_key = TupleKey::new(TupleKeyspace::BlockMeta)
            .string(chain_id)
            .string("latest");
        
        match self.get(latest_key).await? {
            Some(data) => {
                let latest_info: block_meta::LatestBlockInfo = serde_json::from_slice(&data)
                    .map_err(|e| Error::Serialization(format!("Failed to parse latest info: {}", e)))?;
                Ok(Some(latest_info))
            }
            None => Ok(None),
        }
    }

    /// Get block range for a chain
    pub async fn get_block_range(
        &self,
        chain_id: &str,
        start_height: u64,
        end_height: u64,
        limit: Option<usize>,
    ) -> Result<Vec<block_meta::BlockDataValue>> {
        let start_key = TupleKey::new(TupleKeyspace::BlockMeta)
            .string(chain_id)
            .int(start_height as i64);
        
        let end_key = TupleKey::new(TupleKeyspace::BlockMeta)
            .string(chain_id)
            .int((end_height + 1) as i64); // Exclusive end
        
        // Use range query with start and end bounds
        let results = self.range(start_key, limit).await?;
        let mut blocks = Vec::new();
        
        for (key_bytes, value_bytes) in results {
            // Check if key is within our range
            if let Ok(block_data) = serde_json::from_slice::<block_meta::BlockDataValue>(&value_bytes) {
                if block_data.block_height >= start_height && block_data.block_height <= end_height {
                    blocks.push(block_data);
                } else if block_data.block_height > end_height {
                    break; // Past our range
                }
            }
        }
        
        // Sort by block height (should already be sorted by tuple encoding)
        blocks.sort_by_key(|b| b.block_height);
        
        Ok(blocks)
    }

    /// Handle block reorganization
    pub async fn handle_block_reorg(
        &self,
        chain_id: &str,
        reorg_height: u64,
        old_block_hash: [u8; 32],
        new_block_hash: [u8; 32],
        reorg_depth: u64,
    ) -> Result<()> {
        // Record the reorganization event
        let reorg_event = block_meta::BlockReorgEvent {
            chain_id: chain_id.to_string(),
            reorg_height,
            old_block_hash,
            new_block_hash,
            reorg_depth,
            detected_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        let reorg_key = TupleKey::new(TupleKeyspace::BlockMeta)
            .string(chain_id)
            .string("reorg")
            .int(reorg_height as i64);
        
        let serialized = serde_json::to_vec(&reorg_event)
            .map_err(|e| Error::Serialization(format!("Failed to serialize reorg event: {}", e)))?;
        
        self.set(reorg_key, serialized).await?;
        
        // Mark affected blocks as reorganized
        for height in reorg_height..=(reorg_height + reorg_depth) {
            let block_key = TupleKey::new(TupleKeyspace::BlockMeta)
                .string(chain_id)
                .int(height as i64);
            
            if let Some(data) = self.get(block_key.clone()).await? {
                if let Ok(mut block_data) = serde_json::from_slice::<block_meta::BlockDataValue>(&data) {
                    block_data.finality_status = block_meta::FinalityStatus::Reorganized;
                    
                    let updated = serde_json::to_vec(&block_data)
                        .map_err(|e| Error::Serialization(format!("Failed to serialize updated block: {}", e)))?;
                    
                    self.set(block_key, updated).await?;
                }
            }
        }
        
        info!("Handled block reorganization for chain {} at height {} with depth {}", 
              chain_id, reorg_height, reorg_depth);
        Ok(())
    }

    /// Update block finality status
    pub async fn update_block_finality(
        &self,
        chain_id: &str,
        block_height: u64,
        finality_status: block_meta::FinalityStatus,
        confirmations: u64,
    ) -> Result<()> {
        let block_key = TupleKey::new(TupleKeyspace::BlockMeta)
            .string(chain_id)
            .int(block_height as i64);
        
        if let Some(data) = self.get(block_key.clone()).await? {
            if let Ok(mut block_data) = serde_json::from_slice::<block_meta::BlockDataValue>(&data) {
                block_data.finality_status = finality_status;
                block_data.confirmations = confirmations;
                
                let updated = serde_json::to_vec(&block_data)
                    .map_err(|e| Error::Serialization(format!("Failed to serialize block finality update: {}", e)))?;
                
                self.set(block_key, updated).await?;
                
                debug!("Updated block finality for {}/{} to {:?} with {} confirmations", 
                       chain_id, block_height, finality_status, confirmations);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    pub mod contract_state_tests;
} 