//! Layout submission and management handlers for Traverse CLI integration

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::types::ApiError;
use crate::error::HttpError;
use crate::state::AppState;
use crate::types::types::{ApiResponse, ChainId, ContractAddress};

/// Request body for layout submission from Traverse CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSubmissionRequest {
    /// Chain identifier (e.g., "ethereum-mainnet", "cosmos-hub")
    pub chain_id: String,
    /// Contract address
    pub contract_address: String,
    /// Layout commitment (SHA256 hash from Traverse CLI)
    pub layout_commitment: String,
    /// Serialized layout data (JSON from Traverse)
    pub layout_data: serde_json::Value,
    /// Compiler version used
    pub compiler_version: String,
    /// Optional source code hash
    pub source_hash: Option<String>,
    /// Developer submitting the layout
    pub developer_id: String,
    /// Optional project identifier
    pub project_id: Option<String>,
    /// Optional documentation URL
    pub documentation_url: Option<String>,
    /// Optional cryptographic signature
    pub signature: Option<String>,
}

/// Response for layout submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSubmissionResponse {
    /// Unique identifier for the submitted layout
    pub layout_id: String,
    /// Confirmation of the layout commitment
    pub layout_commitment: String,
    /// Timestamp of submission
    pub submitted_at: u64,
    /// Verification status
    pub verification_status: LayoutVerificationStatus,
    /// Any conflicts detected
    pub conflicts: Option<Vec<LayoutConflict>>,
}

/// Layout verification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutVerificationStatus {
    /// Layout is pending verification
    Pending,
    /// Layout has been verified
    Verified,
    /// Layout conflicts with existing layout
    Conflicting,
    /// Layout verification failed
    Failed(String),
}

/// Layout conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConflict {
    /// Existing layout that conflicts
    pub existing_layout_id: String,
    /// Developer who submitted the existing layout
    pub existing_developer: String,
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Conflict details
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    /// Different layout commitments for same contract
    DifferentCommitment,
    /// Incompatible field mappings
    IncompatibleFields,
    /// Version mismatch
    VersionMismatch,
}

/// Query parameters for layout retrieval
#[derive(Debug, Deserialize)]
pub struct LayoutQuery {
    /// Filter by developer ID
    pub developer_id: Option<String>,
    /// Filter by project ID
    pub project_id: Option<String>,
    /// Include conflicting layouts
    pub include_conflicts: Option<bool>,
    /// Trust policy to apply
    pub trust_policy: Option<String>,
}

/// Submit a new contract layout from Traverse CLI
pub async fn submit_layout(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LayoutSubmissionRequest>,
) -> Result<impl IntoResponse, HttpError> {
    info!(
        "Layout submission request for contract {} on chain {} from developer {}",
        request.contract_address, request.chain_id, request.developer_id
    );

    // Validate the request
    validate_layout_submission(&request)?;

    // Convert layout commitment from hex string to bytes
    let layout_commitment = hex::decode(&request.layout_commitment)
        .map_err(|e| HttpError::Api(ApiError::BadRequest(format!("Invalid layout commitment: {}", e))))?;
    
    if layout_commitment.len() != 32 {
        return Err(HttpError::Api(ApiError::BadRequest(
            "Layout commitment must be 32 bytes (SHA256)".to_string()
        ));
    }
    
    let mut commitment_array = [0u8; 32];
    commitment_array.copy_from_slice(&layout_commitment);

    // Serialize the layout data
    let layout_data_bytes = serde_json::to_vec(&request.layout_data)
        .map_err(|e| HttpError::Api(ApiError::BadRequest(format!("Invalid layout data: {}", e))))?;

    // Get storage backend (FDB)
    let storage = &state.storage;
    
    // Check for existing layouts and conflicts
    let existing_layout = storage.get_contract_layout(
        &request.chain_id,
        &request.contract_address,
        None, // Latest version
        &request.developer_id,
        &almanac_storage::fdb::trust::TrustPolicy::permissive(),
    ).await
    .map_err(|e| {
        error!("Failed to check existing layout: {}", e);
        HttpError::Api(ApiError::InternalServerError)
    })?;

    let mut conflicts = Vec::new();
    let mut verification_status = LayoutVerificationStatus::Pending;

    if let Some((existing, _is_trusted)) = existing_layout {
        // Check if commitments match
        if existing.layout_commitment != commitment_array {
            conflicts.push(LayoutConflict {
                existing_layout_id: format!("{}-{}", request.chain_id, request.contract_address),
                existing_developer: existing.provided_by,
                conflict_type: ConflictType::DifferentCommitment,
                details: "Layout commitment differs from existing layout".to_string(),
            });
            verification_status = LayoutVerificationStatus::Conflicting;
        }
    }

    // Create layout key and value
    let layout_key = almanac_storage::fdb::layout::ContractLayoutKey {
        chain_id: request.chain_id.clone(),
        contract_address: request.contract_address.clone(),
        version: Some(format!("v{}", chrono::Utc::now().timestamp())),
    };

    let layout_value = almanac_storage::fdb::layout::ContractLayoutValue {
        layout_commitment: commitment_array,
        layout_data: layout_data_bytes,
        compiler_version: request.compiler_version.clone(),
        source_hash: request.source_hash.as_ref().and_then(|h| {
            hex::decode(h).ok().and_then(|bytes| {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Some(arr)
                } else {
                    None
                }
            })
        }),
        provided_by: request.developer_id.clone(),
        project_id: request.project_id.clone(),
        documentation_url: request.documentation_url.clone(),
        verification_status: almanac_storage::fdb::layout::VerificationStatus::Pending,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };

    // Store the layout
    storage.store_contract_layout(layout_key.clone(), layout_value)
        .await
        .map_err(|e| {
            error!("Failed to store layout: {}", e);
            HttpError::Api(ApiError::InternalServerError)
        })?;

    // Generate layout ID
    let layout_id = format!(
        "{}-{}-{}",
        request.chain_id,
        request.contract_address,
        chrono::Utc::now().timestamp()
    );

    info!("Successfully stored layout {} with commitment {:?}", layout_id, request.layout_commitment);

    let response = LayoutSubmissionResponse {
        layout_id,
        layout_commitment: request.layout_commitment,
        submitted_at: chrono::Utc::now().timestamp() as u64,
        verification_status,
        conflicts: if conflicts.is_empty() { None } else { Some(conflicts) },
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

/// Get layout for a contract with trust context
pub async fn get_layout(
    State(state): State<Arc<AppState>>,
    Path((chain_id, contract_address)): Path<(String, String)>,
    Query(query): Query<LayoutQuery>,
) -> Result<impl IntoResponse, HttpError> {
    debug!(
        "Layout request for contract {} on chain {} with query {:?}",
        contract_address, chain_id, query
    );

    let storage = &state.storage;
    
    // Determine trust policy based on query
    let trust_policy = match query.trust_policy.as_deref() {
        Some("strict") => almanac_storage::fdb::trust::TrustPolicy::strict(
            query.developer_id.clone().map(|d| vec![d]).unwrap_or_default()
        ),
        Some("permissive") => almanac_storage::fdb::trust::TrustPolicy::permissive(),
        Some("explicit") => {
            if let Some(dev_id) = &query.developer_id {
                almanac_storage::fdb::trust::TrustPolicy::explicit(vec![dev_id.clone()])
            } else {
                almanac_storage::fdb::trust::TrustPolicy::permissive()
            }
        }
        _ => almanac_storage::fdb::trust::TrustPolicy::permissive(),
    };

    // Get the layout with trust context
    let requested_by = query.developer_id.as_deref().unwrap_or("anonymous");
    
    let layout_result = storage.get_contract_layout(
        &chain_id,
        &contract_address,
        None, // Latest version
        requested_by,
        &trust_policy,
    ).await
    .map_err(|e| {
        error!("Failed to get layout: {}", e);
        HttpError::Api(ApiError::InternalServerError)
    })?;

    match layout_result {
        Some((layout, is_trusted)) => {
            // Parse the layout data back to JSON
            let layout_data: serde_json::Value = serde_json::from_slice(&layout.layout_data)
                .map_err(|e| {
                    error!("Failed to parse layout data: {}", e);
                    HttpError::Api(ApiError::InternalServerError)
                })?;

            let response = serde_json::json!({
                "chain_id": chain_id,
                "contract_address": contract_address,
                "layout_commitment": hex::encode(layout.layout_commitment),
                "layout_data": layout_data,
                "compiler_version": layout.compiler_version,
                "source_hash": layout.source_hash.map(hex::encode),
                "provided_by": layout.provided_by,
                "project_id": layout.project_id,
                "documentation_url": layout.documentation_url,
                "verification_status": format!("{:?}", layout.verification_status),
                "timestamp": layout.timestamp,
                "is_trusted": is_trusted,
                "trust_context": {
                    "requested_by": requested_by,
                    "trust_policy": query.trust_policy.as_deref().unwrap_or("permissive"),
                }
            });

            Ok((StatusCode::OK, Json(ApiResponse::success(response))))
        }
        None => {
            warn!("Layout not found for contract {} on chain {}", contract_address, chain_id);
            Err(HttpError::Api(ApiError::NotFound(format!(
                "No layout found for contract {} on chain {}",
                contract_address, chain_id
            )))
        }
    }
}

/// Detect layout conflicts for a contract
pub async fn get_layout_conflicts(
    State(state): State<Arc<AppState>>,
    Path((chain_id, contract_address)): Path<(String, String)>,
) -> Result<impl IntoResponse, HttpError> {
    debug!(
        "Checking layout conflicts for contract {} on chain {}",
        contract_address, chain_id
    );

    let storage = &state.storage;
    
    let conflicts = storage.detect_layout_conflicts(&chain_id, &contract_address)
        .await
        .map_err(|e| {
            error!("Failed to detect layout conflicts: {}", e);
            HttpError::Api(ApiError::InternalServerError)
        })?;

    if let Some(conflict) = conflicts {
        let response = serde_json::json!({
            "chain_id": conflict.chain_id,
            "contract_address": conflict.contract_address,
            "conflicting_layouts": conflict.conflicting_layouts.into_iter().map(|(version, layout)| {
                serde_json::json!({
                    "version": version,
                    "layout_commitment": hex::encode(layout.layout_commitment),
                    "provided_by": layout.provided_by,
                    "timestamp": layout.timestamp,
                })
            }).collect::<Vec<_>>(),
            "detected_at": conflict.detected_at,
            "resolution": conflict.resolution,
        });

        Ok((StatusCode::OK, Json(ApiResponse::success(response))))
    } else {
        Ok((StatusCode::OK, Json(ApiResponse::success(serde_json::json!({
            "message": "No layout conflicts detected",
            "chain_id": chain_id,
            "contract_address": contract_address,
        })))))
    }
}

/// Resolve layout conflicts
pub async fn resolve_layout_conflict(
    State(state): State<Arc<AppState>>,
    Path((chain_id, contract_address)): Path<(String, String)>,
    Json(resolution): Json<serde_json::Value>,
) -> Result<impl IntoResponse, HttpError> {
    let chosen_version = resolution["chosen_version"]
        .as_str()
        .ok_or_else(|| HttpError::Api(ApiError::BadRequest("Missing chosen_version".to_string())))?;
    
    let reason = resolution["reason"]
        .as_str()
        .ok_or_else(|| HttpError::Api(ApiError::BadRequest("Missing reason".to_string())))?;
    
    let resolved_by = resolution["resolved_by"]
        .as_str()
        .ok_or_else(|| HttpError::Api(ApiError::BadRequest("Missing resolved_by".to_string())))?;

    let storage = &state.storage;
    
    storage.resolve_layout_conflict(
        &chain_id,
        &contract_address,
        chosen_version,
        reason,
        resolved_by,
    ).await
    .map_err(|e| {
        error!("Failed to resolve layout conflict: {}", e);
        HttpError::Api(ApiError::InternalServerError)
    })?;

    info!(
        "Resolved layout conflict for contract {} on chain {}: chose version {}",
        contract_address, chain_id, chosen_version
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(serde_json::json!({
        "message": "Layout conflict resolved",
        "chain_id": chain_id,
        "contract_address": contract_address,
        "chosen_version": chosen_version,
        "reason": reason,
        "resolved_by": resolved_by,
    })))))
}

/// Validate layout submission request
fn validate_layout_submission(request: &LayoutSubmissionRequest) -> Result<(), HttpError> {
    // Validate chain ID format
    if request.chain_id.is_empty() {
        return Err(HttpError::Api(ApiError::BadRequest("Chain ID cannot be empty".to_string()));
    }

    // Validate contract address format
    if request.contract_address.is_empty() {
        return Err(HttpError::Api(ApiError::BadRequest("Contract address cannot be empty".to_string()));
    }

    // Validate layout commitment format (should be 64 hex chars for SHA256)
    if request.layout_commitment.len() != 64 {
        return Err(HttpError::Api(ApiError::BadRequest(
            "Layout commitment must be 64 hex characters (SHA256)".to_string()
        ));
    }

    // Validate developer ID
    if request.developer_id.is_empty() {
        return Err(HttpError::Api(ApiError::BadRequest("Developer ID cannot be empty".to_string()));
    }

    // Validate compiler version
    if request.compiler_version.is_empty() {
        return Err(HttpError::Api(ApiError::BadRequest("Compiler version cannot be empty".to_string()));
    }

    Ok(())
}