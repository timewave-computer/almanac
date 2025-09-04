use crate::types::{ApiError, ApiResponse};
use crate::AppState;
use almanac_chains::subscription::AccountUpdate;
use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Request to simulate account updates
#[derive(Debug, Clone, Deserialize)]
pub struct SimulateUpdateRequest {
    pub updates: Vec<AccountUpdate>,
}

/// Response from update simulation
#[derive(Debug, Clone, Serialize)]
pub struct SimulateUpdateResponse {
    pub processed: usize,
    pub new_tree_root: String,
}

/// Simulate account updates for testing
#[cfg(debug_assertions)]
pub async fn simulate_account_update(
    State(state): State<AppState>,
    Json(request): Json<SimulateUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Simulating {} account updates", request.updates.len());

    if let Some(ref subscription_manager) = state.subscription_manager {
        let update_count = request.updates.len();
        
        // Process the updates
        let new_root = subscription_manager
            .process_account_updates(request.updates)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
        
        let response = SimulateUpdateResponse {
            processed: update_count,
            new_tree_root: hex::encode(new_root),
        };

        Ok(Json(ApiResponse::success(response)))
    } else {
        Err(ApiError::internal("Subscription manager not available"))
    }
}