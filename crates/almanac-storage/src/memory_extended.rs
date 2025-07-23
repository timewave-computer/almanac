//! In-memory implementation for testing and minimal e2e functionality

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api_adapter::{
    ApiResult, ContractLayoutKey, ContractLayoutValue, DeveloperTrustValue, ExtendedStorageReader,
    LayoutConflict, SemanticConflicts, SemanticMetadataKey, SemanticMetadataValue,
    StorageProofValue, TrustLevel, TrustPolicy, TrustScope, ViewDefinition, ViewQueryResult,
};
use almanac_core::types::BlockStatus;
use almanac_core::{Error, Event, EventFilter, Result};
use almanac_core::{StorageReader, StorageWriter};

/// Extended memory storage for testing
pub struct MemoryExtendedStorage {
    /// Key-value store for arbitrary data
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    /// Event storage
    events: Arc<RwLock<Vec<Box<dyn Event + Send + Sync>>>>,
    /// Block status tracking
    block_status: Arc<RwLock<HashMap<String, HashMap<u64, BlockStatus>>>>,
}

impl MemoryExtendedStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            block_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store arbitrary key-value data
    pub async fn store_data(&self, key: &str, value: Vec<u8>) -> Result<()> {
        let mut data = self.data.write().await;
        data.insert(key.to_string(), value);
        Ok(())
    }

    /// Get arbitrary key-value data
    pub async fn get_data(&self, key: &str) -> Result<Vec<u8>> {
        let data = self.data.read().await;
        data.get(key)
            .cloned()
            .ok_or_else(|| Error::storage("Key not found"))
    }

    /// Check if key exists
    pub async fn has_data(&self, key: &str) -> bool {
        let data = self.data.read().await;
        data.contains_key(key)
    }
}

impl Default for MemoryExtendedStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageWriter for MemoryExtendedStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        let mut events = self.events.write().await;
        events.push(event);
        Ok(())
    }

    async fn store_events_batch(&self, events: Vec<Box<dyn Event>>) -> Result<()> {
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
        let mut block_status = self.block_status.write().await;
        let chain_blocks = block_status
            .entry(chain.to_string())
            .or_insert_with(HashMap::new);
        chain_blocks.insert(block_number, status);
        Ok(())
    }

    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()> {
        let mut block_status = self.block_status.write().await;
        if let Some(chain_blocks) = block_status.get_mut(chain) {
            chain_blocks.retain(|&block, _| block < from_block);
        }
        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl StorageReader for MemoryExtendedStorage {
    async fn get_events(&self, _filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        // Simple implementation - return empty for now
        Ok(Vec::new())
    }

    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        let block_status = self.block_status.read().await;
        if let Some(chain_blocks) = block_status.get(chain) {
            Ok(chain_blocks.keys().max().copied().unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        let block_status = self.block_status.read().await;
        if let Some(chain_blocks) = block_status.get(chain) {
            let latest = chain_blocks
                .iter()
                .filter(|(_, &s)| s == status)
                .map(|(block, _)| *block)
                .max()
                .unwrap_or(0);
            Ok(latest)
        } else {
            Ok(0)
        }
    }

    async fn get_events_with_status(
        &self,
        filters: Vec<EventFilter>,
        _status: BlockStatus,
    ) -> Result<Vec<Box<dyn Event>>> {
        self.get_events(filters).await
    }

    async fn get_latest_block_before(&self, chain: &str, before_block: u64) -> Result<u64> {
        self.get_latest_block(chain)
            .await
            .map(|latest| latest.min(before_block.saturating_sub(1)))
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl ExtendedStorageReader for MemoryExtendedStorage {
    // Trust management methods
    async fn grant_trust(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: TrustScope,
        _level: TrustLevel,
        _expires_at: Option<u64>,
        _conditions: Vec<String>,
    ) -> ApiResult<()> {
        // Minimal implementation for memory storage
        Ok(())
    }

    async fn get_trust_relationship(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: &TrustScope,
    ) -> ApiResult<Option<DeveloperTrustValue>> {
        // Return None for memory storage
        Ok(None)
    }

    async fn revoke_trust(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: TrustScope,
        _reason: &str,
    ) -> ApiResult<()> {
        Ok(())
    }

    async fn get_trust_policy(&self, _developer_id: &str) -> ApiResult<Option<TrustPolicy>> {
        Ok(None)
    }

    async fn update_trust_policy(
        &self,
        _developer_id: &str,
        _policy: TrustPolicy,
    ) -> ApiResult<()> {
        Ok(())
    }

    // Contract layout methods
    async fn store_contract_layout(
        &self,
        _key: ContractLayoutKey,
        _value: ContractLayoutValue,
    ) -> ApiResult<()> {
        Ok(())
    }

    async fn get_contract_layout(
        &self,
        _chain: &str,
        _contract: &str,
        _version: Option<&str>,
        _requested_by: &str,
        _trust_policy: &TrustPolicy,
    ) -> ApiResult<Option<(ContractLayoutValue, bool)>> {
        Ok(None)
    }

    async fn list_layouts(
        &self,
        _chain: &str,
        _contract: &str,
    ) -> ApiResult<Vec<ContractLayoutValue>> {
        Ok(Vec::new())
    }

    async fn verify_layout_commitment(
        &self,
        _chain: &str,
        _contract: &str,
        _commitment: &[u8; 32],
    ) -> ApiResult<bool> {
        Ok(false)
    }

    async fn detect_layout_conflicts(
        &self,
        _chain: &str,
        _contract: &str,
    ) -> ApiResult<Option<LayoutConflict>> {
        Ok(None)
    }

    async fn resolve_layout_conflict(
        &self,
        _chain: &str,
        _contract: &str,
        _chosen_version: &str,
        _reason: &str,
        _resolved_by: &str,
    ) -> ApiResult<()> {
        Ok(())
    }

    // View management methods
    async fn register_view(&self, _definition: ViewDefinition) -> ApiResult<()> {
        Ok(())
    }

    async fn get_view(&self, _view_id: &str) -> ApiResult<Option<ViewDefinition>> {
        Ok(None)
    }

    async fn update_view(&self, _definition: ViewDefinition) -> ApiResult<()> {
        Ok(())
    }

    async fn delete_view(&self, _view_id: &str) -> ApiResult<()> {
        Ok(())
    }

    async fn list_views_for_contract(
        &self,
        _contract_address: &str,
    ) -> ApiResult<Vec<ViewDefinition>> {
        Ok(Vec::new())
    }

    async fn query_view(
        &self,
        _view_id: &str,
        _chain: &str,
        _contract: &str,
        _block_height: u64,
        _include_proofs: bool,
    ) -> ApiResult<ViewQueryResult> {
        Ok(ViewQueryResult {
            data: HashMap::new(),
            proofs: None,
        })
    }

    // Missing trait methods - minimal implementations for memory storage
    async fn get_trusted_developers(
        &self,
        _trustor: &str,
        _min_level: TrustLevel,
    ) -> ApiResult<Vec<(String, TrustScope, TrustLevel)>> {
        Ok(Vec::new())
    }

    async fn check_trust(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: &TrustScope,
        _minimum_level: TrustLevel,
    ) -> ApiResult<bool> {
        Ok(false)
    }

    async fn get_trust_graph(&self, _developer_id: &str) -> ApiResult<HashMap<String, TrustLevel>> {
        Ok(HashMap::new())
    }

    async fn store_semantic_metadata(
        &self,
        _key: SemanticMetadataKey,
        _value: SemanticMetadataValue,
    ) -> ApiResult<()> {
        Ok(())
    }

    async fn get_semantic_metadata(
        &self,
        _key: &SemanticMetadataKey,
    ) -> ApiResult<Option<SemanticMetadataValue>> {
        Ok(None)
    }

    async fn detect_semantic_conflicts(
        &self,
        _chain: &str,
        _contract: &str,
        _field: &str,
    ) -> ApiResult<Option<SemanticConflicts>> {
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
        Ok(())
    }

    async fn submit_layout(
        &self,
        _key: ContractLayoutKey,
        _layout: ContractLayoutValue,
    ) -> ApiResult<()> {
        Ok(())
    }

    async fn get_layout(&self, _key: &ContractLayoutKey) -> ApiResult<Option<ContractLayoutValue>> {
        Ok(None)
    }

    async fn get_storage_proof(
        &self,
        _chain: &str,
        _contract: &str,
        _block_height: u64,
        _storage_key: &[u8; 32],
    ) -> ApiResult<Option<StorageProofValue>> {
        Ok(None)
    }

    async fn get_latest_block_number(&self, _chain: &str) -> ApiResult<Option<u64>> {
        Ok(Some(0))
    }

    async fn get_anchor_idl(&self, _program_id: &str) -> ApiResult<Option<String>> {
        Ok(None)
    }
}
