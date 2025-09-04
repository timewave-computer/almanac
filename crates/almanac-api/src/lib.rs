/// HTTP API server for Almanac
///
/// This crate provides a clean, modular HTTP API server built on Axum
/// with minimal dependencies and clear separation of concerns.
// Core types from api-core
pub mod types;

// API modules
pub mod auth;
pub mod config;
pub mod error;
pub mod handlers;
pub mod health;
pub mod middleware;
pub mod openapi;
pub mod rate_limit;
pub mod routes;
pub mod server;
pub mod state;
pub mod storage_ext;
pub mod memory_subscription_manager;

#[cfg(test)]
pub mod test_helpers;

pub use error::HttpError;
pub use server::{ApiServer, ServerConfig};
pub use state::AppState;

use crate::types::storage::MemoryStorageReader;
use std::net::SocketAddr;
use std::sync::Arc;

#[cfg(feature = "fdb")]
use almanac_storage::api_adapter::{
    ExtendedStorageReader as ExtendedStorageReaderTrait, StorageReader as StorageReaderTrait,
};

/// Application configuration loaded from environment
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub metrics_port: u16,
    pub database_url: Option<String>,
    pub admin_api_key: Option<String>,
    pub rate_limit_per_minute: u64,
    pub enable_cors: bool,
    pub cors_origins: Vec<String>,
    pub storage_backend: StorageBackend,
    pub fdb_cluster_file: Option<String>,
    pub fdb_namespace: String,
}

#[derive(Debug, Clone)]
pub enum StorageBackend {
    Memory,
    FoundationDB,
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let storage_backend = match std::env::var("STORAGE_BACKEND").as_deref() {
            Ok("foundationdb") | Ok("fdb") => StorageBackend::FoundationDB,
            _ => StorageBackend::Memory,
        };

        Ok(Self {
            host: std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("API_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            metrics_port: std::env::var("METRICS_PORT")
                .unwrap_or_else(|_| "9090".to_string())
                .parse()?,
            database_url: std::env::var("DATABASE_URL").ok(),
            admin_api_key: std::env::var("ADMIN_API_KEY").ok(),
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "100".to_string())
                .parse()?,
            enable_cors: std::env::var("ENABLE_CORS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "*".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            storage_backend,
            fdb_cluster_file: std::env::var("FDB_CLUSTER_FILE").ok(),
            fdb_namespace: std::env::var("FDB_NAMESPACE").unwrap_or_else(|_| "almanac".to_string()),
        })
    }
}

/// Run the API server with the given configuration
pub async fn run_server(config: AppConfig) -> anyhow::Result<()> {
    use crate::storage_ext::ExtendedStorageReader;

    // Create storage based on configuration
    let storage: Arc<dyn ExtendedStorageReader> = match config.storage_backend {
        StorageBackend::Memory => {
            // For now, use a wrapper that implements ExtendedStorageReader
            // In production, you'd have a proper memory implementation
            Arc::new(MemoryExtendedStorage::new())
        }
        StorageBackend::FoundationDB => {
            #[cfg(feature = "fdb")]
            {
                use almanac_storage::fdb::{FdbConfig, FdbStorageExt};

                let fdb_config = FdbConfig {
                    cluster_file: config.fdb_cluster_file.clone(),
                    namespace: config.fdb_namespace.clone(),
                    connection_timeout: 30,
                    max_retries: 3,
                };

                let fdb_storage = FdbStorageExt::new(fdb_config)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create FDB storage: {}", e))?;

                Arc::new(FdbStorageAdapter::new(fdb_storage))
            }
            #[cfg(not(feature = "fdb"))]
            {
                anyhow::bail!("FoundationDB support not compiled in. Enable the 'fdb' feature.");
            }
        }
    };

    // Update AppState to use ExtendedStorageReader
    let mut app_state = AppState::new(storage);

    // Add subscription manager based on storage backend
    match config.storage_backend {
        StorageBackend::Memory => {
            // Use MemorySubscriptionManager for memory backend
            use crate::memory_subscription_manager::MemorySubscriptionManager;
            app_state = app_state.with_subscription_manager(MemorySubscriptionManager::new());
        }
        StorageBackend::FoundationDB => {
            // For FDB, we would need to create FdbAccountSubscriptionManager
            // but that requires more complex setup with TupleFdbStorage
            // For now, use memory manager even for FDB backend
            use crate::memory_subscription_manager::MemorySubscriptionManager;
            app_state = app_state.with_subscription_manager(MemorySubscriptionManager::new());
        }
    }

    // Create server config
    let server_config = ServerConfig {
        addr: SocketAddr::from(([0, 0, 0, 0], config.port)),
        graceful_shutdown: true,
        request_timeout_secs: 30,
        auth_config: None,
    };

    // Create and run server
    let server = ApiServer::with_state(server_config, app_state);
    server.run().await
}

// Temporary adapter types until we have proper implementations
struct MemoryExtendedStorage {
    base: MemoryStorageReader,
}

impl MemoryExtendedStorage {
    fn new() -> Self {
        Self {
            base: MemoryStorageReader::new(),
        }
    }
}

// Implement ExtendedStorageReader for MemoryExtendedStorage with stub methods
#[async_trait::async_trait]
impl crate::types::StorageReader for MemoryExtendedStorage {
    async fn get_events(
        &self,
        filter: crate::types::EventFilter,
    ) -> crate::types::ApiResult<Vec<crate::types::ApiEvent>> {
        self.base.get_events(filter).await
    }

    async fn get_latest_block(
        &self,
        chain: &crate::types::ChainId,
    ) -> crate::types::ApiResult<crate::types::BlockNumber> {
        self.base.get_latest_block(chain).await
    }

    async fn health_check(&self) -> crate::types::ApiResult<bool> {
        self.base.health_check().await
    }
}

// Implement ExtendedStorageReader trait with stub methods for now
#[async_trait::async_trait]
impl crate::storage_ext::ExtendedStorageReader for MemoryExtendedStorage {
    // Trust management methods
    async fn grant_trust(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: crate::storage_ext::TrustScope,
        _level: crate::storage_ext::TrustLevel,
        _expires_at: Option<u64>,
        _conditions: Vec<String>,
    ) -> crate::types::ApiResult<()> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    async fn revoke_trust(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: crate::storage_ext::TrustScope,
        _reason: &str,
    ) -> crate::types::ApiResult<()> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    async fn get_trust_policy(
        &self,
        _developer_id: &str,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::TrustPolicy>> {
        Ok(None)
    }

    async fn update_trust_policy(
        &self,
        _developer_id: &str,
        _policy: crate::storage_ext::TrustPolicy,
    ) -> crate::types::ApiResult<()> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    async fn get_trusted_developers(
        &self,
        _trustor: &str,
        _min_level: crate::storage_ext::TrustLevel,
    ) -> crate::types::ApiResult<
        Vec<(
            String,
            crate::storage_ext::TrustScope,
            crate::storage_ext::TrustLevel,
        )>,
    > {
        Ok(vec![])
    }

    async fn get_trust_relationship(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: &crate::storage_ext::TrustScope,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::DeveloperTrustValue>> {
        Ok(None)
    }

    async fn check_trust(
        &self,
        _trustor: &str,
        _trustee: &str,
        _scope: &crate::storage_ext::TrustScope,
        _required_level: crate::storage_ext::TrustLevel,
    ) -> crate::types::ApiResult<bool> {
        Ok(false)
    }

    async fn get_trust_graph(
        &self,
        _developer_id: &str,
    ) -> crate::types::ApiResult<std::collections::HashMap<String, crate::storage_ext::TrustLevel>>
    {
        Ok(std::collections::HashMap::new())
    }

    // Semantic metadata methods
    async fn store_semantic_metadata(
        &self,
        _key: crate::storage_ext::SemanticMetadataKey,
        _value: crate::storage_ext::SemanticMetadataValue,
    ) -> crate::types::ApiResult<()> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    async fn get_semantic_metadata(
        &self,
        _key: &crate::storage_ext::SemanticMetadataKey,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::SemanticMetadataValue>> {
        Ok(None)
    }

    async fn detect_semantic_conflicts(
        &self,
        _chain: &str,
        _contract: &str,
        _field: &str,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::SemanticConflicts>> {
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
    ) -> crate::types::ApiResult<()> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    // Layout management methods
    async fn submit_layout(
        &self,
        _key: crate::storage_ext::ContractLayoutKey,
        _value: crate::storage_ext::ContractLayoutValue,
    ) -> crate::types::ApiResult<()> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    async fn get_layout(
        &self,
        _key: &crate::storage_ext::ContractLayoutKey,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::ContractLayoutValue>> {
        Ok(None)
    }

    async fn list_layouts(
        &self,
        _chain: &str,
        _contract: &str,
    ) -> crate::types::ApiResult<Vec<crate::storage_ext::ContractLayoutValue>> {
        Ok(vec![])
    }

    async fn verify_layout_commitment(
        &self,
        _chain: &str,
        _contract: &str,
        _commitment: &[u8; 32],
    ) -> crate::types::ApiResult<bool> {
        Ok(false)
    }

    // View management methods
    async fn register_view(
        &self,
        _definition: crate::storage_ext::ViewDefinition,
    ) -> crate::types::ApiResult<()> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    async fn get_view(
        &self,
        _view_id: &str,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::ViewDefinition>> {
        Ok(None)
    }

    async fn update_view(
        &self,
        _definition: crate::storage_ext::ViewDefinition,
    ) -> crate::types::ApiResult<()> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    async fn delete_view(&self, _view_id: &str) -> crate::types::ApiResult<()> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    async fn list_views_for_contract(
        &self,
        _contract_address: &str,
    ) -> crate::types::ApiResult<Vec<crate::storage_ext::ViewDefinition>> {
        Ok(vec![])
    }

    async fn query_view(
        &self,
        _view_id: &str,
        _chain: &str,
        _contract: &str,
        _block_height: u64,
        _include_proofs: bool,
    ) -> crate::types::ApiResult<crate::storage_ext::ViewQueryResult> {
        Err(crate::types::ApiError::InternalError(
            "Not implemented for memory storage".to_string(),
        ))
    }

    // Storage proof methods
    async fn get_storage_proof(
        &self,
        _chain: &str,
        _contract: &str,
        _block_height: u64,
        _storage_key: &[u8; 32],
    ) -> crate::types::ApiResult<Option<crate::storage_ext::StorageProofValue>> {
        Ok(None)
    }

    // Block data methods
    async fn get_latest_block_number(&self, _chain: &str) -> crate::types::ApiResult<Option<u64>> {
        Ok(None)
    }

    async fn get_anchor_idl(&self, _program_id: &str) -> crate::types::ApiResult<Option<String>> {
        Ok(None)
    }
}

#[cfg(feature = "fdb")]
struct FdbStorageAdapter {
    storage: almanac_storage::fdb::FdbStorageExt,
}

#[cfg(feature = "fdb")]
impl FdbStorageAdapter {
    fn new(storage: almanac_storage::fdb::FdbStorageExt) -> Self {
        Self { storage }
    }

    /// Convert storage errors to API errors
    fn convert_error(err: almanac_storage::api_adapter::ApiError) -> crate::types::ApiError {
        crate::types::ApiError::StorageError(err.to_string())
    }

    /// Convert API DeveloperTrustValue to storage DeveloperTrustValue
    #[allow(dead_code)]
    fn convert_developer_trust_value_to_storage(
        api_val: crate::storage_ext::DeveloperTrustValue,
    ) -> almanac_storage::api_adapter::DeveloperTrustValue {
        almanac_storage::api_adapter::DeveloperTrustValue {
            trustor: api_val.trustor,
            trustee: api_val.trustee,
            scope: api_val.scope,
            level: api_val.level,
            expires_at: api_val.expires_at,
            created_at: chrono::Utc::now().timestamp() as u64,
            conditions: vec![], // Default to empty conditions for API conversion
            revocation_reason: None, // Default to no revocation
        }
    }

    /// Convert storage DeveloperTrustValue to API DeveloperTrustValue
    fn convert_developer_trust_value_from_storage(
        storage_val: almanac_storage::api_adapter::DeveloperTrustValue,
    ) -> crate::storage_ext::DeveloperTrustValue {
        crate::storage_ext::DeveloperTrustValue {
            trustor: storage_val.trustor,
            trustee: storage_val.trustee,
            scope: storage_val.scope,
            level: storage_val.level,
            expires_at: storage_val.expires_at,
            conditions: vec![], // Storage doesn't have conditions field
        }
    }

    /// Convert API SemanticMetadataKey to storage SemanticMetadataKey
    fn convert_semantic_key_to_storage(
        api_key: crate::storage_ext::SemanticMetadataKey,
    ) -> almanac_storage::api_adapter::SemanticMetadataKey {
        almanac_storage::api_adapter::SemanticMetadataKey {
            chain_id: api_key.chain,
            contract_address: api_key.contract,
            storage_key: [0u8; 32], // Default value - API doesn't have storage_key
            developer_id: api_key.developer,
            project_id: None,
        }
    }

    /// Convert API SemanticMetadataValue to storage SemanticMetadataValue
    fn convert_semantic_value_to_storage(
        api_val: crate::storage_ext::SemanticMetadataValue,
    ) -> almanac_storage::api_adapter::SemanticMetadataValue {
        almanac_storage::api_adapter::SemanticMetadataValue {
            data_type: api_val.data_type,
            semantic_tags: api_val.semantic_tags,
            zero_semantics: api_val.zero_semantics,
            confidence_level: api_val.confidence_level,
            source: "api".to_string(),
            metadata: std::collections::HashMap::new(),
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Convert storage SemanticMetadataValue to API SemanticMetadataValue
    fn convert_semantic_value_from_storage(
        storage_val: almanac_storage::api_adapter::SemanticMetadataValue,
    ) -> crate::storage_ext::SemanticMetadataValue {
        crate::storage_ext::SemanticMetadataValue {
            data_type: storage_val.data_type,
            semantic_tags: storage_val.semantic_tags,
            zero_semantics: storage_val.zero_semantics,
            confidence_level: storage_val.confidence_level,
        }
    }

    /// Convert storage SemanticConflicts to API SemanticConflicts
    fn convert_semantic_conflicts_from_storage(
        storage_val: almanac_storage::api_adapter::SemanticConflicts,
    ) -> crate::storage_ext::SemanticConflicts {
        crate::storage_ext::SemanticConflicts {
            field: "unknown".to_string(), // Storage conflicts doesn't have field
            conflicting_interpretations: storage_val.conflicts,
        }
    }

    /// Convert storage ContractLayoutValue to API ContractLayoutValue
    fn convert_contract_layout_value_from_storage(
        storage_layout: almanac_storage::api_adapter::ContractLayoutValue,
    ) -> crate::storage_ext::ContractLayoutValue {
        crate::storage_ext::ContractLayoutValue {
            layout: serde_json::Value::Object(serde_json::Map::new()),
            commitment: [0u8; 32], // Convert layout_commitment as needed
            developer: storage_layout.developer_id,
        }
    }

    /// Convert API ContractLayoutKey to storage ContractLayoutKey
    fn convert_contract_layout_key_to_storage(
        api_key: crate::storage_ext::ContractLayoutKey,
    ) -> almanac_storage::api_adapter::ContractLayoutKey {
        almanac_storage::api_adapter::ContractLayoutKey {
            chain_id: api_key.chain,
            contract_address: api_key.contract,
            version: api_key.version,
        }
    }

    /// Convert API ContractLayoutValue to storage ContractLayoutValue
    fn convert_contract_layout_value_to_storage(
        api_val: crate::storage_ext::ContractLayoutValue,
    ) -> almanac_storage::api_adapter::ContractLayoutValue {
        almanac_storage::api_adapter::ContractLayoutValue {
            layout_data: almanac_storage::api_adapter::LayoutData {
                slots: vec![],
                compiler_version: "unknown".to_string(),
                created_at: chrono::Utc::now().timestamp() as u64,
            },
            layout_commitment: format!("{:?}", api_val.commitment),
            developer_id: api_val.developer,
            created_at: chrono::Utc::now().timestamp() as u64,
        }
    }
}

#[cfg(feature = "fdb")]
#[async_trait::async_trait]
impl crate::types::StorageReader for FdbStorageAdapter {
    async fn get_events(
        &self,
        filter: crate::types::EventFilter,
    ) -> crate::types::ApiResult<Vec<crate::types::ApiEvent>> {
        // Convert from API EventFilter to storage EventFilter
        let storage_filter = almanac_storage::api_adapter::EventFilter {
            chain: filter.chain.map(|c| c.to_string()),
            from_block: filter.from_block,
            to_block: filter.to_block,
            event_types: filter.event_types,
            offset: filter.offset.map(|o| o as u64),
            limit: filter.limit.map(|l| l as u64),
        };

        // Call storage and convert the result
        let storage_events = StorageReaderTrait::get_events(&self.storage, storage_filter)
            .await
            .map_err(Self::convert_error)?;

        // Convert storage events to API events
        let api_events = storage_events
            .into_iter()
            .map(|storage_event| {
                crate::types::ApiEvent::new(
                    storage_event.chain,
                    storage_event.block_number,
                    storage_event.event_type,
                    storage_event.data,
                )
            })
            .collect();

        Ok(api_events)
    }

    async fn get_latest_block(
        &self,
        chain: &crate::types::ChainId,
    ) -> crate::types::ApiResult<crate::types::BlockNumber> {
        let chain_str = chain.as_str();
        StorageReaderTrait::get_latest_block(&self.storage, &chain_str.to_string())
            .await
            .map_err(Self::convert_error)
    }

    async fn health_check(&self) -> crate::types::ApiResult<bool> {
        StorageReaderTrait::health_check(&self.storage)
            .await
            .map_err(Self::convert_error)
    }
}

#[cfg(feature = "fdb")]
#[async_trait::async_trait]
impl crate::storage_ext::ExtendedStorageReader for FdbStorageAdapter {
    async fn grant_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: crate::storage_ext::TrustScope,
        level: crate::storage_ext::TrustLevel,
        expires_at: Option<u64>,
        conditions: Vec<String>,
    ) -> crate::types::ApiResult<()> {
        ExtendedStorageReaderTrait::grant_trust(
            &self.storage,
            trustor,
            trustee,
            scope,
            level,
            expires_at,
            conditions,
        )
        .await
        .map_err(Self::convert_error)
    }

    async fn revoke_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: crate::storage_ext::TrustScope,
        reason: &str,
    ) -> crate::types::ApiResult<()> {
        self.storage
            .revoke_trust(trustor, trustee, scope, reason)
            .await
            .map_err(Self::convert_error)
    }

    async fn get_trust_policy(
        &self,
        developer_id: &str,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::TrustPolicy>> {
        self.storage
            .get_trust_policy(developer_id)
            .await
            .map_err(Self::convert_error)
    }

    async fn update_trust_policy(
        &self,
        developer_id: &str,
        policy: crate::storage_ext::TrustPolicy,
    ) -> crate::types::ApiResult<()> {
        self.storage
            .update_trust_policy(developer_id, policy)
            .await
            .map_err(Self::convert_error)
    }

    async fn get_trusted_developers(
        &self,
        trustor: &str,
        min_level: crate::storage_ext::TrustLevel,
    ) -> crate::types::ApiResult<
        Vec<(
            String,
            crate::storage_ext::TrustScope,
            crate::storage_ext::TrustLevel,
        )>,
    > {
        self.storage
            .get_trusted_developers(trustor, min_level)
            .await
            .map_err(Self::convert_error)
    }

    async fn get_trust_relationship(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &crate::storage_ext::TrustScope,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::DeveloperTrustValue>> {
        let result = self
            .storage
            .get_trust_relationship(trustor, trustee, scope)
            .await
            .map_err(Self::convert_error)?;

        Ok(result.map(Self::convert_developer_trust_value_from_storage))
    }

    async fn check_trust(
        &self,
        trustor: &str,
        trustee: &str,
        scope: &crate::storage_ext::TrustScope,
        required_level: crate::storage_ext::TrustLevel,
    ) -> crate::types::ApiResult<bool> {
        self.storage
            .check_trust(trustor, trustee, scope, required_level)
            .await
            .map_err(Self::convert_error)
    }

    async fn get_trust_graph(
        &self,
        developer_id: &str,
    ) -> crate::types::ApiResult<std::collections::HashMap<String, crate::storage_ext::TrustLevel>>
    {
        self.storage
            .get_trust_graph(developer_id)
            .await
            .map_err(Self::convert_error)
    }

    async fn store_semantic_metadata(
        &self,
        key: crate::storage_ext::SemanticMetadataKey,
        value: crate::storage_ext::SemanticMetadataValue,
    ) -> crate::types::ApiResult<()> {
        let storage_key = Self::convert_semantic_key_to_storage(key);
        let storage_value = Self::convert_semantic_value_to_storage(value);
        self.storage
            .store_semantic_metadata(storage_key, storage_value)
            .await
            .map_err(Self::convert_error)
    }

    async fn get_semantic_metadata(
        &self,
        key: &crate::storage_ext::SemanticMetadataKey,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::SemanticMetadataValue>> {
        let storage_key = Self::convert_semantic_key_to_storage(key.clone());
        let result = self
            .storage
            .get_semantic_metadata(&storage_key)
            .await
            .map_err(Self::convert_error)?;

        Ok(result.map(Self::convert_semantic_value_from_storage))
    }

    async fn detect_semantic_conflicts(
        &self,
        chain: &str,
        contract: &str,
        field: &str,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::SemanticConflicts>> {
        let result = self
            .storage
            .detect_semantic_conflicts(chain, contract, field)
            .await
            .map_err(Self::convert_error)?;

        Ok(result.map(Self::convert_semantic_conflicts_from_storage))
    }

    async fn resolve_semantic_conflict(
        &self,
        chain: &str,
        contract: &str,
        field: &str,
        chosen_developer: &str,
        method: &str,
        explanation: &str,
    ) -> crate::types::ApiResult<()> {
        self.storage
            .resolve_semantic_conflict(
                chain,
                contract,
                field,
                chosen_developer,
                method,
                explanation,
            )
            .await
            .map_err(Self::convert_error)
    }

    async fn submit_layout(
        &self,
        key: crate::storage_ext::ContractLayoutKey,
        value: crate::storage_ext::ContractLayoutValue,
    ) -> crate::types::ApiResult<()> {
        let storage_key = Self::convert_contract_layout_key_to_storage(key);
        let storage_value = Self::convert_contract_layout_value_to_storage(value);
        self.storage
            .submit_layout(storage_key, storage_value)
            .await
            .map_err(Self::convert_error)
    }

    async fn get_layout(
        &self,
        key: &crate::storage_ext::ContractLayoutKey,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::ContractLayoutValue>> {
        let storage_key = Self::convert_contract_layout_key_to_storage(key.clone());
        let result = self
            .storage
            .get_layout(&storage_key)
            .await
            .map_err(Self::convert_error)?;

        Ok(result.map(Self::convert_contract_layout_value_from_storage))
    }

    async fn list_layouts(
        &self,
        chain: &str,
        contract: &str,
    ) -> crate::types::ApiResult<Vec<crate::storage_ext::ContractLayoutValue>> {
        let result = self
            .storage
            .list_layouts(chain, contract)
            .await
            .map_err(Self::convert_error)?;

        Ok(result
            .into_iter()
            .map(Self::convert_contract_layout_value_from_storage)
            .collect())
    }

    async fn verify_layout_commitment(
        &self,
        chain: &str,
        contract: &str,
        commitment: &[u8; 32],
    ) -> crate::types::ApiResult<bool> {
        self.storage
            .verify_layout_commitment(chain, contract, commitment)
            .await
            .map_err(Self::convert_error)
    }

    async fn register_view(
        &self,
        definition: crate::storage_ext::ViewDefinition,
    ) -> crate::types::ApiResult<()> {
        self.storage
            .register_view(definition)
            .await
            .map_err(Self::convert_error)
    }

    async fn get_view(
        &self,
        view_id: &str,
    ) -> crate::types::ApiResult<Option<crate::storage_ext::ViewDefinition>> {
        self.storage
            .get_view(view_id)
            .await
            .map_err(Self::convert_error)
    }

    async fn update_view(
        &self,
        definition: crate::storage_ext::ViewDefinition,
    ) -> crate::types::ApiResult<()> {
        self.storage
            .update_view(definition)
            .await
            .map_err(Self::convert_error)
    }

    async fn delete_view(&self, view_id: &str) -> crate::types::ApiResult<()> {
        self.storage
            .delete_view(view_id)
            .await
            .map_err(Self::convert_error)
    }

    async fn list_views_for_contract(
        &self,
        contract_address: &str,
    ) -> crate::types::ApiResult<Vec<crate::storage_ext::ViewDefinition>> {
        self.storage
            .list_views_for_contract(contract_address)
            .await
            .map_err(Self::convert_error)
    }

    async fn query_view(
        &self,
        view_id: &str,
        chain: &str,
        contract: &str,
        block_height: u64,
        include_proofs: bool,
    ) -> crate::types::ApiResult<crate::storage_ext::ViewQueryResult> {
        self.storage
            .query_view(view_id, chain, contract, block_height, include_proofs)
            .await
            .map_err(Self::convert_error)
    }

    async fn get_storage_proof(
        &self,
        chain: &str,
        contract: &str,
        block_height: u64,
        storage_key: &[u8; 32],
    ) -> crate::types::ApiResult<Option<crate::storage_ext::StorageProofValue>> {
        let result = self
            .storage
            .get_storage_proof(chain, contract, block_height, storage_key)
            .await
            .map_err(Self::convert_error)?;

        // Convert storage StorageProofValue to API StorageProofValue
        match result {
            Some(storage_proof) => {
                let api_proof = crate::storage_ext::StorageProofValue {
                    proof: storage_proof.proof_data,
                    value: Some(vec![]),   // Convert as needed
                    block_hash: [0u8; 32], // Convert as needed
                };
                Ok(Some(api_proof))
            }
            None => Ok(None),
        }
    }

    async fn get_latest_block_number(&self, chain: &str) -> crate::types::ApiResult<Option<u64>> {
        self.storage
            .get_latest_block_number(chain)
            .await
            .map_err(Self::convert_error)
    }

    async fn get_anchor_idl(&self, program_id: &str) -> crate::types::ApiResult<Option<String>> {
        self.storage
            .get_anchor_idl(program_id)
            .await
            .map_err(Self::convert_error)
    }
}
