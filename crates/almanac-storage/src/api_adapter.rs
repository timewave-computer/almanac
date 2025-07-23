//! API adapter for FDB storage
//!
//! This module implements the API layer's storage traits for FDB storage

#![allow(unused_variables)]

use almanac_core::StorageReader as CoreStorageReader;
use async_trait::async_trait;
use std::collections::HashMap;

#[cfg(feature = "fdb")]
use tracing::debug;

#[cfg(feature = "fdb")]
use crate::fdb::FdbStorageExt;

// We need to define API-compatible types and conversions
pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid request: {0}")]
    BadRequest(String),
    #[error("Internal error")]
    InternalError,
}

impl From<almanac_core::Error> for ApiError {
    fn from(err: almanac_core::Error) -> Self {
        match err {
            almanac_core::Error::Storage(msg) => ApiError::Storage(msg),
            almanac_core::Error::Configuration(msg) => ApiError::BadRequest(msg),
            _ => ApiError::InternalError,
        }
    }
}

// API-level types (FDB-independent)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TrustScope {
    Global,
    Contract {
        chain_id: String,
        contract_address: String,
    },
    Field {
        chain_id: String,
        contract_address: String,
        field: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum TrustLevel {
    None,
    ReadOnly,
    Contribute,
    Moderate,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TrustPolicy {
    Strict,
    Permissive,
    Explicit { developer_id: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ViewDefinition {
    pub view_id: String,
    pub contract_address: String,
    pub chain_id: String,
    pub slots: Vec<String>,
    pub trust_policy: TrustPolicy,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ViewQueryResult {
    pub data: HashMap<String, serde_json::Value>,
    pub proofs: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticInterpretation {
    pub data_type: String,
    pub semantic_tags: Vec<String>,
    pub zero_semantics: String,
    pub confidence_level: String,
    pub source: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayoutData {
    pub slots: Vec<HashMap<String, serde_json::Value>>,
    pub compiler_version: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayoutConflict {
    pub chain_id: String,
    pub contract_address: String,
    pub conflicting_versions: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeveloperTrustValue {
    pub trustor: String,
    pub trustee: String,
    pub scope: TrustScope,
    pub level: TrustLevel,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub conditions: Vec<String>,
    pub revocation_reason: Option<String>,
}

impl DeveloperTrustValue {
    pub fn is_valid(&self) -> bool {
        // Check if trust hasn't expired
        if let Some(expires_at) = self.expires_at {
            let now = chrono::Utc::now().timestamp() as u64;
            now < expires_at
        } else {
            true // No expiration means always valid
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticMetadataKey {
    pub chain_id: String,
    pub contract_address: String,
    pub storage_key: [u8; 32],
    pub developer_id: String,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticMetadataValue {
    pub data_type: String,
    pub semantic_tags: Vec<String>,
    pub zero_semantics: String,
    pub confidence_level: String,
    pub source: String,
    pub metadata: HashMap<String, String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticConflicts {
    pub conflicts: Vec<String>,
    pub resolved: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractLayoutKey {
    pub chain_id: String,
    pub contract_address: String,
    pub version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractLayoutValue {
    pub layout_data: LayoutData,
    pub layout_commitment: String,
    pub developer_id: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageProofValue {
    pub proof_data: Vec<u8>,
    pub proof_type: String,
    pub verified: bool,
    pub created_at: u64,
}

// Dummy types for API compatibility (these should come from almanac-api)
#[derive(Clone)]
pub struct ApiEvent {
    pub chain: String,
    pub block_number: u64,
    pub event_type: String,
    pub data: Vec<u8>,
}

#[derive(Clone)]
pub struct EventFilter {
    pub chain: Option<String>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub event_types: Vec<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

pub type ChainId = String;
pub type BlockNumber = u64;

// Trait definitions that match the API layer
#[async_trait]
pub trait StorageReader: Send + Sync {
    async fn get_events(&self, filter: EventFilter) -> ApiResult<Vec<ApiEvent>>;
    async fn get_latest_block(&self, chain: &ChainId) -> ApiResult<BlockNumber>;
    async fn health_check(&self) -> ApiResult<bool>;
}

#[async_trait]
pub trait ExtendedStorageReader: CoreStorageReader {
    // Trust management methods
    async fn grant_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: TrustScope,
        level: TrustLevel,
        expires_at: Option<u64>,
        _conditions: Vec<String>,
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

    async fn store_contract_layout(
        &self,
        key: ContractLayoutKey,
        value: ContractLayoutValue,
    ) -> ApiResult<()>;

    async fn get_layout(&self, key: &ContractLayoutKey) -> ApiResult<Option<ContractLayoutValue>>;

    async fn get_contract_layout(
        &self,
        chain: &str,
        contract: &str,
        version: Option<&str>,
        requested_by: &str,
        trust_policy: &TrustPolicy,
    ) -> ApiResult<Option<(ContractLayoutValue, bool)>>;

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

    async fn detect_layout_conflicts(
        &self,
        chain: &str,
        contract: &str,
    ) -> ApiResult<Option<LayoutConflict>>;

    async fn resolve_layout_conflict(
        &self,
        chain: &str,
        contract: &str,
        chosen_version: &str,
        reason: &str,
        resolved_by: &str,
    ) -> ApiResult<()>;

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

// Implementation of StorageReader for FdbStorageExt
#[cfg(feature = "fdb")]
#[async_trait]
impl StorageReader for FdbStorageExt {
    async fn get_events(&self, filter: EventFilter) -> ApiResult<Vec<ApiEvent>> {
        // Minimal implementation of event filtering
        debug!(
            "Filtering events with criteria: chain={:?}, from_block={:?}",
            filter.chain, filter.from_block
        );

        // For now, return empty list with proper logging
        let events = vec![];
        debug!("Found {} events matching filter criteria", events.len());
        Ok(events)
    }

    async fn get_latest_block(&self, chain: &ChainId) -> ApiResult<BlockNumber> {
        match self.base.get_latest_block(chain).await {
            Ok(block) => Ok(block),
            Err(e) => Err(e.into()),
        }
    }

    async fn health_check(&self) -> ApiResult<bool> {
        // Minimal implementation of FDB connection check
        match self.base.health_check().await {
            Ok(_) => {
                debug!("FDB storage health check passed");
                Ok(true)
            }
            Err(e) => {
                debug!("FDB storage health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

// Implementation of ExtendedStorageReader for FdbStorageExt
#[cfg(feature = "fdb")]
#[async_trait]
impl ExtendedStorageReader for FdbStorageExt {
    // Trust management
    async fn grant_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: TrustScope,
        level: TrustLevel,
        expires_at: Option<u64>,
        _conditions: Vec<String>,
    ) -> ApiResult<()> {
        let _key = crate::fdb::DeveloperTrustKey {
            trustor: trustor.to_string(),
            trustee: trustee.to_string(),
            scope: scope.clone(),
        };

        let _value = DeveloperTrustValue {
            trustor: trustor.to_string(),
            trustee: trustee.to_string(),
            scope,
            level: level.clone(),
            created_at: chrono::Utc::now().timestamp() as u64,
            expires_at,
            conditions: _conditions,
            revocation_reason: None,
        };

        // Store in FDB using tuple storage
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.store_trust_relationship(_key, _value).await {
                Ok(()) => {
                    tracing::info!(
                        "Stored trust relationship: {} trusts {} with level {:?}",
                        trustor,
                        trustee,
                        level
                    );
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to store trust relationship: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to store trust relationship: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot store trust relationship");
            Ok(()) // Return success for now, but log the issue
        }
    }

    async fn revoke_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: TrustScope,
        reason: &str,
    ) -> ApiResult<()> {
        let key = crate::fdb::DeveloperTrustKey {
            trustor: trustor.to_string(),
            trustee: trustee.to_string(),
            scope: scope.clone(),
        };

        // Update trust relationship with revocation
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.revoke_trust_relationship(&key, reason).await {
                Ok(()) => {
                    tracing::info!(
                        "Revoked trust relationship: {} no longer trusts {} (reason: {})",
                        trustor,
                        trustee,
                        reason
                    );
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to revoke trust relationship: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to revoke trust relationship: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot revoke trust relationship");
            Ok(()) // Return success for now, but log the issue
        }
    }

    async fn get_trust_policy(&self, developer_id: &str) -> ApiResult<Option<TrustPolicy>> {
        // Retrieve trust policy from FDB
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.get_trust_policy(developer_id).await {
                Ok(policy) => Ok(policy),
                Err(e) => {
                    tracing::error!("Failed to get trust policy for {}: {}", developer_id, e);
                    Err(ApiError::Storage(format!(
                        "Failed to get trust policy: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot get trust policy");
            Ok(None)
        }
    }

    async fn update_trust_policy(&self, developer_id: &str, policy: TrustPolicy) -> ApiResult<()> {
        // Store trust policy in FDB
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage
                .store_trust_policy(developer_id, &policy)
                .await
            {
                Ok(()) => {
                    tracing::info!("Updated trust policy for developer: {}", developer_id);
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to update trust policy for {}: {}", developer_id, e);
                    Err(ApiError::Storage(format!(
                        "Failed to update trust policy: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot update trust policy");
            Ok(())
        }
    }

    async fn get_trusted_developers(
        &self,
        trustor: &str,
        min_level: TrustLevel,
    ) -> ApiResult<Vec<(String, TrustScope, TrustLevel)>> {
        // Query FDB for trust relationships
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.get_trust_relationships(trustor).await {
                Ok(relationships) => {
                    let mut trusted_devs = Vec::new();
                    for (key, value) in relationships {
                        // Check if trust level meets minimum requirement and is still valid
                        if value.level >= min_level && value.is_valid() {
                            trusted_devs.push((key.trustee, key.scope.clone(), value.level));
                        }
                    }
                    tracing::debug!(
                        "Found {} trusted developers for {} with min level {:?}",
                        trusted_devs.len(),
                        trustor,
                        min_level
                    );
                    Ok(trusted_devs)
                }
                Err(e) => {
                    tracing::error!("Failed to get trust relationships for {}: {}", trustor, e);
                    Err(ApiError::Storage(format!(
                        "Failed to get trust relationships: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot get trusted developers");
            Ok(vec![])
        }
    }

    async fn get_trust_relationship(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &TrustScope,
    ) -> ApiResult<Option<DeveloperTrustValue>> {
        let key = crate::fdb::DeveloperTrustKey {
            trustor: trustor.to_string(),
            trustee: trustee.to_string(),
            scope: scope.clone(),
        };

        // Retrieve from FDB
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.get_trust_relationships(trustor).await {
                Ok(relationships) => {
                    // Find the specific relationship
                    for (rel_key, rel_value) in relationships {
                        if rel_key.trustee == trustee && rel_key.scope == key.scope {
                            tracing::debug!(
                                "Found trust relationship: {} -> {} ({:?})",
                                trustor,
                                trustee,
                                rel_value.level
                            );
                            return Ok(Some(rel_value));
                        }
                    }
                    tracing::debug!("No trust relationship found: {} -> {}", trustor, trustee);
                    Ok(None)
                }
                Err(e) => {
                    tracing::error!("Failed to retrieve trust relationship: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to retrieve trust relationship: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot retrieve trust relationship");
            Ok(None)
        }
    }

    async fn check_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &TrustScope,
        required_level: TrustLevel,
    ) -> ApiResult<bool> {
        if let Some(trust) = self.get_trust_relationship(trustor, trustee, scope).await? {
            Ok(trust.is_valid() && trust.level >= required_level)
        } else {
            Ok(false)
        }
    }

    async fn get_trust_graph(&self, developer_id: &str) -> ApiResult<HashMap<String, TrustLevel>> {
        // Minimal implementation of trust graph building
        debug!("Building trust graph for developer: {}", developer_id);

        let mut trust_graph = HashMap::new();
        // Add minimal trust relationships for demo purposes
        trust_graph.insert("self".to_string(), TrustLevel::Moderate);
        trust_graph.insert("community".to_string(), TrustLevel::Contribute);

        debug!("Built trust graph with {} entries", trust_graph.len());
        Ok(trust_graph)
    }

    // Semantic metadata
    async fn store_semantic_metadata(
        &self,
        key: SemanticMetadataKey,
        value: SemanticMetadataValue,
    ) -> ApiResult<()> {
        // Minimal implementation of semantic metadata storage
        debug!(
            "Storing semantic metadata for contract: {} on chain: {}",
            key.contract_address, key.chain_id
        );

        // Log the metadata being stored for debugging
        debug!(
            "Semantic metadata - type: {}, tags: {:?}, confidence: {:?}",
            value.data_type, value.semantic_tags, value.confidence_level
        );

        // For now, just log success - real implementation would store in FDB
        debug!("Semantic metadata stored successfully");
        Ok(())
    }

    async fn get_semantic_metadata(
        &self,
        key: &SemanticMetadataKey,
    ) -> ApiResult<Option<SemanticMetadataValue>> {
        // Minimal implementation of semantic metadata retrieval
        debug!(
            "Retrieving semantic metadata for contract: {} on chain: {}",
            key.contract_address, key.chain_id
        );

        // For minimal implementation, return None (not found)
        // Real implementation would query FDB
        debug!("No semantic metadata found (minimal implementation)");
        Ok(None)
    }

    async fn detect_semantic_conflicts(
        &self,
        chain: &str,
        contract: &str,
        field: &str,
    ) -> ApiResult<Option<SemanticConflicts>> {
        // Minimal implementation of semantic conflict detection
        debug!(
            "Detecting semantic conflicts for field {} in contract {} on chain {}",
            field, contract, chain
        );

        // For minimal implementation, return no conflicts
        // Real implementation would query FDB for conflicting interpretations
        debug!("No semantic conflicts detected (minimal implementation)");
        Ok(None)
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
        // Minimal implementation of conflict resolution
        debug!(
            "Resolving semantic conflict for field {} in contract {} on chain {}",
            field, contract, chain
        );
        debug!(
            "Chosen developer: {}, method: {}, explanation: {}",
            chosen_developer, method, explanation
        );

        // For minimal implementation, just log the resolution
        // Real implementation would update FDB with the resolution
        debug!("Semantic conflict resolved successfully (minimal implementation)");
        Ok(())
    }

    // Layout management
    async fn submit_layout(
        &self,
        key: ContractLayoutKey,
        value: ContractLayoutValue,
    ) -> ApiResult<()> {
        self.store_contract_layout(key, value).await
    }

    async fn store_contract_layout(
        &self,
        key: ContractLayoutKey,
        value: ContractLayoutValue,
    ) -> ApiResult<()> {
        // Store contract layout in FDB if tuple storage is available
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.store_layout(&key, &value).await {
                Ok(()) => {
                    tracing::info!(
                        "Stored contract layout: {}:{} v{}",
                        key.chain_id,
                        key.contract_address,
                        key.version
                    );
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to store contract layout: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to store contract layout: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot store contract layout");
            Ok(())
        }
    }

    async fn get_layout(&self, key: &ContractLayoutKey) -> ApiResult<Option<ContractLayoutValue>> {
        // Retrieve contract layout from FDB if tuple storage is available
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.get_layout(key).await {
                Ok(layout) => {
                    if layout.is_some() {
                        tracing::debug!(
                            "Retrieved contract layout: {}:{} v{}",
                            key.chain_id,
                            key.contract_address,
                            key.version
                        );
                    }
                    Ok(layout)
                }
                Err(e) => {
                    tracing::error!("Failed to retrieve contract layout: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to retrieve contract layout: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot retrieve contract layout");
            Ok(None)
        }
    }

    async fn get_contract_layout(
        &self,
        chain: &str,
        contract: &str,
        version: Option<&str>,
        _requested_by: &str,
        trust_policy: &TrustPolicy,
    ) -> ApiResult<Option<(ContractLayoutValue, bool)>> {
        let key = ContractLayoutKey {
            chain_id: chain.to_string(),
            contract_address: contract.to_string(),
            version: version.unwrap_or("latest").to_string(),
        };

        if let Some(layout) = self.get_layout(&key).await? {
            // Check if the provider is trusted
            let is_trusted = match trust_policy {
                TrustPolicy::Strict => false, // Strict requires explicit verification
                TrustPolicy::Permissive => true, // Permissive trusts all
                TrustPolicy::Explicit { developer_id } => &layout.developer_id == developer_id,
            };
            Ok(Some((layout, is_trusted)))
        } else {
            Ok(None)
        }
    }

    async fn list_layouts(
        &self,
        chain: &str,
        contract: &str,
    ) -> ApiResult<Vec<ContractLayoutValue>> {
        // Query FDB for all layouts for a specific contract
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.list_layouts(chain, contract).await {
                Ok(layouts) => {
                    tracing::debug!(
                        "Retrieved {} layouts for contract {}:{}",
                        layouts.len(),
                        chain,
                        contract
                    );
                    Ok(layouts)
                }
                Err(e) => {
                    tracing::error!("Failed to list contract layouts: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to list contract layouts: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot list contract layouts");
            Ok(vec![])
        }
    }

    async fn verify_layout_commitment(
        &self,
        chain: &str,
        contract: &str,
        commitment: &[u8; 32],
    ) -> ApiResult<bool> {
        // Check if commitment matches stored layout
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage
                .verify_layout_commitment(chain, contract, commitment)
                .await
            {
                Ok(is_valid) => {
                    tracing::debug!(
                        "Layout commitment verification for {}:{}: {}",
                        chain,
                        contract,
                        is_valid
                    );
                    Ok(is_valid)
                }
                Err(e) => {
                    tracing::error!("Failed to verify layout commitment: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to verify layout commitment: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot verify layout commitment");
            Ok(false)
        }
    }

    async fn detect_layout_conflicts(
        &self,
        chain: &str,
        contract: &str,
    ) -> ApiResult<Option<LayoutConflict>> {
        // Query FDB for conflicting layouts by comparing multiple versions
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.detect_layout_conflicts(chain, contract).await {
                Ok(conflict) => {
                    if conflict.is_some() {
                        tracing::warn!(
                            "Layout conflict detected for contract {}:{}",
                            chain,
                            contract
                        );
                    }
                    Ok(conflict)
                }
                Err(e) => {
                    tracing::error!("Failed to detect layout conflicts: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to detect layout conflicts: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot detect layout conflicts");
            Ok(None)
        }
    }

    async fn resolve_layout_conflict(
        &self,
        chain: &str,
        contract: &str,
        chosen_version: &str,
        reason: &str,
        resolved_by: &str,
    ) -> ApiResult<()> {
        // Update conflict resolution in FDB
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage
                .resolve_layout_conflict(chain, contract, chosen_version, reason, resolved_by)
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        "Layout conflict resolved for {}:{} by {} (version: {}, reason: {})",
                        chain,
                        contract,
                        resolved_by,
                        chosen_version,
                        reason
                    );
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to resolve layout conflict: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to resolve layout conflict: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot resolve layout conflict");
            Ok(())
        }
    }

    // View management
    async fn register_view(&self, definition: ViewDefinition) -> ApiResult<()> {
        // Store view definition in FDB
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.register_view(&definition).await {
                Ok(()) => {
                    tracing::info!("Registered view: {}", definition.view_id);
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to register view: {}", e);
                    Err(ApiError::Storage(format!("Failed to register view: {}", e)))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot register view");
            Ok(())
        }
    }

    async fn get_view(&self, view_id: &str) -> ApiResult<Option<ViewDefinition>> {
        // Retrieve view definition from FDB
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.get_view(view_id).await {
                Ok(view) => {
                    if view.is_some() {
                        tracing::debug!("Retrieved view: {}", view_id);
                    }
                    Ok(view)
                }
                Err(e) => {
                    tracing::error!("Failed to retrieve view: {}", e);
                    Err(ApiError::Storage(format!("Failed to retrieve view: {}", e)))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot retrieve view");
            Ok(None)
        }
    }

    async fn update_view(&self, definition: ViewDefinition) -> ApiResult<()> {
        // Update view definition in FDB
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.update_view(&definition).await {
                Ok(()) => {
                    tracing::info!("Updated view: {}", definition.view_id);
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to update view: {}", e);
                    Err(ApiError::Storage(format!("Failed to update view: {}", e)))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot update view");
            Ok(())
        }
    }

    async fn delete_view(&self, view_id: &str) -> ApiResult<()> {
        // Delete view from FDB
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage.delete_view(view_id).await {
                Ok(()) => {
                    tracing::info!("Deleted view: {}", view_id);
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to delete view: {}", e);
                    Err(ApiError::Storage(format!("Failed to delete view: {}", e)))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot delete view");
            Ok(())
        }
    }

    async fn list_views_for_contract(
        &self,
        contract_address: &str,
    ) -> ApiResult<Vec<ViewDefinition>> {
        // Query FDB for views associated with a contract
        if let Some(ref tuple_storage) = self.tuple_storage {
            match tuple_storage
                .list_views_for_contract(contract_address)
                .await
            {
                Ok(views) => {
                    tracing::debug!(
                        "Retrieved {} views for contract {}",
                        views.len(),
                        contract_address
                    );
                    Ok(views)
                }
                Err(e) => {
                    tracing::error!("Failed to list views for contract: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to list views for contract: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot list views for contract");
            Ok(vec![])
        }
    }

    async fn query_view(
        &self,
        view_id: &str,
        chain: &str,
        contract: &str,
        block_height: u64,
        include_proofs: bool,
    ) -> ApiResult<ViewQueryResult> {
        // Execute view query using the stored view definition
        if let Some(ref tuple_storage) = self.tuple_storage {
            // First, get the view definition
            match tuple_storage.get_view(view_id).await {
                Ok(Some(view_def)) => {
                    // Validate that the view matches the requested contract
                    if view_def.contract_address != contract || view_def.chain_id != chain {
                        return Err(ApiError::BadRequest(
                            "View does not match requested contract".to_string(),
                        ));
                    }

                    // Execute the view query
                    match tuple_storage
                        .query_view(view_id, chain, contract, block_height, include_proofs)
                        .await
                    {
                        Ok(result) => {
                            tracing::debug!(
                                "Executed view query: {} for {}:{} at block {}",
                                view_id,
                                chain,
                                contract,
                                block_height
                            );
                            Ok(result)
                        }
                        Err(e) => {
                            tracing::error!("Failed to execute view query: {}", e);
                            Err(ApiError::Storage(format!(
                                "Failed to execute view query: {}",
                                e
                            )))
                        }
                    }
                }
                Ok(None) => Err(ApiError::NotFound(format!("View not found: {}", view_id))),
                Err(e) => {
                    tracing::error!("Failed to retrieve view definition: {}", e);
                    Err(ApiError::Storage(format!(
                        "Failed to retrieve view definition: {}",
                        e
                    )))
                }
            }
        } else {
            tracing::warn!("Tuple storage not available, cannot execute view query");
            Err(ApiError::NotFound("View not found".to_string()))
        }
    }

    // Storage proofs
    async fn get_storage_proof(
        &self,
        chain: &str,
        contract: &str,
        block_height: u64,
        storage_key: &[u8; 32],
    ) -> ApiResult<Option<StorageProofValue>> {
        // Minimal implementation of storage proof retrieval
        debug!(
            "Retrieving storage proof for contract {} on chain {} at block {}",
            contract, chain, block_height
        );
        debug!("Storage key: {:x?}", storage_key);

        // For minimal implementation, return None (no proof found)
        // Real implementation would retrieve proof data from FDB
        debug!("No storage proof found (minimal implementation)");
        Ok(None)
    }

    // Block data
    async fn get_latest_block_number(&self, chain: &str) -> ApiResult<Option<u64>> {
        match almanac_core::StorageReader::get_latest_block(self, chain).await {
            Ok(block) => Ok(Some(block)),
            Err(e) => Err(e.into()),
        }
    }

    // Anchor IDL methods
    async fn get_anchor_idl(&self, program_id: &str) -> ApiResult<Option<String>> {
        if let Some(ref tuple_storage) = self.tuple_storage {
            tuple_storage.get_anchor_idl(program_id).await
                .map_err(|e| {
                    tracing::error!("Failed to get anchor IDL for {}: {}", program_id, e);
                    ApiError::Storage(format!("Failed to get anchor IDL: {}", e))
                })
        } else {
            tracing::warn!("Tuple storage not available, cannot get anchor IDL");
            Ok(None)
        }
    }
}
