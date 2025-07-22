use crate::types::ApiResponseBuilder;
use crate::{auth::ApiKeyInfo, error::HttpError, state::AppState};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct CreateKeyResponse {
    pub key: String,
    pub name: String,
    pub rate_limit: Option<u32>,
}

/// Create a new API key
pub async fn create_api_key(
    State(state): State<AppState>,
    Json(request): Json<CreateKeyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    // Generate a new API key
    let key = format!("ak_{}", Uuid::new_v4().to_string().replace("-", ""));

    let key_info = ApiKeyInfo {
        key: key.clone(),
        name: request.name.clone(),
        rate_limit: request.rate_limit.map(|limit| limit as usize),
        enabled: true,
    };

    // Store the key
    state.key_store.add_key(key_info).await;

    let response_data = CreateKeyResponse {
        key,
        name: request.name,
        rate_limit: request.rate_limit,
    };

    let response = ApiResponseBuilder::new(response_data)
        .version(&state.version)
        .build();

    Ok(Json(response))
}

#[derive(Debug, Serialize)]
pub struct KeyListItem {
    pub name: String,
    pub rate_limit: Option<u32>,
    pub enabled: bool,
    pub key_prefix: String, // Show only first 8 chars of key
}

/// List all API keys
pub async fn list_api_keys(State(state): State<AppState>) -> Result<impl IntoResponse, HttpError> {
    let keys = state.key_store.list_keys().await;

    let items: Vec<KeyListItem> = keys
        .into_iter()
        .map(|info| KeyListItem {
            name: info.name,
            rate_limit: info.rate_limit.map(|limit| limit as u32),
            enabled: info.enabled,
            key_prefix: info.key.chars().take(8).collect(),
        })
        .collect();

    let response = ApiResponseBuilder::new(items)
        .version(&state.version)
        .build();

    Ok(Json(response))
}

/// Delete an API key
pub async fn delete_api_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, HttpError> {
    let removed = state.key_store.remove_key(&key).await;

    if !removed {
        return Err(HttpError::Api(crate::types::ApiError::NotFound(
            "API key not found".to_string(),
        )));
    }

    #[derive(Serialize)]
    struct DeleteResponse {
        message: String,
    }

    let response_data = DeleteResponse {
        message: "API key deleted successfully".to_string(),
    };

    let response = ApiResponseBuilder::new(response_data)
        .version(&state.version)
        .build();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct UpdateKeyRequest {
    pub enabled: Option<bool>,
    pub rate_limit: Option<u32>,
}

/// Update an API key
pub async fn update_api_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<UpdateKeyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    // Get existing key info
    let mut key_info = state.key_store.validate_key(&key).await.ok_or_else(|| {
        HttpError::Api(crate::types::ApiError::NotFound(
            "API key not found".to_string(),
        ))
    })?;

    // Update fields if provided
    if let Some(enabled) = request.enabled {
        key_info.enabled = enabled;
    }

    if let Some(rate_limit) = request.rate_limit {
        key_info.rate_limit = Some(rate_limit as usize);
    }

    // Update the key
    state.key_store.add_key(key_info.clone()).await;

    let response = ApiResponseBuilder::new(key_info)
        .version(&state.version)
        .build();

    Ok(Json(response))
}
