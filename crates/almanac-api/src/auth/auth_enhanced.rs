//! Enhanced authentication system with permissions and scoping
//!
//! This module provides a comprehensive API key management system with:
//! - Permission-based access control
//! - Developer-specific scoping
//! - Usage tracking and analytics
//! - Quota management

use crate::error::HttpError;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Enhanced API key with permissions and metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique key ID
    pub key_id: String,
    /// The actual API key (hashed in storage)
    pub key_hash: String,
    /// Display name for the key
    pub name: String,
    /// Developer ID who owns this key
    pub developer_id: String,
    /// Key permissions
    pub permissions: ApiPermissions,
    /// Rate limit configuration
    pub rate_limit: RateLimitConfig,
    /// Usage quotas
    pub quotas: UsageQuotas,
    /// Key metadata
    pub metadata: KeyMetadata,
    /// Whether the key is enabled
    pub enabled: bool,
}

/// API permissions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiPermissions {
    /// Allowed operations
    pub operations: HashSet<ApiOperation>,
    /// Allowed chains
    pub chains: Option<HashSet<String>>,
    /// Allowed contracts
    pub contracts: Option<HashSet<String>>,
    /// Custom scopes
    pub scopes: HashSet<String>,
}

/// API operations
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiOperation {
    // Read operations
    ReadEvents,
    ReadLayouts,
    ReadSemantics,
    ReadViews,
    ReadStorage,
    ReadTrust,

    // Write operations
    WriteLayouts,
    WriteSemantics,
    WriteViews,
    WriteTrust,

    // Admin operations
    ManageKeys,
    ManageUsers,

    // Traverse operations
    GenerateProofs,
    VerifyProofs,
}

/// Rate limit configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Requests per hour
    pub requests_per_hour: u32,
    /// Requests per day
    pub requests_per_day: u32,
    /// Burst limit
    pub burst_limit: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            burst_limit: 10,
        }
    }
}

/// Usage quotas
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageQuotas {
    /// Maximum storage queries per month
    pub storage_queries_monthly: Option<u64>,
    /// Maximum proof generations per month
    pub proof_generations_monthly: Option<u64>,
    /// Maximum data egress in bytes per month
    pub data_egress_monthly: Option<u64>,
}

impl Default for UsageQuotas {
    fn default() -> Self {
        Self {
            storage_queries_monthly: Some(100000),
            proof_generations_monthly: Some(1000),
            data_egress_monthly: Some(1_000_000_000), // 1GB
        }
    }
}

/// Key metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// When the key was created
    pub created_at: DateTime<Utc>,
    /// Last time the key was used
    pub last_used_at: Option<DateTime<Utc>>,
    /// Expiration time (if any)
    pub expires_at: Option<DateTime<Utc>>,
    /// IP whitelist (if any)
    pub ip_whitelist: Option<Vec<String>>,
    /// User agent restrictions (if any)
    pub user_agent_pattern: Option<String>,
    /// Custom tags
    pub tags: HashMap<String, String>,
}

/// Usage statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UsageStats {
    /// Total requests
    pub total_requests: u64,
    /// Requests by operation
    pub requests_by_operation: HashMap<String, u64>,
    /// Data egress in bytes
    pub data_egress_bytes: u64,
    /// Storage queries
    pub storage_queries: u64,
    /// Proof generations
    pub proof_generations: u64,
    /// Last reset timestamp
    pub last_reset: DateTime<Utc>,
}

/// Enhanced API key store
#[derive(Clone)]
pub struct EnhancedApiKeyStore {
    /// Keys indexed by key hash
    keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    /// Usage statistics by key ID
    usage: Arc<RwLock<HashMap<String, UsageStats>>>,
    /// Key lookup by developer ID
    developer_keys: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl Default for EnhancedApiKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EnhancedApiKeyStore {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
            developer_keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a new API key
    pub async fn generate_key(
        &self,
        request: GenerateKeyRequest,
    ) -> Result<GenerateKeyResponse, String> {
        // Generate random key
        let api_key = format!("ak_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let key_hash = self.hash_key(&api_key);
        let key_id = format!("key_{}", uuid::Uuid::new_v4());

        let key = ApiKey {
            key_id: key_id.clone(),
            key_hash: key_hash.clone(),
            name: request.name,
            developer_id: request.developer_id.clone(),
            permissions: request.permissions,
            rate_limit: request.rate_limit.unwrap_or_default(),
            quotas: request.quotas.unwrap_or_default(),
            metadata: KeyMetadata {
                created_at: Utc::now(),
                last_used_at: None,
                expires_at: request.expires_at,
                ip_whitelist: request.ip_whitelist,
                user_agent_pattern: request.user_agent_pattern,
                tags: request.tags.unwrap_or_default(),
            },
            enabled: true,
        };

        // Store the key
        self.keys.write().await.insert(key_hash.clone(), key);

        // Update developer index
        self.developer_keys
            .write()
            .await
            .entry(request.developer_id)
            .or_insert_with(HashSet::new)
            .insert(key_hash);

        // Initialize usage stats
        self.usage
            .write()
            .await
            .insert(key_id.clone(), UsageStats::default());

        Ok(GenerateKeyResponse {
            key_id,
            api_key,
            created_at: Utc::now(),
        })
    }

    /// Validate an API key and check permissions
    pub async fn validate_key_with_permissions(
        &self,
        api_key: &str,
        operation: ApiOperation,
        context: &PermissionContext,
    ) -> Result<ApiKey, ApiKeyError> {
        let key_hash = self.hash_key(api_key);

        let keys = self.keys.read().await;
        let key = keys.get(&key_hash).ok_or(ApiKeyError::InvalidKey)?;

        // Check if key is enabled
        if !key.enabled {
            return Err(ApiKeyError::KeyDisabled);
        }

        // Check expiration
        if let Some(expires_at) = key.metadata.expires_at {
            if Utc::now() > expires_at {
                return Err(ApiKeyError::KeyExpired);
            }
        }

        // Check permissions
        if !key.permissions.operations.contains(&operation) {
            return Err(ApiKeyError::InsufficientPermissions);
        }

        // Check chain restrictions
        if let Some(allowed_chains) = &key.permissions.chains {
            if let Some(chain) = &context.chain {
                if !allowed_chains.contains(chain) {
                    return Err(ApiKeyError::ChainNotAllowed);
                }
            }
        }

        // Check contract restrictions
        if let Some(allowed_contracts) = &key.permissions.contracts {
            if let Some(contract) = &context.contract {
                if !allowed_contracts.contains(contract) {
                    return Err(ApiKeyError::ContractNotAllowed);
                }
            }
        }

        // Update last used timestamp
        self.update_last_used(&key.key_id).await;

        Ok(key.clone())
    }

    /// Update usage statistics
    pub async fn track_usage(
        &self,
        key_id: &str,
        operation: &str,
        data_egress: u64,
        is_storage_query: bool,
        is_proof_generation: bool,
    ) {
        let mut usage = self.usage.write().await;
        if let Some(stats) = usage.get_mut(key_id) {
            stats.total_requests += 1;
            *stats
                .requests_by_operation
                .entry(operation.to_string())
                .or_insert(0) += 1;
            stats.data_egress_bytes += data_egress;
            if is_storage_query {
                stats.storage_queries += 1;
            }
            if is_proof_generation {
                stats.proof_generations += 1;
            }
        }
    }

    /// Check if usage is within quotas
    pub async fn check_quotas(&self, key: &ApiKey) -> Result<(), QuotaError> {
        let usage = self.usage.read().await;
        if let Some(stats) = usage.get(&key.key_id) {
            // Check storage queries
            if let Some(limit) = key.quotas.storage_queries_monthly {
                if stats.storage_queries >= limit {
                    return Err(QuotaError::StorageQueriesExceeded);
                }
            }

            // Check proof generations
            if let Some(limit) = key.quotas.proof_generations_monthly {
                if stats.proof_generations >= limit {
                    return Err(QuotaError::ProofGenerationsExceeded);
                }
            }

            // Check data egress
            if let Some(limit) = key.quotas.data_egress_monthly {
                if stats.data_egress_bytes >= limit {
                    return Err(QuotaError::DataEgressExceeded);
                }
            }
        }

        Ok(())
    }

    /// List keys for a developer
    pub async fn list_developer_keys(&self, developer_id: &str) -> Vec<ApiKeySummary> {
        let dev_keys = self.developer_keys.read().await;
        let keys = self.keys.read().await;

        if let Some(key_hashes) = dev_keys.get(developer_id) {
            key_hashes
                .iter()
                .filter_map(|hash| keys.get(hash))
                .map(|key| ApiKeySummary {
                    key_id: key.key_id.clone(),
                    name: key.name.clone(),
                    created_at: key.metadata.created_at,
                    last_used_at: key.metadata.last_used_at,
                    enabled: key.enabled,
                    permissions_summary: format!("{} operations", key.permissions.operations.len()),
                })
                .collect()
        } else {
            vec![]
        }
    }

    /// Hash an API key
    fn hash_key(&self, key: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Update last used timestamp
    async fn update_last_used(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        for key in keys.values_mut() {
            if key.key_id == key_id {
                key.metadata.last_used_at = Some(Utc::now());
                break;
            }
        }
    }
}

/// Request to generate a new API key
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateKeyRequest {
    pub name: String,
    pub developer_id: String,
    pub permissions: ApiPermissions,
    pub rate_limit: Option<RateLimitConfig>,
    pub quotas: Option<UsageQuotas>,
    pub expires_at: Option<DateTime<Utc>>,
    pub ip_whitelist: Option<Vec<String>>,
    pub user_agent_pattern: Option<String>,
    pub tags: Option<HashMap<String, String>>,
}

/// Response from key generation
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateKeyResponse {
    pub key_id: String,
    pub api_key: String,
    pub created_at: DateTime<Utc>,
}

/// API key summary for listing
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeySummary {
    pub key_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub permissions_summary: String,
}

/// Permission context for validation
#[derive(Debug)]
pub struct PermissionContext {
    pub chain: Option<String>,
    pub contract: Option<String>,
    pub developer_id: Option<String>,
}

/// API key errors
#[derive(Debug, thiserror::Error)]
pub enum ApiKeyError {
    #[error("Invalid API key")]
    InvalidKey,
    #[error("API key is disabled")]
    KeyDisabled,
    #[error("API key has expired")]
    KeyExpired,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("Chain not allowed")]
    ChainNotAllowed,
    #[error("Contract not allowed")]
    ContractNotAllowed,
}

/// Quota errors
#[derive(Debug, thiserror::Error)]
pub enum QuotaError {
    #[error("Storage queries quota exceeded")]
    StorageQueriesExceeded,
    #[error("Proof generations quota exceeded")]
    ProofGenerationsExceeded,
    #[error("Data egress quota exceeded")]
    DataEgressExceeded,
}

/// Enhanced authentication middleware
pub async fn enhanced_auth_middleware(
    State(key_store): State<EnhancedApiKeyStore>,
    mut request: Request,
    next: Next,
) -> Result<Response, HttpError> {
    // Extract API key from header
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            HttpError::Api(crate::types::error::ApiError::Unauthorized(
                "API key required".to_string(),
            ))
        })?;

    // Extract operation from request path and method
    let operation = extract_operation(&request);
    let context = extract_permission_context(&request);

    // Validate key and permissions
    match key_store
        .validate_key_with_permissions(api_key, operation, &context)
        .await
    {
        Ok(key) => {
            // Check quotas
            if let Err(_e) = key_store.check_quotas(&key).await {
                return Err(HttpError::Api(
                    crate::types::error::ApiError::RateLimitExceeded,
                ));
            }

            // Store key info in request extensions for later use
            request.extensions_mut().insert(key);

            let response = next.run(request).await;
            Ok(response)
        }
        Err(e) => Err(HttpError::Api(crate::types::error::ApiError::Unauthorized(
            format!("Authentication failed: {}", e),
        ))),
    }
}

/// Extract operation from request
fn extract_operation(request: &Request) -> ApiOperation {
    let path = request.uri().path();
    let method = request.method();

    // Simple mapping based on path and method
    match (method.as_str(), path) {
        ("GET", p) if p.contains("/events") => ApiOperation::ReadEvents,
        ("GET", p) if p.contains("/layout") => ApiOperation::ReadLayouts,
        ("POST", p) if p.contains("/layout") => ApiOperation::WriteLayouts,
        ("GET", p) if p.contains("/semantics") => ApiOperation::ReadSemantics,
        ("POST", p) if p.contains("/semantics") => ApiOperation::WriteSemantics,
        ("GET", p) if p.contains("/views") => ApiOperation::ReadViews,
        ("POST", p) if p.contains("/views") => ApiOperation::WriteViews,
        ("GET", p) if p.contains("/trust") => ApiOperation::ReadTrust,
        ("POST", p) if p.contains("/trust") => ApiOperation::WriteTrust,
        ("POST", p) if p.contains("/traverse/request") => ApiOperation::GenerateProofs,
        ("POST", p) if p.contains("/traverse/verify") => ApiOperation::VerifyProofs,
        _ => ApiOperation::ReadEvents, // Default
    }
}

/// Extract permission context from request
fn extract_permission_context(request: &Request) -> PermissionContext {
    let path = request.uri().path();

    // Extract chain and contract from path
    // This is a simplified extraction - would need proper parsing
    let parts: Vec<&str> = path.split('/').collect();

    let chain = parts
        .iter()
        .position(|&p| p == "chains")
        .and_then(|i| parts.get(i + 1))
        .map(|s| s.to_string());

    let contract = parts
        .iter()
        .position(|&p| p == "contracts")
        .and_then(|i| parts.get(i + 1))
        .map(|s| s.to_string());

    PermissionContext {
        chain,
        contract,
        developer_id: None, // Would be extracted from token or key info
    }
}
