// Health check API endpoints for monitoring component health
//
// This module provides HTTP endpoints for health monitoring including
// component-specific health checks, system-wide health aggregation,
// and health statistics reporting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use indexer_core::health::{
    HealthMonitor, HealthStatus, HealthCheckResult, HealthStats, HealthEvent
};
use indexer_core::{Error, Result};

/// Health API state
#[derive(Clone)]
pub struct HealthApiState {
    /// Health monitor instance
    pub health_monitor: Arc<HealthMonitor>,
    
    /// API configuration
    pub config: HealthApiConfig,
}

/// Health API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthApiConfig {
    /// Enable detailed health information
    pub detailed_info: bool,
    
    /// Cache health results for this duration (seconds)
    pub cache_duration: u64,
    
    /// Enable health history tracking
    pub enable_history: bool,
    
    /// Maximum history entries to keep
    pub max_history_entries: usize,
}

impl Default for HealthApiConfig {
    fn default() -> Self {
        Self {
            detailed_info: true,
            cache_duration: 30,
            enable_history: true,
            max_history_entries: 100,
        }
    }
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall system status
    pub status: HealthStatus,
    
    /// Response timestamp
    pub timestamp: SystemTime,
    
    /// Individual component health
    pub components: HashMap<String, ComponentHealthInfo>,
    
    /// System health summary
    pub summary: HealthSummary,
    
    /// Health statistics (if detailed)
    pub stats: Option<HealthStats>,
}

/// Component health information
#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentHealthInfo {
    /// Component status
    pub status: HealthStatus,
    
    /// Last check message
    pub message: String,
    
    /// Last check timestamp
    pub last_check: SystemTime,
    
    /// Check duration
    pub check_duration_ms: u64,
    
    /// Additional metadata (if detailed)
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Health summary information
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthSummary {
    /// Total number of components
    pub total_components: usize,
    
    /// Number of healthy components
    pub healthy_components: usize,
    
    /// Number of degraded components
    pub degraded_components: usize,
    
    /// Number of unhealthy components
    pub unhealthy_components: usize,
    
    /// Number of unknown status components
    pub unknown_components: usize,
    
    /// Overall health percentage
    pub health_percentage: f64,
}

/// Query parameters for health endpoints
#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    /// Include detailed information
    #[serde(default)]
    pub detailed: bool,
    
    /// Include statistics
    #[serde(default)]
    pub include_stats: bool,
    
    /// Include metadata
    #[serde(default)]
    pub include_metadata: bool,
}

/// Create health API router
pub fn create_health_router(state: HealthApiState) -> Router {
    Router::new()
        .route("/health", get(get_system_health))
        .route("/health/components", get(get_all_components_health))
        .route("/health/components/:component", get(get_component_health))
        .route("/health/stats", get(get_health_stats))
        .route("/health/summary", get(get_health_summary))
        .route("/health/events", get(get_health_events))
        .route("/readiness", get(get_readiness))
        .route("/liveness", get(get_liveness))
        .with_state(state)
}

/// Get overall system health
pub async fn get_system_health(
    State(state): State<HealthApiState>,
    Query(query): Query<HealthQuery>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let health_monitor = &state.health_monitor;
    
    // Get system health status
    let system_status = health_monitor.get_system_health().await;
    
    // Get all component health
    let all_health = health_monitor.get_all_health().await;
    
    // Build component health info
    let mut components = HashMap::new();
    for (component_name, health_result) in all_health {
        let info = ComponentHealthInfo {
            status: health_result.status,
            message: health_result.message,
            last_check: health_result.timestamp,
            check_duration_ms: health_result.duration.as_millis() as u64,
            metadata: if query.include_metadata || query.detailed {
                Some(health_result.metadata)
            } else {
                None
            },
        };
        components.insert(component_name, info);
    }
    
    // Calculate summary
    let summary = calculate_health_summary(&components);
    
    // Get stats if requested
    let stats = if query.include_stats || query.detailed {
        Some(health_monitor.get_stats().await)
    } else {
        None
    };
    
    let response = HealthResponse {
        status: system_status,
        timestamp: SystemTime::now(),
        components,
        summary,
        stats,
    };
    
    // Set appropriate HTTP status code
    let status_code = match response.status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // Still responding
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        HealthStatus::Unknown => StatusCode::SERVICE_UNAVAILABLE,
    };
    
    if status_code.is_success() {
        Ok(Json(response))
    } else {
        Err(status_code)
    }
}

/// Get health for all components
pub async fn get_all_components_health(
    State(state): State<HealthApiState>,
    Query(query): Query<HealthQuery>,
) -> Json<HashMap<String, ComponentHealthInfo>> {
    let health_monitor = &state.health_monitor;
    let all_health = health_monitor.get_all_health().await;
    
    let mut components = HashMap::new();
    for (component_name, health_result) in all_health {
        let info = ComponentHealthInfo {
            status: health_result.status,
            message: health_result.message,
            last_check: health_result.timestamp,
            check_duration_ms: health_result.duration.as_millis() as u64,
            metadata: if query.include_metadata || query.detailed {
                Some(health_result.metadata)
            } else {
                None
            },
        };
        components.insert(component_name, info);
    }
    
    Json(components)
}

/// Get health for a specific component
pub async fn get_component_health(
    Path(component): Path<String>,
    State(state): State<HealthApiState>,
    Query(query): Query<HealthQuery>,
) -> Result<Json<ComponentHealthInfo>, StatusCode> {
    let health_monitor = &state.health_monitor;
    
    if let Some(health_result) = health_monitor.get_component_health(&component).await {
        let info = ComponentHealthInfo {
            status: health_result.status,
            message: health_result.message,
            last_check: health_result.timestamp,
            check_duration_ms: health_result.duration.as_millis() as u64,
            metadata: if query.include_metadata || query.detailed {
                Some(health_result.metadata)
            } else {
                None
            },
        };
        
        Ok(Json(info))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get health monitoring statistics
pub async fn get_health_stats(
    State(state): State<HealthApiState>,
) -> Json<HealthStats> {
    let health_monitor = &state.health_monitor;
    let stats = health_monitor.get_stats().await;
    Json(stats)
}

/// Get health summary
pub async fn get_health_summary(
    State(state): State<HealthApiState>,
) -> Json<HealthSummary> {
    let health_monitor = &state.health_monitor;
    let all_health = health_monitor.get_all_health().await;
    
    let mut components = HashMap::new();
    for (component_name, health_result) in all_health {
        let info = ComponentHealthInfo {
            status: health_result.status,
            message: health_result.message,
            last_check: health_result.timestamp,
            check_duration_ms: health_result.duration.as_millis() as u64,
            metadata: None,
        };
        components.insert(component_name, info);
    }
    
    let summary = calculate_health_summary(&components);
    Json(summary)
}

/// Get recent health events
pub async fn get_health_events(
    State(state): State<HealthApiState>,
) -> Json<Vec<String>> {
    // For now, return empty events
    // In a full implementation, this would return recent health events
    Json(vec!["Health events endpoint - implementation pending".to_string()])
}

/// Readiness probe endpoint
pub async fn get_readiness(
    State(state): State<HealthApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let health_monitor = &state.health_monitor;
    let system_status = health_monitor.get_system_health().await;
    
    let ready = matches!(system_status, HealthStatus::Healthy | HealthStatus::Degraded);
    
    let response = serde_json::json!({
        "ready": ready,
        "status": system_status,
        "timestamp": SystemTime::now()
    });
    
    if ready {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Liveness probe endpoint
pub async fn get_liveness(
    State(state): State<HealthApiState>,
) -> Json<serde_json::Value> {
    // Liveness just checks if the service is running
    // This endpoint responding means the service is alive
    Json(serde_json::json!({
        "alive": true,
        "timestamp": SystemTime::now()
    }))
}

/// Calculate health summary from component health info
fn calculate_health_summary(components: &HashMap<String, ComponentHealthInfo>) -> HealthSummary {
    let total_components = components.len();
    let mut healthy_components = 0;
    let mut degraded_components = 0;
    let mut unhealthy_components = 0;
    let mut unknown_components = 0;
    
    for info in components.values() {
        match info.status {
            HealthStatus::Healthy => healthy_components += 1,
            HealthStatus::Degraded => degraded_components += 1,
            HealthStatus::Unhealthy => unhealthy_components += 1,
            HealthStatus::Unknown => unknown_components += 1,
        }
    }
    
    let health_percentage = if total_components > 0 {
        (healthy_components as f64 / total_components as f64) * 100.0
    } else {
        0.0
    };
    
    HealthSummary {
        total_components,
        healthy_components,
        degraded_components,
        unhealthy_components,
        unknown_components,
        health_percentage,
    }
}

/// Health check middleware for automatic health reporting
pub async fn health_check_middleware(
    health_monitor: Arc<HealthMonitor>,
) -> impl axum::middleware::FromFn<(), axum::body::Body> {
    axum::middleware::from_fn(move |req, next| {
        let health_monitor = health_monitor.clone();
        async move {
            // Add health information to response headers
            let mut response = next.run(req).await;
            
            let system_health = health_monitor.get_system_health().await;
            response.headers_mut().insert(
                "X-Health-Status",
                system_health.to_string().parse().unwrap(),
            );
            
            response
        }
    })
}

/// Initialize health monitoring for common components
pub async fn initialize_default_health_checks(
    health_monitor: Arc<HealthMonitor>,
) -> Result<()> {
    use indexer_core::health::{DatabaseHealthChecker, ApiHealthChecker};
    
    // Register database health checker
    let db_checker = Arc::new(DatabaseHealthChecker::new(
        "database".to_string(),
        "SELECT 1".to_string(),
    ));
    health_monitor.register_component(db_checker).await;
    
    // Register API health checker
    let api_checker = Arc::new(ApiHealthChecker::new(
        "api_server".to_string(),
        "http://localhost:8080/health".to_string(),
    ));
    health_monitor.register_component(api_checker).await;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use indexer_core::health::HealthCheckConfig;
    
    #[tokio::test]
    async fn test_health_endpoints() {
        let health_monitor = Arc::new(HealthMonitor::new(HealthCheckConfig::default()));
        let config = HealthApiConfig::default();
        let state = HealthApiState {
            health_monitor,
            config,
        };
        
        let app = create_health_router(state);
        let server = TestServer::new(app).unwrap();
        
        // Test system health endpoint
        let response = server.get("/health").await;
        assert!(response.status_code().is_success() || response.status_code() == 503);
        
        // Test readiness endpoint
        let response = server.get("/readiness").await;
        assert!(response.status_code().is_success() || response.status_code() == 503);
        
        // Test liveness endpoint
        let response = server.get("/liveness").await;
        assert!(response.status_code().is_success());
        
        // Test health stats endpoint
        let response = server.get("/health/stats").await;
        assert!(response.status_code().is_success());
    }
    
    #[tokio::test]
    async fn test_health_summary_calculation() {
        let mut components = HashMap::new();
        
        components.insert("comp1".to_string(), ComponentHealthInfo {
            status: HealthStatus::Healthy,
            message: "OK".to_string(),
            last_check: SystemTime::now(),
            check_duration_ms: 10,
            metadata: None,
        });
        
        components.insert("comp2".to_string(), ComponentHealthInfo {
            status: HealthStatus::Degraded,
            message: "Degraded".to_string(),
            last_check: SystemTime::now(),
            check_duration_ms: 20,
            metadata: None,
        });
        
        let summary = calculate_health_summary(&components);
        
        assert_eq!(summary.total_components, 2);
        assert_eq!(summary.healthy_components, 1);
        assert_eq!(summary.degraded_components, 1);
        assert_eq!(summary.health_percentage, 50.0);
    }
} 