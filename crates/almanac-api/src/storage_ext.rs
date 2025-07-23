//! Extended storage trait for API layer
//!
//! This trait extends the basic StorageReader with all the methods needed
//! by the API handlers for trust, semantics, layouts, and views.

use crate::types::{ApiResult, StorageReader};
use async_trait::async_trait;
// Import from api_adapter which provides FDB-independent types
pub use almanac_storage::api_adapter::{
    TrustLevel, TrustPolicy, TrustScope, ViewDefinition, ViewQueryResult,
};

// Stub types for the missing ones - these would be properly implemented when FDB is integrated
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeveloperTrustValue {
    pub trustor: String,
    pub trustee: String,
    pub scope: TrustScope,
    pub level: TrustLevel,
    pub expires_at: Option<u64>,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticMetadataKey {
    pub chain: String,
    pub contract: String,
    pub field: String,
    pub developer: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticMetadataValue {
    pub data_type: String,
    pub semantic_tags: Vec<String>,
    pub zero_semantics: String,
    pub confidence_level: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticConflicts {
    pub field: String,
    pub conflicting_interpretations: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractLayoutKey {
    pub chain: String,
    pub contract: String,
    pub version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractLayoutValue {
    pub layout: serde_json::Value,
    pub commitment: [u8; 32],
    pub developer: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageProofValue {
    pub proof: Vec<u8>,
    pub value: Option<Vec<u8>>,
    pub block_hash: [u8; 32],
}
use std::collections::HashMap;

/// Extended storage trait with all methods needed by API handlers
#[async_trait]
pub trait ExtendedStorageReader: StorageReader {
    // Trust management methods
    async fn grant_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: TrustScope,
        level: TrustLevel,
        expires_at: Option<u64>,
        conditions: Vec<String>,
    ) -> ApiResult<()>;

    async fn revoke_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: TrustScope,
        reason: &str,
    ) -> ApiResult<()>;

    async fn get_trust_policy(&self, developer_id: &str) -> ApiResult<Option<TrustPolicy>>;

    async fn update_trust_policy(&self, developer_id: &str, policy: TrustPolicy) -> ApiResult<()>;

    async fn get_trusted_developers(
        &self,
        trustor: &str,
        min_level: TrustLevel,
    ) -> ApiResult<Vec<(String, TrustScope, TrustLevel)>>;

    async fn get_trust_relationship(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &TrustScope,
    ) -> ApiResult<Option<DeveloperTrustValue>>;

    async fn check_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &TrustScope,
        required_level: TrustLevel,
    ) -> ApiResult<bool>;

    async fn get_trust_graph(&self, developer_id: &str) -> ApiResult<HashMap<String, TrustLevel>>;

    // Semantic metadata methods
    async fn store_semantic_metadata(
        &self,
        key: SemanticMetadataKey,
        value: SemanticMetadataValue,
    ) -> ApiResult<()>;

    async fn get_semantic_metadata(
        &self,
        key: &SemanticMetadataKey,
    ) -> ApiResult<Option<SemanticMetadataValue>>;

    async fn detect_semantic_conflicts(
        &self,
        chain: &str,
        contract: &str,
        field: &str,
    ) -> ApiResult<Option<SemanticConflicts>>;

    async fn resolve_semantic_conflict(
        &self,
        chain: &str,
        contract: &str,
        field: &str,
        chosen_developer: &str,
        method: &str,
        explanation: &str,
    ) -> ApiResult<()>;

    // Layout management methods
    async fn submit_layout(
        &self,
        key: ContractLayoutKey,
        value: ContractLayoutValue,
    ) -> ApiResult<()>;

    async fn get_layout(&self, key: &ContractLayoutKey) -> ApiResult<Option<ContractLayoutValue>>;

    async fn list_layouts(
        &self,
        chain: &str,
        contract: &str,
    ) -> ApiResult<Vec<ContractLayoutValue>>;

    async fn verify_layout_commitment(
        &self,
        chain: &str,
        contract: &str,
        commitment: &[u8; 32],
    ) -> ApiResult<bool>;

    // View management methods
    async fn register_view(&self, definition: ViewDefinition) -> ApiResult<()>;

    async fn get_view(&self, view_id: &str) -> ApiResult<Option<ViewDefinition>>;

    async fn update_view(&self, definition: ViewDefinition) -> ApiResult<()>;

    async fn delete_view(&self, view_id: &str) -> ApiResult<()>;

    async fn list_views_for_contract(
        &self,
        contract_address: &str,
    ) -> ApiResult<Vec<ViewDefinition>>;

    async fn query_view(
        &self,
        view_id: &str,
        chain: &str,
        contract: &str,
        block_height: u64,
        include_proofs: bool,
    ) -> ApiResult<ViewQueryResult>;

    // Storage proof methods
    async fn get_storage_proof(
        &self,
        chain: &str,
        contract: &str,
        block_height: u64,
        storage_key: &[u8; 32],
    ) -> ApiResult<Option<StorageProofValue>>;

    // Block data methods
    async fn get_latest_block_number(&self, chain: &str) -> ApiResult<Option<u64>>;

    // Anchor IDL methods
    async fn get_anchor_idl(&self, program_id: &str) -> ApiResult<Option<String>>;
}

/// Blanket implementation for Arc<T> where T implements StorageReader
#[async_trait]
impl<T> StorageReader for std::sync::Arc<T>
where
    T: StorageReader + ?Sized,
{
    async fn get_events(
        &self,
        filter: crate::types::EventFilter,
    ) -> ApiResult<Vec<crate::types::ApiEvent>> {
        (**self).get_events(filter).await
    }

    async fn get_latest_block(
        &self,
        chain: &crate::types::ChainId,
    ) -> ApiResult<crate::types::BlockNumber> {
        (**self).get_latest_block(chain).await
    }

    async fn health_check(&self) -> ApiResult<bool> {
        (**self).health_check().await
    }
}

/// Blanket implementation for Arc<T> where T implements ExtendedStorageReader
#[async_trait]
impl<T> ExtendedStorageReader for std::sync::Arc<T>
where
    T: ExtendedStorageReader + ?Sized,
{
    async fn grant_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: TrustScope,
        level: TrustLevel,
        expires_at: Option<u64>,
        conditions: Vec<String>,
    ) -> ApiResult<()> {
        (**self)
            .grant_trust(trustor, trustee, scope, level, expires_at, conditions)
            .await
    }

    async fn revoke_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: TrustScope,
        reason: &str,
    ) -> ApiResult<()> {
        (**self).revoke_trust(trustor, trustee, scope, reason).await
    }

    async fn get_trust_policy(&self, developer_id: &str) -> ApiResult<Option<TrustPolicy>> {
        (**self).get_trust_policy(developer_id).await
    }

    async fn update_trust_policy(&self, developer_id: &str, policy: TrustPolicy) -> ApiResult<()> {
        (**self).update_trust_policy(developer_id, policy).await
    }

    async fn get_trusted_developers(
        &self,
        trustor: &str,
        min_level: TrustLevel,
    ) -> ApiResult<Vec<(String, TrustScope, TrustLevel)>> {
        (**self).get_trusted_developers(trustor, min_level).await
    }

    async fn get_trust_relationship(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &TrustScope,
    ) -> ApiResult<Option<DeveloperTrustValue>> {
        (**self)
            .get_trust_relationship(trustor, trustee, scope)
            .await
    }

    async fn check_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &TrustScope,
        required_level: TrustLevel,
    ) -> ApiResult<bool> {
        (**self)
            .check_trust(trustor, trustee, scope, required_level)
            .await
    }

    async fn get_trust_graph(&self, developer_id: &str) -> ApiResult<HashMap<String, TrustLevel>> {
        (**self).get_trust_graph(developer_id).await
    }

    async fn store_semantic_metadata(
        &self,
        key: SemanticMetadataKey,
        value: SemanticMetadataValue,
    ) -> ApiResult<()> {
        (**self).store_semantic_metadata(key, value).await
    }

    async fn get_semantic_metadata(
        &self,
        key: &SemanticMetadataKey,
    ) -> ApiResult<Option<SemanticMetadataValue>> {
        (**self).get_semantic_metadata(key).await
    }

    async fn detect_semantic_conflicts(
        &self,
        chain: &str,
        contract: &str,
        field: &str,
    ) -> ApiResult<Option<SemanticConflicts>> {
        (**self)
            .detect_semantic_conflicts(chain, contract, field)
            .await
    }

    async fn resolve_semantic_conflict(
        &self,
        chain: &str,
        contract: &str,
        field: &str,
        chosen_developer: &str,
        method: &str,
        explanation: &str,
    ) -> ApiResult<()> {
        (**self)
            .resolve_semantic_conflict(
                chain,
                contract,
                field,
                chosen_developer,
                method,
                explanation,
            )
            .await
    }

    async fn submit_layout(
        &self,
        key: ContractLayoutKey,
        value: ContractLayoutValue,
    ) -> ApiResult<()> {
        (**self).submit_layout(key, value).await
    }

    async fn get_layout(&self, key: &ContractLayoutKey) -> ApiResult<Option<ContractLayoutValue>> {
        (**self).get_layout(key).await
    }

    async fn list_layouts(
        &self,
        chain: &str,
        contract: &str,
    ) -> ApiResult<Vec<ContractLayoutValue>> {
        (**self).list_layouts(chain, contract).await
    }

    async fn verify_layout_commitment(
        &self,
        chain: &str,
        contract: &str,
        commitment: &[u8; 32],
    ) -> ApiResult<bool> {
        (**self)
            .verify_layout_commitment(chain, contract, commitment)
            .await
    }

    async fn register_view(&self, definition: ViewDefinition) -> ApiResult<()> {
        (**self).register_view(definition).await
    }

    async fn get_view(&self, view_id: &str) -> ApiResult<Option<ViewDefinition>> {
        (**self).get_view(view_id).await
    }

    async fn update_view(&self, definition: ViewDefinition) -> ApiResult<()> {
        (**self).update_view(definition).await
    }

    async fn delete_view(&self, view_id: &str) -> ApiResult<()> {
        (**self).delete_view(view_id).await
    }

    async fn list_views_for_contract(
        &self,
        contract_address: &str,
    ) -> ApiResult<Vec<ViewDefinition>> {
        (**self).list_views_for_contract(contract_address).await
    }

    async fn query_view(
        &self,
        view_id: &str,
        chain: &str,
        contract: &str,
        block_height: u64,
        include_proofs: bool,
    ) -> ApiResult<ViewQueryResult> {
        (**self)
            .query_view(view_id, chain, contract, block_height, include_proofs)
            .await
    }

    async fn get_storage_proof(
        &self,
        chain: &str,
        contract: &str,
        block_height: u64,
        storage_key: &[u8; 32],
    ) -> ApiResult<Option<StorageProofValue>> {
        (**self)
            .get_storage_proof(chain, contract, block_height, storage_key)
            .await
    }

    async fn get_latest_block_number(&self, chain: &str) -> ApiResult<Option<u64>> {
        (**self).get_latest_block_number(chain).await
    }

    async fn get_anchor_idl(&self, program_id: &str) -> ApiResult<Option<String>> {
        (**self).get_anchor_idl(program_id).await
    }
}
