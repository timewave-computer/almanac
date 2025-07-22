use crate::types::ApiResponseBuilder;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

// Health check types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthState {
    pub status: HealthStatus,
    pub components: std::collections::HashMap<String, ComponentHealth>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_check: chrono::DateTime<chrono::Utc>,
}
use crate::{error::HttpError, state::AppState};

/// Health check handler
pub async fn health_check(State(state): State<AppState>) -> Result<impl IntoResponse, HttpError> {
    // Check storage health
    let storage_healthy = state.storage.health_check().await.unwrap_or(false);

    let storage_component = ComponentHealth {
        status: if storage_healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        },
        message: if storage_healthy {
            None
        } else {
            Some("Storage health check failed".to_string())
        },
        last_check: chrono::Utc::now(),
    };

    let overall_status = if storage_healthy {
        HealthStatus::Healthy
    } else {
        HealthStatus::Degraded
    };

    let mut components = std::collections::HashMap::new();
    components.insert("storage".to_string(), storage_component);

    let health = HealthState {
        status: overall_status.clone(),
        components,
        timestamp: chrono::Utc::now(),
    };

    let response = ApiResponseBuilder::new(health)
        .version(&state.version)
        .build();

    let status_code = match overall_status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK,
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    Ok((status_code, Json(response)))
}

/// Liveness probe handler (for Kubernetes)
pub async fn liveness() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Readiness probe handler (for Kubernetes)
pub async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    // Check if we can reach storage
    match state.storage.health_check().await {
        Ok(true) => (StatusCode::OK, "Ready"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "Not Ready"),
    }
}
