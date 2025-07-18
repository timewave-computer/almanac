//! View management API handlers
//!
//! This module provides HTTP endpoints for creating, managing, and querying
//! custom views over contract state with trust-scoped access control.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, error, info};

use crate::types::ApiError;
use crate::error::HttpError;
use crate::state::AppState;
use crate::types::ApiResponse;
use almanac_infrastructure_storage::tuple_fdb::view_registry::{
    ViewDefinition, SlotDefinition, TrustPolicy, TrustBehavior,
};

/// Request to create a new view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewRequest {
    /// Unique view ID
    pub view_id: String,
    /// Contract address the view applies to
    pub contract_address: String,
    /// Chain ID
    pub chain_id: String,
    /// View description
    pub description: Option<String>,
    /// Slot definitions
    pub slots: Vec<SlotDefinitionDto>,
    /// Decode types for slots
    pub decode_types: HashMap<String, String>,
    /// Trust policy for the view
    pub trust_policy: TrustPolicyDto,
    /// Parent view to inherit from
    pub parent_view: Option<String>,
    /// Developer creating the view
    pub developer_id: String,
}

/// Slot definition DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDefinitionDto {
    /// Field name
    pub name: String,
    /// Storage key or slot
    pub storage_key: String,
    /// Optional decode type
    pub decode_type: Option<String>,
    /// Whether this slot is indexed
    pub indexed: bool,
    /// Optional documentation
    pub documentation: Option<String>,
}

impl From<SlotDefinitionDto> for SlotDefinition {
    fn from(dto: SlotDefinitionDto) -> Self {
        SlotDefinition {
            name: dto.name,
            storage_key: dto.storage_key,
            decode_type: dto.decode_type,
            indexed: dto.indexed,
        }
    }
}

impl From<SlotDefinition> for SlotDefinitionDto {
    fn from(slot: SlotDefinition) -> Self {
        SlotDefinitionDto {
            name: slot.name,
            storage_key: slot.storage_key,
            decode_type: slot.decode_type,
            indexed: slot.indexed,
            documentation: None,
        }
    }
}

/// Trust policy DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPolicyDto {
    /// Default behavior for untrusted sources
    pub default_behavior: TrustBehaviorDto,
    /// Explicit trust list
    pub explicit_trusts: Vec<String>,
    /// Minimum trust level required
    pub minimum_level: String,
}

/// Trust behavior DTO
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustBehaviorDto {
    Accept,
    Reject,
    RequireVerification,
}

impl From<TrustBehaviorDto> for TrustBehavior {
    fn from(dto: TrustBehaviorDto) -> Self {
        match dto {
            TrustBehaviorDto::Accept => TrustBehavior::Accept,
            TrustBehaviorDto::Reject => TrustBehavior::Reject,
            TrustBehaviorDto::RequireVerification => TrustBehavior::RequireVerification,
        }
    }
}

impl From<TrustBehavior> for TrustBehaviorDto {
    fn from(behavior: TrustBehavior) -> Self {
        match behavior {
            TrustBehavior::Accept => TrustBehaviorDto::Accept,
            TrustBehavior::Reject => TrustBehaviorDto::Reject,
            TrustBehavior::RequireVerification => TrustBehaviorDto::RequireVerification,
        }
    }
}

impl From<TrustPolicyDto> for TrustPolicy {
    fn from(dto: TrustPolicyDto) -> Self {
        TrustPolicy {
            default_behavior: dto.default_behavior.into(),
            explicit_trusts: dto.explicit_trusts,
            minimum_level: almanac_infrastructure_storage::tuple_fdb::trust::TrustLevel::ReadOnly,
        }
    }
}

impl From<TrustPolicy> for TrustPolicyDto {
    fn from(policy: TrustPolicy) -> Self {
        TrustPolicyDto {
            default_behavior: policy.default_behavior.into(),
            explicit_trusts: policy.explicit_trusts,
            minimum_level: "read_only".to_string(),
        }
    }
}

/// Response for view creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewResponse {
    /// View ID
    pub view_id: String,
    /// Contract address
    pub contract_address: String,
    /// Chain ID
    pub chain_id: String,
    /// Number of slots defined
    pub slot_count: usize,
    /// When created
    pub created_at: u64,
    /// Created by
    pub created_by: String,
}

/// View query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewQueryResult {
    /// View ID
    pub view_id: String,
    /// Block height of data
    pub block_height: u64,
    /// Slot values
    pub data: HashMap<String, serde_json::Value>,
    /// Storage proofs (if requested)
    pub proofs: Option<HashMap<String, StorageProofDto>>,
    /// Trust context
    pub trust_context: TrustContextDto,
    /// Query timestamp
    pub queried_at: u64,
}

/// Storage proof DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProofDto {
    /// Storage value
    pub value: String,
    /// Merkle proof nodes
    pub proof: Vec<String>,
    /// State root for verification
    pub state_root: String,
    /// Whether verified
    pub verified: bool,
}

/// Trust context DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustContextDto {
    /// Primary data source
    pub primary_source: String,
    /// Trust level
    pub trust_level: String,
    /// Any conflicts detected
    pub conflicts: Vec<String>,
    /// Confidence score
    pub confidence: f64,
}

/// Query parameters for views
#[derive(Debug, Deserialize)]
pub struct ViewQuery {
    /// Block height to query at
    pub block_height: Option<u64>,
    /// Include storage proofs
    pub include_proofs: Option<bool>,
    /// Trust policy override
    pub trust_policy: Option<String>,
}

/// View details with full definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewDetails {
    /// View definition
    pub definition: ViewDefinition,
    /// Access statistics
    pub access_stats: ViewAccessStats,
    /// Parent view (if any)
    pub parent_view: Option<String>,
    /// Child views
    pub child_views: Vec<String>,
}

/// View access statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewAccessStats {
    /// Total queries
    pub total_queries: u64,
    /// Last accessed
    pub last_accessed: Option<u64>,
    /// Unique users
    pub unique_users: u32,
    /// Average query time
    pub avg_query_time_ms: f64,
}

/// Create a new view
pub async fn create_view(
    State(state): State<Arc<AppState>>,
    Path((chain, address)): Path<(String, String)>,
    Json(request): Json<CreateViewRequest>,
) -> Result<impl IntoResponse, HttpError> {
    info!(
        "Creating view {} for contract {} on chain {} by developer {}",
        request.view_id, address, chain, request.developer_id
    );

    // Validate request
    if chain != request.chain_id || address != request.contract_address {
        return Err(HttpError::Api(ApiError::BadRequest("Path parameters don't match request body".to_string())));
    }

    // Convert to storage types
    let definition = ViewDefinition {
        view_id: request.view_id.clone(),
        contract_address: address.clone(),
        slots: request.slots.into_iter().map(Into::into).collect(),
        decode_types: request.decode_types,
        trust_policy: request.trust_policy.into(),
        created_by: request.developer_id.clone(),
        created_at: chrono::Utc::now().timestamp() as u64,
    };

    let storage = &state.storage;

    // Check if view already exists
    if storage.get_view(&request.view_id).await
        .map_err(|_| HttpError::Api(ApiError::InternalServerError))?
        .is_some() {
        return Err(HttpError::Api(ApiError::BadRequest(format!("View {} already exists", request.view_id))));
    }

    // Register the view
    storage.register_view(definition.clone()).await
        .map_err(|e| {
            error!("Failed to register view: {}", e);
            HttpError::Api(ApiError::InternalServerError)
        })?;

    let response = CreateViewResponse {
        view_id: definition.view_id,
        contract_address: address,
        chain_id: chain,
        slot_count: definition.slots.len(),
        created_at: definition.created_at,
        created_by: definition.created_by,
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

/// Get view definition
pub async fn get_view(
    State(state): State<Arc<AppState>>,
    Path((chain, address, view_id)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, HttpError> {
    debug!("Getting view {} for contract {} on chain {}", view_id, address, chain);

    let storage = &state.storage;

    let view = storage.get_view(&view_id).await
        .map_err(|e| {
            error!("Failed to get view: {}", e);
            HttpError::Api(ApiError::InternalServerError)
        })?;

    match view {
        Some(definition) => {
            // Verify it matches the requested contract
            if definition.contract_address != address {
                return Err(HttpError::Api(ApiError::NotFound(format!("View {} not found for contract {}", view_id, address))));
            }

            let details = ViewDetails {
                definition,
                access_stats: ViewAccessStats {
                    total_queries: 0, // Would be tracked in real implementation
                    last_accessed: None,
                    unique_users: 0,
                    avg_query_time_ms: 0.0,
                },
                parent_view: None, // Would check parent relationships
                child_views: vec![], // Would find child views
            };

            Ok((StatusCode::OK, Json(ApiResponse::success(details))))
        }
        None => Err(HttpError::Api(ApiError::NotFound(format!("View {} not found", view_id)))),
    }
}

/// Query a view
pub async fn query_view(
    State(state): State<Arc<AppState>>,
    Path((chain, address, view_id)): Path<(String, String, String)>,
    Query(query): Query<ViewQuery>,
) -> Result<impl IntoResponse, HttpError> {
    info!(
        "Querying view {} for contract {} on chain {} at block {:?}",
        view_id, address, chain, query.block_height
    );

    let storage = &state.storage;

    // Get the view definition
    let view = storage.get_view(&view_id).await
        .map_err(|e| {
            error!("Failed to get view: {}", e);
            HttpError::Api(ApiError::InternalServerError)
        })?
        .ok_or_else(|| HttpError::Api(ApiError::NotFound(format!("View {} not found", view_id))))?;

    // Verify it matches the requested contract
    if view.contract_address != address {
        return Err(HttpError::Api(ApiError::NotFound(format!("View {} not found for contract {}", view_id, address))));
    }

    // Get the latest block if not specified
    let block_height = match query.block_height {
        Some(h) => h,
        None => {
            // Get latest block from storage
            storage.get_latest_block_number(&chain).await
                .map_err(|_| HttpError::Api(ApiError::InternalServerError))?
                .ok_or_else(|| HttpError::Api(ApiError::NotFound("No blocks found for chain".to_string())))?
        }
    };

    // Execute the view query
    let result = storage.query_view(
        &view_id,
        &chain,
        &address,
        block_height,
        query.include_proofs.unwrap_or(false),
    ).await
    .map_err(|e| {
        error!("Failed to query view: {}", e);
        HttpError::Api(ApiError::InternalServerError)
    })?;

    // Convert to response format
    let response = ViewQueryResult {
        view_id,
        block_height,
        data: result.data,
        proofs: if query.include_proofs.unwrap_or(false) {
            Some(result.proofs.into_iter().map(|(k, v)| {
                (k, StorageProofDto {
                    value: hex::encode(&v.value),
                    proof: v.proof.into_iter().map(|p| hex::encode(p)).collect(),
                    state_root: hex::encode(&v.state_root),
                    verified: v.verified,
                })
            }).collect())
        } else {
            None
        },
        trust_context: TrustContextDto {
            primary_source: result.primary_source,
            trust_level: format!("{:?}", result.trust_level),
            conflicts: result.conflicts,
            confidence: result.confidence,
        },
        queried_at: chrono::Utc::now().timestamp() as u64,
    };

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// Update a view
pub async fn update_view(
    State(state): State<Arc<AppState>>,
    Path((chain, address, view_id)): Path<(String, String, String)>,
    Json(request): Json<CreateViewRequest>,
) -> Result<impl IntoResponse, HttpError> {
    info!(
        "Updating view {} for contract {} on chain {}",
        view_id, address, chain
    );

    // Validate path matches request
    if view_id != request.view_id {
        return Err(HttpError::Api(ApiError::BadRequest("View ID in path doesn't match request body".to_string())));
    }

    let storage = &state.storage;

    // Get existing view
    let existing = storage.get_view(&view_id).await
        .map_err(|_| HttpError::Api(ApiError::InternalServerError))?
        .ok_or_else(|| HttpError::Api(ApiError::NotFound(format!("View {} not found", view_id))))?;

    // Verify ownership or admin rights
    if existing.created_by != request.developer_id {
        return Err(HttpError::Api(ApiError::Forbidden("Only the creator can update a view".to_string())));
    }

    // Update the view
    let updated_definition = ViewDefinition {
        view_id: view_id.clone(),
        contract_address: address.clone(),
        slots: request.slots.into_iter().map(Into::into).collect(),
        decode_types: request.decode_types,
        trust_policy: request.trust_policy.into(),
        created_by: existing.created_by,
        created_at: existing.created_at,
    };

    storage.update_view(updated_definition).await
        .map_err(|e| {
            error!("Failed to update view: {}", e);
            HttpError::Api(ApiError::InternalServerError)
        })?;

    Ok((StatusCode::OK, Json(ApiResponse::success(serde_json::json!({
        "message": "View updated successfully",
        "view_id": view_id,
    })))))
}

/// Delete a view
pub async fn delete_view(
    State(state): State<Arc<AppState>>,
    Path((chain, address, view_id)): Path<(String, String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, HttpError> {
    let developer_id = params.get("developer_id")
        .ok_or_else(|| HttpError::Api(ApiError::BadRequest("Missing developer_id parameter".to_string())))?;

    info!(
        "Deleting view {} for contract {} on chain {} by developer {}",
        view_id, address, chain, developer_id
    );

    let storage = &state.storage;

    // Get existing view
    let existing = storage.get_view(&view_id).await
        .map_err(|_| HttpError::Api(ApiError::InternalServerError))?
        .ok_or_else(|| HttpError::Api(ApiError::NotFound(format!("View {} not found", view_id))))?;

    // Verify ownership
    if existing.created_by != developer_id {
        return Err(HttpError::Api(ApiError::Forbidden("Only the creator can delete a view".to_string())));
    }

    // Delete the view
    storage.delete_view(&view_id).await
        .map_err(|e| {
            error!("Failed to delete view: {}", e);
            HttpError::Api(ApiError::InternalServerError)
        })?;

    Ok((StatusCode::OK, Json(ApiResponse::success(serde_json::json!({
        "message": "View deleted successfully",
        "view_id": view_id,
    })))))
}

/// List views for a contract
pub async fn list_views(
    State(state): State<Arc<AppState>>,
    Path((chain, address)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, HttpError> {
    debug!("Listing views for contract {} on chain {}", address, chain);

    let storage = &state.storage;

    // Get all views for the contract
    let views = storage.list_views_for_contract(&address).await
        .map_err(|e| {
            error!("Failed to list views: {}", e);
            HttpError::Api(ApiError::InternalServerError)
        })?;

    // Filter by developer if requested
    let filtered_views: Vec<_> = if let Some(developer_id) = params.get("developer_id") {
        views.into_iter()
            .filter(|v| &v.created_by == developer_id)
            .collect()
    } else {
        views
    };

    // Convert to summary format
    let summaries: Vec<_> = filtered_views.into_iter().map(|v| {
        serde_json::json!({
            "view_id": v.view_id,
            "contract_address": v.contract_address,
            "slot_count": v.slots.len(),
            "created_by": v.created_by,
            "created_at": v.created_at,
            "trust_policy": TrustPolicyDto::from(v.trust_policy),
        })
    }).collect();

    Ok((StatusCode::OK, Json(ApiResponse::success(summaries))))
}

/// Get raw storage with proof
pub async fn get_storage_with_proof(
    State(state): State<Arc<AppState>>,
    Path((chain, address, slot)): Path<(String, String, String)>,
    Query(query): Query<ViewQuery>,
) -> Result<impl IntoResponse, HttpError> {
    debug!(
        "Getting storage slot {} for contract {} on chain {} with proof",
        slot, address, chain
    );

    let storage = &state.storage;

    // Parse storage key
    let storage_key = if slot.starts_with("0x") {
        hex::decode(&slot[2..])
            .map_err(|_| HttpError::Api(ApiError::BadRequest("Invalid hex storage key".to_string())))?
    } else {
        // Treat as field name, would need layout to resolve
        return Err(HttpError::Api(ApiError::BadRequest("Field name resolution not yet implemented".to_string())));
    };

    // Pad to 32 bytes
    let mut storage_key_array = [0u8; 32];
    if storage_key.len() > 32 {
        return Err(HttpError::Api(ApiError::BadRequest("Storage key exceeds 32 bytes".to_string())));
    }
    storage_key_array[..storage_key.len()].copy_from_slice(&storage_key);

    // Get the latest block if not specified
    let block_height = match query.block_height {
        Some(h) => h,
        None => {
            storage.get_latest_block_number(&chain).await
                .map_err(|_| HttpError::Api(ApiError::InternalServerError))?
                .ok_or_else(|| HttpError::Api(ApiError::NotFound("No blocks found for chain".to_string())))?
        }
    };

    // Get storage value with proof
    let proof_result = storage.get_storage_proof(
        &chain,
        &address,
        block_height,
        &storage_key_array,
    ).await
    .map_err(|e| {
        error!("Failed to get storage proof: {}", e);
        HttpError::Api(ApiError::InternalServerError)
    })?;

    match proof_result {
        Some(proof) => {
            let response = serde_json::json!({
                "slot": slot,
                "value": hex::encode(&proof.value),
                "block_height": block_height,
                "proof": StorageProofDto {
                    value: hex::encode(&proof.value),
                    proof: proof.proof.into_iter().map(|p| hex::encode(p)).collect(),
                    state_root: hex::encode(&proof.state_root),
                    verified: proof.verified,
                },
                "timestamp": chrono::Utc::now().timestamp(),
            });

            Ok((StatusCode::OK, Json(ApiResponse::success(response))))
        }
        None => Err(HttpError::Api(ApiError::NotFound(format!("Storage slot {} not found at block {}", slot, block_height))))
    }
}