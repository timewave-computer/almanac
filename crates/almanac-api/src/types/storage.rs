use async_trait::async_trait;
use crate::types::{ApiResult, ApiError, ApiEvent, EventFilter, ChainId, BlockNumber};
use crate::storage_ext::{ExtendedStorageReader, TrustScope, TrustLevel, TrustPolicy, ViewDefinition, ViewQueryResult, DeveloperTrustValue};

/// Minimal storage reader trait for API layer
#[async_trait]
pub trait StorageReader: Send + Sync {
    /// Get events matching the filter
    async fn get_events(&self, filter: EventFilter) -> ApiResult<Vec<ApiEvent>>;
    
    /// Get the latest block number for a chain
    async fn get_latest_block(&self, chain: &ChainId) -> ApiResult<BlockNumber>;
    
    /// Check if storage is healthy
    async fn health_check(&self) -> ApiResult<bool>;
}

/// In-memory storage reader for testing
pub struct MemoryStorageReader {
    events: Vec<ApiEvent>,
    latest_blocks: std::collections::HashMap<ChainId, BlockNumber>,
}

impl MemoryStorageReader {
    /// Create a new memory storage reader
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            latest_blocks: std::collections::HashMap::new(),
        }
    }
    
    /// Add an event to storage
    pub fn add_event(&mut self, event: ApiEvent) {
        // Update latest block
        let chain = ChainId::from(event.chain.clone());
        let block = event.block_number;
        
        self.latest_blocks
            .entry(chain)
            .and_modify(|b| *b = (*b).max(block))
            .or_insert(block);
            
        self.events.push(event);
    }
    
    /// Set the latest block for a chain
    pub fn set_latest_block(&mut self, chain: ChainId, block: BlockNumber) {
        self.latest_blocks.insert(chain, block);
    }
}

impl Default for MemoryStorageReader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageReader for MemoryStorageReader {
    async fn get_events(&self, filter: EventFilter) -> ApiResult<Vec<ApiEvent>> {
        let mut results = Vec::new();
        
        for event in &self.events {
            // Apply chain filter
            if let Some(ref chain) = filter.chain {
                if event.chain != chain.as_str() {
                    continue;
                }
            }
            
            // Apply block range filter
            if let Some(from) = filter.from_block {
                if event.block_number < from {
                    continue;
                }
            }
            
            if let Some(to) = filter.to_block {
                if event.block_number > to {
                    continue;
                }
            }
            
            // Apply event type filter
            if !filter.event_types.is_empty() && !filter.event_types.contains(&event.event_type) {
                continue;
            }
            
            results.push(event.clone());
        }
        
        // Apply pagination
        if let Some(offset) = filter.offset {
            results = results.into_iter().skip(offset as usize).collect();
        }
        
        if let Some(limit) = filter.limit {
            results.truncate(limit as usize);
        }
        
        Ok(results)
    }
    
    async fn get_latest_block(&self, chain: &ChainId) -> ApiResult<BlockNumber> {
        self.latest_blocks
            .get(chain)
            .copied()
            .ok_or_else(|| ApiError::not_found(format!("No blocks found for chain {}", chain)))
    }
    
    async fn health_check(&self) -> ApiResult<bool> {
        Ok(true)
    }
}

#[async_trait]
impl ExtendedStorageReader for MemoryStorageReader {
    // Trust management methods (stub implementations)
    async fn grant_trust(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: TrustScope,
        _level: TrustLevel,
        _expires_at: Option<u64>,
        _conditions: Vec<String>,
    ) -> ApiResult<()> {
        Err(ApiError::internal("Trust management not implemented in memory storage"))
    }

    async fn revoke_trust(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: TrustScope,
        _reason: &str,
    ) -> ApiResult<()> {
        Err(ApiError::internal("Trust management not implemented in memory storage"))
    }

    async fn get_trust_policy(&self, _developer_id: &str) -> ApiResult<Option<TrustPolicy>> {
        Ok(None)
    }

    async fn update_trust_policy(&self, _developer_id: &str, _policy: TrustPolicy) -> ApiResult<()> {
        Err(ApiError::internal("Trust management not implemented in memory storage"))
    }

    async fn get_trusted_developers(
        &self,
        _trustor: &str,
        _min_level: TrustLevel,
    ) -> ApiResult<Vec<(String, TrustScope, TrustLevel)>> {
        Ok(vec![])
    }

    async fn get_trust_relationship(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: &TrustScope,
    ) -> ApiResult<Option<DeveloperTrustValue>> {
        Ok(None)
    }

    async fn check_trust(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: &TrustScope,
        _required_level: TrustLevel,
    ) -> ApiResult<bool> {
        Ok(false)
    }

    async fn get_trust_graph(&self, _developer_id: &str) -> ApiResult<std::collections::HashMap<String, TrustLevel>> {
        Ok(std::collections::HashMap::new())
    }

    // Semantic metadata methods (stub implementations)
    async fn store_semantic_metadata(
        &self,
        _key: crate::storage_ext::SemanticMetadataKey,
        _value: crate::storage_ext::SemanticMetadataValue,
    ) -> ApiResult<()> {
        Err(ApiError::internal("Semantic metadata not implemented in memory storage"))
    }

    async fn get_semantic_metadata(
        &self,
        _key: &crate::storage_ext::SemanticMetadataKey,
    ) -> ApiResult<Option<crate::storage_ext::SemanticMetadataValue>> {
        Ok(None)
    }

    async fn detect_semantic_conflicts(
        &self,
        _chain: &str,
        _contract: &str,
        _field: &str,
    ) -> ApiResult<Option<crate::storage_ext::SemanticConflicts>> {
        Ok(None)
    }

    async fn resolve_semantic_conflict(
        &self,
        _chain: &str,
        _contract: &str,
        _field: &str,
        _chosen_developer: &str,
        _method: &str,
        _explanation: &str,
    ) -> ApiResult<()> {
        Err(ApiError::internal("Semantic conflict resolution not implemented in memory storage"))
    }

    // Layout management methods (stub implementations)
    async fn submit_layout(
        &self,
        _key: crate::storage_ext::ContractLayoutKey,
        _value: crate::storage_ext::ContractLayoutValue,
    ) -> ApiResult<()> {
        Err(ApiError::internal("Layout management not implemented in memory storage"))
    }

    async fn get_layout(
        &self,
        _key: &crate::storage_ext::ContractLayoutKey,
    ) -> ApiResult<Option<crate::storage_ext::ContractLayoutValue>> {
        Ok(None)
    }

    async fn list_layouts(
        &self,
        _chain: &str,
        _contract: &str,
    ) -> ApiResult<Vec<crate::storage_ext::ContractLayoutValue>> {
        Ok(vec![])
    }

    async fn verify_layout_commitment(
        &self,
        _chain: &str,
        _contract: &str,
        _commitment: &[u8; 32],
    ) -> ApiResult<bool> {
        Ok(false)
    }

    // View management methods (stub implementations)
    async fn register_view(&self, _definition: ViewDefinition) -> ApiResult<()> {
        Err(ApiError::internal("View management not implemented in memory storage"))
    }

    async fn get_view(&self, _view_id: &str) -> ApiResult<Option<ViewDefinition>> {
        Ok(None)
    }

    async fn update_view(&self, _definition: ViewDefinition) -> ApiResult<()> {
        Err(ApiError::internal("View management not implemented in memory storage"))
    }

    async fn delete_view(&self, _view_id: &str) -> ApiResult<()> {
        Err(ApiError::internal("View management not implemented in memory storage"))
    }

    async fn list_views_for_contract(&self, _contract_address: &str) -> ApiResult<Vec<ViewDefinition>> {
        Ok(vec![])
    }

    async fn query_view(
        &self,
        _view_id: &str,
        _chain: &str,
        _contract: &str,
        _block_height: u64,
        _include_proofs: bool,
    ) -> ApiResult<ViewQueryResult> {
        Err(ApiError::internal("View queries not implemented in memory storage"))
    }

    // Storage proof methods (stub implementations)
    async fn get_storage_proof(
        &self,
        _chain: &str,
        _contract: &str,
        _block_height: u64,
        _storage_key: &[u8; 32],
    ) -> ApiResult<Option<crate::storage_ext::StorageProofValue>> {
        Ok(None)
    }

    // Block data methods (stub implementations)
    async fn get_latest_block_number(&self, _chain: &str) -> ApiResult<Option<u64>> {
        Ok(None)
    }
}

/// Storage writer trait (for implementations that support writing)
#[async_trait]
pub trait StorageWriter: StorageReader {
    /// Store a new event
    async fn store_event(&self, event: ApiEvent) -> ApiResult<()>;
    
    /// Update the latest block for a chain
    async fn update_latest_block(&self, chain: ChainId, block: BlockNumber) -> ApiResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_memory_storage() {
        let mut storage = MemoryStorageReader::new();
        
        // Add some test events
        let event1 = ApiEvent::new("ethereum", 100, "Transfer", vec![1, 2, 3]);
        let event2 = ApiEvent::new("ethereum", 101, "Approval", vec![4, 5, 6]);
        let event3 = ApiEvent::new("cosmos", 200, "Transfer", vec![7, 8, 9]);
        
        storage.add_event(event1);
        storage.add_event(event2);
        storage.add_event(event3);
        
        // Test chain filter
        let filter = EventFilter::new().chain("ethereum");
        let events = storage.get_events(filter).await.unwrap();
        assert_eq!(events.len(), 2);
        
        // Test block range filter
        let filter = EventFilter::new().block_range(101, 201);
        let events = storage.get_events(filter).await.unwrap();
        assert_eq!(events.len(), 2);
        
        // Test event type filter
        let filter = EventFilter::new().event_type("Transfer");
        let events = storage.get_events(filter).await.unwrap();
        assert_eq!(events.len(), 2);
        
        // Test latest block
        let latest = storage.get_latest_block(&ChainId::from("ethereum")).await.unwrap();
        assert_eq!(latest, 101);
    }
    
    #[tokio::test]
    async fn test_pagination() {
        let mut storage = MemoryStorageReader::new();
        
        // Add 10 events
        for i in 0..10 {
            let event = ApiEvent::new("ethereum", 100 + i, "Transfer", vec![i as u8]);
            storage.add_event(event);
        }
        
        // Test limit
        let filter = EventFilter::new().paginate(5, 0);
        let events = storage.get_events(filter).await.unwrap();
        assert_eq!(events.len(), 5);
        
        // Test offset
        let filter = EventFilter::new().paginate(5, 5);
        let events = storage.get_events(filter).await.unwrap();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].block_number, 105);
    }
}