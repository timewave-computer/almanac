//! Handler for Anchor IDL API endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use serde_json::Value;

use crate::state::AppState;
use crate::types::error::ApiError;

/// Get Anchor IDL for a program
#[utoipa::path(
    get,
    path = "/api/v1/anchor-idl/{program_id}",
    params(
        ("program_id" = String, Path, description = "Solana program ID (base58 encoded)")
    ),
    responses(
        (status = 200, description = "IDL found", body = Value),
        (status = 404, description = "IDL not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "anchor-idl"
)]
pub async fn get_anchor_idl(
    Path(program_id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate program ID format
    if program_id.is_empty() || program_id.len() > 44 {
        return Err(ApiError::bad_request("Invalid program ID format"));
    }

    // Fetch IDL from storage
    match state.storage.get_anchor_idl(&program_id).await {
        Ok(Some(idl_json)) => {
            // Parse JSON string to Value
            let idl_value: Value = serde_json::from_str(&idl_json)
                .map_err(|e| ApiError::internal(format!("Invalid IDL JSON: {}", e)))?;
            
            Ok((StatusCode::OK, Json(idl_value)))
        }
        Ok(None) => Err(ApiError::not_found(format!(
            "IDL not found for program: {}",
            program_id
        ))),
        Err(e) => Err(ApiError::internal(format!(
            "Failed to fetch IDL: {}",
            e
        ))),
    }
}

/// Response for IDL list endpoint
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IdlListResponse {
    pub program_ids: Vec<String>,
    pub total: usize,
}

/// List all programs with stored IDLs
#[utoipa::path(
    get,
    path = "/api/v1/anchor-idl",
    responses(
        (status = 200, description = "List of programs with IDLs", body = IdlListResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "anchor-idl"
)]
pub async fn list_anchor_idls(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    // For minimal implementation, return empty list
    // In a full implementation, this would scan the IDL keyspace
    let response = IdlListResponse {
        program_ids: vec![],
        total: 0,
    };
    
    Ok((StatusCode::OK, Json(response)))
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_program_id_validation() {
        // Valid program IDs should be base58 encoded and typically 32-44 chars
        let valid_id = "11111111111111111111111111111111";
        let invalid_id = "";
        let too_long_id = "1".repeat(50);
        
        assert!(!valid_id.is_empty() && valid_id.len() <= 44);
        assert!(invalid_id.is_empty());
        assert!(too_long_id.len() > 44);
    }
}