/// Storage backends for causality data and SMT nodes
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use indexer_core::types::ChainId;
use indexer_storage::BoxedStorage;

use crate::error::{CausalityError, Result};
use crate::types::{CausalityEvent, CausalityResource, CausalityIndex, Hash, SmtRoot};
use crate::smt::SmtBackend;

/// Storage backend for causality data
#[async_trait]
pub trait CausalityStorageBackend: Send + Sync {
    /// Store a causality event
    async fn store_event(&self, event: &CausalityEvent) -> Result<()>;
    
    /// Get a causality event by ID
    async fn get_event(&self, event_id: &str) -> Result<Option<CausalityEvent>>;
    
    /// Store a causality resource
    async fn store_resource(&self, resource: &CausalityResource) -> Result<()>;
    
    /// Get a causality resource by ID
    async fn get_resource(&self, resource_id: &str) -> Result<Option<CausalityResource>>;
    
    /// Store causality index
    async fn store_index(&self, index: &CausalityIndex) -> Result<()>;
    
    /// Get causality index
    async fn get_index(&self) -> Result<Option<CausalityIndex>>;
    
    /// Get events for a specific chain
    async fn get_chain_events(&self, chain_id: &ChainId) -> Result<Vec<CausalityEvent>>;
    
    /// Get events in a block range
    async fn get_events_in_range(&self, chain_id: &ChainId, start_block: u64, end_block: u64) -> Result<Vec<CausalityEvent>>;
}

/// In-memory storage backend for testing and development
#[derive(Debug, Clone, Default)]
pub struct MemoryCausalityStorage {
    events: Arc<RwLock<HashMap<String, CausalityEvent>>>,
    resources: Arc<RwLock<HashMap<String, CausalityResource>>>,
    index: Arc<RwLock<Option<CausalityIndex>>>,
    chain_events: Arc<RwLock<HashMap<ChainId, Vec<String>>>>,
}

impl MemoryCausalityStorage {
    /// Create a new memory storage backend
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
            resources: Arc::new(RwLock::new(HashMap::new())),
            index: Arc::new(RwLock::new(None)),
            chain_events: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl CausalityStorageBackend for MemoryCausalityStorage {
    async fn store_event(&self, event: &CausalityEvent) -> Result<()> {
        let mut events = self.events.write().await;
        let mut chain_events = self.chain_events.write().await;
        
        events.insert(event.id.clone(), event.clone());
        
        chain_events
            .entry(event.chain_id.clone())
            .or_insert_with(Vec::new)
            .push(event.id.clone());
        
        Ok(())
    }
    
    async fn get_event(&self, event_id: &str) -> Result<Option<CausalityEvent>> {
        let events = self.events.read().await;
        Ok(events.get(event_id).cloned())
    }
    
    async fn store_resource(&self, resource: &CausalityResource) -> Result<()> {
        let mut resources = self.resources.write().await;
        let resource_id = hex::encode(resource.id.inner());
        resources.insert(resource_id, resource.clone());
        Ok(())
    }
    
    async fn get_resource(&self, resource_id: &str) -> Result<Option<CausalityResource>> {
        let resources = self.resources.read().await;
        Ok(resources.get(resource_id).cloned())
    }
    
    async fn store_index(&self, index: &CausalityIndex) -> Result<()> {
        let mut stored_index = self.index.write().await;
        *stored_index = Some(index.clone());
        Ok(())
    }
    
    async fn get_index(&self) -> Result<Option<CausalityIndex>> {
        let index = self.index.read().await;
        Ok(index.clone())
    }
    
    async fn get_chain_events(&self, chain_id: &ChainId) -> Result<Vec<CausalityEvent>> {
        let chain_events = self.chain_events.read().await;
        let events = self.events.read().await;
        
        if let Some(event_ids) = chain_events.get(chain_id) {
            let mut result = Vec::new();
            for event_id in event_ids {
                if let Some(event) = events.get(event_id) {
                    result.push(event.clone());
                }
            }
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn get_events_in_range(&self, chain_id: &ChainId, start_block: u64, end_block: u64) -> Result<Vec<CausalityEvent>> {
        let chain_events = self.get_chain_events(chain_id).await?;
        
        Ok(chain_events
            .into_iter()
            .filter(|event| event.block_number >= start_block && event.block_number <= end_block)
            .collect())
    }
}

/// PostgreSQL storage backend for causality data
#[derive(Clone)]
pub struct PostgresCausalityStorage {
    #[allow(dead_code)]
    storage: BoxedStorage,
}

impl PostgresCausalityStorage {
    /// Create a new PostgreSQL storage backend
    pub fn new(storage: BoxedStorage) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl CausalityStorageBackend for PostgresCausalityStorage {
    async fn store_event(&self, _event: &CausalityEvent) -> Result<()> {
        // TODO: Implement PostgreSQL storage for causality events
        Err(CausalityError::storage_error("PostgreSQL causality storage not yet implemented"))
    }
    
    async fn get_event(&self, _event_id: &str) -> Result<Option<CausalityEvent>> {
        // TODO: Implement PostgreSQL retrieval for causality events
        Err(CausalityError::storage_error("PostgreSQL causality storage not yet implemented"))
    }
    
    async fn store_resource(&self, _resource: &CausalityResource) -> Result<()> {
        // TODO: Implement PostgreSQL storage for causality resources
        Err(CausalityError::storage_error("PostgreSQL causality storage not yet implemented"))
    }
    
    async fn get_resource(&self, _resource_id: &str) -> Result<Option<CausalityResource>> {
        // TODO: Implement PostgreSQL retrieval for causality resources
        Err(CausalityError::storage_error("PostgreSQL causality storage not yet implemented"))
    }
    
    async fn store_index(&self, _index: &CausalityIndex) -> Result<()> {
        // TODO: Implement PostgreSQL storage for causality index
        Err(CausalityError::storage_error("PostgreSQL causality storage not yet implemented"))
    }
    
    async fn get_index(&self) -> Result<Option<CausalityIndex>> {
        // TODO: Implement PostgreSQL retrieval for causality index
        Err(CausalityError::storage_error("PostgreSQL causality storage not yet implemented"))
    }
    
    async fn get_chain_events(&self, _chain_id: &ChainId) -> Result<Vec<CausalityEvent>> {
        // TODO: Implement PostgreSQL retrieval for chain events
        Err(CausalityError::storage_error("PostgreSQL causality storage not yet implemented"))
    }
    
    async fn get_events_in_range(&self, _chain_id: &ChainId, _start_block: u64, _end_block: u64) -> Result<Vec<CausalityEvent>> {
        // TODO: Implement PostgreSQL retrieval for events in range
        Err(CausalityError::storage_error("PostgreSQL causality storage not yet implemented"))
    }
}

/// Combined storage for both causality data and SMT nodes
pub struct CausalityStorage<B: SmtBackend> {
    /// Causality data storage
    causality_backend: Box<dyn CausalityStorageBackend>,
    /// SMT storage backend
    smt_backend: B,
}

impl<B: SmtBackend> CausalityStorage<B> {
    /// Create a new causality storage
    pub fn new(causality_backend: Box<dyn CausalityStorageBackend>, smt_backend: B) -> Self {
        Self {
            causality_backend,
            smt_backend,
        }
    }

    /// Get the causality storage backend
    pub fn causality_backend(&self) -> &dyn CausalityStorageBackend {
        self.causality_backend.as_ref()
    }

    /// Get the SMT storage backend
    pub fn smt_backend(&self) -> &B {
        &self.smt_backend
    }

    /// Store an event and update the SMT
    pub async fn store_event_with_smt(&self, event: &CausalityEvent, _smt_root: SmtRoot) -> Result<()> {
        // Store the event in causality storage
        self.causality_backend.store_event(event).await?;
        
        // Store SMT data would be handled by the SMT layer
        // This is a placeholder for the integration
        
        Ok(())
    }

    /// Store a resource and update the SMT
    pub async fn store_resource_with_smt(&self, resource: &CausalityResource, _smt_root: SmtRoot) -> Result<()> {
        // Store the resource in causality storage
        self.causality_backend.store_resource(resource).await?;
        
        // Store SMT data would be handled by the SMT layer
        // This is a placeholder for the integration
        
        Ok(())
    }
}

// Note: CausalityStorage cannot implement Clone because 
// CausalityStorageBackend trait objects cannot be cloned

/// SMT-specific storage operations
pub struct SmtStorage<B: SmtBackend> {
    backend: B,
}

impl<B: SmtBackend> SmtStorage<B> {
    /// Create a new SMT storage
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// Store SMT node data
    pub async fn store_node(&self, node_hash: &Hash, data: &[u8]) -> Result<()> {
        self.backend.set(b"smt-node", node_hash, data).await?;
        Ok(())
    }

    /// Get SMT node data
    pub async fn get_node(&self, node_hash: &Hash) -> Result<Option<Vec<u8>>> {
        self.backend.get(b"smt-node", node_hash).await
    }

    /// Store SMT leaf data
    pub async fn store_leaf(&self, key: &Hash, data: &[u8]) -> Result<()> {
        self.backend.set(b"smt-data", key, data).await?;
        Ok(())
    }

    /// Get SMT leaf data
    pub async fn get_leaf(&self, key: &Hash) -> Result<Option<Vec<u8>>> {
        self.backend.get(b"smt-data", key).await
    }

    /// Store key mapping
    pub async fn store_key_mapping(&self, node_hash: &Hash, key: &Hash) -> Result<()> {
        self.backend.set(b"smt-key", node_hash, key.as_ref()).await?;
        Ok(())
    }

    /// Get key mapping
    pub async fn get_key_mapping(&self, node_hash: &Hash) -> Result<Option<Hash>> {
        if let Some(data) = self.backend.get(b"smt-key", node_hash).await? {
            if data.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&data);
                Ok(Some(key))
            } else {
                Err(CausalityError::storage_error("Invalid key mapping data length"))
            }
        } else {
            Ok(None)
        }
    }

    /// Check if key mapping exists
    pub async fn has_key_mapping(&self, node_hash: &Hash) -> Result<bool> {
        self.backend.has(b"smt-key", node_hash).await
    }
}

impl<B: SmtBackend> Clone for SmtStorage<B> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use crate::types::empty_hash;

    #[tokio::test]
    async fn test_memory_causality_storage() {
        let storage = MemoryCausalityStorage::new();
        
        let event = CausalityEvent {
            id: "test-event-1".to_string(),
            chain_id: ChainId("test-chain".to_string()),
            block_number: 100,
            tx_hash: "0x123".to_string(),
            event_type: crate::types::CausalityEventType::CrossDomainMessage,
            timestamp: SystemTime::now(),
            data: crate::types::CausalityEventData::CrossDomainMessage {
                source_domain: empty_hash(),
                target_domain: empty_hash(),
                message_type: "test".to_string(),
                payload: b"test data".to_vec(),
            },
        };

        // Store event
        storage.store_event(&event).await.unwrap();

        // Retrieve event
        let retrieved = storage.get_event("test-event-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-event-1");

        // Get chain events
        let chain_events = storage.get_chain_events(&ChainId("test-chain".to_string())).await.unwrap();
        assert_eq!(chain_events.len(), 1);
        assert_eq!(chain_events[0].id, "test-event-1");
    }

    #[tokio::test]
    async fn test_memory_causality_storage_range_query() {
        let storage = MemoryCausalityStorage::new();
        let chain_id = ChainId("test-chain".to_string());
        
        // Store events at different block heights
        for i in 1..=10 {
            let event = CausalityEvent {
                id: format!("event-{}", i),
                chain_id: chain_id.clone(),
                block_number: i * 10,
                tx_hash: format!("0x{}", i),
                event_type: crate::types::CausalityEventType::CrossDomainMessage,
                timestamp: SystemTime::now(),
                data: crate::types::CausalityEventData::CrossDomainMessage {
                    source_domain: empty_hash(),
                    target_domain: empty_hash(),
                    message_type: "test".to_string(),
                    payload: format!("data-{}", i).into_bytes(),
                },
            };
            storage.store_event(&event).await.unwrap();
        }

        // Query events in range
        let events = storage.get_events_in_range(&chain_id, 25, 75).await.unwrap();
        assert_eq!(events.len(), 5); // blocks 30, 40, 50, 60, 70

        // Verify the events are in the correct range
        for event in events {
            assert!(event.block_number >= 25 && event.block_number <= 75);
        }
    }
} 